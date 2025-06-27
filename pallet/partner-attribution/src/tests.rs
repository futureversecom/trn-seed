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

	#[test]
	fn max_partners_exceeded_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Register exactly 200 partners (the MaxPartners limit)
			for i in 0..200 {
				let account = create_account(i);
				assert_ok!(PartnerAttribution::register_partner_account(
					Some(account).into(),
					account
				));
			}

			// Verify we have exactly 200 partners
			assert_eq!(Partners::<Test>::iter().count(), 200);

			// Try to register one more partner - this should fail
			let extra_account = create_account(200);
			assert_noop!(
				PartnerAttribution::register_partner_account(
					Some(extra_account).into(),
					extra_account
				),
				Error::<Test>::MaxPartnersExceeded
			);

			// Verify we still have exactly 200 partners
			assert_eq!(Partners::<Test>::iter().count(), 200);
		});
	}
}

mod update_partner_account {
	use super::*;

	#[test]
	fn update_partner_account_succeeds() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));
			assert_ok!(PartnerAttribution::update_partner_account(Some(alice()).into(), 1, bob()));

			System::assert_last_event(
				Event::PartnerUpdated { partner_id: 1, account: bob() }.into(),
			);

			let partner = Partners::<Test>::get(1).unwrap();
			assert_eq!(partner.account, bob());
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

mod create_futurepass_with_partner {
	use super::*;

	#[test]
	fn create_futurepass_with_partner_succeeds() {
		TestExt::<Test>::default().build().execute_with(|| {
			use crate::mock::RuntimeEvent;

			// First register a partner
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));

			// Create a futurepass with that partner
			assert_ok!(PartnerAttribution::create_futurepass_with_partner(
				Some(charlie()).into(),
				1,
				bob()
			));

			// Get the futurepass address from the emitted event
			let binding = System::events();
			let event = binding.last().expect("Event should exist");
			let futurepass = match event.event {
				RuntimeEvent::PartnerAttribution(Event::AccountAttributed { account, .. }) => {
					account
				},
				_ => panic!("Expected AccountAttributed event"),
			};

			// Verify the futurepass was attributed to the partner
			assert_eq!(Attributions::<Test>::get(&futurepass).unwrap(), 1);

			// Verify event was emitted
			System::assert_last_event(
				Event::AccountAttributed { partner_id: 1, account: futurepass }.into(),
			);
		});
	}

	#[test]
	fn create_futurepass_with_nonexistent_partner_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_noop!(
				PartnerAttribution::create_futurepass_with_partner(
					Some(charlie()).into(),
					1,
					bob()
				),
				Error::<Test>::PartnerNotFound
			);
		});
	}

	#[test]
	fn create_futurepass_when_already_attributed_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			// First register a partner
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));

			// Create first futurepass
			assert_ok!(PartnerAttribution::create_futurepass_with_partner(
				Some(charlie()).into(),
				1,
				bob()
			));

			// Attempt to create another futurepass with same account
			assert_noop!(
				PartnerAttribution::create_futurepass_with_partner(
					Some(charlie()).into(),
					1,
					bob()
				),
				Error::<Test>::AccountAlreadyAttributed
			);
		});
	}
}

// ===== AttributionProvider Trait Tests =====

mod attribution_provider_trait {
	use super::*;
	use seed_pallet_common::AttributionProvider;

