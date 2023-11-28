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

#[cfg(feature = "runtime-benchmarks")]
use frame_support::traits::fungibles::Mutate;
use frame_support::{
	log,
	pallet_prelude::*,
	sp_runtime::traits::One,
	traits::{fungibles::Transfer, Currency},
	transactional, PalletId,
};
use frame_system::{
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::*,
};
#[cfg(feature = "runtime-benchmarks")]
use seed_pallet_common::{CreateExt, Hold};
use seed_primitives::{AccountId, AssetId, Balance, BlockNumber};
use sp_io::hashing::blake2_256;
use sp_runtime::traits::{AccountIdConversion, AtLeast32BitUnsigned, CheckedAdd, Zero};
use sp_std::prelude::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

/// Stores information about a reward pool.
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct PoolInfo<PoolId, AssetId, Balance, BlockNumber> {
	pub id: PoolId,
	pub asset_id: AssetId,
	pub interest_rate: u32,
	pub max_tokens: Balance,
	pub last_updated: BlockNumber,
	pub start_block: BlockNumber,
	pub end_block: BlockNumber,
	pub locked_amount: Balance,
	pub pool_status: PoolStatus,
}

#[derive(Clone, Copy, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum PoolStatus {
	Inactive,
	Provisioning,
	RollingOver,
	Done,
}

impl Default for PoolStatus {
	fn default() -> Self {
		Self::Inactive
	}
}

/// Stores relationship between pools.
#[derive(Default, Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct PoolRelationship<PoolId> {
	pub successor_id: Option<PoolId>,
}

/// Stores user information for a pool.
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct UserInfo<Balance> {
	pub amount: Balance,
	pub reward_debt: Balance,
	pub should_rollover: bool,
	pub rolled_over: bool,
}

impl<Balance: Default> UserInfo<Balance> {
	fn should_migrate(&self) -> bool {
		self.should_rollover && !self.rolled_over
	}
}

impl<Balance: Default> Default for UserInfo<Balance> {
	fn default() -> Self {
		Self {
			amount: Balance::default(),
			reward_debt: Balance::default(),
			should_rollover: true,
			rolled_over: false,
		}
	}
}

