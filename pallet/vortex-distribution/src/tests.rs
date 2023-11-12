// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

use super::*;
use crate::mock::{
	create_account, run_to_block, to_eth, AssetsExt, Balances, RuntimeEvent as MockEvent,
	RuntimeOrigin as Origin, System, Test, TestExt, Timestamp, Vortex, BLOCK_TIME, XRP_ASSET_ID,
};
use frame_support::{assert_noop, assert_ok};
use seed_primitives::{AccountId, Balance};

#[test]
fn create_vtx_dist_with_valid_amount_should_work() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		let vortex_token_amount = 1000;

		let vortex_dis_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root(), vortex_token_amount));

		System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistEnabled {
			id: vortex_dis_id,
		}));

		assert_eq!(VtxDistStatuses::<Test>::get(vortex_dis_id), VtxDistStatus::Enabled);
		assert_eq!(TotalVortex::<Test>::get(vortex_dis_id), vortex_token_amount);
		assert_eq!(NextVortexId::<Test>::get(), vortex_dis_id + 1);
	});
}

#[test]
fn create_vtx_dist_with_zero_amount_should_fail() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		let vortex_token_amount = 0;

		assert_noop!(
			Vortex::create_vtx_dist(Origin::root(), vortex_token_amount),
			crate::Error::<Test>::InvalidAmount
		);
	});
}

#[test]
fn create_vtx_dist_without_root_origin_should_fail() {
	TestExt::default().build().execute_with(|| {
		let non_admin = create_account(2);
		System::set_block_number(1);

		let vortex_token_amount = 1000;

		assert_noop!(
			Vortex::create_vtx_dist(Origin::signed(non_admin), vortex_token_amount),
			frame_support::dispatch::DispatchError::BadOrigin
		);
	});
}

#[test]
fn create_vtx_dist_with_exceed_u32_vtx_dist_id_should_fail() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		let vortex_token_amount = 1000;

		NextVortexId::<Test>::put(u32::MAX);

		assert_noop!(
			Vortex::create_vtx_dist(Origin::root(), vortex_token_amount),
			crate::Error::<Test>::VtxDistIdNotAvailable
		);
	});
}

#[test]
fn disable_vtx_dist_should_work() {
	TestExt::default().build().execute_with(|| {
		// Simulate the admin account
		System::set_block_number(1);

		let vortex_dist_id = NextVortexId::<Test>::get();

		// Create a vortex distribution
		let vortex_token_amount = 1000;
		assert_ok!(Vortex::create_vtx_dist(Origin::root(), vortex_token_amount));

		// Disable the vortex distribution
		assert_ok!(Vortex::disable_vtx_dist(Origin::root(), vortex_dist_id));

		assert_eq!(VtxDistStatuses::<Test>::get(vortex_dist_id), VtxDistStatus::NotEnabled);

		// Check for the VtxDistDisabled event
		System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistDisabled {
			id: vortex_dist_id,
		}));
	});
}

#[test]
fn disable_vtx_dist_nonexistent_should_fail() {
	TestExt::default().build().execute_with(|| {
		// Attempt to disable a non-existent distribution ID
		let non_existent_id = 9999; // Assume this ID does not exist

		assert_noop!(
			Vortex::disable_vtx_dist(Origin::root(), non_existent_id),
			crate::Error::<Test>::VtxDistNotEnabled
		);
	});
}

#[test]
fn disable_vtx_dist_without_permission_should_fail() {
	TestExt::default().build().execute_with(|| {
		// Create a vortex distribution
		let vortex_token_amount = 1000;
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root(), vortex_token_amount));

		// Non-admin attempts to disable the distribution
		let non_admin = create_account(2);

		assert_noop!(
			Vortex::disable_vtx_dist(Origin::signed(non_admin), vortex_dist_id),
			frame_support::dispatch::DispatchError::BadOrigin
		);
	});
}

