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

use codec::{Decode, Encode, MaxEncodedLen};
use core::ops::{Div, Mul};
use frame_support::{ensure, weights::Weight};
use scale_info::TypeInfo;
use seed_primitives::Balance;
use sp_core::U256;
use sp_runtime::Perbill;

pub trait DefaultValues {
	fn evm_base_fee_per_gas() -> U256;
	fn weight_multiplier() -> Perbill;
	fn length_multiplier() -> LengthMultiplier;
	fn transaction_weight() -> Weight;
	fn gas_limit() -> U256;
	fn desired_transaction_fee() -> Balance;
	fn desired_length_fee() -> Balance;
	fn evm_xrp_scale_factor() -> Balance;
}

// This is for tests
#[cfg(test)]
impl DefaultValues for () {
	fn evm_base_fee_per_gas() -> U256 {
		U256::from(9_523_809_523_809u128)
	}

	fn weight_multiplier() -> Perbill {
		Perbill::from_parts(125)
	}

	fn length_multiplier() -> LengthMultiplier {
		LengthMultiplier::new(0)
	}

	fn transaction_weight() -> Weight {
		Weight::from(100_000_000u64)
	}

	fn gas_limit() -> U256 {
		U256::from(10_000_000u128)
	}

	fn desired_transaction_fee() -> Balance {
		Balance::from(100_000u64)
	}

	fn desired_length_fee() -> Balance {
		Balance::from(1u64)
	}

	fn evm_xrp_scale_factor() -> Balance {
		Balance::from(1_000u64)
	}
}

#[derive(TypeInfo, Debug, Copy, Clone, Encode, Decode, PartialEq)]
pub enum CalculationsErrors {
	InputParameterCannotBeZero = 1,
	WeightMultiplierError = 2,
	LengthMultiplierError = 3,
	EvmBaseFeeError = 4,
}
use CalculationsErrors::*;

#[derive(TypeInfo, Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, MaxEncodedLen)]
pub struct LengthMultiplier {
	pub multiplier: Balance,
	pub scaling_factor: u32,
}

impl LengthMultiplier {
	pub const fn new(multiplier: Balance) -> Self {
		Self { multiplier, scaling_factor: 1 }
	}

	pub fn mul(&self, value: Balance) -> Balance {
		value.mul(self.multiplier).div(self.scaling_factor as u128)
	}
}

impl Default for LengthMultiplier {
	fn default() -> Self {
		Self { multiplier: 0, scaling_factor: 1 }
	}
}

pub struct Calculations {
	pub one_xrp: Balance,
	pub xrp_price: Balance,
	pub tx_weight: Weight,
	pub tx_fee: Balance,
	pub len_fee: Balance,
	pub evm_xrp_scale_factor: Balance,
	pub gas_limit: U256,
}
pub struct CalculationsResults {
	pub weight_multiplier: Perbill,
	pub length_multiplier: LengthMultiplier,
	pub evm_base_fee_per_gas: U256,
}

impl Calculations {
	pub fn calculate(&self) -> Result<CalculationsResults, CalculationsErrors> {
		ensure!(self.one_xrp > 0, InputParameterCannotBeZero);
		ensure!(self.xrp_price > 0, InputParameterCannotBeZero);
		ensure!(self.tx_weight > 0, InputParameterCannotBeZero);
		ensure!(self.evm_xrp_scale_factor > 0, InputParameterCannotBeZero);
		ensure!(self.gas_limit > U256::zero(), InputParameterCannotBeZero);

		let weight_multiplier = Self::weight_multiplier(self)?;
		let length_multiplier = Self::length_multiplier(self)?;
		let evm_base_fee_per_gas = Self::evm_base_fee(self)?;

		Ok(CalculationsResults { weight_multiplier, length_multiplier, evm_base_fee_per_gas })
	}

	fn weight_multiplier(&self) -> Result<Perbill, CalculationsErrors> {
		if self.tx_fee == 0 {
			return Ok(Perbill::zero())
		}

		let quotient = self.tx_fee.saturating_mul(self.one_xrp).saturating_div(self.xrp_price);
		let tx_weight = Balance::from(self.tx_weight);

		ensure!(quotient > 0, WeightMultiplierError);
		ensure!(tx_weight >= quotient, WeightMultiplierError);

		Ok(Perbill::from_rational(quotient, tx_weight))
	}

	fn length_multiplier(&self) -> Result<LengthMultiplier, CalculationsErrors> {
		if self.len_fee == 0 {
			return Ok(LengthMultiplier::new(0))
		}

		let value = self.len_fee.saturating_mul(self.one_xrp);
		let scaling_factor = 1_000u32;
		let expanded_value = value.mul(scaling_factor as u128);
		let multiplier = expanded_value.saturating_div(self.xrp_price);

		ensure!(multiplier > 0, LengthMultiplierError);

		Ok(LengthMultiplier { multiplier, scaling_factor })
	}

	fn evm_base_fee(&self) -> Result<U256, CalculationsErrors> {
		if self.tx_fee == 0 {
			return Ok(U256::zero())
		}

		let quotient = self
			.tx_fee
			.saturating_mul(self.evm_xrp_scale_factor)
			.saturating_mul(self.one_xrp)
			.saturating_div(self.xrp_price);
		let quotient = U256::from(quotient);

		ensure!(quotient >= self.gas_limit, EvmBaseFeeError);

		Ok(quotient.div(self.gas_limit))
	}
}
