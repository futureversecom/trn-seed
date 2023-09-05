// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use seed_pallet_common::FeeConfig;
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

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub struct FeeControlFeeConfig {
	pub evm_base_fee_per_gas: U256,
	pub weight_multiplier: Perbill,
	pub length_multiplier: Balance,
}

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
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Weight Info
		type WeightInfo: WeightInfo;
		/// Default EVM fee values
		type FeeConfig: FeeConfig;
	}

	#[pallet::type_value]
	pub fn DefaultFeeConfig<T: Config>() -> FeeControlFeeConfig {
		FeeControlFeeConfig {
			evm_base_fee_per_gas: T::FeeConfig::evm_base_fee_per_gas(),
			weight_multiplier: T::FeeConfig::weight_multiplier(),
			length_multiplier: T::FeeConfig::length_multiplier(),
		}
	}

	#[pallet::storage]
	pub type Data<T> = StorageValue<_, FeeControlFeeConfig, ValueQuery, DefaultFeeConfig<T>>;

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
		Data::<T>::get().weight_multiplier.mul(weight.ref_time() as Balance)
	}

	pub fn length_to_fee(weight: &Weight) -> Balance {
		Data::<T>::get().length_multiplier.mul(weight.ref_time() as Balance)
	}

	pub fn base_fee_per_gas() -> U256 {
		Data::<T>::get().evm_base_fee_per_gas
	}
}

impl<T: Config> FeeConfig for Pallet<T> {
	fn evm_base_fee_per_gas() -> U256 {
		Self::base_fee_per_gas()
	}

	fn weight_multiplier() -> Perbill {
		Data::<T>::get().weight_multiplier
	}

	fn length_multiplier() -> Balance {
		Data::<T>::get().length_multiplier
	}
}

impl<T: Config> fp_evm::FeeCalculator for Pallet<T> {
	fn min_gas_price() -> (U256, Weight) {
		(Self::base_fee_per_gas(), T::DbWeight::get().reads(1))
	}
}