#[test]
fn start_vtx_dist_with_enabled_status_should_work() {
	TestExt::default().build().execute_with(|| {
		// Simulate the admin account
		let admin = create_account(1);
		System::set_block_number(1);

		// Create a vortex distribution with a valid amount
		let vortex_token_amount = 1000;
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root(), vortex_token_amount));

		assert_ok!(Vortex::trigger_vtx_distribution(
			Origin::root(),
			1,
			1,
			admin,
			admin,
			vortex_dist_id
		));

		// Start the vortex distribution
		assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dist_id));

		// Verify the status of the distribution has been set to Paying
		assert_eq!(VtxDistStatuses::<Test>::get(vortex_dist_id), VtxDistStatus::Paying);

		// Check for the VtxDistStarted event
		System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistStarted {
			id: vortex_dist_id,
		}));
	});
}

#[test]
fn start_vtx_dist_with_nonexistent_id_should_fail() {
	TestExt::default().build().execute_with(|| {
		// Attempt to start a distribution with a non-existent ID
		let non_existent_id = 9999; // Assume this ID does not exist

		assert_noop!(
			Vortex::start_vtx_dist(Origin::root(), non_existent_id),
			crate::Error::<Test>::NotTriggered
		);
	});
}

#[test]
fn start_vtx_dist_without_root_origin_should_fail() {
	TestExt::default().build().execute_with(|| {
		// Create a vortex distribution with a valid amount
		let vortex_token_amount = 1000;
		assert_ok!(Vortex::create_vtx_dist(Origin::root(), vortex_token_amount));

		let vortex_dist_id = NextVortexId::<Test>::get();

		// Non-root user attempts to start the distribution
		let non_admin = create_account(2);

		assert_noop!(
			Vortex::start_vtx_dist(Origin::signed(non_admin), vortex_dist_id),
			frame_support::dispatch::DispatchError::BadOrigin
		);
	});
}

#[test]
fn start_vtx_dist_with_already_paying_status_should_fail() {
	TestExt::default().build().execute_with(|| {
		// Create a vortex distribution with a valid amount
		let vortex_token_amount = 1000;
		assert_ok!(Vortex::create_vtx_dist(Origin::root(), vortex_token_amount));

		let vortex_dist_id = NextVortexId::<Test>::get();
		let admin = create_account(1);

		assert_ok!(Vortex::trigger_vtx_distribution(
			Origin::root(),
			1,
			1,
			admin,
			admin,
			vortex_dist_id
		));

		// Start the vortex distribution
		assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dist_id));

		// Attempt to start the same distribution again
		assert_noop!(
			Vortex::start_vtx_dist(Origin::root(), vortex_dist_id),
			crate::Error::<Test>::NotTriggered
		);
	});
}

#[test]
fn pay_unsigned_should_fail_when_called_by_signed_origin() {
	TestExt::default().build().execute_with(|| {
		let user_account = create_account(2);
		let vortex_dist_id = NextVortexId::<Test>::get();

		assert_noop!(
			Vortex::pay_unsigned(Origin::signed(user_account), vortex_dist_id, 2),
			frame_support::dispatch::DispatchError::BadOrigin
		);
	});
}

