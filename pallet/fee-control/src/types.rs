use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::U256;
use sp_runtime::Perbill;

pub trait DefaultValues {
	fn evm_base_fee_per_gas() -> U256;
	fn weight_to_fee_reduction() -> Perbill;
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub struct PalletData {
	pub evm_base_fee_per_gas: U256,
	pub weight_to_fee_reduction: Perbill,
}
