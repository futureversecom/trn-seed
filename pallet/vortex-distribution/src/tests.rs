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
	calculate_vtx, calculate_vtx_price, calculate_vtx_redeem, run_to_block, AssetsExt, Balances,
	NativeAssetId, RuntimeEvent as MockEvent, RuntimeOrigin as Origin, System, Test, TestExt,
	Timestamp, Vortex, BLOCK_TIME,
};
use hex_literal::hex;
use seed_pallet_common::test_prelude::*;

#[test]
fn set_admin_works_with_root_account() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		let admin_account = create_account(2);
		assert_ok!(Vortex::set_admin(Origin::root(), admin_account));

		System::assert_last_event(MockEvent::Vortex(crate::Event::AdminAccountChanged {
			old_key: None,
			new_key: admin_account,
		}));

		assert_eq!(AdminAccount::<Test>::get(), Some(admin_account));
	});
}

#[test]
fn set_admin_fails_without_root_account() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);
		let admin_account = create_account(2);
		assert_noop!(
			Vortex::set_admin(Origin::signed(create_account(3)), admin_account),
			BadOrigin
		);
	});
}

#[test]
fn create_vtx_dist_with_valid_account_should_work() {
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
fn create_vtx_dist_without_valid_origin_should_fail() {
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

#[test]
fn set_fee_pot_asset_balances_works() {
	let alice: AccountId = create_account(1);
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		// create 2 tokens
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&alice, None).unwrap();
		// set fee pot asset balances
		let fee_pot_asset_balances = vec![(usdc, 10), (weth, 10), (ROOT_ASSET_ID, 10)];
		let fee_pot_asset_balances_bounded =
			BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap();
		assert_ok!(Vortex::set_fee_pot_asset_balances(
			Origin::root(),
			vortex_dist_id,
			fee_pot_asset_balances_bounded.clone(),
		));

		// Check for the SetFeePotAssetBalances event
		System::assert_last_event(MockEvent::Vortex(crate::Event::SetFeePotAssetBalances {
			id: vortex_dist_id,
			assets_balances: fee_pot_asset_balances_bounded,
		}));
	});
}

#[test]
fn set_fee_pot_asset_balances_fails() {
	let alice: AccountId = create_account(1);
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		// create 2 tokens
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&alice, None).unwrap();
		// set fee pot asset balances
		let fee_pot_asset_balances = vec![(usdc, 10), (weth, 10), (ROOT_ASSET_ID, 10)];
		let mut fee_pot_asset_balances_bounded =
			BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap();

		// fails if not authorized account
		assert_noop!(
			Vortex::set_fee_pot_asset_balances(
				Origin::signed(create_account(5)),
				vortex_dist_id,
				fee_pot_asset_balances_bounded,
			),
			Error::<Test>::RequireAdmin
		);

		// fail if Vtx asset id is included
		fee_pot_asset_balances_bounded = BoundedVec::try_from(vec![
			(<Test as Config>::VtxAssetId::get(), 10),
			(weth, 10),
			(ROOT_ASSET_ID, 10),
		])
		.unwrap();
		assert_noop!(
			Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				fee_pot_asset_balances_bounded,
			),
			Error::<Test>::AssetsShouldNotIncludeVtxAsset
		);
	});
}

#[test]
fn set_vtx_vault_asset_balances_works() {
	let alice: AccountId = create_account(1);
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		// create 2 tokens
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&alice, None).unwrap();
		// set fee pot asset balances
		let vtx_vault_asset_balances = vec![(usdc, 10), (weth, 10), (ROOT_ASSET_ID, 10)];
		let vtx_vault_asset_balances_bounded =
			BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap();
		assert_ok!(Vortex::set_vtx_vault_asset_balances(
			Origin::root(),
			vortex_dist_id,
			vtx_vault_asset_balances_bounded.clone(),
		));

		// Check for the SetVtxVaultAssetBalances event
		System::assert_last_event(MockEvent::Vortex(crate::Event::SetVtxVaultAssetBalances {
			id: vortex_dist_id,
			assets_balances: vtx_vault_asset_balances_bounded,
		}));
	});
}

#[test]
fn set_vtx_vault_asset_balances_fails() {
	let alice: AccountId = create_account(1);
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		// create 2 tokens
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&alice, None).unwrap();
		// set fee pot asset balances
		let vtx_vault_asset_balances = vec![(usdc, 10), (weth, 10), (ROOT_ASSET_ID, 10)];
		let mut vtx_vault_asset_balances_bounded =
			BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap();

		// fails if not authorized account
		assert_noop!(
			Vortex::set_vtx_vault_asset_balances(
				Origin::signed(create_account(5)),
				vortex_dist_id,
				vtx_vault_asset_balances_bounded,
			),
			Error::<Test>::RequireAdmin
		);

		// fail if Vtx asset id is included
		vtx_vault_asset_balances_bounded = BoundedVec::try_from(vec![
			(<Test as Config>::VtxAssetId::get(), 10),
			(weth, 10),
			(ROOT_ASSET_ID, 10),
		])
		.unwrap();
		assert_noop!(
			Vortex::set_vtx_vault_asset_balances(
				Origin::root(),
				vortex_dist_id,
				vtx_vault_asset_balances_bounded,
			),
			Error::<Test>::AssetsShouldNotIncludeVtxAsset
		);
	});
}

