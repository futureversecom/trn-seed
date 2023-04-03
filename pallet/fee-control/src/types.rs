use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use seed_primitives::Balance;
use sp_core::U256;
use sp_runtime::Perbill;

pub trait DefaultValues {
	fn evm_base_fee_per_gas() -> U256;
	fn weight_multiplier() -> Perbill;
	fn length_multiplier() -> Balance;
}

// This is for tests
#[cfg(test)]
impl DefaultValues for () {
	fn evm_base_fee_per_gas() -> U256 {
		U256::from(15_000_000_000_000u128)
	}

	fn weight_multiplier() -> Perbill {
		Perbill::from_parts(125)
	}

	fn length_multiplier() -> Balance {
		Balance::from(0u32)
	}
}
