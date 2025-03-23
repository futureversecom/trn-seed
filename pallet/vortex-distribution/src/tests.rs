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
use crate::mock::{
	calculate_vtx, calculate_vtx_price, run_to_block, AssetsExt, Balances, NativeAssetId,
	RuntimeEvent as MockEvent, RuntimeOrigin as Origin, System, Test, TestExt, Timestamp, Vortex,
	BLOCK_TIME,
};
use seed_pallet_common::test_prelude::*;

#[test]
fn create_vtx_dist_with_valid_amount_should_work() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		let vortex_dis_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistCreated {
			id: vortex_dis_id,
		}));

		assert_eq!(VtxDistStatuses::<Test>::get(vortex_dis_id), VtxDistStatus::Enabled);
		assert_eq!(TotalVortex::<Test>::get(vortex_dis_id), 0);
		assert_eq!(NextVortexId::<Test>::get(), vortex_dis_id + 1);
	});
}

#[test]
fn create_vtx_dist_without_root_origin_should_fail() {
	TestExt::default().build().execute_with(|| {
		let non_admin = create_account(2);
		System::set_block_number(1);

		assert_noop!(
			Vortex::create_vtx_dist(Origin::signed(non_admin)),
			crate::Error::<Test>::RequireAdmin
		);
	});
}

#[test]
fn create_vtx_dist_with_exceed_u32_vtx_dist_id_should_fail() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		NextVortexId::<Test>::put(u32::MAX);

		assert_noop!(
			Vortex::create_vtx_dist(Origin::root()),
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
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		// Disable the vortex distribution
		assert_ok!(Vortex::disable_vtx_dist(Origin::root(), vortex_dist_id));

		assert_eq!(VtxDistStatuses::<Test>::get(vortex_dist_id), VtxDistStatus::Disabled);

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
			crate::Error::<Test>::VtxDistDisabled
		);
	});
}

#[test]
fn disable_vtx_dist_without_permission_should_fail() {
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		// Non-admin attempts to disable the distribution
		let non_admin = create_account(2);

		assert_noop!(
			Vortex::disable_vtx_dist(Origin::signed(non_admin), vortex_dist_id),
			crate::Error::<Test>::RequireAdmin
		);
	});
}