#[test]
fn set_vtx_total_supply_works() {
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));
		let vtx_current_supply = 1_000;
		assert_ok!(Vortex::set_vtx_total_supply(
			Origin::root(),
			vortex_dist_id,
			vtx_current_supply,
		));

		// Check for the SetVtxTotalSupply event
		System::assert_last_event(MockEvent::Vortex(crate::Event::SetVtxTotalSupply {
			id: vortex_dist_id,
			total_supply: vtx_current_supply,
		}));
	});
}

#[test]
fn set_vtx_total_supply_fails() {
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));
		let vtx_current_supply = 1_000;
		assert_noop!(
			Vortex::set_vtx_total_supply(
				Origin::signed(create_account(5)),
				vortex_dist_id,
				vtx_current_supply,
			),
			Error::<Test>::RequireAdmin
		);
	});
}

#[test]
fn register_reward_points_works() {
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));
		// register reward points
		let reward_points = BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();
		assert_ok!(Vortex::register_reward_points(
			Origin::root(),
			vortex_dist_id,
			reward_points.clone()
		));

		// Check for the VtxRewardPointRegistered event
		System::assert_last_event(MockEvent::Vortex(crate::Event::VtxRewardPointRegistered {
			id: vortex_dist_id,
			reward_points,
		}));
	});
}

#[test]
fn register_reward_points_fails() {
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));
		// register reward points
		let reward_points = BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();

		// fails if not authorized account
		assert_noop!(
			Vortex::register_reward_points(
				Origin::signed(bob),
				vortex_dist_id,
				reward_points.clone()
			),
			Error::<Test>::RequireAdmin
		);

		// fails if status != VtxDistStatus::Enabled
		// disable the vortex_dist_id
		assert_ok!(Vortex::disable_vtx_dist(Origin::root(), vortex_dist_id));
		assert_eq!(VtxDistStatuses::<Test>::get(vortex_dist_id), VtxDistStatus::Disabled);
		assert_noop!(
			Vortex::register_reward_points(Origin::root(), vortex_dist_id, reward_points.clone()),
			Error::<Test>::VtxDistDisabled
		);
	});
}

#[test]
fn register_work_points_works() {
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));
		// register work points
		let work_points = BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();
		assert_ok!(Vortex::register_work_points(
			Origin::root(),
			vortex_dist_id,
			work_points.clone()
		));

		// Check for the VtxWorkPointRegistered event
		System::assert_last_event(MockEvent::Vortex(crate::Event::VtxWorkPointRegistered {
			id: vortex_dist_id,
			work_points,
		}));
	});
}

#[test]
fn register_work_points_fails() {
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));
		// register work points
		let work_points = BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();

		// fails if not authorized account
		assert_noop!(
			Vortex::register_reward_points(
				Origin::signed(bob),
				vortex_dist_id,
				work_points.clone()
			),
			Error::<Test>::RequireAdmin
		);

		// fails if status != VtxDistStatus::Enabled
		// disable the vortex_dist_id
		assert_ok!(Vortex::disable_vtx_dist(Origin::root(), vortex_dist_id));
		assert_eq!(VtxDistStatuses::<Test>::get(vortex_dist_id), VtxDistStatus::Disabled);
		assert_noop!(
			Vortex::register_reward_points(Origin::root(), vortex_dist_id, work_points.clone()),
			Error::<Test>::VtxDistDisabled
		);
	});
}

#[test]
fn set_consider_current_balance_works() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);
		let consider_current_balance = true;
		assert_ok!(Vortex::set_consider_current_balance(Origin::root(), consider_current_balance));
		System::assert_last_event(MockEvent::Vortex(crate::Event::SetConsiderCurrentBalance {
			value: consider_current_balance,
		}));

		assert_eq!(ConsiderCurrentBalance::<Test>::get(), consider_current_balance);
	});
}

#[test]
fn set_consider_current_balance_fails_without_approved_origin() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);
		let consider_current_balance = true;
		assert_noop!(
			Vortex::set_consider_current_balance(
				Origin::signed(create_account(3)),
				consider_current_balance
			),
			Error::<Test>::RequireAdmin
		);
	});
}

#[test]
fn set_disable_redeem_works() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);
		let disable_redeem = true;
		assert_ok!(Vortex::set_disable_redeem(Origin::root(), disable_redeem));
		System::assert_last_event(MockEvent::Vortex(crate::Event::SetDisableRedeem {
			value: disable_redeem,
		}));

		assert_eq!(DisableRedeem::<Test>::get(), disable_redeem);
	});
}

#[test]
fn set_disable_redeem_fails_without_approved_origin() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);
		let disable_redeem = true;
		assert_noop!(
			Vortex::set_disable_redeem(Origin::signed(create_account(3)), disable_redeem),
			Error::<Test>::RequireAdmin
		);
	});
}

