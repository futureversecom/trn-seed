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
use crate::{
	mock::{FeeProxy, Futurepass, Runner, RuntimeOrigin, System, Test, TestExt, XRP_ASSET_ID},
	runner::*,
};
use ethabi::Token;
use frame_support::{assert_noop, assert_ok};
use hex_literal::hex;
use precompile_utils::{
	constants::{
		ERC20_PRECOMPILE_ADDRESS_PREFIX, FEE_FUNCTION_SELECTOR, FEE_FUNCTION_SELECTOR_DEPRECATED,
	},
	ErcIdConversion,
};
use seed_primitives::{AccountId, AssetId, Balance};
use sp_core::{H160, U256};

fn create_account(seed: u64) -> AccountId {
	AccountId::from(H160::from_low_u64_be(seed))
}

/// Tests for the extrinsic call_with_fee_preferences
mod call_with_fee_preferences {
	use super::*;

	#[test]
	fn call_works() {
		TestExt::default().build().execute_with(|| {
			let caller: AccountId = create_account(1);
			let payment_asset: AssetId = 10;
			let max_payment: Balance = 100;
			let call = mock::RuntimeCall::System(frame_system::Call::remark {
				remark: b"Mischief Managed".to_vec(),
			});

			assert_ok!(FeeProxy::call_with_fee_preferences(
				Some(caller).into(),
				payment_asset,
				max_payment,
				Box::new(call)
			));

			System::assert_has_event(
				Event::CallWithFeePreferences { who: caller, payment_asset, max_payment }.into(),
			);
		});
	}

	#[test]
	fn call_works_for_futurepass_proxy_extrinsic() {
		TestExt::default().build().execute_with(|| {
			let owner: AccountId = create_account(1);
			let payment_asset: AssetId = 10;
			let max_payment: Balance = 100;

			assert_ok!(Futurepass::create(RuntimeOrigin::signed(owner), owner));
			let futurepass = pallet_futurepass::Holders::<Test>::get(&owner).unwrap();

			let call = mock::RuntimeCall::System(frame_system::Call::remark {
				remark: b"Mischief Managed".to_vec(),
			});
			let proxy_call =
				mock::RuntimeCall::Futurepass(pallet_futurepass::Call::proxy_extrinsic {
					futurepass,
					call: Box::new(call),
				});

			assert_ok!(FeeProxy::call_with_fee_preferences(
				Some(owner).into(),
				payment_asset,
				max_payment,
				Box::new(proxy_call)
			));

			// assert Futurepass event ProxyExecuted
			System::assert_has_event(
				pallet_futurepass::Event::<Test>::ProxyExecuted { delegate: owner, result: Ok(()) }
					.into(),
			);

			// assert fee proxy event
			System::assert_has_event(
				Event::CallWithFeePreferences { who: owner, payment_asset, max_payment }.into(),
			);
		});
	}

	#[test]
	fn payment_asset_must_differ_from_fee_asset() {
		TestExt::default().build().execute_with(|| {
			let caller: AccountId = create_account(1);
			let payment_asset: AssetId = XRP_ASSET_ID;
			let max_payment: Balance = 100;
			let call = mock::RuntimeCall::System(frame_system::Call::remark {
				remark: b"Mischief Managed".to_vec(),
			});

			// Should fail as the payment token is the fee asset
			assert_noop!(
				FeeProxy::call_with_fee_preferences(
					Some(caller).into(),
					payment_asset,
					max_payment,
					Box::new(call)
				),
				Error::<Test>::FeeTokenIsGasToken
			);
		});
	}

	#[test]
	fn inner_call_results_need_to_be_propagated() {
		TestExt::default().build().execute_with(|| {
			let caller: AccountId = create_account(1);
			let payment_asset: AssetId = 10;
			let max_payment: Balance = 100;
			let call = mock::RuntimeCall::System(frame_system::Call::fill_block {
				ratio: Default::default(),
			});

			// Test that the error returned is the error from the inner call. In this case it is
			// BadOrigin as fill_block requires root. This is the easiest example to use without
			// pulling in more dev dependencies
			assert_noop!(
				FeeProxy::call_with_fee_preferences(
					Some(caller).into(),
					payment_asset,
					max_payment,
					Box::new(call)
				),
				DispatchError::BadOrigin
			);
		});
	}

	#[test]
	fn inner_call_must_differ_from_outer_call() {
		TestExt::default().build().execute_with(|| {
			let caller: AccountId = create_account(1);
			let payment_asset: AssetId = 10;
			let max_payment: Balance = 100;

			let call_inner = mock::RuntimeCall::System(frame_system::Call::remark {
				remark: b"Mischief Managed".to_vec(),
			});

			let call = mock::RuntimeCall::FeeProxy(crate::Call::call_with_fee_preferences {
				payment_asset,
				max_payment,
				call: Box::new(call_inner),
			});

			// Should fail as the inner call is call_with_fee_preferences
			assert_noop!(
				FeeProxy::call_with_fee_preferences(
					Some(caller).into(),
					payment_asset,
					max_payment,
					Box::new(call)
				),
				Error::<Test>::NestedFeePreferenceCall
			);
		});
	}
}

