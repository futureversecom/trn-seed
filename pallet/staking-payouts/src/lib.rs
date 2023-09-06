// // Copyright 2022-2023 Futureverse Corporation Limited
// //
// // Licensed under the Apache License, Version 2.0 (the "License");
// // you may not use this file except in compliance with the License.
// // You may obtain a copy of the License at
// //
// //     http://www.apache.org/licenses/LICENSE-2.0
// //
// // Unless required by applicable law or agreed to in writing, software
// // distributed under the License is distributed on an "AS IS" BASIS,
// // WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// // See the License for the specific language governing permissions and
// // limitations under the License.
// // You may obtain a copy of the License at the root of this project source code

// #![cfg_attr(not(feature = "std"), no_std)]

// pub use pallet::*;

// use frame_system::pallet_prelude::*;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;
// #[cfg(test)]
// mod mock;
// #[cfg(test)]
// mod test;
// mod weights;

// pub use weights::WeightInfo;

// #[frame_support::pallet]
// pub mod pallet {
// 	use super::*;
// 	use frame_support::{
// 		pallet_prelude::*,
// 		traits::{Currency, Imbalance, OnUnbalanced},
// 	};
// 	use pallet_staking::{ActiveEra, WeightInfo};
// 	use seed_primitives::AccountId;
// 	use sp_runtime::{
// 		traits::{Saturating, Zero},
// 		Perbill,
// 	};
// 	use sp_std::{boxed::Box, vec, vec::Vec};

// 	#[pallet::pallet]
// 	#[pallet::generate_store(pub (super) trait Store)]
// 	// TODO: REMOVE
// 	#[pallet::without_storage_info]
// 	pub struct Pallet<T>(_);
// 	use sp_staking::EraIndex;
// 	// When thinking about the currency type, consider that this is a custom Currency implementation
// 	// for multiple assets
// 	pub type BalanceOf<T> = <T as pallet_staking::Config>::CurrencyBalance;
// 	type PositiveImbalanceOf<T> = <<T as pallet_staking::Config>::Currency as Currency<
// 		<T as frame_system::Config>::AccountId,
// 	>>::PositiveImbalance;

// 	#[pallet::config]
// 	pub trait Config: frame_system::Config<AccountId = AccountId> + pallet_staking::Config {
// 		/// The system event type
// 		type Event: From<Event> + IsType<<Self as frame_system::Config>::Event>;

// 		type CurrencyBalance: sp_runtime::traits::AtLeast32BitUnsigned
// 			+ codec::FullCodec
// 			+ Copy
// 			+ MaybeSerializeDeserialize
// 			+ sp_std::fmt::Debug
// 			+ Default
// 			+ From<u64>
// 			+ TypeInfo
// 			+ MaxEncodedLen;
// 		type Currency: Currency<Self::AccountId>;
// 		type PayoutPeriodLength: Get<u32>;
// 		type WeightInfo: WeightInfo;
// 	}

// 	#[derive(Debug, PartialEq, Clone, Encode, Decode, TypeInfo, Eq)]
// 	#[scale_info(skip_type_params(T))]
// 	/// Accumulated payout information, not specific to any token
// 	/// TODO: Need to get staking information to ensure that this represents balance queries on the
// 	/// index token
// 	pub struct AccumulatedPayoutInfo<T: pallet_staking::Config> {
// 		/// Payout for this validator
// 		pub payout_amount: T::CurrencyBalance,
// 		/// List of nominators nominating this validator, and their payouts
// 		// nominators: Vec<(T::AccountId, u128)>
// 		pub nominators: Vec<(T::AccountId, T::CurrencyBalance)>,
// 	}

// 	impl<T: pallet_staking::Config> AccumulatedPayoutInfo<T> {
// 		pub fn new(
// 			payout_amount: T::CurrencyBalance,
// 			nominators: Vec<(T::AccountId, T::CurrencyBalance)>,
// 		) -> Self {
// 			AccumulatedPayoutInfo { payout_amount, nominators }
// 		}
// 	}

// 	/// Unique identifier for payout periods
// 	pub type PayoutPeriodId = u128;

// 	#[pallet::storage]
// 	/// Storage for tracking a validator id solely for iterating through the validator list
// 	/// block-by-block
// 	pub type CurrentValidatorIter<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