/*#[test]
fn start_vtx_dist_with_enabled_status_should_work() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	TestExt::default()
		.with_balances(&[(alice, 1_000_000)])
		.with_asset(<Test as crate::Config>::NativeAssetId::get(), "ROOT", &[(alice, 1_000_000)])
		.with_asset(<Test as crate::Config>::VtxAssetId::get(), "VORTEX", &[(alice, 0)])
		.build()
		.execute_with(|| {
		System::set_block_number(1);


		// create 2 tokens
		let usdc = AssetsExt::create(&alice, Some(100)).unwrap();
		let weth = AssetsExt::create(&alice, Some(100)).unwrap();
		// mint tokens to user - fee vault
		assert_ok!(AssetsExt::mint_into(usdc, &alice, 1_000_000)); //fee vault
		assert_ok!(AssetsExt::mint_into(weth, &alice, 1_000_000)); //fee vault

		// fund the bootstrap
		let bootstrap_root_vault = Vortex::get_root_vault_account();
		assert_ok!(Vortex::safe_transfer(
				NativeAssetId::get(),
				&alice,
				&bootstrap_root_vault,
				1_000_000,
				false
			));

		// create root price
		let root_price: Balance = 3;

		// Create a vortex distribution
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		//set asset list
		assert_ok!(Vortex::set_fee_pot_asset_balances(
			Origin::root(),
			vortex_dist_id,
			BoundedVec::try_from(vec![(usdc,10), (weth,10), (ROOT_ASSET_ID,10)]).unwrap(),
		));

		//set asset price
		assert_ok!(Vortex::set_asset_prices(
			Origin::root(),
			vortex_dist_id,
			BoundedVec::try_from(vec![(usdc, 100), (weth, 200), (ROOT_ASSET_ID, root_price)])
				.unwrap(),
		));

		assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id));

		// Start the vortex distribution
		assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dist_id));

		// Verify the status of the distribution has been set to Paying
		assert_eq!(VtxDistStatuses::<Test>::get(vortex_dist_id), VtxDistStatus::Paying);

		// Check for the VtxDistStarted event
		System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistStarted {
			id: vortex_dist_id,
		}));
	});
}*/

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
		let vortex_dist_id = NextVortexId::<Test>::get();

		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		// Non-root user attempts to start the distribution
		let non_admin = create_account(2);

		assert_noop!(
			Vortex::start_vtx_dist(Origin::signed(non_admin), vortex_dist_id),
			crate::Error::<Test>::RequireAdmin
		);
	});
}
/*
#[test]
fn start_vtx_dist_with_already_paying_status_should_fail() {
	TestExt::default().build().execute_with(|| {
		// create user account
		let alice: AccountId = create_account(1);

		// create 2 tokens
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&alice, None).unwrap();

		// set root price
		let root_price: Balance = 3;

		// set distribution eras
		let start_era: EraIndex = 1;
		let end_era: EraIndex = 10;

		// Create a vortex distribution
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		assert_ok!(Vortex::set_vtx_dist_eras(Origin::root(), vortex_dist_id, start_era, end_era));

		//set asset list
		assert_ok!(Vortex::set_assets_list(
			Origin::root(),
			BoundedVec::try_from(vec![usdc, weth, ROOT_ASSET_ID]).unwrap(),
			vortex_dist_id,
		));

		//set asset price
		assert_ok!(Vortex::set_asset_prices(
			Origin::root(),
			BoundedVec::try_from(vec![(usdc, 100), (weth, 200), (ROOT_ASSET_ID, root_price)])
				.unwrap(),
			vortex_dist_id,
		));
		assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id));

		// Start the vortex distribution
		assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dist_id));

		// Attempt to start the same distribution again
		assert_noop!(
			Vortex::start_vtx_dist(Origin::root(), vortex_dist_id),
			crate::Error::<Test>::NotTriggered
		);
	});
}
*/
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
fn vortex_distribution_success() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	let end_block = 10;

	TestExt::default()
		.with_balances(&[(alice, 2_000_000)])
		.with_asset(<Test as crate::Config>::VtxAssetId::get(), "VORTEX", &[(charlie, 5)])
		.build()
		.execute_with(|| {
			// create 2 tokens
			let usdc = AssetsExt::create(&alice, None).unwrap();
			let weth = AssetsExt::create(&alice, None).unwrap();

			let root_price: Balance = 3;

			let vortex_dis_id = NextVortexId::<Test>::get();

			// mint tokens to user
			assert_ok!(AssetsExt::mint_into(usdc, &alice, 1_000_000));
			assert_ok!(AssetsExt::mint_into(weth, &alice, 1_000_000));

			// Transfer bootstrap
			let root_vault = Vortex::get_root_vault_account();
			let bootstrap_root = 1_000_000;
			assert_ok!(Vortex::safe_transfer(
				NativeAssetId::get(),
				&alice,
				&root_vault,
				bootstrap_root.clone(),
				false
			));

			// Transfer fee pot assets
			let fee_vault = Vortex::get_fee_vault_account();
			assert_ok!(Vortex::safe_transfer(usdc, &alice, &fee_vault, 1_000_000, false));
			assert_ok!(Vortex::safe_transfer(weth, &alice, &fee_vault, 1_000_000, false));
			assert_ok!(Vortex::safe_transfer(
				NativeAssetId::get(),
				&alice,
				&fee_vault,
				1_000_000,
				false
			));

			// create vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set vortex vault asset balances
			let vtx_vault_asset_balances = vec![(usdc, 5), (weth, 5), (ROOT_ASSET_ID, 100)];
			assert_ok!(Vortex::set_vtx_vault_asset_balances(
				Origin::root(),
				vortex_dis_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			let vtx_current_supply = 5;
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dis_id,
				vtx_current_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances = vec![(usdc, 10), (weth, 10), (ROOT_ASSET_ID, 10)];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dis_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price
			let asset_prices = vec![(usdc, 100), (weth, 200), (ROOT_ASSET_ID, root_price)];
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				vortex_dis_id,
				BoundedVec::try_from(asset_prices.clone()).unwrap(),
			));

			// register reward and work points
			let reward_points =
				BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();
			let work_points = BoundedVec::try_from(vec![(bob, 10), (charlie, 10)]).unwrap();
			assert_ok!(Vortex::register_reward_points(
				Origin::root(),
				vortex_dis_id,
				reward_points
			));
			assert_ok!(Vortex::register_work_points(Origin::root(), vortex_dis_id, work_points));

			// trigger vortex distribution and do the preparations for distribution
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dis_id));

			// check VtxPrice tally
			let vtx_price_calculted =
				calculate_vtx_price(&vtx_vault_asset_balances, &asset_prices, vtx_current_supply);
			assert_eq!(VtxPrice::<Test>::get(vortex_dis_id), vtx_price_calculted);
			// check vtx amounts tally
			let (total_vortex_network_reward, total_vortex_bootstrap, total_vortex) = calculate_vtx(
				&fee_pot_asset_balances,
				&asset_prices,
				bootstrap_root,
				root_price,
				vtx_price_calculted,
			);
			assert_eq!(TotalVortex::<Test>::get(vortex_dis_id), total_vortex);
			assert_eq!(TotalNetworkReward::<Test>::get(vortex_dis_id), total_vortex_network_reward);
			assert_eq!(TotalBootstrapReward::<Test>::get(vortex_dis_id), total_vortex_bootstrap);

			// check bob got the vortex reward registered
			let staker_pool =
				total_vortex_bootstrap + (Perbill::from_percent(30) * total_vortex_network_reward);
			let workpoint_pool = Perbill::from_percent(70) * total_vortex_network_reward;
			let bob_staker_point_portion =
				Perbill::from_rational(100_000_u128, 100_000_u128 + 100_000_u128);
			let bob_work_points_portion = Perbill::from_rational(10_u128, 10_u128 + 10_u128);
			let bob_vtx_reward_calculated = (bob_staker_point_portion * staker_pool)
				+ (bob_work_points_portion * workpoint_pool);
			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dis_id, bob),
				(bob_vtx_reward_calculated, false)
			);

			//start the vortex distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dis_id,));
			let vtx_held_vault = Vortex::get_vtx_held_account();
			assert_eq!(
				AssetsExt::balance(<Test as Config>::VtxAssetId::get(), &vtx_held_vault),
				total_vortex
			);
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
				bob_vtx_reward_calculated
			);
			// check vtx total issuance now. should be total_vortex + vtx_current_supply
			assert_eq!(
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get()),
				total_vortex + vtx_current_supply
			);
			// orderbook entry should be disabled once paid
			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dis_id, bob),
				(bob_vtx_reward_calculated, true)
			);
		});
}

