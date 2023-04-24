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

//! Integration tests for evm gas costs
#![cfg(test)]

use crate::{
	tests::{bob, ExtBuilder},
	Ethereum, FeeControl, TxFeePot, XrpCurrency,
};

use super::TxBuilder;
use frame_support::{assert_ok, dispatch::RawOrigin, traits::fungible::Inspect};
use seed_primitives::Balance;

mod fee_control {
	use super::*;

	// If XRP is worth 1 dollar, then the user needs to pay 0.1 XRP to execute a EVM tx.
	// 0.1 XRP is equal to 100_000 units
	// 0.1 XRP * 1 XRP Price =  0.1 dollar
	#[test]
	fn set_xrp_price() {
		ExtBuilder::default().build().execute_with(|| {
			// Setup
			let ok = FeeControl::set_xrp_price(RawOrigin::Root.into(), Balance::from(1_000_000u32));
			assert_ok!(ok);
			let old_balance = XrpCurrency::balance(&bob());
			let old_pot = TxFeePot::era_tx_fees();

			// Call
			let (origin, tx) = TxBuilder::default().build();
			assert_ok!(Ethereum::transact(origin, tx));

			// Check
			const DIFFERENCE: u128 = 100_000u128;
			let actual_balance = XrpCurrency::balance(&bob());
			let expected_balance = old_balance - DIFFERENCE;
			assert_eq!(actual_balance, expected_balance);

			let actual_pot = TxFeePot::era_tx_fees();
			let expected_pot = old_pot + DIFFERENCE;
			assert_eq!(actual_pot, expected_pot);
		});
	}

	// If XRP is worth 0.001 dollars, then the user needs to pay 100 XRP to execute a EVM tx.
	// 100 XRP is equal to 100_000_000 units
	// 100 XRP * 0.001 XRP Price =  0.1 dollar
	#[test]
	fn xrp_price_lower_limit() {
		ExtBuilder::default().build().execute_with(|| {
			// Setup
			let ok = FeeControl::set_xrp_price(RawOrigin::Root.into(), Balance::from(1_000u32));
			assert_ok!(ok);
			let old_balance = XrpCurrency::balance(&bob());
			let old_pot = TxFeePot::era_tx_fees();

			// Call
			let (origin, tx) = TxBuilder::default().build();
			assert_ok!(Ethereum::transact(origin, tx));

			// Check
			const DIFFERENCE: u128 = 100_000_000u128;
			let actual_balance = XrpCurrency::balance(&bob());
			let expected_balance = old_balance - DIFFERENCE;
			assert_eq!(actual_balance, expected_balance);

			let actual_pot = TxFeePot::era_tx_fees();
			let expected_pot = old_pot + DIFFERENCE;
			assert_eq!(actual_pot, expected_pot);
		});
	}

	// If XRP is worth 10 dollars, then the user needs to pay 0.01 XRP to execute a EVM tx.
	// 0.01 XRP is equal to 10_000 units
	// 0.01 XRP * 10 XRP Price =  0.1 dollar
	#[test]
	fn xrp_price_upper_limit() {
		ExtBuilder::default().build().execute_with(|| {
			// Setup
			let ok =
				FeeControl::set_xrp_price(RawOrigin::Root.into(), Balance::from(10_000_000u32));
			assert_ok!(ok);
			let old_balance = XrpCurrency::balance(&bob());
			let old_pot = TxFeePot::era_tx_fees();

			// Call
			let (origin, tx) = TxBuilder::default().build();
			assert_ok!(Ethereum::transact(origin, tx));

			// Check
			const DIFFERENCE: u128 = 10_000u128;
			let actual_balance = XrpCurrency::balance(&bob());
			let expected_balance = old_balance - DIFFERENCE;
			assert_eq!(actual_balance, expected_balance);

			let actual_pot = TxFeePot::era_tx_fees();
			let expected_pot = old_pot + DIFFERENCE;
			assert_eq!(actual_pot, expected_pot);
		});
	}
}
