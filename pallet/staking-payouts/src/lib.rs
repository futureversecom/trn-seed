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

use frame_election_provider_support::SortedListProvider;
use frame_support::{
	pallet_prelude::*,
	sp_runtime::{traits::One, SaturatedConversion},
	storage::{generator::StorageMap, unhashed},
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

#[frame_support::pallet]
pub mod pallet {
	use frame_support::storage::hashed;
	use pallet_staking::UseNominatorsAndValidatorsMap;

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

		/// Interface to access weight values
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	pub type CurrentValidatorIterRawKey<T: Config> = StorageValue<_, Vec<u8>, OptionQuery>;

	#[pallet::storage]
	pub type CurrentValidatorIter<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

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
				// Previous era information is static, compared to current era which may change.
				// Thus it's safe to query over the period of the current era
				// let previous_era = active_era_info.index - 1;
				let previous_era = active_era_info.index.saturating_sub(1);

				// Iteration control over multiple blocks. We can only iterate one validator per
				// block.
				let validator = {
					// If already started iterating through for current  era
					if let Some(current_validator_i) = CurrentValidatorIter::<T>::get() {
						UseNominatorsAndValidatorsMap::<T>::iter_from(&current_validator_i)
							// TODO: Unwrap
							.unwrap()
							.next()
					} else {
						// Not started; need to get first validator
						UseNominatorsAndValidatorsMap::<T>::iter().next()
					}
				};

				CurrentValidatorIter::<T>::set(validator);

				if let Some(validator) = validator {
					// Try perform queries
					pallet_staking::ErasStakers::<T>::get(previous_era, validator);
					pallet_staking::ErasStakersClipped::<T>::get(previous_era, validator);
					pallet_staking::ErasValidatorReward::<T>::get(previous_era);
					pallet_staking::ErasRewardPoints::<T>::get(previous_era);
					pallet_staking::ErasTotalStake::<T>::get(previous_era);
					pallet_staking::Pallet::<T>::history_depth();
					pallet_staking::CurrentEra::<T>::get();
					pallet_staking::Bonded::<T>::get(validator);
					pallet_staking::Ledger::<T>::get(validator);
				}
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
