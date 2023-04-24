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
use mock::{Dex, Event as MockEvent, MockAccountId, Origin, System, Test, TestExt, ALICE, BOB};
use sp_runtime::{traits::BadOrigin, ArithmeticError, DispatchError};

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

		// create 2 tokens
		let usdc = AssetsExt::create(&ALICE, None).unwrap();
		let weth = AssetsExt::create(&ALICE, None).unwrap();

		// normal user can not disable trading_pair
		assert_noop!(Dex::disable_trading_pair(Origin::signed(ALICE), usdc, weth), BadOrigin);

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

		// create 2 tokens
		let usdc = AssetsExt::create(&ALICE, None).unwrap();
		let weth = AssetsExt::create(&ALICE, None).unwrap();

		// normal user can not enable trading_pair
		assert_noop!(Dex::reenable_trading_pair(Origin::signed(ALICE), usdc, weth), BadOrigin);

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
fn add_liquidity() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		// create 2 tokens
		let usdc = AssetsExt::create(&ALICE, None).unwrap();
		let weth = AssetsExt::create(&BOB, None).unwrap();

		// mint tokens to user
		assert_ok!(AssetsExt::mint_into(usdc, &ALICE, to_eth(1)));
		assert_ok!(AssetsExt::mint_into(weth, &ALICE, to_eth(1)));
		assert_ok!(Dex::add_liquidity(
			Origin::signed(ALICE),
			usdc,
			weth,
			to_eth(1),
			to_eth(1),
			to_eth(1),
			to_eth(1),
			0u128, //not used
		));

		// adding LP enables trading pair
		assert_eq!(
			Dex::trading_pair_statuses(TradingPair::new(usdc, weth)),
			TradingPairStatus::Enabled
		);

		// System::assert_has_event(MockEvent::Assets(pallet_assets::Event::Transferred {
		// 	asset_id: usdc,
		// 	from: ALICE,
		// 	to: Dex::account_id(),
		// 	amount: 1_000_000_000_000u128,
		// }));

		// System::assert_has_event(MockEvent::Assets(pallet_assets::Event::Transferred {
		// 	asset_id: weth,
		// 	from: ALICE,
		// 	to: Dex::account_id(),
		// 	amount: 1_000_000_000_000u128,
		// }));

		System::assert_last_event(MockEvent::Dex(crate::Event::AddLiquidity(
			ALICE,
			usdc,
			to_eth(1),
			weth,
			to_eth(1),
			999_999_999_999_999_000u128, // lp token shares
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
			AssetsExt::balance(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), &ALICE),
			999_999_999_999_999_000u128,
		);

		// mint tokens to new user
		assert_ok!(AssetsExt::mint_into(usdc, &BOB, to_eth(2)));
		assert_ok!(AssetsExt::mint_into(weth, &BOB, to_eth(2)));

		// add liquidity to new user fails - as they expect too much lp tokens
		assert_noop!(
			Dex::add_liquidity(
				Origin::signed(BOB),
				usdc,
				weth,
				to_eth(2),
				to_eth(2),
				to_eth(2),
				to_eth(2),
				to_eth(2) + 1, // min lp tokens expected too high
			),
			Error::<Test>::UnacceptableShareIncrement
		);

		// add liquidity to new user succeeds - as min expected lp tokens saisfied
		assert_ok!(Dex::add_liquidity(
			Origin::signed(BOB),
			usdc,
			weth,
			to_eth(2),
			to_eth(2),
			to_eth(2),
			to_eth(2),
			to_eth(2), // mint lp tokens satisfied
		));

		// verify Bob now has LP tokens
		assert_eq!(
			AssetsExt::balance(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), &BOB),
			to_eth(2),
		);

		// bob should have more LP tokens than Alice as Bob provisioned more liquidity
		assert_eq!(
			AssetsExt::balance(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), &ALICE) <
				AssetsExt::balance(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), &BOB),
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
				Origin::signed(BOB),
				usdc,
				weth,
				2_000_000_000_000u128,
				2_000_000_000_000u128,
				2_000_000_000_000u128,
				2_000_000_000_000u128,
				0u128, //not used
			),
			Error::<Test>::MustBeEnabled
		);
	});
}

