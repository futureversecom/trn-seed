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

use super::mock::*;
use crate::{
	types::{ConfigOp::Noop, DecimalBalance, FeeControlData},
	Error, Event as FeeControlEvent, SettingsAndMultipliers,
};
use core::ops::Add;
use frame_support::{assert_noop, assert_ok, weights::Weight};
use sp_core::U256;
use sp_runtime::Perbill;
use Event as RuntimeEvent;

mod set_fee_control_config {
	use super::*;

	#[test]
	fn set_fee_control_config() {
		ExtBuilder::build().execute_with(|| {
			// Setup
			let settings = SettingsAndMultipliers::<Test>::get();
			let new_weight_multiplier = Perbill::from_parts(300);
			let new_length_multiplier = DecimalBalance::new(1u128, Perbill::from_parts(300));
			assert_ne!(settings.weight_multiplier, new_weight_multiplier);
			assert_ne!(settings.length_multiplier, new_length_multiplier);

			let expected_settings = FeeControlData {
				weight_multiplier: new_weight_multiplier,
				length_multiplier: new_length_multiplier,
				reference_evm_base_fee: settings.reference_evm_base_fee.add(1),
				adjusted_evm_base_fee: settings.adjusted_evm_base_fee.add(1),
				input_tx_weight: settings.input_tx_weight.add(1),
				input_gas_limit: settings.input_gas_limit.add(1),
				output_tx_fee: settings.output_tx_fee.add(1),
				output_len_fee: settings.output_len_fee.add(1),
				is_locked: !settings.is_locked,
				refresh_data: !settings.refresh_data,
			};
			assert_ne!(settings, expected_settings);
			let new = expected_settings.clone();

			// Call
			let ok = FeeControl::set_fee_control_config(
				root(),
				new.weight_multiplier.into(),
				new.length_multiplier.into(),
				new.reference_evm_base_fee.into(),
				new.adjusted_evm_base_fee.into(),
				new.input_tx_weight.into(),
				new.input_gas_limit.into(),
				new.output_tx_fee.into(),
				new.output_len_fee.into(),
				new.is_locked.into(),
				new.refresh_data.into(),
			);
			assert_ok!(ok);

			// Storage Check
			let actual_settings = SettingsAndMultipliers::<Test>::get();
			assert_eq!(actual_settings, expected_settings);

			// Event Check
			let event = FeeControlEvent::NewSettingsHaveBeenApplied;
			let event = RuntimeEvent::FeeControl(event);
			System::assert_last_event(event);
		})
	}

	#[test]
	fn noop_doesnt_change_storage_value() {
		ExtBuilder::build().execute_with(|| {
			// Setup
			let expected_settings = SettingsAndMultipliers::<Test>::get();

			// Call
			let ok = FeeControl::set_fee_control_config(
				root(),
				Noop,
				Noop,
				Noop,
				Noop,
				Noop,
				Noop,
				Noop,
				Noop,
				Noop,
				Noop,
			);
			assert_ok!(ok);

			// Storage Check
			let actual_settings = SettingsAndMultipliers::<Test>::get();
			assert_eq!(actual_settings, expected_settings);

			// Event Check
			// Omitted: Already checked in `set_fee_control_config` test.
		})
	}

	#[test]
	fn only_authorized_accounts_can_call_this_extrinsic() {
		ExtBuilder::build().execute_with(|| {
			// Call
			let ok = FeeControl::set_fee_control_config(
				origin(0),
				Noop,
				Noop,
				Noop,
				Noop,
				Noop,
				Noop,
				Noop,
				Noop,
				Noop,
				Noop,
			);
			assert_noop!(ok, frame_support::error::BadOrigin);
		})
	}
}

mod set_xrp_price {
	use super::*;

	#[test]
	fn set_xrp_price() {
		ExtBuilder::build().execute_with(|| {
			// Setup
			let one_xrp = Balance::from(1_000_000u32);
			assert_eq!(<Test as crate::Config>::OneXRP::get(), one_xrp);

			let ok = SettingsBuilder::new()
				.tx_weight(Weight::from(1_000_000u32)) // 1 million weight
				.gas_limit(U256::from(1_000u32)) // 1k gas limit
				.tx_fee(Balance::from(1_000_000u32)) // This is 1€
				.len_fee(Balance::from(1_000_000u32)) // This is 1€
				.done();
			assert_ok!(ok);
			let mut expected_storage = SettingsAndMultipliers::<Test>::get();

			// Call
			let xrp_price = Balance::from(1_000_000u32);
			assert_ok!(FeeControl::set_xrp_price(root(), xrp_price));

			// Storage Check
			expected_storage.weight_multiplier = Perbill::one();
			expected_storage.length_multiplier = DecimalBalance::new(one_xrp, Perbill::zero());
			expected_storage.reference_evm_base_fee = U256::from(10u32).pow(U256::from(15));
			expected_storage.adjusted_evm_base_fee = expected_storage.reference_evm_base_fee;

			let actual_storage = SettingsAndMultipliers::<Test>::get();
			assert_eq!(actual_storage, expected_storage);

			// Event Check
			let event = FeeControlEvent::NewXRPPrice { value: xrp_price };
			let event = RuntimeEvent::FeeControl(event);
			System::assert_last_event(event);
		})
	}

