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
use crate::mock::AssetsExt;
use frame_support::{assert_noop, assert_ok};
use hex::encode;
use mock::{Dex, Event as MockEvent, Origin, System, Test, TestExt};
use seed_primitives::AccountId;
use sp_core::H160;
use sp_runtime::{traits::BadOrigin, ArithmeticError, DispatchError};
use std::str::FromStr;

fn create_account(seed: u64) -> AccountId {
	AccountId::from(H160::from_low_u64_be(seed))
}
/// x * 10e18
fn to_eth(amount: u128) -> u128 {
	amount * 1_000_000_000_000_000_000_u128
}

#[test]
fn test_run() {
	TestExt::default().build().execute_with(|| assert_eq!(1, 1));
}

#[test]
fn disable_trading_pair() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);
		let alice: AccountId = create_account(1);
		// create 2 tokens
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&alice, None).unwrap();

		// normal user can not disable trading_pair
		assert_noop!(Dex::disable_trading_pair(Origin::signed(alice), usdc, weth), BadOrigin);

		// lp token must exist
		assert_noop!(
			Dex::disable_trading_pair(Origin::root(), usdc, weth),
			Error::<Test>::LiquidityProviderTokenNotCreated
		);

		// manually create LP token and enable it
		TradingPairLPToken::<Test>::insert(TradingPair::new(usdc, weth), Some(3));
		TradingPairStatuses::<Test>::insert(
			TradingPair::new(usdc, weth),
			TradingPairStatus::Enabled,
		);

		// disable trading pair successful
		assert_ok!(Dex::disable_trading_pair(Origin::root(), usdc, weth));
		System::assert_last_event(MockEvent::Dex(crate::Event::DisableTradingPair(
			TradingPair::new(usdc, weth),
		)));
		assert_eq!(
			Dex::trading_pair_statuses(TradingPair::new(usdc, weth)),
			TradingPairStatus::NotEnabled
		);

		// disabling trading pair will fail if already disabled
		assert_noop!(
			Dex::disable_trading_pair(Origin::root(), usdc, weth),
			Error::<Test>::MustBeEnabled,
		);
	});
}

#[test]
fn reenable_trading_pair() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		let alice: AccountId = create_account(1);

		// create 2 tokens
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&alice, None).unwrap();

		// normal user can not enable trading_pair
		assert_noop!(Dex::reenable_trading_pair(Origin::signed(alice), usdc, weth), BadOrigin);

		// lp token must exist
		assert_noop!(
			Dex::reenable_trading_pair(Origin::root(), usdc, weth),
			Error::<Test>::LiquidityProviderTokenNotCreated
		);

		// check that pair LP token does not exist
		assert_eq!(Dex::lp_token_id(TradingPair::new(usdc, weth)).is_some(), false);

		// manually create LP token and enable it
		TradingPairLPToken::<Test>::insert(TradingPair::new(usdc, weth), Some(3));
		TradingPairStatuses::<Test>::insert(
			TradingPair::new(usdc, weth),
			TradingPairStatus::Enabled,
		);

		// re-enabling should fail for not-enabled trading pair
		assert_noop!(
			Dex::reenable_trading_pair(Origin::root(), usdc, weth),
			Error::<Test>::MustBeNotEnabled,
		);

		// manually disable trading pair
		<TradingPairStatuses<Test>>::insert(
			TradingPair::new(usdc, weth),
			TradingPairStatus::NotEnabled,
		);

		// a disabled trading pair can be re-enabled
		assert_ok!(Dex::reenable_trading_pair(Origin::root(), usdc, weth));
		assert_eq!(
			Dex::trading_pair_statuses(TradingPair::new(usdc, weth)),
			TradingPairStatus::Enabled
		);
		System::assert_last_event(MockEvent::Dex(crate::Event::EnableTradingPair(
			TradingPair::new(usdc, weth),
		)));

		// cannot enable again
		assert_noop!(
			Dex::reenable_trading_pair(Origin::root(), weth, usdc),
			Error::<Test>::MustBeNotEnabled
		);
	});
}

#[test]
fn trading_pair_pool_address() {
	TestExt::default().build().execute_with(|| {
		let alice: AccountId = create_account(1);

		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&alice, None).unwrap();
		assert_eq!(usdc, 1124);
		assert_eq!(weth, 2148);

		let pool_address: H160 = TradingPair::new(usdc, weth).pool_address::<AccountId>().into();

		let expected_pool_address =
			H160::from_str("dddddddd00000464000008640000000000000000").unwrap();
		assert_eq!(pool_address.to_string(), expected_pool_address.to_string());

		let pool_address_reverse: H160 =
			TradingPair::new(weth, usdc).pool_address::<AccountId>().into();
		assert_eq!(pool_address_reverse, expected_pool_address);

		let hex_address = pool_address.to_fixed_bytes();
		let usdc_hex = &hex_address[5..8]; // 2nd 4 bytes
		let weth_hex = &hex_address[9..12]; // 3rd 4 bytes

		let usdc_decimal = u32::from_str_radix(&hex::encode(usdc_hex), 16).unwrap();
		let weth_decimal = u32::from_str_radix(&hex::encode(weth_hex), 16).unwrap();

		assert_eq!(usdc_decimal, usdc);
		assert_eq!(weth_decimal, weth);
	});
}

#[test]
fn quote() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		// quote fails if amount_a is 0
		assert_noop!(Dex::quote(U256::zero(), 0, 0), Error::<Test>::InsufficientAmount);

		// quote fails if either reserves are 0
		assert_noop!(Dex::quote(U256::from(100), 0, 0), Error::<Test>::InsufficientLiquidity);
		assert_noop!(
			Dex::quote(U256::from(100), 0, 100_u128),
			Error::<Test>::InsufficientLiquidity
		);
		assert_noop!(
			Dex::quote(U256::from(100), 100_u128, 0),
			Error::<Test>::InsufficientLiquidity
		);

		// quote succeeds if amount and reserves are non-zero
		assert_eq!(Dex::quote(U256::from(100), 100_u128, 100_u128), Ok(U256::from(100)));
		assert_eq!(Dex::quote(U256::from(200), 100_u128, 100_u128), Ok(U256::from(200)));
		assert_eq!(Dex::quote(U256::from(1), 100_u128, 100_u128), Ok(U256::from(1)));
		assert_eq!(Dex::quote(U256::from(200), 1_u128, 1_u128), Ok(U256::from(200)));
		assert_eq!(Dex::quote(U256::from(200), 1_u128, 200_u128), Ok(U256::from(40_000)));
	});
}

#[test]
fn create_lp_token() {
	TestExt::default().build().execute_with(|| {
		let alice: AccountId = create_account(1);

		let usdc =
			AssetsExt::create_with_metadata(&alice, "usdc".into(), "usdc".into(), 6, None).unwrap();
		let weth = AssetsExt::create_with_metadata(&alice, "weth".into(), "weth".into(), 18, None)
			.unwrap();

		assert_eq!(usdc, 1124);
		assert_eq!(weth, 2148);

		let usdc_symbol_bytes = AssetsExt::symbol(&usdc);
		let weth_symbol_bytes = AssetsExt::symbol(&weth);
		let usdc_symbol = sp_std::str::from_utf8(&usdc_symbol_bytes).unwrap();
		let weth_symbol = sp_std::str::from_utf8(&weth_symbol_bytes).unwrap();

		assert_eq!(usdc_symbol, "usdc");
		assert_eq!(weth_symbol, "weth");

		let trading_pair = TradingPair::new(usdc, weth);

		let lp_token = Dex::create_lp_token(&trading_pair).unwrap();
		assert_eq!(lp_token, 3172);

		let lp_token_name_bytes =
			<AssetsExt as frame_support::traits::fungibles::InspectMetadata<AccountId>>::name(
				&lp_token,
			);
		let lp_token_symbol_bytes = AssetsExt::symbol(&lp_token);
		let lp_token_name = sp_std::str::from_utf8(&lp_token_name_bytes).unwrap();
		let lp_token_symbol = sp_std::str::from_utf8(&lp_token_symbol_bytes).unwrap();

		assert_eq!(lp_token_name, "LP usdc weth");
		assert_eq!(lp_token_symbol, "LP-1124-2148");
	});
}

