#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{
	pBTC, pETH, pETHBTCPair, pUSDC, pUSDCBTCPair, pUSDCETHPair, Dex, Event, ExtBuilder, Origin,
	System, Test, ALICE, BOB,
};
/*
use mock::{
	AUSDBTCPair, AUSDDOTPair, DexModule, Event, ExtBuilder, Origin, Runtime, System, Tokens, ACA, ALICE,
	AUSD, BOB, PBTC, PDOT,
};
*/
//use orml_traits::MultiReservableCurrency;
use sp_runtime::traits::BadOrigin;

#[test]
fn test_run() {
	ExtBuilder::default().build().execute_with(|| assert_eq!(1, 1));
}

#[test]
fn list_provisioning_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			Dex::list_trading_pair(
				Origin::signed(ALICE),
				pUSDC,
				pETH,
				1_000_000_000_000u128,
				1_000_000_000_000u128,
				5_000_000_000_000u128,
				2_000_000_000_000u128,
				10,
			),
			BadOrigin
		);

		assert_eq!(
			Dex::trading_pair_statuses(pUSDCETHPair::get()),
			TradingPairStatus::<_, _>::NotEnabled
		);
		assert_ok!(Dex::list_trading_pair(
			Origin::root(),
			pUSDC,
			pETH,
			1_000_000_000_000u128,
			1_000_000_000_000u128,
			5_000_000_000_000u128,
			2_000_000_000_000u128,
			10,
		));
		// assert_eq!(
		// 	Dex::trading_pair_statuses(pUSDCETHPair::get()),
		// 	TradingPairStatus::<_, _>::Provisioning(TradingPairProvisionParameters {
		// 		min_contribution: (1_000_000_000_000u128, 1_000_000_000_000u128),
		// 		target_provision: (5_000_000_000_000u128, 2_000_000_000_000u128),
		// 		accumulated_provision: (0, 0),
		// 		not_before: 10,
		// 	})
		// );
		System::assert_last_event(Event::Dex(crate::Event::ListTradingPair(pUSDCETHPair::get())));

		assert_noop!(
			Dex::list_trading_pair(
				Origin::root(),
				pUSDC,
				pUSDC,
				1_000_000_000_000u128,
				1_000_000_000_000u128,
				5_000_000_000_000u128,
				2_000_000_000_000u128,
				10,
			),
			Error::<Test>::NotAllowedList
		);

		assert_noop!(
			Dex::list_trading_pair(
				Origin::root(),
				pUSDC,
				pETH,
				1_000_000_000_000u128,
				1_000_000_000_000u128,
				5_000_000_000_000u128,
				2_000_000_000_000u128,
				10,
			),
			Error::<Test>::MustBeNotEnabled
		);
	});
}

#[test]
fn enable_diabled_trading_pair_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(Dex::enable_trading_pair(Origin::signed(ALICE), pUSDC, pETH), BadOrigin);

		assert_eq!(
			Dex::trading_pair_statuses(pUSDCETHPair::get()),
			TradingPairStatus::<_, _>::NotEnabled
		);
		assert_noop!(
			Dex::enable_trading_pair(Origin::root(), pUSDC, pETH),
			Error::<Test>::LiquidityProviderTokenNotCreated
		);

		assert_ok!(Dex::list_trading_pair(Origin::root(), pUSDC, pETH, 1, 1, 1, 1, 100));

		// assert_ok!(Dex::enable_trading_pair(Origin::root(), pUSDC, pETH));

		// assert_eq!(
		// 	Dex::trading_pair_statuses(pUSDCETHPair::get()),
		// 	TradingPairStatus::<_, _>::Enabled
		// );
		// System::assert_last_event(Event::Dex(crate::Event::EnableTradingPair(pUSDCETHPair::
		// get())));

		// assert_noop!(
		// 	Dex::enable_trading_pair(Origin::root(), pETH, pUSDC),
		// 	Error::<Test>::MustBeNotEnabled
		// );
	});
}

// #[test]
// fn enable_provisioning_without_provision_work() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		System::set_block_number(1);

// 		assert_ok!(Dex::list_trading_pair(
// 			Origin::root(),
// 			PUSD,
// 			pETH,
// 			1_000_000_000_000u128,
// 			1_000_000_000_000u128,
// 			5_000_000_000_000u128,
// 			2_000_000_000_000u128,
// 			10,
// 		));
// 		assert_ok!(Dex::list_trading_pair(
// 			Origin::root(),
// 			PUSD,
// 			PBTC,
// 			1_000_000_000_000u128,
// 			1_000_000_000_000u128,
// 			5_000_000_000_000u128,
// 			2_000_000_000_000u128,
// 			10,
// 		));
// 		assert_ok!(Dex::add_liquidity(
// 			Origin::signed(ALICE),
// 			PUSD,
// 			PBTC,
// 			1_000_000_000_000u128,
// 			1_000_000_000_000u128,
// 			0u128, //not used
// 			false, //not used
// 		));

// 		assert_eq!(
// 			Dex::trading_pair_statuses(PUSDDOTPair::get()),
// 			TradingPairStatus::<_, _>::Provisioning(TradingPairProvisionParameters {
// 				min_contribution: (1_000_000_000_000u128, 1_000_000_000_000u128),
// 				target_provision: (5_000_000_000_000u128, 2_000_000_000_000u128),
// 				accumulated_provision: (0, 0),
// 				not_before: 10,
// 			})
// 		);
// 		assert_ok!(Dex::enable_trading_pair(Origin::root(), PUSD, PDOT));
// 		assert_eq!(
// 			Dex::trading_pair_statuses(PUSDDOTPair::get()),
// 			TradingPairStatus::<_, _>::Enabled
// 		);
// 		System::assert_last_event(Event::Dex(crate::Event::EnableTradingPair(PUSDDOTPair::get())));

