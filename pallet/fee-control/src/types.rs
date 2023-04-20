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

use seed_primitives::Balance;
use sp_core::U256;
use sp_runtime::Perbill;

pub trait DefaultValues {
	fn evm_base_fee_per_gas() -> U256;
	fn weight_multiplier() -> Perbill;
	fn length_multiplier() -> Balance;
}

impl DefaultValues for () {
	fn evm_base_fee_per_gas() -> U256 {
		// Floor network base fee per gas
		// 0.000015 XRP per gas, 15000 GWEI
		U256::from(15_000_000_000_000u128)
	}
	fn weight_multiplier() -> Perbill {
		Perbill::from_parts(125)
	}
	fn length_multiplier() -> Balance {
		Balance::from(0u32)
	}
}