#[test]
fn set_asset_prices_should_work() {
	let alice: AccountId = create_account(1);

	TestExt::default().build().execute_with(|| {
		// Retrieve the ID of the newly created vortex distribution.
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		// create 2 tokens
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&alice, None).unwrap();

		// set fee pot asset balances
		let fee_pot_asset_balances = vec![(usdc, 100), (weth, 100), (ROOT_ASSET_ID, 100)];
		assert_ok!(Vortex::set_fee_pot_asset_balances(
			Origin::root(),
			vortex_dist_id,
			BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
		));

		//set asset price
		let asset_prices = vec![(usdc, 100), (weth, 200), (ROOT_ASSET_ID, 100)];
		let asset_prices_bounded = BoundedVec::try_from(asset_prices.clone()).unwrap();
		assert_ok!(Vortex::set_asset_prices(
			Origin::root(),
			vortex_dist_id,
			asset_prices_bounded.clone()
		));

		// Check that the correct event was emitted.
		System::assert_last_event(MockEvent::Vortex(crate::Event::SetAssetPrices {
			id: vortex_dist_id,
			asset_prices: asset_prices_bounded,
		}));
	});
}

#[test]
fn set_asset_prices_with_invalid_asset_id_should_fail() {
	let alice: AccountId = create_account(1);
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		// create 2 tokens
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&alice, None).unwrap();

		// set fee pot asset balances
		let fee_pot_asset_balances = vec![(usdc, 100), (weth, 100), (ROOT_ASSET_ID, 100)];
		assert_ok!(Vortex::set_fee_pot_asset_balances(
			Origin::root(),
			vortex_dist_id,
			BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
		));

		// set asset price
		// asset_prices vector includes (120, 100) where asset id 120 is not in the fee pot balances
		let asset_prices = vec![(usdc, 100), (weth, 200), (ROOT_ASSET_ID, 100), (120, 100)];
		let asset_prices_bounded = BoundedVec::try_from(asset_prices.clone()).unwrap();
		assert_noop!(
			Vortex::set_asset_prices(Origin::root(), vortex_dist_id, asset_prices_bounded),
			Error::<Test>::AssetNotInFeePotList
		);

		// set asset price
		// asset_prices vector includes VTX_ASSET_ID which is invalid
		let invalid_asset_prices = vec![(VTX_ASSET_ID, 500)];
		let bounded_invalid_asset_prices = BoundedVec::try_from(invalid_asset_prices).unwrap();
		assert_noop!(
			Vortex::set_asset_prices(Origin::root(), vortex_dist_id, bounded_invalid_asset_prices),
			Error::<Test>::AssetsShouldNotIncludeVtxAsset
		);
	});
}

#[test]
fn set_asset_prices_without_permission_should_fail() {
	let alice: AccountId = create_account(1);
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));

		// create 2 tokens
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&alice, None).unwrap();

		// set fee pot asset balances
		let fee_pot_asset_balances = vec![(usdc, 100), (weth, 100), (ROOT_ASSET_ID, 100)];
		assert_ok!(Vortex::set_fee_pot_asset_balances(
			Origin::root(),
			vortex_dist_id,
			BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
		));

		//set asset price
		let asset_prices = vec![(usdc, 100), (weth, 200), (ROOT_ASSET_ID, 100)];
		let asset_prices_bounded = BoundedVec::try_from(asset_prices.clone()).unwrap();

		// Attempt to set asset prices without the required permissions.
		assert_noop!(
			Vortex::set_asset_prices(
				Origin::signed(create_account(2)),
				vortex_dist_id,
				asset_prices_bounded,
			),
			crate::Error::<Test>::RequireAdmin
		);
	});
}