/// https://github.com/futureversecom/seed/issues/15
#[test]
fn add_liquidity_issue_15() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		// create 2 tokens
		let usdc = AssetsExt::create(&ALICE.clone(), None).unwrap();
		let weth = AssetsExt::create(&BOB.clone(), None).unwrap();

		// mint tokens to user
		assert_ok!(AssetsExt::mint_into(usdc, &ALICE, to_eth(10)));
		assert_ok!(AssetsExt::mint_into(weth, &ALICE, to_eth(10)));
		assert_ok!(Dex::add_liquidity(
			Origin::signed(ALICE),
			usdc,
			weth,
			to_eth(1),
			to_eth(1),
			to_eth(1),
			to_eth(1),
			0u128, //not used
		));

		assert_ok!(Dex::add_liquidity(
			Origin::signed(ALICE),
			usdc,
			weth,
			to_eth(2),
			to_eth(1),
			to_eth(1),
			to_eth(1),
			0u128, //not used
		));
		assert_eq!(
			AssetsExt::balance(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), &ALICE),
			1_999_999_999_999_999_000_u128,
		);
		assert_eq!(AssetsExt::balance(usdc, &ALICE), 8_000_000_000_000_000_000_u128);
		assert_eq!(AssetsExt::balance(weth, &ALICE), 8_000_000_000_000_000_000_u128);
	});
}

#[test]
fn remove_liquidity_simple() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		// create 2 tokens (by different users)
		let usdc = AssetsExt::create(&ALICE, None).unwrap();
		let weth = AssetsExt::create(&BOB, None).unwrap();

		// add liquidity as user
		assert_ok!(AssetsExt::mint_into(usdc, &ALICE, to_eth(2)));
		assert_ok!(AssetsExt::mint_into(weth, &ALICE, to_eth(2)));
		assert_ok!(Dex::add_liquidity(
			Origin::signed(ALICE),
			usdc,
			weth,
			to_eth(2),
			to_eth(2),
			to_eth(2),
			to_eth(2),
			1_999_999_999_999_999_000u128, // min expected LP token shares
		));
		let lp_token_id = Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap();
		assert_eq!(AssetsExt::balance(lp_token_id, &ALICE), 1_999_999_999_999_999_000u128);
		assert_eq!(AssetsExt::balance(usdc, &ALICE), 0);
		assert_eq!(AssetsExt::balance(weth, &ALICE), 0);

		// providing all-1 LP token shares should succeed
		assert_ok!(Dex::remove_liquidity(
			Origin::signed(ALICE),
			usdc,
			weth,
			1_999_999_999_999_999_000u128, // all lp -1 to retrieve input tokens
			0u128,                         // ignoring expected input token liquidity
			0u128,                         // ignoring expected input token liquidity
		));

		System::assert_last_event(MockEvent::Dex(crate::Event::RemoveLiquidity(
			ALICE,
			usdc,
			1_999_999_999_999_999_000u128,
			weth,
			1_999_999_999_999_999_000u128,
			1_999_999_999_999_999_000u128,
		)));

		assert_eq!(
			AssetsExt::balance(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), &ALICE),
			0,
		);
		assert_eq!(AssetsExt::balance(usdc, &ALICE), 1_999_999_999_999_999_000u128);
		assert_eq!(AssetsExt::balance(weth, &ALICE), 1_999_999_999_999_999_000u128);
	});
}

