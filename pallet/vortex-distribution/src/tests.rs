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
	calculate_attribution_rewards, calculate_vtx, calculate_vtx_price, calculate_vtx_redeem,
	run_to_block, AssetsExt, Balances, MockPartnerAttribution, NativeAssetId,
	RuntimeEvent as MockEvent, RuntimeOrigin as Origin, System, Test, TestExt, Timestamp, Vortex,
	BLOCK_TIME,
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
		// register reward points as this is required for work points registration
		let reward_points = BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();
		assert_ok!(Vortex::register_reward_points(
			Origin::root(),
			vortex_dist_id,
			reward_points.clone()
		));
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
fn register_work_points_works_with_zero_reward_points_registered() {
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));
		// register reward points as this is required for work points registration
		let reward_points = BoundedVec::try_from(vec![(bob, 0), (charlie, 0)]).unwrap();
		assert_ok!(Vortex::register_reward_points(
			Origin::root(),
			vortex_dist_id,
			reward_points.clone()
		));
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
fn register_work_points_fails_if_no_reward_points_registered() {
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);
	TestExt::default().build().execute_with(|| {
		let vortex_dist_id = NextVortexId::<Test>::get();
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));
		// register work points
		let work_points = BoundedVec::try_from(vec![(bob, 100_000), (charlie, 100_000)]).unwrap();
		assert_noop!(
			Vortex::register_work_points(Origin::root(), vortex_dist_id, work_points.clone()),
			Error::<Test>::RewardPointsNotRegistered
		);
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
fn set_asset_prices_should_work_even_if_asset_id_is_not_in_fee_pot_list() {
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
		let asset_id_not_in_feepot_list = 100;
		let asset_prices = vec![
			(usdc, 100),
			(weth, 200),
			(ROOT_ASSET_ID, 100),
			(asset_id_not_in_feepot_list, 200),
		];
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

	// asset ids
	let usdc_asset_id = 10;
	let weth_asset_id = 11;

	// vortex vault pre asset balances
	let vortex_vault = Vortex::get_vtx_vault_account();
	let usdc_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let weth_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let root_vtx_vault_balance = 100_u128 * 10_u128.pow(6);
	let vtx_current_supply = 1_000_u128 * 10_u128.pow(6);

	// fee pot asset balance
	let fee_vault = Vortex::get_fee_vault_account();
	let usdc_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let weth_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let root_fee_pot_balance = 1_u128 * 10_u128.pow(6);

	// bootstrap balance
	let root_vault = Vortex::get_root_vault_account();
	let bootstrap_root = 100_u128 * 10_u128.pow(6);

	TestExt::default()
		.with_balances(&[
			(alice, 2_000_000),
			(fee_vault, root_fee_pot_balance),
			(vortex_vault, root_vtx_vault_balance),
			(root_vault, bootstrap_root),
		])
		.with_asset(
			<Test as crate::Config>::VtxAssetId::get(),
			"VORTEX",
			&[(charlie, vtx_current_supply)],
		)
		.with_asset_decimals(
			usdc_asset_id,
			"USDC",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, usdc_vtx_vault_balance),
				(fee_vault, usdc_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			weth_asset_id,
			"WETH",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, weth_vtx_vault_balance),
				(fee_vault, weth_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			ROOT_ASSET_ID,
			"ROOT",
			6,
			&[(alice, 1_000_000), (vortex_vault, root_vtx_vault_balance)], // this is just for the metadata
		)
		.build()
		.execute_with(|| {
			let vortex_dist_id = NextVortexId::<Test>::get();

			// check account have correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), bootstrap_root);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &vortex_vault),
				root_vtx_vault_balance
			);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &vortex_vault), usdc_vtx_vault_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &vortex_vault), weth_vtx_vault_balance);

			// create vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set vortex vault pre asset balances
			let vtx_vault_asset_balances = vec![
				(usdc_asset_id, usdc_vtx_vault_balance),
				(weth_asset_id, weth_vtx_vault_balance),
				(ROOT_ASSET_ID, root_vtx_vault_balance),
			];
			assert_ok!(Vortex::set_vtx_vault_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dist_id,
				vtx_current_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances = vec![
				(usdc_asset_id, usdc_fee_pot_balance),
				(weth_asset_id, weth_fee_pot_balance),
				(ROOT_ASSET_ID, root_fee_pot_balance),
			];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price. prices should be multiplied by the usd factor 10**6
			let usdc_price: Balance = 100_u128 * 10_u128.pow(6);
			let weth_price: Balance = 200_u128 * 10_u128.pow(6);
			let root_price: Balance = 3_u128 * 10_u128.pow(6);
			let asset_prices = vec![
				(usdc_asset_id, usdc_price),
				(weth_asset_id, weth_price),
				(ROOT_ASSET_ID, root_price),
			];
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
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);

			// trigger vortex distribution and do the preparations for distribution
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id));
			// Check that the correct event was emitted.
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggering {
				id: vortex_dist_id,
			}));
			// run a few blocks to move the Vtx status from Triggering to Triggered
			for i in 2_u32..4 {
				System::set_block_number(i.into());
				Vortex::on_idle(i.into(), Weight::from_all(1_000_000_000_000_u64));
			}
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggered {
				id: vortex_dist_id,
			}));

			// check balances have been transferred to vtx vault account
			// check fee pot and bootstrap root account has correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), 0);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), 0);
			let vtx_vault_account = Vortex::get_vtx_vault_account();
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &vtx_vault_account),
				root_vtx_vault_balance + bootstrap_root + root_fee_pot_balance
			);
			assert_eq!(
				AssetsExt::balance(usdc_asset_id, &vtx_vault_account),
				usdc_fee_pot_balance + usdc_vtx_vault_balance
			);
			assert_eq!(
				AssetsExt::balance(weth_asset_id, &vtx_vault_account),
				weth_fee_pot_balance + weth_vtx_vault_balance
			);

			// check VtxPrice tally
			let vtx_vault_asset_balances_with_decimals = vec![
				(usdc_asset_id, usdc_vtx_vault_balance, 18),
				(weth_asset_id, weth_vtx_vault_balance, 18),
				(ROOT_ASSET_ID, root_vtx_vault_balance, 6),
			];
			let vtx_price_calculted = calculate_vtx_price(
				&vtx_vault_asset_balances_with_decimals,
				&asset_prices,
				vtx_current_supply,
			);
			println!("vtx_price_calculted: {}", vtx_price_calculted);
			assert_eq!(VtxPrice::<Test>::get(vortex_dist_id), vtx_price_calculted);
			// check vtx amounts tally
			let fee_pot_asset_balances_with_decimals = vec![
				(usdc_asset_id, usdc_fee_pot_balance, 18),
				(weth_asset_id, weth_fee_pot_balance, 18),
				(ROOT_ASSET_ID, root_fee_pot_balance, 6),
			];
			let (total_vortex_network_reward, total_vortex_bootstrap, total_vortex) = calculate_vtx(
				&fee_pot_asset_balances_with_decimals,
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
			let staker_pool = total_vortex_bootstrap
				+ (Perquintill::from_percent(30) * total_vortex_network_reward);
			let workpoint_pool = Perquintill::from_percent(70) * total_vortex_network_reward;
			let bob_staker_point_portion =
				Perquintill::from_rational(100_000_u128, 100_000_u128 + 100_000_u128);
			let bob_work_points_portion = Perquintill::from_rational(10_u128, 10_u128 + 10_u128);
			let bob_vtx_reward_calculated = (bob_staker_point_portion * staker_pool)
				+ (bob_work_points_portion * workpoint_pool);
			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dist_id, bob),
				(bob_vtx_reward_calculated.div(PRECISION_MULTIPLIER), false)
			);
		});
}

#[test]
fn trigger_vtx_distribution_should_fail_if_already_triggered() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	// asset ids
	let usdc_asset_id = 10;
	let weth_asset_id = 11;

	// vortex vault pre asset balances
	let vortex_vault = Vortex::get_vtx_vault_account();
	let usdc_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let weth_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let root_vtx_vault_balance = 100_u128 * 10_u128.pow(6);
	let vtx_current_supply = 1_000_u128 * 10_u128.pow(6);

	// fee pot asset balance
	let fee_vault = Vortex::get_fee_vault_account();
	let usdc_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let weth_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let root_fee_pot_balance = 1_u128 * 10_u128.pow(6);

	// bootstrap balance
	let root_vault = Vortex::get_root_vault_account();
	let bootstrap_root = 100_u128 * 10_u128.pow(6);

	TestExt::default()
		.with_balances(&[
			(alice, 2_000_000),
			(fee_vault, root_fee_pot_balance),
			(vortex_vault, root_vtx_vault_balance),
			(root_vault, bootstrap_root),
		])
		.with_asset(
			<Test as crate::Config>::VtxAssetId::get(),
			"VORTEX",
			&[(charlie, vtx_current_supply)],
		)
		.with_asset_decimals(
			usdc_asset_id,
			"USDC",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, usdc_vtx_vault_balance),
				(fee_vault, usdc_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			weth_asset_id,
			"WETH",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, weth_vtx_vault_balance),
				(fee_vault, weth_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			ROOT_ASSET_ID,
			"ROOT",
			6,
			&[(alice, 1_000_000), (vortex_vault, root_vtx_vault_balance)], // this is just for the metadata
		)
		.build()
		.execute_with(|| {
			let vortex_dist_id = NextVortexId::<Test>::get();

			// check account have correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), bootstrap_root);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &vortex_vault),
				root_vtx_vault_balance
			);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &vortex_vault), usdc_vtx_vault_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &vortex_vault), weth_vtx_vault_balance);

			// create vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set vortex vault pre asset balances
			let vtx_vault_asset_balances = vec![
				(usdc_asset_id, usdc_vtx_vault_balance),
				(weth_asset_id, weth_vtx_vault_balance),
				(ROOT_ASSET_ID, root_vtx_vault_balance),
			];
			assert_ok!(Vortex::set_vtx_vault_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dist_id,
				vtx_current_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances = vec![
				(usdc_asset_id, usdc_fee_pot_balance),
				(weth_asset_id, weth_fee_pot_balance),
				(ROOT_ASSET_ID, root_fee_pot_balance),
			];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price. prices should be multiplied by the usd factor 10**6
			let usdc_price: Balance = 100_u128 * 10_u128.pow(6);
			let weth_price: Balance = 200_u128 * 10_u128.pow(6);
			let root_price: Balance = 3_u128 * 10_u128.pow(6);
			let asset_prices = vec![
				(usdc_asset_id, usdc_price),
				(weth_asset_id, weth_price),
				(ROOT_ASSET_ID, root_price),
			];
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
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);

			// trigger vortex distribution and do the preparations for distribution
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id));
			// Check that the correct event was emitted.
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggering {
				id: vortex_dist_id,
			}));
			// run a few blocks to move the Vtx status from Triggering to Triggered
			for i in 2_u32..4 {
				System::set_block_number(i.into());
				Vortex::on_idle(i.into(), Weight::from_all(1_000_000_000_000_u64));
			}
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggered {
				id: vortex_dist_id,
			}));

			// check balances have been transferred to vtx vault account
			// check fee pot and bootstrap root account has correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), 0);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), 0);
			let vtx_vault_account = Vortex::get_vtx_vault_account();
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &vtx_vault_account),
				root_vtx_vault_balance + bootstrap_root + root_fee_pot_balance
			);
			assert_eq!(
				AssetsExt::balance(usdc_asset_id, &vtx_vault_account),
				usdc_vtx_vault_balance + usdc_fee_pot_balance
			);
			assert_eq!(
				AssetsExt::balance(weth_asset_id, &vtx_vault_account),
				weth_vtx_vault_balance + weth_fee_pot_balance
			);

			// check VtxPrice tally
			let vtx_vault_asset_balances_with_decimals = vec![
				(usdc_asset_id, usdc_vtx_vault_balance, 18),
				(weth_asset_id, weth_vtx_vault_balance, 18),
				(ROOT_ASSET_ID, root_vtx_vault_balance, 6),
			];
			let vtx_price_calculted = calculate_vtx_price(
				&vtx_vault_asset_balances_with_decimals,
				&asset_prices,
				vtx_current_supply,
			);
			println!("vtx_price_calculted: {}", vtx_price_calculted);
			assert_eq!(VtxPrice::<Test>::get(vortex_dist_id), vtx_price_calculted);
			// check vtx amounts tally
			let fee_pot_asset_balances_with_decimals = vec![
				(usdc_asset_id, usdc_fee_pot_balance, 18),
				(weth_asset_id, weth_fee_pot_balance, 18),
				(ROOT_ASSET_ID, root_fee_pot_balance, 6),
			];
			let (total_vortex_network_reward, total_vortex_bootstrap, total_vortex) = calculate_vtx(
				&fee_pot_asset_balances_with_decimals,
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
			let staker_pool = total_vortex_bootstrap
				+ (Perquintill::from_percent(30) * total_vortex_network_reward);
			let workpoint_pool = Perquintill::from_percent(70) * total_vortex_network_reward;
			let bob_staker_point_portion =
				Perquintill::from_rational(100_000_u128, 100_000_u128 + 100_000_u128);
			let bob_work_points_portion = Perquintill::from_rational(10_u128, 10_u128 + 10_u128);
			let bob_vtx_reward_calculated = (bob_staker_point_portion * staker_pool)
				+ (bob_work_points_portion * workpoint_pool);
			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dist_id, bob),
				(bob_vtx_reward_calculated.div(PRECISION_MULTIPLIER), false)
			);
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
#[test]
fn trigger_vtx_distribution_should_fail_vortex_price_zero() {
	let alice: AccountId = create_account(1);

	TestExt::default()
		.with_balances(&[(alice, 2_000_000)])
		.build()
		.execute_with(|| {
			// Create a new vortex distribution
			let vortex_dist_id = NextVortexId::<Test>::get();
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// Create a token
			let usdc = AssetsExt::create(&alice, None).unwrap();

			// Set up asset balances with zero values
			let vtx_vault_asset_balances = vec![(usdc, 0), (ROOT_ASSET_ID, 0)];
			assert_ok!(Vortex::set_vtx_vault_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));

			// Set VTX total supply to a non-zero value
			let vtx_current_supply = 1_000_000;
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dist_id,
				vtx_current_supply,
			));

			// Set fee pot asset balances
			let fee_pot_asset_balances = vec![(usdc, 0), (ROOT_ASSET_ID, 0)];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			// Set asset prices
			let asset_prices = vec![(usdc, 0), (ROOT_ASSET_ID, 0)];
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(asset_prices.clone()).unwrap(),
			));

			// Attempt to trigger distribution - should fail because VTX price will be zero
			assert_noop!(
				Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id),
				crate::Error::<Test>::VortexPriceIsZero
			);
		});
}

