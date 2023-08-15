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

use frame_support::{
	pallet_prelude::*,
	sp_runtime::{traits::One, SaturatedConversion},
	PalletId,
};
use frame_system::pallet_prelude::*;
use seed_primitives::AccountId;
use sp_core::H160;
use sp_std::prelude::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;
mod weights;

pub use weights::WeightInfo;

// Value used to show that the origin of the ping is from this pallet
pub const PING: u8 = 0;
// Value used to show that the origin of the ping is from Ethereum
pub const PONG: u8 = 1;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	// TODO: REMOVE
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> + pallet_staking::Config {
		/// The system event type
		type Event: From<Event> + IsType<<Self as frame_system::Config>::Event>;
		/// This pallet's Id, used for deriving a sovereign account ID
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Interface to access weight values
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	pub type CurrentValidatorIterRawKey<T: Config> = StorageValue<_, Vec<u8>, OptionQuery>;

	#[pallet::storage]
	pub type PayoutPeriod<T: Config> = StorageValue<_, u128, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event {
		DidThing,
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> u64 {
			let active_era = pallet_staking::ActiveEra::<T>::get();
			let mut consumed_weight = 0;

			if let Some(active_era_info) = active_era {
				// 	// Previous era information is static, compared to current era which may change.
				// 	// Thus it's safe to query over the period of the current era
				let previous_era = active_era_info.index - 1;

				// 	// Iteration control over multiple blocks. We can only iterate one validator per
				// 	// block.
				let current_validator_iter = {
					if let Some(current_validator_i) = CurrentValidatorIterRawKey::<T>::get() {
						let validator = pallet_staking::ErasValidatorReward::<T>::iter_keys_from(
							current_validator_i,
						)
						.next()
						.unwrap();
						pallet_staking::ErasValidatorReward::<T>::hashed_key_for(validator)
					} else {
						// Not started; need to get first validator
						let current_validator_index =
							pallet_staking::ErasValidatorReward::<T>::iter_keys().next().unwrap();

						let raw_key = pallet_staking::ErasValidatorReward::<T>::hashed_key_for(
							current_validator_index,
						);
						CurrentValidatorIterRawKey::<T>::set(Some(raw_key.clone()));
						raw_key
						// log::info!("Current validator: {:?}", current_validator_i);
					}
				};

				// pallet_staking::ErasStakers::<T>::get(previous_era, current_validator_iter);
				// pallet_staking::ErasStakersClipped::<T>::get();
				// pallet_staking::ErasValidatorReward::<T>::get();
				// pallet_staking::ErasRewardPoints::<T>::get();
				// pallet_staking::ErasTotalStake::<T>::get();
				// pallet_staking::HistoryDepth::<T>::get();
				// pallet_staking::CurrentEra::<T>::get();
				// pallet_staking::Bonded::<T>::get();
				// pallet_staking::Ledger::<T>::get();
			}

			consumed_weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10000000)]
		pub fn payout_stakers(origin: OriginFor<T>, payout_period_id: u128) -> DispatchResult {
			Ok(())
		}
	}
}