/// Unit tests for the decode_input function on the runner
mod decode_input {
	use super::*;

	#[test]
	fn decode_input_works() {
		TestExt::default().build().execute_with(|| {
			// Abi generated from below parameters using the following function name:
			// callWithFeePreferences
			// abi can be easily generated here https://abi.hashex.org/
			let exp_payment_asset = 16000_u32;
			let exp_max_payment = 123_456_789 as Balance;
			let exp_target = H160::from_slice(&hex!("cCccccCc00003E80000000000000000000000000"));
			let exp_input: Vec<u8> =
				hex!("a9059cbb0000000000000000000000007a107fc1794f505cb351148f529accae12ffbcd8000000000000000000000000000000000000000000000000000000000000007b"
			).to_vec();
			let mut input = FEE_FUNCTION_SELECTOR_DEPRECATED.to_vec();
			input.append(&mut ethabi::encode(&[
				Token::Address(Test::runtime_id_to_evm_id(exp_payment_asset, ERC20_PRECOMPILE_ADDRESS_PREFIX).0),
				Token::Uint(exp_max_payment.into()),
				Token::Address(exp_target),
				Token::Bytes(exp_input.clone())],
			));

			assert_eq!(
				Runner::decode_input(input),
				Ok((exp_payment_asset, exp_max_payment, exp_target, exp_input.clone()))
			);

			let mut input = FEE_FUNCTION_SELECTOR.to_vec();
			input.append(&mut ethabi::encode(&[
				Token::Address(Test::runtime_id_to_evm_id(exp_payment_asset, ERC20_PRECOMPILE_ADDRESS_PREFIX).0),
				Token::Uint(exp_max_payment.into()),
				Token::Address(exp_target),
				Token::Bytes(exp_input.clone())],
			));

			assert_eq!(
				Runner::decode_input(input),
				Ok((exp_payment_asset, exp_max_payment, exp_target, exp_input))
			);
		});
	}

	#[test]
	fn invalid_function_selector_should_fail() {
		TestExt::default().build().execute_with(|| {
			let bad_selector_input = vec![0x01, 0x02, 0x03, 0x04];
			assert_noop!(
				Runner::decode_input(bad_selector_input),
				FeePreferencesError::InvalidFunctionSelector
			);
		});
	}

	#[test]
	fn empty_input_should_fail() {
		TestExt::default().build().execute_with(|| {
			assert_noop!(
				Runner::decode_input(Default::default()),
				FeePreferencesError::InvalidInputArguments
			);
		});
	}

	#[test]
	fn invalid_input_args_should_fail() {
		TestExt::default().build().execute_with(|| {
			let mut input = FEE_FUNCTION_SELECTOR_DEPRECATED.to_vec();
			input.append(&mut ethabi::encode(&[
				Token::Bytes(vec![1_u8, 2, 3, 4, 5]),
				Token::Array(vec![
					Token::Uint(1u64.into()),
					Token::Uint(2u64.into()),
					Token::Uint(3u64.into()),
					Token::Uint(4u64.into()),
					Token::Uint(5u64.into()),
				]),
			]));
			assert_noop!(Runner::decode_input(input), FeePreferencesError::FailedToDecodeInput);

			let mut input = FEE_FUNCTION_SELECTOR.to_vec();
			input.append(&mut ethabi::encode(&[
				Token::Bytes(vec![1_u8, 2, 3, 4, 5]),
				Token::Array(vec![
					Token::Uint(1u64.into()),
					Token::Uint(2u64.into()),
					Token::Uint(3u64.into()),
					Token::Uint(4u64.into()),
					Token::Uint(5u64.into()),
				]),
			]));
			assert_noop!(Runner::decode_input(input), FeePreferencesError::FailedToDecodeInput);
		});
	}

	#[test]
	fn zero_payment_asset_should_fail() {
		TestExt::default().build().execute_with(|| {
			let mut input = FEE_FUNCTION_SELECTOR_DEPRECATED.to_vec();
			input.append(&mut ethabi::encode(&[
				Token::Address(H160::zero()),
				Token::Uint(5u64.into()),
				Token::Address(H160::default()),
				Token::Bytes(vec![1_u8, 2, 3, 4, 5]),
			]));
			assert_noop!(
				Runner::decode_input(input.to_vec()),
				FeePreferencesError::InvalidPaymentAsset
			);

			let mut input = FEE_FUNCTION_SELECTOR.to_vec();
			input.append(&mut ethabi::encode(&[
				Token::Address(H160::zero()),
				Token::Uint(5u64.into()),
				Token::Address(H160::default()),
				Token::Bytes(vec![1_u8, 2, 3, 4, 5]),
			]));
			assert_noop!(
				Runner::decode_input(input.to_vec()),
				FeePreferencesError::InvalidPaymentAsset
			);
		});
	}
}