#[test]
fn trigger_vtx_distribution_works_with_attributions() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);
	let partner1: AccountId = create_account(4);
	let partner2: AccountId = create_account(5);

	// asset ids
	let usdc_asset_id = 10;
	let weth_asset_id = 11;

	// vortex vault pre asset balances
	let vortex_vault = Vortex::get_vtx_vault_account();
	let usdc_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let weth_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let root_vtx_vault_balance = 100_u128 * 10_u128.pow(6);
	let xrp_vtx_vault_balance = 100_u128 * 10_u128.pow(6);
	let vtx_current_supply = 1_000_u128 * 10_u128.pow(6);

	// fee pot asset balance
	let fee_vault = Vortex::get_fee_vault_account();
	let usdc_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let weth_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let root_fee_pot_balance = 1_u128 * 10_u128.pow(6);
	let xrp_fee_pot_balance = 10_u128 * 10_u128.pow(6);

	// bootstrap balance
	let root_vault = Vortex::get_root_vault_account();
	let bootstrap_root = 100_u128 * 10_u128.pow(6);

	// attributions
	let attributions = vec![
		(partner1, 1_000_000, Some(Permill::from_percent(5))),
		(partner2, 2_000_000, Some(Permill::from_percent(10))),
	];

	TestExt::default()
		.with_balances(&[
			(alice, 2_000_000),
			(fee_vault, root_fee_pot_balance),
			(vortex_vault, root_vtx_vault_balance),
			(root_vault, bootstrap_root),
		])
		.with_asset(
			<Test as crate::Config>::VtxAssetId::get(),
			"VORTEX",
			&[(charlie, vtx_current_supply)],
		)
		.with_asset_decimals(
			usdc_asset_id,
			"USDC",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, usdc_vtx_vault_balance),
				(fee_vault, usdc_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			weth_asset_id,
			"WETH",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, weth_vtx_vault_balance),
				(fee_vault, weth_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			XRP_ASSET_ID,
			"XRP",
			6,
			&[(vortex_vault, xrp_vtx_vault_balance), (fee_vault, xrp_fee_pot_balance)],
		)
		.with_asset_decimals(
			ROOT_ASSET_ID,
			"ROOT",
			6,
			&[(alice, 1_000_000), (vortex_vault, root_vtx_vault_balance)], // this is just for the metadata
		)
		.with_attributions(&attributions)
		.build()
		.execute_with(|| {
			let vortex_dist_id = NextVortexId::<Test>::get();

			// check account have correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), bootstrap_root);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);
			assert_eq!(AssetsExt::balance(XRP_ASSET_ID, &fee_vault), xrp_fee_pot_balance);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &vortex_vault),
				root_vtx_vault_balance
			);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &vortex_vault), usdc_vtx_vault_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &vortex_vault), weth_vtx_vault_balance);

			// create vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set vortex vault pre asset balances
			let vtx_vault_asset_balances = vec![
				(usdc_asset_id, usdc_vtx_vault_balance),
				(weth_asset_id, weth_vtx_vault_balance),
				(ROOT_ASSET_ID, root_vtx_vault_balance),
				(XRP_ASSET_ID, xrp_vtx_vault_balance),
			];
			assert_ok!(Vortex::set_vtx_vault_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dist_id,
				vtx_current_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances = vec![
				(usdc_asset_id, usdc_fee_pot_balance),
				(weth_asset_id, weth_fee_pot_balance),
				(ROOT_ASSET_ID, root_fee_pot_balance),
				(XRP_ASSET_ID, xrp_fee_pot_balance),
			];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price. prices should be multiplied by the usd factor 10**6
			let usdc_price: Balance = 100_u128 * 10_u128.pow(6);
			let weth_price: Balance = 200_u128 * 10_u128.pow(6);
			let root_price: Balance = 3_u128 * 10_u128.pow(6);
			let xrp_price: Balance = 100_u128 * 10_u128.pow(6); // Example price for XRP
			let asset_prices = vec![
				(usdc_asset_id, usdc_price),
				(weth_asset_id, weth_price),
				(ROOT_ASSET_ID, root_price),
				(XRP_ASSET_ID, xrp_price), // Example asset ID for XRP
			];
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
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);
			assert_eq!(AssetsExt::balance(XRP_ASSET_ID, &fee_vault), xrp_fee_pot_balance);

			// trigger vortex distribution and do the preparations for distribution
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id));
			// Check that the correct events were emitted.
			System::assert_has_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggering {
				id: vortex_dist_id,
			}));
			System::assert_has_event(MockEvent::Vortex(crate::Event::PartnerAttributionsUpdated {
				vtx_id: vortex_dist_id,
			}));
			// run a few blocks to move the Vtx status from Triggering to Triggered
			for i in 2_u32..4 {
				System::set_block_number(i.into());
				Vortex::on_idle(i.into(), Weight::from_all(1_000_000_000_000_u64));
			}
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggered {
				id: vortex_dist_id,
			}));

			// check balances have been transferred to vtx vault account
			// check fee pot and bootstrap root account has correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), 0);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), 0);
			assert_eq!(AssetsExt::balance(XRP_ASSET_ID, &fee_vault), 0);
			let vtx_vault_account = Vortex::get_vtx_vault_account();
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &vtx_vault_account),
				root_vtx_vault_balance + bootstrap_root + root_fee_pot_balance
			);
			assert_eq!(
				AssetsExt::balance(usdc_asset_id, &vtx_vault_account),
				usdc_fee_pot_balance + usdc_vtx_vault_balance
			);
			assert_eq!(
				AssetsExt::balance(weth_asset_id, &vtx_vault_account),
				weth_fee_pot_balance + weth_vtx_vault_balance
			);
			assert_eq!(
				AssetsExt::balance(XRP_ASSET_ID, &vtx_vault_account),
				xrp_fee_pot_balance + xrp_vtx_vault_balance
			);

			// check VtxPrice tally
			let vtx_vault_asset_balances_with_decimals = vec![
				(usdc_asset_id, usdc_vtx_vault_balance, 18),
				(weth_asset_id, weth_vtx_vault_balance, 18),
				(ROOT_ASSET_ID, root_vtx_vault_balance, 6),
				(XRP_ASSET_ID, xrp_vtx_vault_balance, 6),
			];
			let vtx_price_calculted = calculate_vtx_price(
				&vtx_vault_asset_balances_with_decimals,
				&asset_prices,
				vtx_current_supply,
			);
			assert_eq!(VtxPrice::<Test>::get(vortex_dist_id), vtx_price_calculted);
			// check vtx amounts tally
			let fee_pot_asset_balances_with_decimals = vec![
				(usdc_asset_id, usdc_fee_pot_balance, 18),
				(weth_asset_id, weth_fee_pot_balance, 18),
				(ROOT_ASSET_ID, root_fee_pot_balance, 6),
				(XRP_ASSET_ID, xrp_fee_pot_balance, 6),
			];
			let (total_vortex_network_reward, total_vortex_bootstrap, total_vortex) = calculate_vtx(
				&fee_pot_asset_balances_with_decimals,
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

			// check attributions
			// attributions are noted onchain for future reference
			assert_eq!(PartnerAttributions::<Test>::get(vortex_dist_id), attributions);
			// check attributions on partner attribution pallet is reset
			assert_eq!(MockPartnerAttribution::get_current_attributions(), Vec::new());
			// calculate attribution rewards
			let attribution_rewards_calculated = calculate_attribution_rewards(
				&attributions,
				xrp_price,
				vtx_price_calculted,
				total_vortex_network_reward,
			);
			// assert with onchain value
			assert_eq!(
				PartnerAttributionRewards::<Test>::get(vortex_dist_id),
				attribution_rewards_calculated
			);
			let total_attribution_reward: Balance =
				attribution_rewards_calculated.iter().map(|(_, r)| r).sum();
			assert_eq!(
				total_attribution_reward,
				TotalAttributionRewards::<Test>::get(vortex_dist_id)
			);

			// check bob got the vortex reward registered
			let net_network_reward =
				total_vortex_network_reward.saturating_sub(total_attribution_reward);
			let staker_pool =
				total_vortex_bootstrap + (Perquintill::from_percent(30) * net_network_reward);
			let workpoint_pool = Perquintill::from_percent(70) * net_network_reward;
			let bob_staker_point_portion =
				Perquintill::from_rational(100_000_u128, 100_000_u128 + 100_000_u128);
			let bob_work_points_portion = Perquintill::from_rational(10_u128, 10_u128 + 10_u128);
			let bob_vtx_reward_calculated = (bob_staker_point_portion * staker_pool)
				+ (bob_work_points_portion * workpoint_pool);
			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dist_id, bob),
				(bob_vtx_reward_calculated.div(PRECISION_MULTIPLIER), false)
			);
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
fn start_vtx_dist_fails_during_reward_calculations_ongoing_period() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	// asset ids
	let usdc_asset_id = 10;
	let weth_asset_id = 11;

	// vortex vault pre asset balances
	let vortex_vault = Vortex::get_vtx_vault_account();
	let usdc_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let weth_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let root_vtx_vault_balance = 100_u128 * 10_u128.pow(6);
	let vtx_current_supply = 1_000_u128 * 10_u128.pow(6);

	// fee pot asset balance
	let fee_vault = Vortex::get_fee_vault_account();
	let usdc_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let weth_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let root_fee_pot_balance = 1_u128 * 10_u128.pow(6);

	// bootstrap balance
	let root_vault = Vortex::get_root_vault_account();
	let bootstrap_root = 100_u128 * 10_u128.pow(6);

	TestExt::default()
		.with_balances(&[
			(alice, 2_000_000),
			(fee_vault, root_fee_pot_balance),
			(vortex_vault, root_vtx_vault_balance),
			(root_vault, bootstrap_root),
		])
		.with_asset(
			<Test as crate::Config>::VtxAssetId::get(),
			"VORTEX",
			&[(charlie, vtx_current_supply)],
		)
		.with_asset_decimals(
			usdc_asset_id,
			"USDC",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, usdc_vtx_vault_balance),
				(fee_vault, usdc_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			weth_asset_id,
			"WETH",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, weth_vtx_vault_balance),
				(fee_vault, weth_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			ROOT_ASSET_ID,
			"ROOT",
			6,
			&[(alice, 1_000_000), (vortex_vault, root_vtx_vault_balance)], // this is just for the metadata
		)
		.build()
		.execute_with(|| {
			let vortex_dist_id = NextVortexId::<Test>::get();

			// check account have correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), bootstrap_root);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &vortex_vault),
				root_vtx_vault_balance
			);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &vortex_vault), usdc_vtx_vault_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &vortex_vault), weth_vtx_vault_balance);

			// create vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set vortex vault pre asset balances
			let vtx_vault_asset_balances = vec![
				(usdc_asset_id, usdc_vtx_vault_balance),
				(weth_asset_id, weth_vtx_vault_balance),
				(ROOT_ASSET_ID, root_vtx_vault_balance),
			];
			assert_ok!(Vortex::set_vtx_vault_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dist_id,
				vtx_current_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances = vec![
				(usdc_asset_id, usdc_fee_pot_balance),
				(weth_asset_id, weth_fee_pot_balance),
				(ROOT_ASSET_ID, root_fee_pot_balance),
			];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price. prices should be multiplied by the usd factor 10**6
			let usdc_price: Balance = 100_u128 * 10_u128.pow(6);
			let weth_price: Balance = 200_u128 * 10_u128.pow(6);
			let root_price: Balance = 3_u128 * 10_u128.pow(6);
			let asset_prices = vec![
				(usdc_asset_id, usdc_price),
				(weth_asset_id, weth_price),
				(ROOT_ASSET_ID, root_price),
			];
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
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);

			// trigger vortex distribution and do the preparations for distribution
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id));
			// Check that the correct event was emitted.
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggering {
				id: vortex_dist_id,
			}));

			// Verify distribution is in Triggering state and cannot be started
			assert_ne!(VtxDistStatuses::<Test>::get(vortex_dist_id), VtxDistStatus::Triggered);
			assert_eq!(VtxDistStatuses::<Test>::get(vortex_dist_id), VtxDistStatus::Triggering);

			// Attempt to start distribution during reward calculation - should fail
			assert_noop!(
				Vortex::start_vtx_dist(Origin::root(), vortex_dist_id),
				crate::Error::<Test>::NotTriggered
			);
		});
}

