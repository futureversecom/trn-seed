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

#![cfg_attr(not(feature = "std"), no_std)]

use core::ops::Mul;
use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use seed_primitives::Balance;
use sp_core::U256;
use sp_runtime::Perbill;

pub use pallet::*;
pub use types::*;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;
pub mod types;
mod weights;

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub struct FeeConfig {
	pub evm_base_fee_per_gas: U256,
	pub weight_multiplier: Perbill,
	pub length_multiplier: LengthMultiplier,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

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
	pub fn DefaultFeeConfig<T: Config>() -> FeeConfig {
		FeeConfig {
			evm_base_fee_per_gas: T::DefaultValues::evm_base_fee_per_gas(),
			weight_multiplier: T::DefaultValues::weight_multiplier(),
			length_multiplier: T::DefaultValues::length_multiplier(),
		}
	}

	#[pallet::storage]
	pub type Data<T> = StorageValue<_, FeeConfig, ValueQuery, DefaultFeeConfig<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T> {
		XrpPriceUpdate {
			xrp_price: Balance,
			weight_multiplier: Perbill,
			length_multiplier: LengthMultiplier,
			evm_base_fee_per_gas: U256,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		InputParameterCannotBeZero,
		WeightMultiplierError,
		LengthMultiplierError,
		EvmBaseFeeError,
	}

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

		#[pallet::weight(T::WeightInfo::set_length_multiplier())]
		pub fn set_length_multiplier(
			origin: OriginFor<T>,
			value: LengthMultiplier,
		) -> DispatchResult {
			ensure_root(origin)?;
			Data::<T>::mutate(|x| {
				x.length_multiplier = value;
			});

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::set_xrp_price())]
		#[transactional]
		pub fn set_xrp_price(
			origin: OriginFor<T>,
			#[pallet::compact] xrp_price: Balance,
		) -> DispatchResult {
			ensure_root(origin)?;

			Data::<T>::try_mutate(|x| -> DispatchResult {
				let calc = Calculations {
					one_xrp: Balance::from(1_000_000u128),
					xrp_price,
					tx_weight: T::DefaultValues::transaction_weight(),
					tx_fee: T::DefaultValues::desired_transaction_fee(),
					len_fee: T::DefaultValues::desired_length_fee(),
					evm_xrp_scale_factor: T::DefaultValues::evm_xrp_scale_factor(),
					gas_limit: T::DefaultValues::gas_limit(),
				};
				let CalculationsResults {
					weight_multiplier,
					length_multiplier,
					evm_base_fee_per_gas,
				} = calc.calculate().map_err(|e| Error::<T>::from(e))?;

				*x = FeeConfig { evm_base_fee_per_gas, weight_multiplier, length_multiplier };

				Self::deposit_event(Event::<T>::XrpPriceUpdate {
					xrp_price,
					weight_multiplier,
					length_multiplier,
					evm_base_fee_per_gas,
				});

				Ok(())
			})?;

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

impl<T: Config> From<CalculationsErrors> for Error<T> {
	fn from(val: CalculationsErrors) -> Error<T> {
		use CalculationsErrors::*;
		match val {
			InputParameterCannotBeZero => Error::<T>::InputParameterCannotBeZero,
			WeightMultiplierError => Error::<T>::WeightMultiplierError,
			LengthMultiplierError => Error::<T>::LengthMultiplierError,
			EvmBaseFeeError => Error::<T>::EvmBaseFeeError,
		}
	}
}
