// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use crate::Pallet as SyloActionPermissions;

use alloc::format;
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::sp_runtime::Saturating;
use frame_support::{assert_ok, traits::GetCallMetadata, BoundedVec};
use frame_system::RawOrigin;
use sp_core::H160;
use sp_std::collections::btree_set::BTreeSet;

/// Helper function to create an account for benchmarking.
fn account<T: Config>(name: &'static str) -> T::AccountId
where
	T::AccountId: From<H160>,
	<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
{
	bench_account(name, 0, 0)
}

fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId>
where
	T::AccountId: From<H160>,
	<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
{
	RawOrigin::Signed(acc.clone())
}

fn to_call_id<T: Get<u32>>(pallet: &str, function: &str) -> CallId<T> {
	(
		BoundedVec::truncate_from(pallet.as_bytes().to_vec()),
		BoundedVec::truncate_from(function.as_bytes().to_vec()),
	)
}

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent)
where
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
	<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
{
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

benchmarks! {
	where_clause {
		where
			<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
			<T as frame_system::Config>::AccountId: From<sp_core::H160> + Into<sp_core::H160>,
	}

	grant_dispatch_permission {
		let c in 1 .. T::MaxCallIds::get();

		let grantor = account::<T>("Grantor");
		let grantee = account::<T>("Grantee");

		let spender = Spender::Grantor;
		let spending_balance = Some(100);

		let mut allowed_calls = BoundedBTreeSet::new();
		for i in 0..c {
			let pallet = format!("pallet_{}", i);
			let function = format!("function_{}", i);
			allowed_calls.try_insert(to_call_id::<T::StringLimit>(&pallet, &function)).unwrap();
		}

		let expiry = Some(<frame_system::Pallet<T>>::block_number().saturating_add(1u32.into()));
	}: _(origin::<T>(&grantor), grantee.clone(), spender.clone(), spending_balance, allowed_calls.clone(), expiry)
	verify {
		let permission = DispatchPermissions::<T>::get(&grantor, &grantee).unwrap();
		assert_eq!(permission.spender, spender);
		assert_eq!(permission.spending_balance, spending_balance);
		assert_eq!(permission.allowed_calls, allowed_calls);
		assert_eq!(permission.expiry, expiry);
	}

	update_dispatch_permission {
		let c in 1 .. T::MaxCallIds::get();

		let grantor = account::<T>("Grantor");
		let grantee = account::<T>("Grantee");

		let spender = Spender::Grantor;
		let spending_balance = Some(100);

		let mut initial_allowed_calls = BoundedBTreeSet::new();
		initial_allowed_calls.try_insert(to_call_id::<T::StringLimit>("pallet_initial", "function_initial")).unwrap();

		let expiry = Some(<frame_system::Pallet<T>>::block_number().saturating_add(1u32.into()));

		assert_ok!(SyloActionPermissions::<T>::grant_dispatch_permission(
			origin::<T>(&grantor).into(),
			grantee.clone(),
			spender.clone(),
			spending_balance,
			initial_allowed_calls,
			expiry,
		));

		let mut updated_allowed_calls = BoundedBTreeSet::new();
		for i in 0..c {
			let pallet = format!("pallet_{}", i);
			let function = format!("function_{}", i);
			updated_allowed_calls.try_insert(to_call_id::<T::StringLimit>(&pallet, &function)).unwrap();
		}

		let updated_expiry = Some(<frame_system::Pallet<T>>::block_number().saturating_add(20u32.into()));
		let updated_spending_balance = Some(200);
	}: _(origin::<T>(&grantor), grantee.clone(), Some(spender.clone()), Some(updated_spending_balance), Some(updated_allowed_calls.clone()), Some(updated_expiry))
	verify {
		let permission = DispatchPermissions::<T>::get(&grantor, &grantee).unwrap();
		assert_eq!(permission.spender, spender);
		assert_eq!(permission.spending_balance, updated_spending_balance);
		assert_eq!(permission.allowed_calls, updated_allowed_calls);
		assert_eq!(permission.expiry, updated_expiry);
	}

	revoke_dispatch_permission {
		let grantor = account::<T>("Grantor");
		let grantee = account::<T>("Grantee");

		let spender = Spender::Grantor;
		let spending_balance = Some(100);

		let mut allowed_calls = BoundedBTreeSet::new();
		allowed_calls.try_insert(to_call_id::<T::StringLimit>("pallet", "function")).unwrap();

		let expiry = Some(<frame_system::Pallet<T>>::block_number().saturating_add(1u32.into()));

		assert_ok!(SyloActionPermissions::<T>::grant_dispatch_permission(
			origin::<T>(&grantor).into(),
			grantee.clone(),
			spender.clone(),
			spending_balance,
			allowed_calls,
			expiry,
		));

	}: _(origin::<T>(&grantor), grantee.clone())
	verify {
		assert!(DispatchPermissions::<T>::get(&grantor, &grantee).is_none());
	}

	transact {
		let grantor = account::<T>("Grantor");
		let grantee = account::<T>("Grantee");

		let spender = Spender::Grantor;
		let spending_balance = Some(100);

		let mut allowed_calls = BoundedBTreeSet::new();
		allowed_calls.try_insert(to_call_id::<T::StringLimit>("*", "*")).unwrap();

		let expiry = Some(<frame_system::Pallet<T>>::block_number().saturating_add(1u32.into()));

		assert_ok!(SyloActionPermissions::<T>::grant_dispatch_permission(
			origin::<T>(&grantor).into(),
			grantee.clone(),
			Spender::Grantee,
			None,
			allowed_calls,
			None,
		));

		let call: <T as Config>::RuntimeCall = frame_system::Call::<T>::remark { remark: b"Mischief Managed".to_vec() }.into();
		let boxed_call = Box::new(call.clone());
	}: _(origin::<T>(&grantee), grantor.clone(), boxed_call)
	verify {
		assert_last_event::<T>(Event::PermissionTransactExecuted { grantor, grantee }.into());
	}
}

impl_benchmark_test_suite!(
	SyloActionPermissions,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
