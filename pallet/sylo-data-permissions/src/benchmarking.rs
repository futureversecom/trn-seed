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

use crate::Pallet as SyloDataPermissions;

use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, BoundedVec};
use frame_system::RawOrigin;
use pallet_sylo_data_verification::Pallet as SyloDataVerification;
use sp_core::{H160, H256};

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId
where
	T::AccountId: From<H160>,
{
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

pub fn bounded_string<T: Get<u32>>(name: &str) -> BoundedVec<u8, T> {
	BoundedVec::truncate_from(name.as_bytes().to_vec())
}

pub fn setup_validation_record<T: pallet_sylo_data_verification::Config>(
	caller: T::AccountId,
	tags: BoundedVec<BoundedVec<u8, T::StringLimit>, T::MaxTags>,
) -> BoundedVec<u8, <T as pallet_sylo_data_verification::Config>::StringLimit> {
	let data_id = bounded_string::<T::StringLimit>("data-id");
	let resolvers = BoundedVec::new();
	let data_type = bounded_string::<T::StringLimit>("data-type");
	let checksum = H256::from_low_u64_be(123);

	assert_ok!(SyloDataVerification::<T>::create_validation_record(
		RawOrigin::Signed(caller).into(),
		data_id.clone(),
		resolvers,
		data_type,
		tags,
		checksum,
	));

	return data_id;
}

benchmarks! {
	where_clause {
		where
			<T as frame_system::Config>::AccountId: From<sp_core::H160> + Into<sp_core::H160>,
			T: pallet_sylo_data_verification::Config,
	}

	grant_data_permissions {
		let p in 1 .. T::MaxPermissions::get();

		let alice = account::<T>("Alice");
		let bob = account::<T>("Bob");

		let data_id = setup_validation_record::<T>(alice.clone(), BoundedVec::new());

		let data_id: Vec<u8> = data_id.into();
		let data_id = BoundedVec::try_from(data_id).unwrap();

		let mut data_ids = BoundedVec::new();
		for _ in 0..p {
			data_ids.force_push(data_id.clone());
		}

	}: _(origin::<T>(&alice), alice.clone(), bob.clone(), data_ids.clone(), DataPermission::VIEW, None, false)
	verify {
		assert_eq!(
			PermissionRecords::<T>::get((&alice, data_id, &bob)).len(),
			<u32 as TryInto<usize>>::try_into(p).unwrap(),
		);
	}

	revoke_data_permission {
		let alice = account::<T>("Alice");
		let bob = account::<T>("Bob");

		let data_id = setup_validation_record::<T>(alice.clone(), BoundedVec::new());

		let data_id: Vec<u8> = data_id.into();
		let data_id = BoundedVec::try_from(data_id).unwrap();

		let data_ids = BoundedVec::<_, <T as pallet::Config>::MaxPermissions>::try_from(
			vec![data_id.clone()]
		).unwrap();

		assert_ok!(SyloDataPermissions::<T>::grant_data_permissions(
			RawOrigin::Signed(alice.clone()).into(),
			alice.clone(),
			bob.clone(),
			data_ids,
			DataPermission::VIEW,
			None,
			false
		));

	}: _(origin::<T>(&alice), alice.clone(), 0, bob.clone(), data_id.clone())
	verify {
		assert_eq!(
			PermissionRecords::<T>::get((&alice, data_id, &bob)).len(),
			0,
		);
	}

	grant_tagged_permissions {
		let p in 1 .. T::MaxPermissions::get();

		let alice = account::<T>("Alice");
		let bob = account::<T>("Bob");

		let mut tags = BoundedVec::new();
		for _ in 0..p {
			tags.force_push(bounded_string::<<T as Config>::StringLimit>("tag"));
		}

	}: _(origin::<T>(&alice), bob.clone(), DataPermission::VIEW, tags.clone(), None, false)
	verify {
		assert_eq!(
			TaggedPermissionRecords::<T>::get(&alice, &bob).get(0).unwrap().1.tags,
			tags,
		);
	}

	revoke_tagged_permissions {
		let alice = account::<T>("Alice");
		let bob = account::<T>("Bob");

		let tags = BoundedVec::new();

		assert_ok!(SyloDataPermissions::<T>::grant_tagged_permissions(
			RawOrigin::Signed(alice.clone()).into(),
			bob.clone(),
			DataPermission::VIEW,
			tags.clone(),
			None,
			false
		));

	}: _(origin::<T>(&alice), bob.clone(), 0)
	verify {
		assert_eq!(
			TaggedPermissionRecords::<T>::get(&alice, &bob).len(),
			0,
		);
	}

	grant_permission_reference {
		let alice = account::<T>("Alice");
		let bob = account::<T>("Bob");

		let permission_record_id = setup_validation_record::<T>(alice.clone(), BoundedVec::new());

		let permission_record_id: Vec<u8> = permission_record_id.into();
		let permission_record_id = BoundedVec::try_from(permission_record_id).unwrap();

	}: _(origin::<T>(&alice), bob.clone(), permission_record_id.clone())
	verify {
		assert!(
			PermissionReferences::<T>::get(&alice, &bob).is_some()
		);
	}

	revoke_permission_reference {
		let alice = account::<T>("Alice");
		let bob = account::<T>("Bob");

		let permission_record_id = setup_validation_record::<T>(alice.clone(), BoundedVec::new());

		let permission_record_id: Vec<u8> = permission_record_id.into();
		let permission_record_id = BoundedVec::try_from(permission_record_id).unwrap();

		assert_ok!(SyloDataPermissions::<T>::grant_permission_reference(
			RawOrigin::Signed(alice.clone()).into(),
			bob.clone(),
			permission_record_id,
		));

	}: _(origin::<T>(&alice), bob.clone())
	verify {
		assert_eq!(
			PermissionReferences::<T>::get(&alice, &bob),
			None,
		);
	}
}

impl_benchmark_test_suite!(
	SyloDataPermissions,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