/// Unit tests for the get_fee_preferences_data function in the runner file
mod get_fee_preferences_data {
	use super::*;

	#[test]
	fn get_fee_preferences_data_works() {
		TestExt::default().build().execute_with(|| {
			let gas_limit: u64 = 100;
			let base_fee_per_gas: U256 = 200.into();
			let max_fee_per_gas: U256 = 300.into();
			let payment_asset_id: AssetId = 12;

			let expected_path = vec![payment_asset_id, <Test as Config>::FeeAssetId::get()];
			let (expected_fee, max_fee) = Runner::calculate_total_gas(
				gas_limit,
				base_fee_per_gas,
				Some(max_fee_per_gas),
				None,
			)
			.unwrap();

			let expected_fee_scaled: Balance =
				seed_pallet_common::utils::scale_wei_to_correct_decimals(expected_fee, 0);
			let max_fee_scaled: Balance =
				seed_pallet_common::utils::scale_wei_to_correct_decimals(max_fee, 0);
			assert_eq!(
				get_fee_preferences_data::<Test, Test, crate::mock::Futurepass>(
					gas_limit,
					base_fee_per_gas,
					Some(max_fee_per_gas),
					None,
					payment_asset_id
				),
				Ok(FeePreferencesData {
					total_fee_scaled: expected_fee_scaled,
					path: expected_path,
					max_fee_scaled,
				})
			);
		});
	}
}

/// Unit tests for the calculate total gas function on the runner
mod calculate_total_gas {
	use super::*;

	#[test]
	fn base_fee_only() {
		TestExt::default().build().execute_with(|| {
			let gas_limit: u64 = 100;
			let base_fee_per_gas: U256 = 200.into();

			let (total_fee, max_fee) =
				Runner::calculate_total_gas(gas_limit, base_fee_per_gas, None, None).unwrap();

			assert_eq!(total_fee, U256::from(20_000));
			assert_eq!(max_fee, U256::from(40_000));
		});
	}

	#[test]
	fn max_fee_per_gas() {
		TestExt::default().build().execute_with(|| {
			let gas_limit: u64 = 100;
			let base_fee_per_gas: U256 = 200.into();
			let max_fee_per_gas: U256 = 300.into();

			let (total_fee, max_fee) = Runner::calculate_total_gas(
				gas_limit,
				base_fee_per_gas,
				Some(max_fee_per_gas),
				None,
			)
			.unwrap();

			assert_eq!(total_fee, U256::from(20_000));
			assert_eq!(max_fee, U256::from(30_000));
		});
	}

	#[test]
	fn max_priority_fee_per_gas() {
		TestExt::default().build().execute_with(|| {
			let gas_limit: u64 = 100;
			let base_fee_per_gas: U256 = 200.into();
			let max_priority_fee_per_gas: U256 = 50.into();

			let (total_fee, max_fee) = Runner::calculate_total_gas(
				gas_limit,
				base_fee_per_gas,
				None,
				Some(max_priority_fee_per_gas),
			)
			.unwrap();

			assert_eq!(total_fee, U256::from(25_000));
			assert_eq!(max_fee, U256::from(45000));
		});
	}

	#[test]
	fn max_fee_per_gas_with_max_priority_fee_per_gas() {
		TestExt::default().build().execute_with(|| {
			let gas_limit: u64 = 100;
			let base_fee_per_gas: U256 = 200.into();
			let max_fee_per_gas: U256 = 300.into();
			let max_priority_fee_per_gas: U256 = 50.into();

			let (total_fee, max_fee) = Runner::calculate_total_gas(
				gas_limit,
				base_fee_per_gas,
				Some(max_fee_per_gas),
				Some(max_priority_fee_per_gas),
			)
			.unwrap();

			assert_eq!(total_fee, U256::from(25_000));
			assert_eq!(max_fee, U256::from(30_000));
		});
	}

	#[test]
	fn max_fee_per_gas_too_large_should_fail() {
		TestExt::default().build().execute_with(|| {
			let gas_limit: u64 = 100;
			let base_fee_per_gas: U256 = 200.into();
			let max_fee_per_gas = U256::MAX;

			assert_noop!(
				Runner::calculate_total_gas(
					gas_limit,
					base_fee_per_gas,
					Some(max_fee_per_gas),
					None
				),
				FeePreferencesError::FeeOverflow
			);
		});
	}
}