// 		assert_noop!(
// 			Dex::enable_trading_pair(Origin::root(), PUSD, PBTC),
// 			Error::<Test>::MustBeNotEnabled
// 		);
// 	});
// }

/*
#[test]
fn end_provisioning_trading_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(Dex::list_trading_pair(
			Origin::root(),
			PUSD,
			PDOT,
			1_000_000_000_000u128,
			1_000_000_000_000u128,
			5_000_000_000_000u128,
			2_000_000_000_000u128,
			10,
		));
		assert_eq!(
			Dex::trading_pair_statuses(PUSDDOTPair::get()),
			TradingPairStatus::<_, _>::Provisioning(TradingPairProvisionParameters {
				min_contribution: (1_000_000_000_000u128, 1_000_000_000_000u128),
				target_provision: (5_000_000_000_000u128, 2_000_000_000_000u128),
				accumulated_provision: (0, 0),
				not_before: 10,
			})
		);

		assert_ok!(Dex::list_trading_pair(
			Origin::root(),
			PUSD,
			PBTC,
			1_000_000_000_000u128,
			1_000_000_000_000u128,
			5_000_000_000_000u128,
			2_000_000_000_000u128,
			10,
		));
		assert_ok!(Dex::add_provision(
			Origin::signed(ALICE),
			PUSD,
			PBTC,
			1_000_000_000_000u128,
			2_000_000_000_000u128
		));

		assert_noop!(
			Dex::end_provisioning(Origin::root(), PUSD, PBTC),
			Error::<Test>::UnqualifiedProvision
		);
		System::set_block_number(10);

		assert_eq!(
			Dex::trading_pair_statuses(PUSDBTCPair::get()),
			TradingPairStatus::<_, _>::Provisioning(TradingPairProvisionParameters {
				min_contribution: (1_000_000_000_000u128, 1_000_000_000_000u128),
				target_provision: (5_000_000_000_000u128, 2_000_000_000_000u128),
				accumulated_provision: (1_000_000_000_000u128, 2_000_000_000_000u128),
				not_before: 10,
			})
		);
		assert_eq!(
			Dex::initial_share_exchange_rates(PUSDBTCPair::get()),
			Default::default()
		);
		assert_eq!(Dex::liquidity_pool(PUSDBTCPair::get()), (0, 0));
		assert_eq!(Assets::total_issuance(PUSDBTCPair::get().dex_share_currency_id()), 0);
		assert_eq!(
			Assets::free_balance(PUSDBTCPair::get().dex_share_currency_id(), &Dex::account_id()),
			0
		);

		assert_ok!(Dex::end_provisioning(
			Origin::root(),
			PUSD,
			PBTC
		));
		System::assert_last_event(Event::Dex(crate::Event::ProvisioningToEnabled(
			PUSDBTCPair::get(),
			1_000_000_000_000u128,
			2_000_000_000_000u128,
			2_000_000_000_000u128,
		)));
		assert_eq!(
			Dex::trading_pair_statuses(PUSDBTCPair::get()),
			TradingPairStatus::<_, _>::Enabled
		);
		assert_eq!(
			Dex::initial_share_exchange_rates(PUSDBTCPair::get()),
			(ExchangeRate::one(), ExchangeRate::checked_from_rational(1, 2).unwrap())
		);
		assert_eq!(
			Dex::liquidity_pool(PUSDBTCPair::get()),
			(1_000_000_000_000u128, 2_000_000_000_000u128)
		);
		assert_eq!(
			Assets::total_issuance(PUSDBTCPair::get().dex_share_currency_id()),
			2_000_000_000_000u128
		);
		assert_eq!(
			Assets::free_balance(PUSDBTCPair::get().dex_share_currency_id(), &Dex::account_id()),
			2_000_000_000_000u128
		);
	});
}

#[test]
fn disable_trading_pair_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(Dex::enable_trading_pair(
			Origin::root(),
			PUSD,
			PDOT
		));
		assert_eq!(
			Dex::trading_pair_statuses(PUSDDOTPair::get()),
			TradingPairStatus::<_, _>::Enabled
		);

		assert_noop!(
			Dex::disable_trading_pair(Origin::signed(ALICE), PUSD, PDOT),
			BadOrigin
		);

		assert_ok!(Dex::disable_trading_pair(
			Origin::root(),
			PUSD,
			PDOT
		));
		assert_eq!(
			Dex::trading_pair_statuses(PUSDDOTPair::get()),
			TradingPairStatus::<_, _>::NotEnabled
		);
		System::assert_last_event(Event::Dex(crate::Event::DisableTradingPair(PUSDDOTPair::get())));

		assert_noop!(
			Dex::disable_trading_pair(Origin::root(), PUSD, PDOT),
			Error::<Test>::MustBeEnabled
		);

		assert_ok!(Dex::list_trading_pair(
			Origin::root(),
			PUSD,
			PBTC,
			1_000_000_000_000u128,
			1_000_000_000_000u128,
			5_000_000_000_000u128,
			2_000_000_000_000u128,
			10,
		));
		assert_noop!(
			Dex::disable_trading_pair(Origin::root(), PUSD, PBTC),
			Error::<Test>::MustBeEnabled
		);
	});
}

#[test]
fn add_provision_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			Dex::add_provision(
				Origin::signed(ALICE),
				PUSD,
				PDOT,
				5_000_000_000_000u128,
				1_000_000_000_000u128,
			),
			Error::<Test>::MustBeProvisioning
		);

		assert_ok!(Dex::list_trading_pair(
			Origin::root(),
			PUSD,
			PDOT,
			5_000_000_000_000u128,
			1_000_000_000_000u128,
			5_000_000_000_000_000u128,
			1_000_000_000_000_000u128,
			10,
		));

		assert_noop!(
			Dex::add_provision(
				Origin::signed(ALICE),
				PUSD,
				PDOT,
				4_999_999_999_999u128,
				999_999_999_999u128,
			),
			Error::<Test>::InvalidContributionIncrement
		);

		assert_eq!(
			Dex::trading_pair_statuses(PUSDDOTPair::get()),
			TradingPairStatus::<_, _>::Provisioning(TradingPairProvisionParameters {
				min_contribution: (5_000_000_000_000u128, 1_000_000_000_000u128),
				target_provision: (5_000_000_000_000_000u128, 1_000_000_000_000_000u128),
				accumulated_provision: (0, 0),
				not_before: 10,
			})
		);
		assert_eq!(Dex::provisioning_pool(PUSDDOTPair::get(), ALICE), (0, 0));
		assert_eq!(Assets::free_balance(PUSD, &ALICE), 1_000_000_000_000_000_000u128);
		assert_eq!(Assets::free_balance(PDOT, &ALICE), 1_000_000_000_000_000_000u128);
		assert_eq!(Assets::free_balance(PUSD, &Dex::account_id()), 0);
		assert_eq!(Assets::free_balance(PDOT, &Dex::account_id()), 0);
		let alice_ref_count_0 = System::consumers(&ALICE);

		assert_ok!(Dex::add_provision(
			Origin::signed(ALICE),
			PUSD,
			PDOT,
			5_000_000_000_000u128,
			0,
		));
		assert_eq!(
			Dex::trading_pair_statuses(PUSDDOTPair::get()),
			TradingPairStatus::<_, _>::Provisioning(TradingPairProvisionParameters {
				min_contribution: (5_000_000_000_000u128, 1_000_000_000_000u128),
				target_provision: (5_000_000_000_000_000u128, 1_000_000_000_000_000u128),
				accumulated_provision: (5_000_000_000_000u128, 0),
				not_before: 10,
			})
		);
		assert_eq!(
			Dex::provisioning_pool(PUSDDOTPair::get(), ALICE),
			(5_000_000_000_000u128, 0)
		);
		assert_eq!(Assets::free_balance(PUSD, &ALICE), 999_995_000_000_000_000u128);
		assert_eq!(Assets::free_balance(PDOT, &ALICE), 1_000_000_000_000_000_000u128);
		assert_eq!(
			Assets::free_balance(PUSD, &Dex::account_id()),
			5_000_000_000_000u128
		);
		assert_eq!(Assets::free_balance(PDOT, &Dex::account_id()), 0);
		let alice_ref_count_1 = System::consumers(&ALICE);
		assert_eq!(alice_ref_count_1, alice_ref_count_0 + 1);
		System::assert_last_event(Event::Dex(crate::Event::AddProvision(
			ALICE,
			PUSD,
			5_000_000_000_000u128,
			PDOT,
			0,
		)));
	});
}

#[test]
fn claim_dex_share_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(Dex::list_trading_pair(
			Origin::root(),
			PUSD,
			PDOT,
			5_000_000_000_000u128,
			1_000_000_000_000u128,
			5_000_000_000_000_000u128,
			1_000_000_000_000_000u128,
			0,
		));

		assert_ok!(Dex::add_provision(
			Origin::signed(ALICE),
			PUSD,
			PDOT,
			1_000_000_000_000_000u128,
			200_000_000_000_000u128,
		));
		assert_ok!(Dex::add_provision(
			Origin::signed(BOB),
			PUSD,
			PDOT,
			4_000_000_000_000_000u128,
			800_000_000_000_000u128,
		));

		assert_noop!(
			Dex::claim_dex_share(Origin::signed(ALICE), ALICE, PUSD, PDOT),
			Error::<Test>::StillProvisioning
		);

		assert_ok!(Dex::end_provisioning(
			Origin::root(),
			PUSD,
			PDOT
		));

		let lp_currency_id = PUSDDOTPair::get().dex_share_currency_id();

		assert_eq!(
			InitialShareExchangeRates::<Test>::contains_key(PUSDDOTPair::get()),
			true
		);
		assert_eq!(
			Dex::initial_share_exchange_rates(PUSDDOTPair::get()),
			(ExchangeRate::one(), ExchangeRate::saturating_from_rational(5, 1))
		);
		assert_eq!(
			Assets::free_balance(lp_currency_id, &Dex::account_id()),
			10_000_000_000_000_000u128
		);
		assert_eq!(
			Dex::provisioning_pool(PUSDDOTPair::get(), ALICE),
			(1_000_000_000_000_000u128, 200_000_000_000_000u128)
		);
		assert_eq!(
			Dex::provisioning_pool(PUSDDOTPair::get(), BOB),
			(4_000_000_000_000_000u128, 800_000_000_000_000u128)
		);
		assert_eq!(Assets::free_balance(lp_currency_id, &ALICE), 0);
		assert_eq!(Assets::free_balance(lp_currency_id, &BOB), 0);

		let alice_ref_count_0 = System::consumers(&ALICE);
		let bob_ref_count_0 = System::consumers(&BOB);

		assert_ok!(Dex::claim_dex_share(Origin::signed(ALICE), ALICE, PUSD, PDOT));
		assert_eq!(
			Assets::free_balance(lp_currency_id, &Dex::account_id()),
			8_000_000_000_000_000u128
		);
		assert_eq!(Dex::provisioning_pool(PUSDDOTPair::get(), ALICE), (0, 0));
		assert_eq!(Assets::free_balance(lp_currency_id, &ALICE), 2_000_000_000_000_000u128);
		assert_eq!(System::consumers(&ALICE), alice_ref_count_0 - 1);
		assert_eq!(
			InitialShareExchangeRates::<Test>::contains_key(PUSDDOTPair::get()),
			true
		);

		assert_ok!(Dex::disable_trading_pair(
			Origin::root(),
			PUSD,
			PDOT
		));
		assert_ok!(Dex::claim_dex_share(Origin::signed(BOB), BOB, PUSD, PDOT));
		assert_eq!(Assets::free_balance(lp_currency_id, &Dex::account_id()), 0);
		assert_eq!(Dex::provisioning_pool(PUSDDOTPair::get(), BOB), (0, 0));
		assert_eq!(Assets::free_balance(lp_currency_id, &BOB), 8_000_000_000_000_000u128);
		assert_eq!(System::consumers(&BOB), bob_ref_count_0 - 1);
		assert_eq!(
			InitialShareExchangeRates::<Test>::contains_key(PUSDDOTPair::get()),
			false
		);
	});
}

#[test]
fn get_liquidity_work() {
	ExtBuilder::default().build().execute_with(|| {
		LiquidityPool::<Test>::insert(PUSDDOTPair::get(), (1000, 20));
		assert_eq!(Dex::liquidity_pool(PUSDDOTPair::get()), (1000, 20));
		assert_eq!(Dex::get_liquidity(PUSD, PDOT), (1000, 20));
		assert_eq!(Dex::get_liquidity(PDOT, PUSD), (20, 1000));
	});
}

#[test]
fn get_target_amount_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Dex::get_target_amount(10000, 0, 1000), 0);
		assert_eq!(Dex::get_target_amount(0, 20000, 1000), 0);
		assert_eq!(Dex::get_target_amount(10000, 20000, 0), 0);
		assert_eq!(Dex::get_target_amount(10000, 1, 1000000), 0);
		assert_eq!(Dex::get_target_amount(10000, 20000, 10000), 9949);
		assert_eq!(Dex::get_target_amount(10000, 20000, 1000), 1801);
	});
}

#[test]
fn get_supply_amount_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Dex::get_supply_amount(10000, 0, 1000), 0);
		assert_eq!(Dex::get_supply_amount(0, 20000, 1000), 0);
		assert_eq!(Dex::get_supply_amount(10000, 20000, 0), 0);
		assert_eq!(Dex::get_supply_amount(10000, 1, 1), 0);
		assert_eq!(Dex::get_supply_amount(10000, 20000, 9949), 9999);
		assert_eq!(Dex::get_target_amount(10000, 20000, 9999), 9949);
		assert_eq!(Dex::get_supply_amount(10000, 20000, 1801), 1000);
		assert_eq!(Dex::get_target_amount(10000, 20000, 1000), 1801);
	});
}

#[test]
fn get_target_amounts_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.build()
		.execute_with(|| {
			LiquidityPool::<Test>::insert(PUSDDOTPair::get(), (50000, 10000));
			LiquidityPool::<Test>::insert(PUSDBTCPair::get(), (100000, 10));
			assert_noop!(
				Dex::get_target_amounts(&vec![PDOT], 10000, None),
				Error::<Test>::InvalidTradingPathLength,
			);
			assert_noop!(
				Dex::get_target_amounts(&vec![PDOT, PUSD, PBTC, PDOT], 10000, None),
				Error::<Test>::InvalidTradingPathLength,
			);
			assert_noop!(
				Dex::get_target_amounts(&vec![PDOT, PUSD, ACA], 10000, None),
				Error::<Test>::MustBeEnabled,
			);
			assert_eq!(
				Dex::get_target_amounts(&vec![PDOT, PUSD], 10000, None),
				Ok(vec![10000, 24874])
			);
			assert_eq!(
				Dex::get_target_amounts(&vec![PDOT, PUSD], 10000, Ratio::checked_from_rational(50, 100)),
				Ok(vec![10000, 24874])
			);
			assert_noop!(
				Dex::get_target_amounts(&vec![PDOT, PUSD], 10000, Ratio::checked_from_rational(49, 100)),
				Error::<Test>::ExceedPriceImpactLimit,
			);
			assert_eq!(
				Dex::get_target_amounts(&vec![PDOT, PUSD, PBTC], 10000, None),
				Ok(vec![10000, 24874, 1])
			);
			assert_noop!(
				Dex::get_target_amounts(&vec![PDOT, PUSD, PBTC], 100, None),
				Error::<Test>::ZeroTargetAmount,
			);
			assert_noop!(
				Dex::get_target_amounts(&vec![PDOT, PBTC], 100, None),
				Error::<Test>::InsufficientLiquidity,
			);
		});
}

#[test]
fn calculate_amount_for_big_number_work() {
	ExtBuilder::default().build().execute_with(|| {
		LiquidityPool::<Test>::insert(
			PUSDDOTPair::get(),
			(171_000_000_000_000_000_000_000, 56_000_000_000_000_000_000_000),
		);
		assert_eq!(
			Dex::get_supply_amount(
				171_000_000_000_000_000_000_000,
				56_000_000_000_000_000_000_000,
				1_000_000_000_000_000_000_000
			),
			3_140_495_867_768_595_041_323
		);
		assert_eq!(
			Dex::get_target_amount(
				171_000_000_000_000_000_000_000,
				56_000_000_000_000_000_000_000,
				3_140_495_867_768_595_041_323
			),
			1_000_000_000_000_000_000_000
		);
	});
}

#[test]
fn get_supply_amounts_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.build()
		.execute_with(|| {
			LiquidityPool::<Test>::insert(PUSDDOTPair::get(), (50000, 10000));
			LiquidityPool::<Test>::insert(PUSDBTCPair::get(), (100000, 10));
			assert_noop!(
				Dex::get_supply_amounts(&vec![PDOT], 10000, None),
				Error::<Test>::InvalidTradingPathLength,
			);
			assert_noop!(
				Dex::get_supply_amounts(&vec![PDOT, PUSD, PBTC, PDOT], 10000, None),
				Error::<Test>::InvalidTradingPathLength,
			);
			assert_noop!(
				Dex::get_supply_amounts(&vec![PDOT, PUSD, ACA], 10000, None),
				Error::<Test>::MustBeEnabled,
			);
			assert_eq!(
				Dex::get_supply_amounts(&vec![PDOT, PUSD], 24874, None),
				Ok(vec![10000, 24874])
			);
			assert_eq!(
				Dex::get_supply_amounts(&vec![PDOT, PUSD], 25000, Ratio::checked_from_rational(50, 100)),
				Ok(vec![10102, 25000])
			);
			assert_noop!(
				Dex::get_supply_amounts(&vec![PDOT, PUSD], 25000, Ratio::checked_from_rational(49, 100)),
				Error::<Test>::ExceedPriceImpactLimit,
			);
			assert_noop!(
				Dex::get_supply_amounts(&vec![PDOT, PUSD, PBTC], 10000, None),
				Error::<Test>::ZeroSupplyAmount,
			);
			assert_noop!(
				Dex::get_supply_amounts(&vec![PDOT, PBTC], 10000, None),
				Error::<Test>::InsufficientLiquidity,
			);
		});
}

#[test]
fn _swap_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.build()
		.execute_with(|| {
			LiquidityPool::<Test>::insert(PUSDDOTPair::get(), (50000, 10000));

			assert_eq!(Dex::get_liquidity(PUSD, PDOT), (50000, 10000));
			assert_noop!(
				Dex::_swap(PUSD, PDOT, 50000, 5001),
				Error::<Test>::InvariantCheckFailed
			);
			assert_ok!(Dex::_swap(PUSD, PDOT, 50000, 5000));
			assert_eq!(Dex::get_liquidity(PUSD, PDOT), (100000, 5000));
			assert_ok!(Dex::_swap(PDOT, PUSD, 100, 800));
			assert_eq!(Dex::get_liquidity(PUSD, PDOT), (99200, 5100));
		});
}

#[test]
fn _swap_by_path_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.build()
		.execute_with(|| {
			LiquidityPool::<Test>::insert(PUSDDOTPair::get(), (50000, 10000));
			LiquidityPool::<Test>::insert(PUSDBTCPair::get(), (100000, 10));

			assert_eq!(Dex::get_liquidity(PUSD, PDOT), (50000, 10000));
			assert_eq!(Dex::get_liquidity(PUSD, PBTC), (100000, 10));
			assert_ok!(Dex::_swap_by_path(&vec![PDOT, PUSD], &vec![10000, 25000]));
			assert_eq!(Dex::get_liquidity(PUSD, PDOT), (25000, 20000));
			assert_ok!(Dex::_swap_by_path(&vec![PDOT, PUSD, PBTC], &vec![100000, 20000, 1]));
			assert_eq!(Dex::get_liquidity(PUSD, PDOT), (5000, 120000));
			assert_eq!(Dex::get_liquidity(PUSD, PBTC), (120000, 9));
		});
}

#[test]
fn add_liquidity_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_noop!(
				Dex::add_liquidity(Origin::signed(ALICE), ACA, PUSD, 100_000_000, 100_000_000, 0, false),
				Error::<Test>::MustBeEnabled
			);
			assert_noop!(
				Dex::add_liquidity(Origin::signed(ALICE), PUSD, PDOT, 0, 100_000_000, 0, false),
				Error::<Test>::InvalidLiquidityIncrement
			);

			assert_eq!(Dex::get_liquidity(PUSD, PDOT), (0, 0));
			assert_eq!(Assets::free_balance(PUSD, &Dex::account_id()), 0);
			assert_eq!(Assets::free_balance(PDOT, &Dex::account_id()), 0);
			assert_eq!(
				Assets::free_balance(PUSDDOTPair::get().dex_share_currency_id(), &ALICE),
				0
			);
			assert_eq!(
				Assets::reserved_balance(PUSDDOTPair::get().dex_share_currency_id(), &ALICE),
				0
			);
			assert_eq!(Assets::free_balance(PUSD, &ALICE), 1_000_000_000_000_000_000);
			assert_eq!(Assets::free_balance(PDOT, &ALICE), 1_000_000_000_000_000_000);

			assert_ok!(Dex::add_liquidity(
				Origin::signed(ALICE),
				PUSD,
				PDOT,
				5_000_000_000_000,
				1_000_000_000_000,
				0,
				false,
			));
			System::assert_last_event(Event::Dex(crate::Event::AddLiquidity(
				ALICE,
				PUSD,
				5_000_000_000_000,
				PDOT,
				1_000_000_000_000,
				10_000_000_000_000,
			)));
			assert_eq!(
				Dex::get_liquidity(PUSD, PDOT),
				(5_000_000_000_000, 1_000_000_000_000)
			);
			assert_eq!(Assets::free_balance(PUSD, &Dex::account_id()), 5_000_000_000_000);
			assert_eq!(Assets::free_balance(PDOT, &Dex::account_id()), 1_000_000_000_000);
			assert_eq!(
				Assets::free_balance(PUSDDOTPair::get().dex_share_currency_id(), &ALICE),
				10_000_000_000_000
			);
			assert_eq!(
				Assets::reserved_balance(PUSDDOTPair::get().dex_share_currency_id(), &ALICE),
				0
			);
			assert_eq!(Assets::free_balance(PUSD, &ALICE), 999_995_000_000_000_000);
			assert_eq!(Assets::free_balance(PDOT, &ALICE), 999_999_000_000_000_000);
			assert_eq!(
				Assets::free_balance(PUSDDOTPair::get().dex_share_currency_id(), &BOB),
				0
			);
			assert_eq!(
				Assets::reserved_balance(PUSDDOTPair::get().dex_share_currency_id(), &BOB),
				0
			);
			assert_eq!(Assets::free_balance(PUSD, &BOB), 1_000_000_000_000_000_000);
			assert_eq!(Assets::free_balance(PDOT, &BOB), 1_000_000_000_000_000_000);

			assert_noop!(
				Dex::add_liquidity(Origin::signed(BOB), PUSD, PDOT, 4, 1, 0, true,),
				Error::<Test>::InvalidLiquidityIncrement,
			);

			assert_noop!(
				Dex::add_liquidity(
					Origin::signed(BOB),
					PUSD,
					PDOT,
					50_000_000_000_000,
					8_000_000_000_000,
					80_000_000_000_001,
					true,
				),
				Error::<Test>::UnacceptableShareIncrement
			);

			assert_ok!(Dex::add_liquidity(
				Origin::signed(BOB),
				PUSD,
				PDOT,
				50_000_000_000_000,
				8_000_000_000_000,
				80_000_000_000_000,
				true,
			));
			System::assert_last_event(Event::Dex(crate::Event::AddLiquidity(
				BOB,
				PUSD,
				40_000_000_000_000,
				PDOT,
				8_000_000_000_000,
				80_000_000_000_000,
			)));
			assert_eq!(
				Dex::get_liquidity(PUSD, PDOT),
				(45_000_000_000_000, 9_000_000_000_000)
			);
			assert_eq!(Assets::free_balance(PUSD, &Dex::account_id()), 45_000_000_000_000);
			assert_eq!(Assets::free_balance(PDOT, &Dex::account_id()), 9_000_000_000_000);
			assert_eq!(
				Assets::free_balance(PUSDDOTPair::get().dex_share_currency_id(), &BOB),
				0
			);
			assert_eq!(
				Assets::reserved_balance(PUSDDOTPair::get().dex_share_currency_id(), &BOB),
				80_000_000_000_000
			);
			assert_eq!(Assets::free_balance(PUSD, &BOB), 999_960_000_000_000_000);
			assert_eq!(Assets::free_balance(PDOT, &BOB), 999_992_000_000_000_000);
		});
}

#[test]
fn remove_liquidity_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_ok!(Dex::add_liquidity(
				Origin::signed(ALICE),
				PUSD,
				PDOT,
				5_000_000_000_000,
				1_000_000_000_000,
				0,
				false
			));
			assert_noop!(
				Dex::remove_liquidity(
					Origin::signed(ALICE),
					PUSDDOTPair::get().dex_share_currency_id(),
					PDOT,
					100_000_000,
					0,
					0,
					false,
				),
				Error::<Test>::InvalidCurrencyId
			);

			assert_eq!(
				Dex::get_liquidity(PUSD, PDOT),
				(5_000_000_000_000, 1_000_000_000_000)
			);
			assert_eq!(Assets::free_balance(PUSD, &Dex::account_id()), 5_000_000_000_000);
			assert_eq!(Assets::free_balance(PDOT, &Dex::account_id()), 1_000_000_000_000);
			assert_eq!(
				Assets::free_balance(PUSDDOTPair::get().dex_share_currency_id(), &ALICE),
				10_000_000_000_000
			);
			assert_eq!(Assets::free_balance(PUSD, &ALICE), 999_995_000_000_000_000);
			assert_eq!(Assets::free_balance(PDOT, &ALICE), 999_999_000_000_000_000);

			assert_noop!(
				Dex::remove_liquidity(
					Origin::signed(ALICE),
					PUSD,
					PDOT,
					8_000_000_000_000,
					4_000_000_000_001,
					800_000_000_000,
					false,
				),
				Error::<Test>::UnacceptableLiquidityWithdrawn
			);
			assert_noop!(
				Dex::remove_liquidity(
					Origin::signed(ALICE),
					PUSD,
					PDOT,
					8_000_000_000_000,
					4_000_000_000_000,
					800_000_000_001,
					false,
				),
				Error::<Test>::UnacceptableLiquidityWithdrawn
			);
			assert_ok!(Dex::remove_liquidity(
				Origin::signed(ALICE),
				PUSD,
				PDOT,
				8_000_000_000_000,
				4_000_000_000_000,
				800_000_000_000,
				false,
			));
			System::assert_last_event(Event::Dex(crate::Event::RemoveLiquidity(
				ALICE,
				PUSD,
				4_000_000_000_000,
				PDOT,
				800_000_000_000,
				8_000_000_000_000,
			)));
			assert_eq!(
				Dex::get_liquidity(PUSD, PDOT),
				(1_000_000_000_000, 200_000_000_000)
			);
			assert_eq!(Assets::free_balance(PUSD, &Dex::account_id()), 1_000_000_000_000);
			assert_eq!(Assets::free_balance(PDOT, &Dex::account_id()), 200_000_000_000);
			assert_eq!(
				Assets::free_balance(PUSDDOTPair::get().dex_share_currency_id(), &ALICE),
				2_000_000_000_000
			);
			assert_eq!(Assets::free_balance(PUSD, &ALICE), 999_999_000_000_000_000);
			assert_eq!(Assets::free_balance(PDOT, &ALICE), 999_999_800_000_000_000);

			assert_ok!(Dex::remove_liquidity(
				Origin::signed(ALICE),
				PUSD,
				PDOT,
				2_000_000_000_000,
				0,
				0,
				false,
			));
			System::assert_last_event(Event::Dex(crate::Event::RemoveLiquidity(
				ALICE,
				PUSD,
				1_000_000_000_000,
				PDOT,
				200_000_000_000,
				2_000_000_000_000,
			)));
			assert_eq!(Dex::get_liquidity(PUSD, PDOT), (0, 0));
			assert_eq!(Assets::free_balance(PUSD, &Dex::account_id()), 0);
			assert_eq!(Assets::free_balance(PDOT, &Dex::account_id()), 0);
			assert_eq!(
				Assets::free_balance(PUSDDOTPair::get().dex_share_currency_id(), &ALICE),
				0
			);
			assert_eq!(Assets::free_balance(PUSD, &ALICE), 1_000_000_000_000_000_000);
			assert_eq!(Assets::free_balance(PDOT, &ALICE), 1_000_000_000_000_000_000);

			assert_ok!(Dex::add_liquidity(
				Origin::signed(BOB),
				PUSD,
				PDOT,
				5_000_000_000_000,
				1_000_000_000_000,
				0,
				true
			));
			assert_eq!(
				Assets::free_balance(PUSDDOTPair::get().dex_share_currency_id(), &BOB),
				0
			);
			assert_eq!(
				Assets::reserved_balance(PUSDDOTPair::get().dex_share_currency_id(), &BOB),
				10_000_000_000_000
			);
			assert_ok!(Dex::remove_liquidity(
				Origin::signed(BOB),
				PUSD,
				PDOT,
				2_000_000_000_000,
				0,
				0,
				true,
			));
			assert_eq!(
				Assets::free_balance(PUSDDOTPair::get().dex_share_currency_id(), &BOB),
				0
			);
			assert_eq!(
				Assets::reserved_balance(PUSDDOTPair::get().dex_share_currency_id(), &BOB),
				8_000_000_000_000
			);
		});
}

#[test]
fn do_swap_with_exact_supply_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_ok!(Dex::add_liquidity(
				Origin::signed(ALICE),
				PUSD,
				PDOT,
				500_000_000_000_000,
				100_000_000_000_000,
				0,
				false,
			));
			assert_ok!(Dex::add_liquidity(
				Origin::signed(ALICE),
				PUSD,
				PBTC,
				100_000_000_000_000,
				10_000_000_000,
				0,
				false,
			));

			assert_eq!(
				Dex::get_liquidity(PUSD, PDOT),
				(500_000_000_000_000, 100_000_000_000_000)
			);
			assert_eq!(
				Dex::get_liquidity(PUSD, PBTC),
				(100_000_000_000_000, 10_000_000_000)
			);
			assert_eq!(
				Assets::free_balance(PUSD, &Dex::account_id()),
				600_000_000_000_000
			);
			assert_eq!(Assets::free_balance(PDOT, &Dex::account_id()), 100_000_000_000_000);
			assert_eq!(Assets::free_balance(PBTC, &Dex::account_id()), 10_000_000_000);
			assert_eq!(Assets::free_balance(PUSD, &BOB), 1_000_000_000_000_000_000);
			assert_eq!(Assets::free_balance(PDOT, &BOB), 1_000_000_000_000_000_000);
			assert_eq!(Assets::free_balance(PBTC, &BOB), 1_000_000_000_000_000_000);

			assert_noop!(
				Dex::do_swap_with_exact_supply(
					&BOB,
					&[PDOT, PUSD],
					100_000_000_000_000,
					250_000_000_000_000,
					None
				),
				Error::<Test>::InsufficientTargetAmount
			);
			assert_noop!(
				Dex::do_swap_with_exact_supply(
					&BOB,
					&[PDOT, PUSD],
					100_000_000_000_000,
					0,
					Ratio::checked_from_rational(10, 100)
				),
				Error::<Test>::ExceedPriceImpactLimit,
			);
			assert_noop!(
				Dex::do_swap_with_exact_supply(&BOB, &[PDOT, PUSD, PBTC, PDOT], 100_000_000_000_000, 0, None),
				Error::<Test>::InvalidTradingPathLength,
			);
			assert_noop!(
				Dex::do_swap_with_exact_supply(&BOB, &[PDOT, ACA], 100_000_000_000_000, 0, None),
				Error::<Test>::MustBeEnabled,
			);

			assert_ok!(Dex::do_swap_with_exact_supply(
				&BOB,
				&[PDOT, PUSD],
				100_000_000_000_000,
				200_000_000_000_000,
				None
			));
			System::assert_last_event(Event::Dex(crate::Event::Swap(
				BOB,
				vec![PDOT, PUSD],
				100_000_000_000_000,
				248_743_718_592_964,
			)));
			assert_eq!(
				Dex::get_liquidity(PUSD, PDOT),
				(251_256_281_407_036, 200_000_000_000_000)
			);
			assert_eq!(
				Dex::get_liquidity(PUSD, PBTC),
				(100_000_000_000_000, 10_000_000_000)
			);
			assert_eq!(
				Assets::free_balance(PUSD, &Dex::account_id()),
				351_256_281_407_036
			);
			assert_eq!(Assets::free_balance(PDOT, &Dex::account_id()), 200_000_000_000_000);
			assert_eq!(Assets::free_balance(PBTC, &Dex::account_id()), 10_000_000_000);
			assert_eq!(Assets::free_balance(PUSD, &BOB), 1_000_248_743_718_592_964);
			assert_eq!(Assets::free_balance(PDOT, &BOB), 999_900_000_000_000_000);
			assert_eq!(Assets::free_balance(PBTC, &BOB), 1_000_000_000_000_000_000);

			assert_ok!(Dex::do_swap_with_exact_supply(
				&BOB,
				&[PDOT, PUSD, PBTC],
				200_000_000_000_000,
				1,
				None
			));
			System::assert_last_event(Event::Dex(crate::Event::Swap(
				BOB,
				vec![PDOT, PUSD, PBTC],
				200_000_000_000_000,
				5_530_663_837,
			)));
			assert_eq!(
				Dex::get_liquidity(PUSD, PDOT),
				(126_259_437_892_983, 400_000_000_000_000)
			);
			assert_eq!(
				Dex::get_liquidity(PUSD, PBTC),
				(224_996_843_514_053, 4_469_336_163)
			);
			assert_eq!(
				Assets::free_balance(PUSD, &Dex::account_id()),
				351_256_281_407_036
			);
			assert_eq!(Assets::free_balance(PDOT, &Dex::account_id()), 400_000_000_000_000);
			assert_eq!(Assets::free_balance(PBTC, &Dex::account_id()), 4_469_336_163);
			assert_eq!(Assets::free_balance(PUSD, &BOB), 1_000_248_743_718_592_964);
			assert_eq!(Assets::free_balance(PDOT, &BOB), 999_700_000_000_000_000);
			assert_eq!(Assets::free_balance(PBTC, &BOB), 1_000_000_005_530_663_837);
		});
}

#[test]
fn do_swap_with_exact_target_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_ok!(Dex::add_liquidity(
				Origin::signed(ALICE),
				PUSD,
				PDOT,
				500_000_000_000_000,
				100_000_000_000_000,
				0,
				false,
			));
			assert_ok!(Dex::add_liquidity(
				Origin::signed(ALICE),
				PUSD,
				PBTC,
				100_000_000_000_000,
				10_000_000_000,
				0,
				false,
			));

			assert_eq!(
				Dex::get_liquidity(PUSD, PDOT),
				(500_000_000_000_000, 100_000_000_000_000)
			);
			assert_eq!(
				Dex::get_liquidity(PUSD, PBTC),
				(100_000_000_000_000, 10_000_000_000)
			);
			assert_eq!(
				Assets::free_balance(PUSD, &Dex::account_id()),
				600_000_000_000_000
			);
			assert_eq!(Assets::free_balance(PDOT, &Dex::account_id()), 100_000_000_000_000);
			assert_eq!(Assets::free_balance(PBTC, &Dex::account_id()), 10_000_000_000);
			assert_eq!(Assets::free_balance(PUSD, &BOB), 1_000_000_000_000_000_000);
			assert_eq!(Assets::free_balance(PDOT, &BOB), 1_000_000_000_000_000_000);
			assert_eq!(Assets::free_balance(PBTC, &BOB), 1_000_000_000_000_000_000);

			assert_noop!(
				Dex::do_swap_with_exact_target(
					&BOB,
					&[PDOT, PUSD],
					250_000_000_000_000,
					100_000_000_000_000,
					None
				),
				Error::<Test>::ExcessiveSupplyAmount
			);
			assert_noop!(
				Dex::do_swap_with_exact_target(
					&BOB,
					&[PDOT, PUSD],
					250_000_000_000_000,
					200_000_000_000_000,
					Ratio::checked_from_rational(10, 100)
				),
				Error::<Test>::ExceedPriceImpactLimit,
			);
			assert_noop!(
				Dex::do_swap_with_exact_target(
					&BOB,
					&[PDOT, PUSD, PBTC, PDOT],
					250_000_000_000_000,
					200_000_000_000_000,
					None
				),
				Error::<Test>::InvalidTradingPathLength,
			);
			assert_noop!(
				Dex::do_swap_with_exact_target(&BOB, &[PDOT, ACA], 250_000_000_000_000, 200_000_000_000_000, None),
				Error::<Test>::MustBeEnabled,
			);

			assert_ok!(Dex::do_swap_with_exact_target(
				&BOB,
				&[PDOT, PUSD],
				250_000_000_000_000,
				200_000_000_000_000,
				None
			));
			System::assert_last_event(Event::Dex(crate::Event::Swap(
				BOB,
				vec![PDOT, PUSD],
				101_010_101_010_102,
				250_000_000_000_000,
			)));
			assert_eq!(
				Dex::get_liquidity(PUSD, PDOT),
				(250_000_000_000_000, 201_010_101_010_102)
			);
			assert_eq!(
				Dex::get_liquidity(PUSD, PBTC),
				(100_000_000_000_000, 10_000_000_000)
			);
			assert_eq!(
				Assets::free_balance(PUSD, &Dex::account_id()),
				350_000_000_000_000
			);
			assert_eq!(Assets::free_balance(PDOT, &Dex::account_id()), 201_010_101_010_102);
			assert_eq!(Assets::free_balance(PBTC, &Dex::account_id()), 10_000_000_000);
			assert_eq!(Assets::free_balance(PUSD, &BOB), 1_000_250_000_000_000_000);
			assert_eq!(Assets::free_balance(PDOT, &BOB), 999_898_989_898_989_898);
			assert_eq!(Assets::free_balance(PBTC, &BOB), 1_000_000_000_000_000_000);

			assert_ok!(Dex::do_swap_with_exact_target(
				&BOB,
				&[PDOT, PUSD, PBTC],
				5_000_000_000,
				2_000_000_000_000_000,
				None
			));
			System::assert_last_event(Event::Dex(crate::Event::Swap(
				BOB,
				vec![PDOT, PUSD, PBTC],
				137_654_580_386_993,
				5_000_000_000,
			)));
			assert_eq!(
				Dex::get_liquidity(PUSD, PDOT),
				(148_989_898_989_898, 338_664_681_397_095)
			);
			assert_eq!(
				Dex::get_liquidity(PUSD, PBTC),
				(201_010_101_010_102, 5_000_000_000)
			);
			assert_eq!(
				Assets::free_balance(PUSD, &Dex::account_id()),
				350_000_000_000_000
			);
			assert_eq!(Assets::free_balance(PDOT, &Dex::account_id()), 338_664_681_397_095);
			assert_eq!(Assets::free_balance(PBTC, &Dex::account_id()), 5_000_000_000);
			assert_eq!(Assets::free_balance(PUSD, &BOB), 1_000_250_000_000_000_000);
			assert_eq!(Assets::free_balance(PDOT, &BOB), 999_761_335_318_602_905);
			assert_eq!(Assets::free_balance(PBTC, &BOB), 1_000_000_005_000_000_000);
		});
}

#[test]
fn initialize_added_liquidity_pools_genesis_work() {
	ExtBuilder::default()
		.initialize_enabled_trading_pairs()
		.initialize_added_liquidity_pools(ALICE)
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			assert_eq!(Dex::get_liquidity(PUSD, PDOT), (1000000, 2000000));
			assert_eq!(Assets::free_balance(PUSD, &Dex::account_id()), 2000000);
			assert_eq!(Assets::free_balance(PDOT, &Dex::account_id()), 3000000);
			assert_eq!(
				Assets::free_balance(PUSDDOTPair::get().dex_share_currency_id(), &ALICE),
				2000000
			);
		});
}
*/
