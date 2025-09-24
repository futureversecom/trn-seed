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

//! Integration tests for evm config
#![cfg(test)]

use super::{TxBuilder, BASE_TX_GAS_COST, MAX_PRIORITY_FEE_PER_GAS, MINIMUM_XRP_TX_COST};
use crate::{
	constants::ONE_XRP,
	impls::scale_wei_to_6dp,
	tests::{alice, bob, charlie, ExtBuilder},
	Assets, AssetsExt, Dex, Ethereum, FeeControl, FeeProxy, Futurepass, Runtime, RuntimeOrigin,
	System, TxFeePot, XrpCurrency, EVM,
};
use ethabi::Token;

use frame_support::{
	assert_ok,
	dispatch::{GetDispatchInfo, RawOrigin},
	traits::{
		fungible::Inspect,
		fungibles::Inspect as Inspects,
		tokens::{Fortitude, Preservation},
	},
};
use frame_system::RawOrigin::Root;
use hex_literal::hex;

use crate::{constants::XRP_ASSET_ID, impls::scale_6dp_to_wei};
use pallet_transaction_payment::ChargeTransactionPayment;
use precompile_utils::{constants::ERC20_PRECOMPILE_ADDRESS_PREFIX, ErcIdConversion};
use seed_client::chain_spec::get_account_id_from_seed;
use seed_primitives::{AccountId, AssetId, Balance};
use sp_core::{ecdsa, H160, U256};
use sp_runtime::{traits::SignedExtension, DispatchError::BadOrigin};

#[test]
fn evm_base_transaction_cost_uses_xrp() {
	ExtBuilder::default().build().execute_with(|| {
		let base_tx_gas_cost_scaled =
			scale_wei_to_6dp(BASE_TX_GAS_COST * FeeControl::base_fee_per_gas().as_u128());
		let charlie_initial_balance = XrpCurrency::balance(&charlie());
		assert_eq!(base_tx_gas_cost_scaled, MINIMUM_XRP_TX_COST); // ensure minimum tx price is 0.315 XRP

		let (origin, tx) = TxBuilder::default().origin(charlie()).build();
		// gas only in xrp
		assert_ok!(Ethereum::transact(origin, tx));

		let charlie_new_balance = XrpCurrency::balance(&charlie());
		assert!(charlie_new_balance < charlie_initial_balance);
		let empty_call_gas_cost = charlie_initial_balance - charlie_new_balance;
		assert_eq!(empty_call_gas_cost, base_tx_gas_cost_scaled); // 0.315 XRP is lowest cost of TX
	});
}

#[test]
fn evm_transfer_transaction_uses_xrp() {
	ExtBuilder::default().build().execute_with(|| {
		let charlie_initial_balance = XrpCurrency::balance(&charlie());

		// transfer in xrp
		let value = 5 * 10_u128.pow(18_u32);
		let (origin, tx) = TxBuilder::default().origin(charlie()).value(U256::from(value)).build();
		assert_ok!(Ethereum::transact(origin, tx));

		let expected_total_cost_of_tx =
			scale_wei_to_6dp(BASE_TX_GAS_COST * FeeControl::base_fee_per_gas().as_u128() + value);
		let charlie_balance_change = charlie_initial_balance - XrpCurrency::balance(&charlie());
		assert_eq!(charlie_balance_change, expected_total_cost_of_tx);
		assert_eq!(charlie_initial_balance + 5 * ONE_XRP, XrpCurrency::balance(&bob()),);
	});
}

#[test]
fn evm_call_success_by_any_address() {
	ExtBuilder::default().build().execute_with(|| {
		let result = EVM::call(
			RuntimeOrigin::signed(charlie()),
			charlie().into(),
			bob().into(),
			Vec::new(),
			U256::default(),
			1000000,
			U256::from(1_500_000_000_000_000u64),
			None,
			None,
			Vec::new(),
		);
		result.expect("EVM can be called");
	});
}

#[test]
fn evm_call_fail_by_origin_mismatch() {
	ExtBuilder::default().build().execute_with(|| {
		let result = EVM::call(
			RuntimeOrigin::signed(alice()),
			charlie().into(),
			bob().into(),
			Vec::new(),
			U256::default(),
			1000000,
			U256::from(1_500_000_000_000_000u64),
			None,
			None,
			Vec::new(),
		);
		assert!(result.is_err());
		assert_eq!(result.unwrap_err().error, BadOrigin);
	});
}