#[frame_support::pallet]
pub mod pallet {
	use frame_support::traits::fungibles::InspectMetadata;

	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config<AccountId = AccountId> + SendTransactionTypes<Call<Self>>
	{
		/// Event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// PoolId is used to distinguish between different pools that manage and facilitate
		/// the attendence and rewarding of assets.
		type PoolId: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen;

		/// The native token asset Id (managed by pallet-balances)
		#[pallet::constant]
		type NativeAssetId: Get<AssetId>;

		/// Admin origin
		type ApproveOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Currency type
		type Currency: Currency<Self::AccountId, Balance = Balance>;

		/// Assets pallet
		#[cfg(not(feature = "runtime-benchmarks"))]
		type Assets: Transfer<Self::AccountId, AssetId = AssetId, Balance = Balance>
			+ InspectMetadata<Self::AccountId, AssetId = AssetId>;

		/// Assets pallet - for benchmarking to manipulate assets
		#[cfg(feature = "runtime-benchmarks")]
		type Assets: Transfer<Self::AccountId, AssetId = AssetId, Balance = Balance>
			+ Hold<AccountId = Self::AccountId>
			+ Mutate<Self::AccountId, AssetId = AssetId>
			+ CreateExt<AccountId = Self::AccountId>
			+ Transfer<Self::AccountId, Balance = Balance>
			+ InspectMetadata<Self::AccountId, AssetId = AssetId>;

		/// Pallete ID
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Interval between unsigned transactions
		#[pallet::constant]
		type UnsignedInterval: Get<BlockNumber>;

		/// Max number of users to rollover per block
		#[pallet::constant]
		type RolloverBatchSize: Get<u32>;

		/// Max pivot string length
		type MaxStringLength: Get<u32>;

		/// Interest rate for 100% base point. Use the base point to calculate actual interest rate.
		/// e.g. when 10000 is base points, and interest rate is 1000 when creating pool, then the
		/// actual interest rate is 10% (1000 / 10000).
		#[pallet::constant]
		type InterestRateBasePoint: Get<u32>;

		/// Provides the public call to weight mapping
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	pub(super) type Pools<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::PoolId,
		PoolInfo<T::PoolId, AssetId, Balance, T::BlockNumber>,
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

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Reward pool created.
		PoolCreated {
			pool_id: T::PoolId,
			asset_id: AssetId,
			interest_rate: u32,
			max_tokens: Balance,
			start_block: T::BlockNumber,
			end_block: T::BlockNumber,
		},
		/// User could join pool.
		PoolProvisioning { pool_id: T::PoolId },
		/// Pool starts to rollover users that want to continue to next pool.
		PoolRollingOver { pool_id: T::PoolId },
		/// Pool rollover is done and ready for users to claim rewards.
		PoolDone { pool_id: T::PoolId },
		/// Pool closed, no more users can join.
		PoolClosed { pool_id: T::PoolId },
		/// Set pool successor, when predecessor pool is done, users will be rolled over to
		/// successor pool.
		SetSuccession { predecessor_pool_id: T::PoolId, successor_pool_id: T::PoolId },
		/// User info updated, currently only rollover preference is updated.
		UserInfoUpdated { pool_id: T::PoolId, account_id: T::AccountId, should_rollover: bool },
		/// User joined pool.
		UserJoined { account_id: T::AccountId, pool_id: T::PoolId, amount: Balance },
		/// User exited pool.
		UserExited { account_id: T::AccountId, pool_id: T::PoolId, amount: Balance },
		/// Rewards claimed.
		RewardsClaimed { account_id: T::AccountId, pool_id: T::PoolId, amount: Balance },
	}

	#[pallet::error]
	pub enum Error<T> {
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
		/// Cannot exit pool, no tokens staked
		NoTokensStaked,
		/// Reward pool is not active
		PoolNotActive,
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
		/// Pivot string too long
		PivotStringTooLong,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates a liquidity pool for a specified asset over a range of future blocks.
		///
		/// The pool created by this function allows users to stake specified assets and,
		/// depending on the pool's performance, receive rewards in the form of native tokens
		/// (ROOT). This function can only be called by an admin user who has the necessary
		/// permissions.
		///
		/// Parameters:
		/// - `origin`: The origin account that is creating the pool. Must be an admin.
		/// - `asset_id`: The ID of the asset for which the pool is being created.
		/// - `interest_rate`: The interest rate for the pool, dictating the reward distribution.
		/// - `max_tokens`: The maximum amount of tokens that can be staked in this pool.
		/// - `start_block`: The starting block number from which the pool will begin accepting
		///   stakes.
		/// - `end_block`: The ending block number after which the pool will no longer accept
		///   stakes.
		///
		/// Restrictions:
		/// - The `start_block` must be in the future and less than `end_block`.
		/// - The `end_block` must be greater than the `start_block`.
		///
		/// Emits `PoolCreated` event when successful.
		#[pallet::weight(T::WeightInfo::create_pool())]
		#[transactional]
		pub fn create_pool(
			origin: OriginFor<T>,
			asset_id: AssetId,
			interest_rate: u32,
			max_tokens: Balance,
			start_block: T::BlockNumber,
			end_block: T::BlockNumber,
		) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;

			let id = NextPoolId::<T>::get();
			let next_pool_id = id.checked_add(&One::one()).ok_or(Error::<T>::NoAvailablePoolId)?;

			ensure!(
				start_block > frame_system::Pallet::<T>::block_number(),
				Error::<T>::InvalidBlockRange
			);
			ensure!(start_block < end_block, Error::<T>::InvalidBlockRange);

