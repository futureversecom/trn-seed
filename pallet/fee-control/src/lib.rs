#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

use frame_support::pallet_prelude::*;
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
pub use types::*;

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
		/// Weight Info
		type WeightInfo: WeightInfo;
		/// Default values
		type DefaultValues: DefaultValues;
	}

	#[pallet::type_value]
	pub fn DefaultPalletData<T: Config>() -> PalletData {
		PalletData {
			evm_base_fee_per_gas: T::DefaultValues::evm_base_fee_per_gas(),
			weight_multiplier: T::DefaultValues::weight_multiplier(),
			length_multiplier: T::DefaultValues::length_multiplier(),
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

		#[pallet::weight(T::WeightInfo::set_weight_multiplier())]
		pub fn set_weight_multiplier(origin: OriginFor<T>, value: Perbill) -> DispatchResult {
			ensure_root(origin)?;
			Data::<T>::mutate(|x| {
				x.weight_multiplier = value;
			});

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn weight_to_fee(weight: &Weight) -> Balance {
		Data::<T>::get().weight_multiplier.mul(*weight as Balance)
	}

	pub fn length_to_fee(weight: &Weight) -> Balance {
		Data::<T>::get().length_multiplier.mul(*weight as Balance)
	}

	pub fn base_fee_per_gas() -> U256 {
		Data::<T>::get().evm_base_fee_per_gas
	}
}

impl<T: Config> fp_evm::FeeCalculator for Pallet<T> {
	fn min_gas_price() -> (U256, Weight) {
		(Self::base_fee_per_gas(), T::DbWeight::get().reads(1))
	}
}