#[test]
fn pay_unsigned_should_fail_if_status_is_not_paying() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	let end_block = 10;

	TestExt::default()
		.with_balances(&[(alice, 1_000_000)])
		.with_asset(<Test as crate::Config>::NativeAssetId::get(), "ROOT", &[(alice, 1_000_000)])
		.with_asset(<Test as crate::Config>::VtxAssetId::get(), "VORTEX", &[(alice, 0)])
		.build()
		.execute_with(|| {
			// create 3 tokens
			let usdc = AssetsExt::create(&alice, None).unwrap();
			let weth = AssetsExt::create(&alice, None).unwrap();

			let vortex_dis_id = NextVortexId::<Test>::get();

			// mint tokens to user - fee vault
			assert_ok!(AssetsExt::mint_into(usdc, &alice, 1_000_000)); //fee vault
			assert_ok!(AssetsExt::mint_into(weth, &alice, 1_000_000)); //fee vault

			// list vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root(), 1_000_000));
			//set asset price
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				BoundedVec::try_from(vec![(usdc, 100), (weth, 200)]).unwrap(),
				vortex_dis_id,
			));

			//register vortex token rewards for everyone
			assert_ok!(Vortex::register_rewards(
				Origin::root(),
				vortex_dis_id,
				BoundedVec::try_from(vec![(bob, 500_000), (charlie, 500_000)]).unwrap()
			));

			//trigger vortext reward calcuation and assets/root transfer to vault
			assert_ok!(Vortex::trigger_vtx_distribution(
				Origin::root(),
				1,
				1,
				alice, //as root vault
				alice, //as fee vault
				vortex_dis_id,
			));

			//start the vortex distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dis_id,));

			assert_eq!(VtxDistOrderbook::<Test>::get(vortex_dis_id, bob), (500_000, false));

			run_to_block(end_block);

			assert_ok!(Vortex::pay_unsigned(Origin::none(), vortex_dis_id, end_block));

			assert!(
				!System::events().iter().all(|record| {
					match record.event {
						MockEvent::Vortex(crate::Event::VtxDistPaidOut { .. }) => false,
						_ => true,
					}
				}),
				"No payouts should occur as the distribution status is not 'Paying'."
			);
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob),
				500_000
			);
			assert_eq!(
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get()),
				1_000_000
			);

			assert_eq!(VtxDistOrderbook::<Test>::get(vortex_dis_id, bob), (500_000, true));
		});
}

#[test]
fn pay_unsigned_with_multiple_payout_blocks() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	let end_block = 1000;

	TestExt::default()
		.with_balances(&[(alice, 1_000_000)])
		.with_asset(<Test as crate::Config>::NativeAssetId::get(), "ROOT", &[(alice, 1_000_000)])
		.with_asset(<Test as crate::Config>::VtxAssetId::get(), "VORTEX", &[(alice, 0)])
		.build()
		.execute_with(|| {
			// create 3 tokens
			let usdc = AssetsExt::create(&alice, None).unwrap();
			let weth = AssetsExt::create(&alice, None).unwrap();

			let vortex_dis_id = NextVortexId::<Test>::get();

			// mint tokens to user - fee vault
			assert_ok!(AssetsExt::mint_into(usdc, &alice, 1_000_000)); //fee vault
			assert_ok!(AssetsExt::mint_into(weth, &alice, 1_000_000)); //fee vault

			// list vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root(), 100_000_000));
			//set asset price
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				BoundedVec::try_from(vec![(usdc, 100), (weth, 200)]).unwrap(),
				vortex_dis_id,
			));

			//register vortex token rewards for everyone
			let mut rewards_vec = vec![(bob, 500_000), (charlie, 500_000)];
			for i in 0..5000 {
				rewards_vec.push((create_account(i + 4), 100));
			}

			assert_ok!(Vortex::register_rewards(
				Origin::root(),
				vortex_dis_id,
				BoundedVec::try_from(rewards_vec).unwrap()
			));

			//trigger vortext reward calcuation and assets/root transfer to vault
			assert_ok!(Vortex::trigger_vtx_distribution(
				Origin::root(),
				1,
				1,
				alice, //as root vault
				alice, //as fee vault
				vortex_dis_id,
			));

			//start the vortex distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dis_id,));

			assert_eq!(VtxDistOrderbook::<Test>::get(vortex_dis_id, bob), (500_000, false));

			while System::block_number() < end_block {
				System::set_block_number(System::block_number() + 1);
				Vortex::on_initialize(System::block_number());
				Timestamp::set_timestamp(System::block_number() * BLOCK_TIME);

				let next_unsigned_at = NextUnsignedAt::<Test>::get();
				if next_unsigned_at > System::block_number() {
					continue
				}

				assert_ok!(Vortex::pay_unsigned(
					Origin::none(),
					vortex_dis_id,
					System::block_number()
				));
			}

			assert!(
				!System::events().iter().all(|record| {
					match record.event {
						MockEvent::Vortex(crate::Event::VtxDistPaidOut { .. }) => false,
						_ => true,
					}
				}),
				"No payouts should occur as the distribution status is not 'Paying'."
			);
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob),
				500_000
			);
			assert_eq!(
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get()),
				1_500_000
			);

			assert_eq!(VtxDistOrderbook::<Test>::get(vortex_dis_id, bob), (500_000, true));
		});
}