	#[test]
	fn get_attributions_returns_only_partners_with_accumulated_fees() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Register multiple partners
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));
			assert_ok!(PartnerAttribution::register_partner_account(Some(bob()).into(), bob()));
			assert_ok!(PartnerAttribution::register_partner_account(
				Some(charlie()).into(),
				charlie()
			));

			// Upgrade partners with fee percentages
			assert_ok!(PartnerAttribution::upgrade_partner(
				RawOrigin::Root.into(),
				1,
				Permill::from_percent(5)
			));
			assert_ok!(PartnerAttribution::upgrade_partner(
				RawOrigin::Root.into(),
				2,
				Permill::from_percent(10)
			));
			assert_ok!(PartnerAttribution::upgrade_partner(
				RawOrigin::Root.into(),
				3,
				Permill::from_percent(15)
			));

			// Initially, all partners should have zero accumulated fees
			let attributions =
				<PartnerAttribution as AttributionProvider<AccountId>>::get_attributions();
			assert_eq!(attributions.len(), 0);

			// Manually set accumulated fees for some partners (simulating fee accumulation)
			Partners::<Test>::mutate(1, |maybe_partner| {
				if let Some(partner) = maybe_partner {
					partner.accumulated_fees = 1000;
				}
			});
			Partners::<Test>::mutate(2, |maybe_partner| {
				if let Some(partner) = maybe_partner {
					partner.accumulated_fees = 2000;
				}
			});
			// Partner 3 remains with zero accumulated fees

			// Now get attributions - should only include partners with non-zero accumulated fees
			let attributions =
				<PartnerAttribution as AttributionProvider<AccountId>>::get_attributions();
			assert_eq!(attributions.len(), 2);

			// Verify the returned data
			let partner1_attribution =
				attributions.iter().find(|(account, _, _)| *account == alice()).unwrap();
			let partner2_attribution =
				attributions.iter().find(|(account, _, _)| *account == bob()).unwrap();

			assert_eq!(partner1_attribution.0, alice());
			assert_eq!(partner1_attribution.1, 1000);
			assert_eq!(partner1_attribution.2, Some(Permill::from_percent(5)));

			assert_eq!(partner2_attribution.0, bob());
			assert_eq!(partner2_attribution.1, 2000);
			assert_eq!(partner2_attribution.2, Some(Permill::from_percent(10)));

			// Partner 3 (charlie) should not be in the results since accumulated_fees is 0
			assert!(attributions.iter().find(|(account, _, _)| *account == charlie()).is_none());
		});
	}

	#[test]
	fn get_attributions_excludes_partners_without_fee_percentage() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Register a partner without upgrading (no fee percentage)
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));

			// Set accumulated fees
			Partners::<Test>::mutate(1, |maybe_partner| {
				if let Some(partner) = maybe_partner {
					partner.accumulated_fees = 1000;
				}
			});

			// Get attributions - should return empty because partner has no fee percentage
			let attributions =
				<PartnerAttribution as AttributionProvider<AccountId>>::get_attributions();
			assert_eq!(attributions.len(), 0);

			// Now upgrade the partner with a fee percentage
			assert_ok!(PartnerAttribution::upgrade_partner(
				RawOrigin::Root.into(),
				1,
				Permill::from_percent(5)
			));

			// Now get attributions - should return the partner
			let attributions =
				<PartnerAttribution as AttributionProvider<AccountId>>::get_attributions();
			assert_eq!(attributions.len(), 1);

			let attribution = &attributions[0];
			assert_eq!(attribution.0, alice());
			assert_eq!(attribution.1, 1000);
			assert_eq!(attribution.2, Some(Permill::from_percent(5)));
		});
	}

	#[test]
	fn get_attributions_returns_empty_when_no_partners() {
		TestExt::<Test>::default().build().execute_with(|| {
			let attributions =
				<PartnerAttribution as AttributionProvider<AccountId>>::get_attributions();
			assert_eq!(attributions.len(), 0);
			assert!(attributions.is_empty());
		});
	}

	#[test]
	fn get_attributions_returns_empty_when_all_partners_have_zero_fees() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Register multiple partners
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));
			assert_ok!(PartnerAttribution::register_partner_account(Some(bob()).into(), bob()));

			// Upgrade partners with fee percentages
			assert_ok!(PartnerAttribution::upgrade_partner(
				RawOrigin::Root.into(),
				1,
				Permill::from_percent(5)
			));
			assert_ok!(PartnerAttribution::upgrade_partner(
				RawOrigin::Root.into(),
				2,
				Permill::from_percent(10)
			));

			// All partners have zero accumulated fees by default
			let attributions =
				<PartnerAttribution as AttributionProvider<AccountId>>::get_attributions();
			assert_eq!(attributions.len(), 0);
			assert!(attributions.is_empty());
		});
	}

	#[test]
	fn reset_balances_clears_all_accumulated_fees() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Register multiple partners
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));
			assert_ok!(PartnerAttribution::register_partner_account(Some(bob()).into(), bob()));
			assert_ok!(PartnerAttribution::register_partner_account(
				Some(charlie()).into(),
				charlie()
			));

			// Upgrade all partners with fee percentages
			assert_ok!(PartnerAttribution::upgrade_partner(
				RawOrigin::Root.into(),
				1,
				Permill::from_percent(5)
			));
			assert_ok!(PartnerAttribution::upgrade_partner(
				RawOrigin::Root.into(),
				2,
				Permill::from_percent(10)
			));
			assert_ok!(PartnerAttribution::upgrade_partner(
				RawOrigin::Root.into(),
				3,
				Permill::from_percent(15)
			));

			// Set accumulated fees for all partners
			Partners::<Test>::mutate(1, |maybe_partner| {
				if let Some(partner) = maybe_partner {
					partner.accumulated_fees = 1000;
				}
			});
			Partners::<Test>::mutate(2, |maybe_partner| {
				if let Some(partner) = maybe_partner {
					partner.accumulated_fees = 2000;
				}
			});
			Partners::<Test>::mutate(3, |maybe_partner| {
				if let Some(partner) = maybe_partner {
					partner.accumulated_fees = 3000;
				}
			});

			// Verify fees are set
			let attributions_before =
				<PartnerAttribution as AttributionProvider<AccountId>>::get_attributions();
			assert_eq!(attributions_before.len(), 3);

			// Call reset_balances
			<PartnerAttribution as AttributionProvider<AccountId>>::reset_balances();

			// Verify all accumulated fees are cleared
			let attributions_after =
				<PartnerAttribution as AttributionProvider<AccountId>>::get_attributions();
			assert_eq!(attributions_after.len(), 0);
			assert!(attributions_after.is_empty());

			// Verify individual partner fees are cleared
			let partner1 = Partners::<Test>::get(1).unwrap();
			let partner2 = Partners::<Test>::get(2).unwrap();
			let partner3 = Partners::<Test>::get(3).unwrap();

			assert_eq!(partner1.accumulated_fees, 0);
			assert_eq!(partner2.accumulated_fees, 0);
			assert_eq!(partner3.accumulated_fees, 0);
		});
	}

	#[test]
	fn reset_balances_preserves_other_partner_data() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Register a partner and upgrade it
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));
			assert_ok!(PartnerAttribution::upgrade_partner(
				RawOrigin::Root.into(),
				1,
				Permill::from_percent(10)
			));

			// Set accumulated fees
			Partners::<Test>::mutate(1, |maybe_partner| {
				if let Some(partner) = maybe_partner {
					partner.accumulated_fees = 1000;
				}
			});

			// Call reset_balances
			<PartnerAttribution as AttributionProvider<AccountId>>::reset_balances();

			// Verify accumulated fees are cleared but other data is preserved
			let partner = Partners::<Test>::get(1).unwrap();
			assert_eq!(partner.accumulated_fees, 0);
			assert_eq!(partner.owner, alice());
			assert_eq!(partner.account, alice());
			assert_eq!(partner.fee_percentage, Some(Permill::from_percent(10)));
		});
	}

	#[test]
	fn reset_balances_works_with_no_partners() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Call reset_balances when no partners exist
			<PartnerAttribution as AttributionProvider<AccountId>>::reset_balances();

			// Should not panic and should not affect anything
			let attributions =
				<PartnerAttribution as AttributionProvider<AccountId>>::get_attributions();
			assert_eq!(attributions.len(), 0);
		});
	}

	#[test]
	fn attribution_provider_trait_compliance() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Test that PartnerAttribution implements AttributionProvider correctly

			// Register a partner and set up test data
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));
			assert_ok!(PartnerAttribution::upgrade_partner(
				RawOrigin::Root.into(),
				1,
				Permill::from_percent(5)
			));

			// Set accumulated fees
			Partners::<Test>::mutate(1, |maybe_partner| {
				if let Some(partner) = maybe_partner {
					partner.accumulated_fees = 1000;
				}
			});

			// Test get_attributions method
			let attributions =
				<PartnerAttribution as AttributionProvider<AccountId>>::get_attributions();
			assert_eq!(attributions.len(), 1);
			assert_eq!(attributions[0].0, alice());
			assert_eq!(attributions[0].1, 1000);
			assert_eq!(attributions[0].2, Some(Permill::from_percent(5)));

			// Test reset_balances method
			<PartnerAttribution as AttributionProvider<AccountId>>::reset_balances();

			let attributions_after_reset =
				<PartnerAttribution as AttributionProvider<AccountId>>::get_attributions();
			assert_eq!(attributions_after_reset.len(), 0);
			assert!(attributions_after_reset.is_empty());
		});
	}

	#[test]
	fn get_attributions_handles_large_numbers() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Register a partner
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));
			assert_ok!(PartnerAttribution::upgrade_partner(
				RawOrigin::Root.into(),
				1,
				Permill::from_percent(5)
			));

			// Set a large accumulated fee value
			let large_fee = u128::MAX / 2;
			Partners::<Test>::mutate(1, |maybe_partner| {
				if let Some(partner) = maybe_partner {
					partner.accumulated_fees = large_fee;
				}
			});

			// Get attributions
			let attributions =
				<PartnerAttribution as AttributionProvider<AccountId>>::get_attributions();
			assert_eq!(attributions.len(), 1);
			assert_eq!(attributions[0].1, large_fee);
		});
	}

	#[test]
	fn get_attributions_returns_correct_order() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Register multiple partners
			assert_ok!(PartnerAttribution::register_partner_account(Some(alice()).into(), alice()));
			assert_ok!(PartnerAttribution::register_partner_account(Some(bob()).into(), bob()));
			assert_ok!(PartnerAttribution::register_partner_account(
				Some(charlie()).into(),
				charlie()
			));

			// Upgrade all partners with fee percentages
			assert_ok!(PartnerAttribution::upgrade_partner(
				RawOrigin::Root.into(),
				1,
				Permill::from_percent(5)
			));
			assert_ok!(PartnerAttribution::upgrade_partner(
				RawOrigin::Root.into(),
				2,
				Permill::from_percent(10)
			));
			assert_ok!(PartnerAttribution::upgrade_partner(
				RawOrigin::Root.into(),
				3,
				Permill::from_percent(15)
			));

			// Set accumulated fees
			Partners::<Test>::mutate(1, |maybe_partner| {
				if let Some(partner) = maybe_partner {
					partner.accumulated_fees = 1000;
				}
			});
			Partners::<Test>::mutate(2, |maybe_partner| {
				if let Some(partner) = maybe_partner {
					partner.accumulated_fees = 2000;
				}
			});
			Partners::<Test>::mutate(3, |maybe_partner| {
				if let Some(partner) = maybe_partner {
					partner.accumulated_fees = 3000;
				}
			});

			// Get attributions multiple times to ensure consistent ordering
			let attributions1 =
				<PartnerAttribution as AttributionProvider<AccountId>>::get_attributions();
			let attributions2 =
				<PartnerAttribution as AttributionProvider<AccountId>>::get_attributions();

			assert_eq!(attributions1.len(), 3);
			assert_eq!(attributions2.len(), 3);

			// The order should be consistent (based on partner ID)
			assert_eq!(attributions1, attributions2);

			// Verify all expected partners are present
			let accounts: Vec<AccountId> =
				attributions1.iter().map(|(account, _, _)| *account).collect();
			assert!(accounts.contains(&alice()));
			assert!(accounts.contains(&bob()));
			assert!(accounts.contains(&charlie()));
		});
	}
}