			let pool_info = PoolInfo {
				id,
				asset_id,
				interest_rate,
				max_tokens,
				last_updated: frame_system::Pallet::<T>::block_number(),
				start_block,
				end_block,
				locked_amount: Zero::zero(),
				pool_status: Default::default(),
			};

			NextPoolId::<T>::mutate(|id| {
				*id = next_pool_id;
			});

			Pools::<T>::insert(id, pool_info);

			let max_rewards = Self::calculate_reward(
				max_tokens,
				0,
				interest_rate,
				T::InterestRateBasePoint::get(),
				T::Assets::decimals(&asset_id),
				T::Assets::decimals(&T::NativeAssetId::get()),
			);

			// Transfer max rewards to pool vault account
			T::Assets::transfer(
				T::NativeAssetId::get(),
				&Self::account_id(),
				&Self::get_vault_account(id).unwrap(),
				max_rewards,
				false,
			)?;

			Self::deposit_event(Event::PoolCreated {
				pool_id: id,
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block,
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
		#[pallet::weight(T::WeightInfo::set_pool_succession())]
		pub fn set_pool_succession(
			origin: OriginFor<T>,
			predecessor_pool_id: T::PoolId,
			successor_pool_id: T::PoolId,
		) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;

			// Check that predecessor exists
			let predecessor_pool = Pools::<T>::get(predecessor_pool_id)
				.ok_or(Error::<T>::PredecessorPoolDoesNotExist)?;
			// Check that successor exists
			let successor_pool =
				Pools::<T>::get(successor_pool_id).ok_or(Error::<T>::SuccessorPoolDoesNotExist)?;

			// Check successor max_tokens is greater than predecessor max_tokens
			ensure!(
				successor_pool.max_tokens >= predecessor_pool.max_tokens,
				Error::<T>::SuccessorPoolSizeShouldBeGreaterThanPredecessor
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
		/// - `should_roll_over`: A boolean indicating whether the user's stake should rollover to a
		///   successor pool.
		///
		/// Restrictions:
		/// - The pool must be in the `Provisioning` status, not yet active or closed.
		///
		/// Emits `UserInfoUpdated` event if the rollover preference is successfully updated.
		#[pallet::weight(T::WeightInfo::set_pool_rollover())]
		pub fn set_pool_rollover(
			origin: OriginFor<T>,
			id: T::PoolId,
			should_roll_over: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let Some(pool) = Pools::<T>::get(id) else {
				Err(Error::<T>::PoolDoesNotExist)?
			};

			ensure!(pool.pool_status == PoolStatus::Provisioning, Error::<T>::PoolNotActive);

			PoolUsers::<T>::try_mutate(id, who, |pool_user| -> DispatchResult {
				let pool_user = pool_user.as_mut().ok_or(Error::<T>::NoTokensStaked)?;
				pool_user.should_rollover = should_roll_over;
				Ok(())
			})?;

			Self::deposit_event(Event::UserInfoUpdated {
				pool_id: id,
				account_id: who,
				should_rollover: should_roll_over,
			});

			Ok(())
		}

		/// Closes an active reward pool.
		///
		/// This function allows an admin to close an active pool. Once closed, the pool stops
		/// accepting new stakes.
		///
		/// Parameters:
		/// - `origin`: The origin account that is closing the pool. Must be an admin.
		/// - `id`: The ID of the pool being closed.
		///
		/// Restrictions:
		/// - The pool identified by `id` must exist and be active.
		///
		/// Emits `PoolClosed` event when the pool is successfully closed.
		#[pallet::weight(T::WeightInfo::close_pool())]
		pub fn close_pool(origin: OriginFor<T>, id: T::PoolId) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;

			ensure!(Pools::<T>::contains_key(id), Error::<T>::PoolDoesNotExist);

			Pools::<T>::remove(id);
			PoolUsers::<T>::drain_prefix(id);
			PoolRelationships::<T>::remove(id);
			RolloverPivot::<T>::remove(id);

			Self::deposit_event(Event::PoolClosed { pool_id: id });
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
		/// - The pool must be in the `Provisioning` status and accepting new stakes.
		/// - The total staked amount after the user's stake must not exceed the pool's
		///   `max_tokens`.
		///
		/// Emits `UserJoined` event if the user successfully joins the pool.
		#[pallet::weight(T::WeightInfo::join_pool())]
		#[transactional]
		pub fn join_pool(origin: OriginFor<T>, id: T::PoolId, amount: Balance) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let vault_account = Self::get_vault_account(id).unwrap();

			let Some(pool) = Pools::<T>::get(id) else {
				Err(Error::<T>::PoolDoesNotExist)?
			};

			ensure!(pool.pool_status == PoolStatus::Provisioning, Error::<T>::PoolNotActive);

			ensure!(
				pool.locked_amount + amount <= pool.max_tokens,
				Error::<T>::StakingLimitExceeded
			);

			T::Assets::transfer(pool.asset_id, &who, &vault_account, amount, false)?;

			let user = PoolUsers::<T>::get(id, &who).unwrap_or_default();
			PoolUsers::<T>::insert(
				id,
				&who,
				UserInfo { amount: user.amount.saturating_add(amount), ..UserInfo::default() },
			);

			Pools::<T>::try_mutate(id, |pool_info| -> DispatchResult {
				let pool_info = pool_info.as_mut().ok_or(Error::<T>::PoolDoesNotExist)?;
				pool_info.locked_amount = pool_info.locked_amount.saturating_add(amount);
				Ok(())
			})?;

			Self::deposit_event(Event::UserJoined { account_id: who, pool_id: id, amount });
			Ok(())
		}

		/// Allows a user to exit from a reward pool, withdrawing their staked tokens.
		///
		/// Parameters:
		/// - `origin`: The account of the user exiting the pool.
		/// - `id`: The ID of the pool from which the user is exiting.
		///
		/// Restrictions:
		/// - The pool must be in the `Provisioning` status and not closed.
		/// - The user must have tokens staked in the pool.
		///
		/// Emits `UserExited` event if the user successfully exits the pool and claims any rewards.
		#[pallet::weight(T::WeightInfo::exit_pool())]
		#[transactional]
		pub fn exit_pool(origin: OriginFor<T>, id: T::PoolId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let vault_account = Self::get_vault_account(id).unwrap();

			let Some(pool) = Pools::<T>::get(id) else {
				Err(Error::<T>::PoolDoesNotExist)?
			};

			ensure!(pool.pool_status == PoolStatus::Provisioning, Error::<T>::PoolNotActive);

			let Some(user_info) = PoolUsers::<T>::get(id, &who) else {
				Err(Error::<T>::NoTokensStaked)?
			};
			ensure!(user_info.amount > Zero::zero(), Error::<T>::NoTokensStaked);

			let amount = user_info.amount;
			T::Assets::transfer(pool.asset_id, &vault_account, &who, amount, false)?;

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
		/// Users who have staked tokens in a pool that has reached the `Done` status
		/// can call this function to claim their share of the reward.
		///
		/// Parameters:
		/// - `origin`: The account of the user claiming the reward.
		/// - `id`: The ID of the pool from which the reward is being claimed.
		///
		/// Restrictions:
		/// - The pool must be in the `Done` status.
		/// - The user must have staked tokens in the pool and not already claimed their reward.
		///
		/// Emits `RewardsClaimed` event if the reward is successfully claimed by the user.
		#[pallet::weight(T::WeightInfo::claim_reward())]
		#[transactional]
		pub fn claim_reward(origin: OriginFor<T>, id: T::PoolId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let vault_account = Self::get_vault_account(id).unwrap();

			let Some(user_info) = PoolUsers::<T>::get(id, &who) else {
				Err(Error::<T>::NoTokensStaked)?
			};
			let Some(pool) = Pools::<T>::get(id) else {
				Err(Error::<T>::PoolDoesNotExist)?
			};

			ensure!(pool.pool_status == PoolStatus::Done, Error::<T>::NotReadyForClaimingReward);

			let reward = Self::calculate_reward(
				user_info.amount,
				user_info.reward_debt,
				pool.interest_rate,
				T::InterestRateBasePoint::get(),
				T::Assets::decimals(&pool.asset_id),
				T::Assets::decimals(&T::NativeAssetId::get()),
			);

			if reward == Zero::zero() {
				return Ok(())
			}

			let amount = if user_info.should_rollover == false {
				T::Assets::transfer(pool.asset_id, &vault_account, &who, user_info.amount, false)?;
				user_info.amount
			} else {
				Zero::zero()
			};

			// Transfer reward to user
			T::Assets::transfer(T::NativeAssetId::get(), &vault_account, &who, reward, false)?;

			Pools::<T>::try_mutate(id, |pool_info| -> DispatchResult {
				let pool_info = pool_info.as_mut().ok_or(Error::<T>::PoolDoesNotExist)?;
				pool_info.last_updated = frame_system::Pallet::<T>::block_number();
				pool_info.locked_amount = pool_info.locked_amount.saturating_sub(amount);
				Ok(())
			})?;
			PoolUsers::<T>::try_mutate(id, who, |pool_user| -> DispatchResult {
				let pool_user = pool_user.as_mut().ok_or(Error::<T>::NoTokensStaked)?;
				pool_user.reward_debt = pool_user.reward_debt.saturating_add(reward);
				Ok(())
			})?;

			Self::deposit_event(Event::RewardsClaimed {
				account_id: who,
				pool_id: id,
				amount: reward,
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
		///   `PoolDone` if the pool is completed.
		#[pallet::weight(T::WeightInfo::rollover_unsigned())]
		#[transactional]
		pub fn rollover_unsigned(
			origin: OriginFor<T>,
			id: T::PoolId,
			_current_block: BlockNumberFor<T>,
		) -> DispatchResult {
			ensure_none(origin)?;
			let Some(pool_info) = Pools::<T>::get(id) else {
				return Err(Error::<T>::PoolDoesNotExist.into())
			};
			log::warn!("start processing the rollover");
			if let PoolStatus::RollingOver = pool_info.pool_status {
				if let Some(vault_account) = Self::get_vault_account(id) {
					// Check for successor
					let successor_id =
						PoolRelationships::<T>::get(id).unwrap_or_default().successor_id;

					if let Some(successor_id) = successor_id {
						let Some(successor_pool_info) = Pools::<T>::get(successor_id) else {
							return Err(Error::<T>::PoolDoesNotExist.into())
						};
						// Migrate users to successor
						let start_key = RolloverPivot::<T>::get(id);
						let payout_pivot: Vec<u8> = start_key
							.clone()
							.try_into()
							.map_err(|_| Error::<T>::PivotStringTooLong)?;

						let mut map_iterator = match RolloverPivot::<T>::contains_key(id) {
							true => <PoolUsers<T>>::iter_prefix_from(id, payout_pivot),
							false => <PoolUsers<T>>::iter_prefix(id),
						};

						let mut count = 0;
						let mut rollover_amount: Balance = Zero::zero();
						let mut predecessor_pool_status = pool_info.pool_status;

						let successor_vault_account =
							Self::get_vault_account(successor_id).unwrap();

						while let Some((who, user_info)) = map_iterator.next() {
							if user_info.should_migrate() {
								// Check if successor pool has enough space
								// If not, set previous pool to done to prevent further rollover
								if successor_pool_info
									.locked_amount
									.saturating_add(rollover_amount)
									.saturating_add(user_info.amount) > successor_pool_info
									.max_tokens
								{
									predecessor_pool_status = PoolStatus::Done;
									break
								} else {
									rollover_amount =
										rollover_amount.saturating_add(user_info.amount);
								}

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
							if count > T::RolloverBatchSize::get() {
								break
							}
						}
						let current_last_raw_key: BoundedVec<u8, T::MaxStringLength> =
							BoundedVec::try_from(map_iterator.last_raw_key().to_vec())
								.map_err(|_| Error::<T>::PivotStringTooLong)?;
						if current_last_raw_key == start_key.clone() {
							predecessor_pool_status = PoolStatus::Done;
						}
						RolloverPivot::<T>::insert(id, current_last_raw_key);

						T::Assets::transfer(
							pool_info.asset_id,
							&vault_account,
							&successor_vault_account,
							rollover_amount,
							false,
						)?;
						Pools::<T>::try_mutate(id, |pool_info| -> DispatchResult {
							let pool_info =
								pool_info.as_mut().ok_or(Error::<T>::PoolDoesNotExist)?;
							pool_info.last_updated = frame_system::Pallet::<T>::block_number();
							pool_info.locked_amount =
								pool_info.locked_amount.saturating_sub(rollover_amount);
							pool_info.pool_status = predecessor_pool_status;
							Ok(())
						})?;
						if predecessor_pool_status == PoolStatus::Done {
							Self::deposit_event(Event::PoolDone { pool_id: id });
						}
						Pools::<T>::try_mutate(successor_id, |pool_info| -> DispatchResult {
							let pool_info =
								pool_info.as_mut().ok_or(Error::<T>::PoolDoesNotExist)?;
							pool_info.last_updated = frame_system::Pallet::<T>::block_number();
							pool_info.locked_amount =
								pool_info.locked_amount.saturating_add(rollover_amount);
							Ok(())
						})?;
					}
				}
			}

			let current_block = <frame_system::Pallet<T>>::block_number();
			log::warn!("current block {:?}", current_block);
			let next_unsigned_at = current_block + T::UnsignedInterval::get().into();
			<NextRolloverUnsignedAt<T>>::put(next_unsigned_at);
			log::warn!("proposed next unsigned at {:?}", next_unsigned_at);
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_idle(now: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			let mut cost_weight = T::DbWeight::get().reads(1u64);
			if remaining_weight.all_lte(cost_weight) {
				return Weight::zero()
			}

			for (id, pool_info) in Pools::<T>::iter() {
				match pool_info.pool_status {
					PoolStatus::Inactive =>
						if pool_info.start_block <= now {
							if remaining_weight.all_lte(
								cost_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1)),
							) {
								return cost_weight
							}

							cost_weight =
								cost_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
							Pools::<T>::mutate(id, |pool| {
								*pool = Some(PoolInfo {
									pool_status: PoolStatus::Provisioning,
									last_updated: now,
									..pool_info
								});
							});
							Self::deposit_event(Event::PoolProvisioning { pool_id: id });
						},
					PoolStatus::Provisioning => {
						if pool_info.end_block <= now {
							if remaining_weight.all_lte(
								cost_weight.saturating_add(T::DbWeight::get().reads_writes(1, 2)),
							) {
								return cost_weight
							}

							// Transfer remaining tokens back to vault account
							Self::refund_surplus_reward(id, &pool_info).unwrap_or_default();

							cost_weight =
								cost_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
							// Check for successor
							let successor_id =
								PoolRelationships::<T>::get(id).unwrap_or_default().successor_id;
							if successor_id.is_some() {
								Pools::<T>::mutate(id, |pool| {
									*pool = Some(PoolInfo {
										pool_status: PoolStatus::RollingOver,
										last_updated: now,
										..pool_info
									});
								});
								Self::deposit_event(Event::PoolRollingOver { pool_id: id });
							} else {
								// if no successor, set pool done
								Pools::<T>::mutate(id, |pool| {
									*pool = Some(PoolInfo {
										pool_status: PoolStatus::Done,
										last_updated: now,
										..pool_info
									});
								});
								Self::deposit_event(Event::PoolDone { pool_id: id });
							}
						}
					},
					_ => continue,
				}
			}

			cost_weight
		}

		fn offchain_worker(now: BlockNumberFor<T>) {
			if let Err(e) = Self::do_offchain_worker(now) {
				log::info!(
				  target: "liquidity_pools offchain worker",
				  "error happened in offchain worker at {:?}: {:?}",
				  now,
				  e,
				);
			} else {
				log::debug!(
				  target: "liquidity_pools offchain worker",
				  "offchain worker start at block: {:?} already done!",
				  now,
				);
			}
		}
	}

	const UNSIGNED_PRIORITY: TransactionPriority = TransactionPriority::max_value() / 2;

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			match call {
				Call::rollover_unsigned { id, current_block } => {
					let current_block_number = <frame_system::Pallet<T>>::block_number();
					if &current_block_number < current_block {
						return InvalidTransaction::Future.into()
					}
					ValidTransaction::with_tag_prefix("LiquidityPoolsChainWorker")
						.priority(UNSIGNED_PRIORITY)
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
		/// - `asset_decimals`: The number of decimal places used for the asset token.
		/// - `native_decimals`: The number of decimal places used for the native token.
		///
		/// Returns:
		/// - The calculated reward amount in native tokens, after adjusting for decimal places and
		///   subtracting the reward debt.
		pub fn calculate_reward(
			user_joined_amount: Balance,
			reward_debt: Balance,
			interest_rate: u32,
			interest_rate_base_point: u32,
			asset_decimals: u8,
			native_decimals: u8,
		) -> Balance {
			// Calculate reward in asset token
			let mut reward = user_joined_amount
				.saturating_mul(interest_rate.into())
				.saturating_div(interest_rate_base_point.into());
			// Remaining rewards
			reward = reward.saturating_sub(reward_debt);

			// Convert reward to native token based on decimals
			if asset_decimals > native_decimals {
				reward = reward
					.saturating_div(10_u128.pow((asset_decimals - native_decimals).into()).into());
			} else if asset_decimals < native_decimals {
				reward = reward
					.saturating_mul(10_u128.pow((native_decimals - asset_decimals).into()).into());
			}

			reward
		}

		pub fn get_vault_account(pool_id: T::PoolId) -> Option<T::AccountId> {
			// use the module account id and offered asset id as entropy to generate reward
			// vault id.
			let entropy =
				(b"modlpy/palletlqd", Self::account_id(), pool_id).using_encoded(blake2_256);
			if let Ok(pool_vault_account) = T::AccountId::decode(&mut &entropy[..]) {
				return Some(pool_vault_account)
			}
			None
		}

		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		fn do_offchain_worker(now: BlockNumberFor<T>) -> DispatchResult {
			if !sp_io::offchain::is_validator() {
				return Err(Error::<T>::OffchainErrNotValidator)?
			}
			let next_rollover_unsigned_at = <NextRolloverUnsignedAt<T>>::get();
			if next_rollover_unsigned_at > now {
				return Err(Error::<T>::OffchainErrTooEarly)?
			}

			for (id, pool_info) in Pools::<T>::iter() {
				match pool_info.pool_status {
					PoolStatus::RollingOver => {
						if pool_info.end_block <= now {
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
							log::warn!("confused state, should not be here");
							continue
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
				T::Assets::decimals(&pool_info.asset_id),
				T::Assets::decimals(&T::NativeAssetId::get()),
			);
			let pool_vault_account = Self::get_vault_account(pool_id).unwrap();
			if reward > Zero::zero() {
				T::Assets::transfer(
					pool_info.asset_id,
					&pool_vault_account,
					&Self::account_id(),
					reward,
					false,
				)?;
			}
			Ok(())
		}
	}
}