#[test]
fn set_vtx_dist_eras_should_work() {
	TestExt::default().build().execute_with(|| {
		// Create a new vortex distribution with a valid amount.
		let vortex_token_amount: Balance = 1000;
		assert_ok!(Vortex::create_vtx_dist(Origin::root(), vortex_token_amount));

		// Retrieve the ID of the newly created vortex distribution.
		let vortex_dist_id = NextVortexId::<Test>::get();

		// Define the start and end eras for the distribution.
		let start_era: EraIndex = 1;
		let end_era: EraIndex = 10;

		// Set the eras for the vortex distribution.
		assert_ok!(Vortex::set_vtx_dist_eras(Origin::root(), vortex_dist_id, start_era, end_era));

		// Check that the correct event was emitted.
		System::assert_last_event(MockEvent::Vortex(crate::Event::SetVtxDistEras {
			id: vortex_dist_id,
			start_era,
			end_era,
		}));
	});
}

#[test]
fn set_vtx_dist_eras_with_invalid_era_should_fail() {
	TestExt::default().build().execute_with(|| {
		// Create a new vortex distribution.
		let vortex_token_amount: Balance = 1000;
		assert_ok!(Vortex::create_vtx_dist(Origin::root(), vortex_token_amount));
		let vortex_dist_id = NextVortexId::<Test>::get();

		// Attempt to set end era before the start era, which should fail.
		let start_era: EraIndex = 10;
		let end_era: EraIndex = 1;

		assert_noop!(
			Vortex::set_vtx_dist_eras(Origin::root(), vortex_dist_id, start_era, end_era),
			Error::<Test>::InvalidEndBlock
		);
	});
}

#[test]
fn set_vtx_dist_eras_without_permission_should_fail() {
	TestExt::default().build().execute_with(|| {
		// Create a new vortex distribution.
		let vortex_token_amount: Balance = 1000;
		assert_ok!(Vortex::create_vtx_dist(Origin::root(), vortex_token_amount));
		let vortex_dist_id = NextVortexId::<Test>::get();

		// Attempt to set the eras for the distribution without the required permissions.
		let start_era: EraIndex = 1;
		let end_era: EraIndex = 10;
		let non_admin = create_account(2);

		assert_noop!(
			Vortex::set_vtx_dist_eras(
				Origin::signed(non_admin),
				vortex_dist_id,
				start_era,
				end_era
			),
			frame_support::dispatch::DispatchError::BadOrigin
		);
	});
}

#[test]
fn set_asset_prices_should_work() {
	TestExt::default().build().execute_with(|| {
		// Admin creates a new vortex distribution with a specified amount.
		let vortex_token_amount: Balance = 1000;
		assert_ok!(Vortex::create_vtx_dist(Origin::root(), vortex_token_amount));

		// Retrieve the ID of the newly created vortex distribution.
		let vortex_dist_id = NextVortexId::<Test>::get();

		// Define some asset prices to be set.
		let asset_prices: Vec<(AssetId, Balance)> = vec![(100, 500), (101, 300)];
		let bounded_asset_prices: BoundedVec<_, _> =
			BoundedVec::try_from(asset_prices.clone()).expect("Should not exceed limit");

		// Set asset prices for the vortex distribution.
		assert_ok!(Vortex::set_asset_prices(
			Origin::root(),
			bounded_asset_prices.clone(),
			vortex_dist_id
		));

		// Check that the correct event was emitted.
		System::assert_last_event(MockEvent::Vortex(crate::Event::SetAssetPrices {
			id: vortex_dist_id,
			asset_prices: bounded_asset_prices,
		}));
	});
}

