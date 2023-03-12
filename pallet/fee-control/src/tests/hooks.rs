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
use crate::SettingsAndMultipliers;
use frame_support::{traits::OnFinalize, weights::Weight};
use sp_runtime::Permill;
use std::ops::Mul;

const MAX_WEIGHT: Weight = 1000000000000u64;

#[test]
fn below_or_equal_threshold_no_change_should_be_visible() {
	ExtBuilder::build().execute_with(|| {
		let threshold = <Test as crate::Config>::Threshold::get() - Permill::from_percent(1);
		let consumed_weight = threshold.mul(MAX_WEIGHT);
		let expected_fee = SettingsAndMultipliers::<Test>::get().adjusted_evm_base_fee;

		// Sanity check
		assert!(threshold > Permill::zero());

		// Call
		System::set_block_consumed_resources(consumed_weight, 0);
		FeeControl::on_finalize(0);

		// Check
		let actual_fee = SettingsAndMultipliers::<Test>::get().adjusted_evm_base_fee;
		assert_eq!(actual_fee, expected_fee)
	})
}

#[test]
fn above_threshold_should_change_adjusted_fee() {
	ExtBuilder::build().execute_with(|| {
		let adjusted_fee = SettingsAndMultipliers::<Test>::get().adjusted_evm_base_fee;
		let reference_fee = SettingsAndMultipliers::<Test>::get().reference_evm_base_fee;
		let elasticity = <Test as crate::Config>::Elasticity::get();
		let expected_fee = reference_fee + elasticity.mul(u128::try_from(reference_fee).unwrap());

		// Sanity check
		assert_eq!(adjusted_fee, reference_fee);
		assert!(expected_fee > adjusted_fee);

		// Call
		System::set_block_consumed_resources(MAX_WEIGHT, 0);
		FeeControl::on_finalize(0);

		// Check
		let actual_fee = SettingsAndMultipliers::<Test>::get().adjusted_evm_base_fee;
		assert_eq!(actual_fee, expected_fee)
	})
}

#[test]
fn adjusted_fee_reaches_maximum() {
	ExtBuilder::build().execute_with(|| {
		let reference_fee = SettingsAndMultipliers::<Test>::get().reference_evm_base_fee;
		let elasticity = <Test as crate::Config>::Elasticity::get();
		let mut expected_fee = reference_fee;
		let max_fee = reference_fee * 2;

		for _ in 0..100_000 {
			expected_fee = expected_fee + elasticity.mul(u128::try_from(reference_fee).unwrap());

			// Call
			System::set_block_consumed_resources(MAX_WEIGHT, 0);
			FeeControl::on_finalize(0);

			// Check
			let actual_fee = SettingsAndMultipliers::<Test>::get().adjusted_evm_base_fee;
			assert_eq!(actual_fee, expected_fee);

			if max_fee == expected_fee {
				break
			}
		}

		assert_eq!(max_fee, expected_fee);

		// Check
		let actual_fee = SettingsAndMultipliers::<Test>::get().adjusted_evm_base_fee;
		assert_eq!(actual_fee, max_fee);
	})
}

#[test]
fn adjusted_fee_can_fluctuate() {
	ExtBuilder::build().execute_with(|| {
		let reference_fee = SettingsAndMultipliers::<Test>::get().reference_evm_base_fee;
		let elasticity = <Test as crate::Config>::Elasticity::get();
		let mut expected_fee = reference_fee;
		let i = 2;

		for _ in 0..i {
			expected_fee = expected_fee + elasticity.mul(u128::try_from(reference_fee).unwrap());

			// Call
			System::set_block_consumed_resources(MAX_WEIGHT, 0);
			FeeControl::on_finalize(0);

			// Check
			let actual_fee = SettingsAndMultipliers::<Test>::get().adjusted_evm_base_fee;
			assert_eq!(actual_fee, expected_fee);
		}

		for _ in 0..i {
			expected_fee = expected_fee - elasticity.mul(u128::try_from(reference_fee).unwrap());

			// Call
			System::set_block_consumed_resources(0, 0);
			FeeControl::on_finalize(0);

			// Check
			let actual_fee = SettingsAndMultipliers::<Test>::get().adjusted_evm_base_fee;
			assert_eq!(actual_fee, expected_fee);
		}

		// Check
		let actual_fee = SettingsAndMultipliers::<Test>::get().adjusted_evm_base_fee;
		assert_eq!(actual_fee, reference_fee);
	})
}

#[test]
fn adjusted_fee_cannot_go_below_reference_fee() {
	ExtBuilder::build().execute_with(|| {
		let reference_fee = SettingsAndMultipliers::<Test>::get().reference_evm_base_fee;
		let elasticity = <Test as crate::Config>::Elasticity::get();
		let mut expected_fee = reference_fee;
		let max_fee = reference_fee * 2;

		for _ in 0..100_000 {
			expected_fee = expected_fee + elasticity.mul(u128::try_from(reference_fee).unwrap());

			// Call
			System::set_block_consumed_resources(MAX_WEIGHT, 0);
			FeeControl::on_finalize(0);

			// Check
			let actual_fee = SettingsAndMultipliers::<Test>::get().adjusted_evm_base_fee;
			assert_eq!(actual_fee, expected_fee);

			if max_fee == expected_fee {
				break
			}
		}

		for _ in 0..100_000 {
			expected_fee = expected_fee - elasticity.mul(u128::try_from(reference_fee).unwrap());

			// Call
			System::set_block_consumed_resources(0, 0);
			FeeControl::on_finalize(0);

			// Check
			let actual_fee = SettingsAndMultipliers::<Test>::get().adjusted_evm_base_fee;
			assert_eq!(actual_fee, expected_fee);

			if expected_fee == reference_fee {
				break
			}
		}

		assert_eq!(reference_fee, expected_fee);
		let actual_fee = SettingsAndMultipliers::<Test>::get().adjusted_evm_base_fee;
		assert_eq!(actual_fee, reference_fee);

		// Call
		System::set_block_consumed_resources(0, 0);
		FeeControl::on_finalize(0);

		// Check
		let actual_fee = SettingsAndMultipliers::<Test>::get().adjusted_evm_base_fee;
		assert_eq!(actual_fee, reference_fee);
	})
}
