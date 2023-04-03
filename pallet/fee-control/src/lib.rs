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
pub mod types;
use types::*;

mod weights;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Default values
		type DefaultValues: DefaultValues;
		/// Weight Info
		type WeightInfo: WeightInfo;
	}

	#[pallet::type_value]
	pub fn DefaultPalletData<T: Config>() -> PalletData {
		PalletData {
			evm_base_fee_per_gas: T::DefaultValues::evm_base_fee_per_gas(),
			weight_to_fee_reduction: T::DefaultValues::weight_to_fee_reduction(),
		}
	}

	#[pallet::storage]
	pub type Data<T> = StorageValue<_, PalletData, ValueQuery, DefaultPalletData<T>>;

	#[pallet::event]
	pub enum Event<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::set_evm_base_fee())]
		pub fn set_evm_base_fee(origin: OriginFor<T>, value: U256) -> DispatchResult {
			ensure_root(origin)?;
			Data::<T>::mutate(|x| {
				x.evm_base_fee_per_gas = value;
			});

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::set_extrinsic_weight_to_fee_factor())]
		pub fn set_extrinsic_weight_to_fee_factor(
			origin: OriginFor<T>,
			value: Perbill,
		) -> DispatchResult {
			ensure_root(origin)?;
			Data::<T>::mutate(|x| {
				x.weight_to_fee_reduction = value;
			});

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn weight_to_fee(weight: &Weight) -> Balance {
		Data::<T>::get().weight_to_fee_reduction.mul(*weight as Balance)
	}

	pub fn base_fee_per_gas() -> U256 {
		Data::<T>::get().evm_base_fee_per_gas
	}
}

impl<T: Config> fp_evm::FeeCalculator for Pallet<T> {
	fn min_gas_price() -> (U256, Weight) {
		(Data::<T>::get().evm_base_fee_per_gas, T::DbWeight::get().reads(1))
	}
}