// 	#[pallet::storage]
// 	pub type CurrentPayoutPeriod<T: Config> = StorageValue<_, PayoutPeriodId, ValueQuery>;

// 	#[pallet::storage]
// 	/// Eras which were already processed
// 	pub type ProcessedEras<T: Config> = StorageMap<_, Blake2_128Concat, EraIndex, bool,
// 	ValueQuery>;

// 	#[pallet::storage]
// 	pub type AccumulatedRewardsList<T: Config> = StorageDoubleMap<
// 		_,
// 		Identity,
// 		// Validator id
// 		T::AccountId,
// 		Blake2_128Concat,
// 		// Current payout period
// 		PayoutPeriodId,
// 		// This validator's payout, and the list of payouts for its nominators
// 		AccumulatedPayoutInfo<T>,
// 		OptionQuery,
// 	>;

// 	#[pallet::event]
// 	#[pallet::generate_deposit(pub(super) fn deposit_event)]
// 	pub enum Event {
// 		OnInitializeErr(Vec<u8>),
// 	}

// 	#[pallet::error]
// 	#[derive(Clone)]
// 	pub enum Error<T> {
// 		NoValidatorToIterate
// 	}

// 	// Control iteration through validators. This is just `UseNominatorsAndValidatorsMap` of
// 	// pallet_staking, but without nominators
// 	pub struct UseValidatorsMap<T>(sp_std::marker::PhantomData<T>);
// 	impl<T: Config> UseValidatorsMap<T> {
// 		fn iter() -> Box<dyn Iterator<Item = T::AccountId>> {
// 			Box::new(pallet_staking::Validators::<T>::iter().map(|(v, _)| v))
// 		}
// 		fn iter_from(
// 			start: &T::AccountId,
// 		) -> Result<Box<dyn Iterator<Item = T::AccountId>>, Error<T>> {
// 			if pallet_staking::Validators::<T>::contains_key(start) {
// 				let start_key = pallet_staking::Validators::<T>::hashed_key_for(start);
// 				Ok(Box::new(pallet_staking::Validators::<T>::iter_from(start_key).map(|(n, _)| n)))
// 			} else {
// 				// Err(())
// 				Err(Error::<T>::NoValidatorToIterate)
// 			}
// 		}
// 	}

// 	#[pallet::hooks]
// 	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
// 		fn on_initialize(now: T::BlockNumber) -> u64 {
// 			// TODO: Decide between active and current
// 			let active_era = pallet_staking::ActiveEra::<T>::get();
// 			let consumed_weight = 0;

// 			if let Some(active_era) = active_era {
// 				// Previous era information is static, compared to current era which may change.
// 				// Thus it's safe to query over the period of the current era

// 				// Cannot check previous era if there are none
// 				if active_era.index == 0 {
// 					return consumed_weight
// 				};
// 				let previous_era = active_era.index.saturating_sub(1);

// 				// If already processed, no need to re-do work
// 				if ProcessedEras::<T>::get(previous_era) {
// 					return consumed_weight
// 				};

// 				// Iteration control over multiple blocks. We can only iterate one validator per
// 				// block.
// 				let validator = {
// 					// If already started iterating through for current  era
// 					if let Some(current_validator_i) = CurrentValidatorIter::<T>::get() {
// 						UseValidatorsMap::<T>::iter_from(&current_validator_i)
// 							// TODO: Unwrap
// 							.unwrap()
// 							.next()
// 					} else {
// 						// Not started; need to get first validator
// 						UseValidatorsMap::<T>::iter().next()
// 					}
// 				};

// 				CurrentValidatorIter::<T>::set(validator);

// 				if let Some(validator) = validator {
// 					let mut payout_period = CurrentPayoutPeriod::<T>::get();

// 					// We need to increment payout period as we go
// 					if active_era.index % T::PayoutPeriodLength::get() == 0 {
// 						payout_period = payout_period.saturating_add(1);
// 						CurrentPayoutPeriod::<T>::set(payout_period);
// 					}

// 					Self::accumulate_payouts(validator, previous_era)
// 						.map_err(|e| Self::deposit_event(Event::OnInitializeErr(e.encode())));
// 				}
// 			}

// 			consumed_weight
// 		}
// 	}

// 	impl<T: Config> Pallet<T> {}
// }
