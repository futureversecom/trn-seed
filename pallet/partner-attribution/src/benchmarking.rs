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

#[allow(unused_imports)]
use crate::Pallet as PartnerAttribution;

use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, traits::fungibles::Mutate};
use frame_system::RawOrigin;

// fund account with ROOT
pub fn fund<T: Config>(account: &T::AccountId)
where
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
{
	let root_asset_id: u32 = 1;
	assert_ok!(T::MultiCurrency::mint_into(root_asset_id.into(), &account, 1_000_000u32.into()));
}

benchmarks! {
	where_clause { where <T as frame_system::Config>::AccountId: From<sp_core::H160> }

	register_partner_account {
		let acc: T::AccountId = account("acc", 0, 0);
	}: _(RawOrigin::Signed(acc.clone()), acc.clone())
	verify {
		assert_eq!(NextPartnerId::<T>::get(), 2);
		assert_eq!(PartnerCount::<T>::get(), 1);
		let partner = Partners::<T>::get(1).unwrap();
		assert_eq!(partner.owner, acc.clone());
		assert_eq!(partner.account, acc);
		assert_eq!(partner.fee_percentage, None);
		assert_eq!(partner.accumulated_fees, 0);
	}

	update_partner_account {
		let acc: T::AccountId = account("acc", 0, 0);
		let new_acc: T::AccountId = account("new_acc", 0, 0);
		PartnerAttribution::<T>::register_partner_account(RawOrigin::Signed(acc.clone()).into(), acc.clone()).unwrap();
	}: _(RawOrigin::Signed(acc.clone()), 1, new_acc.clone())
	verify {
		let partner = Partners::<T>::get(1).unwrap();
		assert_eq!(partner.owner, acc);
		assert_eq!(partner.account, new_acc);
	}

	attribute_account {
		let acc: T::AccountId = account("acc", 0, 0);
		let partner_id: u128 = 1;
		PartnerAttribution::<T>::register_partner_account(RawOrigin::Signed(acc.clone()).into(), acc.clone()).unwrap();

		let futurepass_bytes = {
			let mut bytes = [0u8; 20];
			bytes[..4].copy_from_slice(precompile_utils::constants::FUTUREPASS_PRECOMPILE_ADDRESS_PREFIX);
			bytes
		};
		let futurepass_account: T::AccountId = sp_core::H160::from(futurepass_bytes).into();
	}: _(RawOrigin::Signed(futurepass_account.clone()), partner_id)
	verify {
		let got_partner_id = Attributions::<T>::get(futurepass_account).unwrap();
		assert_eq!(got_partner_id, partner_id);
	}

	upgrade_partner {
		let acc: T::AccountId = account("acc", 0, 0);
		PartnerAttribution::<T>::register_partner_account(RawOrigin::Signed(acc.clone()).into(), acc.clone()).unwrap();
	}: _(RawOrigin::Root, 1, Permill::from_percent(10u32))
	verify {
		let partner = Partners::<T>::get(1).unwrap();
		assert_eq!(partner.owner, acc.clone());
		assert_eq!(partner.account, acc);
		assert_eq!(partner.fee_percentage, Some(Permill::from_percent(10)));
		assert_eq!(partner.accumulated_fees, 0);
	}

	create_futurepass_with_partner {
		let acc: T::AccountId = account("acc", 0, 0);
		fund::<T>(&acc); // fund acc to pay for futurepass creation

		let delegated_acc: T::AccountId = account("delegated", 0, 0);
		let partner_id: u128 = 1;

		PartnerAttribution::<T>::register_partner_account(RawOrigin::Signed(acc.clone()).into(), acc.clone()).unwrap();

		assert!(Attributions::<T>::iter().next().is_none());
	}: _(RawOrigin::Signed(acc.clone()), partner_id, delegated_acc.clone())
	verify {
		// Verify attribution was created - by getting 1st element of the attributions map
		let (_, got_partner_id) = Attributions::<T>::iter()
			.next()
			.expect("Attribution should exist");
		assert_eq!(got_partner_id, partner_id);
	}

	remove_partner {
		let acc: T::AccountId = account("acc", 0, 0);
		PartnerAttribution::<T>::register_partner_account(RawOrigin::Signed(acc.clone()).into(), acc.clone()).unwrap();
		assert_eq!(PartnerCount::<T>::get(), 1);
	}: _(RawOrigin::Root, 1)
	verify {
		assert_eq!(PartnerCount::<T>::get(), 0);
		assert!(Partners::<T>::get(1).is_none());
	}
}

impl_benchmark_test_suite!(
	PartnerAttribution,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