#[test]
fn start_vtx_dist_success() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	// asset ids
	let usdc_asset_id = 10;
	let weth_asset_id = 11;

	// vortex vault pre asset balances
	let vortex_vault = Vortex::get_vtx_vault_account();
	let usdc_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let weth_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let root_vtx_vault_balance = 100_u128 * 10_u128.pow(6);
	let vtx_current_supply = 1_000_u128 * 10_u128.pow(6);

	// fee pot asset balance
	let fee_vault = Vortex::get_fee_vault_account();
	let usdc_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let weth_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let root_fee_pot_balance = 1_u128 * 10_u128.pow(6);

	// bootstrap balance
	let root_vault = Vortex::get_root_vault_account();
	let bootstrap_root = 100_u128 * 10_u128.pow(6);

	TestExt::default()
		.with_balances(&[
			(alice, 2_000_000),
			(fee_vault, root_fee_pot_balance),
			(vortex_vault, root_vtx_vault_balance),
			(root_vault, bootstrap_root),
		])
		.with_asset(
			<Test as crate::Config>::VtxAssetId::get(),
			"VORTEX",
			&[(charlie, vtx_current_supply)],
		)
		.with_asset_decimals(
			usdc_asset_id,
			"USDC",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, usdc_vtx_vault_balance),
				(fee_vault, usdc_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			weth_asset_id,
			"WETH",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, weth_vtx_vault_balance),
				(fee_vault, weth_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			ROOT_ASSET_ID,
			"ROOT",
			6,
			&[(alice, 1_000_000), (vortex_vault, root_vtx_vault_balance)], // this is just for the metadata
		)
		.build()
		.execute_with(|| {
			let vortex_dist_id = NextVortexId::<Test>::get();

			// check account have correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), bootstrap_root);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &vortex_vault),
				root_vtx_vault_balance
			);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &vortex_vault), usdc_vtx_vault_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &vortex_vault), weth_vtx_vault_balance);

			// create vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set vortex vault pre asset balances
			let vtx_vault_asset_balances = vec![
				(usdc_asset_id, usdc_vtx_vault_balance),
				(weth_asset_id, weth_vtx_vault_balance),
				(ROOT_ASSET_ID, root_vtx_vault_balance),
			];
			assert_ok!(Vortex::set_vtx_vault_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dist_id,
				vtx_current_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances = vec![
				(usdc_asset_id, usdc_fee_pot_balance),
				(weth_asset_id, weth_fee_pot_balance),
				(ROOT_ASSET_ID, root_fee_pot_balance),
			];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price. prices should be multiplied by the usd factor 10**6
			let usdc_price: Balance = 100_u128 * 10_u128.pow(6);
			let weth_price: Balance = 200_u128 * 10_u128.pow(6);
			let root_price: Balance = 3_u128 * 10_u128.pow(6);
			let asset_prices = vec![
				(usdc_asset_id, usdc_price),
				(weth_asset_id, weth_price),
				(ROOT_ASSET_ID, root_price),
			];
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
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);

			// trigger vortex distribution and do the preparations for distribution
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id));
			// Check that the correct event was emitted.
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggering {
				id: vortex_dist_id,
			}));

			// Verify distribution is in Triggering state and cannot be started
			assert_ne!(VtxDistStatuses::<Test>::get(vortex_dist_id), VtxDistStatus::Triggered);
			assert_eq!(VtxDistStatuses::<Test>::get(vortex_dist_id), VtxDistStatus::Triggering);

			// run a few blocks to move the Vtx status from Triggering to Triggered
			for i in 2_u32..10 {
				System::set_block_number(i.into());
				Vortex::on_idle(i.into(), Weight::from_all(1_000_000_000_000_u64));
			}
			assert_eq!(VtxDistStatuses::<Test>::get(vortex_dist_id), VtxDistStatus::Triggered);

			// start the distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dist_id));
			// check the status is Paying
			assert_eq!(VtxDistStatuses::<Test>::get(vortex_dist_id), VtxDistStatus::Paying);
			// Check that the correct event was emitted.
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistStarted {
				id: vortex_dist_id,
			}));
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

	// asset ids
	let usdc_asset_id = 10;
	let weth_asset_id = 11;

	// vortex vault pre asset balances
	let vortex_vault = Vortex::get_vtx_vault_account();
	let usdc_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let weth_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let root_vtx_vault_balance = 100_u128 * 10_u128.pow(6);
	let vtx_current_supply = 1_000_u128 * 10_u128.pow(6);

	// fee pot asset balance
	let fee_vault = Vortex::get_fee_vault_account();
	let usdc_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let weth_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let root_fee_pot_balance = 1_u128 * 10_u128.pow(6);

	// bootstrap balance
	let root_vault = Vortex::get_root_vault_account();
	let bootstrap_root = 100_u128 * 10_u128.pow(6);

	let end_block = 1000;

	TestExt::default()
		.with_balances(&[
			(alice, 2_000_000),
			(fee_vault, root_fee_pot_balance),
			(vortex_vault, root_vtx_vault_balance),
			(root_vault, bootstrap_root),
		])
		.with_asset(
			<Test as crate::Config>::VtxAssetId::get(),
			"VORTEX",
			&[(charlie, vtx_current_supply)],
		)
		.with_asset_decimals(
			usdc_asset_id,
			"USDC",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, usdc_vtx_vault_balance),
				(fee_vault, usdc_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			weth_asset_id,
			"WETH",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, weth_vtx_vault_balance),
				(fee_vault, weth_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			ROOT_ASSET_ID,
			"ROOT",
			6,
			&[(alice, 1_000_000), (vortex_vault, root_vtx_vault_balance)], // this is just for the metadata
		)
		.build()
		.execute_with(|| {
			let vortex_dist_id = NextVortexId::<Test>::get();

			// check account have correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), bootstrap_root);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &vortex_vault),
				root_vtx_vault_balance
			);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &vortex_vault), usdc_vtx_vault_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &vortex_vault), weth_vtx_vault_balance);

			// create vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set vortex vault pre asset balances
			let vtx_vault_asset_balances = vec![
				(usdc_asset_id, usdc_vtx_vault_balance),
				(weth_asset_id, weth_vtx_vault_balance),
				(ROOT_ASSET_ID, root_vtx_vault_balance),
			];
			assert_ok!(Vortex::set_vtx_vault_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dist_id,
				vtx_current_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances = vec![
				(usdc_asset_id, usdc_fee_pot_balance),
				(weth_asset_id, weth_fee_pot_balance),
				(ROOT_ASSET_ID, root_fee_pot_balance),
			];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price. prices should be multiplied by the usd factor 10**6
			let usdc_price: Balance = 100_u128 * 10_u128.pow(6);
			let weth_price: Balance = 200_u128 * 10_u128.pow(6);
			let root_price: Balance = 3_u128 * 10_u128.pow(6);
			let asset_prices = vec![
				(usdc_asset_id, usdc_price),
				(weth_asset_id, weth_price),
				(ROOT_ASSET_ID, root_price),
			];
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(asset_prices.clone()).unwrap(),
			));

			// register reward and work points
			let mut reward_points_vec = vec![(bob, 100_000), (charlie, 100_000)];
			let mut total_reward_points = 100_000 + 100_000;
			let account_count = 3000;
			for i in 0..account_count {
				reward_points_vec.push((create_account(i + 4), 100_000));
				total_reward_points += 100_000;
			}
			let reward_points = BoundedVec::try_from(reward_points_vec).unwrap();

			let mut work_points_vec = vec![(bob, 10), (charlie, 10)];
			let mut total_work_points = 10 + 10;
			for i in 0..account_count {
				work_points_vec.push((create_account(i + 4), 10));
				total_work_points += 10;
			}
			let work_points = BoundedVec::try_from(work_points_vec).unwrap();
			assert_ok!(Vortex::register_reward_points(
				Origin::root(),
				vortex_dist_id,
				reward_points.clone()
			));
			assert_ok!(Vortex::register_work_points(Origin::root(), vortex_dist_id, work_points));

			//trigger vortext reward calcuation and assets/root transfer to vault
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id,));
			// run a few blocks to move the Vtx status from Triggering to Triggered
			for i in 2_u32..10 {
				System::set_block_number(i.into());
				Vortex::on_idle(i.into(), Weight::from_all(1_000_000_000_000_u64));
			}

			// check VtxPrice tally
			let vtx_vault_asset_balances_with_decimals = vec![
				(usdc_asset_id, usdc_vtx_vault_balance, 18),
				(weth_asset_id, weth_vtx_vault_balance, 18),
				(ROOT_ASSET_ID, root_vtx_vault_balance, 6),
			];
			let vtx_price_calculted = calculate_vtx_price(
				&vtx_vault_asset_balances_with_decimals,
				&asset_prices,
				vtx_current_supply,
			);
			assert_eq!(VtxPrice::<Test>::get(vortex_dist_id), vtx_price_calculted);
			println!("vtx_price_calculted: {:?}", vtx_price_calculted);
			// check vtx amounts tally
			let fee_pot_asset_balances_with_decimals = vec![
				(usdc_asset_id, usdc_fee_pot_balance, 18),
				(weth_asset_id, weth_fee_pot_balance, 18),
				(ROOT_ASSET_ID, root_fee_pot_balance, 6),
			];
			let (total_vortex_network_reward, total_vortex_bootstrap, total_vortex) = calculate_vtx(
				&fee_pot_asset_balances_with_decimals,
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
			println!("total_vortex: {:?}", total_vortex);
			println!("total_vortex_network_reward: {:?}", total_vortex_network_reward);
			println!("total_vortex_bootstrap: {:?}", total_vortex_bootstrap);

			// check bob got the vortex reward registered
			let staker_pool = total_vortex_bootstrap
				+ (Perquintill::from_percent(30) * total_vortex_network_reward);
			let workpoint_pool = Perquintill::from_percent(70) * total_vortex_network_reward;
			let bob_staker_point_portion =
				Perquintill::from_rational(100_000_u128, total_reward_points);
			let bob_work_points_portion = Perquintill::from_rational(10_u128, total_work_points);
			println!("bob_staker_point_portion: {:?}", bob_staker_point_portion);
			println!("bob_work_points_portion: {:?}", bob_work_points_portion);
			println!("staker_pool: {:?}", staker_pool);
			println!("workpoint_pool: {:?}", workpoint_pool);

			let bob_vtx_reward_calculated = ((bob_staker_point_portion * staker_pool)
				+ (bob_work_points_portion * workpoint_pool))
				.div(PRECISION_MULTIPLIER);
			println!("bob stker rewards: {:?}", bob_staker_point_portion * staker_pool);
			println!("bob workpoint rewards: {:?}", bob_work_points_portion * workpoint_pool);

			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dist_id, bob),
				(bob_vtx_reward_calculated, false)
			);
			println!("bob_vtx_reward_calculated: {:?}", bob_vtx_reward_calculated);

			// check if the last account entry balance
			let last_account_entry = create_account(account_count - 1 + 4);
			let last_account_entry_vtx_balance_before =
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &last_account_entry);
			assert_eq!(last_account_entry_vtx_balance_before, 0);

			//start the vortex distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dist_id,));

			// reset events
			System::reset_events();
			let num_reward_accounts = reward_points.len();
			let num_reward_registered_accounts =
				VtxDistOrderbook::<Test>::iter_prefix(vortex_dist_id).count();
			assert_eq!(num_reward_registered_accounts, num_reward_accounts);
			// run pay_unsigned one time, assert not everybody got the rewards at first run
			let mut acconts_got_paid = vec![];
			assert_ok!(Vortex::pay_unsigned(
				Origin::none(),
				vortex_dist_id,
				System::block_number()
			));
			// Iterate VtxDistPaidOut events
			System::events().iter().for_each(|record| match record.event {
				MockEvent::Vortex(crate::Event::VtxDistPaidOut { who, .. }) => {
					acconts_got_paid.push(who)
				},
				_ => {},
			});
			println!("acconts_got_paid: {:?}", acconts_got_paid.len());
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
					vortex_dist_id,
					System::block_number()
				));
				System::events().iter().for_each(|record| match record.event {
					MockEvent::Vortex(crate::Event::VtxDistPaidOut { who, .. }) => {
						acconts_got_paid.push(who)
					},
					MockEvent::Vortex(Event::VtxDistDone { .. }) => {
						dist_done = true;
					},
					_ => {},
				});
				println!("acconts_got_paid: {:?}", acconts_got_paid.len());
			}

			// check VtxDistStatuses status
			assert_eq!(VtxDistStatuses::<Test>::get(vortex_dist_id), VtxDistStatus::Done);
			// check the number of accounts that got rewards
			// TODO: check why this is failing 2001 to 2002, check all accounts get paid in a manual test
			println!(
				"vtx held pot: {:?}",
				AssetsExt::balance(
					<Test as crate::Config>::VtxAssetId::get(),
					&Vortex::get_vtx_held_account()
				)
			);
			assert_eq!(acconts_got_paid.len(), num_reward_accounts);

			// check bob received the reward
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob),
				bob_vtx_reward_calculated
			);
			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dist_id, bob),
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
				VtxDistOrderbook::<Test>::get(vortex_dist_id, last_account_entry),
				(last_account_entry_vtx_balance_after, true)
			);

			assert_eq!(
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get()),
				total_vortex.div(PRECISION_MULTIPLIER) + vtx_current_supply
			);
		});
}

