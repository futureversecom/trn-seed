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

use codec::{Codec, Decode, Encode, MaxEncodedLen};
use core::ops::{Div, Mul};
use frame_support::{ensure, weights::Weight};
use scale_info::TypeInfo;
use seed_primitives::Balance;
use sp_core::U256;
use sp_runtime::Perbill;

#[derive(TypeInfo, Debug, Copy, Clone, Encode, Decode, PartialEq)]
pub enum CalculatorErrors {
	OneXRPCannotBeZero = 1,
	XRPPriceCannotBeZero = 2,
	EvmXrpScaleFactorCannotBeZero = 3,
	InputTxWeightCannotBeZero = 4,
	InputGasLimitCannotBeZero = 5,
	OutputTxFeeCannotBeZero = 6,
	OutputLenFeeCannotBeZero = 7,
	WeightMultiplierQuotientCannotBeZero = 8,
	OneWeightCannotBeWorthMoreThanOneXRP = 9,
	LengthMultiplierCalculationError = 10,
	EvmMultiplierCalculationError = 11,
}

pub struct FeeMultiplierCalculator;

use CalculatorErrors::*;
impl FeeMultiplierCalculator {
	pub fn weight_multiplier(
		one_xrp: Balance,
		xrp_price: Balance,
		input_tx_weight: Weight,
		output_tx_fee: Balance,
	) -> Result<Perbill, CalculatorErrors> {
		//
		// General Formula:
		//
		//           OTF * ( 1 XRP )
		// 1.	Q = -----------------
		//                 XP
		//
		//            Q
		// 2.	R = -----
		//           ITW
		//
		// Constraints:
		// 1. 1 XRP > 0	(one_xrp)
		// 2. XP > 0    (xrp_price)
		// 3. ITW > 0   (input_tx_weight)
		// 4. OTF > 0   (output_tx_fee)
		// 5. Q > 0     (quotient)
		//

		// Constraints
		ensure!(one_xrp > 0, OneXRPCannotBeZero);
		ensure!(xrp_price > 0, XRPPriceCannotBeZero);
		ensure!(input_tx_weight > 0, InputTxWeightCannotBeZero);
		ensure!(output_tx_fee > 0, OutputTxFeeCannotBeZero);

		// Formula
		let quotient = output_tx_fee.saturating_mul(one_xrp).saturating_div(xrp_price);
		let input_tx_weight = Balance::from(input_tx_weight);

		ensure!(quotient > 0, WeightMultiplierQuotientCannotBeZero);
		ensure!(input_tx_weight >= quotient, OneWeightCannotBeWorthMoreThanOneXRP);

		Ok(Perbill::from_rational(quotient, input_tx_weight))
	}

	pub fn length_multiplier(
		one_xrp: Balance,
		xrp_price: Balance,
		output_len_fee: Balance,
	) -> Result<DecimalBalance, CalculatorErrors> {
		//
		// General Formula:
		//
		// 1.	V = OLF * ( 1 XRP )
		//
		//            V
		// 2.   I = -----
		//           XP
		//
		// 3. 	Rem = V % XP
		// 4.	R = (I, Rem)
		//
		// Constraints:
		// 1. 1 XRP > 0	(one_xrp)
		// 2. XP > 0	(xrp_price)
		// 3. OLF > 0	(output_len_fee)
		//

		// Constraints
		ensure!(one_xrp > 0, OneXRPCannotBeZero);
		ensure!(xrp_price > 0, XRPPriceCannotBeZero);
		ensure!(output_len_fee > 0, OutputLenFeeCannotBeZero);

		let value = output_len_fee.saturating_mul(one_xrp);
		let integer = value.saturating_div(xrp_price);
		let remainder = value.saturating_sub(xrp_price.saturating_mul(integer));

		ensure!(xrp_price >= remainder, LengthMultiplierCalculationError);

		let decimal = Perbill::from_rational(remainder, xrp_price);

		Ok(DecimalBalance::new(integer, decimal))
	}
	pub fn evm_base_fee(
		one_xrp: Balance,
		xrp_price: Balance,
		evm_xrp_scale_factor: Balance,
		input_gas_limit: U256,
		output_tx_fee: Balance,
	) -> Result<U256, CalculatorErrors> {
		//
		// General Formula:
		//
		//           OTF * XSF * ( 1 XRP )
		// 1.	Q = -----------------------
		//                    XP
		//
		//            Q
		// 2.	R = -----
		//           IGL
		//
		// Constraints:
		// 1. 1 XRP > 0	(one_xrp)
		// 2. XP > 0	(xrp_price)
		// 3. IGL > 0	(input_gas_limit)
		// 4. XSF > 0	(evm_xrp_scale_factor)
		// 5. OTF > 0	(output_tx_fee)
		// 6. Q > 0     (quotient)
		//

		// Constraints
		ensure!(one_xrp > 0, OneXRPCannotBeZero);
		ensure!(xrp_price > 0, XRPPriceCannotBeZero);
		ensure!(evm_xrp_scale_factor > 0, EvmXrpScaleFactorCannotBeZero);
		ensure!(input_gas_limit > U256::zero(), InputGasLimitCannotBeZero);
		ensure!(output_tx_fee > 0, OutputTxFeeCannotBeZero);

		let quotient = output_tx_fee
			.saturating_mul(evm_xrp_scale_factor)
			.saturating_mul(one_xrp)
			.saturating_div(xrp_price);
		let quotient = U256::from(quotient);

		ensure!(quotient >= input_gas_limit, EvmMultiplierCalculationError);

		Ok(quotient.div(input_gas_limit))
	}
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub struct FeeControlData {
	// Non-Static data. This changes on every update call.
	pub weight_multiplier: Perbill,
	pub length_multiplier: DecimalBalance,
	pub reference_evm_base_fee: U256,
	pub adjusted_evm_base_fee: U256,

	// Semi-static data. This can change on every update call but mostly it doesn't.
	pub input_tx_weight: Weight,
	pub input_gas_limit: U256,

	// Static data. This doesn't change unless we manually change it.
	pub output_tx_fee: Balance,
	pub output_len_fee: Balance,

	// Additional functionality
	pub is_locked: bool,
	pub refresh_data: bool,
}

/// Possible operations on the configuration values of this pallet.
#[derive(TypeInfo, Debug, Clone, Encode, Decode, PartialEq)]
pub enum ConfigOp<T: Codec> {
	/// Don't change.
	Noop,
	/// Set the given value.
	Set(T),
}

impl<T: Codec> ConfigOp<T> {
	pub fn new_or_existing(self, default: T) -> T {
		match self {
			ConfigOp::Noop => default,
			ConfigOp::Set(x) => x,
		}
	}
}

impl<T: Codec> From<T> for ConfigOp<T> {
	fn from(value: T) -> Self {
		ConfigOp::Set(value)
	}
}

#[derive(TypeInfo, Debug, Clone, Encode, Decode, PartialEq, Eq, MaxEncodedLen)]
pub struct DecimalBalance {
	pub integer: Balance,
	pub decimal: Perbill,
}

impl DecimalBalance {
	pub fn new(integer: Balance, decimal: Perbill) -> Self {
		Self { integer, decimal }
	}

	pub fn mul(&self, value: Balance) -> Balance {
		self.integer.mul(value) + self.decimal.mul(value)
	}

	pub fn zero() -> Self {
		Self { integer: Balance::default(), decimal: Perbill::zero() }
	}
}