	#[test]
	fn more_realistic_values() {
		ExtBuilder::build().execute_with(|| {
			// Setup
			let ok = SettingsBuilder::new()
				.tx_weight(Weight::from(500_000_000u32)) // 500 million weight
				.gas_limit(U256::from(20_000u32)) // 20k gas limit
				.tx_fee(Balance::from(100_000u32)) // This is 0.1€
				.len_fee(Balance::from(1u32)) // This is 0.000001€
				.done();
			assert_ok!(ok);
			let mut expected_storage = SettingsAndMultipliers::<Test>::get();

			// Call
			let xrp_price = Balance::from(250_000u32); // This is 0.25€
			assert_ok!(FeeControl::set_xrp_price(root(), xrp_price));

			// Storage Check
			expected_storage.weight_multiplier = Perbill::from_rational(1u32, 1250u32);
			expected_storage.length_multiplier = DecimalBalance::new(4u128, Perbill::zero());
			expected_storage.reference_evm_base_fee = U256::from(20_000_000_000_000u128);
			expected_storage.adjusted_evm_base_fee = expected_storage.reference_evm_base_fee;

			let actual_storage = SettingsAndMultipliers::<Test>::get();
			assert_eq!(actual_storage, expected_storage);
		})
	}

	#[test]
	fn xrp_worth_more_than_one_dollar() {
		ExtBuilder::build().execute_with(|| {
			// Setup
			let ok = SettingsBuilder::new()
				.tx_weight(Weight::from(500_000_000u32)) // 500 million weight
				.gas_limit(U256::from(20_000u32)) // 20k gas limit
				.tx_fee(Balance::from(100_000u32)) // This is 0.1€
				.len_fee(Balance::from(1_000u32)) // This is 0.000001€
				.done();
			assert_ok!(ok);
			let mut expected_storage = SettingsAndMultipliers::<Test>::get();

			// Call
			let xrp_price = Balance::from(10_250_000u32); // This is 10.25€
			assert_ok!(FeeControl::set_xrp_price(root(), xrp_price));

			// Storage Check
			expected_storage.weight_multiplier = Perbill::from_rational(1u32, 51_250u32);
			expected_storage.length_multiplier =
				DecimalBalance::new(97u128, Perbill::from_rational(5_750_000u128, xrp_price));
			expected_storage.reference_evm_base_fee = U256::from(487_804_878_048u128);
			expected_storage.adjusted_evm_base_fee = expected_storage.reference_evm_base_fee;

			let actual_storage = SettingsAndMultipliers::<Test>::get();
			assert_eq!(actual_storage, expected_storage);
		})
	}

	#[test]
	fn increase_in_reference_base_fee_causes_increase_in_adjusted_fee() {
		ExtBuilder::build().execute_with(|| {
			let reference_fee = SettingsAndMultipliers::<Test>::get().reference_evm_base_fee;
			let adjusted_fee = SettingsAndMultipliers::<Test>::get().adjusted_evm_base_fee;
			let expected_reference_fee = U256::from(20_000_000_000_000u128);
			let expected_adjusted_fee = expected_reference_fee;
			assert!(expected_reference_fee > reference_fee);
			assert!(expected_adjusted_fee > adjusted_fee);

			// Setup
			let ok = SettingsBuilder::new()
				.tx_weight(Weight::from(500_000_000u32)) // 500 million weight
				.gas_limit(U256::from(20_000u32)) // 20k gas limit
				.tx_fee(Balance::from(100_000u32)) // This is 0.1€
				.len_fee(Balance::from(1u32)) // This is 0.000001€
				.done();
			assert_ok!(ok);

			// Call
			let xrp_price = Balance::from(250_000u32); // This is 0.25€
			assert_ok!(FeeControl::set_xrp_price(root(), xrp_price));

			// Storage Check
			let actual_reference_fee = SettingsAndMultipliers::<Test>::get().reference_evm_base_fee;
			let actual_adjusted_fee = SettingsAndMultipliers::<Test>::get().adjusted_evm_base_fee;
			assert_eq!(actual_reference_fee, expected_reference_fee);
			assert_eq!(actual_adjusted_fee, expected_adjusted_fee);
		})
	}