#[test]
/// This test tests whether we can call evm.call using FeeProxy.call_with_fee_preferences()
fn fee_proxy_call_evm_with_fee_preferences() {
	ExtBuilder::default().build().execute_with(|| {
		let new_account = get_account_id_from_seed::<ecdsa::Public>("Im Broke");

		// The next minted asset id
		let payment_asset = AssetsExt::next_asset_uuid().unwrap();

		// Lets create an asset
		assert_ok!(AssetsExt::create_asset(
			RawOrigin::Signed(alice()).into(),
			b"Test".to_vec(),
			b"Test".to_vec(),
			6,
			None,
			None
		));

		// Check Bob's initial balance is 0
		assert_eq!(AssetsExt::balance(payment_asset, &bob()), 0);

		// Mint these assets into Alice and new_account
		assert_ok!(Assets::mint(
			RawOrigin::Signed(alice()).into(),
			payment_asset,
			alice(),
			10_000_000_000_000_000
		));
		assert_ok!(Assets::mint(
			RawOrigin::Signed(alice()).into(),
			payment_asset,
			new_account,
			10_000_000_000_000_000
		));

		// Add liquidity to the dex, this will allow for exchange internally when the call is made
		assert_ok!(Dex::add_liquidity(
			RawOrigin::Signed(alice()).into(),
			XRP_ASSET_ID,
			payment_asset,
			1_000_000_000_000,
			1_000_000_000_000,
			1,
			1,
			None,
			None,
		));

		let transfer_amount: Balance = 12345;
		let target: H160 = <Runtime as ErcIdConversion<AssetId>>::runtime_id_to_evm_id(
			payment_asset,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.into();

		// Setup input for an erc20 transfer to Bob
		// transfer(address,uint256)
		let mut input: Vec<u8> = [0xa9, 0x05, 0x9c, 0xbb].to_vec();
		input.append(&mut ethabi::encode(&[
			Token::Address(bob().into()),
			Token::Uint(transfer_amount.into()),
		]));
		// Setup inner EVM.call call
		let (gas_limit, max_fee_per_gas) = (50_000_u128, 15_000_000_000_000_u128);
		let inner_call = crate::RuntimeCall::EVM(pallet_evm::Call::call {
			source: new_account.into(),
			target,
			input,
			value: U256::default(),
			gas_limit: gas_limit as u64,
			max_fee_per_gas: U256::from(max_fee_per_gas),
			max_priority_fee_per_gas: None,
			nonce: None,
			access_list: vec![],
		});

		let max_payment = scale_wei_to_6dp(gas_limit * max_fee_per_gas * 2);
		let call =
			crate::RuntimeCall::FeeProxy(pallet_fee_proxy::Call::call_with_fee_preferences {
				payment_asset,
				max_payment,
				call: Box::new(inner_call.clone()),
			});

		let dispatch_info = call.get_dispatch_info();

		// Call pre_dispatch, which hits OnChargeTransaction and exchanges the fee
		assert_ok!(<ChargeTransactionPayment<Runtime> as SignedExtension>::pre_dispatch(
			ChargeTransactionPayment::from(0),
			&new_account,
			&call,
			&dispatch_info,
			1,
		));

		// call.dispatch();
		assert_ok!(FeeProxy::call_with_fee_preferences(
			RawOrigin::Signed(new_account).into(),
			payment_asset,
			max_payment,
			Box::new(inner_call)
		));

		// Check Bob has been transferred the correct amount
		assert_eq!(AssetsExt::balance(payment_asset, &bob()), transfer_amount);
	});
}

#[test]
/// Test whether fee proxy can proxy futurepass proxy_extrinsic and validate futurepass pays fee in
/// tokens
fn call_with_fee_preferences_futurepass_proxy_extrinsic() {
	ExtBuilder::default().build().execute_with(|| {
		let new_account = get_account_id_from_seed::<ecdsa::Public>("Im Broke");

		// next minted asset id
		let payment_asset = AssetsExt::next_asset_uuid().unwrap();

		// create an asset
		assert_ok!(AssetsExt::create_asset(
			RawOrigin::Signed(alice()).into(),
			b"Test".to_vec(),
			b"Test".to_vec(),
			6,
			None,
			None
		));

		// mint these assets into alice and new_account
		assert_ok!(Assets::mint(
			RawOrigin::Signed(alice()).into(),
			payment_asset,
			alice(),
			10_000_000_000_000_000
		));

		// add liquidity to the dex, this will allow for exchange internally when the call is made
		assert_ok!(Dex::add_liquidity(
			RawOrigin::Signed(alice()).into(),
			2,
			payment_asset,
			1_000_000_000_000,
			10_000,
			1,
			1,
			None,
			None,
		));

		assert_ok!(Futurepass::create(RuntimeOrigin::signed(alice()), new_account));
		let futurepass = pallet_futurepass::Holders::<Runtime>::get(new_account).unwrap();

		// mint payment assets into futurepass - for futurepass to pay for proxy_extrinsic
		assert_ok!(Assets::mint(
			RawOrigin::Signed(alice()).into(),
			payment_asset,
			futurepass,
			10_000_000_000_000_000
		));

		// get balances of new account and futurepass - for comparison later
		let caller_xrp_balance_before = XrpCurrency::balance(&new_account);
		let caller_token_balance_before = AssetsExt::balance(payment_asset, &new_account);
		let futurepass_xrp_balance_before = XrpCurrency::balance(&futurepass);
		let futurepass_token_balance_before = AssetsExt::balance(payment_asset, &futurepass);

		let inner_call = crate::RuntimeCall::System(frame_system::Call::remark {
			remark: b"Mischief Managed".to_vec(),
		});
		let proxy_extrinsic_call =
			crate::RuntimeCall::Futurepass(pallet_futurepass::Call::proxy_extrinsic {
				futurepass,
				call: Box::new(inner_call),
			});

		let max_payment: Balance = 10_000_000_000_000_000;
		let fee_proxy_call =
			crate::RuntimeCall::FeeProxy(pallet_fee_proxy::Call::call_with_fee_preferences {
				payment_asset,
				max_payment,
				call: Box::new(proxy_extrinsic_call.clone()),
			});

		// call pre_dispatch, which hits OnChargeTransaction and exchanges the fee
		let dispatch_info = fee_proxy_call.get_dispatch_info();
		assert_ok!(<ChargeTransactionPayment<Runtime> as SignedExtension>::pre_dispatch(
			ChargeTransactionPayment::from(0),
			&new_account,
			&fee_proxy_call,
			&dispatch_info,
			1,
		));

		// call.dispatch();
		assert_ok!(FeeProxy::call_with_fee_preferences(
			RawOrigin::Signed(new_account).into(),
			payment_asset,
			max_payment,
			Box::new(proxy_extrinsic_call)
		));

		// get balances of new account and futurepass after feeproxy calls - for comparison
		let caller_xrp_balance_after = XrpCurrency::balance(&new_account);
		let caller_token_balance_after = AssetsExt::balance(payment_asset, &new_account);
		let futurepass_xrp_balance_after =
			XrpCurrency::reducible_balance(&futurepass, Preservation::Preserve, Fortitude::Polite);
		let futurepass_token_balance_after = AssetsExt::balance(payment_asset, &futurepass);

		// vaidate futurepass should only have paid in tokens
		assert_eq!(caller_xrp_balance_before, caller_xrp_balance_after);
		assert_eq!(caller_token_balance_before, caller_token_balance_after);
		assert_eq!(futurepass_xrp_balance_before, futurepass_xrp_balance_after);
		assert_ne!(futurepass_token_balance_before, futurepass_token_balance_after);
		assert!(futurepass_token_balance_before > futurepass_token_balance_after);
	});
}

#[test]
fn transactions_cost_goes_to_tx_pot() {
	ExtBuilder::default().build().execute_with(|| {
		// Setup
		let old_pot = pallet_tx_fee_pot::EraTxFees::<Runtime>::get();

		// Call
		let (origin, tx) = TxBuilder::default().build();
		assert_ok!(Ethereum::transact(origin, tx));

		// Check
		let expected_change = 157_500u128;
		assert_eq!(pallet_tx_fee_pot::EraTxFees::<Runtime>::get(), old_pot + expected_change);
	})
}

#[test]
fn zero_evm_base_fee_means_free_transactions() {
	ExtBuilder::default().build().execute_with(|| {
		// Setup
		assert_ok!(FeeControl::set_evm_base_fee(Root.into(), U256::from(0)));
		let old_balance = XrpCurrency::balance(&bob());

		// Call
		let (origin, tx) = TxBuilder::default().origin(bob()).build();
		assert_ok!(Ethereum::transact(origin, tx));

		// Check
		let new_balance = XrpCurrency::balance(&bob());
		let expected_change = 0u128;
		assert_eq!(new_balance, old_balance - expected_change);
	})
}

#[test]
fn evm_base_fee_changes_transaction_fee() {
	ExtBuilder::default().build().execute_with(|| {
		// Test is quite simple:
		// First we set base fee to 1X
		// Then we set base fee to 2X
		// At the end we test that the new balance is equal to old - 3X

		// Setup
		let base_fee = U256::from(10_000_000_000_000u128);
		assert_ok!(FeeControl::set_evm_base_fee(Root.into(), base_fee));
		let original_balance = XrpCurrency::balance(&bob());
		let (origin, tx) = TxBuilder::default().origin(bob()).build();
		assert_ok!(Ethereum::transact(origin, tx));

		let second_balance = XrpCurrency::balance(&bob());
		let original_change = original_balance - second_balance;

		// Call
		assert_ok!(FeeControl::set_evm_base_fee(Root.into(), base_fee * 2));
		let (origin, tx) = TxBuilder::default().origin(bob()).build();
		assert_ok!(Ethereum::transact(origin, tx));

		// Check
		let third_balance = XrpCurrency::balance(&bob());
		let new_change = original_change * 2;

		assert_eq!(third_balance, second_balance - new_change);
		assert_eq!(new_change, original_change * 2);
		assert_eq!(third_balance, original_balance - original_change - new_change);
		assert!(new_change > original_change);
	})
}

#[test]
fn evm_extra_gas_refunded_and_miner_paid() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		let base_fee: u128 = FeeControl::base_fee_per_gas().as_u128();
		let fee_pot_accout = TxFeePot::account_id();
		let miner_account = AccountId::from(hex!("0000000000000000000000000000000000000000")); // miner is 0x0000000000000000000000000000000000000000

		let bob_balance_before = XrpCurrency::balance(&bob());
		let fee_pot_balance_before = XrpCurrency::balance(&fee_pot_accout);
		let miner_balance_before = XrpCurrency::balance(&miner_account);
		let (origin, tx) = TxBuilder::ethers_default_gas().origin(bob()).build();
		assert_ok!(Ethereum::transact(origin, tx));
		let bob_balance_after = XrpCurrency::balance(&bob());
		let fee_pot_balance_after = XrpCurrency::balance(&fee_pot_accout);
		let miner_balance_after = XrpCurrency::balance(&miner_account);

		// calculations
		let initial_withdraw_fee = BASE_TX_GAS_COST * (2 * base_fee + MAX_PRIORITY_FEE_PER_GAS);
		let actual_fee = BASE_TX_GAS_COST * (base_fee + MAX_PRIORITY_FEE_PER_GAS);
		let burned_fee = BASE_TX_GAS_COST * base_fee;
		let refund_fee =
			scale_wei_to_6dp(scale_6dp_to_wei(scale_wei_to_6dp(initial_withdraw_fee)) - actual_fee);
		let priority_fee =
			scale_wei_to_6dp(initial_withdraw_fee) - refund_fee - scale_wei_to_6dp(burned_fee);

		// Check bob is only charged the actual fee
		assert_eq!(
			bob_balance_before - bob_balance_after,
			scale_wei_to_6dp(initial_withdraw_fee) - refund_fee
		);
		// check the fee pot received the burned_fee
		assert_eq!(fee_pot_balance_after - fee_pot_balance_before, scale_wei_to_6dp(burned_fee));
		// check miner received the priority fee
		assert_eq!(miner_balance_after - miner_balance_before, priority_fee);

		println!("events: {:?}", System::events());
		// check events
		// Note that after https://github.com/paritytech/frontier/pull/857, initial withdrawal has become
		// based on (base fee + priority fee) rather than max fee per gas. Hence no refunds in this
		// test scenario Initial fee withdraw
		System::assert_has_event(
			pallet_assets_ext::Event::<Runtime>::InternalWithdraw {
				asset_id: XRP_ASSET_ID,
				who: bob(),
				amount: scale_wei_to_6dp(actual_fee),
			}
			.into(),
		);
		// burned fee to tx fee pot
		System::assert_has_event(
			pallet_assets_ext::Event::<Runtime>::InternalDeposit {
				asset_id: XRP_ASSET_ID,
				who: fee_pot_accout,
				amount: scale_wei_to_6dp(burned_fee),
			}
			.into(),
		);
		// priority fee to the miner
		System::assert_has_event(
			pallet_assets_ext::Event::<Runtime>::InternalDeposit {
				asset_id: XRP_ASSET_ID,
				who: miner_account,
				amount: priority_fee,
			}
			.into(),
		);
	})
}