#[test]
fn create_lp_token_long_symbol() {
	TestExt::default().build().execute_with(|| {
		let alice: AccountId = create_account(1);

		let usdc = AssetsExt::create_with_metadata(
			&alice,
			"usdc".into(),
			"usdc-something-very-long".into(),
			6,
			None,
		)
		.unwrap();
		let weth = AssetsExt::create_with_metadata(
			&alice,
			"weth".into(),
			"weth-symbol-very-very-long".into(),
			18,
			None,
		)
		.unwrap();

		assert_eq!(usdc, 1124);
		assert_eq!(weth, 2148);

		let usdc_symbol_bytes = AssetsExt::symbol(&usdc);
		let weth_symbol_bytes = AssetsExt::symbol(&weth);
		let usdc_symbol = sp_std::str::from_utf8(&usdc_symbol_bytes).unwrap();
		let weth_symbol = sp_std::str::from_utf8(&weth_symbol_bytes).unwrap();

		assert_eq!(usdc_symbol, "usdc-something-very-long");
		assert_eq!(weth_symbol, "weth-symbol-very-very-long");

		let trading_pair = TradingPair::new(usdc, weth);

		let lp_token = Dex::create_lp_token(&trading_pair).unwrap();
		assert_eq!(lp_token, 3172);

		let lp_token_name_bytes =
			<AssetsExt as frame_support::traits::fungibles::InspectMetadata<AccountId>>::name(
				&lp_token,
			);
		let lp_token_symbol_bytes = AssetsExt::symbol(&lp_token);
		let lp_token_name = sp_std::str::from_utf8(&lp_token_name_bytes).unwrap();
		let lp_token_symbol = sp_std::str::from_utf8(&lp_token_symbol_bytes).unwrap();

		assert_eq!(lp_token_name, "LP usdc-something-very- weth-symbol-very-ver");
		assert_eq!(lp_token_symbol, "LP-1124-2148");
	});
}

#[test]
fn add_liquidity() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);
		let alice: AccountId = create_account(1);
		let bob: AccountId = create_account(2);
		let charlie: AccountId = create_account(3);

		// create 2 tokens
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&bob, None).unwrap();

		// mint tokens to user
		assert_ok!(AssetsExt::mint_into(usdc, &alice, to_eth(1)));
		assert_ok!(AssetsExt::mint_into(weth, &alice, to_eth(1)));
		assert_ok!(Dex::add_liquidity(
			Origin::signed(alice),
			usdc,
			weth,
			to_eth(1),
			to_eth(1),
			to_eth(1),
			to_eth(1),
			None,
			None,
		));

		// adding LP enables trading pair
		assert_eq!(
			Dex::trading_pair_statuses(TradingPair::new(usdc, weth)),
			TradingPairStatus::Enabled
		);

		System::assert_last_event(MockEvent::Dex(crate::Event::AddLiquidity(
			alice,
			usdc,
			to_eth(1),
			weth,
			to_eth(1),
			999_999_999_999_999_000u128, // lp token shares
			alice,
		)));

		// mint tokens to alice
		assert_ok!(AssetsExt::mint_into(usdc, &alice, to_eth(1)));
		assert_ok!(AssetsExt::mint_into(weth, &alice, to_eth(1)));

		// add liquidity from alice to charlie
		assert_ok!(Dex::add_liquidity(
			Origin::signed(alice),
			usdc,
			weth,
			to_eth(1),
			to_eth(1),
			to_eth(1),
			to_eth(1),
			Some(charlie),
			None,
		));

		System::assert_last_event(MockEvent::Dex(crate::Event::AddLiquidity(
			alice,
			usdc,
			to_eth(1),
			weth,
			to_eth(1),
			1_000_000_000_000_000_000u128, // lp token shares
			charlie,
		)));

		// the created lp token should be the 3rd created token (first 22bit) + 100 (last 10bits)
		assert_eq!(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), 3 << 10 | 100);

		// check that the next asset id should be 4 (2 assets + 1 lp token)

		// lp token is the same token independent of trading pair token ordering
		assert_eq!(
			Dex::lp_token_id(TradingPair::new(usdc, weth)),
			Dex::lp_token_id(TradingPair::new(weth, usdc))
		);

		// verify Alice now has LP tokens
		assert_eq!(
			AssetsExt::balance(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), &alice),
			999_999_999_999_999_000u128,
		);

		// verify Charlie now has LP tokens
		assert_eq!(
			AssetsExt::balance(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), &charlie),
			1_000_000_000_000_000_000u128,
		);

		// mint tokens to new user
		assert_ok!(AssetsExt::mint_into(usdc, &bob, to_eth(2)));
		assert_ok!(AssetsExt::mint_into(weth, &bob, to_eth(2)));

		// add liquidity to new user fails - as the deadline has been missed
		assert_noop!(
			Dex::add_liquidity(
				Origin::signed(bob),
				usdc,
				weth,
				to_eth(2),
				to_eth(2),
				to_eth(2),
				to_eth(2),
				None,
				Some(0),
			),
			Error::<Test>::DeadlineMissed
		);

		// add liquidity to new user succeeds - as the deadline meets
		assert_ok!(Dex::add_liquidity(
			Origin::signed(bob),
			usdc,
			weth,
			to_eth(2),
			to_eth(2),
			to_eth(2),
			to_eth(2),
			None,
			Some(2),
		));

		// verify Bob now has LP tokens
		assert_eq!(
			AssetsExt::balance(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), &bob),
			to_eth(2),
		);

		// bob should have more LP tokens than Alice as Bob provisioned more liquidity
		assert_eq!(
			AssetsExt::balance(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), &alice) <
				AssetsExt::balance(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), &bob),
			true
		);

		// disable trading pair
		TradingPairStatuses::<Test>::insert(
			TradingPair::new(usdc, weth),
			TradingPairStatus::NotEnabled,
		);

		// user cannot add liquidity to disabled pair
		assert_noop!(
			Dex::add_liquidity(
				Origin::signed(bob),
				usdc,
				weth,
				2_000_000_000_000u128,
				2_000_000_000_000u128,
				2_000_000_000_000u128,
				2_000_000_000_000u128,
				None,
				None,
			),
			Error::<Test>::MustBeEnabled
		);
	});
}