#[test]
fn set_asset_prices_with_invalid_asset_id_should_fail() {
	TestExt::default().build().execute_with(|| {
		// Admin creates a new vortex distribution.
		let vortex_token_amount: Balance = 1000;
		assert_ok!(Vortex::create_vtx_dist(Origin::root(), vortex_token_amount));
		let vortex_dist_id = NextVortexId::<Test>::get();

		// Define an invalid asset price (e.g., using the VTX asset ID which should not be allowed).
		let invalid_asset_prices: Vec<(AssetId, Balance)> = vec![(2, 500)];
		let bounded_invalid_asset_prices: BoundedVec<_, _> =
			BoundedVec::try_from(invalid_asset_prices).expect("Should not exceed limit");

		// Attempt to set asset prices with an invalid asset ID, which should fail.
		assert_noop!(
			Vortex::set_asset_prices(Origin::root(), bounded_invalid_asset_prices, vortex_dist_id),
			Error::<Test>::AssetsShouldNotIncludeVtxAsset
		);
	});
}

#[test]
fn set_asset_prices_without_permission_should_fail() {
	TestExt::default().build().execute_with(|| {
		// Admin creates a new vortex distribution.
		let vortex_token_amount: Balance = 1000;
		assert_ok!(Vortex::create_vtx_dist(Origin::root(), vortex_token_amount));
		let vortex_dist_id = NextVortexId::<Test>::get();

		// Non-admin account tries to set asset prices.
		let non_admin = create_account(2);
		let asset_prices: Vec<(AssetId, Balance)> = vec![(XRP_ASSET_ID, 500)];
		let bounded_asset_prices: BoundedVec<_, _> =
			BoundedVec::try_from(asset_prices).expect("Should not exceed limit");

		// Attempt to set asset prices without the required permissions.
		assert_noop!(
			Vortex::set_asset_prices(
				Origin::signed(non_admin),
				bounded_asset_prices,
				vortex_dist_id
			),
			frame_support::dispatch::DispatchError::BadOrigin
		);
	});
}

#[test]
fn register_rewards_with_invalid_distribution_id_should_fail() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	TestExt::default()
		.with_balances(&[(alice, 1_000_000)])
		.with_asset(<Test as crate::Config>::NativeAssetId::get(), "ROOT", &[(alice, 1_000_000)])
		.with_asset(<Test as crate::Config>::VtxAssetId::get(), "VORTEX", &[(alice, 0)])
		.build()
		.execute_with(|| {
			// create 3 tokens
			let usdc = AssetsExt::create(&alice, None).unwrap();
			let weth = AssetsExt::create(&alice, None).unwrap();

			let vortex_dis_id = NextVortexId::<Test>::get();

			// mint tokens to user - fee vault
			assert_ok!(AssetsExt::mint_into(usdc, &alice, 1_000_000)); //fee vault
			assert_ok!(AssetsExt::mint_into(weth, &alice, 1_000_000)); //fee vault

			// list vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root(), 1_000_000));
			//set asset price
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				BoundedVec::try_from(vec![(usdc, 100), (weth, 200)]).unwrap(),
				vortex_dis_id,
			));
			//trigger vortext reward calcuation and assets/root transfer to vault
			assert_ok!(Vortex::trigger_vtx_distribution(
				Origin::root(),
				1,
				1,
				alice, //as root vault
				alice, //as fee vault
				vortex_dis_id,
			));

			assert_noop!(
				Vortex::register_rewards(
					Origin::root(),
					vortex_dis_id,
					BoundedVec::try_from(vec![(bob, 500_000), (charlie, 500_000)]).unwrap()
				),
				Error::<Test>::VtxDistNotEnabled
			);
		});
}