#[test]
fn redeem_fails_if_amount_exceeds_user_balance() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	TestExt::default()
		.with_asset(<Test as crate::Config>::VtxAssetId::get(), "VORTEX", &[(alice, 1_000_000)])
		.build()
		.execute_with(|| {
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));
			assert_noop!(
				Vortex::redeem_tokens_from_vault(Origin::signed(bob), 1200),
				Error::<Test>::InvalidAmount
			);
		});
}

#[test]
fn redeem_fails_if_no_vtx() {
	let bob: AccountId = create_account(2);
	TestExt::default().build().execute_with(|| {
		assert_ok!(Vortex::create_vtx_dist(Origin::root()));
		assert_noop!(
			Vortex::redeem_tokens_from_vault(Origin::signed(bob), 1200),
			Error::<Test>::NoVtxAssetMinted
		);
	});
}

#[test]
fn redeem_fails_if_disabled() {
	let alice: AccountId = create_account(1);
	TestExt::default()
		.with_asset(<Test as crate::Config>::VtxAssetId::get(), "VORTEX", &[(alice, 1_000_000)])
		.build()
		.execute_with(|| {
			Vortex::create_vtx_dist(Origin::root()).unwrap();
			assert_ok!(Vortex::set_disable_redeem(Origin::root(), true,));
			assert_noop!(
				Vortex::redeem_tokens_from_vault(Origin::signed(alice), 1200),
				Error::<Test>::VtxRedeemDisabled
			);
		});
}

#[test]
fn redeem_tokens_from_vault_works() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	// asset ids
	let usdc_asset_id = 10;
	let weth_asset_id = 11;

	// vortex vault pre asset balances
	let vortex_vault = Vortex::get_vtx_vault_account();
	let usdc_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let weth_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let root_vtx_vault_balance = 100_u128 * 10_u128.pow(6);
	let vtx_current_supply = 1_000_u128 * 10_u128.pow(6);

	// fee pot asset balance
	let fee_vault = Vortex::get_fee_vault_account();
	let usdc_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let weth_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let root_fee_pot_balance = 1_u128 * 10_u128.pow(6);

	// Transfer bootstrap
	let root_vault = Vortex::get_root_vault_account();
	let bootstrap_root = 100_u128 * 10_u128.pow(6);

	TestExt::default()
		.with_balances(&[
			(alice, 2_000_000),
			(fee_vault, root_fee_pot_balance),
			(vortex_vault, root_vtx_vault_balance),
			(root_vault, bootstrap_root),
		])
		.with_asset(
			<Test as crate::Config>::VtxAssetId::get(),
			"VORTEX",
			&[(charlie, vtx_current_supply)],
		)
		.with_asset_decimals(
			usdc_asset_id,
			"USDC",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, usdc_vtx_vault_balance),
				(fee_vault, usdc_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			weth_asset_id,
			"WETH",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, weth_vtx_vault_balance),
				(fee_vault, weth_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			ROOT_ASSET_ID,
			"ROOT",
			6,
			&[(alice, 1_000_000), (vortex_vault, root_vtx_vault_balance)], // this is just for the metadata
		)
		.build()
		.execute_with(|| {
			let vortex_dist_id = NextVortexId::<Test>::get();

			// check account have correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), bootstrap_root);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &vortex_vault),
				root_vtx_vault_balance
			);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &vortex_vault), usdc_vtx_vault_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &vortex_vault), weth_vtx_vault_balance);

			// create vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set vortex vault pre asset balances
			let vtx_vault_asset_balances = vec![
				(usdc_asset_id, usdc_vtx_vault_balance),
				(weth_asset_id, weth_vtx_vault_balance),
				(ROOT_ASSET_ID, root_vtx_vault_balance),
			];
			assert_ok!(Vortex::set_vtx_vault_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dist_id,
				vtx_current_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances = vec![
				(usdc_asset_id, usdc_fee_pot_balance),
				(weth_asset_id, weth_fee_pot_balance),
				(ROOT_ASSET_ID, root_fee_pot_balance),
			];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price. prices should be multiplied by the usd factor 10**6
			let usdc_price: Balance = 100_u128 * 10_u128.pow(6);
			let weth_price: Balance = 200_u128 * 10_u128.pow(6);
			let root_price: Balance = 3_u128 * 10_u128.pow(6);
			let asset_prices = vec![
				(usdc_asset_id, usdc_price),
				(weth_asset_id, weth_price),
				(ROOT_ASSET_ID, root_price),
			];
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
				reward_points.clone()
			));
			assert_ok!(Vortex::register_work_points(Origin::root(), vortex_dist_id, work_points));

			// check fee pot and bootstrap root account has correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), bootstrap_root);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);

			// trigger vortex distribution and do the preparations for distribution
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id));
			// Check that the correct event was emitted.
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggering {
				id: vortex_dist_id,
			}));
			// run a few blocks to move the Vtx status from Triggering to Triggered
			for i in 2_u32..4 {
				System::set_block_number(i.into());
				Vortex::on_idle(i.into(), Weight::from_all(1_000_000_000_000_u64));
			}
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggered {
				id: vortex_dist_id,
			}));

			let bob_vtx_reward = VtxDistOrderbook::<Test>::get(vortex_dist_id, bob).0;
			let total_vtx_to_be_minted = TotalVortex::<Test>::get(vortex_dist_id);

			//set the VtxVaultRedeemAssetList
			let vtx_redeem_asset_list =
				BoundedVec::try_from(vec![usdc_asset_id, weth_asset_id, ROOT_ASSET_ID]).unwrap();
			assert_ok!(Vortex::set_vtx_vault_redeem_asset_list(
				Origin::root(),
				vtx_redeem_asset_list.clone()
			));

			//start the vortex distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dist_id,));

			// reset events
			System::reset_events();
			let num_reward_accounts = reward_points.len();
			let num_reward_registered_accounts =
				VtxDistOrderbook::<Test>::iter_prefix(vortex_dist_id).count();
			assert_eq!(num_reward_registered_accounts, num_reward_accounts);
			// run pay_unsigned one time, assert not everybody got the rewards at first run
			assert_ok!(Vortex::pay_unsigned(
				Origin::none(),
				vortex_dist_id,
				System::block_number()
			));

			System::assert_has_event(MockEvent::Vortex(Event::VtxDistPaidOut {
				id: vortex_dist_id,
				who: bob,
				amount: bob_vtx_reward,
			}));

			// check bob received the reward
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob),
				bob_vtx_reward
			);
			assert_eq!(VtxDistOrderbook::<Test>::get(vortex_dist_id, bob), (bob_vtx_reward, true));

			assert_eq!(
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get()),
				total_vtx_to_be_minted.div(PRECISION_MULTIPLIER) + vtx_current_supply
			);

			// Try redeem
			println!("bob_vtx_reward_calculated: {:?}", bob_vtx_reward);
			let current_total_vortex =
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get());
			let vtx_redeem_asset_balances = vtx_redeem_asset_list
				.iter()
				.map(|asset_id| {
					let amount = AssetsExt::balance(*asset_id, &Vortex::get_vtx_vault_account());
					(*asset_id, amount)
				})
				.collect::<Vec<_>>();

			assert_ok!(Vortex::redeem_tokens_from_vault(Origin::signed(bob), bob_vtx_reward));
			//check Bob's balances
			let bob_redeem_amounts = calculate_vtx_redeem(
				&vtx_redeem_asset_balances,
				bob_vtx_reward,
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
	let asset_prices = vec![(1, 16_140), (2, 2_461_300)];
	let vtx_current_supply = 10_000_000;
	let bootstrap_root = 3_000_000;
	let root_price = asset_prices[0].1;
	let account = AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"));
	let account_staker_reward_points = 2_000_000_u128;
	let account_worker_points = 10_000_000_u128;
	let total_staker_reward_points = 3_000_000;
	let total_worker_points = 11_000_000;
	println!("vtx price breakdown for account: {:?}", account);

	// check VtxPrice tally
	let vtx_vault_asset_balances_with_decimals = vec![(1, 100_000_000, 6), (2, 10_000_000, 6)];
	let vtx_price_calculted = calculate_vtx_price(
		&vtx_vault_asset_balances_with_decimals,
		&asset_prices,
		vtx_current_supply,
	);
	println!("vtx_price_calculted: {:?}", vtx_price_calculted);
	// check vtx amounts tally
	let fee_pot_asset_balances_with_decimals = vec![(1, 200_000_000, 18), (2, 10_000_000, 18)];
	let (total_vortex_network_reward, total_vortex_bootstrap, total_vortex) = calculate_vtx(
		&fee_pot_asset_balances_with_decimals,
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
		total_vortex_bootstrap + (Perquintill::from_percent(30) * total_vortex_network_reward);
	let workpoint_pool = Perquintill::from_percent(70) * total_vortex_network_reward;
	println!("staker_pool: {:?}", staker_pool);
	println!("workpoint_pool: {:?}", workpoint_pool);

	let account_staker_point_portion =
		Perquintill::from_rational(account_staker_reward_points, total_staker_reward_points);
	let account_work_points_portion =
		Perquintill::from_rational(account_worker_points, total_worker_points);
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

#[test]
fn set_vtx_vault_redeem_asset_list_works() {
	TestExt::default().build().execute_with(|| {
		// register reward points
		let redeem_asset_list = BoundedVec::try_from(vec![1, 2, 3]).unwrap();
		assert_ok!(Vortex::set_vtx_vault_redeem_asset_list(
			Origin::root(),
			redeem_asset_list.clone()
		));

		// Check for the SetVtxVaultRedeemAssetList event
		System::assert_last_event(MockEvent::Vortex(crate::Event::SetVtxVaultRedeemAssetList {
			asset_list: redeem_asset_list,
		}));
	});
}

#[test]
fn set_vtx_vault_redeem_asset_list_fails_with_non_authorized_origin() {
	TestExt::default().build().execute_with(|| {
		// register reward points
		let redeem_asset_list = BoundedVec::try_from(vec![1, 2, 3]).unwrap();
		assert_noop!(
			Vortex::set_vtx_vault_redeem_asset_list(
				Origin::signed(create_account(3)),
				redeem_asset_list.clone()
			),
			Error::<Test>::RequireAdmin
		);
	});
}

#[test]
fn set_enable_manual_reward_input_can_be_used_for_legacy_flow_before_trigger() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	// asset ids
	let usdc_asset_id = 10;
	let weth_asset_id = 11;

	// vortex vault pre asset balances
	let vortex_vault = Vortex::get_vtx_vault_account();
	let usdc_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let weth_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let root_vtx_vault_balance = 100_u128 * 10_u128.pow(6);
	let vtx_current_supply = 1_000_u128 * 10_u128.pow(6);

	// fee pot asset balance
	let fee_vault = Vortex::get_fee_vault_account();
	let usdc_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let weth_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let root_fee_pot_balance = 1_u128 * 10_u128.pow(6);

	// Transfer bootstrap
	let root_vault = Vortex::get_root_vault_account();
	let bootstrap_root = 100_u128 * 10_u128.pow(6);

	let end_block = 10;

	TestExt::default()
		.with_balances(&[
			(alice, 2_000_000),
			(fee_vault, root_fee_pot_balance),
			(vortex_vault, root_vtx_vault_balance),
			(root_vault, bootstrap_root),
		])
		.with_asset(
			<Test as crate::Config>::VtxAssetId::get(),
			"VORTEX",
			&[(charlie, vtx_current_supply)],
		)
		.with_asset_decimals(
			usdc_asset_id,
			"USDC",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, usdc_vtx_vault_balance),
				(fee_vault, usdc_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			weth_asset_id,
			"WETH",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, weth_vtx_vault_balance),
				(fee_vault, weth_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			ROOT_ASSET_ID,
			"ROOT",
			6,
			&[(alice, 1_000_000), (vortex_vault, root_vtx_vault_balance)], // this is just for the metadata
		)
		.build()
		.execute_with(|| {
			let vortex_dist_id = NextVortexId::<Test>::get();

			// check account have correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), bootstrap_root);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &vortex_vault),
				root_vtx_vault_balance
			);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &vortex_vault), usdc_vtx_vault_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &vortex_vault), weth_vtx_vault_balance);

			// create vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set vortex vault pre asset balances
			let vtx_vault_asset_balances = vec![
				(usdc_asset_id, usdc_vtx_vault_balance),
				(weth_asset_id, weth_vtx_vault_balance),
				(ROOT_ASSET_ID, root_vtx_vault_balance),
			];
			assert_ok!(Vortex::set_vtx_vault_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dist_id,
				vtx_current_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances = vec![
				(usdc_asset_id, usdc_fee_pot_balance),
				(weth_asset_id, weth_fee_pot_balance),
				(ROOT_ASSET_ID, root_fee_pot_balance),
			];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price. prices should be multiplied by the usd factor 10**6
			let usdc_price: Balance = 100_u128 * 10_u128.pow(6);
			let weth_price: Balance = 200_u128 * 10_u128.pow(6);
			let root_price: Balance = 3_u128 * 10_u128.pow(6);
			let asset_prices = vec![
				(usdc_asset_id, usdc_price),
				(weth_asset_id, weth_price),
				(ROOT_ASSET_ID, root_price),
			];
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(asset_prices.clone()).unwrap(),
			));

			// Intentionally left commented to show the diff
			/*
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
			 */

			// register rewards manually pre trigger
			assert_ok!(Vortex::set_enable_manual_reward_input(Origin::root(), true));
			let bob_reward = 10_000_000; // in drops
			let charlie_reward = 20_000_000; // in drops
			let rewards =
				BoundedVec::try_from(vec![(bob, bob_reward), (charlie, charlie_reward)]).unwrap();
			assert_ok!(Vortex::register_rewards(Origin::root(), vortex_dist_id, rewards));
			let total_vtx_manual_input = bob_reward + charlie_reward;

			// trigger vortex distribution and do the preparations for distribution
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id));

			// Vtx status directly goes to the Triggered, as no need to calculate the rewards
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggered {
				id: vortex_dist_id,
			}));
			assert_eq!(VtxDistStatuses::<Test>::get(vortex_dist_id), VtxDistStatus::Triggered);

			// check VtxPrice tally
			let vtx_vault_asset_balances_with_decimals = vec![
				(usdc_asset_id, usdc_vtx_vault_balance, 18),
				(weth_asset_id, weth_vtx_vault_balance, 18),
				(ROOT_ASSET_ID, root_vtx_vault_balance, 6),
			];
			let vtx_price_calculted = calculate_vtx_price(
				&vtx_vault_asset_balances_with_decimals,
				&asset_prices,
				vtx_current_supply,
			);
			assert_eq!(VtxPrice::<Test>::get(vortex_dist_id), vtx_price_calculted);
			// check vtx amounts tally
			let fee_pot_asset_balances_with_decimals = vec![
				(usdc_asset_id, usdc_fee_pot_balance, 18),
				(weth_asset_id, weth_fee_pot_balance, 18),
				(ROOT_ASSET_ID, root_fee_pot_balance, 6),
			];
			let (total_vortex_network_reward, total_vortex_bootstrap, total_vortex) = calculate_vtx(
				&fee_pot_asset_balances_with_decimals,
				&asset_prices,
				bootstrap_root,
				root_price,
				vtx_price_calculted,
			);
			assert_ne!(TotalVortex::<Test>::get(vortex_dist_id), total_vortex);
			assert_eq!(
				TotalVortex::<Test>::get(vortex_dist_id),
				total_vtx_manual_input.saturating_mul(PRECISION_MULTIPLIER)
			);
			assert_eq!(
				TotalNetworkReward::<Test>::get(vortex_dist_id),
				total_vortex_network_reward
			);
			assert_eq!(TotalBootstrapReward::<Test>::get(vortex_dist_id), total_vortex_bootstrap);
			assert_eq!(VtxDistOrderbook::<Test>::get(vortex_dist_id, bob), (bob_reward, false));

			// let's say we decided to change the charlie's rewards manually even after the trigger
			let charlie_adjusted_reward = 150_000;
			let rewards = BoundedVec::try_from(vec![(charlie, charlie_adjusted_reward)]).unwrap();
			assert_ok!(Vortex::register_rewards(Origin::root(), vortex_dist_id, rewards));
			let total_vtx_manual_input_adjusted = bob_reward + charlie_adjusted_reward;
			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dist_id, charlie),
				(charlie_adjusted_reward, false)
			);
			assert_eq!(
				TotalVortex::<Test>::get(vortex_dist_id),
				total_vtx_manual_input_adjusted.saturating_mul(PRECISION_MULTIPLIER)
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
				total_vtx_manual_input_adjusted
			);
			run_to_block(end_block);
			assert_ok!(Vortex::pay_unsigned(Origin::none(), vortex_dist_id, end_block));
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &bob),
				bob_reward
			);
			// check vtx total issuance now. should be total_vortex + vtx_current_supply
			assert_eq!(
				AssetsExt::total_issuance(<Test as crate::Config>::VtxAssetId::get()),
				total_vtx_manual_input_adjusted + vtx_current_supply
			);
			// orderbook entry should be disabled once paid
			assert_eq!(VtxDistOrderbook::<Test>::get(vortex_dist_id, bob), (bob_reward, true));
			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dist_id, charlie),
				(charlie_adjusted_reward, true)
			);
		});
}

