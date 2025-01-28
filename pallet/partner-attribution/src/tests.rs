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

use super::*;
use crate::mock::{PartnerAttribution, System, Test};
use seed_pallet_common::test_prelude::*;

mod register_partner_account {
	use super::*;

	#[test]
	fn register_partner_account_succeeds() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));

			System::assert_last_event(
				Event::PartnerRegistered {
					partner_id: 1,
					partner: PartnerInformation::<AccountId> {
						owner: alice(),
						account: alice(),
						fee_percentage: None,
						accumulated_fees: 0,
					},
				}
				.into(),
			);

			let partner = Partners::<Test>::get(1).unwrap();
			assert_eq!(partner.owner, alice());
			assert_eq!(partner.account, alice());
			assert_eq!(partner.fee_percentage, None);
			assert_eq!(partner.accumulated_fees, 0);
		});
	}

	#[test]
	fn no_ids_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Put max sale_id
			NextPartnerId::<Test>::put(u128::MAX);

			assert_noop!(
				PartnerAttribution::register_partner_account(Some(alice()).into(), alice(),),
				Error::<Test>::NoAvailableIds
			);
		});
	}
}

mod update_partner_account {
	use super::*;

	#[test]
	fn update_partner_account_succeeds() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));
			assert_ok!(PartnerAttribution::update_partner_account(
				Some(alice()).into(),
				1,
				Some(bob())
			));

			System::assert_last_event(
				Event::PartnerUpdated { partner_id: 1, account: bob() }.into(),
			);

			let partner = Partners::<Test>::get(1).unwrap();
			assert_eq!(partner.account, bob());
		});
	}

	#[test]
	fn remove_partner_succeeds() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));
			assert_ok!(PartnerAttribution::update_partner_account(Some(alice()).into(), 1, None));

			System::assert_last_event(
				Event::PartnerRemoved { partner_id: 1, account: alice() }.into(),
			);

			assert!(Partners::<Test>::get(1).is_none());
		});
	}

	#[test]
	fn remove_non_existent_partner_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_noop!(
				PartnerAttribution::update_partner_account(Some(alice()).into(), 1, None),
				Error::<Test>::PartnerNotFound
			);
		});
	}

	#[test]
	fn remove_partner_without_permission_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));
			assert_noop!(
				PartnerAttribution::update_partner_account(Some(bob()).into(), 1, None),
				Error::<Test>::Unauthorized
			);
		});
	}
}

mod attribute_account {
	use super::*;

	#[test]
	fn attribute_account_succeeds() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));
			assert_ok!(PartnerAttribution::attribute_account(Some(bob()).into(), 1));

			System::assert_last_event(
				Event::AccountAttributed { partner_id: 1, account: bob() }.into(),
			);

			assert_eq!(Attributions::<Test>::get(&bob()).unwrap(), 1);
		});
	}

	#[test]
	fn remove_non_existent_account_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_noop!(
				PartnerAttribution::attribute_account(Some(bob()).into(), 1),
				Error::<Test>::PartnerNotFound
			);
		});
	}
}

mod upgrade_partner {
	use super::*;

	#[test]
	fn upgrade_partner_succeeds() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));
			assert_ok!(PartnerAttribution::upgrade_partner(
				RawOrigin::Root.into(),
				1,
				Permill::from_percent(10)
			));

			System::assert_last_event(
				Event::PartnerUpgraded {
					partner_id: 1,
					account: alice(),
					fee_percentage: Permill::from_percent(10),
				}
				.into(),
			);

			let partner = Partners::<Test>::get(1).unwrap();
			assert_eq!(partner.fee_percentage, Some(Permill::from_percent(10)));
		});
	}

	#[test]
	fn upgrade_non_existent_partner_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_noop!(
				PartnerAttribution::upgrade_partner(
					RawOrigin::Root.into(),
					1,
					Permill::from_percent(10)
				),
				Error::<Test>::PartnerNotFound
			);
		});
	}

	#[test]
	fn upgrade_partner_without_permission_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));
			assert_noop!(
				PartnerAttribution::upgrade_partner(
					Some(bob()).into(),
					1,
					Permill::from_percent(10)
				),
				BadOrigin
			);
			assert_noop!(
				PartnerAttribution::upgrade_partner(
					Some(alice()).into(),
					1,
					Permill::from_percent(10)
				),
				BadOrigin
			);
		});
	}
}