#[test]
fn register_rewards_without_permission_should_fail() {
	TestExt::default().build().execute_with(|| {
		// Admin creates a new vortex distribution.
		let vortex_token_amount: Balance = 1000;
		assert_ok!(Vortex::create_vtx_dist(Origin::root(), vortex_token_amount));
		let vortex_dist_id = NextVortexId::<Test>::get();

		// Define some rewards to be registered.
		let rewards: Vec<(AccountId, Balance)> = vec![(create_account(2), 500)];
		let bounded_rewards: BoundedVec<_, _> =
			BoundedVec::try_from(rewards).expect("Should not exceed limit");

		// Non-admin account tries to register rewards.
		let non_admin = create_account(3);

		// Attempt to register rewards without the required permissions.
		assert_noop!(
			Vortex::register_rewards(Origin::signed(non_admin), vortex_dist_id, bounded_rewards),
			frame_support::dispatch::DispatchError::BadOrigin
		);
	});
}

#[test]
fn register_rewards_fails_if_amount_exceeds_total() {
	let alice: AccountId = create_account(1);
	TestExt::default().build().execute_with(|| {
		Vortex::create_vtx_dist(Origin::root(), 1000).unwrap();

		assert_noop!(
			Vortex::register_rewards(
				Origin::root(),
				1,
				BoundedVec::try_from(vec![(alice, 1001)]).unwrap()
			),
			Error::<Test>::InvalidAmount,
		);
	});
}

#[test]
fn redeem_fails_if_amount_exceeds_balance() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	TestExt::default()
		.with_asset(<Test as crate::Config>::VtxAssetId::get(), "VORTEX", &[(alice, 1_000_000)])
		.build()
		.execute_with(|| {
			Vortex::create_vtx_dist(Origin::root(), 1000).unwrap();

			assert_noop!(
				Vortex::redeem_tokens_from_vault(Origin::signed(bob), 1, 1200),
				Error::<Test>::InvalidAmount
			);
		});
}

#[test]
fn trigger_vtx_distribution_should_work() {
	TestExt::default().build().execute_with(|| {
		// Assume the existence of a vault account.
		let vault_account = create_account(2);
		// Assume the existence of a fee account.
		let fee_account = create_account(3);

		// Admin creates a new vortex distribution with a specified amount.
		let vortex_token_amount: Balance = 1000;
		assert_ok!(Vortex::create_vtx_dist(Origin::root(), vortex_token_amount));

		// Retrieve the ID of the newly created vortex distribution.
		let vortex_dist_id = NextVortexId::<Test>::get();

		// Simulate root and vortex prices.
		let root_price: Balance = 100;
		let vortex_price: Balance = 200;

		// Trigger the vortex distribution process.
		assert_ok!(Vortex::trigger_vtx_distribution(
			Origin::root(),
			root_price,
			vortex_price,
			vault_account.clone(),
			fee_account.clone(),
			vortex_dist_id,
		));

		// Check that the correct event was emitted.
		System::assert_last_event(MockEvent::Vortex(crate::Event::TriggerVtxDistribution {
			id: vortex_dist_id,
			root_vault: vault_account,
			fee_vault: fee_account,
			root_price,
			vortex_price,
		}));
	});
}

#[test]
fn trigger_vtx_distribution_should_fail_if_already_triggered() {
	TestExt::default().build().execute_with(|| {
		// Assume the existence of a vault account.
		let vault_account = create_account(2);
		// Assume the existence of a fee account.
		let fee_account = create_account(3);

		// Admin creates a new vortex distribution with a specified amount.
		let vortex_token_amount: Balance = 1000;
		assert_ok!(Vortex::create_vtx_dist(Origin::root(), vortex_token_amount));

		// Retrieve the ID of the newly created vortex distribution.
		let vortex_dist_id = NextVortexId::<Test>::get();

		// Simulate root and vortex prices.
		let root_price: Balance = 100;
		let vortex_price: Balance = 200;

		// Trigger the vortex distribution process.
		assert_ok!(Vortex::trigger_vtx_distribution(
			Origin::root(),
			root_price,
			vortex_price,
			vault_account.clone(),
			fee_account.clone(),
			vortex_dist_id,
		));

		// Attempt to trigger the same distribution again should fail.
		assert_noop!(
			Vortex::trigger_vtx_distribution(
				Origin::root(),
				root_price,
				vortex_price,
				vault_account.clone(),
				fee_account.clone(),
				vortex_dist_id,
			),
			Error::<Test>::AlreadyTriggered
		);
	});
}