// vortex price should be 1  if vtx_existing_supply is 0

#[test]
fn vtx_price_calculation_cycle_5() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	// asset ids
	let asto_asset_id = 4_196;
	let sylo_asset_id = 2_148;
	let eth_asset_id = 1_124;
	let xrp_asset_id = 2;
	let root_asset_id = 1;
	let vtx_asset_id = <Test as crate::Config>::VtxAssetId::get();
	// vortex vault pre asset balances - same as cycle 5
	let vortex_vault = Vortex::get_vtx_vault_account();
	let root_balance_in_vtx_vault = 12768121398776_u128;
	let asto_balance_in_vtx_vault = 3319193139068424549_u128;
	let xrp_balance_in_vtx_vault = 33360272058_u128;
	let sylo_balance_in_vtx_vault = 1598213439551632365_u128;
	let eth_balance_in_vtx_vault = 459527286806489_u128;
	let vtx_total_supply = 870428149751_u128;

	// fee pot asset balance - set mock values
	let fee_vault = Vortex::get_fee_vault_account();
	let asto_fee_pot_balance = 1_000_000;
	let sylo_fee_pot_balance = 1_000_000;
	let eth_fee_pot_balance = 1_000_000;
	let xrp_fee_pot_balance = 1_000_000;
	let root_fee_pot_balance = 1_000_000;

	let root_vault = Vortex::get_root_vault_account();
	let bootstrap_root = 1_000_000;

	// prices should be multiplied by the usd factor 10**6
	let root_price = (0.01614 * 1000000.00) as u128;
	let xrp_price = (2.4613 * 1000000.00) as u128;
	let sylo_price = (0.0006042 * 1000000.00) as u128;
	let asto_price = (0.01746 * 1000000.00) as u128;
	let eth_price = (2681.04 * 1000000.00) as u128;

	TestExt::default()
		.with_balances(&[
			(alice, 2_000_000),
			(vortex_vault, root_balance_in_vtx_vault),
			(fee_vault, root_fee_pot_balance),
			(root_vault, bootstrap_root),
		])
		.with_asset_decimals(root_asset_id, "ROOT", 6, &[(alice, 1_000_000)]) // note this is just to create the ROOT asset in the system
		.with_asset_decimals(
			xrp_asset_id,
			"XRP",
			6,
			&[
				(alice, 1_000_000),
				(vortex_vault, xrp_balance_in_vtx_vault),
				(fee_vault, xrp_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			asto_asset_id,
			"ASTO",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, asto_balance_in_vtx_vault),
				(fee_vault, asto_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			sylo_asset_id,
			"SYLO",
			18,
			&[
				(alice, 5),
				(vortex_vault, sylo_balance_in_vtx_vault),
				(fee_vault, sylo_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			eth_asset_id,
			"ETH",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, eth_balance_in_vtx_vault),
				(fee_vault, eth_fee_pot_balance),
			],
		)
		.with_asset_decimals(vtx_asset_id, "VTX", 6, &[(alice, vtx_total_supply)])
		.build()
		.execute_with(|| {
			let vortex_dist_id = NextVortexId::<Test>::get();

			// check accounts has correct balances
			assert_eq!(AssetsExt::balance(root_asset_id, &vortex_vault), root_balance_in_vtx_vault);
			assert_eq!(AssetsExt::balance(xrp_asset_id, &vortex_vault), xrp_balance_in_vtx_vault);
			assert_eq!(AssetsExt::balance(asto_asset_id, &vortex_vault), asto_balance_in_vtx_vault);
			assert_eq!(AssetsExt::balance(sylo_asset_id, &vortex_vault), sylo_balance_in_vtx_vault);
			assert_eq!(AssetsExt::balance(eth_asset_id, &vortex_vault), eth_balance_in_vtx_vault);
			assert_eq!(AssetsExt::balance(asto_asset_id, &fee_vault), asto_fee_pot_balance);
			assert_eq!(AssetsExt::balance(sylo_asset_id, &fee_vault), sylo_fee_pot_balance);
			assert_eq!(AssetsExt::balance(eth_asset_id, &fee_vault), eth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(xrp_asset_id, &fee_vault), xrp_fee_pot_balance);
			assert_eq!(AssetsExt::balance(root_asset_id, &fee_vault), root_fee_pot_balance);
			assert_eq!(AssetsExt::balance(root_asset_id, &root_vault), bootstrap_root);
			// create vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set vortex vault pre asset balances
			let vtx_vault_asset_balances = vec![
				(root_asset_id, root_balance_in_vtx_vault),
				(xrp_asset_id, xrp_balance_in_vtx_vault),
				(asto_asset_id, asto_balance_in_vtx_vault),
				(sylo_asset_id, sylo_balance_in_vtx_vault),
				(eth_asset_id, eth_balance_in_vtx_vault),
			];
			assert_ok!(Vortex::set_vtx_vault_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dist_id,
				vtx_total_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances = vec![
				(asto_asset_id, asto_fee_pot_balance),
				(sylo_asset_id, sylo_fee_pot_balance),
				(eth_asset_id, eth_fee_pot_balance),
				(xrp_asset_id, xrp_fee_pot_balance),
				(root_asset_id, root_fee_pot_balance),
			];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price
			let asset_prices = vec![
				(root_asset_id, root_price),
				(xrp_asset_id, xrp_price),
				(asto_asset_id, asto_price),
				(sylo_asset_id, sylo_price),
				(eth_asset_id, eth_price),
			];
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(asset_prices.clone()).unwrap(),
			));

			// register reward and work points - mock values
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
			// Check that the correct event was emitted.
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggering {
				id: vortex_dist_id,
			}));
			// run a few blocks to move the Vtx status from Triggering to Triggered
			for i in 2_u32..4 {
				System::set_block_number(i.into());
				Vortex::on_idle(i.into(), Weight::from_all(1_000_000_000_000_u64));
			}
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggered {
				id: vortex_dist_id,
			}));

			// check balances have been transferred to vtx vault account
			// check fee pot and bootstrap root account has correct balances
			assert_eq!(AssetsExt::balance(root_asset_id, &root_vault), 0);
			assert_eq!(AssetsExt::balance(xrp_asset_id, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(eth_asset_id, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(asto_asset_id, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(sylo_asset_id, &fee_vault), 0);
			assert_eq!(
				AssetsExt::balance(root_asset_id, &vortex_vault),
				root_balance_in_vtx_vault + bootstrap_root + root_fee_pot_balance
			);
			assert_eq!(
				AssetsExt::balance(xrp_asset_id, &vortex_vault),
				xrp_balance_in_vtx_vault + xrp_fee_pot_balance
			);
			assert_eq!(
				AssetsExt::balance(eth_asset_id, &vortex_vault),
				eth_balance_in_vtx_vault + eth_fee_pot_balance
			);
			assert_eq!(
				AssetsExt::balance(asto_asset_id, &vortex_vault),
				asto_balance_in_vtx_vault + asto_fee_pot_balance
			);
			assert_eq!(
				AssetsExt::balance(sylo_asset_id, &vortex_vault),
				sylo_balance_in_vtx_vault + sylo_fee_pot_balance
			);
			// check VtxPrice tally
			let vtx_vault_asset_balances_with_decimals = vec![
				(root_asset_id, root_balance_in_vtx_vault, 6),
				(xrp_asset_id, xrp_balance_in_vtx_vault, 6),
				(asto_asset_id, asto_balance_in_vtx_vault, 18),
				(sylo_asset_id, sylo_balance_in_vtx_vault, 18),
				(eth_asset_id, eth_balance_in_vtx_vault, 18),
			];
			let vtx_price_calculted = calculate_vtx_price(
				&vtx_vault_asset_balances_with_decimals,
				&asset_prices,
				vtx_total_supply,
			);
			println!("vtx_price_calculted: {}", vtx_price_calculted);
			assert_eq!(VtxPrice::<Test>::get(vortex_dist_id), vtx_price_calculted);
			// check vtx amounts tally
			let fee_pot_asset_balances_with_decimals = vec![
				(root_asset_id, root_fee_pot_balance, 6),
				(xrp_asset_id, xrp_fee_pot_balance, 6),
				(asto_asset_id, asto_fee_pot_balance, 18),
				(sylo_asset_id, sylo_fee_pot_balance, 18),
				(eth_asset_id, eth_fee_pot_balance, 18),
			];
			let (total_vortex_network_reward, total_vortex_bootstrap, total_vortex) = calculate_vtx(
				&fee_pot_asset_balances_with_decimals,
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
			let staker_pool = total_vortex_bootstrap
				+ (Perquintill::from_percent(30) * total_vortex_network_reward);
			let workpoint_pool = Perquintill::from_percent(70) * total_vortex_network_reward;
			let bob_staker_point_portion =
				Perquintill::from_rational(100_000_u128, 100_000_u128 + 100_000_u128);
			let bob_work_points_portion = Perquintill::from_rational(10_u128, 10_u128 + 10_u128);
			let bob_vtx_reward_calculated = (bob_staker_point_portion * staker_pool)
				+ (bob_work_points_portion * workpoint_pool);
			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dist_id, bob),
				(bob_vtx_reward_calculated.div(PRECISION_MULTIPLIER), false)
			);
		});
}

