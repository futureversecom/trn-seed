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
	traits::{fungibles::Transfer, Currency, ExistenceRequirement},
	transactional, PalletId,
};
use frame_system::{
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::*,
};
#[cfg(feature = "runtime-benchmarks")]
use seed_pallet_common::{CreateExt, Hold};
use seed_primitives::{AccountId, BlockNumber};
use sp_io::hashing::blake2_256;
use sp_runtime::traits::{AccountIdConversion, AtLeast32BitUnsigned, CheckedAdd, Saturating, Zero};
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

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
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
	pub predecessor_id: Option<PoolId>,
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

		/// Pool ID type
		type PoolId: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen;

		/// Asset ID type
		type AssetId: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen;

		/// Token balance type
		type Balance: Member
			+ Parameter
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ TypeInfo;

		/// Incentive admin origin
		type ApproveOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Currency type
		type Currency: Currency<Self::AccountId, Balance = Self::Balance>;

		/// Assets pallet
		#[cfg(not(feature = "runtime-benchmarks"))]
		type Assets: Transfer<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>;

		/// Assets pallet - for benchmarking to manipulate assets
		#[cfg(feature = "runtime-benchmarks")]
		type Assets: Transfer<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>
			+ Hold<AccountId = Self::AccountId>
			+ Mutate<Self::AccountId, AssetId = Self::AssetId>
			+ CreateExt<AccountId = Self::AccountId>
			+ Transfer<Self::AccountId, Balance = Self::Balance>;

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

		/// Provides the public call to weight mapping
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	pub(super) type Pools<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::PoolId,
		PoolInfo<T::PoolId, T::AssetId, T::Balance, T::BlockNumber>,
	>;

	#[pallet::storage]
	pub(super) type PoolUsers<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::PoolId,
		Twox64Concat,
		T::AccountId,
		UserInfo<T::Balance>,
	>;

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
		/// Reward pool created. [id, asset_id, interest_rate, max_tokens, end_block]
		PoolCreated(T::PoolId, T::AssetId, u32, T::Balance, T::BlockNumber, T::BlockNumber),
		/// Reward pool provisioning. [id]
		PoolProvisioning(T::PoolId),
		/// Reward pool rollovering. [id]
		PoolRollingOver(T::PoolId),
		/// Reward pool done. [id]
		PoolDone(T::PoolId),
		/// Reward pool closed. [id]
		PoolClosed(T::PoolId),
		/// Set pool successor. [predecessor_id, successor_id]
		SetSuccession(T::PoolId, T::PoolId),
		/// User info updated. [pool_id, user, should_rollover]
		UserInfoUpdated(T::PoolId, T::AccountId, bool),
		/// User joined pool. [user, pool_id, amount]
		UserJoined(T::AccountId, T::PoolId, T::Balance),
		/// User exited pool. [user, pool_id, amount]
		UserExited(T::AccountId, T::PoolId, T::Balance),
		/// Rewards claimed. [user, pool_id, amount]
		RewardsClaimed(T::AccountId, T::PoolId, T::Balance),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Invalid block range
		InvalidBlockRange,
		/// Pool already exists
		PoolAlreadyExists,
		/// Pool does not exist
		PoolDoesNotExist,
		/// Pool does not exist
		SuccessorPoolDoesNotExist,
		/// Pool does not exist
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
		/// Create an incentive pool
		///
		/// Parameters:
		/// - `origin`: Originator account
		/// - `asset_id`: Asset ID used for the pool
		/// - `interest_rate`: Annualized interest rate of the pool
		/// - `max_tokens`: Maximum token amount limit for the pool
		/// - `start_block`: Start block of the pool
		/// - `end_block`: End block of the pool
		#[pallet::weight(T::WeightInfo::create_pool())]
		pub fn create_pool(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			interest_rate: u32,
			max_tokens: T::Balance,
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

			Self::deposit_event(Event::PoolCreated(
				id,
				asset_id,
				interest_rate,
				max_tokens,
				start_block,
				end_block,
			));
			Ok(())
		}

		/// Set successor relationship between pools
		///
		/// Parameters:
		/// - `origin`: Originator account
		/// - `predecessor_id`: Predecessor pool ID
		/// - `successor_id`: Successor pool ID
		#[pallet::weight(T::WeightInfo::set_incentive_pool_succession())]
		pub fn set_incentive_pool_succession(
			origin: OriginFor<T>,
			predecessor_id: T::PoolId,
			successor_id: T::PoolId,
		) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;

			// Check that predecessor exists
			let predecessor_pool =
				Pools::<T>::get(predecessor_id).ok_or(Error::<T>::PredecessorPoolDoesNotExist)?;
			// Check that successor exists
			let successor_pool =
				Pools::<T>::get(successor_id).ok_or(Error::<T>::SuccessorPoolDoesNotExist)?;

			// Check successor max_tokens is greater than predecessor max_tokens
			ensure!(
				successor_pool.max_tokens >= predecessor_pool.max_tokens,
				Error::<T>::SuccessorPoolSizeShouldBeGreaterThanPredecessor
			);

			<PoolRelationships<T>>::insert(
				&predecessor_id,
				PoolRelationship { predecessor_id: None, successor_id: Some(successor_id) },
			);

			Self::deposit_event(Event::SetSuccession(predecessor_id, successor_id));

			Ok(())
		}

		/// Update user preference to roll over to next pool
		///
		/// Parameters:
		/// - `origin`: Originator account
		/// - `id`: Pool ID
		/// - `should_roll_over`: Whether to roll over to next pool
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

			Self::deposit_event(Event::UserInfoUpdated(id, who, should_roll_over));

			Ok(())
		}

		/// Close reward pool
		#[pallet::weight(T::WeightInfo::close_pool())]
		pub fn close_pool(origin: OriginFor<T>, id: T::PoolId) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;

			ensure!(Pools::<T>::contains_key(id), Error::<T>::PoolDoesNotExist);

			Pools::<T>::remove(id);
			PoolUsers::<T>::drain_prefix(id);
			PoolRelationships::<T>::remove(id);
			RolloverPivot::<T>::remove(id);

			Self::deposit_event(Event::PoolClosed(id));
			Ok(())
		}

		/// Join reward pool
		#[pallet::weight(T::WeightInfo::join_pool())]
		#[transactional]
		pub fn join_pool(
			origin: OriginFor<T>,
			id: T::PoolId,
			amount: T::Balance,
		) -> DispatchResult {
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

			Self::deposit_event(Event::UserJoined(who, id, amount));
			Ok(())
		}

		/// Exit reward pool
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

			Self::deposit_event(Event::UserExited(who, id, amount));
			Ok(())
		}

		/// Claim reward for user
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

			ensure!(
				frame_system::Pallet::<T>::block_number() > pool.end_block,
				Error::<T>::NotReadyForClaimingReward
			);

			let reward =
				Self::calculate_reward(user_info.amount, pool.interest_rate, user_info.reward_debt);
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
			T::Currency::transfer(&vault_account, &who, reward, ExistenceRequirement::AllowDeath)?;

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

			Self::deposit_event(Event::RewardsClaimed(who, id, reward));
			Ok(())
		}

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
						let mut total_rolled_over_amount = successor_pool_info.locked_amount;

						let successor_vault_account =
							Self::get_vault_account(successor_id).unwrap();

						while let Some((who, user_info)) = map_iterator.next() {
							if user_info.should_migrate() {
								total_rolled_over_amount =
									total_rolled_over_amount.saturating_add(user_info.amount);
								// Check if successor pool has enough space
								// If not, set previous pool to done to prevent further rollover
								if total_rolled_over_amount > successor_pool_info.max_tokens {
									Pools::<T>::try_mutate(id, |pool_info| -> DispatchResult {
										let pool_info = pool_info
											.as_mut()
											.ok_or(Error::<T>::PoolDoesNotExist)?;
										pool_info.last_updated =
											frame_system::Pallet::<T>::block_number();
										pool_info.pool_status = PoolStatus::Done;
										Ok(())
									})?;
									Self::deposit_event(Event::PoolDone(id));
									break
								}

								PoolUsers::<T>::insert(successor_id, who, user_info.clone());
								T::Assets::transfer(
									pool_info.asset_id,
									&vault_account,
									&successor_vault_account,
									user_info.amount,
									false,
								)?;
								PoolUsers::<T>::try_mutate(
									id,
									who,
									|pool_user| -> DispatchResult {
										*pool_user =
											Some(UserInfo { rolled_over: true, ..user_info });
										Ok(())
									},
								)?;
								Pools::<T>::try_mutate(id, |pool_info| -> DispatchResult {
									let pool_info =
										pool_info.as_mut().ok_or(Error::<T>::PoolDoesNotExist)?;
									pool_info.last_updated =
										frame_system::Pallet::<T>::block_number();
									pool_info.locked_amount =
										pool_info.locked_amount.saturating_sub(user_info.amount);
									Ok(())
								})?;
								Pools::<T>::try_mutate(
									successor_id,
									|pool_info| -> DispatchResult {
										let pool_info = pool_info
											.as_mut()
											.ok_or(Error::<T>::PoolDoesNotExist)?;
										pool_info.last_updated =
											frame_system::Pallet::<T>::block_number();
										pool_info.locked_amount = pool_info
											.locked_amount
											.saturating_add(user_info.amount);
										Ok(())
									},
								)?;
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
							Pools::<T>::try_mutate(id, |pool_info| -> DispatchResult {
								let pool_info =
									pool_info.as_mut().ok_or(Error::<T>::PoolDoesNotExist)?;
								pool_info.last_updated = frame_system::Pallet::<T>::block_number();
								pool_info.pool_status = PoolStatus::Done;
								Ok(())
							})?;
							Self::deposit_event(Event::PoolDone(id));
						}
						RolloverPivot::<T>::insert(id, current_last_raw_key);
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
		fn on_initialize(now: BlockNumberFor<T>) -> frame_support::weights::Weight {
			let mut total_weight = Weight::zero();

			for (id, pool_info) in Pools::<T>::iter() {
				match pool_info.pool_status {
					PoolStatus::Inactive =>
						if pool_info.start_block == now {
							Pools::<T>::mutate(id, |pool| {
								*pool = Some(PoolInfo {
									pool_status: PoolStatus::Provisioning,
									last_updated: now,
									..pool_info
								});
							});
							total_weight += T::DbWeight::get().reads_writes(1, 1);
							Self::deposit_event(Event::PoolProvisioning(id));
						},
					PoolStatus::Provisioning => {
						if pool_info.end_block == now {
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
								Self::deposit_event(Event::PoolRollingOver(id));
							} else {
								// if no successor, set pool done
								Pools::<T>::mutate(id, |pool| {
									*pool = Some(PoolInfo {
										pool_status: PoolStatus::Done,
										last_updated: now,
										..pool_info
									});
								});
								Self::deposit_event(Event::PoolDone(id));
							}
							total_weight += T::DbWeight::get().reads_writes(1, 1);
						}
					},
					_ => continue,
				}
			}
			total_weight
		}

		fn offchain_worker(now: BlockNumberFor<T>) {
			if let Err(e) = Self::do_offchain_worker(now) {
				log::info!(
				  target: "incentive offchain worker",
				  "error happened in offchain worker at {:?}: {:?}",
				  now,
				  e,
				);
			} else {
				log::debug!(
				  target: "incentive offchain worker",
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
					ValidTransaction::with_tag_prefix("IncentiveChainWorker")
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
		fn calculate_reward(
			user_joined_amount: T::Balance,
			interest_rate: u32,
			reward_debt: T::Balance,
		) -> T::Balance {
			let mut reward = user_joined_amount.saturating_mul(interest_rate.into());
			reward = reward.saturating_sub(reward_debt);

			reward
		}

		pub fn get_vault_account(pool_id: T::PoolId) -> Option<T::AccountId> {
			// use incentive module account id and offered asset id as entropy to generate reward
			// vault id.
			let entropy =
				(b"modlpy/palletinc", Self::account_id(), pool_id).using_encoded(blake2_256);
			if let Ok(pool_vault_account) = T::AccountId::decode(&mut &entropy[..]) {
				return Some(pool_vault_account)
			}
			None
		}

		fn account_id() -> T::AccountId {
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
	}
}