#[test]
fn add_shared_liquidity() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);
		let alice: AccountId = create_account(1);
		let bob: AccountId = create_account(2);

		// create 3 tokens
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&bob, None).unwrap();
		let asto = AssetsExt::create(&bob, None).unwrap();

		// mint tokens to user
		assert_ok!(AssetsExt::mint_into(usdc, &alice, to_eth(1)));
		assert_ok!(AssetsExt::mint_into(weth, &alice, to_eth(1)));
		// add liquidity <usdc-weth>
		assert_ok!(Dex::add_liquidity(
			Origin::signed(alice),
			usdc,
			weth,
			to_eth(1),
			to_eth(1),
			to_eth(1),
			to_eth(1),
			None,
			None,
		));
		let trading_pair: TradingPair = TradingPair::new(usdc, weth);

		// adding LP enables trading pair
		assert_eq!(Dex::trading_pair_statuses(trading_pair), TradingPairStatus::Enabled);

		System::assert_last_event(MockEvent::Dex(crate::Event::AddLiquidity(
			alice,
			usdc,
			to_eth(1),
			weth,
			to_eth(1),
			999_999_999_999_999_000u128, // lp token shares
			alice,
		)));

		// lp token is the same token independent of trading pair token ordering
		assert_eq!(
			Dex::lp_token_id(TradingPair::new(usdc, weth)),
			Dex::lp_token_id(TradingPair::new(weth, usdc))
		);

		// mint tokens to new user
		assert_ok!(AssetsExt::mint_into(usdc, &bob, to_eth(2)));
		assert_ok!(AssetsExt::mint_into(asto, &bob, to_eth(2)));

		// add liquidity to new user succeeds
		// add liquidity <usdc-asto>
		assert_ok!(Dex::add_liquidity(
			Origin::signed(bob),
			usdc,
			asto,
			to_eth(2),
			to_eth(2),
			to_eth(2),
			to_eth(2),
			None,
			None,
		));

		let pool_address: AccountId = trading_pair.pool_address::<AccountId>();

		let (reserve_0, reserve_1) = LiquidityPool::<Test>::get(trading_pair);
		let balance_0 = AssetsExt::balance(trading_pair.0, &pool_address);
		let balance_1 = AssetsExt::balance(trading_pair.1, &pool_address);

		assert_eq!(reserve_0, balance_0);
		assert_eq!(reserve_1, balance_1);

		let trading_pair_2: TradingPair = TradingPair::new(asto, usdc);
		let pool_address: AccountId = trading_pair_2.pool_address::<AccountId>();

		let (reserve_2, reserve_3) = LiquidityPool::<Test>::get(trading_pair_2);
		let balance_2 = AssetsExt::balance(trading_pair_2.0, &pool_address);
		let balance_3 = AssetsExt::balance(trading_pair_2.1, &pool_address);

		assert_eq!(reserve_2, balance_2);
		assert_eq!(reserve_3, balance_3);

		assert_ok!(AssetsExt::mint_into(usdc, &bob, to_eth(2)));

		// swap <usdc/weth>
		assert_ok!(Dex::swap_with_exact_supply(
			Origin::signed(bob),
			to_eth(1), // input usdc
			0u128,     // min expected weth
			vec![usdc, weth],
			None,
			None,
		));

		// validate reserves for usdc/weth have been updated after swap
		let (reserve_0_0, reserve_1_0) = LiquidityPool::<Test>::get(trading_pair);
		assert_ne!(reserve_0, reserve_0_0);
		assert_ne!(reserve_1, reserve_1_0);
		assert_eq!(reserve_0, 1000000000000000000);
		assert_eq!(reserve_0_0, 2000000000000000000);
		assert_eq!(reserve_1, 1000000000000000000);
		assert_eq!(reserve_1_0, 500751126690035053);

		// validate pool address for usdc/weth has accumulated liquidity/tokens after the swap
		let pool_address: AccountId = trading_pair.pool_address::<AccountId>();
		let balance_0_0 = AssetsExt::balance(trading_pair.0, &pool_address);
		let balance_1_0 = AssetsExt::balance(trading_pair.1, &pool_address);
		assert_ne!(balance_0, balance_0_0); // balance should change before and after swap
		assert_ne!(balance_1, balance_1_0); // balance should change before and after swap
		assert_eq!(balance_0, 1000000000000000000);
		assert_eq!(balance_0_0, 2000000000000000000);
		assert_eq!(balance_1, 1000000000000000000);
		assert_eq!(balance_1_0, 500751126690035053);

		// validate reserves for usdc/asto stays same after swap
		let (reserve_2_0, reserve_3_0) = LiquidityPool::<Test>::get(trading_pair_2);
		assert_eq!(reserve_2, reserve_2_0);
		assert_eq!(reserve_3, reserve_3_0);

		// validate pool address for usdc/asto has not accumulated liquidity/tokens after the swap
		let pool_address: AccountId = trading_pair_2.pool_address::<AccountId>();
		let balance_2_0 = AssetsExt::balance(trading_pair_2.0, &pool_address);
		let balance_3_0 = AssetsExt::balance(trading_pair_2.1, &pool_address);
		assert_eq!(balance_2, balance_2_0); // balance should not change
		assert_eq!(balance_3, balance_3_0); // balance should not change

		assert_ok!(AssetsExt::mint_into(usdc, &bob, to_eth(2)));

		// swap <usdc/asto>
		assert_ok!(Dex::swap_with_exact_supply(
			Origin::signed(bob),
			to_eth(1), // input usdc
			0u128,     // min expected asto
			vec![usdc, asto],
			None,
			None,
		));

		// validate reserves for usdc/asto change after swap
		let (reserve_2_0_0, reserve_3_0_0) = LiquidityPool::<Test>::get(trading_pair_2);
		assert_ne!(reserve_2_0, reserve_2_0_0);
		assert_ne!(reserve_3_0, reserve_3_0_0);
		assert_eq!(reserve_2_0, 2000000000000000000);
		assert_eq!(reserve_2_0_0, 3000000000000000000);
		assert_eq!(reserve_3_0, 2000000000000000000);
		assert_eq!(reserve_3_0_0, 1334668001334668002);

		// validate pool address for usdc/asto has accumulated liquidity/tokens after the swap
		let pool_address: AccountId = trading_pair_2.pool_address::<AccountId>();
		let balance_2_0_0 = AssetsExt::balance(trading_pair_2.0, &pool_address);
		let balance_3_0_0 = AssetsExt::balance(trading_pair_2.1, &pool_address);
		assert_ne!(balance_2_0, balance_2_0_0); // balance should change before and after swap
		assert_ne!(balance_3_0, balance_3_0_0); // balance should change before and after swap
		assert_eq!(balance_2_0, 2000000000000000000);
		assert_eq!(balance_2_0_0, 3000000000000000000);
		assert_eq!(balance_3_0, 2000000000000000000);
		assert_eq!(balance_3_0_0, 1334668001334668002);

		// validate reserves for usdc/weth stay same after swap
		let (reserve_0_0_0, reserve_1_0_0) = LiquidityPool::<Test>::get(trading_pair);
		assert_eq!(reserve_0_0, reserve_0_0_0);
		assert_eq!(reserve_1_0, reserve_1_0_0);

		// validate pool address for usdc/weth does not accumulate liquidity/tokens after the swap
		let pool_address: AccountId = trading_pair.pool_address::<AccountId>();
		let balance_0_0_0 = AssetsExt::balance(trading_pair.0, &pool_address);
		let balance_1_0_0 = AssetsExt::balance(trading_pair.1, &pool_address);
		assert_eq!(balance_0_0, balance_0_0_0); // balance should change before and after swap
		assert_eq!(balance_1_0, balance_1_0_0); // balance should change before and after swap
	});
}

