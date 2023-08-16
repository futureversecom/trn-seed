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
	traits::{Currency, Imbalance, OnUnbalanced},
};
use frame_system::pallet_prelude::*;
use seed_primitives::AccountId;
use sp_runtime::{
	traits::{Saturating, Zero},
	Perbill,
};
use sp_staking::EraIndex;
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
	use super::*;
	use pallet_staking::{UseNominatorsAndValidatorsMap, WeightInfo};

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	// TODO: REMOVE
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	// When thinking about the currency type, consider that this is a custom Currency implementation
	// for multiple assets
	pub type BalanceOf<T> = <T as pallet_staking::Config>::CurrencyBalance;
	type PositiveImbalanceOf<T> = <<T as pallet_staking::Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::PositiveImbalance;
	type NegativeImbalanceOf<T> = <<T as pallet_staking::Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::NegativeImbalance;

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
		// type Currency: LockableCurrency<
		// 	Self::AccountId,
		// 	Moment = Self::BlockNumber,
		// 	Balance = Self::CurrencyBalance,
		// >;

		type Currency: Currency<Self::AccountId>;

		/// Interface to access weight values
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	pub type CurrentValidatorIterRawKey<T: Config> = StorageValue<_, Vec<u8>, OptionQuery>;

	#[pallet::storage]
	pub type CurrentValidatorIter<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pallet::storage]
	pub type PayoutPeriod<T: Config> = StorageValue<_, u128, ValueQuery>;

	#[pallet::storage]
	pub type AccumulatedRewards<T: Config> = StorageDoubleMap<
		_,
		Identity,
		T::AccountId,
		Blake2_128Concat,
		EraIndex,
		BalanceOf<T>,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event {}

	#[pallet::error]
	pub enum Error<T> {
		AlreadyClaimed,
		InvalidEraToReward,
		NotController,
		NotStash,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> u64 {
			let active_era = pallet_staking::ActiveEra::<T>::get();
			let consumed_weight = 0;

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
					let payout_period = PayoutPeriod::<T>::get();

					Self::do_payout_stakers(validator, payout_period, previous_era);
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

	impl<T: Config> Pallet<T> {
		// Same logic as pallet_stakers payout, except stores and accumulates the payouts
		pub(super) fn do_payout_stakers(
			validator_stash: T::AccountId,
			// era: EraIndex,
			payout_period: u128,
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

			// Input data seems good, no errors allowed after this point

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
			if let Some(imbalance) = Self::accumulate_payouts(
				&ledger.stash,
				validator_staking_payout + validator_commission_payout,
				&previous_era,
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
				if let Some(imbalance) =
					Self::accumulate_payouts(&nominator.who, nominator_reward, &previous_era)
				{
					// Note: this logic does not count payouts for `RewardDestination::None`.
					nominator_payout_count += 1;
					total_imbalance.subsume(imbalance);
				}
			}

			T::Reward::on_unbalanced(total_imbalance);
			debug_assert!(nominator_payout_count <= T::MaxNominatorRewardedPerValidator::get());
			Ok(Some(<T as Config>::WeightInfo::payout_stakers_alive_staked(nominator_payout_count))
				.into())
		}

		// Store the current payouts, accumulating any payouts for the account from the previous era
		fn accumulate_payouts(
			who: &T::AccountId,
			amt: <T as pallet_staking::Config>::CurrencyBalance,
			previous_era: &EraIndex,
		) -> Option<PositiveImbalanceOf<T>> {
			// TODO: Clean up unwrap
			let current_era = pallet_staking::CurrentEra::<T>::get().unwrap();

			if let Some(previous_payout) = AccumulatedRewards::<T>::get(who, previous_era) {
				let payout = amt.saturating_add(previous_payout);
				AccumulatedRewards::<T>::insert(&who, current_era, payout);
			} else {
				AccumulatedRewards::<T>::insert(&who, current_era, amt);
			};

			// TODO: change
			None
		}
	}
}
