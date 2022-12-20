#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

use frame_support::{pallet_prelude::*, weights::WeightToFee};
use frame_system::pallet_prelude::*;

use seed_primitives::Balance;

use sp_core::U256;
use sp_runtime::Perbill;

use core::ops::Mul;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;

mod weights;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);
	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type DefaultEvmBaseFeePerGas: Get<U256>;
		type WeightToFeeReduction: Get<Perbill>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::type_value]
	pub fn DefaultEvmBaseFeePerGas<T: Config>() -> U256 {
		T::DefaultEvmBaseFeePerGas::get()
	}

	#[pallet::type_value]
	pub fn DefaultWeightToFeeReduction<T: Config>() -> Perbill {
		T::WeightToFeeReduction::get()
	}

	#[pallet::storage]
	#[pallet::getter(fn base_fee_per_gas)]
	pub type EvmBaseFeePerGas<T> = StorageValue<_, U256, ValueQuery, DefaultEvmBaseFeePerGas<T>>;

	#[pallet::storage]
	#[pallet::getter(fn extrinsic_weight_to_fee)]
	pub type ExtrinsicWeightToFee<T> =
		StorageValue<_, Perbill, ValueQuery, DefaultWeightToFeeReduction<T>>;

	#[pallet::event]
	pub enum Event<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::set_evm_base_fee())]
		pub fn set_evm_base_fee(origin: OriginFor<T>, value: U256) -> DispatchResult {
			ensure_root(origin)?;
			<EvmBaseFeePerGas<T>>::put(value);
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::set_extrinsic_weight_to_fee_factor())]
		pub fn set_extrinsic_weight_to_fee_factor(
			origin: OriginFor<T>,
			value: Perbill,
		) -> DispatchResult {
			ensure_root(origin)?;
			ExtrinsicWeightToFee::<T>::put(value);
			Ok(())
		}
	}

	// Substrate extrinsics fee control
	impl<T> WeightToFee for Pallet<T>
	where
		T: Config,
	{
		type Balance = Balance;
		fn weight_to_fee(weight: &Weight) -> Balance {
			Self::extrinsic_weight_to_fee().mul(*weight as Balance)
		}
	}
}

impl<T: Config> fp_evm::FeeCalculator for Pallet<T> {
	fn min_gas_price() -> (U256, Weight) {
		(<EvmBaseFeePerGas<T>>::get(), T::DbWeight::get().reads(1))
	}
}