// unit test
#[test]
fn get_trading_pair_address() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		let alice: AccountId = create_account(1);

		// create 2 tokens
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&alice, None).unwrap();

		// TradingPair::new(usdc, weth);
		let trading_pair = TradingPair::new(usdc, weth);
		let pool_address: AccountId = trading_pair.pool_address::<AccountId>();
		let pool_address = encode(H160(pool_address.into()).as_bytes());
		assert_eq!(pool_address, "dddddddd00000464000008640000000000000000");

		let trading_pair_reverse = TradingPair::new(weth, usdc);
		let pool_address_reverse: AccountId = trading_pair_reverse.pool_address::<AccountId>();
		let pool_address_reverse = encode(H160(pool_address_reverse.into()).as_bytes());
		assert_eq!(pool_address_reverse, "dddddddd00000464000008640000000000000000");

		assert_eq!(pool_address, pool_address_reverse);
	});
}
/// https://github.com/futureversecom/seed/issues/15
#[test]
fn add_liquidity_issue_15() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		let alice: AccountId = create_account(1);
		let bob: AccountId = create_account(2);

		// create 2 tokens
		let usdc = AssetsExt::create(&alice.clone(), None).unwrap();
		let weth = AssetsExt::create(&bob.clone(), None).unwrap();

		// mint tokens to user
		assert_ok!(AssetsExt::mint_into(usdc, &alice, to_eth(10)));
		assert_ok!(AssetsExt::mint_into(weth, &alice, to_eth(10)));
		assert_ok!(Dex::add_liquidity(
			Origin::signed(alice),
			usdc,
			weth,
			to_eth(1),
			to_eth(1),
			to_eth(1),
			to_eth(1),
			None,
			None,
		));

		assert_ok!(Dex::add_liquidity(
			Origin::signed(alice),
			usdc,
			weth,
			to_eth(2),
			to_eth(1),
			to_eth(1),
			to_eth(1),
			None,
			None,
		));
		assert_eq!(
			AssetsExt::balance(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), &alice),
			1_999_999_999_999_999_000_u128,
		);
		assert_eq!(AssetsExt::balance(usdc, &alice), 8_000_000_000_000_000_000_u128);
		assert_eq!(AssetsExt::balance(weth, &alice), 8_000_000_000_000_000_000_u128);
	});
}

#[test]
fn remove_liquidity_simple() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		let alice: AccountId = create_account(1);
		let bob: AccountId = create_account(2);

		// create 2 tokens (by different users)
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&bob, None).unwrap();

		// add liquidity as user
		assert_ok!(AssetsExt::mint_into(usdc, &alice, to_eth(2)));
		assert_ok!(AssetsExt::mint_into(weth, &alice, to_eth(2)));
		assert_ok!(Dex::add_liquidity(
			Origin::signed(alice),
			usdc,
			weth,
			to_eth(2),
			to_eth(2),
			to_eth(2),
			to_eth(2),
			None,
			None,
		));

		let lp_token_id = Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap();
		assert_eq!(AssetsExt::balance(lp_token_id, &alice), 1_999_999_999_999_999_000u128);
		assert_eq!(AssetsExt::balance(usdc, &alice), 0);
		assert_eq!(AssetsExt::balance(weth, &alice), 0);

		// providing all-1 LP token shares should fail - deadline is in the past
		assert_noop!(
			Dex::remove_liquidity(
				Origin::signed(alice),
				usdc,
				weth,
				1_999_999_999_999_999_000u128, // all lp -1 to retrieve input tokens
				0u128,                         // ignoring expected input token liquidity
				0u128,                         // ignoring expected input token liquidity
				Some(bob),                     // Bob is the token recipient
				Some(0),
			),
			Error::<Test>::DeadlineMissed
		);

		// providing all-1 LP token shares should succeed
		assert_ok!(Dex::remove_liquidity(
			Origin::signed(alice),
			usdc,
			weth,
			1_999_999_999_999_999_000u128, // all lp -1 to retrieve input tokens
			0u128,                         // ignoring expected input token liquidity
			0u128,                         // ignoring expected input token liquidity
			Some(bob),                     // Bob is the token recipient
			Some(2),
		));

		System::assert_last_event(MockEvent::Dex(crate::Event::RemoveLiquidity(
			alice,
			usdc,
			1_999_999_999_999_999_000u128,
			weth,
			1_999_999_999_999_999_000u128,
			1_999_999_999_999_999_000u128,
			bob,
		)));

		assert_eq!(
			AssetsExt::balance(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), &alice),
			0,
		);
		assert_eq!(AssetsExt::balance(usdc, &bob), 1_999_999_999_999_999_000u128);
		assert_eq!(AssetsExt::balance(weth, &bob), 1_999_999_999_999_999_000u128);
	});
}

#[test]
fn remove_liquidity_full() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		let alice: AccountId = create_account(1);
		let bob: AccountId = create_account(2);

		// create 2 tokens (by different users)
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&bob, None).unwrap();

		// fails if no LP tokens withdrawn
		assert_eq!(
			Dex::remove_liquidity(
				Origin::signed(alice),
				usdc,
				weth,
				0u128,
				2u128,
				2u128,
				None,
				None
			)
			.is_ok(),
			false
		);

		// remove liquidity fails if LP token doesnt exist
		assert_noop!(
			Dex::remove_liquidity(
				Origin::signed(alice),
				usdc,
				weth,
				1u128,
				2u128,
				2u128,
				None,
				None,
			),
			Error::<Test>::InvalidAssetId
		);

		// maually create and enable LP token
		let lp_token_id = AssetsExt::create(&alice, None).unwrap();
		TradingPairLPToken::<Test>::insert(TradingPair::new(usdc, weth), Some(lp_token_id));
		TradingPairStatuses::<Test>::insert(
			TradingPair::new(usdc, weth),
			TradingPairStatus::Enabled,
		);

		// add liquidity as user
		assert_ok!(AssetsExt::mint_into(usdc, &alice, to_eth(2)));
		assert_ok!(AssetsExt::mint_into(weth, &alice, to_eth(2)));
		assert_ok!(Dex::add_liquidity(
			Origin::signed(alice),
			usdc,
			weth,
			to_eth(2),
			to_eth(2),
			to_eth(2),
			to_eth(2),
			None,
			None,
		));
		let lp_token_id = Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(); // TODO remove
		assert_eq!(AssetsExt::balance(lp_token_id, &alice), 1_999_999_999_999_999_000u128);
		assert_eq!(AssetsExt::balance(usdc, &alice), 0);
		assert_eq!(AssetsExt::balance(weth, &alice), 0);

		// remove liquidity fails if user expects more balance of a token than they have
		assert_noop!(
			Dex::remove_liquidity(
				Origin::signed(alice),
				usdc,
				weth,
				1u128,
				to_eth(2) + 1, // more balance than user had LPed
				0u128,
				None,
				None,
			),
			Error::<Test>::InsufficientWithdrawnAmountA
		);

		assert_noop!(
			Dex::remove_liquidity(
				Origin::signed(alice),
				usdc,
				weth,
				1u128,
				0u128,
				to_eth(2) + 1, // more balance than user had LPed
				None,
				None,
			),
			Error::<Test>::InsufficientWithdrawnAmountB
		);

		assert_noop!(
			Dex::remove_liquidity(
				Origin::signed(alice),
				usdc,
				weth,
				100u128, // provided LP token shares too low to retrieve input tokens
				2_000_000_000_000u128,
				2_000_000_000_000u128,
				None,
				None,
			),
			Error::<Test>::InsufficientWithdrawnAmountA
		);

		// providing all-1 LP token shares should succeed
		assert_ok!(Dex::remove_liquidity(
			Origin::signed(alice),
			usdc,
			weth,
			1_999_999_999_999_999_000u128 - 1, // all lp -1 to retrieve input tokens
			0u128,                             // ignoring expected input token liquidity
			0u128,                             // ignoring expected input token liquidity
			None,
			None,
		));

		System::assert_last_event(MockEvent::Dex(crate::Event::RemoveLiquidity(
			alice,
			usdc,
			1_999_999_999_999_998_999_u128,
			weth,
			1_999_999_999_999_998_999_u128,
			1_999_999_999_999_998_999_u128,
			alice,
		)));

		assert_eq!(
			AssetsExt::balance(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), &alice),
			1,
		);
		assert_eq!(AssetsExt::balance(usdc, &alice), 1_999_999_999_999_998_999_u128);
		assert_eq!(AssetsExt::balance(weth, &alice), 1_999_999_999_999_998_999_u128);

		// disable trading pair
		TradingPairStatuses::<Test>::insert(
			TradingPair::new(usdc, weth),
			TradingPairStatus::NotEnabled,
		);

		// can still successfully remove liquidity if trading pair is disabled
		// remove last lp token remaining
		assert_ok!(Dex::remove_liquidity(
			Origin::signed(alice),
			usdc,
			weth,
			1,
			0u128, // ignoring expected input token liquidity
			0u128, // ignoring expected input token liquidity
			None,
			None,
		));

		// removing all liquidity should imply user has recieved all input tokens
		assert_eq!(
			AssetsExt::balance(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), &alice),
			0u128,
		);
		// do not get 100% tokens back as some lost due to minimum liquidity minting
		assert_eq!(AssetsExt::balance(usdc, &alice), 1_999_999_999_999_999_000_u128);
		assert_eq!(AssetsExt::balance(weth, &alice), 1_999_999_999_999_999_000_u128);
	});
}