#[test]
fn trigger_vtx_distribution_should_fail_without_permission() {
	TestExt::default().build().execute_with(|| {
		// Assume the existence of a vault account.
		let vault_account = create_account(2);
		// Assume the existence of a fee account.
		let fee_account = create_account(3);

		// Admin creates a new vortex distribution with a specified amount.
		let vortex_token_amount: Balance = 1000;
		assert_ok!(Vortex::create_vtx_dist(Origin::root(), vortex_token_amount));

		// Retrieve the ID of the newly created vortex distribution.
		let vortex_dist_id = NextVortexId::<Test>::get();

		// Simulate root and vortex prices.
		let root_price: Balance = 100;
		let vortex_price: Balance = 200;

		// A non-admin user attempts to trigger the distribution.
		let non_admin_account = create_account(4);
		assert_noop!(
			Vortex::trigger_vtx_distribution(
				Origin::signed(non_admin_account),
				root_price,
				vortex_price,
				vault_account.clone(),
				fee_account.clone(),
				vortex_dist_id,
			),
			frame_support::dispatch::DispatchError::BadOrigin
		);
	});
}

#[test]
fn redeem_tokens_from_vault_should_work() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	let end_block = 10;

	TestExt::default()
		.with_balances(&[(alice, 1_000_000)])
		.with_asset(<Test as crate::Config>::NativeAssetId::get(), "ROOT", &[(alice, 1_000_000)])
		.with_asset(<Test as crate::Config>::VtxAssetId::get(), "VORTEX", &[(alice, 0)])
		.build()
		.execute_with(|| {
			// create 3 tokens
			let usdc = AssetsExt::create(&alice, None).unwrap();
			let weth = AssetsExt::create(&alice, None).unwrap();

			let vortex_dis_id = NextVortexId::<Test>::get();

			// mint tokens to user - fee vault
			assert_ok!(AssetsExt::mint_into(usdc, &alice, 1_000_000)); //fee vault
			assert_ok!(AssetsExt::mint_into(weth, &alice, 1_000_000)); //fee vault

			// list vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root(), 1_000_000));
			//set asset price
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				BoundedVec::try_from(vec![(usdc, 100), (weth, 200)]).unwrap(),
				vortex_dis_id,
			));

			//register vortex token rewards for everyone
			assert_ok!(Vortex::register_rewards(
				Origin::root(),
				vortex_dis_id,
				BoundedVec::try_from(vec![(bob, 500_000), (charlie, 500_000)]).unwrap()
			));

			//trigger vortext reward calcuation and assets/root transfer to vault
			assert_ok!(Vortex::trigger_vtx_distribution(
				Origin::root(),
				1,
				1,
				alice, //as root vault
				alice, //as fee vault
				vortex_dis_id,
			));

			//start the vortex distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dis_id,));

			run_to_block(end_block);

			assert_ok!(Vortex::pay_unsigned(Origin::none(), vortex_dis_id, end_block));
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob),
				500_000
			);
			assert_eq!(
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get()),
				1_000_000
			);

			assert_ok!(Vortex::redeem_tokens_from_vault(
				Origin::signed(bob),
				vortex_dis_id,
				500_000
			));

			//check withdraw result
			assert_eq!(AssetsExt::balance(usdc, &bob), 500_000);
			assert_eq!(Balances::free_balance(&bob), 500_000);
		});
}