#[test]
fn pay_unsigned_with_multiple_payout_blocks() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	let end_block = 1000;

	TestExt::default()
		.with_balances(&[(alice, 2_000_000)])
		.with_asset(<Test as crate::Config>::VtxAssetId::get(), "VORTEX", &[(charlie, 5)])
		.build()
		.execute_with(|| {
			// create 2 tokens
			let usdc = AssetsExt::create(&alice, None).unwrap();
			let weth = AssetsExt::create(&alice, None).unwrap();

			let root_price: Balance = 3;

			let vortex_dis_id = NextVortexId::<Test>::get();

			// mint tokens to user
			assert_ok!(AssetsExt::mint_into(usdc, &alice, 1_000_000));
			assert_ok!(AssetsExt::mint_into(weth, &alice, 1_000_000));

			// Transfer bootstrap
			let root_vault = Vortex::get_root_vault_account();
			let bootstrap_root = 1_000_000;
			assert_ok!(Vortex::safe_transfer(
				NativeAssetId::get(),
				&alice,
				&root_vault,
				bootstrap_root.clone(),
				false
			));

			// Transfer fee pot assets
			let fee_vault = Vortex::get_fee_vault_account();
			assert_ok!(Vortex::safe_transfer(usdc, &alice, &fee_vault, 1_000_000, false));
			assert_ok!(Vortex::safe_transfer(weth, &alice, &fee_vault, 1_000_000, false));
			assert_ok!(Vortex::safe_transfer(
				NativeAssetId::get(),
				&alice,
				&fee_vault,
				1_000_000,
				false
			));

			// create vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set vortex vault asset balances
			let vtx_vault_asset_balances = vec![(usdc, 5), (weth, 5), (ROOT_ASSET_ID, 100)];
			assert_ok!(Vortex::set_vtx_vault_asset_balances(
				Origin::root(),
				vortex_dis_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			let vtx_current_supply = 5;
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dis_id,
				vtx_current_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances = vec![(usdc, 100), (weth, 100), (ROOT_ASSET_ID, 100)];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dis_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price
			let asset_prices = vec![(usdc, 100), (weth, 200), (ROOT_ASSET_ID, root_price)];
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				vortex_dis_id,
				BoundedVec::try_from(asset_prices.clone()).unwrap(),
			));

			// register reward and work points
			let mut reward_points_vec = vec![(bob, 100_000), (charlie, 100_000)];
			let mut total_reward_points = 100_000 + 100_000;
			for i in 0..5000 {
				reward_points_vec.push((create_account(i + 4), 100_000));
				total_reward_points += 100_000;
			}
			let reward_points = BoundedVec::try_from(reward_points_vec).unwrap();

			let mut work_points_vec = vec![(bob, 10), (charlie, 10)];
			let mut total_work_points = 10 + 10;
			for i in 0..5000 {
				work_points_vec.push((create_account(i + 4), 10));
				total_work_points += 10;
			}
			let work_points = BoundedVec::try_from(work_points_vec).unwrap();
			assert_ok!(Vortex::register_reward_points(
				Origin::root(),
				vortex_dis_id,
				reward_points
			));
			assert_ok!(Vortex::register_work_points(Origin::root(), vortex_dis_id, work_points));

			//trigger vortext reward calcuation and assets/root transfer to vault
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dis_id,));

			// check VtxPrice tally
			let vtx_price_calculted =
				calculate_vtx_price(&vtx_vault_asset_balances, &asset_prices, vtx_current_supply);
			assert_eq!(VtxPrice::<Test>::get(vortex_dis_id), vtx_price_calculted);
			println!("vtx_price_calculted: {:?}", vtx_price_calculted);
			// check vtx amounts tally
			let (total_vortex_network_reward, total_vortex_bootstrap, total_vortex) = calculate_vtx(
				&fee_pot_asset_balances,
				&asset_prices,
				bootstrap_root,
				root_price,
				vtx_price_calculted,
			);
			assert_eq!(TotalVortex::<Test>::get(vortex_dis_id), total_vortex);
			assert_eq!(TotalNetworkReward::<Test>::get(vortex_dis_id), total_vortex_network_reward);
			assert_eq!(TotalBootstrapReward::<Test>::get(vortex_dis_id), total_vortex_bootstrap);
			println!("total_vortex: {:?}", total_vortex);
			println!("total_vortex_network_reward: {:?}", total_vortex_network_reward);
			println!("total_vortex_bootstrap: {:?}", total_vortex_bootstrap);

			// check bob got the vortex reward registered
			let staker_pool =
				total_vortex_bootstrap + (Perbill::from_percent(30) * total_vortex_network_reward);
			let workpoint_pool = Perbill::from_percent(70) * total_vortex_network_reward;
			let bob_staker_point_portion =
				Perbill::from_rational(100_000_u128, total_reward_points);
			let bob_work_points_portion = Perbill::from_rational(10_u128, total_work_points);
			println!("bob_staker_point_portion: {:?}", bob_staker_point_portion);
			println!("bob_work_points_portion: {:?}", bob_work_points_portion);
			println!("staker_pool: {:?}", staker_pool);
			println!("workpoint_pool: {:?}", workpoint_pool);

			let bob_vtx_reward_calculated = (bob_staker_point_portion * staker_pool)
				+ (bob_work_points_portion * workpoint_pool);
			println!("bob stker rewards: {:?}", bob_staker_point_portion * staker_pool);
			println!("bob workpoint rewards: {:?}", bob_work_points_portion * workpoint_pool);

			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dis_id, bob),
				(bob_vtx_reward_calculated, false)
			);
			println!("bob_vtx_reward_calculated: {:?}", bob_vtx_reward_calculated);

			//start the vortex distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dis_id,));

			while System::block_number() < end_block {
				System::set_block_number(System::block_number() + 1);
				Vortex::on_initialize(System::block_number());
				Timestamp::set_timestamp(System::block_number() * BLOCK_TIME);

				let next_unsigned_at = NextUnsignedAt::<Test>::get();
				if next_unsigned_at > System::block_number() {
					continue;
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
				bob_vtx_reward_calculated
			);
			assert_eq!(
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get()),
				total_vortex + vtx_current_supply
			);

			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dis_id, bob),
				(bob_vtx_reward_calculated, true)
			);
		});
}