#[test]
fn swap_with_exact_supply() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		let alice: AccountId = create_account(1);
		let bob: AccountId = create_account(2);
		let charlie: AccountId = create_account(3);

		let weth = AssetsExt::create(&alice, None).unwrap();
		let usdc = AssetsExt::create(&alice, None).unwrap();

		// mint tokens to user
		assert_ok!(AssetsExt::mint_into(usdc, &alice, to_eth(100)));
		assert_ok!(AssetsExt::mint_into(weth, &alice, to_eth(100)));

		// provide liquidity - note: differing amount of input tokens - ratio 1:2
		assert_ok!(Dex::add_liquidity(
			Origin::signed(alice),
			weth,
			usdc,
			to_eth(1),
			to_eth(2),
			to_eth(1),
			to_eth(2),
			None,
			None,
		));
		assert_eq!(AssetsExt::balance(weth, &alice), to_eth(100) - to_eth(1));
		assert_eq!(AssetsExt::balance(usdc, &alice), to_eth(100) - to_eth(2));

		// swap should fail if the deadline is in the past
		assert_noop!(
			Dex::swap_with_exact_supply(
				Origin::signed(bob),
				to_eth(1), // input weth <- insufficient balance
				10u128,    // expected usdc
				vec![weth, usdc],
				None,
				Some(0),
			),
			Error::<Test>::DeadlineMissed
		);

		// swap should fail if user does not have sufficient balance of input tokens
		assert_noop!(
			Dex::swap_with_exact_supply(
				Origin::signed(bob),
				to_eth(1), // input weth <- insufficient balance
				10u128,    // expected usdc
				vec![weth, usdc],
				None,
				None,
			),
			pallet_assets::Error::<Test>::NoAccount
		);

		// mint weth for 2nd user and allow them to perform swap against usdc
		assert_ok!(AssetsExt::mint_into(weth, &bob, to_eth(3)));
		assert_eq!(AssetsExt::balance(usdc, &bob), 0); // no balance initially for bob

		// swap should fail if user expects more output tokens than they can get
		assert_noop!(
			Dex::swap_with_exact_supply(
				Origin::signed(bob),
				to_eth(1), // input weth
				to_eth(1), // min expected usdc <- too much
				vec![weth, usdc],
				None,
				None,
			),
			Error::<Test>::InsufficientTargetAmount
		);

		// swap succeeds if user has sufficient balance of input tokens
		// and min expected output tokens are provided
		assert_ok!(Dex::swap_with_exact_supply(
			Origin::signed(bob),
			to_eth(1), // input weth
			0u128,     // min expected usdc
			vec![weth, usdc],
			None,
			Some(2),
		));

		let out_usdc_amount_1 = 998_497_746_619_929_894_u128;

		// verify swap event and user balances
		System::assert_last_event(MockEvent::Dex(crate::Event::Swap(
			bob,
			vec![weth, usdc],
			to_eth(1),
			out_usdc_amount_1,
			bob,
		)));
		assert_eq!(AssetsExt::balance(weth, &bob), to_eth(2));
		assert_eq!(AssetsExt::balance(usdc, &bob), out_usdc_amount_1);

		// verify dex trading pair liquidity changes (usdc removed, weth added)
		assert_eq!(
			Dex::get_liquidity(weth, usdc),
			(
				// init weth liquidity + user deposited weth
				to_eth(1) + to_eth(1),
				// init usdc liquidity - user withdrawn usdc
				to_eth(2) - out_usdc_amount_1,
			)
		);

		// user b swaps again with same params
		assert_ok!(Dex::swap_with_exact_supply(
			Origin::signed(bob),
			to_eth(1), // input weth
			10u128,    // min expected usdc
			vec![weth, usdc],
			None,
			None,
		));

		let out_usdc_amount_2 = 333_165_747_954_597_896_u128;

		// verify swap event and user balances
		System::assert_last_event(MockEvent::Dex(crate::Event::Swap(
			bob,
			vec![weth, usdc],
			to_eth(1),
			out_usdc_amount_2,
			bob,
		)));
		assert_eq!(AssetsExt::balance(weth, &bob), to_eth(1));
		assert_eq!(AssetsExt::balance(usdc, &bob), out_usdc_amount_1 + out_usdc_amount_2);
		assert_eq!(AssetsExt::balance(usdc, &bob), 1_331_663_494_574_527_790u128);

		// verify that 2nd swap returns less output tokens than first
		// - due to shift in constant product -> resulting in higher slippage
		assert_eq!(out_usdc_amount_1 > out_usdc_amount_2, true);

		// user bob swaps again with recipient charlie
		assert_ok!(Dex::swap_with_exact_supply(
			Origin::signed(bob),
			to_eth(1), // input weth
			10u128,    // min expected usdc
			vec![weth, usdc],
			Some(charlie),
			None,
		));

		let out_usdc_amount_3 = 166_707_904_905_978_432_u128;

		// verify swap event and user balances
		System::assert_last_event(MockEvent::Dex(crate::Event::Swap(
			bob,
			vec![weth, usdc],
			to_eth(1),
			out_usdc_amount_3,
			charlie,
		)));
		assert_eq!(AssetsExt::balance(weth, &bob), 0);
		assert_eq!(AssetsExt::balance(usdc, &bob), out_usdc_amount_1 + out_usdc_amount_2);
		assert_eq!(AssetsExt::balance(usdc, &charlie), out_usdc_amount_3);
	});
}

