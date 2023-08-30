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

use frame_system::pallet_prelude::*;

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
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, Imbalance, OnUnbalanced},
	};
	use pallet_staking::{ActiveEra, WeightInfo};
	use seed_primitives::AccountId;
	use sp_runtime::{
		traits::{Saturating, Zero},
		Perbill,
	};
	use sp_std::{boxed::Box, vec, vec::Vec};

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	// TODO: REMOVE
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);
	use sp_staking::EraIndex;
	// When thinking about the currency type, consider that this is a custom Currency implementation
	// for multiple assets
	pub type BalanceOf<T> = <T as pallet_staking::Config>::CurrencyBalance;
	type PositiveImbalanceOf<T> = <<T as pallet_staking::Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::PositiveImbalance;

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> + pallet_staking::Config {
		/// The system event type
		type Event: From<Event> + IsType<<Self as frame_system::Config>::Event>;

		type CurrencyBalance: sp_runtime::traits::AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ sp_std::fmt::Debug
			+ Default
			+ From<u64>
			+ TypeInfo
			+ MaxEncodedLen;
		type Currency: Currency<Self::AccountId>;
		type PayoutPeriodLength: Get<u32>;
		type WeightInfo: WeightInfo;
	}

	#[derive(Debug, PartialEq, Clone, Encode, Decode, TypeInfo, Eq)]
	#[scale_info(skip_type_params(T))]
	/// Accumulated payout information, not specific to any token
	/// TODO: Need to get staking information to ensure that this represents balance queries on the
	/// index token
	pub struct AccumulatedPayoutInfo<T: pallet_staking::Config> {
		/// Payout for this validator
		pub payout_amount: T::CurrencyBalance,
		/// List of nominators nominating this validator, and their payouts
		// nominators: Vec<(T::AccountId, u128)>
		pub nominators: Vec<(T::AccountId, T::CurrencyBalance)>,
	}

	impl<T: pallet_staking::Config> AccumulatedPayoutInfo<T> {
		pub fn new(
			payout_amount: T::CurrencyBalance,
			nominators: Vec<(T::AccountId, T::CurrencyBalance)>,
		) -> Self {
			AccumulatedPayoutInfo { payout_amount, nominators }
		}
	}

	/// Unique identifier for payout periods
	pub type PayoutPeriodId = u128;

	#[pallet::storage]
	/// Storage for tracking a validator id solely for iterating through the validator list
	/// block-by-block
	pub type CurrentValidatorIter<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pallet::storage]
	pub type CurrentPayoutPeriod<T: Config> = StorageValue<_, PayoutPeriodId, ValueQuery>;

	#[pallet::storage]
	/// Eras which were already processed
	pub type ProcessedEras<T: Config> = StorageMap<_, Blake2_128Concat, EraIndex, bool, ValueQuery>;

	#[pallet::storage]
	pub type AccumulatedRewardsList<T: Config> = StorageDoubleMap<
		_,
		Identity,
		// Validator id
		T::AccountId,
		Blake2_128Concat,
		// Current payout period
		PayoutPeriodId,
		// This validator's payout, and the list of payouts for its nominators
		AccumulatedPayoutInfo<T>,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event {
		OnInitializeErr(Vec<u8>),
	}

	#[pallet::error]
	#[derive(Clone)]
	pub enum Error<T> {
		AlreadyClaimed,
		InvalidEraToReward,
		NotController,
		NotStash,
		NoValidatorToIterate,
		TooEarly,
	}

	// Control iteration through validators. This is just `UseNominatorsAndValidatorsMap` of
	// pallet_staking, but without nominators
	pub struct UseValidatorsMap<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> UseValidatorsMap<T> {
		fn iter() -> Box<dyn Iterator<Item = T::AccountId>> {
			Box::new(pallet_staking::Validators::<T>::iter().map(|(v, _)| v))
		}
		fn iter_from(
			start: &T::AccountId,
		) -> Result<Box<dyn Iterator<Item = T::AccountId>>, Error<T>> {
			if pallet_staking::Validators::<T>::contains_key(start) {
				let start_key = pallet_staking::Validators::<T>::hashed_key_for(start);
				Ok(Box::new(pallet_staking::Validators::<T>::iter_from(start_key).map(|(n, _)| n)))
			} else {
				// Err(())
				Err(Error::<T>::NoValidatorToIterate)
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> u64 {
			// TODO: Decide between active and current
			let active_era = pallet_staking::ActiveEra::<T>::get();
			let consumed_weight = 0;

			if let Some(active_era) = active_era {
				// Previous era information is static, compared to current era which may change.
				// Thus it's safe to query over the period of the current era

				// Cannot check previous era if there are none
				if active_era.index == 0 {
					return consumed_weight
				};
				let previous_era = active_era.index.saturating_sub(1);

				// If already processed, no need to re-do work
				if ProcessedEras::<T>::get(previous_era) {
					return consumed_weight
				};

				// Iteration control over multiple blocks. We can only iterate one validator per
				// block.
				let validator = {
					// If already started iterating through for current  era
					if let Some(current_validator_i) = CurrentValidatorIter::<T>::get() {
						UseValidatorsMap::<T>::iter_from(&current_validator_i)
							// TODO: Unwrap
							.unwrap()
							.next()
					} else {
						// Not started; need to get first validator
						UseValidatorsMap::<T>::iter().next()
					}
				};

				CurrentValidatorIter::<T>::set(validator);

				if let Some(validator) = validator {
					let mut payout_period = CurrentPayoutPeriod::<T>::get();

					// We need to increment payout period as we go
					if active_era.index % T::PayoutPeriodLength::get() == 0 {
						payout_period = payout_period.saturating_add(1);
						CurrentPayoutPeriod::<T>::set(payout_period);
					}

					Self::accumulate_payouts(validator, previous_era)
						.map_err(|e| Self::deposit_event(Event::OnInitializeErr(e.encode())));
				}
			}

			consumed_weight
		}
	}

	impl<T: Config> Pallet<T> {
		// Get the accumulated data for a validator and its nominators for the given payout period,
		// also clearing any data associated with that period
		pub fn take_accumulated_payouts_staker(
			validator_stash: T::AccountId,
			payout_period: u128,
		) -> Result<AccumulatedPayoutInfo<T>, Error<T>> {
			ensure!(CurrentPayoutPeriod::<T>::get() >= payout_period, Error::<T>::TooEarly);
			let rewards = AccumulatedRewardsList::<T>::take(validator_stash, payout_period);

			// TODO: check any remaining values to remove

			// Likely already claimed. Maybe edge case for unacummulated rewards
			rewards.ok_or(Error::<T>::AlreadyClaimed)
		}

		// Same logic as pallet_stakers payout, except stores and accumulates the payouts
		pub fn accumulate_payouts(
			validator_stash: T::AccountId,
			previous_era: EraIndex,
		) -> DispatchResultWithPostInfo {
			let history_depth = pallet_staking::Pallet::<T>::history_depth();

			// Note: if era has no reward to be claimed, era may be future. better not to update
			// `ledger.claimed_rewards` in this case.
			let era_payout = pallet_staking::ErasValidatorReward::<T>::get(previous_era)
				// TODO: :Determine weight
				// .ok_or_else(|| Error::<T>::InvalidEraToReward.with_weight(100000))?;
				.ok_or_else(|| Error::<T>::InvalidEraToReward)?;

			let controller =
				pallet_staking::Bonded::<T>::get(&validator_stash).ok_or_else(|| {
					// Error::<T>::NotStash.
					// with_weight(::WeightInfo::payout_stakers_alive_staked(0))
					Error::<T>::NotStash
				})?;
			let mut ledger =
				pallet_staking::Ledger::<T>::get(&controller).ok_or(Error::<T>::NotController)?;

			// Check this one
			ledger
				.claimed_rewards
				// TODO: Is previous era okay here?
				.retain(|&x| x >= previous_era.saturating_sub(history_depth));

			// Is previous era okay here?
			match ledger.claimed_rewards.binary_search(&previous_era) {
				Ok(_) =>
				// return Err(Error::<T>::AlreadyClaimed
				// 	.with_weight(<T as Config>::WeightInfo::payout_stakers_alive_staked(0))),
					return Err(Error::<T>::AlreadyClaimed.into()),
				Err(pos) => ledger.claimed_rewards.insert(pos, previous_era),
			}

			let exposure =
				pallet_staking::ErasStakersClipped::<T>::get(&previous_era, &ledger.stash);

			// Check this one
			pallet_staking::Ledger::<T>::insert(&controller, &ledger);

			// Get Era reward points. It has TOTAL and INDIVIDUAL
			// Find the fraction of the era reward that belongs to the validator
			// Take that fraction of the eras rewards to split to nominator and validator
			//
			// Then look at the validator, figure out the proportion of their reward
			// which goes to them and each of their nominators.
			let era_reward_points = pallet_staking::ErasRewardPoints::<T>::get(&previous_era);
			let total_reward_points = era_reward_points.total;

			let validator_reward_points = era_reward_points
				.individual
				.get(&ledger.stash)
				.copied()
				.unwrap_or_else(Zero::zero);

			// Nothing to do if they have no reward points.
			if validator_reward_points.is_zero() {
				return Ok(Some(<T as Config>::WeightInfo::payout_stakers_alive_staked(0)).into())
			}

			// This is the fraction of the total reward that the validator and the
			// nominators will get.
			let validator_total_reward_part =
				Perbill::from_rational(validator_reward_points, total_reward_points);

			// This is how much validator + nominators are entitled to.
			let validator_total_payout = validator_total_reward_part * era_payout;

			let validator_prefs =
				pallet_staking::Pallet::<T>::eras_validator_prefs(&previous_era, &validator_stash);
			// Validator first gets a cut off the top.
			let validator_commission = validator_prefs.commission;
			let validator_commission_payout = validator_commission * validator_total_payout;

			let validator_leftover_payout = validator_total_payout - validator_commission_payout;
			// Now let's calculate how this is split to the validator.
			let validator_exposure_part = Perbill::from_rational(exposure.own, exposure.total);
			let validator_staking_payout = validator_exposure_part * validator_leftover_payout;

			let mut total_imbalance = PositiveImbalanceOf::<T>::zero();

			// We can now make total validator payout:
			if let Some(imbalance) = Self::do_accumulate_payouts(
				&ledger.stash,
				validator_staking_payout + validator_commission_payout,
				None,
			) {
				total_imbalance.subsume(imbalance);
			}

			// Track the number of payout ops to nominators. Note:
			// `WeightInfo::payout_stakers_alive_staked` always assumes at least a validator is paid
			// out, so we do not need to count their payout op.
			let mut nominator_payout_count: u32 = 0;

			// Lets now calculate how this is split to the nominators.
			// Reward only the clipped exposures. Note this is not necessarily sorted.
			for nominator in exposure.others.iter() {
				let nominator_exposure_part =
					Perbill::from_rational(nominator.value, exposure.total);

				let nominator_reward: BalanceOf<T> =
					nominator_exposure_part * validator_leftover_payout;
				// We can now make nominator payout:
				if let Some(imbalance) = Self::do_accumulate_payouts(
					&nominator.who,
					nominator_reward,
					Some(&ledger.stash),
				) {
					// Note: this logic does not count payouts for `RewardDestination::None`.
					nominator_payout_count += 1;
					total_imbalance.subsume(imbalance);
				}
			}

			T::Reward::on_unbalanced(total_imbalance);
			debug_assert!(nominator_payout_count <= T::MaxNominatorRewardedPerValidator::get());
			ProcessedEras::<T>::insert(previous_era, true);
			Ok(Some(<T as Config>::WeightInfo::payout_stakers_alive_staked(nominator_payout_count))
				.into())
		}

		// Store the current payouts, accumulating any payouts for the account from the previous era
		fn do_accumulate_payouts(
			who: &T::AccountId,
			amt: <T as pallet_staking::Config>::CurrencyBalance,
			nominating: Option<&T::AccountId>,
		) -> Option<PositiveImbalanceOf<T>> {
			// TODO: Clean up unwrap
			let payout_period = CurrentPayoutPeriod::<T>::get();

			// If we are nominating, we must work with the payouts tracking according to the
			// validator we are nominating
			let account_to_check = nominating.unwrap_or(who);

			// If there is a payout existing, we need to accumulate any existing accumulated payouts
			// for previous eras

			if let Some(current_payout) =
				AccumulatedRewardsList::<T>::get(account_to_check, payout_period)
			{
				// If we are a nominator, add us to the list of nominator accumulated rewards
				if nominating.is_some() {
					// TODO: needs to be .find()
					current_payout.nominators.iter().map(|w| {
						if w.0 == *who {
							w.1.saturating_add(amt);
						}
						w
					});

					AccumulatedRewardsList::<T>::insert(
						account_to_check,
						payout_period,
						current_payout,
					);
				} else {
					// Else, we are a validator and simply add the accumulation
					current_payout.payout_amount.saturating_add(amt);
					AccumulatedRewardsList::<T>::insert(
						account_to_check,
						payout_period,
						current_payout,
					);
				}
			} else {
				// Else, we are initializing the payouts for the payout period
				let nominators = if nominating.is_some() { vec![(*who, amt)] } else { vec![] };
				let payout_info = AccumulatedPayoutInfo { payout_amount: amt, nominators };
				AccumulatedRewardsList::<T>::insert(account_to_check, payout_period, payout_info);
			};

			// TODO: change. Need to get imbalance
			None
		}
	}
}