#[test]
fn remove_liquidity_full() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		// create 2 tokens (by different users)
		let usdc = AssetsExt::create(&ALICE, None).unwrap();
		let weth = AssetsExt::create(&BOB, None).unwrap();

		// fails if no LP tokens withdrawn
		assert_eq!(
			Dex::remove_liquidity(Origin::signed(ALICE), usdc, weth, 0u128, 2u128, 2u128).is_ok(),
			false
		);

		// remove liquidity fails if LP token doesnt exist
		assert_noop!(
			Dex::remove_liquidity(Origin::signed(ALICE), usdc, weth, 1u128, 2u128, 2u128,),
			Error::<Test>::InvalidAssetId
		);

		// maually create and enable LP token
		let lp_token_id = AssetsExt::create(&ALICE, None).unwrap();
		TradingPairLPToken::<Test>::insert(TradingPair::new(usdc, weth), Some(lp_token_id));
		TradingPairStatuses::<Test>::insert(
			TradingPair::new(usdc, weth),
			TradingPairStatus::Enabled,
		);

		// add liquidity as user
		assert_ok!(AssetsExt::mint_into(usdc, &ALICE, to_eth(2)));
		assert_ok!(AssetsExt::mint_into(weth, &ALICE, to_eth(2)));
		assert_ok!(Dex::add_liquidity(
			Origin::signed(ALICE),
			usdc,
			weth,
			to_eth(2),
			to_eth(2),
			to_eth(2),
			to_eth(2),
			1_999_999_999_999_999_000u128, // min expected LP token shares
		));
		let lp_token_id = Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(); // TODO remove
		assert_eq!(AssetsExt::balance(lp_token_id, &ALICE), 1_999_999_999_999_999_000u128);
		assert_eq!(AssetsExt::balance(usdc, &ALICE), 0);
		assert_eq!(AssetsExt::balance(weth, &ALICE), 0);

		// remove liquidity fails if user expects more balance of a token than they have
		assert_noop!(
			Dex::remove_liquidity(
				Origin::signed(ALICE),
				usdc,
				weth,
				1u128,
				to_eth(2) + 1, // more balance than user had LPed
				0u128,
			),
			Error::<Test>::InsufficientWithdrawnAmountA
		);

		assert_noop!(
			Dex::remove_liquidity(
				Origin::signed(ALICE),
				usdc,
				weth,
				1u128,
				0u128,
				to_eth(2) + 1, // more balance than user had LPed
			),
			Error::<Test>::InsufficientWithdrawnAmountB
		);

		assert_noop!(
			Dex::remove_liquidity(
				Origin::signed(ALICE),
				usdc,
				weth,
				100u128, // provided LP token shares too low to retrieve input tokens
				2_000_000_000_000u128,
				2_000_000_000_000u128,
			),
			Error::<Test>::InsufficientWithdrawnAmountA
		);

		// providing all-1 LP token shares should succeed
		assert_ok!(Dex::remove_liquidity(
			Origin::signed(ALICE),
			usdc,
			weth,
			1_999_999_999_999_999_000u128 - 1, // all lp -1 to retrieve input tokens
			0u128,                             // ignoring expected input token liquidity
			0u128,                             // ignoring expected input token liquidity
		));

		System::assert_last_event(MockEvent::Dex(crate::Event::RemoveLiquidity(
			ALICE,
			usdc,
			1_999_999_999_999_998_999_u128,
			weth,
			1_999_999_999_999_998_999_u128,
			1_999_999_999_999_998_999_u128,
		)));

		assert_eq!(
			AssetsExt::balance(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), &ALICE),
			1,
		);
		assert_eq!(AssetsExt::balance(usdc, &ALICE), 1_999_999_999_999_998_999_u128);
		assert_eq!(AssetsExt::balance(weth, &ALICE), 1_999_999_999_999_998_999_u128);

		// disable trading pair
		TradingPairStatuses::<Test>::insert(
			TradingPair::new(usdc, weth),
			TradingPairStatus::NotEnabled,
		);

		// can still successfully remove liquidity if trading pair is disabled
		// remove last lp token remaining
		assert_ok!(Dex::remove_liquidity(
			Origin::signed(ALICE),
			usdc,
			weth,
			1,
			0u128, // ignoring expected input token liquidity
			0u128, // ignoring expected input token liquidity
		));

		// removing all liquidity should imply user has recieved all input tokens
		assert_eq!(
			AssetsExt::balance(Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap(), &ALICE),
			0u128,
		);
		// do not get 100% tokens back as some lost due to minimum liquidity minting
		assert_eq!(AssetsExt::balance(usdc, &ALICE), 1_999_999_999_999_999_000_u128);
		assert_eq!(AssetsExt::balance(weth, &ALICE), 1_999_999_999_999_999_000_u128);
	});
}