#[test]
fn perform_multiple_pair_swap_with_exact_supply() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);
		// restrict the trading path length to 2

		let alice: AccountId = create_account(1);

		let a = AssetsExt::create(&alice, None).unwrap();
		let b = AssetsExt::create(&alice, None).unwrap();
		let c = AssetsExt::create(&alice, None).unwrap();

		// mint tokens to user
		assert_ok!(AssetsExt::mint_into(a, &alice, to_eth(100)));
		assert_ok!(AssetsExt::mint_into(b, &alice, to_eth(100)));
		assert_ok!(AssetsExt::mint_into(c, &alice, to_eth(100)));

		// provide liquidity (a-b)
		assert_ok!(Dex::add_liquidity(
			Origin::signed(alice),
			a,
			b,
			100_000_000u128,
			100_000_000u128,
			100_000_000u128,
			100_000_000u128,
			None,
			None,
		));

		// provide liquidity (b-c)
		assert_ok!(Dex::add_liquidity(
			Origin::signed(alice),
			b,
			c,
			100_000_000u128,
			100_000_000u128,
			100_000_000u128,
			100_000_000u128,
			None,
			None,
		));

		assert_ok!(
			Dex::get_amounts_out(50_000u128, &[a, b, c]),
			vec![50_000u128, 49_825u128, 49_650u128]
		);

		// swap with exact supply ( path a->b->c )
		assert_ok!(Dex::swap_with_exact_supply(
			Origin::signed(alice),
			50_000u128, // input a
			1u128,      // expect c
			vec![a, b, c],
			None,
			None,
		));

		let trading_pair = TradingPair::new(a, b);
		let pool_address: AccountId = trading_pair.pool_address::<AccountId>();
		let (reserve_0, reserve_1) = LiquidityPool::<Test>::get(trading_pair);
		let balance_0 = AssetsExt::balance(trading_pair.0, &pool_address);
		let balance_1 = AssetsExt::balance(trading_pair.1, &pool_address);

		assert_eq!(reserve_0, balance_0);
		assert_eq!(reserve_1, balance_1);
		assert_eq!(reserve_0, 100_050_000u128); // 100_000_000u128 + 50_000u128
		assert_eq!(reserve_1, 99_950_175u128); // 100_000_000u128 - 49_825u128

		let trading_pair_2: TradingPair = TradingPair::new(b, c);
		let pool_address: AccountId = trading_pair_2.pool_address::<AccountId>();

		let (reserve_2, reserve_3) = LiquidityPool::<Test>::get(trading_pair_2);
		let balance_2 = AssetsExt::balance(trading_pair_2.0, &pool_address);
		let balance_3 = AssetsExt::balance(trading_pair_2.1, &pool_address);
		assert_eq!(reserve_2, balance_2);
		assert_eq!(reserve_3, balance_3);
		assert_eq!(reserve_2, 100_049_825u128); // 100_000_000u128 + 49_825u128
		assert_eq!(reserve_3, 99_950_350u128); // 100_000_000u128 - 49_650u128

		// Alice's a and c balance
		let alice_a = AssetsExt::balance(a, &alice);
		let alice_b = AssetsExt::balance(b, &alice);
		let alice_c = AssetsExt::balance(c, &alice);
		assert_eq!(alice_a, to_eth(100) - 100_000_000u128 - 50_000u128); // Initial minted - liquidity added - swap a for c
		assert_eq!(alice_b, to_eth(100) - 200_000_000u128); // Initial minted - liquidity added ( in pool [a-b] & [b-c]
		assert_eq!(alice_c, to_eth(100) - 100_000_000u128 + 49_650u128); // Initial minted - liquidity added
		                                                         // + swap a for c
	});
}

#[test]
fn swap_with_exact_target() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		let alice: AccountId = create_account(1);
		let bob: AccountId = create_account(2);
		let charlie: AccountId = create_account(3);

		// create tokens (by different users)
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&alice, None).unwrap();

		// mint tokens to user
		assert_ok!(AssetsExt::mint_into(usdc, &alice, to_eth(100)));
		assert_ok!(AssetsExt::mint_into(weth, &alice, to_eth(100)));

		// provide liquidity - note: differing amount of input tokens - ratio 2:1
		assert_ok!(Dex::add_liquidity(
			Origin::signed(alice),
			weth,
			usdc,
			to_eth(8),
			to_eth(4),
			to_eth(8),
			to_eth(4),
			None,
			None,
		));

		// swap should fail if user does not have sufficient balance of input tokens
		assert_noop!(
			Dex::swap_with_exact_target(
				Origin::signed(bob),
				10u128,                // expected usdc
				1_000_000_000_000u128, // max input weth <- insufficient balance
				vec![weth, usdc],
				None,
				None,
			),
			pallet_assets::Error::<Test>::NoAccount
		);

		// mint weth for 2nd user and allow them to perform swap against usdc
		assert_ok!(AssetsExt::mint_into(weth, &bob, to_eth(20)));

		// swap should fail if eqiuvalent tokens asked for are not available
		assert_noop!(
			Dex::swap_with_exact_target(
				Origin::signed(bob),
				to_eth(4), // expected <- too much
				to_eth(4), // max input weth willing to give
				vec![weth, usdc],
				None,
				None,
			),
			ArithmeticError::DivisionByZero
		);

		// fails if too much output tokens are expected
		assert_noop!(
			Dex::swap_with_exact_target(
				Origin::signed(bob),
				to_eth(1) / 2,
				to_eth(1), // max input weth willing to give
				vec![weth, usdc],
				None,
				None,
			),
			Error::<Test>::ExcessiveSupplyAmount
		);

		// swap should fail if the deadline is in the past
		assert_noop!(
			Dex::swap_with_exact_target(
				Origin::signed(bob),
				to_eth(1), // want usdc
				to_eth(5), // max input weth willing to give
				vec![weth, usdc],
				None,
				Some(0),
			),
			Error::<Test>::DeadlineMissed
		);

		// swap succeeds if user has sufficient balance of input tokens
		// and expected output tokens are provided
		assert_ok!(Dex::swap_with_exact_target(
			Origin::signed(bob),
			to_eth(1), // want usdc
			to_eth(5), // max input weth willing to give
			vec![weth, usdc],
			None,
			Some(2), // deadline in future
		));

		let in_weth_amount_1 = 2_674_690_738_883_316_617_u128;

		// verify swap event and user balances
		System::assert_last_event(MockEvent::Dex(crate::Event::Swap(
			bob,
			vec![weth, usdc],
			in_weth_amount_1, // supply amount
			to_eth(1),        // target amount
			bob,
		)));
		assert_eq!(AssetsExt::balance(weth, &bob), to_eth(20) - in_weth_amount_1);
		assert_eq!(AssetsExt::balance(weth, &bob), 17_325_309_261_116_683_383_u128);
		assert_eq!(AssetsExt::balance(usdc, &bob), to_eth(1));

		// verify dex trading pair liquidity changes (weth added, usdc removed)
		assert_eq!(
			Dex::get_liquidity(usdc, weth),
			(to_eth(4) - to_eth(1), to_eth(8) + in_weth_amount_1)
		);

		// user b swaps again with same params
		assert_ok!(Dex::swap_with_exact_target(
			Origin::signed(bob),
			to_eth(1), // want usdc
			to_eth(6), // max input weth willing to give
			vec![weth, usdc],
			None,
			None,
		));

		let in_weth_amount_2 = 5_353_405_586_200_259_086_u128;

		// verify swap event and user balances
		System::assert_last_event(MockEvent::Dex(crate::Event::Swap(
			bob,
			vec![weth, usdc],
			in_weth_amount_2, // supply amount
			to_eth(1),        // target amount
			bob,
		)));
		assert_eq!(
			AssetsExt::balance(weth, &bob),
			to_eth(20) - in_weth_amount_1 - in_weth_amount_2
		);
		assert_eq!(AssetsExt::balance(weth, &bob), 11_971_903_674_916_424_297_u128);
		assert_eq!(AssetsExt::balance(usdc, &bob), to_eth(2));

		// verify that 2nd swap requires more input tokens than first for the same output
		// - due to shift in constant product -> resulting in higher slippage
		assert_eq!(in_weth_amount_1 < in_weth_amount_2, true);

		// mint another 20 weth for bob and allow them to perform swap against usdc
		assert_ok!(AssetsExt::mint_into(weth, &bob, to_eth(20)));

		// user bob swaps again with recipient charlie
		assert_ok!(Dex::swap_with_exact_target(
			Origin::signed(bob),
			to_eth(1),  // want usdc
			to_eth(20), // max input weth willing to give
			vec![weth, usdc],
			Some(charlie),
			None,
		));

		let in_weth_amount_3 = 16_076_325_300_986_535_309_u128;

		// verify swap event and user balances
		System::assert_last_event(MockEvent::Dex(crate::Event::Swap(
			bob,
			vec![weth, usdc],
			in_weth_amount_3, // supply amount
			to_eth(1),        // target amount
			charlie,
		)));
		assert_eq!(
			AssetsExt::balance(weth, &bob),
			to_eth(40) - in_weth_amount_1 - in_weth_amount_2 - in_weth_amount_3
		);
		assert_eq!(AssetsExt::balance(usdc, &charlie), to_eth(1));
	});
}

