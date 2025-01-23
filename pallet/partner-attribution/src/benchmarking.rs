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
use frame_system::RawOrigin;

benchmarks! {
	register_partner {
		let acc: T::AccountId = account("acc", 0, 0);
	}: _(RawOrigin::Signed(acc.clone()), acc.clone())
	verify {
		assert_eq!(NextPartnerId::<T>::get(), 2);
		let partner = Partners::<T>::get(1).unwrap();
		assert_eq!(partner.owner, acc.clone());
		assert_eq!(partner.account, acc);
		assert_eq!(partner.fee_percentage, None);
		assert_eq!(partner.accumulated_fees, 0);
	}

	update_partner_account {
		let acc: T::AccountId = account("acc", 0, 0);
		let new_acc: T::AccountId = account("new_acc", 0, 0);
		PartnerAttribution::<T>::register_partner(RawOrigin::Signed(acc.clone()).into(), acc.clone()).unwrap();
	}: _(RawOrigin::Signed(acc.clone()), 1, Some(new_acc.clone()))
	verify {
		let partner = Partners::<T>::get(1).unwrap();
		assert_eq!(partner.owner, acc);
		assert_eq!(partner.account, new_acc);
	}

	attribute_account {
		let acc: T::AccountId = account("acc", 0, 0);
		let partner_id: u128 = 1;
		PartnerAttribution::<T>::register_partner(RawOrigin::Signed(acc.clone()).into(), acc.clone()).unwrap();
	}: _(RawOrigin::Signed(acc.clone()), Some(partner_id))
	verify {
		let got_partner_id = Attributions::<T>::get(acc).unwrap();
		assert_eq!(got_partner_id, partner_id);
	}

	upgrade_partner {
		let acc: T::AccountId = account("acc", 0, 0);
		PartnerAttribution::<T>::register_partner(RawOrigin::Signed(acc.clone()).into(), acc.clone()).unwrap();
	}: _(RawOrigin::Root, 1, Permill::from_percent(10u32))
	verify {
		let partner = Partners::<T>::get(1).unwrap();
		assert_eq!(partner.owner, acc.clone());
		assert_eq!(partner.account, acc);
		assert_eq!(partner.fee_percentage, Some(Permill::from_percent(10)));
		assert_eq!(partner.accumulated_fees, 0);
	}
}

impl_benchmark_test_suite!(
	PartnerAttribution,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