/// Tests for batch_all EVM gas cost handling in fee proxy
mod batch_all_evm_support {
	use super::*;
	use crate::{RuntimeCall, RuntimeEvent};

	#[test]
	fn batch_all_with_multiple_evm_calls_works() {
		ExtBuilder::default().build().execute_with(|| {
			System::set_block_number(1);
			let new_account = get_account_id_from_seed::<ecdsa::Public>("BatchTest");

			// Fund new_account with XRP for gas fees
			assert_ok!(Assets::mint(
				RawOrigin::Signed(alice()).into(),
				XRP_ASSET_ID,
				new_account,
				1_000_000_000_000
			));

			// The next minted asset id
			let payment_asset = AssetsExt::next_asset_uuid().unwrap();

			// Create an asset
			assert_ok!(AssetsExt::create_asset(
				RawOrigin::Signed(alice()).into(),
				b"BatchTest".to_vec(),
				b"BATCH".to_vec(),
				6,
				None,
				None
			));

			// Mint assets into Alice and new_account
			assert_ok!(Assets::mint(
				RawOrigin::Signed(alice()).into(),
				payment_asset,
				alice(),
				10_000_000_000_000_000
			));
			assert_ok!(Assets::mint(
				RawOrigin::Signed(alice()).into(),
				payment_asset,
				new_account,
				10_000_000_000_000_000
			));

			// Add liquidity to the dex for asset swapping
			assert_ok!(Dex::add_liquidity(
				RawOrigin::Signed(alice()).into(),
				XRP_ASSET_ID,
				payment_asset,
				1_000_000_000_000,
				1_000_000_000_000,
				1,
				1,
				None,
				None,
			));

			// Create multiple EVM calls for the batch with realistic gas prices
			let evm_call_1 = RuntimeCall::EVM(pallet_evm::Call::call {
				source: new_account.into(),
				target: H160::from_low_u64_be(1),
				input: vec![],
				value: U256::zero(),
				gas_limit: 21000,
				max_fee_per_gas: U256::from(15_000_000_000_000u64), // 15 Twei
				max_priority_fee_per_gas: Some(U256::from(1_500_000_000u64)), // 1.5 gwei
				nonce: None,
				access_list: vec![],
			});

			let evm_call_2 = RuntimeCall::EVM(pallet_evm::Call::call {
				source: new_account.into(),
				target: H160::from_low_u64_be(2),
				input: vec![],
				value: U256::zero(),
				gas_limit: 30000,
				max_fee_per_gas: U256::from(15_000_000_000_000u64), // 15 Twei
				max_priority_fee_per_gas: Some(U256::from(1_500_000_000u64)), // 1.5 gwei
				nonce: None,
				access_list: vec![],
			});

			let evm_call_3 = RuntimeCall::EVM(pallet_evm::Call::call {
				source: new_account.into(),
				target: H160::from_low_u64_be(3),
				input: vec![],
				value: U256::zero(),
				gas_limit: 25000,
				max_fee_per_gas: U256::from(15_000_000_000_000u64), // 15 Twei
				max_priority_fee_per_gas: Some(U256::from(1_500_000_000u64)), // 1.5 gwei
				nonce: None,
				access_list: vec![],
			});

			// Create batch_all call containing multiple EVM calls
			let batch_call = RuntimeCall::Utility(pallet_utility::Call::batch_all {
				calls: vec![evm_call_1, evm_call_2, evm_call_3],
			});

			let max_payment = scale_wei_to_6dp(100_000 * 15_000_000_000_000u128 * 2); // Enough to cover all EVM calls

			// reset events
			System::reset_events();

			// Test that the call succeeds and gas costs are calculated for all EVM calls in the batch
			assert_ok!(FeeProxy::call_with_fee_preferences(
				RawOrigin::Signed(new_account).into(),
				payment_asset,
				max_payment,
				Box::new(batch_call)
			));

			// Check that all 3 EVM calls were processed (1 failed, 2 executed successfully)
			System::assert_has_event(
				pallet_evm::Event::<Runtime>::ExecutedFailed {
					// Failed coz 0x1 is ECRecover and does not accept empty input
					address: H160::from_low_u64_be(1),
				}
				.into(),
			);
			System::assert_has_event(
				pallet_evm::Event::<Runtime>::Executed { address: H160::from_low_u64_be(2) }.into(),
			);
			System::assert_has_event(
				pallet_evm::Event::<Runtime>::Executed { address: H160::from_low_u64_be(3) }.into(),
			);

			// Check that all batch items completed
			let events = System::events();
			let item_completed_count = events
				.iter()
				.filter(|e| {
					matches!(e.event, RuntimeEvent::Utility(pallet_utility::Event::ItemCompleted))
				})
				.count();
			assert_eq!(
				item_completed_count, 3,
				"Should have 3 ItemCompleted events for 3 EVM calls"
			);

			// Check that the batch completed successfully
			System::assert_has_event(pallet_utility::Event::BatchCompleted.into());

			// Check that the fee proxy event was emitted
			System::assert_has_event(
				pallet_fee_proxy::Event::<Runtime>::CallWithFeePreferences {
					who: new_account,
					payment_asset,
					max_payment,
				}
				.into(),
			);
		});
	}