#[test]
fn swap_with_exact_supply() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		let weth = AssetsExt::create(&ALICE, None).unwrap();
		let usdc = AssetsExt::create(&ALICE, None).unwrap();

		// mint tokens to user
		assert_ok!(AssetsExt::mint_into(usdc, &ALICE, to_eth(100)));
		assert_ok!(AssetsExt::mint_into(weth, &ALICE, to_eth(100)));

		// provide liquidity - note: differing amount of input tokens - ratio 1:2
		assert_ok!(Dex::add_liquidity(
			Origin::signed(ALICE),
			weth,
			usdc,
			to_eth(1),
			to_eth(2),
			to_eth(1),
			to_eth(2),
			0u128,
		));
		assert_eq!(AssetsExt::balance(weth, &ALICE), to_eth(100) - to_eth(1));
		assert_eq!(AssetsExt::balance(usdc, &ALICE), to_eth(100) - to_eth(2));

		// swap should fail if user does not have sufficient balance of input tokens
		assert_noop!(
			Dex::swap_with_exact_supply(
				Origin::signed(BOB),
				to_eth(1), // input weth <- insufficient balance
				10u128,    // expected usdc
				vec![weth, usdc],
			),
			pallet_assets::Error::<Test>::NoAccount
		);

		// mint weth for 2nd user and allow them to perform swap against usdc
		assert_ok!(AssetsExt::mint_into(weth, &BOB, to_eth(2)));
		assert_eq!(AssetsExt::balance(usdc, &BOB), 0); // no balance initially for bob

		// swap should fail if user expects more output tokens than they can get
		assert_noop!(
			Dex::swap_with_exact_supply(
				Origin::signed(BOB),
				to_eth(1), // input weth
				to_eth(1), // min expected usdc <- too much
				vec![weth, usdc],
			),
			Error::<Test>::InsufficientTargetAmount
		);

		// swap succeeds if user has sufficient balance of input tokens
		// and min expected output tokens are provided
		assert_ok!(Dex::swap_with_exact_supply(
			Origin::signed(BOB),
			to_eth(1), // input weth
			0u128,     // min expected usdc
			vec![weth, usdc],
		));

		let out_usdc_amount_1 = 998_497_746_619_929_894_u128;

		// verify swap event and user balances
		System::assert_last_event(MockEvent::Dex(crate::Event::Swap(
			BOB,
			vec![weth, usdc],
			to_eth(1),
			out_usdc_amount_1,
		)));
		assert_eq!(AssetsExt::balance(weth, &BOB), to_eth(1));
		assert_eq!(AssetsExt::balance(usdc, &BOB), out_usdc_amount_1);

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
			Origin::signed(BOB),
			to_eth(1), // input weth
			10u128,    // min expected usdc
			vec![weth, usdc],
		));

		let out_usdc_amount_2 = 333_165_747_954_597_896_u128;

		// verify swap event and user balances
		System::assert_last_event(MockEvent::Dex(crate::Event::Swap(
			BOB,
			vec![weth, usdc],
			to_eth(1),
			out_usdc_amount_2,
		)));
		assert_eq!(AssetsExt::balance(weth, &BOB), 0);
		assert_eq!(AssetsExt::balance(usdc, &BOB), out_usdc_amount_1 + out_usdc_amount_2);
		assert_eq!(AssetsExt::balance(usdc, &BOB), 1_331_663_494_574_527_790u128);

		// verify that 2nd swap returns less output tokens than first
		// - due to shift in constant product -> resulting in higher slippage
		assert_eq!(out_usdc_amount_1 > out_usdc_amount_2, true);
	});
}