/*
#[test]
fn set_vtx_dist_eras_should_work() {
	TestExt::default().build().execute_with(|| {
		// Retrieve the ID of the newly created vortex distribution.
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

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
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

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
		let vortex_dist_id = NextVortexId::<Test>::get();
		// Create a new vortex distribution.
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

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
			crate::Error::<Test>::RequireAdmin
		);
	});
}

#[test]
fn set_asset_prices_should_work() {
	TestExt::default().build().execute_with(|| {
		// Retrieve the ID of the newly created vortex distribution.
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		let root_price: Balance = 1;

		// Define some asset prices to be set.
		let asset_prices: Vec<(AssetId, Balance)> =
			vec![(100, 500), (101, 300), (ROOT_ASSET_ID, root_price)];
		let bounded_asset_prices: BoundedVec<_, _> =
			BoundedVec::try_from(asset_prices.clone()).expect("Should not exceed limit");

		//set asset list before set asset price
		assert_ok!(Vortex::set_assets_list(
			Origin::root(),
			BoundedVec::try_from(vec![100, 101, ROOT_ASSET_ID]).unwrap(),
			vortex_dist_id.clone(),
		));

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
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		let root_price: Balance = 1;

		// Define an invalid asset price (e.g., using the VTX asset ID which should not be allowed).
		let invalid_asset_prices: Vec<(AssetId, Balance)> = vec![(VTX_ASSET_ID, 500)];
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
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		let root_price: Balance = 1;

		// Non-admin account tries to set asset prices.
		let non_admin = create_account(2);
		let asset_prices: Vec<(AssetId, Balance)> =
			vec![(XRP_ASSET_ID, 500), (ROOT_ASSET_ID, root_price)];
		let bounded_asset_prices: BoundedVec<_, _> =
			BoundedVec::try_from(asset_prices).expect("Should not exceed limit");

		// Attempt to set asset prices without the required permissions.
		assert_noop!(
			Vortex::set_asset_prices(
				Origin::signed(non_admin),
				bounded_asset_prices,
				vortex_dist_id
			),
			crate::Error::<Test>::RequireAdmin
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

			let root_price: Balance = 1;

			let vortex_dis_id = NextVortexId::<Test>::get();

			// mint tokens to user - fee vault
			assert_ok!(AssetsExt::mint_into(usdc, &alice, 1_000_000)); //fee vault
			assert_ok!(AssetsExt::mint_into(weth, &alice, 1_000_000)); //fee vault

			// list vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			//set asset list
			assert_ok!(Vortex::set_assets_list(
				Origin::root(),
				BoundedVec::try_from(vec![usdc, weth, ROOT_ASSET_ID]).unwrap(),
				vortex_dis_id,
			));

			//set asset price
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				BoundedVec::try_from(vec![(usdc, 100), (weth, 200), (ROOT_ASSET_ID, root_price)])
					.unwrap(),
				vortex_dis_id,
			));
			//trigger vortext reward calcuation and assets/root transfer to vault
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dis_id,));

			assert_noop!(
				Vortex::register_rewards(
					Origin::root(),
					vortex_dis_id,
					BoundedVec::try_from(vec![(bob, 500_000), (charlie, 500_000)]).unwrap()
				),
				Error::<Test>::VtxDistDisabled
			);

			let era = 1;
			// register effective balance and work points
			let balances = BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();
			let points = BoundedVec::try_from(vec![(bob, 10), (charlie, 10)]).unwrap();
			let rates = BoundedVec::try_from(vec![(bob, 1), (charlie, 1)]).unwrap();

			assert_noop!(
				Vortex::register_eff_bal_n_wk_pts(
					Origin::root(),
					vortex_dis_id,
					era,
					balances.clone(),
					points.clone(),
					rates.clone(),
				),
				Error::<Test>::VtxDistDisabled
			);
		});
}

#[test]
fn register_rewards_without_permission_should_fail() {
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		// Define some rewards to be registered.
		let rewards: Vec<(AccountId, Balance)> = vec![(create_account(2), 500)];
		let bounded_rewards: BoundedVec<_, _> =
			BoundedVec::try_from(rewards).expect("Should not exceed limit");

		// Non-admin account tries to register rewards.
		let non_admin = create_account(3);

		// Attempt to register rewards without the required permissions.
		assert_noop!(
			Vortex::register_rewards(Origin::signed(non_admin), vortex_dist_id, bounded_rewards),
			crate::Error::<Test>::RequireAdmin
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
			Vortex::create_vtx_dist(Origin::root()).unwrap();

			assert_noop!(
				Vortex::redeem_tokens_from_vault(Origin::signed(bob), 1, 1200),
				Error::<Test>::InvalidAmount
			);
		});
}

#[test]
fn trigger_vtx_distribution_should_work() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	TestExt::default()
		.with_balances(&[(alice, 2_000_000)])
		.with_asset(<Test as crate::Config>::NativeAssetId::get(), "ROOT", &[(alice, 1_000_000)])
		.with_asset(<Test as crate::Config>::VtxAssetId::get(), "VORTEX", &[(alice, 0)])
		.build()
		.execute_with(|| {
			// create 3 tokens
			let usdc = AssetsExt::create(&alice, None).unwrap();
			let weth = AssetsExt::create(&alice, None).unwrap();

			let root_price: Balance = 3;

			let vortex_dist_id = NextVortexId::<Test>::get();

			// mint tokens to user - fee vault
			let root_vault = Vortex::get_root_vault_account();
			assert_ok!(Vortex::safe_transfer(
				NativeAssetId::get(),
				&alice,
				&root_vault,
				1_000_000,
				false
			));
			let fee_vault = Vortex::get_fee_vault_account();
			assert_ok!(Vortex::safe_transfer(
				NativeAssetId::get(),
				&alice,
				&fee_vault,
				1_000_000,
				false
			));
			assert_ok!(AssetsExt::mint_into(usdc, &fee_vault, 1_000_000)); //fee vault
			assert_ok!(AssetsExt::mint_into(weth, &fee_vault, 1_000_000)); //fee vault

			// list vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set distribution eras
			let start_era: EraIndex = 1;
			let end_era: EraIndex = 10;
			assert_ok!(Vortex::set_vtx_dist_eras(
				Origin::root(),
				vortex_dist_id,
				start_era,
				end_era
			));

			//set asset list
			assert_ok!(Vortex::set_assets_list(
				Origin::root(),
				BoundedVec::try_from(vec![usdc, weth, ROOT_ASSET_ID]).unwrap(),
				vortex_dist_id,
			));

			//set asset price
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				BoundedVec::try_from(vec![(usdc, 100), (weth, 200), (ROOT_ASSET_ID, root_price)])
					.unwrap(),
				vortex_dist_id,
			));

			// //register vortex token rewards for everyone
			// assert_ok!(Vortex::register_rewards(
			// 	Origin::root(),
			// 	vortex_dist_id,
			// 	BoundedVec::try_from(vec![(bob, 500_000), (charlie, 500_000)]).unwrap()
			// ));

			// register effective balance and work points
			let balances = BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();
			let points = BoundedVec::try_from(vec![(bob, 10), (charlie, 10)]).unwrap();
			let rates = BoundedVec::try_from(vec![(bob, 1), (charlie, 1)]).unwrap();

			for era in start_era..=end_era {
				assert_ok!(Vortex::register_eff_bal_n_wk_pts(
					Origin::root(),
					vortex_dist_id,
					era,
					balances.clone(),
					points.clone(),
					rates.clone(),
				));
			}

			//trigger vortext reward calcuation and assets/root transfer to vault
			assert_eq!(Balances::free_balance(fee_vault), 1_000_000);
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id,));
			assert_eq!(Balances::free_balance(fee_vault), 0);

			// Check that the correct event was emitted.
			System::assert_last_event(MockEvent::Vortex(crate::Event::TriggerVtxDistribution {
				id: vortex_dist_id,
			}));
		});
}

#[test]
fn trigger_vtx_distribution_should_fail_if_already_triggered() {
	TestExt::default().build().execute_with(|| {
		// create user account
		let alice: AccountId = create_account(1);

		// create 2 tokens
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&alice, None).unwrap();

		// create root price
		let root_price: Balance = 3;

		// create distribution eras
		let start_era: EraIndex = 1;
		let end_era: EraIndex = 10;

		// Create a vortex distribution
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		// set eras
		assert_ok!(Vortex::set_vtx_dist_eras(Origin::root(), vortex_dist_id, start_era, end_era));

		//set asset list
		assert_ok!(Vortex::set_assets_list(
			Origin::root(),
			BoundedVec::try_from(vec![usdc, weth, ROOT_ASSET_ID]).unwrap(),
			vortex_dist_id,
		));

		//set asset price
		assert_ok!(Vortex::set_asset_prices(
			Origin::root(),
			BoundedVec::try_from(vec![(usdc, 100), (weth, 200), (ROOT_ASSET_ID, root_price)])
				.unwrap(),
			vortex_dist_id,
		));

		// Trigger the vortex distribution process.
		assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id,));

		// Attempt to trigger the same distribution again should fail.
		assert_noop!(
			Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id,),
			Error::<Test>::CannotTrigger
		);
	});
}

#[test]
fn trigger_vtx_distribution_should_fail_without_permission() {
	TestExt::default().build().execute_with(|| {
		// Admin creates a new vortex distribution
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		// Retrieve the ID of the newly created vortex distribution.
		let vortex_dist_id = NextVortexId::<Test>::get();

		// A non-admin user attempts to trigger the distribution.
		let non_admin_account = create_account(4);
		assert_noop!(
			Vortex::trigger_vtx_distribution(Origin::signed(non_admin_account), vortex_dist_id,),
			crate::Error::<Test>::RequireAdmin
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
			// create 2 tokens
			let usdc = AssetsExt::create(&alice, None).unwrap();
			let weth = AssetsExt::create(&alice, None).unwrap();

			let root_price: Balance = 3;

			let vortex_dis_id = NextVortexId::<Test>::get();

			// mint tokens to user - fee vault
			assert_ok!(AssetsExt::mint_into(usdc, &alice, 1_000_000)); //fee vault
			assert_ok!(AssetsExt::mint_into(weth, &alice, 1_000_000)); //fee vault

			// move token to vaults
			let root_vault = Vortex::get_root_vault_account();
			assert_ok!(Vortex::safe_transfer(
				NativeAssetId::get(),
				&alice,
				&root_vault,
				1_000_000,
				false
			));
			let fee_vault = Vortex::get_fee_vault_account();
			assert_ok!(Vortex::safe_transfer(usdc, &alice, &fee_vault, 1_000_000, false));

			assert_ok!(Vortex::safe_transfer(weth, &alice, &fee_vault, 1_000_000, false));

			// list vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set distribution eras
			let start_era: EraIndex = 1;
			let end_era: EraIndex = 10;
			assert_ok!(Vortex::set_vtx_dist_eras(
				Origin::root(),
				vortex_dis_id,
				start_era,
				end_era
			));

			//set asset list
			assert_ok!(Vortex::set_assets_list(
				Origin::root(),
				BoundedVec::try_from(vec![
					usdc,
					weth,
					<Test as crate::Config>::NativeAssetId::get()
				])
				.unwrap(),
				vortex_dis_id,
			));

			//set asset price
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				BoundedVec::try_from(vec![
					(usdc, 100),
					(weth, 200),
					(<Test as crate::Config>::NativeAssetId::get(), root_price)
				])
				.unwrap(),
				vortex_dis_id,
			));

			// register effective balance and work points
			let balances = BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();
			let points = BoundedVec::try_from(vec![(bob, 10), (charlie, 10)]).unwrap();
			let rates = BoundedVec::try_from(vec![(bob, 1), (charlie, 1)]).unwrap();

			for era in start_era..=end_era {
				assert_ok!(Vortex::register_eff_bal_n_wk_pts(
					Origin::root(),
					vortex_dis_id,
					era,
					balances.clone(),
					points.clone(),
					rates.clone(),
				));
			}

			//trigger vortext reward calcuation and assets/root transfer to vault
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dis_id,));

			//start the vortex distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dis_id,));

			run_to_block(end_block);

			assert_ok!(Vortex::pay_unsigned(Origin::none(), vortex_dis_id, end_block));
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob),
				151500000
			);
			assert_eq!(
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get()),
				303000000
			);

			VtxDistStatuses::<Test>::mutate(vortex_dis_id, |status| {
				*status = VtxDistStatus::Done;
			});

			assert_ok!(Vortex::redeem_tokens_from_vault(
				Origin::signed(bob),
				vortex_dis_id,
				500_000
			));
			//check Bob's balances
			assert_eq!(AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob), 0);
			assert_eq!(AssetsExt::balance(usdc, &bob), 500_000);
			assert_eq!(AssetsExt::balance(weth, &bob), 500_000);
			assert_eq!(Balances::free_balance(bob), 1_000_000);

			// Redeem Charlie's tokens
			assert_ok!(Vortex::redeem_tokens_from_vault(
				Origin::signed(charlie),
				vortex_dis_id,
				500_000
			));
			//check Charlie's balances
			assert_eq!(AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &charlie), 0);
			assert_eq!(AssetsExt::balance(usdc, &charlie), 500_000);
			assert_eq!(AssetsExt::balance(weth, &charlie), 500_000);
			assert_eq!(Balances::free_balance(charlie), 1_000_000);
		});
}

#[test]
fn redeem_tokens_from_vault_should_work_without_root_token_in_asset_prices() {
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
			// create 2 tokens
			let usdc = AssetsExt::create(&alice, None).unwrap();
			let weth = AssetsExt::create(&alice, None).unwrap();

			let root_price: Balance = 3;

			let vortex_dis_id = NextVortexId::<Test>::get();

			// mint tokens to user - fee vault
			assert_ok!(AssetsExt::mint_into(usdc, &alice, 1_000_000)); //fee vault
			assert_ok!(AssetsExt::mint_into(weth, &alice, 1_000_000)); //fee vault

			// move token to vaults
			let root_vault = Vortex::get_root_vault_account();
			assert_ok!(Vortex::safe_transfer(
				NativeAssetId::get(),
				&alice,
				&root_vault,
				1_000_000,
				false
			));
			let fee_vault = Vortex::get_fee_vault_account();
			assert_ok!(Vortex::safe_transfer(usdc, &alice, &fee_vault, 1_000_000, false));

			assert_ok!(Vortex::safe_transfer(weth, &alice, &fee_vault, 1_000_000, false));

			// list vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set distribution eras
			let start_era: EraIndex = 1;
			let end_era: EraIndex = 10;
			assert_ok!(Vortex::set_vtx_dist_eras(
				Origin::root(),
				vortex_dis_id,
				start_era,
				end_era
			));

			//set asset list
			assert_ok!(Vortex::set_assets_list(
				Origin::root(),
				BoundedVec::try_from(vec![
					usdc,
					weth,
					<Test as crate::Config>::NativeAssetId::get()
				])
				.unwrap(),
				vortex_dis_id,
			));

			//set asset price
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				BoundedVec::try_from(vec![(usdc, 100), (weth, 200), (ROOT_ASSET_ID, root_price)])
					.unwrap(),
				vortex_dis_id,
			));

			// register effective balance and work points
			let balances = BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();
			let points = BoundedVec::try_from(vec![(bob, 10), (charlie, 10)]).unwrap();
			let rates = BoundedVec::try_from(vec![(bob, 1), (charlie, 1)]).unwrap();

			for era in start_era..=end_era {
				assert_ok!(Vortex::register_eff_bal_n_wk_pts(
					Origin::root(),
					vortex_dis_id,
					era,
					balances.clone(),
					points.clone(),
					rates.clone(),
				));
			}

			//trigger vortext reward calcuation and assets/root transfer to vault
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dis_id,));

			//start the vortex distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dis_id,));

			run_to_block(end_block);

			assert_ok!(Vortex::pay_unsigned(Origin::none(), vortex_dis_id, end_block));
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob),
				151500000
			);
			assert_eq!(
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get()),
				303000000
			);

			VtxDistStatuses::<Test>::mutate(vortex_dis_id, |status| {
				*status = VtxDistStatus::Done;
			});

			assert_ok!(Vortex::redeem_tokens_from_vault(
				Origin::signed(bob),
				vortex_dis_id,
				151500000
			));
			//check Bob's balances
			assert_eq!(AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob), 0);
			assert_eq!(AssetsExt::balance(usdc, &bob), 500_000);
			assert_eq!(AssetsExt::balance(weth, &bob), 500_000);
			assert_eq!(Balances::free_balance(bob), 0);

			// Redeem Charlie's tokens
			assert_ok!(Vortex::redeem_tokens_from_vault(
				Origin::signed(charlie),
				vortex_dis_id,
				500_000
			));
			//check Charlie's balances
			assert_eq!(AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &charlie), 0);
			assert_eq!(AssetsExt::balance(usdc, &charlie), 500_000);
			assert_eq!(AssetsExt::balance(weth, &charlie), 500_000);
			assert_eq!(Balances::free_balance(charlie), 0);
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
			// create 2 tokens
			let usdc = AssetsExt::create(&alice, None).unwrap();
			let weth = AssetsExt::create(&alice, None).unwrap();

			let root_price: Balance = 3;

			let vortex_dis_id = NextVortexId::<Test>::get();

			// mint tokens to user - fee vault
			assert_ok!(AssetsExt::mint_into(usdc, &alice, 1_000_000)); //fee vault
			assert_ok!(AssetsExt::mint_into(weth, &alice, 1_000_000)); //fee vault

			// move token to vaults
			let root_vault = Vortex::get_root_vault_account();
			assert_ok!(Vortex::safe_transfer(
				NativeAssetId::get(),
				&alice,
				&root_vault,
				1_000_000,
				false
			));
			let fee_vault = Vortex::get_fee_vault_account();
			assert_ok!(Vortex::safe_transfer(usdc, &alice, &fee_vault, 1_000_000, false));

			assert_ok!(Vortex::safe_transfer(weth, &alice, &fee_vault, 1_000_000, false));

			// list vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set distribution eras
			let start_era: EraIndex = 1;
			let end_era: EraIndex = 10;
			assert_ok!(Vortex::set_vtx_dist_eras(
				Origin::root(),
				vortex_dis_id,
				start_era,
				end_era
			));

			//set asset list
			assert_ok!(Vortex::set_assets_list(
				Origin::root(),
				BoundedVec::try_from(vec![usdc, weth, ROOT_ASSET_ID]).unwrap(),
				vortex_dis_id,
			));

			//set asset price
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				BoundedVec::try_from(vec![(usdc, 100), (weth, 200), (ROOT_ASSET_ID, root_price)])
					.unwrap(),
				vortex_dis_id,
			));

			// register effective balance and work points
			let balances = BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();
			let points = BoundedVec::try_from(vec![(bob, 10), (charlie, 10)]).unwrap();
			let rates = BoundedVec::try_from(vec![(bob, 1), (charlie, 1)]).unwrap();

			for era in start_era..=end_era {
				assert_ok!(Vortex::register_eff_bal_n_wk_pts(
					Origin::root(),
					vortex_dis_id,
					era,
					balances.clone(),
					points.clone(),
					rates.clone(),
				));
			}

			//trigger vortext reward calcuation and assets/root transfer to vault
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dis_id,));

			//start the vortex distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dis_id,));

			run_to_block(end_block);

			assert_ok!(Vortex::pay_unsigned(Origin::none(), vortex_dis_id, end_block));
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob),
				151500000
			);
			assert_eq!(
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get()),
				303000000
			);

			// Define an excessive redeem amount.
			let excessive_amount: Balance = 151500001;

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
		.with_balances(&[(alice, 1_000_000)])
		.with_asset(<Test as crate::Config>::NativeAssetId::get(), "ROOT", &[(alice, 1_000_000)])
		.with_asset(<Test as crate::Config>::VtxAssetId::get(), "VORTEX", &[(alice, 1_000_000)])
		.build()
		.execute_with(|| {
			// create 3 tokens
			let usdc = AssetsExt::create(&alice, None).unwrap();
			let weth = AssetsExt::create(&alice, None).unwrap();
			let vortex_dis_id = NextVortexId::<Test>::get();

			let root_price: Balance = 10;

			// create distribution eras
			let start_era: EraIndex = 1;
			let end_era: EraIndex = 10;

			// mint tokens to user - fee vault
			assert_ok!(AssetsExt::mint_into(usdc, &alice, 1_000_000)); //fee vault
			assert_ok!(AssetsExt::mint_into(weth, &alice, 1_000_000)); //fee vault

			// move token to vaults
			let root_vault = Vortex::get_root_vault_account();
			assert_ok!(Vortex::safe_transfer(
				NativeAssetId::get(),
				&alice,
				&root_vault,
				1_000_000,
				false
			));
			let fee_vault = Vortex::get_fee_vault_account();
			assert_ok!(Vortex::safe_transfer(usdc, &alice, &fee_vault, 1_000_000, false));

			assert_ok!(Vortex::safe_transfer(weth, &alice, &fee_vault, 1_000_000, false));

			assert_eq!(
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get()),
				1000000
			);

			assert_eq!(TotalVortex::<Test>::get(vortex_dis_id), 0);

			// list vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set eras
			assert_ok!(Vortex::set_vtx_dist_eras(
				Origin::root(),
				vortex_dis_id,
				start_era,
				end_era
			));

			//set asset list
			assert_ok!(Vortex::set_assets_list(
				Origin::root(),
				BoundedVec::try_from(vec![usdc, weth, ROOT_ASSET_ID]).unwrap(),
				vortex_dis_id,
			));

			//set asset price
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				BoundedVec::try_from(vec![(usdc, 100), (weth, 200), (ROOT_ASSET_ID, root_price)])
					.unwrap(),
				vortex_dis_id,
			));

			//register vortex token rewards for everyone
			// assert_ok!(Vortex::register_rewards(
			// 	Origin::root(),
			// 	vortex_dis_id,
			// 	BoundedVec::try_from(vec![(bob, 50), (charlie, 100)]).unwrap()
			// ));

			// register effective balance and work points
			let balances = BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();
			let points = BoundedVec::try_from(vec![(bob, 10), (charlie, 10)]).unwrap();
			let rates = BoundedVec::try_from(vec![(bob, 1), (charlie, 1)]).unwrap();

			for era in start_era..=end_era {
				assert_ok!(Vortex::register_eff_bal_n_wk_pts(
					Origin::root(),
					vortex_dis_id,
					era,
					balances.clone(),
					points.clone(),
					rates.clone(),
				));
			}

			//trigger vortext reward calcuation and assets/root transfer to vault
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dis_id,));
			assert_eq!(TotalVortex::<Test>::get(vortex_dis_id), 999999);
			assert_eq!(TotalNetworkReward::<Test>::get(vortex_dis_id), 967741);
			assert_eq!(TotalBootstrapReward::<Test>::get(vortex_dis_id), 32258);
			assert_eq!(TotalEffectiveBalanceEra::<Test>::get(vortex_dis_id), 2000000);
			assert_eq!(AccountTotalEffectiveBalance::<Test>::get(vortex_dis_id, bob), 1000000);
			assert_eq!(TotalWorkPointsEra::<Test>::get(vortex_dis_id), 200);
			assert_eq!(AccountTotalWorkPoints::<Test>::get(vortex_dis_id, bob), 100);

			//start the vortex distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dis_id,));

			assert_eq!(
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get()),
				1999999
			);

			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &alice),
				1000000
			);
			assert_eq!(VtxDistOrderbook::<Test>::get(vortex_dis_id, bob), (499999, false));
			assert_eq!(VtxDistOrderbook::<Test>::get(vortex_dis_id, charlie), (499999, false));

			run_to_block(end_block);

			assert_ok!(Vortex::pay_unsigned(Origin::none(), vortex_dis_id, end_block));

			//check payout result
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob),
				499999
			);
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &charlie),
				499999
			);
		});
}
*/