/// multiple_swaps_with_multiple_lp is a complicated test which verifies
/// - that multiple users can add lp
/// - that multiple users can swap against that lp
/// - that lp can be removed by all lp owners
#[test]
fn multiple_swaps_with_multiple_lp() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		let alice: AccountId = create_account(1);
		let bob: AccountId = create_account(2);
		let charlie: AccountId = create_account(3);
		let danny: AccountId = create_account(4);
		let elliot: AccountId = create_account(5);

		// create tokens
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&alice, None).unwrap();

		// mint 100 tokens to alice
		assert_ok!(AssetsExt::mint_into(usdc, &alice, to_eth(100)));
		assert_ok!(AssetsExt::mint_into(weth, &alice, to_eth(100)));

		// mint 100 tokens to bob
		assert_ok!(AssetsExt::mint_into(usdc, &bob, to_eth(100)));
		assert_ok!(AssetsExt::mint_into(weth, &bob, to_eth(100)));

		// mint 10 tokens to charlie
		assert_ok!(AssetsExt::mint_into(usdc, &charlie, to_eth(50)));
		assert_ok!(AssetsExt::mint_into(weth, &charlie, to_eth(50)));

		// mint 10 tokens to danny
		assert_ok!(AssetsExt::mint_into(usdc, &danny, to_eth(10)));
		assert_ok!(AssetsExt::mint_into(weth, &danny, to_eth(10)));

		// mint 10 tokens to elliot
		assert_ok!(AssetsExt::mint_into(usdc, &elliot, to_eth(10)));
		assert_ok!(AssetsExt::mint_into(weth, &elliot, to_eth(10)));

		// alice provides liquidity for USDC/WETH pair - in ratio 1:3
		assert_ok!(Dex::add_liquidity(
			Origin::signed(alice),
			usdc,
			weth,
			to_eth(10),
			to_eth(30),
			to_eth(10),
			to_eth(30),
			None,
			None,
		));

		// bob provides liquidity for USDC/WETH pair - in ratio 1:3
		assert_ok!(Dex::add_liquidity(
			Origin::signed(bob),
			usdc,
			weth,
			to_eth(10),
			to_eth(30),
			to_eth(10),
			to_eth(30),
			None,
			None,
		));

		let lp_usdc_weth = Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap();

		// lp providers alice have lp tokens
		assert_eq!(AssetsExt::balance(lp_usdc_weth, &alice), 17_320_508_075_688_771_935_u128);
		assert_eq!(AssetsExt::balance(lp_usdc_weth, &bob), 17_320_508_075_688_772_935_u128);

		// charlie swaps 5 USDC for WETH
		assert_ok!(Dex::swap_with_exact_supply(
			Origin::signed(charlie),
			to_eth(5), // max input weth willing to give
			0u128,
			vec![usdc, weth],
			None,
			None,
		));
		assert_eq!(AssetsExt::balance(usdc, &charlie), to_eth(50) - to_eth(5));
		assert_eq!(AssetsExt::balance(weth, &charlie), 61_971_182_709_625_775_465_u128);

		// elliot swaps x USDC for 5 WETH
		assert_ok!(Dex::swap_with_exact_target(
			Origin::signed(elliot),
			to_eth(5), // exact want amount of weth
			to_eth(5),
			vec![usdc, weth],
			None,
			None,
		));
		assert_eq!(AssetsExt::balance(usdc, &elliot), 7_086_228_804_778_169_590_u128);
		assert_eq!(AssetsExt::balance(weth, &elliot), to_eth(10) + to_eth(5));

		let (reserve_0, reserve_1) = Dex::get_liquidity(usdc, weth);
		assert_eq!(reserve_0, 27_913_771_195_221_830_410_u128);
		assert_eq!(reserve_1, 43_028_817_290_374_224_535_u128);

		// charlie provides liquidity for USDC/WETH pair - in different ratio
		assert_ok!(Dex::add_liquidity(
			Origin::signed(charlie),
			usdc,
			weth,
			to_eth(2),
			to_eth(4),
			to_eth(1),
			to_eth(2),
			None,
			None,
		));
		assert_eq!(AssetsExt::balance(lp_usdc_weth, &charlie), 2_482_001_869_909_090_520_u128);

		// danny swaps x USDC for 2 WETH
		assert_ok!(Dex::swap_with_exact_target(
			Origin::signed(danny),
			to_eth(2), // exact want amount of weth
			to_eth(2),
			vec![usdc, weth],
			None,
			None,
		));
		assert_eq!(AssetsExt::balance(usdc, &danny), 8_639_648_189_269_446_680_u128);
		assert_eq!(AssetsExt::balance(weth, &danny), to_eth(10) + to_eth(2));

		// elliot fails to remove any liquidity (he has none)
		assert_noop!(
			Dex::remove_liquidity(
				Origin::signed(elliot),
				usdc,
				weth,
				AssetsExt::balance(lp_usdc_weth, &elliot),
				to_eth(10),
				to_eth(10),
				None,
				None,
			),
			Error::<Test>::InsufficientLiquidityBurnt
		);

		assert_eq!(AssetsExt::balance(lp_usdc_weth, &charlie), 2_482_001_869_909_090_520_u128);
		// charlie removes all his liquidity
		assert_ok!(Dex::remove_liquidity(
			Origin::signed(charlie),
			usdc,
			weth,
			AssetsExt::balance(lp_usdc_weth, &charlie),
			to_eth(1),
			to_eth(1),
			None,
			None,
		));
		assert_eq!(AssetsExt::balance(lp_usdc_weth, &charlie), 0u128);
		assert_eq!(AssetsExt::balance(usdc, &charlie), 45_090_951_542_141_088_801_u128);
		assert_eq!(AssetsExt::balance(weth, &charlie), 61_837_465_032_443_069_536_u128);

		// alice removes all her liquidity
		assert_ok!(Dex::remove_liquidity(
			Origin::signed(alice),
			usdc,
			weth,
			AssetsExt::balance(lp_usdc_weth, &alice),
			to_eth(10),
			to_eth(10),
			None,
			None,
		));
		assert_eq!(AssetsExt::balance(lp_usdc_weth, &alice), 0u128);
		assert_eq!(AssetsExt::balance(usdc, &alice), 104_591_585_731_905_646_622_u128);
		assert_eq!(AssetsExt::balance(weth, &alice), 90_581_267_483_778_464_043_u128);

		// bob removes all his liquidity
		assert_ok!(Dex::remove_liquidity(
			Origin::signed(bob),
			usdc,
			weth,
			AssetsExt::balance(lp_usdc_weth, &bob),
			to_eth(10),
			to_eth(10),
			None,
			None,
		));
		assert_eq!(AssetsExt::balance(lp_usdc_weth, &bob), 0u128);
		assert_eq!(AssetsExt::balance(usdc, &bob), 104_591_585_731_905_647_464_u128);
		assert_eq!(AssetsExt::balance(weth, &bob), 90_581_267_483_778_465_232_u128);
	});
}