#[test]
fn trigger_vtx_distribution_with_high_vtx_price() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	// asset ids
	let usdc_asset_id = 10;
	let weth_asset_id = 11;

	// vortex vault pre asset balances
	let vortex_vault = Vortex::get_vtx_vault_account();
	let usdc_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let weth_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let root_vtx_vault_balance = 100_u128 * 10_u128.pow(6);
	// lowering  vtx_current_supply would shoot up the vtx price
	let vtx_current_supply = 1_u128 * 10_u128.pow(6);

	// fee pot asset balance
	let fee_vault = Vortex::get_fee_vault_account();
	let usdc_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let weth_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let root_fee_pot_balance = 1_u128 * 10_u128.pow(6);

	// bootstrap balance
	let root_vault = Vortex::get_root_vault_account();
	let bootstrap_root = 100_u128 * 10_u128.pow(6);

	TestExt::default()
		.with_balances(&[
			(alice, 2_000_000),
			(fee_vault, root_fee_pot_balance),
			(vortex_vault, root_vtx_vault_balance),
			(root_vault, bootstrap_root),
		])
		.with_asset(
			<Test as crate::Config>::VtxAssetId::get(),
			"VORTEX",
			&[(charlie, vtx_current_supply)],
		)
		.with_asset_decimals(
			usdc_asset_id,
			"USDC",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, usdc_vtx_vault_balance),
				(fee_vault, usdc_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			weth_asset_id,
			"WETH",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, weth_vtx_vault_balance),
				(fee_vault, weth_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			ROOT_ASSET_ID,
			"ROOT",
			6,
			&[(alice, 1_000_000), (vortex_vault, root_vtx_vault_balance)], // this is just for the metadata
		)
		.build()
		.execute_with(|| {
			let vortex_dist_id = NextVortexId::<Test>::get();

			// check account have correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), bootstrap_root);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &vortex_vault),
				root_vtx_vault_balance
			);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &vortex_vault), usdc_vtx_vault_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &vortex_vault), weth_vtx_vault_balance);

			// create vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set vortex vault pre asset balances
			let vtx_vault_asset_balances = vec![
				(usdc_asset_id, usdc_vtx_vault_balance),
				(weth_asset_id, weth_vtx_vault_balance),
				(ROOT_ASSET_ID, root_vtx_vault_balance),
			];
			assert_ok!(Vortex::set_vtx_vault_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dist_id,
				vtx_current_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances = vec![
				(usdc_asset_id, usdc_fee_pot_balance),
				(weth_asset_id, weth_fee_pot_balance),
				(ROOT_ASSET_ID, root_fee_pot_balance),
			];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price. prices should be multiplied by the usd factor 10**6
			let usdc_price: Balance = 100_u128 * 10_u128.pow(6);
			let weth_price: Balance = 200_u128 * 10_u128.pow(6);
			let root_price: Balance = 3_u128 * 10_u128.pow(6);
			let asset_prices = vec![
				(usdc_asset_id, usdc_price),
				(weth_asset_id, weth_price),
				(ROOT_ASSET_ID, root_price),
			];
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
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);

			// trigger vortex distribution and do the preparations for distribution
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id));
			// Check that the correct event was emitted.
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggering {
				id: vortex_dist_id,
			}));
			// run a few blocks to move the Vtx status from Triggering to Triggered
			for i in 2_u32..4 {
				System::set_block_number(i.into());
				Vortex::on_idle(i.into(), Weight::from_all(1_000_000_000_000_u64));
			}
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggered {
				id: vortex_dist_id,
			}));

			// check balances have been transferred to vtx vault account
			// check fee pot and bootstrap root account has correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), 0);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), 0);
			let vtx_vault_account = Vortex::get_vtx_vault_account();
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &vtx_vault_account),
				root_vtx_vault_balance + bootstrap_root + root_fee_pot_balance
			);
			assert_eq!(
				AssetsExt::balance(usdc_asset_id, &vtx_vault_account),
				usdc_fee_pot_balance + usdc_vtx_vault_balance
			);
			assert_eq!(
				AssetsExt::balance(weth_asset_id, &vtx_vault_account),
				weth_fee_pot_balance + weth_vtx_vault_balance
			);

			// check VtxPrice tally
			let vtx_vault_asset_balances_with_decimals = vec![
				(usdc_asset_id, usdc_vtx_vault_balance, 18),
				(weth_asset_id, weth_vtx_vault_balance, 18),
				(ROOT_ASSET_ID, root_vtx_vault_balance, 6),
			];
			let vtx_price_calculted = calculate_vtx_price(
				&vtx_vault_asset_balances_with_decimals,
				&asset_prices,
				vtx_current_supply,
			);
			println!("vtx_price_calculted: {}", vtx_price_calculted);
			assert_eq!(VtxPrice::<Test>::get(vortex_dist_id), vtx_price_calculted);
			assert_eq!(VtxPrice::<Test>::get(vortex_dist_id), 1800000000); // price is 1800 in standard units. i.e USD
															   // check vtx amounts tally
			let fee_pot_asset_balances_with_decimals = vec![
				(usdc_asset_id, usdc_fee_pot_balance, 18),
				(weth_asset_id, weth_fee_pot_balance, 18),
				(ROOT_ASSET_ID, root_fee_pot_balance, 6),
			];
			let (total_vortex_network_reward, total_vortex_bootstrap, total_vortex) = calculate_vtx(
				&fee_pot_asset_balances_with_decimals,
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
			println!(
				"total_vortex_network_reward: {}",
				TotalNetworkReward::<Test>::get(vortex_dist_id)
			);
			println!(
				"total_vortex_bootstrap: {}",
				TotalBootstrapReward::<Test>::get(vortex_dist_id)
			);
			println!("total_vortex: {}", TotalVortex::<Test>::get(vortex_dist_id));
			// check the values below, onchain calculations supports upto 1 drop precision
			assert_eq!(TotalVortex::<Test>::get(vortex_dist_id).div(PRECISION_MULTIPLIER), 334999);
			assert_eq!(
				TotalNetworkReward::<Test>::get(vortex_dist_id).div(PRECISION_MULTIPLIER),
				168333
			);
			assert_eq!(
				TotalBootstrapReward::<Test>::get(vortex_dist_id).div(PRECISION_MULTIPLIER),
				166666
			);

			// check bob got the vortex reward registered
			let staker_pool = total_vortex_bootstrap
				+ (Perquintill::from_percent(30) * total_vortex_network_reward);
			let workpoint_pool = Perquintill::from_percent(70) * total_vortex_network_reward;
			let bob_staker_point_portion =
				Perquintill::from_rational(100_000_u128, 100_000_u128 + 100_000_u128);
			let bob_work_points_portion = Perquintill::from_rational(10_u128, 10_u128 + 10_u128);
			let bob_vtx_reward_calculated = (bob_staker_point_portion * staker_pool)
				+ (bob_work_points_portion * workpoint_pool);
			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dist_id, bob),
				(bob_vtx_reward_calculated.div(PRECISION_MULTIPLIER), false)
			);
		});
}