#[test]
fn trigger_vtx_distribution_works() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	TestExt::default()
		.with_balances(&[(alice, 2_000_000)])
		.with_asset(<Test as crate::Config>::VtxAssetId::get(), "VORTEX", &[(charlie, 5)])
		.build()
		.execute_with(|| {
			// create 2 tokens
			let usdc = AssetsExt::create(&alice, None).unwrap();
			let weth = AssetsExt::create(&alice, None).unwrap();

			let root_price: Balance = 3;

			let vortex_dist_id = NextVortexId::<Test>::get();

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
				vortex_dist_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			let vtx_current_supply = 5;
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dist_id,
				vtx_current_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances =
				vec![(usdc, 1_000_000), (weth, 1_000_000), (ROOT_ASSET_ID, 1_000_000)];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price
			let asset_prices = vec![(usdc, 100), (weth, 200), (ROOT_ASSET_ID, root_price)];
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(asset_prices.clone()).unwrap(),
			));

			// register reward and work points
			let reward_points =
				BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();
			let work_points = BoundedVec::try_from(vec![(bob, 10), (charlie, 10)]).unwrap();
			assert_ok!(Vortex::register_reward_points(
				Origin::root(),
				vortex_dist_id,
				reward_points
			));
			assert_ok!(Vortex::register_work_points(Origin::root(), vortex_dist_id, work_points));

			// check fee pot and bootstrap root account has correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), bootstrap_root);
			assert_eq!(AssetsExt::balance(usdc, &fee_vault), 1_000_000);
			assert_eq!(AssetsExt::balance(weth, &fee_vault), 1_000_000);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), 1_000_000);

			// trigger vortex distribution and do the preparations for distribution
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id));
			// Check that the correct event was emitted.
			System::assert_last_event(MockEvent::Vortex(crate::Event::TriggerVtxDistribution {
				id: vortex_dist_id,
			}));

			// check balances have been transferred to vtx vault account
			// check fee pot and bootstrap root account has correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), 0);
			assert_eq!(AssetsExt::balance(usdc, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(weth, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), 0);
			let vtx_vault_account = Vortex::get_vtx_vault_account();
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &vtx_vault_account),
				bootstrap_root + 1_000_000
			);
			assert_eq!(AssetsExt::balance(usdc, &vtx_vault_account), 1_000_000);
			assert_eq!(AssetsExt::balance(weth, &vtx_vault_account), 1_000_000);

			// check VtxPrice tally
			let vtx_price_calculted =
				calculate_vtx_price(&vtx_vault_asset_balances, &asset_prices, vtx_current_supply);
			assert_eq!(VtxPrice::<Test>::get(vortex_dist_id), vtx_price_calculted);
			// check vtx amounts tally
			let (total_vortex_network_reward, total_vortex_bootstrap, total_vortex) = calculate_vtx(
				&fee_pot_asset_balances,
				&asset_prices,
				bootstrap_root,
				root_price,
				vtx_price_calculted,
			);
			assert_eq!(TotalVortex::<Test>::get(vortex_dist_id), total_vortex);
			assert_eq!(
				TotalNetworkReward::<Test>::get(vortex_dist_id),
				total_vortex_network_reward
			);
			assert_eq!(TotalBootstrapReward::<Test>::get(vortex_dist_id), total_vortex_bootstrap);

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
				VtxDistOrderbook::<Test>::get(vortex_dist_id, bob),
				(bob_vtx_reward_calculated, false)
			);
		});
}

#[test]
fn trigger_vtx_distribution_should_fail_if_already_triggered() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	TestExt::default()
		.with_balances(&[(alice, 2_000_000)])
		.with_asset(<Test as crate::Config>::VtxAssetId::get(), "VORTEX", &[(charlie, 5)])
		.build()
		.execute_with(|| {
			// create 2 tokens
			let usdc = AssetsExt::create(&alice, None).unwrap();
			let weth = AssetsExt::create(&alice, None).unwrap();

			let root_price: Balance = 3;

			let vortex_dist_id = NextVortexId::<Test>::get();

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
				vortex_dist_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			let vtx_current_supply = 5;
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dist_id,
				vtx_current_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances = vec![(usdc, 10), (weth, 10), (ROOT_ASSET_ID, 10)];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price
			let asset_prices = vec![(usdc, 100), (weth, 200), (ROOT_ASSET_ID, root_price)];
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(asset_prices.clone()).unwrap(),
			));

			// register reward and work points
			let reward_points =
				BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();
			let work_points = BoundedVec::try_from(vec![(bob, 10), (charlie, 10)]).unwrap();
			assert_ok!(Vortex::register_reward_points(
				Origin::root(),
				vortex_dist_id,
				reward_points
			));
			assert_ok!(Vortex::register_work_points(Origin::root(), vortex_dist_id, work_points));

			// check fee pot and bootstrap root account has correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), bootstrap_root);
			assert_eq!(AssetsExt::balance(usdc, &fee_vault), 1_000_000);
			assert_eq!(AssetsExt::balance(weth, &fee_vault), 1_000_000);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), 1_000_000);

			// trigger vortex distribution and do the preparations for distribution
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id));
			// Check that the correct event was emitted.
			System::assert_last_event(MockEvent::Vortex(crate::Event::TriggerVtxDistribution {
				id: vortex_dist_id,
			}));

			// Trigger again should fail
			assert_noop!(
				Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id),
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

// trigger_vtx_distribution_should_fail_vortex_price_zero
// trigger_vtx_distribution_should_fail_root_price_zero

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