	#[test]
	fn batch_all_with_evm_and_non_evm_calls_works() {
		ExtBuilder::default().build().execute_with(|| {
			System::set_block_number(1);
			let new_account = get_account_id_from_seed::<ecdsa::Public>("MixedBatchTest");

			// Fund new_account with XRP for gas fees
			assert_ok!(Assets::mint(
				RawOrigin::Signed(alice()).into(),
				XRP_ASSET_ID,
				new_account,
				1_000_000_000_000
			));

			// The next minted asset id
			let payment_asset = AssetsExt::next_asset_uuid().unwrap();

			// Create an asset
			assert_ok!(AssetsExt::create_asset(
				RawOrigin::Signed(alice()).into(),
				b"MixedTest".to_vec(),
				b"MIXED".to_vec(),
				6,
				None,
				None
			));

			// Mint assets into Alice and new_account
			assert_ok!(Assets::mint(
				RawOrigin::Signed(alice()).into(),
				payment_asset,
				alice(),
				10_000_000_000_000_000
			));
			assert_ok!(Assets::mint(
				RawOrigin::Signed(alice()).into(),
				payment_asset,
				new_account,
				10_000_000_000_000_000
			));

			// Add liquidity to the dex for asset swapping
			assert_ok!(Dex::add_liquidity(
				RawOrigin::Signed(alice()).into(),
				XRP_ASSET_ID,
				payment_asset,
				1_000_000_000_000,
				1_000_000_000_000,
				1,
				1,
				None,
				None,
			));

			// Create EVM call for the batch
			let evm_call = RuntimeCall::EVM(pallet_evm::Call::call {
				source: new_account.into(),
				target: H160::from_low_u64_be(1),
				input: vec![],
				value: U256::zero(),
				gas_limit: 25000,
				max_fee_per_gas: U256::from(15_000_000_000_000u64), // 15 Twei
				max_priority_fee_per_gas: Some(U256::from(1_500_000_000u64)), // 1.5 gwei
				nonce: None,
				access_list: vec![],
			});

			// Create non-EVM calls for the batch
			let remark_call_1 = RuntimeCall::System(frame_system::Call::remark {
				remark: b"Mixed batch test 1".to_vec(),
			});

			let remark_call_2 = RuntimeCall::System(frame_system::Call::remark_with_event {
				remark: b"Mixed batch test 2".to_vec(),
			});

			// Create batch_all call containing both EVM and non-EVM calls
			let batch_call = RuntimeCall::Utility(pallet_utility::Call::batch_all {
				calls: vec![remark_call_1, evm_call, remark_call_2],
			});

			let max_payment = scale_wei_to_6dp(50_000 * 15_000_000_000_000u128 * 2); // Enough to cover EVM call

			// reset events
			System::reset_events();

			// Test that the call succeeds and gas costs are calculated only for EVM calls
			assert_ok!(FeeProxy::call_with_fee_preferences(
				RawOrigin::Signed(new_account).into(),
				payment_asset,
				max_payment,
				Box::new(batch_call)
			));

			// Check that the EVM call was processed (executed or failed)
			let events = System::events();
			let has_evm_event = events.iter().any(|e| {
				matches!(
					e.event,
					RuntimeEvent::EVM(pallet_evm::Event::Executed { .. })
						| RuntimeEvent::EVM(pallet_evm::Event::ExecutedFailed { .. })
				)
			});
			assert!(has_evm_event, "Should have at least one EVM event");

			// Check that remark_with_event call emitted event (only remark_call_2)
			System::assert_has_event(
				frame_system::Event::<Runtime>::Remarked {
					sender: new_account,
					hash: sp_core::blake2_256(b"Mixed batch test 2").into(),
				}
				.into(),
			);

			// Check that all batch items completed (2 remarks + 1 EVM call = 3 items)
			let item_completed_count = events
				.iter()
				.filter(|e| {
					matches!(e.event, RuntimeEvent::Utility(pallet_utility::Event::ItemCompleted))
				})
				.count();
			assert_eq!(
				item_completed_count, 3,
				"Should have 3 ItemCompleted events for mixed batch"
			);

			// Check that the batch completed successfully
			System::assert_has_event(pallet_utility::Event::BatchCompleted.into());

			// Check that the fee proxy event was emitted
			System::assert_has_event(
				pallet_fee_proxy::Event::<Runtime>::CallWithFeePreferences {
					who: new_account,
					payment_asset,
					max_payment,
				}
				.into(),
			);
		});
	}

