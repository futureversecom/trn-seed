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

//! Integration tests for the partner attribution pallet.
#![cfg(test)]

use crate::{
	tests::{alice, ExtBuilder},
	Futurepass, PartnerAttribution, Runtime, RuntimeOrigin,
};
use frame_support::{assert_err, assert_ok};
use seed_pallet_common::test_prelude::create_account;

mod attribute_account {
	use super::*;

	#[test]
	fn attribute_account_fails_with_non_futurepass_account() {
		ExtBuilder::default().build().execute_with(|| {
			let next_partner_id = 1;
			let partner_account = create_account(0);
			let non_futurepass_account = create_account(1);

			assert_ok!(PartnerAttribution::register_partner_account(
				RuntimeOrigin::signed(partner_account),
				partner_account
			));

			let created_partner =
				pallet_partner_attribution::Partners::<Runtime>::get(next_partner_id).unwrap();
			assert_eq!(created_partner.account, partner_account);

			assert_err!(
				PartnerAttribution::attribute_account(
					RuntimeOrigin::signed(non_futurepass_account),
					next_partner_id
				),
				pallet_partner_attribution::Error::<Runtime>::CallerNotFuturepass,
			);
		});
	}

	#[test]
	fn attribute_account_works_with_futurepass_account() {
		ExtBuilder::default().build().execute_with(|| {
			let next_partner_id = 1;
			let partner_account = create_account(0);

			assert_ok!(Futurepass::create(RuntimeOrigin::signed(alice()), alice()));
			let futurepass = pallet_futurepass::Holders::<Runtime>::get(alice()).unwrap();

			assert_ok!(PartnerAttribution::register_partner_account(
				RuntimeOrigin::signed(partner_account),
				partner_account
			));

			let created_partner =
				pallet_partner_attribution::Partners::<Runtime>::get(next_partner_id).unwrap();
			assert_eq!(created_partner.account, partner_account);

			assert_ok!(PartnerAttribution::attribute_account(
				RuntimeOrigin::signed(futurepass),
				next_partner_id
			));

			let attributed_partner =
				pallet_partner_attribution::Attributions::<Runtime>::get(&futurepass).unwrap();
			assert_eq!(attributed_partner, next_partner_id);
		});
	}
}