#[test]
fn start_vtx_dist_success() {
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

			let vortex_dist_id = NextVortexId::<Test>::get();

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
				vortex_dist_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			let vtx_current_supply = 5;
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dist_id,
				vtx_current_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances = vec![(usdc, 10), (weth, 10), (ROOT_ASSET_ID, 10)];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price
			let asset_prices = vec![(usdc, 100), (weth, 200), (ROOT_ASSET_ID, root_price)];
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(asset_prices.clone()).unwrap(),
			));

			// register reward and work points
			let reward_points =
				BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();
			let work_points = BoundedVec::try_from(vec![(bob, 10), (charlie, 10)]).unwrap();
			assert_ok!(Vortex::register_reward_points(
				Origin::root(),
				vortex_dist_id,
				reward_points
			));
			assert_ok!(Vortex::register_work_points(Origin::root(), vortex_dist_id, work_points));

			// trigger vortex distribution and do the preparations for distribution
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id));

			// check VtxPrice tally
			let vtx_price_calculted =
				calculate_vtx_price(&vtx_vault_asset_balances, &asset_prices, vtx_current_supply);
			assert_eq!(VtxPrice::<Test>::get(vortex_dist_id), vtx_price_calculted);
			// check vtx amounts tally
			let (total_vortex_network_reward, total_vortex_bootstrap, total_vortex) = calculate_vtx(
				&fee_pot_asset_balances,
				&asset_prices,
				bootstrap_root,
				root_price,
				vtx_price_calculted,
			);
			assert_eq!(TotalVortex::<Test>::get(vortex_dist_id), total_vortex);
			assert_eq!(
				TotalNetworkReward::<Test>::get(vortex_dist_id),
				total_vortex_network_reward
			);
			assert_eq!(TotalBootstrapReward::<Test>::get(vortex_dist_id), total_vortex_bootstrap);

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
				VtxDistOrderbook::<Test>::get(vortex_dist_id, bob),
				(bob_vtx_reward_calculated, false)
			);

			//start the vortex distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dist_id,));
			// Check that the correct event was emitted.
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistStarted {
				id: vortex_dist_id,
			}));
			let vtx_held_vault = Vortex::get_vtx_held_account();
			assert_eq!(
				AssetsExt::balance(<Test as Config>::VtxAssetId::get(), &vtx_held_vault),
				total_vortex
			);
			run_to_block(end_block);
			assert_ok!(Vortex::pay_unsigned(Origin::none(), vortex_dist_id, end_block));
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
				VtxDistOrderbook::<Test>::get(vortex_dist_id, bob),
				(bob_vtx_reward_calculated, true)
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
			for i in 0..2000 {
				reward_points_vec.push((create_account(i + 4), 100_000));
				total_reward_points += 100_000;
			}
			let reward_points = BoundedVec::try_from(reward_points_vec).unwrap();

			let mut work_points_vec = vec![(bob, 10), (charlie, 10)];
			let mut total_work_points = 10 + 10;
			for i in 0..2000 {
				work_points_vec.push((create_account(i + 4), 10));
				total_work_points += 10;
			}
			let work_points = BoundedVec::try_from(work_points_vec).unwrap();
			assert_ok!(Vortex::register_reward_points(
				Origin::root(),
				vortex_dis_id,
				reward_points.clone()
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

			// check if the last account entry balance
			let last_account_entry = create_account(1999 + 4);
			let last_account_entry_vtx_balance_before =
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &last_account_entry);
			assert_eq!(last_account_entry_vtx_balance_before, 0);

			//start the vortex distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dis_id,));

			// reset events
			System::reset_events();
			let num_reward_accounts = reward_points.len();
			let num_reward_registered_accounts =
				VtxDistOrderbook::<Test>::iter_prefix(vortex_dis_id).count();
			assert_eq!(num_reward_registered_accounts, num_reward_accounts);
			// run pay_unsigned one time, assert not everybody got the rewards at first run
			let mut acconts_got_paid = vec![];
			assert_ok!(Vortex::pay_unsigned(Origin::none(), vortex_dis_id, System::block_number()));
			// Iterate VtxDistPaidOut events
			System::events().iter().for_each(|record| match record.event {
				MockEvent::Vortex(crate::Event::VtxDistPaidOut { who, .. }) => {
					acconts_got_paid.push(who)
				},
				_ => {},
			});
			// assert that not everybody got the rewards at first run
			assert!(acconts_got_paid.len() < num_reward_accounts);
			let mut dist_done = false;

			while System::block_number() < end_block {
				if dist_done {
					break;
				}
				System::reset_events();
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

				System::events().iter().for_each(|record| match record.event {
					MockEvent::Vortex(crate::Event::VtxDistPaidOut { who, .. }) => {
						acconts_got_paid.push(who)
					},
					MockEvent::Vortex(Event::VtxDistDone { .. }) => {
						assert_eq!(acconts_got_paid.len(), num_reward_accounts);
						dist_done = true;
					},
					_ => {},
				});
			}

			// check VtxDistStatuses status
			assert_eq!(VtxDistStatuses::<Test>::get(vortex_dis_id), VtxDistStatus::Done);
			// check the number of accounts that got rewards
			assert_eq!(acconts_got_paid.len(), num_reward_accounts);

			// check bob received the reward
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob),
				bob_vtx_reward_calculated
			);
			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dis_id, bob),
				(bob_vtx_reward_calculated, true)
			);

			// check if the last account entry got reward
			let last_account_entry_vtx_balance_after =
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &last_account_entry);
			println!(
				"last_account_entry_vtx_balance_after: {:?}",
				last_account_entry_vtx_balance_after
			);
			assert_ne!(last_account_entry_vtx_balance_after, 0);
			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dis_id, last_account_entry),
				(last_account_entry_vtx_balance_after, true)
			);

			assert_eq!(
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get()),
				total_vortex + vtx_current_supply
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
				Vortex::redeem_tokens_from_vault(Origin::signed(bob), 1200),
				Error::<Test>::InvalidAmount
			);
		});
}