#[test]
fn swap_with_exact_target() {
	TestExt::default().build().execute_with(|| {
		System::set_block_number(1);

		// create tokens (by different users)
		let usdc = AssetsExt::create(&ALICE, None).unwrap();
		let weth = AssetsExt::create(&ALICE, None).unwrap();

		// mint tokens to user
		assert_ok!(AssetsExt::mint_into(usdc, &ALICE, to_eth(100)));
		assert_ok!(AssetsExt::mint_into(weth, &ALICE, to_eth(100)));

		// provide liquidity - note: differing amount of input tokens - ratio 2:1
		assert_ok!(Dex::add_liquidity(
			Origin::signed(ALICE),
			weth,
			usdc,
			to_eth(8),
			to_eth(4),
			to_eth(8),
			to_eth(4),
			0u128,
		));

		// swap should fail if user does not have sufficient balance of input tokens
		assert_noop!(
			Dex::swap_with_exact_target(
				Origin::signed(BOB),
				10u128,                // expected usdc
				1_000_000_000_000u128, // max input weth <- insufficient balance
				vec![weth, usdc],
			),
			pallet_assets::Error::<Test>::NoAccount
		);

		// mint weth for 2nd user and allow them to perform swap against usdc
		assert_ok!(AssetsExt::mint_into(weth, &BOB, to_eth(20)));

		// swap should fail if eqiuvalent tokens asked for are not available
		assert_noop!(
			Dex::swap_with_exact_target(
				Origin::signed(BOB),
				to_eth(4), // expected <- too much
				to_eth(4), // max input weth willing to give
				vec![weth, usdc],
			),
			ArithmeticError::DivisionByZero
		);

		// fails if too much output tokens are expected
		assert_noop!(
			Dex::swap_with_exact_target(
				Origin::signed(BOB),
				to_eth(1) / 2,
				to_eth(1), // max input weth willing to give
				vec![weth, usdc],
			),
			Error::<Test>::ExcessiveSupplyAmount
		);

		// swap succeeds if user has sufficient balance of input tokens
		// and expected output tokens are provided
		assert_ok!(Dex::swap_with_exact_target(
			Origin::signed(BOB),
			to_eth(1), // want usdc
			to_eth(5), // max input weth willing to give
			vec![weth, usdc],
		));

		let in_weth_amount_1 = 2_674_690_738_883_316_617_u128;

		// verify swap event and user balances
		System::assert_last_event(MockEvent::Dex(crate::Event::Swap(
			BOB,
			vec![weth, usdc],
			in_weth_amount_1, // supply amount
			to_eth(1),        // target amount
		)));
		assert_eq!(AssetsExt::balance(weth, &BOB), to_eth(20) - in_weth_amount_1);
		assert_eq!(AssetsExt::balance(weth, &BOB), 17_325_309_261_116_683_383_u128);
		assert_eq!(AssetsExt::balance(usdc, &BOB), to_eth(1));

		// verify dex trading pair liquidity changes (weth added, usdc removed)
		assert_eq!(
			Dex::get_liquidity(usdc, weth),
			(to_eth(4) - to_eth(1), to_eth(8) + in_weth_amount_1)
		);

		// user b swaps again with same params
		assert_ok!(Dex::swap_with_exact_target(
			Origin::signed(BOB),
			to_eth(1), // want usdc
			to_eth(6), // max input weth willing to give
			vec![weth, usdc],
		));

		let in_weth_amount_2 = 5_353_405_586_200_259_086_u128;

		// verify swap event and user balances
		System::assert_last_event(MockEvent::Dex(crate::Event::Swap(
			BOB,
			vec![weth, usdc],
			in_weth_amount_2, // supply amount
			to_eth(1),        // target amount
		)));
		assert_eq!(
			AssetsExt::balance(weth, &BOB),
			to_eth(20) - in_weth_amount_1 - in_weth_amount_2
		);
		assert_eq!(AssetsExt::balance(weth, &BOB), 11_971_903_674_916_424_297_u128);
		assert_eq!(AssetsExt::balance(usdc, &BOB), to_eth(2));

		// verify that 2nd swap requires more input tokens than first for the same output
		// - due to shift in constant product -> resulting in higher slippage
		assert_eq!(in_weth_amount_1 < in_weth_amount_2, true);
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

		pub const CHARLIE: MockAccountId = 3;
		pub const DANNY: MockAccountId = 4;
		pub const ELLIOT: MockAccountId = 5;

		// create tokens
		let usdc = AssetsExt::create(&ALICE, None).unwrap();
		let weth = AssetsExt::create(&ALICE, None).unwrap();

		// mint 100 tokens to alice
		assert_ok!(AssetsExt::mint_into(usdc, &ALICE, to_eth(100)));
		assert_ok!(AssetsExt::mint_into(weth, &ALICE, to_eth(100)));

		// mint 100 tokens to bob
		assert_ok!(AssetsExt::mint_into(usdc, &BOB, to_eth(100)));
		assert_ok!(AssetsExt::mint_into(weth, &BOB, to_eth(100)));

		// mint 10 tokens to charlie
		assert_ok!(AssetsExt::mint_into(usdc, &CHARLIE, to_eth(50)));
		assert_ok!(AssetsExt::mint_into(weth, &CHARLIE, to_eth(50)));

		// mint 10 tokens to danny
		assert_ok!(AssetsExt::mint_into(usdc, &DANNY, to_eth(10)));
		assert_ok!(AssetsExt::mint_into(weth, &DANNY, to_eth(10)));

		// mint 10 tokens to elliot
		assert_ok!(AssetsExt::mint_into(usdc, &ELLIOT, to_eth(10)));
		assert_ok!(AssetsExt::mint_into(weth, &ELLIOT, to_eth(10)));

		// alice provides liquidity for USDC/WETH pair - in ratio 1:3
		assert_ok!(Dex::add_liquidity(
			Origin::signed(ALICE),
			usdc,
			weth,
			to_eth(10),
			to_eth(30),
			to_eth(10),
			to_eth(30),
			0u128
		));

		// bob provides liquidity for USDC/WETH pair - in ratio 1:3
		assert_ok!(Dex::add_liquidity(
			Origin::signed(BOB),
			usdc,
			weth,
			to_eth(10),
			to_eth(30),
			to_eth(10),
			to_eth(30),
			0u128
		));

		let lp_usdc_weth = Dex::lp_token_id(TradingPair::new(usdc, weth)).unwrap();

		// lp providers alice have lp tokens
		assert_eq!(AssetsExt::balance(lp_usdc_weth, &ALICE), 17_320_508_075_688_771_935_u128);
		assert_eq!(AssetsExt::balance(lp_usdc_weth, &BOB), 17_320_508_075_688_772_935_u128);

		// charlie swaps 5 USDC for WETH
		assert_ok!(Dex::swap_with_exact_supply(
			Origin::signed(CHARLIE),
			to_eth(5), // max input weth willing to give
			0u128,
			vec![usdc, weth],
		));
		assert_eq!(AssetsExt::balance(usdc, &CHARLIE), to_eth(50) - to_eth(5));
		assert_eq!(AssetsExt::balance(weth, &CHARLIE), 61_971_182_709_625_775_465_u128);

		// elliot swaps x USDC for 5 WETH
		assert_ok!(Dex::swap_with_exact_target(
			Origin::signed(ELLIOT),
			to_eth(5), // exact want amount of weth
			to_eth(5),
			vec![usdc, weth],
		));
		assert_eq!(AssetsExt::balance(usdc, &ELLIOT), 7_086_228_804_778_169_590_u128);
		assert_eq!(AssetsExt::balance(weth, &ELLIOT), to_eth(10) + to_eth(5));

		let (reserve_0, reserve_1) = Dex::get_liquidity(usdc, weth);
		assert_eq!(reserve_0, 27_913_771_195_221_830_410_u128);
		assert_eq!(reserve_1, 43_028_817_290_374_224_535_u128);

		// charlie provides liquidity for USDC/WETH pair - in different ratio
		assert_ok!(Dex::add_liquidity(
			Origin::signed(CHARLIE),
			usdc,
			weth,
			to_eth(2),
			to_eth(4),
			to_eth(1),
			to_eth(2),
			0u128
		));
		assert_eq!(AssetsExt::balance(lp_usdc_weth, &CHARLIE), 2_482_001_869_909_090_520_u128);

		// danny swaps x USDC for 2 WETH
		assert_ok!(Dex::swap_with_exact_target(
			Origin::signed(DANNY),
			to_eth(2), // exact want amount of weth
			to_eth(2),
			vec![usdc, weth],
		));
		assert_eq!(AssetsExt::balance(usdc, &DANNY), 8_639_648_189_269_446_680_u128);
		assert_eq!(AssetsExt::balance(weth, &DANNY), to_eth(10) + to_eth(2));

		// elliot fails to remove any liquidity (he has none)
		assert_noop!(
			Dex::remove_liquidity(
				Origin::signed(ELLIOT),
				usdc,
				weth,
				AssetsExt::balance(lp_usdc_weth, &ELLIOT),
				to_eth(10),
				to_eth(10),
			),
			Error::<Test>::InsufficientLiquidityBurnt
		);

		assert_eq!(AssetsExt::balance(lp_usdc_weth, &CHARLIE), 2_482_001_869_909_090_520_u128);
		// charlie removes all his liquidity
		assert_ok!(Dex::remove_liquidity(
			Origin::signed(CHARLIE),
			usdc,
			weth,
			AssetsExt::balance(lp_usdc_weth, &CHARLIE),
			to_eth(1),
			to_eth(1),
		));
		assert_eq!(AssetsExt::balance(lp_usdc_weth, &CHARLIE), 0u128);
		assert_eq!(AssetsExt::balance(usdc, &CHARLIE), 45_090_951_542_141_088_801_u128);
		assert_eq!(AssetsExt::balance(weth, &CHARLIE), 61_837_465_032_443_069_536_u128);

		// alice removes all her liquidity
		assert_ok!(Dex::remove_liquidity(
			Origin::signed(ALICE),
			usdc,
			weth,
			AssetsExt::balance(lp_usdc_weth, &ALICE),
			to_eth(10),
			to_eth(10),
		));
		assert_eq!(AssetsExt::balance(lp_usdc_weth, &ALICE), 0u128);
		assert_eq!(AssetsExt::balance(usdc, &ALICE), 104_591_585_731_905_646_622_u128);
		assert_eq!(AssetsExt::balance(weth, &ALICE), 90_581_267_483_778_464_043_u128);

		// bob removes all his liquidity
		assert_ok!(Dex::remove_liquidity(
			Origin::signed(BOB),
			usdc,
			weth,
			AssetsExt::balance(lp_usdc_weth, &BOB),
			to_eth(10),
			to_eth(10),
		));
		assert_eq!(AssetsExt::balance(lp_usdc_weth, &BOB), 0u128);
		assert_eq!(AssetsExt::balance(usdc, &BOB), 104_591_585_731_905_647_464_u128);
		assert_eq!(AssetsExt::balance(weth, &BOB), 90_581_267_483_778_465_232_u128);
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

				// create tokens
				let token_0 = AssetsExt::create(&ALICE, None).unwrap();
				let token_1 = AssetsExt::create(&ALICE, None).unwrap();

				// mint input tokens to alice for LP
				assert_ok!(AssetsExt::mint_into(token_0, &ALICE, lp_amount_token_1));
				assert_ok!(AssetsExt::mint_into(token_1, &ALICE, lp_amount_token_2));

				// add liquidity
				assert_ok!(Dex::add_liquidity(
					Origin::signed(ALICE),
					token_0,
					token_1,
					lp_amount_token_1,
					lp_amount_token_2,
					lp_amount_token_1,
					lp_amount_token_2,
					0u128
				));

				// mint input tokens to bob for swap
				assert_ok!(AssetsExt::mint_into(token_0, &BOB, $amount_in));

				let result: Result<u128, DispatchError> = $amount_out;

				match result {
					Ok(amount_out) => {
						assert_ok!(Dex::swap_with_exact_supply(
							Origin::signed(BOB),
							$amount_in,
							$amount_out_min,
							vec![token_0, token_1],
						));

						assert_eq!(AssetsExt::balance(token_0, &BOB), 0u128);
						assert_eq!(AssetsExt::balance(token_1, &BOB), amount_out);
					},
					Err(err) => {
						assert_noop!(
							Dex::swap_with_exact_supply(
								Origin::signed(BOB),
								$amount_in,
								$amount_out_min,
								vec![token_0, token_1],
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

				// create tokens
				let token_0 = AssetsExt::create(&ALICE, None).unwrap();
				let token_1 = AssetsExt::create(&ALICE, None).unwrap();

				// mint input tokens to alice for LP
				assert_ok!(AssetsExt::mint_into(token_0, &ALICE, lp_amount_token_1));
				assert_ok!(AssetsExt::mint_into(token_1, &ALICE, lp_amount_token_2));

				// add liquidity
				assert_ok!(Dex::add_liquidity(
					Origin::signed(ALICE),
					token_0,
					token_1,
					lp_amount_token_1,
					lp_amount_token_2,
					lp_amount_token_1,
					lp_amount_token_2,
					0u128
				));

				// mint input tokens to bob for swap
				assert_ok!(AssetsExt::mint_into(token_0, &BOB, $amount_in_max));

				let result: Result<u128, DispatchError> = $amount_in;

				match result {
					Ok(amount_in) => {
						assert_ok!(Dex::swap_with_exact_target(
							Origin::signed(BOB),
							$amount_out,
							$amount_in_max,
							vec![token_0, token_1],
						));

						assert_eq!(AssetsExt::balance(token_0, &BOB), amount_in);
						assert_eq!(AssetsExt::balance(token_1, &BOB), $amount_out);
					},
					Err(err) => {
						assert_noop!(
							Dex::swap_with_exact_target(
								Origin::signed(BOB),
								$amount_out,
								$amount_in_max,
								vec![token_0, token_1],
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