	#[test]
	fn proxy_extrinsic_with_batch_evm_and_non_evm_calls_works() {
		ExtBuilder::default().build().execute_with(|| {
			System::set_block_number(1);
			let new_account = get_account_id_from_seed::<ecdsa::Public>("ProxyBatchTest");

			// Fund new_account with XRP for gas fees
			assert_ok!(Assets::mint(
				RawOrigin::Signed(alice()).into(),
				XRP_ASSET_ID,
				new_account,
				1_000_000_000_000
			));

			// The next minted asset id
			let payment_asset = AssetsExt::next_asset_uuid().unwrap();

			// Create an asset
			assert_ok!(AssetsExt::create_asset(
				RawOrigin::Signed(alice()).into(),
				b"ProxyTest".to_vec(),
				b"PROXY".to_vec(),
				6,
				None,
				None
			));

			// Mint assets into Alice and new_account
			assert_ok!(Assets::mint(
				RawOrigin::Signed(alice()).into(),
				payment_asset,
				alice(),
				10_000_000_000_000_000
			));
			assert_ok!(Assets::mint(
				RawOrigin::Signed(alice()).into(),
				payment_asset,
				new_account,
				10_000_000_000_000_000
			));

			// Add liquidity to the dex for asset swapping
			assert_ok!(Dex::add_liquidity(
				RawOrigin::Signed(alice()).into(),
				XRP_ASSET_ID,
				payment_asset,
				1_000_000_000_000,
				1_000_000_000_000,
				1,
				1,
				None,
				None,
			));

			// Create futurepass
			assert_ok!(Futurepass::create(RuntimeOrigin::signed(alice()), new_account));
			let futurepass = pallet_futurepass::Holders::<Runtime>::get(new_account).unwrap();

			// Mint payment assets into futurepass
			assert_ok!(Assets::mint(
				RawOrigin::Signed(alice()).into(),
				payment_asset,
				futurepass,
				10_000_000_000_000_000
			));

			// Fund futurepass with XRP for EVM gas fees
			assert_ok!(Assets::mint(
				RawOrigin::Signed(alice()).into(),
				XRP_ASSET_ID,
				futurepass,
				1_000_000_000_000
			));

			// Create EVM calls for the batch
			let evm_call_1 = RuntimeCall::EVM(pallet_evm::Call::call {
				source: futurepass.into(), // NOTE: source should be the futurepass.
				target: H160::from_low_u64_be(1),
				input: vec![],
				value: U256::zero(),
				gas_limit: 40000,
				max_fee_per_gas: U256::from(15_000_000_000_000u64), // 15 Twei
				max_priority_fee_per_gas: Some(U256::from(1_500_000_000u64)), // 1.5 gwei
				nonce: None,
				access_list: vec![],
			});

			let evm_call_2 = RuntimeCall::EVM(pallet_evm::Call::call {
				source: futurepass.into(), // NOTE: source should be the futurepass.
				target: H160::from_low_u64_be(2),
				input: vec![],
				value: U256::zero(),
				gas_limit: 35000,
				max_fee_per_gas: U256::from(15_000_000_000_000u64), // 15 Twei
				max_priority_fee_per_gas: Some(U256::from(1_500_000_000u64)), // 1.5 gwei
				nonce: None,
				access_list: vec![],
			});

			// Create non-EVM calls for the batch
			let remark_call_1 = RuntimeCall::System(frame_system::Call::remark {
				remark: b"Proxy test 1".to_vec(),
			});

			let remark_call_2 = RuntimeCall::System(frame_system::Call::remark_with_event {
				remark: b"Proxy test 2".to_vec(),
			});

			// Create batch_all call containing both EVM and non-EVM calls
			let batch_call = RuntimeCall::Utility(pallet_utility::Call::batch_all {
				calls: vec![evm_call_1, remark_call_1, evm_call_2, remark_call_2],
			});

			// Wrap in proxy_extrinsic
			let proxy_call = RuntimeCall::Futurepass(pallet_futurepass::Call::proxy_extrinsic {
				futurepass,
				call: Box::new(batch_call),
			});

			let max_payment = scale_wei_to_6dp(100_000 * 15_000_000_000_000u128 * 2); // Enough to cover EVM calls

			// reset events
			System::reset_events();

			// Test the proxy call with mixed EVM and non-EVM calls in batch_all
			assert_ok!(FeeProxy::call_with_fee_preferences(
				RawOrigin::Signed(new_account).into(),
				payment_asset,
				max_payment,
				Box::new(proxy_call)
			));

			// Check that EVM calls were processed
			let events = System::events();
			let evm_event_count = events
				.iter()
				.filter(|e| {
					matches!(
						e.event,
						RuntimeEvent::EVM(pallet_evm::Event::Executed { .. })
							| RuntimeEvent::EVM(pallet_evm::Event::ExecutedFailed { .. })
					)
				})
				.count();
			assert!(evm_event_count >= 2, "Should have at least 2 EVM events for 2 EVM calls");

			// Check that EVM calls executed (note: address 0x1 is ECRecover, 0x2 is Sha256)
			System::assert_has_event(
				pallet_evm::Event::<Runtime>::Executed { address: H160::from_low_u64_be(1) }.into(),
			);
			System::assert_has_event(
				pallet_evm::Event::<Runtime>::Executed { address: H160::from_low_u64_be(2) }.into(),
			);

			// Check that remark_with_event call emitted event (sender should be futurepass)
			System::assert_has_event(
				frame_system::Event::<Runtime>::Remarked {
					sender: futurepass,
					hash: sp_core::blake2_256(b"Proxy test 2").into(),
				}
				.into(),
			);

			// Check that all batch items completed (2 EVM calls + 2 remarks = 4 items)
			let item_completed_count = events
				.iter()
				.filter(|e| {
					matches!(e.event, RuntimeEvent::Utility(pallet_utility::Event::ItemCompleted))
				})
				.count();
			assert_eq!(
				item_completed_count, 4,
				"Should have 4 ItemCompleted events for proxy batch"
			);

			// Check that the batch completed successfully
			System::assert_has_event(pallet_utility::Event::BatchCompleted.into());

			// Check that futurepass proxy executed successfully
			System::assert_has_event(
				pallet_futurepass::Event::<Runtime>::ProxyExecuted {
					delegate: new_account,
					result: Ok(()),
				}
				.into(),
			);

			// Check that the fee proxy event was emitted
			System::assert_has_event(
				pallet_fee_proxy::Event::<Runtime>::CallWithFeePreferences {
					who: new_account,
					payment_asset,
					max_payment,
				}
				.into(),
			);
		});
	}
}