#[test]
fn redeem_tokens_from_vault_works() {
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
			assert_ok!(AssetsExt::mint_into(usdc, &alice, 10_000_000));
			assert_ok!(AssetsExt::mint_into(weth, &alice, 10_000_000));

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
			assert_ok!(Vortex::safe_transfer(usdc, &alice, &fee_vault, 10_000_000, false));
			assert_ok!(Vortex::safe_transfer(weth, &alice, &fee_vault, 10_000_000, false));
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
			let fee_pot_asset_balances =
				vec![(usdc, 10_000_000), (weth, 10_000_000), (ROOT_ASSET_ID, 100)];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dis_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price
			let asset_prices =
				vec![(usdc, 1_000_000), (weth, 2_000_000), (ROOT_ASSET_ID, root_price)];
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				vortex_dis_id,
				BoundedVec::try_from(asset_prices.clone()).unwrap(),
			));

			// register reward and work points
			let reward_points_vec = vec![(bob, 1000_000), (charlie, 100_000)];
			let total_reward_points = 1000_000 + 100_000;
			let reward_points = BoundedVec::try_from(reward_points_vec).unwrap();

			let work_points_vec = vec![(bob, 100), (charlie, 10)];
			let total_work_points = 100 + 10;
			let work_points = BoundedVec::try_from(work_points_vec).unwrap();
			assert_ok!(Vortex::register_reward_points(
				Origin::root(),
				vortex_dis_id,
				reward_points.clone()
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
				Perbill::from_rational(10_00_000_u128, total_reward_points);
			let bob_work_points_portion = Perbill::from_rational(100_u128, total_work_points);
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

			//set the VtxVaultRedeemAssetList
			let vtx_redeem_asset_list =
				BoundedVec::try_from(vec![usdc, weth, ROOT_ASSET_ID]).unwrap();
			assert_ok!(Vortex::set_vtx_vault_redeem_asset_list(
				Origin::root(),
				vtx_redeem_asset_list.clone()
			));

			//start the vortex distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dis_id,));

			// reset events
			System::reset_events();
			let num_reward_accounts = reward_points.len();
			let num_reward_registered_accounts =
				VtxDistOrderbook::<Test>::iter_prefix(vortex_dis_id).count();
			assert_eq!(num_reward_registered_accounts, num_reward_accounts);
			// run pay_unsigned one time, assert not everybody got the rewards at first run
			let mut acconts_got_paid = vec![];
			assert_ok!(Vortex::pay_unsigned(Origin::none(), vortex_dis_id, System::block_number()));
			// Iterate VtxDistPaidOut events
			System::events().iter().for_each(|record| match record.event {
				MockEvent::Vortex(crate::Event::VtxDistPaidOut { who, .. }) => {
					acconts_got_paid.push(who)
				},
				_ => {},
			});

			let mut dist_done = false;
			while System::block_number() < end_block {
				if dist_done {
					break;
				}
				System::reset_events();
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

				System::events().iter().for_each(|record| match record.event {
					MockEvent::Vortex(crate::Event::VtxDistPaidOut { who, .. }) => {
						acconts_got_paid.push(who)
					},
					MockEvent::Vortex(Event::VtxDistDone { .. }) => {
						assert_eq!(acconts_got_paid.len(), num_reward_accounts);
						dist_done = true;
					},
					_ => {},
				});
			}

			// check VtxDistStatuses status
			assert_eq!(VtxDistStatuses::<Test>::get(vortex_dis_id), VtxDistStatus::Done);
			// check the number of accounts that got rewards
			assert_eq!(acconts_got_paid.len(), num_reward_accounts);

			// check bob received the reward
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob),
				bob_vtx_reward_calculated
			);
			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dis_id, bob),
				(bob_vtx_reward_calculated, true)
			);

			assert_eq!(
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get()),
				total_vortex + vtx_current_supply
			);

			// Try redeem
			println!("bob_vtx_reward_calculated: {:?}", bob_vtx_reward_calculated);
			let current_total_vortex =
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get());
			let vtx_redeem_asset_balances = vtx_redeem_asset_list
				.iter()
				.map(|asset_id| {
					let amount = AssetsExt::balance(*asset_id, &Vortex::get_vtx_vault_account());
					(*asset_id, amount)
				})
				.collect::<Vec<_>>();

			assert_ok!(Vortex::redeem_tokens_from_vault(
				Origin::signed(bob),
				bob_vtx_reward_calculated
			));
			//check Bob's balances
			let bob_redeem_amounts = calculate_vtx_redeem(
				&vtx_redeem_asset_balances,
				bob_vtx_reward_calculated,
				current_total_vortex,
			);
			for (asset_id, amount) in bob_redeem_amounts {
				if asset_id == ROOT_ASSET_ID {
					assert_eq!(Balances::free_balance(bob), amount);
				} else {
					assert_eq!(AssetsExt::balance(asset_id, &bob), amount);
				}
			}
			assert_eq!(AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob), 0);
		});
}