#[test]
fn vtx_price_calculation_cycle_5_real_data() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	// asset ids
	let asto_asset_id = 4_196;
	let sylo_asset_id = 2_148;
	let eth_asset_id = 1_124;
	let xrp_asset_id = 2;
	let root_asset_id = 1;
	let vtx_asset_id = <Test as crate::Config>::VtxAssetId::get();
	// vortex vault pre asset balances - same as cycle 5
	let vortex_vault = Vortex::get_vtx_vault_account();
	let root_balance_in_vtx_vault = 12768121398776_u128;
	let asto_balance_in_vtx_vault = 3319193139068424549_u128;
	let xrp_balance_in_vtx_vault = 33360272058_u128;
	let sylo_balance_in_vtx_vault = 1598213439551632365_u128;
	let eth_balance_in_vtx_vault = 459527286806489_u128;
	let vtx_total_supply = 870428149751_u128;

	// fee pot asset balance
	let fee_vault = Vortex::get_fee_vault_account();
	let asto_fee_pot_balance = 6_750_001_162_780_848_574;
	let xrp_fee_pot_balance = 14_694_106_335;
	let root_fee_pot_balance = 18_631_164_600;

	let root_vault = Vortex::get_root_vault_account();
	let bootstrap_root = 17_057_331_103_329;

	// prices should be multiplied by the usd factor 10**6
	let root_price = (0.01614 * 1000000.00) as u128;
	let xrp_price = (2.4613 * 1000000.00) as u128;
	let sylo_price = (0.0006042 * 1000000.00) as u128;
	let asto_price = (0.01746 * 1000000.00) as u128;
	let eth_price = (2681.04 * 1000000.00) as u128;

	TestExt::default()
		.with_balances(&[
			(alice, 2_000_000),
			(vortex_vault, root_balance_in_vtx_vault),
			(fee_vault, root_fee_pot_balance),
			(root_vault, bootstrap_root),
		])
		.with_asset_decimals(root_asset_id, "ROOT", 6, &[(alice, 1_000_000)]) // note this is just to create the ROOT asset in the system
		.with_asset_decimals(
			xrp_asset_id,
			"XRP",
			6,
			&[
				(alice, 1_000_000),
				(vortex_vault, xrp_balance_in_vtx_vault),
				(fee_vault, xrp_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			asto_asset_id,
			"ASTO",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, asto_balance_in_vtx_vault),
				(fee_vault, asto_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			sylo_asset_id,
			"SYLO",
			18,
			&[(alice, 5), (vortex_vault, sylo_balance_in_vtx_vault)],
		)
		.with_asset_decimals(
			eth_asset_id,
			"ETH",
			18,
			&[(alice, 1_000_000), (vortex_vault, eth_balance_in_vtx_vault)],
		)
		.with_asset_decimals(vtx_asset_id, "VTX", 6, &[(alice, vtx_total_supply)])
		.build()
		.execute_with(|| {
			let vortex_dist_id = NextVortexId::<Test>::get();

			// check accounts has correct balances
			assert_eq!(AssetsExt::balance(root_asset_id, &vortex_vault), root_balance_in_vtx_vault);
			assert_eq!(AssetsExt::balance(xrp_asset_id, &vortex_vault), xrp_balance_in_vtx_vault);
			assert_eq!(AssetsExt::balance(asto_asset_id, &vortex_vault), asto_balance_in_vtx_vault);
			assert_eq!(AssetsExt::balance(sylo_asset_id, &vortex_vault), sylo_balance_in_vtx_vault);
			assert_eq!(AssetsExt::balance(eth_asset_id, &vortex_vault), eth_balance_in_vtx_vault);
			assert_eq!(AssetsExt::balance(asto_asset_id, &fee_vault), asto_fee_pot_balance);
			assert_eq!(AssetsExt::balance(xrp_asset_id, &fee_vault), xrp_fee_pot_balance);
			assert_eq!(AssetsExt::balance(root_asset_id, &fee_vault), root_fee_pot_balance);
			assert_eq!(AssetsExt::balance(root_asset_id, &root_vault), bootstrap_root);
			// create vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set vortex vault pre asset balances
			let vtx_vault_asset_balances = vec![
				(root_asset_id, root_balance_in_vtx_vault),
				(xrp_asset_id, xrp_balance_in_vtx_vault),
				(asto_asset_id, asto_balance_in_vtx_vault),
				(sylo_asset_id, sylo_balance_in_vtx_vault),
				(eth_asset_id, eth_balance_in_vtx_vault),
			];
			assert_ok!(Vortex::set_vtx_vault_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dist_id,
				vtx_total_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances = vec![
				(asto_asset_id, asto_fee_pot_balance),
				(xrp_asset_id, xrp_fee_pot_balance),
				(root_asset_id, root_fee_pot_balance),
			];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price
			let asset_prices = vec![
				(root_asset_id, root_price),
				(xrp_asset_id, xrp_price),
				(asto_asset_id, asto_price),
				(sylo_asset_id, sylo_price),
				(eth_asset_id, eth_price),
			];
			assert_ok!(Vortex::set_asset_prices(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(asset_prices.clone()).unwrap(),
			));

			// register reward and work points
			let reward_points =
				BoundedVec::try_from(vec![(bob, 24600), (charlie, 17057331103329 - 24600)])
					.unwrap();
			let work_points = BoundedVec::try_from(vec![(charlie, 39180127080137)]).unwrap();
			assert_ok!(Vortex::register_reward_points(
				Origin::root(),
				vortex_dist_id,
				reward_points
			));
			assert_ok!(Vortex::register_work_points(Origin::root(), vortex_dist_id, work_points));

			// trigger vortex distribution and do the preparations for distribution
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id));
			// Check that the correct event was emitted.
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggering {
				id: vortex_dist_id,
			}));
			// run a few blocks to move the Vtx status from Triggering to Triggered
			for i in 2_u32..4 {
				System::set_block_number(i.into());
				Vortex::on_idle(i.into(), Weight::from_all(1_000_000_000_000_u64));
			}
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggered {
				id: vortex_dist_id,
			}));

			// check balances have been transferred to vtx vault account
			// check fee pot and bootstrap root account has correct balances
			assert_eq!(AssetsExt::balance(root_asset_id, &root_vault), 0);
			assert_eq!(AssetsExt::balance(xrp_asset_id, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(eth_asset_id, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(asto_asset_id, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(sylo_asset_id, &fee_vault), 0);
			assert_eq!(
				AssetsExt::balance(root_asset_id, &vortex_vault),
				root_balance_in_vtx_vault + bootstrap_root + root_fee_pot_balance
			);
			assert_eq!(
				AssetsExt::balance(xrp_asset_id, &vortex_vault),
				xrp_balance_in_vtx_vault + xrp_fee_pot_balance
			);
			assert_eq!(AssetsExt::balance(eth_asset_id, &vortex_vault), eth_balance_in_vtx_vault);
			assert_eq!(
				AssetsExt::balance(asto_asset_id, &vortex_vault),
				asto_balance_in_vtx_vault + asto_fee_pot_balance
			);
			assert_eq!(AssetsExt::balance(sylo_asset_id, &vortex_vault), sylo_balance_in_vtx_vault);
			// check VtxPrice tally
			let vtx_vault_asset_balances_with_decimals = vec![
				(root_asset_id, root_balance_in_vtx_vault, 6),
				(xrp_asset_id, xrp_balance_in_vtx_vault, 6),
				(asto_asset_id, asto_balance_in_vtx_vault, 18),
				(sylo_asset_id, sylo_balance_in_vtx_vault, 18),
				(eth_asset_id, eth_balance_in_vtx_vault, 18),
			];
			let vtx_price_calculted = calculate_vtx_price(
				&vtx_vault_asset_balances_with_decimals,
				&asset_prices,
				vtx_total_supply,
			);
			println!("vtx_price_calculted: {}", vtx_price_calculted);
			assert_eq!(VtxPrice::<Test>::get(vortex_dist_id), vtx_price_calculted);
			// check vtx amounts tally
			let fee_pot_asset_balances_with_decimals = vec![
				(root_asset_id, root_fee_pot_balance, 6),
				(xrp_asset_id, xrp_fee_pot_balance, 6),
				(asto_asset_id, asto_fee_pot_balance, 18),
			];
			let (total_vortex_network_reward, total_vortex_bootstrap, total_vortex) = calculate_vtx(
				&fee_pot_asset_balances_with_decimals,
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
			let staker_pool = total_vortex_bootstrap
				+ (Perquintill::from_percent(30) * total_vortex_network_reward);
			let workpoint_pool = Perquintill::from_percent(70) * total_vortex_network_reward;
			let bob_staker_point_portion =
				Perquintill::from_rational(24600_u128, 17_057_331_103_329);
			let bob_work_points_portion = Perquintill::from_rational(0_u128, 39180127080137_u128);
			let bob_staker_reward_calculated = bob_staker_point_portion * staker_pool;
			let bob_work_points_reward_calculated = bob_work_points_portion * workpoint_pool;
			let bob_vtx_reward_calculated =
				bob_staker_reward_calculated + bob_work_points_reward_calculated;
			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dist_id, bob),
				(bob_vtx_reward_calculated.div(PRECISION_MULTIPLIER), false)
			);
		});
}

#[test]
fn test_precision_in_perwuintill_from_rational() {
	let account_staker_points = 1_u128;
	let total_staker_points = 17057331103329_u128;

	let staker_point_portion =
		Perquintill::from_rational(account_staker_points, total_staker_points);

	assert_eq!(staker_point_portion, Perquintill::from_float(0.000000000000058625_f64));
}

#[test]
fn start_vtx_dist_success_with_attributions() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);
	let partner1: AccountId = create_account(4);
	let partner2: AccountId = create_account(5);

	// asset ids
	let usdc_asset_id = 10;
	let weth_asset_id = 11;

	// vortex vault pre asset balances
	let vortex_vault = Vortex::get_vtx_vault_account();
	let usdc_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let weth_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let root_vtx_vault_balance = 100_u128 * 10_u128.pow(6);
	let xrp_vtx_vault_balance = 100_u128 * 10_u128.pow(6);
	let vtx_current_supply = 1_000_u128 * 10_u128.pow(6);

	// fee pot asset balance
	let fee_vault = Vortex::get_fee_vault_account();
	let usdc_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let weth_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let root_fee_pot_balance = 1_u128 * 10_u128.pow(6);
	let xrp_fee_pot_balance = 10_u128 * 10_u128.pow(6);

	// bootstrap balance
	let root_vault = Vortex::get_root_vault_account();
	let bootstrap_root = 100_u128 * 10_u128.pow(6);

	// attributions
	let attributions = vec![
		(partner1, 1_000_000, Some(Permill::from_percent(5))),
		(partner2, 2_000_000, Some(Permill::from_percent(10))),
	];

	TestExt::default()
		.with_balances(&[
			(alice, 2_000_000),
			(fee_vault, root_fee_pot_balance),
			(vortex_vault, root_vtx_vault_balance),
			(root_vault, bootstrap_root),
		])
		.with_asset(
			<Test as crate::Config>::VtxAssetId::get(),
			"VORTEX",
			&[(charlie, vtx_current_supply)],
		)
		.with_asset_decimals(
			usdc_asset_id,
			"USDC",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, usdc_vtx_vault_balance),
				(fee_vault, usdc_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			weth_asset_id,
			"WETH",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, weth_vtx_vault_balance),
				(fee_vault, weth_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			XRP_ASSET_ID,
			"XRP",
			6,
			&[(vortex_vault, xrp_vtx_vault_balance), (fee_vault, xrp_fee_pot_balance)],
		)
		.with_asset_decimals(
			ROOT_ASSET_ID,
			"ROOT",
			6,
			&[(alice, 1_000_000), (vortex_vault, root_vtx_vault_balance)], // this is just for the metadata
		)
		.with_attributions(&attributions)
		.build()
		.execute_with(|| {
			let vortex_dist_id = NextVortexId::<Test>::get();

			// check account have correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), bootstrap_root);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);
			assert_eq!(AssetsExt::balance(XRP_ASSET_ID, &fee_vault), xrp_fee_pot_balance);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &vortex_vault),
				root_vtx_vault_balance
			);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &vortex_vault), usdc_vtx_vault_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &vortex_vault), weth_vtx_vault_balance);

			// create vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set vortex vault pre asset balances
			let vtx_vault_asset_balances = vec![
				(usdc_asset_id, usdc_vtx_vault_balance),
				(weth_asset_id, weth_vtx_vault_balance),
				(ROOT_ASSET_ID, root_vtx_vault_balance),
				(XRP_ASSET_ID, xrp_vtx_vault_balance),
			];
			assert_ok!(Vortex::set_vtx_vault_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dist_id,
				vtx_current_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances = vec![
				(usdc_asset_id, usdc_fee_pot_balance),
				(weth_asset_id, weth_fee_pot_balance),
				(ROOT_ASSET_ID, root_fee_pot_balance),
				(XRP_ASSET_ID, xrp_fee_pot_balance),
			];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price. prices should be multiplied by the usd factor 10**6
			let usdc_price: Balance = 100_u128 * 10_u128.pow(6);
			let weth_price: Balance = 200_u128 * 10_u128.pow(6);
			let root_price: Balance = 3_u128 * 10_u128.pow(6);
			let xrp_price: Balance = 100_u128 * 10_u128.pow(6); // Example price for XRP
			let asset_prices = vec![
				(usdc_asset_id, usdc_price),
				(weth_asset_id, weth_price),
				(ROOT_ASSET_ID, root_price),
				(XRP_ASSET_ID, xrp_price), // Example asset ID for XRP
			];
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
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);
			assert_eq!(AssetsExt::balance(XRP_ASSET_ID, &fee_vault), xrp_fee_pot_balance);

			// trigger vortex distribution and do the preparations for distribution
			assert_ok!(Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id));
			// Check that the correct events were emitted.
			System::assert_has_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggering {
				id: vortex_dist_id,
			}));
			System::assert_has_event(MockEvent::Vortex(crate::Event::PartnerAttributionsUpdated {
				vtx_id: vortex_dist_id,
			}));
			// run a few blocks to move the Vtx status from Triggering to Triggered
			for i in 2_u32..4 {
				System::set_block_number(i.into());
				Vortex::on_idle(i.into(), Weight::from_all(1_000_000_000_000_u64));
			}
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistributionTriggered {
				id: vortex_dist_id,
			}));

			// check balances have been transferred to vtx vault account
			// check fee pot and bootstrap root account has correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), 0);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), 0);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), 0);
			assert_eq!(AssetsExt::balance(XRP_ASSET_ID, &fee_vault), 0);
			let vtx_vault_account = Vortex::get_vtx_vault_account();
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &vtx_vault_account),
				root_vtx_vault_balance + bootstrap_root + root_fee_pot_balance
			);
			assert_eq!(
				AssetsExt::balance(usdc_asset_id, &vtx_vault_account),
				usdc_fee_pot_balance + usdc_vtx_vault_balance
			);
			assert_eq!(
				AssetsExt::balance(weth_asset_id, &vtx_vault_account),
				weth_fee_pot_balance + weth_vtx_vault_balance
			);
			assert_eq!(
				AssetsExt::balance(XRP_ASSET_ID, &vtx_vault_account),
				xrp_fee_pot_balance + xrp_vtx_vault_balance
			);

			// check VtxPrice tally
			let vtx_vault_asset_balances_with_decimals = vec![
				(usdc_asset_id, usdc_vtx_vault_balance, 18),
				(weth_asset_id, weth_vtx_vault_balance, 18),
				(ROOT_ASSET_ID, root_vtx_vault_balance, 6),
				(XRP_ASSET_ID, xrp_vtx_vault_balance, 6),
			];
			let vtx_price_calculted = calculate_vtx_price(
				&vtx_vault_asset_balances_with_decimals,
				&asset_prices,
				vtx_current_supply,
			);
			assert_eq!(VtxPrice::<Test>::get(vortex_dist_id), vtx_price_calculted);
			// check vtx amounts tally
			let fee_pot_asset_balances_with_decimals = vec![
				(usdc_asset_id, usdc_fee_pot_balance, 18),
				(weth_asset_id, weth_fee_pot_balance, 18),
				(ROOT_ASSET_ID, root_fee_pot_balance, 6),
				(XRP_ASSET_ID, xrp_fee_pot_balance, 6),
			];
			let (total_vortex_network_reward, total_vortex_bootstrap, total_vortex) = calculate_vtx(
				&fee_pot_asset_balances_with_decimals,
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

			// check attributions
			// attributions are noted onchain for future reference
			assert_eq!(PartnerAttributions::<Test>::get(vortex_dist_id), attributions);
			// check attributions on partner attribution pallet is reset
			assert_eq!(MockPartnerAttribution::get_current_attributions(), Vec::new());
			// calculate attribution rewards
			let attribution_rewards_calculated = calculate_attribution_rewards(
				&attributions,
				xrp_price,
				vtx_price_calculted,
				total_vortex_network_reward,
			);
			// assert with onchain value
			assert_eq!(
				PartnerAttributionRewards::<Test>::get(vortex_dist_id),
				attribution_rewards_calculated
			);
			let total_attribution_reward: Balance =
				attribution_rewards_calculated.iter().map(|(_, r)| r).sum();
			assert_eq!(
				total_attribution_reward,
				TotalAttributionRewards::<Test>::get(vortex_dist_id)
			);

			// check bob got the vortex reward registered
			let net_network_reward =
				total_vortex_network_reward.saturating_sub(total_attribution_reward);
			let staker_pool =
				total_vortex_bootstrap + (Perquintill::from_percent(30) * net_network_reward);
			let workpoint_pool = Perquintill::from_percent(70) * net_network_reward;
			let bob_staker_point_portion =
				Perquintill::from_rational(100_000_u128, 100_000_u128 + 100_000_u128);
			let bob_work_points_portion = Perquintill::from_rational(10_u128, 10_u128 + 10_u128);
			let bob_vtx_reward_calculated = (bob_staker_point_portion * staker_pool)
				+ (bob_work_points_portion * workpoint_pool);
			assert_eq!(
				VtxDistOrderbook::<Test>::get(vortex_dist_id, bob),
				(bob_vtx_reward_calculated.div(PRECISION_MULTIPLIER), false)
			);

			// Verify distribution is in Triggered state and can be started
			assert_eq!(VtxDistStatuses::<Test>::get(vortex_dist_id), VtxDistStatus::Triggered);

			// Get initial VTX balances for partners
			let vtx_held_account = Vortex::get_vtx_held_account();
			let partner1_initial_vtx =
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &partner1);
			let partner2_initial_vtx =
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &partner2);
			let vtx_held_initial =
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &vtx_held_account);

			// start the distribution
			assert_ok!(Vortex::start_vtx_dist(Origin::root(), vortex_dist_id));
			// check the status is Paying
			assert_eq!(VtxDistStatuses::<Test>::get(vortex_dist_id), VtxDistStatus::Paying);
			// Check that the correct event was emitted.
			System::assert_last_event(MockEvent::Vortex(crate::Event::VtxDistStarted {
				id: vortex_dist_id,
			}));
			let total_new_vortex_minted =
				TotalVortex::<Test>::get(vortex_dist_id).saturating_div(PRECISION_MULTIPLIER); // in drops

			// Verify partner attribution rewards were distributed
			let partner1_expected_reward = attribution_rewards_calculated
				.iter()
				.find(|(account, _)| account == &partner1)
				.map(|(_, amount)| amount.saturating_div(PRECISION_MULTIPLIER))
				.unwrap_or(0);
			let partner2_expected_reward = attribution_rewards_calculated
				.iter()
				.find(|(account, _)| account == &partner2)
				.map(|(_, amount)| amount.saturating_div(PRECISION_MULTIPLIER))
				.unwrap_or(0);

			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &partner1),
				partner1_initial_vtx + partner1_expected_reward
			);
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &partner2),
				partner2_initial_vtx + partner2_expected_reward
			);
			// held pot should have the total new vortex minted minus the attribution rewards
			assert_eq!(
				AssetsExt::balance(<Test as crate::Config>::VtxAssetId::get(), &vtx_held_account),
				vtx_held_initial + total_new_vortex_minted
					- partner1_expected_reward
					- partner2_expected_reward
			);

			// Check that partner attribution reward events were emitted
			System::assert_has_event(MockEvent::Vortex(
				crate::Event::PartnerAttributionRewardPaid {
					vtx_id: vortex_dist_id,
					account: partner1,
					amount: partner1_expected_reward,
				},
			));
			System::assert_has_event(MockEvent::Vortex(
				crate::Event::PartnerAttributionRewardPaid {
					vtx_id: vortex_dist_id,
					account: partner2,
					amount: partner2_expected_reward,
				},
			));
		});
}