	#[test]
	fn if_equal_decrease_in_reference_base_fee_causes_decrease_in_adjusted_fee() {
		ExtBuilder::build().execute_with(|| {
			let reference_fee = SettingsAndMultipliers::<Test>::get().reference_evm_base_fee;
			let adjusted_fee = SettingsAndMultipliers::<Test>::get().adjusted_evm_base_fee;
			let expected_reference_fee = U256::from(4_000_000_000_000u128);
			let expected_adjusted_fee = expected_reference_fee;
			assert!(expected_reference_fee < reference_fee);
			assert!(expected_adjusted_fee < adjusted_fee);

			// Setup
			let ok = SettingsBuilder::new()
				.tx_weight(Weight::from(500_000_000u32)) // 500 million weight
				.gas_limit(U256::from(20_000u32)) // 20k gas limit
				.tx_fee(Balance::from(100_000u32)) // This is 0.1€
				.len_fee(Balance::from(1u32)) // This is 0.000001€
				.done();
			assert_ok!(ok);

			// Call
			let xrp_price = Balance::from(1_250_000u32); // This is 0.25€
			assert_ok!(FeeControl::set_xrp_price(root(), xrp_price));

			// Storage Check
			let actual_reference_fee = SettingsAndMultipliers::<Test>::get().reference_evm_base_fee;
			let actual_adjusted_fee = SettingsAndMultipliers::<Test>::get().adjusted_evm_base_fee;
			assert_eq!(actual_reference_fee, expected_reference_fee);
			assert_eq!(actual_adjusted_fee, expected_adjusted_fee);
		})
	}

	#[test]
	fn xrp_price_cannot_be_zero() {
		ExtBuilder::build().execute_with(|| {
			// Call
			let err = FeeControl::set_xrp_price(root(), Balance::from(0u32));
			assert_noop!(err, Error::<Test>::XRPPriceCannotBeZero);
		})
	}

	#[test]
	fn input_tx_weight_cannot_be_zero() {
		ExtBuilder::build().execute_with(|| {
			// Setup
			let ok = SettingsBuilder::new().tx_weight(Weight::from(0u32)).done();
			assert_ok!(ok);

			// Call
			let err = FeeControl::set_xrp_price(root(), Balance::from(1_000_000u128));
			assert_noop!(err, Error::<Test>::InputTxWeightCannotBeZero);
		})
	}

	#[test]
	fn output_tx_fee_cannot_be_zero() {
		ExtBuilder::build().execute_with(|| {
			// Setup
			let ok = SettingsBuilder::new().tx_fee(Balance::from(0u128)).done();
			assert_ok!(ok);

			// Call
			let err = FeeControl::set_xrp_price(root(), Balance::from(1_000_000u128));
			assert_noop!(err, Error::<Test>::OutputTxFeeCannotBeZero);
		})
	}

	#[test]
	fn weight_quotient_cannot_be_zero() {
		ExtBuilder::build().execute_with(|| {
			// Setup
			let ok = SettingsBuilder::new().tx_fee(Balance::from(1u128)).done();
			assert_ok!(ok);

			// Call
			let err =
				FeeControl::set_xrp_price(root(), Balance::from(1_000_000_000_000_000_000_000u128));
			assert_noop!(err, Error::<Test>::WeightMultiplierQuotientCannotBeZero);
		})
	}

	#[test]
	fn one_weight_cannot_be_worth_more_than_one_xrp() {
		ExtBuilder::build().execute_with(|| {
			// Setup
			let ok = SettingsBuilder::new()
				.tx_weight(Weight::from(1_000u32))
				.tx_fee(Balance::from(1_000_000u128))
				.done();
			assert_ok!(ok);

			// Call
			let err = FeeControl::set_xrp_price(root(), Balance::from(1_000_000u128));
			assert_noop!(err, Error::<Test>::OneWeightCannotBeWorthMoreThanOneXRP);
		})
	}

	#[test]
	fn output_length_fee_cannot_be_zero() {
		ExtBuilder::build().execute_with(|| {
			// Setup
			let ok = SettingsBuilder::new().len_fee(Balance::from(0u32)).done();
			assert_ok!(ok);

			// Call
			let err = FeeControl::set_xrp_price(root(), Balance::from(1_000_000u128));
			assert_noop!(err, Error::<Test>::OutputLenFeeCannotBeZero);
		})
	}

	#[test]
	fn input_gas_limit_cannot_be_zero() {
		ExtBuilder::build().execute_with(|| {
			// Setup
			let ok = SettingsBuilder::new().gas_limit(U256::from(0u32)).done();
			assert_ok!(ok);

			// Call
			let err = FeeControl::set_xrp_price(root(), Balance::from(1_000_000u128));
			assert_noop!(err, Error::<Test>::InputGasLimitCannotBeZero);
		})
	}

	#[test]
	fn evm_multiplier_calculation_error() {
		ExtBuilder::build().execute_with(|| {
			// Setup
			let ok = SettingsBuilder::new()
				.tx_fee(Balance::from(1u32))
				.gas_limit(U256::from(1_000_000_000_000_000_000u128))
				.done();
			assert_ok!(ok);

			// Call
			let err = FeeControl::set_xrp_price(root(), Balance::from(1_000_000u128));
			assert_noop!(err, Error::<Test>::EvmMultiplierCalculationError);
		})
	}
}