// given the required inputs, this will print the reward breakdown for the specified account
// run with "cargo test -p pallet-vortex-distribution "print_reward_details_for_account" -- --nocapture"
#[test]
fn print_reward_details_for_account() {
	// inputs
	let vtx_vault_asset_balances = vec![(1, 100_000_000), (2, 10_000_000)];
	let asset_prices = vec![(1, 16_140), (2, 2_461_300)];
	let vtx_current_supply = 10_000_000;
	let fee_pot_asset_balances = vec![(1, 200_000_000), (2, 10_000_000)];
	let bootstrap_root = 3_000_000;
	let root_price = asset_prices[0].1;
	let account = AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"));
	let account_staker_reward_points = 2_000_000_u128;
	let account_worker_points = 10_000_000_u128;
	let total_staker_reward_points = 3_000_000;
	let total_worker_points = 11_000_000;

	// check VtxPrice tally
	let vtx_price_calculted =
		calculate_vtx_price(&vtx_vault_asset_balances, &asset_prices, vtx_current_supply);
	println!("vtx_price_calculted: {:?}", vtx_price_calculted);
	// check vtx amounts tally
	let (total_vortex_network_reward, total_vortex_bootstrap, total_vortex) = calculate_vtx(
		&fee_pot_asset_balances,
		&asset_prices,
		bootstrap_root,
		root_price,
		vtx_price_calculted,
	);
	println!("total_vortex_network_reward: {:?}", total_vortex_network_reward);
	println!("total_vortex_bootstrap: {:?}", total_vortex_bootstrap);
	println!("total_vortex: {:?}", total_vortex);

	// check vortex reward for the account
	let staker_pool =
		total_vortex_bootstrap + (Perbill::from_percent(30) * total_vortex_network_reward);
	let workpoint_pool = Perbill::from_percent(70) * total_vortex_network_reward;
	println!("staker_pool: {:?}", staker_pool);
	println!("workpoint_pool: {:?}", workpoint_pool);

	let account_staker_point_portion =
		Perbill::from_rational(account_staker_reward_points, total_staker_reward_points);
	let account_work_points_portion =
		Perbill::from_rational(account_worker_points, total_worker_points);
	println!("account_staker_point_portion: {:?}", account_staker_point_portion);
	println!("account_work_points_portion: {:?}", account_work_points_portion);

	let account_vtx_reward_calculated = (account_staker_point_portion * staker_pool)
		+ (account_work_points_portion * workpoint_pool);
	println!("account_vtx_reward_calculated: {:?}", account_vtx_reward_calculated);
}

#[test]
fn set_enable_manual_reward_input_works() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);
		let enable_manual_reward_input = true;
		assert_ok!(Vortex::set_enable_manual_reward_input(
			Origin::root(),
			enable_manual_reward_input
		));
		System::assert_last_event(MockEvent::Vortex(crate::Event::SetEnableManualRewardInput {
			value: enable_manual_reward_input,
		}));

		assert_eq!(EnableManualRewardInput::<Test>::get(), enable_manual_reward_input);
	});
}

#[test]
fn set_enable_manual_reward_input_fails_without_approved_origin() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);
		let enable_manual_reward_input = true;
		assert_noop!(
			Vortex::set_enable_manual_reward_input(
				Origin::signed(create_account(3)),
				enable_manual_reward_input
			),
			Error::<Test>::RequireAdmin
		);
	});
}

#[test]
fn register_rewards_works() {
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));
		assert_ok!(Vortex::set_enable_manual_reward_input(RawOrigin::Root.into(), true));
		// register reward points
		let rewards = BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();
		assert_ok!(Vortex::register_rewards(Origin::root(), vortex_dist_id, rewards.clone()));

		// Check for the RewardRegistered event
		System::assert_last_event(MockEvent::Vortex(crate::Event::RewardRegistered {
			id: vortex_dist_id,
			rewards,
		}));
	});
}

#[test]
fn register_rewards_fails() {
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));
		// register reward points
		let reward_points = BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();

		// fails if not authorized account
		assert_noop!(
			Vortex::register_rewards(Origin::signed(bob), vortex_dist_id, reward_points.clone()),
			Error::<Test>::RequireAdmin
		);

		// fails if EnableManualRewardInput is set to false
		assert_noop!(
			Vortex::register_rewards(Origin::root(), vortex_dist_id, reward_points.clone()),
			Error::<Test>::ManualRewardInputDisabled
		);

		assert_ok!(Vortex::set_enable_manual_reward_input(RawOrigin::Root.into(), true));
		// fails if status != VtxDistStatus::Enabled
		// disable the vortex_dist_id
		assert_ok!(Vortex::disable_vtx_dist(Origin::root(), vortex_dist_id));
		assert_eq!(VtxDistStatuses::<Test>::get(vortex_dist_id), VtxDistStatus::Disabled);
		assert_noop!(
			Vortex::register_rewards(Origin::root(), vortex_dist_id, reward_points.clone()),
			Error::<Test>::VtxDistDisabled
		);
	});
}

