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
extern crate alloc;

pub use pallet::*;

use crate::alloc::vec::Vec;
use frame_support::{
	log,
	pallet_prelude::*,
	sp_runtime::traits::One,
	traits::{
		fungibles::{metadata::Inspect as InspectMetadata, Inspect, Mutate},
		tokens::Preservation,
	},
	transactional, BoundedVec, PalletId,
};
use frame_system::{
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::*,
};
use seed_primitives::{AccountId, AssetId, Balance};
use sp_arithmetic::helpers_128bit::multiply_by_rational_with_rounding;
use sp_io::hashing::blake2_256;
use sp_runtime::{
	traits::{
		AccountIdConversion, AtLeast32BitUnsigned, CheckedAdd, SaturatedConversion, Saturating,
		ValidateUnsigned, Zero,
	},
	transaction_validity::{
		InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity,
		ValidTransaction,
	},
};

#[cfg(feature = "runtime-benchmarks")]
use seed_pallet_common::CreateExt;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod types;
use types::*;

pub mod weights;
pub use weights::WeightInfo;

/// The logging target for this pallet
#[allow(dead_code)]
pub(crate) const LOG_TARGET: &str = "liquidity-pools";

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config<AccountId = AccountId> + SendTransactionTypes<Call<Self>>
	{
		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// This pallet's Id, used for deriving a sovereign account ID
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// PoolId is used to distinguish between different pools that manage and facilitate
		/// the attendance and rewarding of assets.
		type PoolId: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen;

		/// Unsigned transaction interval
		#[pallet::constant]
		type UnsignedInterval: Get<BlockNumberFor<Self>>;

		/// Max number of users to rollover per block
		#[pallet::constant]
		type RolloverBatchSize: Get<u32>;

		/// Max number of users to process per closure batch (FRN-68)
		#[pallet::constant]
		type ClosureBatchSize: Get<u32>;

		/// Max pivot string length
		type MaxStringLength: Get<u32>;

		/// Interest rate for 100% base point. Use the base point to calculate actual interest rate.
		/// e.g. when 10000 is base points, and interest rate is 1000 when creating pool, then the
		/// actual interest rate is 10% (1000 / 10000).
		#[pallet::constant]
		type InterestRateBasePoint: Get<u32>;

		/// Assets pallet
		#[cfg(not(feature = "runtime-benchmarks"))]
		type MultiCurrency: Inspect<Self::AccountId, AssetId = AssetId>
			+ InspectMetadata<Self::AccountId>
			+ Mutate<Self::AccountId, Balance = Balance>;

		/// Assets pallet - for benchmarking to manipulate assets
		#[cfg(feature = "runtime-benchmarks")]
		type MultiCurrency: Inspect<Self::AccountId, AssetId = AssetId>
			+ InspectMetadata<Self::AccountId>
			+ Mutate<Self::AccountId, Balance = Balance>
			+ CreateExt<AccountId = Self::AccountId>;

		/// Interface to access weight values
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	pub(super) type Pools<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::PoolId,
		PoolInfo<T::PoolId, AssetId, Balance, BlockNumberFor<T>>,
	>;

	#[pallet::storage]
	pub(super) type PoolUsers<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::PoolId, Twox64Concat, T::AccountId, UserInfo<Balance>>;

	#[pallet::storage]
	pub type PoolRelationships<T: Config> =
		StorageMap<_, Twox64Concat, T::PoolId, PoolRelationship<T::PoolId>>;

	#[pallet::storage]
	pub(super) type NextPoolId<T: Config> = StorageValue<_, T::PoolId, ValueQuery>;

	#[pallet::storage]
	pub(super) type NextRolloverUnsignedAt<T: Config> =
		StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::storage]
	pub(super) type RolloverPivot<T: Config> =
		StorageMap<_, Twox64Concat, T::PoolId, BoundedVec<u8, T::MaxStringLength>, ValueQuery>;

	// FRN-68: New storage items for bounded pool closure
	#[pallet::storage]
	pub(super) type ClosingPools<T: Config> =
		StorageMap<_, Twox64Concat, T::PoolId, ClosureState<T::PoolId>>;

	// FRN-69: Storage for idle processing state
	#[pallet::storage]
	pub(super) type IdleProcessingStatus<T: Config> =
		StorageValue<_, IdleProcessingState<T::PoolId>, ValueQuery>;

	// FRN-71: Storage for fair processing state
	#[pallet::storage]
	pub(super) type ProcessingStatus<T: Config> =
		StorageValue<_, ProcessingState<T::PoolId>, ValueQuery>;

	// FRN-71: Priority queue for urgent pool updates
	#[pallet::storage]
	pub(super) type UrgentPoolUpdates<T: Config> = StorageMap<_, Twox64Concat, T::PoolId, ()>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Reward pool created, user could join pool
		PoolOpen {
			pool_id: T::PoolId,
			reward_asset_id: AssetId,
			staked_asset_id: AssetId,
			interest_rate: u32,
			max_tokens: Balance,
			lock_start_block: BlockNumberFor<T>,
			lock_end_block: BlockNumberFor<T>,
		},
		/// Pool starts to lock.
		PoolStarted { pool_id: T::PoolId },
		/// Pool starts to rollover users that want to continue to next pool.
		PoolRenewing { pool_id: T::PoolId },
		/// Pool rollover is done and ready for users to claim rewards.
		PoolMatured { pool_id: T::PoolId },
		/// Pool closed, no more users can join.
		PoolClosed {
			pool_id: T::PoolId,
			reward_asset_amount: Balance,
			staked_asset_amount: Balance,
			receiver: T::AccountId,
		},
		/// Set pool successor, when predecessor pool is done, users will be rolled over to
		/// successor pool.
		SetSuccession { predecessor_pool_id: T::PoolId, successor_pool_id: T::PoolId },
		/// User info updated, currently only rollover preference is updated.
		UserInfoUpdated { pool_id: T::PoolId, account_id: T::AccountId, should_rollover: bool },
		/// User joined pool.
		UserJoined { account_id: T::AccountId, pool_id: T::PoolId, amount: Balance },
		/// User exited pool.
		UserExited { account_id: T::AccountId, pool_id: T::PoolId, amount: Balance },
		/// User rolled over to its successor pool.
		UserRolledOver {
			account_id: T::AccountId,
			pool_id: T::PoolId,
			rolled_to_pool_id: T::PoolId,
			amount: Balance,
		},
		/// Rewards claimed.
		RewardsClaimed { account_id: T::AccountId, pool_id: T::PoolId, amount: Balance },

		// FRN-68: Pool closure events
		/// Pool closure initiated
		PoolClosureInitiated { pool_id: T::PoolId, closure_type: ClosureType },
		/// Pool closure batch processed
		PoolClosureBatchProcessed { pool_id: T::PoolId, users_processed: u32, remaining_users: u32 },
		/// Pool closure completed
		PoolClosureCompleted { pool_id: T::PoolId },

		// FRN-71: Pool processing events
		/// Pool update triggered manually
		PoolUpdateTriggered { pool_id: T::PoolId },
		/// Pool added to urgent processing queue
		PoolAddedToUrgentQueue { pool_id: T::PoolId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Not pool creator
		NotPoolCreator,
		/// Invalid block range
		InvalidBlockRange,
		/// Pool already exists
		PoolAlreadyExists,
		/// Pool does not exist
		PoolDoesNotExist,
		/// Successor pool does not exist
		SuccessorPoolDoesNotExist,
		/// Predecessor pool does not exist
		PredecessorPoolDoesNotExist,
		/// Successor pool size should be greater than predecessor
		SuccessorPoolSizeShouldBeGreaterThanPredecessor,
		/// Successor pool size should be locked after predecessor
		SuccessorPoolSizeShouldBeLockedAfterPredecessor,
		/// Rollover pools should be the same asset
		RolloverPoolsShouldBeTheSameAsset,
		/// Cannot exit pool, no tokens staked
		NoTokensStaked,
		/// Reward pool is not open
		PoolNotOpen,
		/// Reward pool is not ready for reward
		NotReadyForClaimingReward,
		/// Exceeds max pool id
		NoAvailablePoolId,
		/// Staking limit exceeded
		StakingLimitExceeded,
		/// Offchain error not a validator
		OffchainErrNotValidator,
		/// Offchain error too early
		OffchainErrTooEarly,
		/// Offchain error on submitting transaction
		OffchainErrSubmitTransaction,
		/// Offchain error wrong transaction source
		OffchainErrWrongTransactionSource,
		/// Pivot string too long
		PivotStringTooLong,
		/// Reward calculation overflow
		RewardCalculationOverflow,
		/// Cannot close pool with active user stakes
		CannotClosePoolWithActiveUsers,

		// FRN-68: Pool closure errors
		/// Pool not in closing state
		PoolNotClosing,
		/// No closure batch to process
		NoClosureBatchToProcess,
		/// Pool closure already in progress
		PoolClosureAlreadyInProgress,

		// FRN-70: Unsigned transaction validation errors
		/// Invalid transaction source
		InvalidTransactionSource,
		/// Transaction timing validation failed
		TransactionTimingValidationFailed,
		/// Pool state validation failed for unsigned transaction
		PoolStateValidationFailed,
		/// System state validation failed for unsigned transaction
		SystemStateValidationFailed,

		// FRN-71: Fair processing errors
		/// Pool not eligible for urgent processing
		PoolNotEligibleForUrgentProcessing,
		/// Processing state corrupted
		ProcessingStateCorrupted,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates a liquidity pool for a specified asset over a range of future blocks.
		///
		/// The pool created by this function allows users to stake specified assets and,
		/// depending on the pool's performance, receive rewards in the reward asset.
		///
		/// Parameters:
		/// - `origin`: The origin account that is creating the pool.
		/// - `asset_id`: The ID of the asset for which the pool is being created.
		/// - `interest_rate`: The interest rate for the pool, dictating the reward distribution.
		/// - `max_tokens`: The maximum amount of tokens that can be staked in this pool.
		/// - `lock_start_block`: The starting block number from which the pool will begin to lock.
		///   After it the pool will no longer accept stakes.
		/// - `lock_end_block`: The ending block number after which the pool will be able to claim
		///   rewards.
		///
		/// Restrictions:
		/// - The `lock_start_block` must be in the future and less than `lock_end_block`.
		/// - The `lock_end_block` must be greater than the `lock_start_block`.
		///
		/// Emits `PoolCreated` event when successful.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_pool())]
		#[transactional]
		pub fn create_pool(
			origin: OriginFor<T>,
			reward_asset_id: AssetId,
			staked_asset_id: AssetId,
			interest_rate: u32,
			max_tokens: Balance,
			lock_start_block: BlockNumberFor<T>,
			lock_end_block: BlockNumberFor<T>,
		) -> DispatchResult {
			let creator = ensure_signed(origin)?;

			let id = NextPoolId::<T>::mutate(|id| -> Result<T::PoolId, DispatchError> {
				let current_id = *id;
				*id = id.checked_add(&One::one()).ok_or(Error::<T>::NoAvailablePoolId)?;
				Ok(current_id)
			})?;

			ensure!(
				lock_start_block > frame_system::Pallet::<T>::block_number(),
				Error::<T>::InvalidBlockRange
			);
			ensure!(lock_start_block < lock_end_block, Error::<T>::InvalidBlockRange);

			let max_rewards = Self::calculate_reward(
				max_tokens,
				0,
				interest_rate,
				T::InterestRateBasePoint::get(),
				T::MultiCurrency::decimals(staked_asset_id),
				T::MultiCurrency::decimals(reward_asset_id),
			)?;

			// Transfer max rewards to pool vault account
			if max_rewards > 0 {
				T::MultiCurrency::transfer(
					reward_asset_id,
					&creator,
					&Self::get_vault_account(id),
					max_rewards,
					Preservation::Expendable,
				)?;
			}

			let pool_info = PoolInfo {
				id,
				creator,
				reward_asset_id,
				staked_asset_id,
				interest_rate,
				max_tokens,
				last_updated: frame_system::Pallet::<T>::block_number(),
				lock_start_block,
				lock_end_block,
				locked_amount: Zero::zero(),
				pool_status: PoolStatus::Open,
			};
			Pools::<T>::insert(id, pool_info);

			Self::deposit_event(Event::PoolOpen {
				pool_id: id,
				reward_asset_id,
				staked_asset_id,
				interest_rate,
				max_tokens,
				lock_start_block,
				lock_end_block,
			});
			Ok(())
		}

		/// Sets up a successor relationship between two pools.
		///
		/// This function allows admin users to link pools in a sequential manner, so when one pool
		/// ends, users who are set to rollover can automatically join the successor pool to
		/// continue receiving rewards.
		///
		/// Parameters:
		/// - `origin`: The origin account setting the pool succession. Must be an admin.
		/// - `predecessor_pool_id`: The ID of the predecessor pool that will link to a successor
		///   upon completion.
		/// - `successor_pool_id`: The ID of the successor pool that will continue from where the
		///   predecessor pool ends.
		///
		/// Restrictions:
		/// - Both the predecessor and successor pools must exist.
		/// - The `max_tokens` of the successor pool should be greater than or equal to the
		///   predecessor pool.
		///
		/// Emits `SetSuccession` event when successful.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_pool_succession())]
		pub fn set_pool_succession(
			origin: OriginFor<T>,
			predecessor_pool_id: T::PoolId,
			successor_pool_id: T::PoolId,
		) -> DispatchResult {
			let creator = ensure_signed(origin)?;

			// Check that predecessor exists
			let predecessor_pool = Pools::<T>::get(predecessor_pool_id)
				.ok_or(Error::<T>::PredecessorPoolDoesNotExist)?;
			ensure!(predecessor_pool.creator == creator, Error::<T>::NotPoolCreator);

			// Check that successor exists
			let successor_pool =
				Pools::<T>::get(successor_pool_id).ok_or(Error::<T>::SuccessorPoolDoesNotExist)?;
			ensure!(successor_pool.creator == creator, Error::<T>::NotPoolCreator);

			// Check successor max_tokens is greater than predecessor max_tokens
			ensure!(
				successor_pool.max_tokens >= predecessor_pool.max_tokens,
				Error::<T>::SuccessorPoolSizeShouldBeGreaterThanPredecessor
			);
			ensure!(
				successor_pool.lock_start_block > predecessor_pool.lock_end_block,
				Error::<T>::SuccessorPoolSizeShouldBeLockedAfterPredecessor
			);
			ensure!(
				successor_pool.staked_asset_id == predecessor_pool.staked_asset_id,
				Error::<T>::RolloverPoolsShouldBeTheSameAsset
			);

			<PoolRelationships<T>>::insert(
				&predecessor_pool_id,
				PoolRelationship { successor_id: Some(successor_pool_id) },
			);

			Self::deposit_event(Event::SetSuccession { predecessor_pool_id, successor_pool_id });

			Ok(())
		}

		/// Updates a user's preference for rolling over to the next pool.
		///
		/// Users can choose to automatically transfer their stake to a successor pool
		/// once the current pool ends, allowing for continuous reward accumulation without manual
		/// intervention.
		///
		/// Parameters:
		/// - `origin`: The account of the user setting the rollover preference.
		/// - `id`: The ID of the pool for which the rollover preference is being set.
		/// - `should_rollover`: A boolean indicating whether the user's stake should rollover to a
		///   successor pool.
		///
		/// Restrictions:
		/// - The pool must be in the `Open` status, not yet active or closed.
		///
		/// Emits `UserInfoUpdated` event if the rollover preference is successfully updated.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::set_pool_rollover())]
		pub fn set_pool_rollover(
			origin: OriginFor<T>,
			id: T::PoolId,
			should_rollover: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let pool = Pools::<T>::get(id).ok_or(Error::<T>::PoolDoesNotExist)?;

			ensure!(pool.pool_status == PoolStatus::Open, Error::<T>::PoolNotOpen);

			PoolUsers::<T>::try_mutate(id, who, |pool_user| -> DispatchResult {
				let pool_user = pool_user.as_mut().ok_or(Error::<T>::NoTokensStaked)?;
				pool_user.should_rollover = should_rollover;
				Ok(())
			})?;

			Self::deposit_event(Event::UserInfoUpdated {
				pool_id: id,
				account_id: who,
				should_rollover,
			});

			Ok(())
		}

		/// FRN-68: Initiates bounded closure of a pool with deferred cleanup.
		///
		/// This function starts the pool closure process using bounded processing
		/// to handle pools with many users safely without exceeding weight limits.
		///
		/// Parameters:
		/// - `origin`: The origin account that is closing the pool. Must be the pool creator.
		/// - `id`: The ID of the pool being closed.
		///
		/// Restrictions:
		/// - The pool identified by `id` must exist.
		/// - Only the pool creator can initiate closure.
		/// - Pool cannot already be in closing state.
		///
		/// Emits `PoolClosureInitiated` event when closure is started.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::close_pool())]
		#[transactional]
		pub fn close_pool(origin: OriginFor<T>, id: T::PoolId) -> DispatchResult {
			let creator = ensure_signed(origin)?;

			let pool = Pools::<T>::get(id).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(pool.creator == creator, Error::<T>::NotPoolCreator);
			ensure!(
				pool.pool_status != PoolStatus::Closing,
				Error::<T>::PoolClosureAlreadyInProgress
			);

			// Count total users in the pool
			let total_users = PoolUsers::<T>::iter_prefix(id).count() as u32;

			if total_users == 0 {
				// No users, can close immediately
				Self::close_empty_pool(id, &pool, creator)?;
			} else {
				// Has users, start bounded closure process
				Self::initiate_bounded_closure(id, total_users, ClosureType::Normal)?;
			}

			Ok(())
		}

		/// Allows a user to join an active reward pool by staking a specified amount of tokens.
		///
		/// By staking, users can earn rewards based on the pool's performance and the amount of
		/// their stake.
		///
		/// Parameters:
		/// - `origin`: The account of the user joining the pool.
		/// - `id`: The ID of the pool the user is joining.
		/// - `amount`: The amount of tokens the user is staking in the pool.
		///
		/// Restrictions:
		/// - The pool must be in the `Open` status and accepting new stakes.
		/// - The total staked amount after the user's stake must not exceed the pool's
		///   `max_tokens`.
		///
		/// Emits `UserJoined` event if the user successfully joins the pool.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::enter_pool())]
		#[transactional]
		pub fn enter_pool(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			amount: Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let vault_account = Self::get_vault_account(pool_id);

			let pool = Pools::<T>::get(pool_id).ok_or(Error::<T>::PoolDoesNotExist)?;

			ensure!(pool.pool_status == PoolStatus::Open, Error::<T>::PoolNotOpen);

			ensure!(
				pool.locked_amount + amount <= pool.max_tokens,
				Error::<T>::StakingLimitExceeded
			);

			T::MultiCurrency::transfer(
				pool.staked_asset_id,
				&who,
				&vault_account,
				amount,
				Preservation::Expendable,
			)?;

			PoolUsers::<T>::mutate(pool_id, who, |pool_user| {
				if let Some(pool_user) = pool_user {
					pool_user.amount = pool_user.amount.saturating_add(amount);
				} else {
					*pool_user = Some(UserInfo { amount, ..UserInfo::default() });
				}
			});

			Pools::<T>::try_mutate(pool_id, |pool_info| -> DispatchResult {
				let pool_info = pool_info.as_mut().ok_or(Error::<T>::PoolDoesNotExist)?;
				pool_info.locked_amount = pool_info.locked_amount.saturating_add(amount);
				Ok(())
			})?;

			Self::deposit_event(Event::UserJoined { account_id: who, pool_id, amount });
			Ok(())
		}

		/// Allows a user to exit from a reward pool, withdrawing their staked tokens.
		///
		/// Parameters:
		/// - `origin`: The account of the user exiting the pool.
		/// - `id`: The ID of the pool from which the user is exiting.
		///
		/// Restrictions:
		/// - The pool must be in the `Open` status and not closed.
		/// - The user must have tokens staked in the pool.
		///
		/// Emits `UserExited` event if the user successfully exits the pool and claims any rewards.
		#[pallet::weight(T::WeightInfo::exit_pool())]
		#[transactional]
		#[pallet::call_index(5)]
		pub fn exit_pool(origin: OriginFor<T>, id: T::PoolId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let pool_vault_account = Self::get_vault_account(id);

			let pool = Pools::<T>::get(id).ok_or(Error::<T>::PoolDoesNotExist)?;

			ensure!(pool.pool_status == PoolStatus::Open, Error::<T>::PoolNotOpen);

			let user_info = PoolUsers::<T>::get(id, &who).ok_or(Error::<T>::NoTokensStaked)?;
			ensure!(user_info.amount > Zero::zero(), Error::<T>::NoTokensStaked);

			let amount = user_info.amount;
			T::MultiCurrency::transfer(
				pool.staked_asset_id,
				&pool_vault_account,
				&who,
				amount,
				Preservation::Expendable,
			)?;

			Pools::<T>::try_mutate(id, |pool_info| -> DispatchResult {
				let pool_info = pool_info.as_mut().ok_or(Error::<T>::PoolDoesNotExist)?;
				pool_info.locked_amount = pool_info.locked_amount.saturating_sub(amount);
				Ok(())
			})?;

			PoolUsers::<T>::remove(id, &who);

			Self::deposit_event(Event::UserExited { account_id: who, pool_id: id, amount });
			Ok(())
		}

		/// Claims the reward for a user from a pool that has ended.
		///
		/// Users who have staked tokens in a pool that has reached the `Matured` status
		/// can call this function to claim their share of the reward.
		///
		/// Parameters:
		/// - `origin`: The account of the user claiming the reward.
		/// - `id`: The ID of the pool from which the reward is being claimed.
		///
		/// Restrictions:
		/// - The pool must be in the `Matured` status.
		/// - The user must have staked tokens in the pool and not already claimed their reward.
		///
		/// Emits `RewardsClaimed` event if the reward is successfully claimed by the user.
		#[pallet::weight(T::WeightInfo::claim_reward())]
		#[transactional]
		#[pallet::call_index(6)]
		pub fn claim_reward(origin: OriginFor<T>, id: T::PoolId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let pool_vault_account = Self::get_vault_account(id);

			let pool = Pools::<T>::get(id).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(pool.pool_status == PoolStatus::Matured, Error::<T>::NotReadyForClaimingReward);

			let user_info = PoolUsers::<T>::get(id, &who).ok_or(Error::<T>::NoTokensStaked)?;
			let reward = Self::calculate_reward(
				user_info.amount,
				user_info.reward_debt,
				pool.interest_rate,
				T::InterestRateBasePoint::get(),
				T::MultiCurrency::decimals(pool.staked_asset_id),
				T::MultiCurrency::decimals(pool.reward_asset_id),
			)?;

			if reward > Zero::zero() {
				let amount = if user_info.should_rollover == false || user_info.rolled_over == false
				{
					T::MultiCurrency::transfer(
						pool.staked_asset_id,
						&pool_vault_account,
						&who,
						user_info.amount,
						Preservation::Expendable,
					)?;
					user_info.amount
				} else {
					Zero::zero()
				};

				// Transfer reward to user
				T::MultiCurrency::transfer(
					pool.reward_asset_id,
					&pool_vault_account,
					&who,
					reward,
					Preservation::Expendable,
				)?;

				Pools::<T>::try_mutate(id, |pool_info| -> DispatchResult {
					let pool_info = pool_info.as_mut().ok_or(Error::<T>::PoolDoesNotExist)?;
					pool_info.last_updated = frame_system::Pallet::<T>::block_number();
					pool_info.locked_amount = pool_info.locked_amount.saturating_sub(amount);
					Ok(())
				})?;
				PoolUsers::<T>::remove(id, &who);
			}

			Self::deposit_event(Event::RewardsClaimed {
				account_id: who,
				pool_id: id,
				amount: reward,
			});
			Ok(())
		}
		/// Emergency recovery function for users to recover their staked funds from closed pools.
		///
		/// This function provides a safety net for users whose funds might be trapped in pools
		/// that have been closed by the creator. Users can recover their original staked amount
		/// even if the pool is in Closed status.
		///
		/// Parameters:
		/// - `origin`: The account of the user recovering their funds.
		/// - `id`: The ID of the pool from which funds are being recovered.
		///
		/// Restrictions:
		/// - The user must have a recorded stake in the pool.
		/// - The pool vault must have sufficient funds to cover the user's stake.
		///
		/// Emits `UserExited` event when funds are successfully recovered.
		#[pallet::weight(T::WeightInfo::exit_pool())]
		#[transactional]
		#[pallet::call_index(8)] // Note: This assumes call_index 8 is available
		pub fn emergency_recover_funds(origin: OriginFor<T>, id: T::PoolId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let pool_vault_account = Self::get_vault_account(id);

			// Get user info - must exist and have staked amount
			let user_info = PoolUsers::<T>::get(id, &who).ok_or(Error::<T>::NoTokensStaked)?;
			ensure!(user_info.amount > Zero::zero(), Error::<T>::NoTokensStaked);

			// Get pool info - allow recovery even from closed pools
			let pool = Pools::<T>::get(id).ok_or(Error::<T>::PoolDoesNotExist)?;

			// Check if vault has sufficient staked assets to cover user's stake
			let vault_staked_balance =
				T::MultiCurrency::balance(pool.staked_asset_id, &pool_vault_account);
			ensure!(vault_staked_balance >= user_info.amount, Error::<T>::NoTokensStaked);

			// Transfer user's staked amount back to them
			T::MultiCurrency::transfer(
				pool.staked_asset_id,
				&pool_vault_account,
				&who,
				user_info.amount,
				Preservation::Expendable,
			)?;

			// Update pool's locked amount if pool still exists
			if pool.pool_status != PoolStatus::Closed {
				Pools::<T>::try_mutate(id, |pool_info| -> DispatchResult {
					let pool_info = pool_info.as_mut().ok_or(Error::<T>::PoolDoesNotExist)?;
					pool_info.locked_amount =
						pool_info.locked_amount.saturating_sub(user_info.amount);
					Ok(())
				})?;
			}

			// Remove user from pool
			PoolUsers::<T>::remove(id, &who);

			Self::deposit_event(Event::UserExited {
				account_id: who,
				pool_id: id,
				amount: user_info.amount,
			});

			Ok(())
		}

		/// Processes the rollover of users from one pool to its successor in an unsigned
		/// transaction.
		///
		/// This function is typically called by off-chain workers to handle the rollover process
		/// automatically. It moves users who have opted in for rollover from the predecessor pool
		/// to the successor pool.
		///
		/// Parameters:
		/// - `origin`: Must be unsigned, indicating that it's an off-chain initiated transaction.
		/// - `id`: The ID of the pool being rolled over.
		/// - `_current_block`: The current block number.
		///
		/// Restrictions:
		/// - The pool must be in the `RollingOver` status.
		/// - There must be a successor pool defined for the rollover to proceed.
		///
		/// Notes:
		/// - This function will emit various events based on the progress of the rollover, such as
		///   `PoolMatured` if the pool is completed.
		#[pallet::weight(T::WeightInfo::rollover_unsigned())]
		#[transactional]
		#[pallet::call_index(7)]
		pub fn rollover_unsigned(
			origin: OriginFor<T>,
			id: T::PoolId,
			_current_block: BlockNumberFor<T>,
		) -> DispatchResult {
			ensure_none(origin)?;

			log::warn!("start processing the rollover");

			let pool_info = Pools::<T>::get(id).ok_or(Error::<T>::PoolDoesNotExist)?;

			if let PoolStatus::Renewing = pool_info.pool_status {
				let pool_vault_account = Self::get_vault_account(id);
				// Check for successor
				let successor_id = PoolRelationships::<T>::get(id).unwrap_or_default().successor_id;

				if let Some(successor_id) = successor_id {
					let successor_pool_info =
						Pools::<T>::get(successor_id).ok_or(Error::<T>::PoolDoesNotExist)?;
					// Migrate users to successor
					let start_key = RolloverPivot::<T>::get(id);
					let payout_pivot: Vec<u8> =
						start_key.clone().try_into().map_err(|_| Error::<T>::PivotStringTooLong)?;

					let mut map_iterator = match RolloverPivot::<T>::contains_key(id) {
						true => <PoolUsers<T>>::iter_prefix_from(id, payout_pivot),
						false => <PoolUsers<T>>::iter_prefix(id),
					};

					let mut count = 0;
					let mut rollover_amount: Balance = Zero::zero();
					let mut predecessor_pool_status = pool_info.pool_status;

					let successor_vault_account = Self::get_vault_account(successor_id);

					let rollover_batch_size = T::RolloverBatchSize::get();

					while let Some((who, user_info)) = map_iterator.next() {
						if user_info.should_migrate() {
							// Check if successor pool has enough space
							// If not, set previous pool to done to prevent further rollover
							if successor_pool_info
								.locked_amount
								.saturating_add(rollover_amount)
								.saturating_add(user_info.amount)
								> successor_pool_info.max_tokens
							{
								predecessor_pool_status = PoolStatus::Matured;
								break;
							} else {
								rollover_amount = rollover_amount.saturating_add(user_info.amount);
							}

							// Update rolled over status of predecessor pool
							PoolUsers::<T>::mutate(id, who, |pool_user| {
								if let Some(pool_user) = pool_user {
									pool_user.rolled_over = true;
								}
							});

							Self::deposit_event(Event::UserRolledOver {
								account_id: who,
								pool_id: id,
								rolled_to_pool_id: successor_id,
								amount: user_info.amount,
							});

							// Update amount of successor pool
							PoolUsers::<T>::mutate(successor_id, who, |pool_user| {
								if let Some(pool_user) = pool_user {
									pool_user.amount =
										pool_user.amount.saturating_add(user_info.amount);
								} else {
									*pool_user = Some(user_info.clone());
								}
							});
						}

						count += 1;
						if count > rollover_batch_size {
							break;
						}
					}
					let current_last_raw_key: BoundedVec<u8, T::MaxStringLength> =
						BoundedVec::try_from(map_iterator.last_raw_key().to_vec())
							.map_err(|_| Error::<T>::PivotStringTooLong)?;
					if current_last_raw_key == start_key.clone() {
						predecessor_pool_status = PoolStatus::Matured;
					}
					RolloverPivot::<T>::insert(id, current_last_raw_key);

					// Transfer rollover amount to successor pool
					T::MultiCurrency::transfer(
						pool_info.staked_asset_id,
						&pool_vault_account,
						&successor_vault_account,
						rollover_amount,
						Preservation::Expendable,
					)?;

					// Update predecessor pool
					Pools::<T>::try_mutate(id, |pool_info| -> DispatchResult {
						let pool_info = pool_info.as_mut().ok_or(Error::<T>::PoolDoesNotExist)?;
						pool_info.last_updated = frame_system::Pallet::<T>::block_number();
						pool_info.locked_amount =
							pool_info.locked_amount.saturating_sub(rollover_amount);
						pool_info.pool_status = predecessor_pool_status;
						Ok(())
					})?;
					if predecessor_pool_status == PoolStatus::Matured {
						Self::deposit_event(Event::PoolMatured { pool_id: id });
					}

					// Update successor pool
					Pools::<T>::try_mutate(successor_id, |pool_info| -> DispatchResult {
						let pool_info = pool_info.as_mut().ok_or(Error::<T>::PoolDoesNotExist)?;
						pool_info.last_updated = frame_system::Pallet::<T>::block_number();
						pool_info.locked_amount =
							pool_info.locked_amount.saturating_add(rollover_amount);
						Ok(())
					})?;
				}
			}

			let current_block = <frame_system::Pallet<T>>::block_number();
			log::warn!("current block {:?}", current_block);
			let next_unsigned_at = current_block + T::UnsignedInterval::get().into();
			<NextRolloverUnsignedAt<T>>::put(next_unsigned_at);
			log::warn!("proposed next unsigned at {:?}", next_unsigned_at);
			Ok(())
		}

		/// FRN-71: Manual trigger for pool status updates with priority processing.
		///
		/// This function allows manual triggering of pool status updates for specific pools,
		/// placing them in the urgent processing queue for immediate attention.
		///
		/// Parameters:
		/// - `origin`: The origin account triggering the update.
		/// - `pool_id`: The ID of the pool to update.
		///
		/// Restrictions:
		/// - The pool must exist.
		/// - Pool must be eligible for status updates.
		///
		/// Emits `PoolUpdateTriggered` event when successful.
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::trigger_pool_update())]
		pub fn trigger_pool_update(origin: OriginFor<T>, pool_id: T::PoolId) -> DispatchResult {
			ensure_signed(origin)?;

			let pool = Pools::<T>::get(pool_id).ok_or(Error::<T>::PoolDoesNotExist)?;

			// Check if pool is eligible for urgent processing
			let current_block = frame_system::Pallet::<T>::block_number();
			let is_eligible = match pool.pool_status {
				PoolStatus::Open if pool.lock_start_block <= current_block => true,
				PoolStatus::Started if pool.lock_end_block <= current_block => true,
				_ => false,
			};

			ensure!(is_eligible, Error::<T>::PoolNotEligibleForUrgentProcessing);

			// Add to urgent processing queue
			UrgentPoolUpdates::<T>::insert(pool_id, ());

			Self::deposit_event(Event::PoolUpdateTriggered { pool_id });
			Self::deposit_event(Event::PoolAddedToUrgentQueue { pool_id });

			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// FRN-69: Rewritten on_idle with proper weight accounting framework
		fn on_idle(now: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			let mut total_weight_used = Weight::zero();
			let base_weight = T::DbWeight::get().reads(1u64);

			// Early exit if we don't have enough weight for basic operations
			if remaining_weight.all_lte(base_weight) {
				return Weight::zero();
			}

			total_weight_used = total_weight_used.saturating_add(base_weight);

			// FRN-71: Process urgent pool updates first
			let urgent_weight = Self::process_urgent_pool_updates(
				now,
				remaining_weight.saturating_sub(total_weight_used),
			);
			total_weight_used = total_weight_used.saturating_add(urgent_weight);

			// FRN-68: Process closing pools with bounded processing
			let closure_weight = Self::process_closing_pools(
				now,
				remaining_weight.saturating_sub(total_weight_used),
			);
			total_weight_used = total_weight_used.saturating_add(closure_weight);

			// FRN-69: Process regular pool status updates with bounded iteration
			let status_weight = Self::process_pool_status_updates(
				now,
				remaining_weight.saturating_sub(total_weight_used),
			);
			total_weight_used = total_weight_used.saturating_add(status_weight);

			// Update processing state
			IdleProcessingStatus::<T>::mutate(|state| {
				state.total_weight_consumed =
					state.total_weight_consumed.saturating_add(total_weight_used.ref_time());
			});

			total_weight_used
		}

		fn offchain_worker(now: BlockNumberFor<T>) {
			match Self::do_offchain_worker(now) {
				Ok(_) => log::debug!(
				  target: "liquidity-pools offchain worker",
				  "🤖 offchain worker start at block: {:?}; done.",
				  now,
				),
				Err(e) => log::error!(
					target: "liquidity-pools offchain worker",
					"⛔️ offchain worker error at block [{:?}]: {:?}",
					now,
					e,
				),
			}
		}
	}

	const UNSIGNED_PRIORITY: TransactionPriority = TransactionPriority::max_value() / 2;

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		/// FRN-70: Enhanced unsigned transaction validation with comprehensive checks
		fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			match call {
				Call::rollover_unsigned { id, current_block } => {
					// Comprehensive validation pipeline
					Self::validate_transaction_source(&source)?;
					Self::validate_timing(current_block)?;
					Self::validate_pool_state(id)?;
					Self::validate_system_state()?;

					// Calculate priority based on pool stakes
					let priority =
						Self::calculate_transaction_priority(id).unwrap_or(UNSIGNED_PRIORITY);

					ValidTransaction::with_tag_prefix("LiquidityPoolsChainWorker")
						.priority(priority)
						.and_provides(id)
						.longevity(64_u64)
						.propagate(true)
						.build()
				},
				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	impl<T: Config> Pallet<T> {
		/// Generate a unique, deterministic vault account for a pool
		pub fn get_vault_account(pool_id: T::PoolId) -> T::AccountId {
			let account_id: T::AccountId = T::PalletId::get().into_account_truncating();
			let entropy = (T::PalletId::get().0, account_id, pool_id).using_encoded(blake2_256);
			T::AccountId::decode(&mut &entropy[..])
				.expect("Created account ID is always valid; qed")
		}

		/// Calculates the reward amount for a user based on the amount staked in an asset,
		/// the interest rate, and the already accumulated reward debt.
		///
		/// This function computes the reward in the asset's token based on the user's staked amount
		/// and the specified interest rate. It then deducts any previously accumulated reward debt
		/// from this amount. Finally, it converts the calculated reward to the equivalent amount in
		/// native tokens, taking into account the difference in decimal places between the asset
		/// token and the native token.
		///
		/// Parameters:
		/// - `user_joined_amount`: The amount of the asset token that the user has staked in the
		///   pool.
		/// - `reward_debt`: The amount of reward that has already been accounted for the user,
		///   which should be subtracted from the newly calculated reward.
		/// - `interest_rate`: The interest rate for the pool, expressed in basis points.
		/// - `interest_rate_base_point`: The divisor used to convert the basis point interest rate
		///   into a proportion.
		/// - `staked_asset_decimals`: The number of decimal places used for the asset token.
		/// - `reward_asset_decimals`: The number of decimal places used for the native token.
		///
		/// Returns:
		/// - `Ok(Balance)`: The calculated reward amount in native tokens, after adjusting for decimal places and
		///   subtracting the reward debt.
		/// - `Err(DispatchError)`: If the reward calculation overflows.
		pub fn calculate_reward(
			user_joined_amount: Balance,
			reward_debt: Balance,
			interest_rate: u32,
			interest_rate_base_point: u32,
			staked_asset_decimals: u8,
			reward_asset_decimals: u8,
		) -> Result<Balance, DispatchError> {
			// Calculate reward in asset token
			let mut reward = multiply_by_rational_with_rounding(
				user_joined_amount,
				interest_rate.into(),
				interest_rate_base_point.into(),
				sp_runtime::Rounding::Down,
			)
			.ok_or(Error::<T>::RewardCalculationOverflow)?;

			// Remaining rewards
			reward = reward.saturating_sub(reward_debt);

			// Convert reward to native token based on decimals
			if staked_asset_decimals > reward_asset_decimals {
				reward = reward.saturating_div(
					10_u128.pow((staked_asset_decimals - reward_asset_decimals).into()).into(),
				);
			} else if staked_asset_decimals < reward_asset_decimals {
				reward = reward.saturating_mul(
					10_u128.pow((reward_asset_decimals - staked_asset_decimals).into()).into(),
				);
			}
			Ok(reward)
		}

		fn do_offchain_worker(now: BlockNumberFor<T>) -> DispatchResult {
			if !sp_io::offchain::is_validator() {
				return Err(Error::<T>::OffchainErrNotValidator)?;
			}
			let next_rollover_unsigned_at = <NextRolloverUnsignedAt<T>>::get();
			if next_rollover_unsigned_at > now {
				return Err(Error::<T>::OffchainErrTooEarly)?;
			}

			for (id, pool_info) in Pools::<T>::iter() {
				match pool_info.pool_status {
					PoolStatus::Renewing => {
						if pool_info.lock_end_block <= now {
							log::info!("start sending unsigned rollover tx");
							let call = Call::rollover_unsigned { id, current_block: now };
							SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(
								call.into(),
							)
							.map_err(|e| {
								log::error!("Error submitting unsigned transaction: {:?}", e);
								<Error<T>>::OffchainErrSubmitTransaction
							})?;
						} else {
							log::error!("confused state, should not be here");
							return Err(Error::<T>::OffchainErrSubmitTransaction)?;
						};
					},
					_ => continue,
				}
			}
			Ok(())
		}

		fn refund_surplus_reward(
			pool_id: T::PoolId,
			pool_info: &PoolInfo<T::PoolId, AssetId, Balance, BlockNumberFor<T>>,
		) -> DispatchResult {
			let reward = Self::calculate_reward(
				pool_info.max_tokens.saturating_sub(pool_info.locked_amount),
				Zero::zero(),
				pool_info.interest_rate,
				T::InterestRateBasePoint::get(),
				T::MultiCurrency::decimals(pool_info.staked_asset_id),
				T::MultiCurrency::decimals(pool_info.reward_asset_id),
			)?;
			let pool_vault_account = Self::get_vault_account(pool_id);
			if reward > Zero::zero() {
				T::MultiCurrency::transfer(
					pool_info.reward_asset_id,
					&pool_vault_account,
					&pool_info.creator,
					reward,
					Preservation::Expendable,
				)?;
			}
			Ok(())
		}

		// FRN-68: Bounded pool closure helper functions
		pub fn initiate_bounded_closure(
			pool_id: T::PoolId,
			total_users: u32,
			closure_type: ClosureType,
		) -> DispatchResult {
			let closure_state = ClosureState {
				pool_id,
				closure_type,
				users_processed: 0,
				total_users,
				last_processed_user: None,
			};

			// Update pool status to Closing
			Pools::<T>::try_mutate(pool_id, |pool_info| -> DispatchResult {
				let pool_info = pool_info.as_mut().ok_or(Error::<T>::PoolDoesNotExist)?;
				pool_info.pool_status = PoolStatus::Closing;
				Ok(())
			})?;

			ClosingPools::<T>::insert(pool_id, closure_state);

			Self::deposit_event(Event::PoolClosureInitiated { pool_id, closure_type });

			Ok(())
		}

		fn close_empty_pool(
			pool_id: T::PoolId,
			pool: &PoolInfo<T::PoolId, AssetId, Balance, BlockNumberFor<T>>,
			creator: T::AccountId,
		) -> DispatchResult {
			let pool_vault_account = Self::get_vault_account(pool_id);

			let reward_asset_amount =
				T::MultiCurrency::balance(pool.reward_asset_id, &pool_vault_account);
			let staked_asset_amount =
				T::MultiCurrency::balance(pool.staked_asset_id, &pool_vault_account);

			// Transfer remaining funds to creator
			if reward_asset_amount > Zero::zero() {
				T::MultiCurrency::transfer(
					pool.reward_asset_id,
					&pool_vault_account,
					&creator,
					reward_asset_amount,
					Preservation::Expendable,
				)?;
			}

			if staked_asset_amount > Zero::zero() {
				T::MultiCurrency::transfer(
					pool.staked_asset_id,
					&pool_vault_account,
					&creator,
					staked_asset_amount,
					Preservation::Expendable,
				)?;
			}

			// Clean up storage
			Pools::<T>::remove(pool_id);
			PoolRelationships::<T>::remove(pool_id);
			RolloverPivot::<T>::remove(pool_id);

			Self::deposit_event(Event::PoolClosed {
				pool_id,
				reward_asset_amount,
				staked_asset_amount,
				receiver: creator,
			});

			Ok(())
		}

		fn process_closing_pools(now: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			let mut weight_used = Weight::zero();
			let base_weight = T::WeightInfo::process_closing_pools();

			if remaining_weight.all_lte(base_weight) {
				return Weight::zero();
			}

			// Process each closing pool
			for (pool_id, _closure_state) in ClosingPools::<T>::iter() {
				let batch_weight = T::WeightInfo::process_closure_batch();
				if remaining_weight.all_lte(weight_used.saturating_add(batch_weight)) {
					break;
				}

				let process_weight =
					Self::process_closure_batch(pool_id, now).unwrap_or(Weight::zero());
				weight_used = weight_used.saturating_add(process_weight);
			}

			weight_used
		}

		fn process_closure_batch(
			pool_id: T::PoolId,
			_now: BlockNumberFor<T>,
		) -> Result<Weight, DispatchError> {
			let mut weight_used = T::DbWeight::get().reads(1);

			let closure_state =
				ClosingPools::<T>::get(pool_id).ok_or(Error::<T>::NoClosureBatchToProcess)?;
			let batch_size = T::ClosureBatchSize::get();
			let mut users_processed = 0u32;

			// Get pool info for vault account
			let pool = Pools::<T>::get(pool_id).ok_or(Error::<T>::PoolDoesNotExist)?;
			let pool_vault_account = Self::get_vault_account(pool_id);

			// Process users in batches
			let mut user_iterator = if let Some(ref last_key) = closure_state.last_processed_user {
				PoolUsers::<T>::iter_prefix_from(pool_id, last_key.clone().into_inner())
			} else {
				PoolUsers::<T>::iter_prefix(pool_id)
			};

			let mut last_processed_key: Option<
				BoundedVec<u8, frame_support::traits::ConstU32<1024>>,
			> = None;

			while let Some((user_account, user_info)) = user_iterator.next() {
				if users_processed >= batch_size {
					break;
				}

				// Return user's staked funds
				if user_info.amount > Zero::zero() {
					T::MultiCurrency::transfer(
						pool.staked_asset_id,
						&pool_vault_account,
						&user_account,
						user_info.amount,
						Preservation::Expendable,
					)?;

					weight_used = weight_used.saturating_add(T::DbWeight::get().reads_writes(2, 2));
				}

				// Remove user from pool
				PoolUsers::<T>::remove(pool_id, &user_account);
				weight_used = weight_used.saturating_add(T::DbWeight::get().writes(1));

				users_processed += 1;
				last_processed_key =
					BoundedVec::try_from(user_iterator.last_raw_key().to_vec()).ok();
			}

			// Update closure state
			if users_processed > 0 {
				let new_users_processed =
					closure_state.users_processed.saturating_add(users_processed);
				let remaining_users = closure_state.total_users.saturating_sub(new_users_processed);

				if new_users_processed >= closure_state.total_users {
					// Closure complete
					Self::complete_pool_closure(pool_id, &pool)?;
					weight_used = weight_used.saturating_add(T::DbWeight::get().writes(2));
				} else {
					// Update progress
					ClosingPools::<T>::mutate(pool_id, |state| {
						if let Some(state) = state {
							state.users_processed = new_users_processed;
							state.last_processed_user = last_processed_key;
						}
					});
					weight_used = weight_used.saturating_add(T::DbWeight::get().writes(1));
				}

				Self::deposit_event(Event::PoolClosureBatchProcessed {
					pool_id,
					users_processed,
					remaining_users,
				});
			}

			Ok(weight_used)
		}

		fn complete_pool_closure(
			pool_id: T::PoolId,
			pool: &PoolInfo<T::PoolId, AssetId, Balance, BlockNumberFor<T>>,
		) -> DispatchResult {
			let pool_vault_account = Self::get_vault_account(pool_id);

			// Transfer remaining funds to creator
			let reward_balance =
				T::MultiCurrency::balance(pool.reward_asset_id, &pool_vault_account);
			let staked_balance =
				T::MultiCurrency::balance(pool.staked_asset_id, &pool_vault_account);

			if reward_balance > Zero::zero() {
				T::MultiCurrency::transfer(
					pool.reward_asset_id,
					&pool_vault_account,
					&pool.creator,
					reward_balance,
					Preservation::Expendable,
				)?;
			}

			if staked_balance > Zero::zero() {
				T::MultiCurrency::transfer(
					pool.staked_asset_id,
					&pool_vault_account,
					&pool.creator,
					staked_balance,
					Preservation::Expendable,
				)?;
			}

			// Clean up storage
			ClosingPools::<T>::remove(pool_id);
			Pools::<T>::remove(pool_id);
			PoolRelationships::<T>::remove(pool_id);
			RolloverPivot::<T>::remove(pool_id);

			Self::deposit_event(Event::PoolClosureCompleted { pool_id });
			Self::deposit_event(Event::PoolClosed {
				pool_id,
				reward_asset_amount: reward_balance,
				staked_asset_amount: staked_balance,
				receiver: pool.creator.clone(),
			});

			Ok(())
		}

		// FRN-69: Weight accounting helper functions
		fn process_pool_status_updates(now: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			let mut weight_used = Weight::zero();
			let base_weight = T::WeightInfo::process_pool_status_updates();

			if remaining_weight.all_lte(base_weight) {
				return Weight::zero();
			}

			// Get processing state for fair round-robin
			let mut processing_state = ProcessingStatus::<T>::get();
			let max_pools_per_block = 10u32; // Bounded processing

			let mut pools_processed = 0u32;
			let mut found_start = processing_state.last_processed_pool.is_none();
			let pool_iterator = Pools::<T>::iter();

			for (pool_id, pool_info) in pool_iterator {
				// Skip pools until we find our starting point (round-robin resume)
				if !found_start {
					if Some(pool_id) == processing_state.last_processed_pool {
						found_start = true;
					}
					continue;
				}

				if pools_processed >= max_pools_per_block {
					break;
				}

				let update_weight = T::DbWeight::get().reads_writes(1, 1);
				if remaining_weight.all_lte(weight_used.saturating_add(update_weight)) {
					break;
				}

				let updated = match pool_info.pool_status {
					PoolStatus::Open if pool_info.lock_start_block <= now => {
						Pools::<T>::mutate(&pool_id, |pool| {
							if let Some(pool_info) = pool {
								pool_info.pool_status = PoolStatus::Started;
								pool_info.last_updated = now;
								Self::deposit_event(Event::PoolStarted { pool_id });
							}
						});
						true
					},
					PoolStatus::Started if pool_info.lock_end_block <= now => {
						Self::refund_surplus_reward(pool_id, &pool_info).ok();

						let has_successor = PoolRelationships::<T>::get(&pool_id)
							.unwrap_or_default()
							.successor_id
							.is_some();

						if has_successor {
							Pools::<T>::mutate(&pool_id, |pool| {
								if let Some(pool_info) = pool {
									pool_info.pool_status = PoolStatus::Renewing;
									pool_info.last_updated = now;
									Self::deposit_event(Event::PoolRenewing { pool_id });
								}
							});
						} else {
							Pools::<T>::mutate(&pool_id, |pool| {
								if let Some(pool_info) = pool {
									pool_info.pool_status = PoolStatus::Matured;
									pool_info.last_updated = now;
									Self::deposit_event(Event::PoolMatured { pool_id });
								}
							});
						}
						true
					},
					_ => false,
				};

				if updated {
					weight_used = weight_used.saturating_add(update_weight);
				}

				pools_processed += 1;
				processing_state.last_processed_pool = Some(pool_id);
			}

			// Update processing state
			processing_state.round_robin_position =
				processing_state.round_robin_position.saturating_add(pools_processed);
			ProcessingStatus::<T>::put(processing_state);

			weight_used
		}

		// FRN-71: Fair processing helper functions
		pub fn process_urgent_pool_updates(
			now: BlockNumberFor<T>,
			remaining_weight: Weight,
		) -> Weight {
			let mut weight_used = Weight::zero();
			let update_weight = T::DbWeight::get().reads_writes(2, 2);

			// Process urgent pools first
			let urgent_pools: Vec<T::PoolId> = UrgentPoolUpdates::<T>::iter_keys().collect();

			for pool_id in urgent_pools {
				if remaining_weight.all_lte(weight_used.saturating_add(update_weight)) {
					break;
				}

				if let Some(pool_info) = Pools::<T>::get(&pool_id) {
					let should_update = match pool_info.pool_status {
						PoolStatus::Open if pool_info.lock_start_block <= now => true,
						PoolStatus::Started if pool_info.lock_end_block <= now => true,
						_ => false,
					};

					if should_update {
						// Process the urgent update (similar to regular processing but with priority)
						Self::process_single_pool_update(pool_id, &pool_info, now);
						weight_used = weight_used.saturating_add(update_weight);
					}

					// Remove from urgent queue regardless
					UrgentPoolUpdates::<T>::remove(&pool_id);
				}
			}

			weight_used
		}

		fn process_single_pool_update(
			pool_id: T::PoolId,
			pool_info: &PoolInfo<T::PoolId, AssetId, Balance, BlockNumberFor<T>>,
			now: BlockNumberFor<T>,
		) {
			match pool_info.pool_status {
				PoolStatus::Open if pool_info.lock_start_block <= now => {
					Pools::<T>::mutate(&pool_id, |pool| {
						if let Some(pool_info) = pool {
							pool_info.pool_status = PoolStatus::Started;
							pool_info.last_updated = now;
							Self::deposit_event(Event::PoolStarted { pool_id });
						}
					});
				},
				PoolStatus::Started if pool_info.lock_end_block <= now => {
					Self::refund_surplus_reward(pool_id, pool_info).ok();

					let has_successor = PoolRelationships::<T>::get(&pool_id)
						.unwrap_or_default()
						.successor_id
						.is_some();

					if has_successor {
						Pools::<T>::mutate(&pool_id, |pool| {
							if let Some(pool_info) = pool {
								pool_info.pool_status = PoolStatus::Renewing;
								pool_info.last_updated = now;
								Self::deposit_event(Event::PoolRenewing { pool_id });
							}
						});
					} else {
						Pools::<T>::mutate(&pool_id, |pool| {
							if let Some(pool_info) = pool {
								pool_info.pool_status = PoolStatus::Matured;
								pool_info.last_updated = now;
								Self::deposit_event(Event::PoolMatured { pool_id });
							}
						});
					}
				},
				_ => {},
			}
		}

		// FRN-70: Enhanced unsigned transaction validation functions
		fn validate_transaction_source(
			source: &TransactionSource,
		) -> Result<(), InvalidTransaction> {
			match source {
				TransactionSource::External => Ok(()),
				TransactionSource::InBlock => Ok(()),
				_ => Err(InvalidTransaction::Call),
			}
		}

		fn validate_timing(current_block: &BlockNumberFor<T>) -> Result<(), InvalidTransaction> {
			let block_number = frame_system::Pallet::<T>::block_number();

			// Check if transaction is not from the future
			if &block_number < current_block {
				return Err(InvalidTransaction::Future);
			}

			// Check if transaction is not too old (longevity check)
			let max_age = 64u32.into();
			if block_number.saturating_sub(*current_block) > max_age {
				return Err(InvalidTransaction::Stale);
			}

			// Check timing against rollover schedule
			let next_rollover_at = NextRolloverUnsignedAt::<T>::get();
			if next_rollover_at > block_number {
				return Err(InvalidTransaction::Future);
			}

			Ok(())
		}

		fn validate_pool_state(pool_id: &T::PoolId) -> Result<(), InvalidTransaction> {
			let pool = Pools::<T>::get(pool_id).ok_or(InvalidTransaction::Call)?;

			// Pool must be in Renewing state for rollover
			if pool.pool_status != PoolStatus::Renewing {
				return Err(InvalidTransaction::Call);
			}

			// Check if pool has reached its end block
			let current_block = frame_system::Pallet::<T>::block_number();
			if pool.lock_end_block > current_block {
				return Err(InvalidTransaction::Future);
			}

			// Verify pool has a successor
			let relationship = PoolRelationships::<T>::get(pool_id).unwrap_or_default();
			if relationship.successor_id.is_none() {
				return Err(InvalidTransaction::Call);
			}

			Ok(())
		}

		fn validate_system_state() -> Result<(), InvalidTransaction> {
			// Check if system is in maintenance mode or has other restrictions
			// This is a placeholder for system-wide validation

			// Validate that we're not in a paused state
			// In a real implementation, you might check maintenance mode pallet

			Ok(())
		}

		pub fn calculate_transaction_priority(pool_id: &T::PoolId) -> Option<TransactionPriority> {
			if let Some(pool) = Pools::<T>::get(pool_id) {
				// Higher priority for pools with more locked tokens
				let base_priority = UNSIGNED_PRIORITY;
				let stake_multiplier =
					pool.locked_amount.saturated_into::<u64>() / 1_000_000_000_000u64; // Adjust scaling
				Some(base_priority.saturating_add(stake_multiplier))
			} else {
				None
			}
		}
	}
}