#[test]
fn trigger_vtx_distribution_fails_with_too_many_attribution_partners() {
	let alice: AccountId = create_account(1);
	let bob: AccountId = create_account(2);
	let charlie: AccountId = create_account(3);

	// asset ids
	let usdc_asset_id = 10;
	let weth_asset_id = 11;

	// vortex vault pre asset balances
	let vortex_vault = Vortex::get_vtx_vault_account();
	let usdc_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let weth_vtx_vault_balance = 5_u128 * 10_u128.pow(18);
	let root_vtx_vault_balance = 100_u128 * 10_u128.pow(6);
	let xrp_vtx_vault_balance = 100_u128 * 10_u128.pow(6);
	let vtx_current_supply = 1_000_u128 * 10_u128.pow(6);

	// fee pot asset balance
	let fee_vault = Vortex::get_fee_vault_account();
	let usdc_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let weth_fee_pot_balance = 1_u128 * 10_u128.pow(18);
	let root_fee_pot_balance = 1_u128 * 10_u128.pow(6);
	let xrp_fee_pot_balance = 10_u128 * 10_u128.pow(6);

	// bootstrap balance
	let root_vault = Vortex::get_root_vault_account();
	let bootstrap_root = 100_u128 * 10_u128.pow(6);

	// Create more than MaxAttributionPartners (200) attributions
	let mut attributions = Vec::new();
	for i in 0..201 {
		// 201 partners, exceeding the limit of 200
		let partner = create_account(i + 4); // Start from account 4 to avoid conflicts
		attributions.push((partner, 1_000_000, Some(Permill::from_percent(1))));
	}

	TestExt::default()
		.with_balances(&[
			(alice, 2_000_000),
			(fee_vault, root_fee_pot_balance),
			(vortex_vault, root_vtx_vault_balance),
			(root_vault, bootstrap_root),
		])
		.with_asset(
			<Test as crate::Config>::VtxAssetId::get(),
			"VORTEX",
			&[(charlie, vtx_current_supply)],
		)
		.with_asset_decimals(
			usdc_asset_id,
			"USDC",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, usdc_vtx_vault_balance),
				(fee_vault, usdc_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			weth_asset_id,
			"WETH",
			18,
			&[
				(alice, 1_000_000),
				(vortex_vault, weth_vtx_vault_balance),
				(fee_vault, weth_fee_pot_balance),
			],
		)
		.with_asset_decimals(
			XRP_ASSET_ID,
			"XRP",
			6,
			&[(vortex_vault, xrp_vtx_vault_balance), (fee_vault, xrp_fee_pot_balance)],
		)
		.with_asset_decimals(
			ROOT_ASSET_ID,
			"ROOT",
			6,
			&[(alice, 1_000_000), (vortex_vault, root_vtx_vault_balance)], // this is just for the metadata
		)
		.with_attributions(&attributions)
		.build()
		.execute_with(|| {
			let vortex_dist_id = NextVortexId::<Test>::get();

			// check account have correct balances
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), bootstrap_root);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);
			assert_eq!(AssetsExt::balance(XRP_ASSET_ID, &fee_vault), xrp_fee_pot_balance);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &vortex_vault),
				root_vtx_vault_balance
			);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &vortex_vault), usdc_vtx_vault_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &vortex_vault), weth_vtx_vault_balance);

			// create vortex distribution
			assert_ok!(Vortex::create_vtx_dist(Origin::root()));

			// set vortex vault pre asset balances
			let vtx_vault_asset_balances = vec![
				(usdc_asset_id, usdc_vtx_vault_balance),
				(weth_asset_id, weth_vtx_vault_balance),
				(ROOT_ASSET_ID, root_vtx_vault_balance),
				(XRP_ASSET_ID, xrp_vtx_vault_balance),
			];
			assert_ok!(Vortex::set_vtx_vault_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(vtx_vault_asset_balances.clone()).unwrap(),
			));
			// set Vtx current supply
			assert_ok!(Vortex::set_vtx_total_supply(
				Origin::root(),
				vortex_dist_id,
				vtx_current_supply,
			));

			// set fee pot asset balances
			let fee_pot_asset_balances = vec![
				(usdc_asset_id, usdc_fee_pot_balance),
				(weth_asset_id, weth_fee_pot_balance),
				(ROOT_ASSET_ID, root_fee_pot_balance),
				(XRP_ASSET_ID, xrp_fee_pot_balance),
			];
			assert_ok!(Vortex::set_fee_pot_asset_balances(
				Origin::root(),
				vortex_dist_id,
				BoundedVec::try_from(fee_pot_asset_balances.clone()).unwrap(),
			));

			//set asset price. prices should be multiplied by the usd factor 10**6
			let usdc_price: Balance = 100_u128 * 10_u128.pow(6);
			let weth_price: Balance = 200_u128 * 10_u128.pow(6);
			let root_price: Balance = 3_u128 * 10_u128.pow(6);
			let xrp_price: Balance = 100_u128 * 10_u128.pow(6); // Example price for XRP
			let asset_prices = vec![
				(usdc_asset_id, usdc_price),
				(weth_asset_id, weth_price),
				(ROOT_ASSET_ID, root_price),
				(XRP_ASSET_ID, xrp_price), // Example asset ID for XRP
			];
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
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);
			assert_eq!(AssetsExt::balance(XRP_ASSET_ID, &fee_vault), xrp_fee_pot_balance);

			// Verify that attributions exceed the limit
			assert_eq!(attributions.len(), 201);
			assert!(attributions.len() > 200); // MaxAttributionPartners limit

			// Attempt to trigger vortex distribution with too many attribution partners
			// This should fail because the attributions exceed MaxAttributionPartners
			assert_noop!(
				Vortex::trigger_vtx_distribution(Origin::root(), vortex_dist_id),
				crate::Error::<Test>::ExceededMaxPartners
			);

			// Verify that balances remain unchanged
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &root_vault), bootstrap_root);
			assert_eq!(AssetsExt::balance(usdc_asset_id, &fee_vault), usdc_fee_pot_balance);
			assert_eq!(AssetsExt::balance(weth_asset_id, &fee_vault), weth_fee_pot_balance);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &fee_vault), root_fee_pot_balance);
			assert_eq!(AssetsExt::balance(XRP_ASSET_ID, &fee_vault), xrp_fee_pot_balance);
		});
}

#[test]
fn test_calculate_attribution_rewards_with_print() {
	TestExt::default().build().execute_with(|| {
		/*[
		  [
			0x2F47E09860B4DC7326A1c1Ba5A2E15158ee92020
			20,000,000
			2.00%
		  ]
		  [
			0x3A09deEc70E73482Ffc6C63E9b6F19cd5bbD9D17
			100,000,000
			1.00%
		  ]
		  [
			0xDB25222D3a898194f5932c7D439f54F4540c633f
			200,000,000
			2.00%
		  ]
		  [
			0x102250882E43273F02149B1228261c4E48EcCDa8
			40,000,000
			1.00%
		  ]
		] */
		// Test inputs using real attribution data
		let attributions = vec![
			(alice(), 20_000_000, Some(Permill::from_percent(2))), // 20,000,000 in drops (6 decimals), 2.00%
			(bob(), 100_000_000, Some(Permill::from_percent(1))),  // 100,000,000 in drops (6 decimals), 1.00%
			(charlie(), 200_000_000, Some(Permill::from_percent(2))), // 200,000,000 in drops (6 decimals), 2.00%
			(dave(), 40_000_000, Some(Permill::from_percent(1))),     // 40,000,000 in drops (6 decimals), 1.00%
		];

		let xrp_price = 2_574_200; // 1 XRP = 1 USD (with price multiplier)
		let vtx_price = 2_972_196; // 2 USD per VTX (with price multiplier)
		let total_network_reward = 15_799_675_338_369_340; // 1B VTX (with precision multiplier)

		println!("=== Attribution Rewards Calculation Test ===");
		println!("Inputs:");
		println!("  XRP Price: {} (with multiplier)", xrp_price);
		println!("  VTX Price: {} (with multiplier)", vtx_price);
		println!("  Total Network Reward: {} (with precision multiplier)", total_network_reward);
		println!("  Attributions:");
		for (i, (account, amount, fee_percentage)) in attributions.iter().enumerate() {
			println!(
				"    {}: Account={:?}, Amount={}, Fee%={:?}",
				i + 1,
				account,
				amount,
				fee_percentage
			);
		}

		// Calculate attribution rewards
		let attribution_rewards = calculate_attribution_rewards(
			&attributions,
			xrp_price,
			vtx_price,
			total_network_reward,
		);

		println!("\nResults:");
		println!("  Total Partners with Rewards: {}", attribution_rewards.len());
		println!("  Individual Rewards:");
		println!("{:?}", attribution_rewards);
	});
}