/*
#[test]
fn redeem_tokens_from_vault_should_work_without_root_token_in_asset_prices() {
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
			assert_ok!(AssetsExt::mint_into(usdc, &alice, 10_000_000));
			assert_ok!(AssetsExt::mint_into(weth, &alice, 10_000_000));

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
			assert_ok!(Vortex::safe_transfer(usdc, &alice, &fee_vault, 10_000_000, false));
			assert_ok!(Vortex::safe_transfer(weth, &alice, &fee_vault, 10_000_000, false));
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
			let fee_pot_asset_balances = vec![(usdc, 10_000_000), (weth, 10_000_000), (ROOT_ASSET_ID, 100)];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dis_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price
			let asset_prices = vec![(usdc, 1_000_000), (weth, 2_000_000)];
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				vortex_dis_id,
				BoundedVec::try_from(asset_prices.clone()).unwrap(),
			));

			// register reward and work points
			let mut reward_points_vec = vec![(bob, 1000_000), (charlie, 100_000)];
			let mut total_reward_points = 1000_000 + 100_000;
			let reward_points = BoundedVec::try_from(reward_points_vec).unwrap();

			let mut work_points_vec = vec![(bob, 100), (charlie, 10)];
			let mut total_work_points = 100 + 10;
			let work_points = BoundedVec::try_from(work_points_vec).unwrap();
			assert_ok!(Vortex::register_reward_points(
				Origin::root(),
				vortex_dis_id,
				reward_points.clone()
			));
			assert_ok!(Vortex::register_work_points(Origin::root(), vortex_dis_id, work_points));

			//trigger vortext reward calcuation and assets/root transfer to vault
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dis_id,));

			// check VtxPrice tally
			let mut asset_prices_with_root_zero = asset_prices;
			asset_prices_with_root_zero.extend_from_slice(&[(ROOT_ASSET_ID, root_price)]);
			let vtx_price_calculted =
				calculate_vtx_price(&vtx_vault_asset_balances, &asset_prices_with_root_zero, vtx_current_supply);
			assert_eq!(VtxPrice::<Test>::get(vortex_dis_id), vtx_price_calculted);
			println!("vtx_price_calculted: {:?}", vtx_price_calculted);
			// check vtx amounts tally
			let (total_vortex_network_reward, total_vortex_bootstrap, total_vortex) = calculate_vtx(
				&fee_pot_asset_balances,
				&asset_prices_with_root_zero,
				bootstrap_root,
				0,
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
				Perbill::from_rational(10_00_000_u128, total_reward_points);
			let bob_work_points_portion = Perbill::from_rational(100_u128, total_work_points);
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

			//set the VtxVaultRedeemAssetList
			let vtx_redeem_asset_list = BoundedVec::try_from(vec![usdc, weth, ROOT_ASSET_ID]).unwrap();
			assert_ok!(Vortex::set_vtx_vault_redeem_asset_list(
				Origin::root(),
				vtx_redeem_asset_list.clone()
			));

			//start the vortex distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dis_id,));

			// reset events
			System::reset_events();
			let num_reward_accounts = reward_points.len();
			let num_reward_registered_accounts =
				VtxDistOrderbook::<Test>::iter_prefix(vortex_dis_id).count();
			assert_eq!(num_reward_registered_accounts, num_reward_accounts);
			// run pay_unsigned one time, assert not everybody got the rewards at first run
			let mut acconts_got_paid = vec![];
			assert_ok!(Vortex::pay_unsigned(Origin::none(), vortex_dis_id, System::block_number()));
			// Iterate VtxDistPaidOut events
			System::events().iter().for_each(|record| match record.event {
				MockEvent::Vortex(crate::Event::VtxDistPaidOut { who, .. }) => {
					acconts_got_paid.push(who)
				},
				_ => {},
			});

			let mut dist_done = false;
			while System::block_number() < end_block {
				if dist_done {
					break;
				}
				System::reset_events();
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

				System::events().iter().for_each(|record| match record.event {
					MockEvent::Vortex(crate::Event::VtxDistPaidOut { who, .. }) => {
						acconts_got_paid.push(who)
					},
					MockEvent::Vortex(Event::VtxDistDone { .. }) => {
						assert_eq!(acconts_got_paid.len(), num_reward_accounts);
						dist_done = true;
					},
					_ => {},
				});
			}

			// check VtxDistStatuses status
			assert_eq!(VtxDistStatuses::<Test>::get(vortex_dis_id), VtxDistStatus::Done);
			// check the number of accounts that got rewards
			assert_eq!(acconts_got_paid.len(), num_reward_accounts);

			// check bob received the reward
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob),
				bob_vtx_reward_calculated
			);
			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dis_id, bob),
				(bob_vtx_reward_calculated, true)
			);

			assert_eq!(
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get()),
				total_vortex + vtx_current_supply
			);

			// Try redeem
			println!("bob_vtx_reward_calculated: {:?}", bob_vtx_reward_calculated);
			let current_total_vortex = AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get());
			let vtx_redeem_asset_balances = vtx_redeem_asset_list.iter().map(|asset_id| {
				let amount = AssetsExt::balance(*asset_id, &Vortex::get_vtx_vault_account());
				(*asset_id, amount)
			})
				.collect::<Vec<_>>();

			assert_ok!(Vortex::redeem_tokens_from_vault(
				Origin::signed(bob),
				bob_vtx_reward_calculated
			));
			//check Bob's balances
			let bob_redeem_amounts = calculate_vtx_redeem(&vtx_redeem_asset_balances, bob_vtx_reward_calculated, current_total_vortex);
			for (asset_id, amount) in bob_redeem_amounts {
				if asset_id == ROOT_ASSET_ID {
					assert_eq!(Balances::free_balance(bob), amount);
				} else {
					assert_eq!(AssetsExt::balance(asset_id, &bob), amount);
				}
			}
			assert_eq!(AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob), 0);
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
}
*/