#[test]
fn redeem_tokens_from_vault_should_fail_for_insufficient_balance() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	let end_block = 10;

	TestExt::default()
		.with_balances(&[(alice, 1_000_000)])
		.with_asset(<Test as crate::Config>::NativeAssetId::get(), "ROOT", &[(alice, 1_000_000)])
		.with_asset(<Test as crate::Config>::VtxAssetId::get(), "VORTEX", &[(alice, 0)])
		.build()
		.execute_with(|| {
			// create 3 tokens
			let usdc = AssetsExt::create(&alice, None).unwrap();
			let weth = AssetsExt::create(&alice, None).unwrap();

			let vortex_dis_id = NextVortexId::<Test>::get();

			// mint tokens to user - fee vault
			assert_ok!(AssetsExt::mint_into(usdc, &alice, 1_000_000)); //fee vault
			assert_ok!(AssetsExt::mint_into(weth, &alice, 1_000_000)); //fee vault

			// list vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root(), 1_000_000));
			//set asset price
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				BoundedVec::try_from(vec![(usdc, 100), (weth, 200)]).unwrap(),
				vortex_dis_id,
			));

			//register vortex token rewards for everyone
			assert_ok!(Vortex::register_rewards(
				Origin::root(),
				vortex_dis_id,
				BoundedVec::try_from(vec![(bob, 500_000), (charlie, 500_000)]).unwrap()
			));

			//trigger vortext reward calcuation and assets/root transfer to vault
			assert_ok!(Vortex::trigger_vtx_distribution(
				Origin::root(),
				1,
				1,
				alice, //as root vault
				alice, //as fee vault
				vortex_dis_id,
			));

			//start the vortex distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dis_id,));

			run_to_block(end_block);

			assert_ok!(Vortex::pay_unsigned(Origin::none(), vortex_dis_id, end_block));
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob),
				500_000
			);
			assert_eq!(
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get()),
				1_000_000
			);

			// Define an excessive redeem amount.
			let excessive_amount: Balance = 1_000_000;

			assert_noop!(
				Vortex::redeem_tokens_from_vault(
					Origin::signed(bob),
					vortex_dis_id,
					excessive_amount,
				),
				Error::<Test>::InvalidAmount
			);
		});
}

#[test]
fn vortex_distribution_should_work() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	let end_block = 10;

	TestExt::default()
		.with_asset(<Test as crate::Config>::NativeAssetId::get(), "ROOT", &[(alice, 1_000_000)])
		.with_asset(<Test as crate::Config>::VtxAssetId::get(), "VORTEX", &[(alice, 1_000_000)])
		.build()
		.execute_with(|| {
			// create 3 tokens
			let usdc = AssetsExt::create(&alice, None).unwrap();
			let weth = AssetsExt::create(&alice, None).unwrap();
			let vortex_dis_id = NextVortexId::<Test>::get();

			// mint tokens to user - fee vault
			assert_ok!(AssetsExt::mint_into(usdc, &alice, to_eth(1))); //fee vault
			assert_ok!(AssetsExt::mint_into(weth, &alice, to_eth(1))); //fee vault

			// list vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root(), 1_000_000));

			//set asset price
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				BoundedVec::try_from(vec![(usdc, 100), (weth, 200)]).unwrap(),
				vortex_dis_id,
			));

			//register vortex token rewards for everyone
			assert_ok!(Vortex::register_rewards(
				Origin::root(),
				vortex_dis_id,
				BoundedVec::try_from(vec![(bob, 50), (charlie, 100)]).unwrap()
			));

			//trigger vortext reward calcuation and assets/root transfer to vault
			assert_ok!(Vortex::trigger_vtx_distribution(
				Origin::root(),
				100,
				200,
				alice, //as root vault
				alice, //as fee vault
				vortex_dis_id,
			));

			//start the vortex distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dis_id,));

			run_to_block(end_block);

			assert_ok!(Vortex::pay_unsigned(Origin::none(), vortex_dis_id, end_block));

			//check payout result
			assert_eq!(AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob), 50);
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &charlie),
				100
			);
		});
}
