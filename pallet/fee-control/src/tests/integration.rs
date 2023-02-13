/* Copyright 2021-2022 Centrality Investments Limited
 *
 * Licensed under the LGPL, Version 3.0 (the "License");
 * you may not use this file except in compliance with the License.
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * You may obtain a copy of the License at the root of this project source code,
 * or at:
 *     https://centrality.ai/licenses/gplv3.txt
 *     https://centrality.ai/licenses/lgplv3.txt
 */
use super::mock_integration::*;
use crate::{Config, SettingsAndMultipliers};
use frame_support::{
	assert_ok,
	weights::{DispatchClass, DispatchInfo, Pays, PostDispatchInfo, Weight},
};
use pallet_transaction_payment::ChargeTransactionPayment;
use sp_runtime::{traits::SignedExtension, Perbill};

/// create a transaction info struct from weight. Handy to avoid building the whole struct.
pub fn info_from_weight(w: Weight) -> DispatchInfo {
	// pays_fee: Pays::Yes -- class: DispatchClass::Normal
	DispatchInfo { weight: w, ..Default::default() }
}

fn default_post_info() -> PostDispatchInfo {
	PostDispatchInfo { actual_weight: None, pays_fee: Default::default() }
}

const CALL: &<Test as frame_system::Config>::Call =
	&Call::Balances(pallet_balances::Call::transfer_keep_alive { dest: BOB, value: 1_000_000u128 });

// !!!
// EVM Tests are located inside runtime/src/tests/evm_tests.rs
// !!!

#[test]
fn tx_fee_test_with_compute_fee() {
	ExtBuilder::build().execute_with(|| {
		// Setup
		let ok = SettingsBuilder::new()
			.tx_fee(Balance::from(1_000_000u32)) // This is 1€
			.done();
		assert_ok!(ok);

		let xrp_price = Balance::from(1_000_000u32);
		assert_ok!(FeeControl::set_xrp_price(root(), xrp_price));

		let total_weight = SettingsAndMultipliers::<Test>::get().input_tx_weight;
		let tx_weight = total_weight.saturating_sub(BaseWeight::get());

		let dispatch_info =
			DispatchInfo { weight: tx_weight, class: DispatchClass::Normal, pays_fee: Pays::Yes };

		// Call and Check
		let expected_fee = <Test as Config>::OneXRP::get();
		assert_eq!(TransactionPayment::compute_fee(0, &dispatch_info, 0), expected_fee);
	})
}

#[test]
fn tx_fee_test_with_dispatch_call() {
	ExtBuilder::build().execute_with(|| {
		// Setup
		let ok = SettingsBuilder::new()
			.tx_fee(Balance::from(1_000_000u32)) // This is 1€
			.done();
		assert_ok!(ok);

		let xrp_price = Balance::from(1_000_000u32);
		assert_ok!(FeeControl::set_xrp_price(root(), xrp_price));

		let total_weight = SettingsAndMultipliers::<Test>::get().input_tx_weight;
		let tx_weight = total_weight.saturating_sub(BaseWeight::get());

		let old_alice_balance = Balances::free_balance(&ALICE);
		let old_treasury_balance = Balances::free_balance(&TREASURY);
		let len = 0;

		// Call
		let pre = ChargeTransactionPayment::<Test>::from(0)
			.pre_dispatch(&ALICE, CALL, &info_from_weight(tx_weight), len)
			.unwrap();

		assert_ok!(ChargeTransactionPayment::<Test>::post_dispatch(
			Some(pre),
			&info_from_weight(tx_weight),
			&default_post_info(),
			len,
			&Ok(())
		));

		// Check
		let new_alice_balance = Balances::free_balance(&ALICE);
		let new_treasury_balance = Balances::free_balance(&TREASURY);

		let one_xrp = <Test as Config>::OneXRP::get();
		assert_eq!(old_treasury_balance + one_xrp, new_treasury_balance);
		assert_eq!(old_alice_balance - one_xrp, new_alice_balance);
	})
}

#[test]
fn len_fee_test_with_compute_fee() {
	ExtBuilder::build().execute_with(|| {
		// Setup
		let ok = SettingsBuilder::new()
			.len_fee(Balance::from(1_000_000u32)) // This is 1€
			.tx_fee(Balance::from(1_000_000u32)) // This is 1€
			.done();
		assert_ok!(ok);

		let xrp_price = Balance::from(1_000_000u32);
		assert_ok!(FeeControl::set_xrp_price(root(), xrp_price));

		// This disables weight multiplier
		let ok = SettingsBuilder::new()
			.weight_multiplier(Perbill::zero()) // This is 0€
			.done();
		assert_ok!(ok);

		let dispatch_info =
			DispatchInfo { weight: 0, class: DispatchClass::Normal, pays_fee: Pays::Yes };

		// Call and Check
		let len = 25u32;
		let expected_fee = len as u128 * <Test as Config>::OneXRP::get();
		assert_eq!(TransactionPayment::compute_fee(len, &dispatch_info, 0), expected_fee);
	})
}

#[test]
fn len_fee_test_with_dispatch_call() {
	ExtBuilder::build().execute_with(|| {
		// Setup
		let ok = SettingsBuilder::new()
			.len_fee(Balance::from(1_000_000u32)) // This is 1€
			.done();
		assert_ok!(ok);

		let xrp_price = Balance::from(1_000_000u32);
		assert_ok!(FeeControl::set_xrp_price(root(), xrp_price));

		// This disables weight multiplier
		let ok = SettingsBuilder::new()
			.weight_multiplier(Perbill::zero()) //  This is 0€
			.done();
		assert_ok!(ok);

		let old_alice_balance = Balances::free_balance(&ALICE);
		let old_treasury_balance = Balances::free_balance(&TREASURY);
		let len = 25;

		// Call
		let pre = ChargeTransactionPayment::<Test>::from(0)
			.pre_dispatch(&ALICE, CALL, &info_from_weight(0), len)
			.unwrap();

		assert_ok!(ChargeTransactionPayment::<Test>::post_dispatch(
			Some(pre),
			&info_from_weight(0),
			&default_post_info(),
			len,
			&Ok(())
		));

		// Check
		let new_alice_balance = Balances::free_balance(&ALICE);
		let new_treasury_balance = Balances::free_balance(&TREASURY);

		let difference = len as u128 * <Test as Config>::OneXRP::get();
		assert_eq!(old_treasury_balance + difference, new_treasury_balance);
		assert_eq!(old_alice_balance - difference, new_alice_balance);
	})
}
