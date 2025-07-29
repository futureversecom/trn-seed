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
use pallet_transaction_payment::Multiplier;
use sp_runtime::FixedPointNumber;

use core::ops::Mul;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod weights;

pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub struct FeeControlFeeConfig {
	pub evm_base_fee_per_gas: U256,
	pub weight_multiplier: u32,
	pub length_multiplier: Balance,
	pub minimum_multiplier: Multiplier,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	#[pallet::pallet]
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
			weight_multiplier: T::FeeConfig::weight_multiplier().deconstruct(),
			length_multiplier: T::FeeConfig::length_multiplier(),
			minimum_multiplier: T::FeeConfig::minimum_multiplier(),
		}
	}

	#[pallet::storage]
	pub type Data<T> = StorageValue<_, FeeControlFeeConfig, ValueQuery, DefaultFeeConfig<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T> {
		/// The EVM base fee has been set to `base_fee`
		EvmBaseFeeSet { base_fee: U256 },
		/// The weight multiplier has been set to `weight_multiplier`
		WeightMultiplierSet { weight_multiplier: u32 },
		/// The length multiplier has been set to `length_multiplier`
		LengthMultiplierSet { length_multiplier: Balance },
		/// The minimum fee multiplier has been set to `minimum_multiplier`
		MinimumMultiplierSet { numerator: u128, minimum_multiplier: Multiplier },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The Weight multiplier is invalid, it must be less than or equal to 1_000_000_000
		InvalidWeightMultiplier,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_evm_base_fee())]
		pub fn set_evm_base_fee(origin: OriginFor<T>, value: U256) -> DispatchResult {
			ensure_root(origin)?;
			Data::<T>::mutate(|x| {
				x.evm_base_fee_per_gas = value;
			});

			Self::deposit_event(Event::<T>::EvmBaseFeeSet { base_fee: value });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_weight_multiplier())]
		pub fn set_weight_multiplier(origin: OriginFor<T>, value: u32) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(value <= 1_000_000_000, Error::<T>::InvalidWeightMultiplier);
			Data::<T>::mutate(|x| {
				x.weight_multiplier = value;
			});

			Self::deposit_event(Event::<T>::WeightMultiplierSet { weight_multiplier: value });
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::set_weight_multiplier())]
		pub fn set_length_multiplier(origin: OriginFor<T>, value: Balance) -> DispatchResult {
			ensure_root(origin)?;
			Data::<T>::mutate(|x| {
				x.length_multiplier = value;
			});

			Self::deposit_event(Event::<T>::LengthMultiplierSet { length_multiplier: value });
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::set_minimum_multiplier())]
		pub fn set_minimum_multiplier(origin: OriginFor<T>, numerator: u128) -> DispatchResult {
			ensure_root(origin)?;
			let minimum_multiplier = Multiplier::saturating_from_rational(numerator, 1_000_000_000u128);
			Data::<T>::mutate(|x| {
				x.minimum_multiplier = minimum_multiplier;
			});

			Self::deposit_event(Event::<T>::MinimumMultiplierSet { numerator, minimum_multiplier });
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn weight_to_fee(weight: &Weight) -> Balance {
		Perbill::from_parts(Data::<T>::get().weight_multiplier).mul(weight.ref_time() as Balance)
	}

	pub fn length_to_fee(weight: &Weight) -> Balance {
		Data::<T>::get().length_multiplier.mul(weight.ref_time() as Balance)
	}

	pub fn base_fee_per_gas() -> U256 {
		Data::<T>::get().evm_base_fee_per_gas
	}

	pub fn minimum_multiplier() -> Multiplier {
		Data::<T>::get().minimum_multiplier
	}
}

impl<T: Config> FeeConfig for Pallet<T> {
	fn evm_base_fee_per_gas() -> U256 {
		Self::base_fee_per_gas()
	}

	fn weight_multiplier() -> Perbill {
		Perbill::from_parts(Data::<T>::get().weight_multiplier)
	}

	fn length_multiplier() -> Balance {
		Data::<T>::get().length_multiplier
	}

	fn minimum_multiplier() -> Multiplier {
		Data::<T>::get().minimum_multiplier
	}
}

impl<T: Config> fp_evm::FeeCalculator for Pallet<T> {
	fn min_gas_price() -> (U256, Weight) {
		(Self::base_fee_per_gas(), T::DbWeight::get().reads(1))
	}
}