#[test]
fn query_with_trading_pair() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);
		let alice: AccountId = create_account(1);
		let bob: AccountId = create_account(2);

		// create 2 tokens
		let usdc = AssetsExt::create(&alice, None).unwrap();
		let weth = AssetsExt::create(&bob, None).unwrap();

		// mint tokens to user
		assert_ok!(AssetsExt::mint_into(usdc, &alice, to_eth(5)));
		assert_ok!(AssetsExt::mint_into(weth, &alice, to_eth(1)));
		assert_ok!(Dex::add_liquidity(
			Origin::signed(alice),
			usdc,
			weth,
			to_eth(5),
			to_eth(1),
			to_eth(5),
			to_eth(1),
			None,
			None,
		));

		// The trading pair should be enabled regardless of the order of the query inputs
		assert_eq!(Dex::get_trading_pair_status(usdc, weth), TradingPairStatus::Enabled);
		assert_eq!(Dex::get_trading_pair_status(weth, usdc), TradingPairStatus::Enabled);

		// The trading pair should have the unique lp token id regardless of the order of the query
		// inputs
		let asset_id = 3 << 10 | 100;
		assert_eq!(Dex::get_lp_token_id(usdc, weth).unwrap(), asset_id);
		assert_eq!(Dex::get_lp_token_id(weth, usdc).unwrap(), asset_id);

		// The trading pair should return the corresponding balances according to the order of the
		// query inputs
		assert_eq!(Dex::get_liquidity(usdc, weth), (to_eth(5), to_eth(1)));
		assert_eq!(Dex::get_liquidity(weth, usdc), (to_eth(1), to_eth(5)));
	});
}

// macro swap with exact supply
// - `$name`: name of the test
// - `$liquidity`: LP user adds liquidity with $liquidity[0] and $liquidity[1]
// - `$amount_in`: user mints $amount_in tokens
// - `$amount_out_min`: user swaps $amount_in tokens for atleast $amount_out_min tokens
// - `$amount_out`: user checks that $amount_out tokens were received - or error if swap fails
macro_rules! swap_with_exact_supply_multi {
	(
		$name:ident,
		$liquidity:expr,
		$amount_in:expr,
		$amount_out_min:expr,
		$amount_out:expr,
	) => {
		#[test]
		fn $name() {
			TestExt::default().build().execute_with(|| {
				System::set_block_number(1);

				let (lp_amount_token_1, lp_amount_token_2) = $liquidity;

				let alice: AccountId = create_account(1);
				let bob: AccountId = create_account(2);

				// create tokens
				let token_0 = AssetsExt::create(&alice, None).unwrap();
				let token_1 = AssetsExt::create(&alice, None).unwrap();

				// mint input tokens to alice for LP
				assert_ok!(AssetsExt::mint_into(token_0, &alice, lp_amount_token_1));
				assert_ok!(AssetsExt::mint_into(token_1, &alice, lp_amount_token_2));

				// add liquidity
				assert_ok!(Dex::add_liquidity(
					Origin::signed(alice),
					token_0,
					token_1,
					lp_amount_token_1,
					lp_amount_token_2,
					lp_amount_token_1,
					lp_amount_token_2,
					None,
					None,
				));

				// mint input tokens to bob for swap
				assert_ok!(AssetsExt::mint_into(token_0, &bob, $amount_in));

				let result: Result<u128, DispatchError> = $amount_out;

				match result {
					Ok(amount_out) => {
						assert_ok!(Dex::swap_with_exact_supply(
							Origin::signed(bob),
							$amount_in,
							$amount_out_min,
							vec![token_0, token_1],
							None,
							None,
						));

						assert_eq!(AssetsExt::balance(token_0, &bob), 0u128);
						assert_eq!(AssetsExt::balance(token_1, &bob), amount_out);
					},
					Err(err) => {
						assert_noop!(
							Dex::swap_with_exact_supply(
								Origin::signed(bob),
								$amount_in,
								$amount_out_min,
								vec![token_0, token_1],
								None,
								None,
							),
							err
						);
					},
				}
			});
		}
	};
}

swap_with_exact_supply_multi!(
	swap_with_exact_supply_1,
	(to_eth(100), to_eth(100)),
	to_eth(10),
	to_eth(9),
	Ok(9_066_108_938_801_491_315_u128),
);

swap_with_exact_supply_multi!(
	swap_with_exact_supply_2,
	(to_eth(100), to_eth(100)),
	to_eth(10),
	to_eth(10),
	Err(Error::<Test>::InsufficientTargetAmount.into()),
);

swap_with_exact_supply_multi!(
	swap_with_exact_supply_3,
	(to_eth(1), to_eth(1)),
	to_eth(10),
	to_eth(2),
	Err(Error::<Test>::InsufficientTargetAmount.into()),
);

swap_with_exact_supply_multi!(
	swap_with_exact_supply_4,
	(to_eth(1), to_eth(1)),
	to_eth(10),
	to_eth(0u128),
	Ok(908_842_297_174_111_212_u128),
);

// macro swap with exact supply
// - `$name`: name of the test
// - `$liquidity`: LP user adds liquidity with $liquidity[0] and $liquidity[1]
// - `$amount_out`: exact amount of output tokens wanted
// - `$amount_in_max`: maximum input tokens user willing to pay for exact amount of tokens
// - `$amount_in`: actual amount of input tokens utilised in swap - or error if swap fails
macro_rules! swap_with_exact_target_multi {
	(
		$name:ident,
		$liquidity:expr,
		$amount_out:expr,
		$amount_in_max:expr,
		$amount_in:expr,
	) => {
		#[test]
		fn $name() {
			TestExt::default().build().execute_with(|| {
				System::set_block_number(1);

				let (lp_amount_token_1, lp_amount_token_2) = $liquidity;

				let alice: AccountId = create_account(1);
				let bob: AccountId = create_account(2);

				// create tokens
				let token_0 = AssetsExt::create(&alice, None).unwrap();
				let token_1 = AssetsExt::create(&alice, None).unwrap();

				// mint input tokens to alice for LP
				assert_ok!(AssetsExt::mint_into(token_0, &alice, lp_amount_token_1));
				assert_ok!(AssetsExt::mint_into(token_1, &alice, lp_amount_token_2));

				// add liquidity
				assert_ok!(Dex::add_liquidity(
					Origin::signed(alice),
					token_0,
					token_1,
					lp_amount_token_1,
					lp_amount_token_2,
					lp_amount_token_1,
					lp_amount_token_2,
					None,
					None,
				));

				// mint input tokens to bob for swap
				assert_ok!(AssetsExt::mint_into(token_0, &bob, $amount_in_max));

				let result: Result<u128, DispatchError> = $amount_in;

				match result {
					Ok(amount_in) => {
						assert_ok!(Dex::swap_with_exact_target(
							Origin::signed(bob),
							$amount_out,
							$amount_in_max,
							vec![token_0, token_1],
							None,
							None,
						));

						assert_eq!(AssetsExt::balance(token_0, &bob), amount_in);
						assert_eq!(AssetsExt::balance(token_1, &bob), $amount_out);
					},
					Err(err) => {
						assert_noop!(
							Dex::swap_with_exact_target(
								Origin::signed(bob),
								$amount_out,
								$amount_in_max,
								vec![token_0, token_1],
								None,
								None,
							),
							err
						);
					},
				}
			});
		}
	};
}

swap_with_exact_target_multi!(
	swap_with_exact_target_1,
	(to_eth(100), to_eth(100)),
	to_eth(9),
	to_eth(10),
	Ok(80_130_501_394_292_768_u128),
);

swap_with_exact_target_multi!(
	swap_with_exact_target_2,
	(to_eth(100), to_eth(100)),
	to_eth(10),
	to_eth(10),
	Err(Error::<Test>::ExcessiveSupplyAmount.into()),
);

swap_with_exact_target_multi!(
	swap_with_exact_target_3,
	(to_eth(1), to_eth(1)),
	to_eth(2),
	to_eth(10),
	Err(DispatchError::Arithmetic(ArithmeticError::Underflow)),
);

swap_with_exact_target_multi!(
	swap_with_exact_target_4,
	(to_eth(10), to_eth(10)),
	to_eth(10),
	to_eth(0u128),
	Err(DispatchError::Arithmetic(ArithmeticError::DivisionByZero)),
);

swap_with_exact_target_multi!(
	swap_with_exact_target_5,
	(to_eth(100), to_eth(100)),
	to_eth(10),
	to_eth(20),
	Ok(8_855_455_254_652_847_431_u128),
);
