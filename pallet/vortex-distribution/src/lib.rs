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
mod benchmarking;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

use codec::{Decode, Encode, HasCompact};
use frame_support::{
	dispatch::DispatchResult,
	log,
	pallet_prelude::*,
	traits::{
		tokens::{
			fungibles::{self, Inspect, Mutate},
			Fortitude, Precision, Preservation,
		},
		Get,
	},
	transactional,
	weights::constants::RocksDbWeight as DbWeight,
	PalletId,
};
use frame_system::{
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::*,
};
use pallet_staking::BalanceOf;
use scale_info::TypeInfo;
use seed_pallet_common::CreateExt;
use seed_primitives::{AssetId, OffchainErr};
use sp_runtime::{
	traits::{
		AccountIdConversion, AtLeast32BitUnsigned, CheckedAdd, One, Saturating, StaticLookup, Zero,
	},
	Perbill, RuntimeDebug,
};
use sp_std::{convert::TryInto, prelude::*};

pub const VTX_DIST_UNSIGNED_PRIORITY: TransactionPriority = TransactionPriority::MAX / 2;

#[derive(
	Clone, Copy, Encode, Decode, RuntimeDebug, PartialEq, PartialOrd, Eq, TypeInfo, MaxEncodedLen,
)]
pub enum VtxDistStatus {
	Disabled,
	Enabled,
	Triggering,
	Triggered,
	Paying,
	Done,
}

impl Default for VtxDistStatus {
	fn default() -> Self {
		Self::Disabled
	}
}

type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_staking::Config + SendTransactionTypes<Call<Self>>
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		/// Multi currency
		type MultiCurrency: CreateExt<AccountId = Self::AccountId>
			+ fungibles::Inspect<Self::AccountId, AssetId = AssetId>
			+ fungibles::metadata::Inspect<Self::AccountId>
			+ fungibles::Mutate<Self::AccountId, Balance = BalanceOf<Self>>;

		/// The native token asset Id (managed by pallet-balances)
		#[pallet::constant]
		type NativeAssetId: Get<AssetId>;

		/// Vortex token asset Id
		#[pallet::constant]
		type VtxAssetId: Get<AssetId>;

		/// Vortex vault pot id that holds fresh minted vortex
		#[pallet::constant]
		type VtxHeldPotId: Get<PalletId>;

		/// Vortex distribution pot id
		#[pallet::constant]
		type VtxDistPotId: Get<PalletId>;

		/// Vortex root pot id
		#[pallet::constant]
		type RootPotId: Get<PalletId>;

		/// Tx fee pot id
		#[pallet::constant]
		type TxFeePotId: Get<PalletId>;

		/// Unsigned transaction interval
		#[pallet::constant]
		type UnsignedInterval: Get<BlockNumberFor<Self>>;

		/// Payout batch size
		#[pallet::constant]
		type PayoutBatchSize: Get<u32>;

		/// Max asset prices items
		type MaxAssetPrices: Get<u32>;

		/// Max rewards items
		type MaxRewards: Get<u32>;

		/// Max pivot string length
		type MaxStringLength: Get<u32>;

		/// Vortex distribution identifier
		type VtxDistIdentifier: Member
			+ Parameter
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ HasCompact
			+ MaxEncodedLen;

		/// History depth
		#[pallet::constant]
		type HistoryDepth: Get<u32>;
	}

	#[pallet::storage]
	pub(super) type AdminAccount<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pallet::storage]
	pub(super) type NextVortexId<T: Config> = StorageValue<_, T::VtxDistIdentifier, ValueQuery>;

	/// Stores balance consideration criteria, current or stored
	#[pallet::storage]
	pub(super) type ConsiderCurrentBalance<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Stores disable redeem
	#[pallet::storage]
	pub(super) type DisableRedeem<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Stores enabling manual reward input
	#[pallet::storage]
	pub(super) type EnableManualRewardInput<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Stores VtxVault latest asset id list that can be redeemed.
	#[pallet::storage]
	pub(super) type VtxVaultRedeemAssetList<T: Config> =
		StorageValue<_, BoundedVec<AssetId, T::MaxAssetPrices>, ValueQuery>;

	/// Stores total Reward points for each cycle when the rewards are registered.
	#[pallet::storage]
	pub(super) type TotalRewardPoints<T: Config> =
		StorageMap<_, Twox64Concat, T::VtxDistIdentifier, BalanceOf<T>, ValueQuery>;

	/// Stores total work points for each cycle when the work points are registered.
	#[pallet::storage]
	pub(super) type TotalWorkPoints<T: Config> =
		StorageMap<_, Twox64Concat, T::VtxDistIdentifier, BalanceOf<T>, ValueQuery>;

	/// Stores status of each vortex distribution
	#[pallet::storage]
	pub type VtxDistStatuses<T: Config> =
		StorageMap<_, Twox64Concat, T::VtxDistIdentifier, VtxDistStatus, ValueQuery>;

	/// Stores Vtx total supply for each vortex distribution
	#[pallet::storage]
	pub type VtxTotalSupply<T: Config> =
		StorageMap<_, Twox64Concat, T::VtxDistIdentifier, BalanceOf<T>, ValueQuery>;

	/// Stores Vtx price each vortex distribution
	#[pallet::storage]
	pub type VtxPrice<T: Config> =
		StorageMap<_, Twox64Concat, T::VtxDistIdentifier, BalanceOf<T>, ValueQuery>;

	/// Stores order books for each vortex distribution
	#[pallet::storage]
	pub(super) type VtxDistOrderbook<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::VtxDistIdentifier,
		Blake2_128Concat,
		T::AccountId,
		(BalanceOf<T>, bool), //here balance is the reward amount to payout
		ValueQuery,
		GetDefault,
		ConstU32<{ u32::MAX }>,
	>;

	/// Stores reward points for each account, each vortex distribution
	#[pallet::storage]
	pub(super) type RewardPoints<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::VtxDistIdentifier,
		Blake2_128Concat,
		T::AccountId,
		BalanceOf<T>, // balance is the reward points for each account
		ValueQuery,
	>;

	/// Stores work points for each account, each vortex distribution
	#[pallet::storage]
	pub(super) type WorkPoints<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::VtxDistIdentifier,
		Blake2_128Concat,
		T::AccountId,
		BalanceOf<T>, // balance is the work points for each account
		ValueQuery,
	>;

	/// Stores Fee pot asset list for each vortex distribution
	#[pallet::storage]
	pub(super) type FeePotAssetsList<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::VtxDistIdentifier,
		BoundedVec<(AssetId, BalanceOf<T>), T::MaxAssetPrices>,
		ValueQuery,
	>;

	/// Stores Vortex vault asset list for each vortex distribution
	#[pallet::storage]
	pub(super) type VtxVaultAssetsList<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::VtxDistIdentifier,
		BoundedVec<(AssetId, BalanceOf<T>), T::MaxAssetPrices>,
		ValueQuery,
	>;

	/// Stores asset prices for each vortex distribution
	#[pallet::storage]
	pub(super) type AssetPrices<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::VtxDistIdentifier,
		Twox64Concat,
		AssetId,
		BalanceOf<T>,
		ValueQuery,
	>;

	/// Stores total network reward for each distribution
	#[pallet::storage]
	pub(super) type TotalNetworkReward<T: Config> =
		StorageMap<_, Twox64Concat, T::VtxDistIdentifier, BalanceOf<T>, ValueQuery>;

	/// Stores total bootstrap reward for each distribution
	#[pallet::storage]
	pub(super) type TotalBootstrapReward<T: Config> =
		StorageMap<_, Twox64Concat, T::VtxDistIdentifier, BalanceOf<T>, ValueQuery>;

	/// Stores total vortex amount for each distribution
	#[pallet::storage]
	pub(super) type TotalVortex<T: Config> =
		StorageMap<_, Twox64Concat, T::VtxDistIdentifier, BalanceOf<T>, ValueQuery>;

	/// Stores next unsigned tx block number
	#[pallet::storage]
	pub(super) type NextUnsignedAt<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// Stores payout pivot block for each vortex distribution
	#[pallet::storage]
	pub(super) type VtxDistPayoutPivot<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::VtxDistIdentifier,
		BoundedVec<u8, T::MaxStringLength>,
		ValueQuery,
	>;

	/// Stores reward calculation pivot block for each vortex distribution
	#[pallet::storage]
	pub(super) type VtxRewardCalculationPivot<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::VtxDistIdentifier,
		BoundedVec<u8, T::MaxStringLength>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Admin Account changed
		AdminAccountChanged { old_key: Option<T::AccountId>, new_key: T::AccountId },

		/// Distribution created
		VtxDistCreated { id: T::VtxDistIdentifier },

		/// Distribution disabled
		VtxDistDisabled { id: T::VtxDistIdentifier },

		/// Distribution done
		VtxDistDone { id: T::VtxDistIdentifier },

		/// Distribution paid out
		VtxDistPaidOut { id: T::VtxDistIdentifier, who: T::AccountId, amount: BalanceOf<T> },

		/// Distribution started
		VtxDistStarted { id: T::VtxDistIdentifier },

		/// Set Fee pot asset balances
		SetFeePotAssetBalances {
			id: T::VtxDistIdentifier,
			assets_balances: BoundedVec<(AssetId, BalanceOf<T>), T::MaxAssetPrices>,
		},

		/// Set Vtx vault asset balances
		SetVtxVaultAssetBalances {
			id: T::VtxDistIdentifier,
			assets_balances: BoundedVec<(AssetId, BalanceOf<T>), T::MaxAssetPrices>,
		},

		/// Set asset prices
		SetAssetPrices {
			id: T::VtxDistIdentifier,
			asset_prices: BoundedVec<(AssetId, BalanceOf<T>), T::MaxAssetPrices>,
		},

		/// Vtx work points registered
		VtxWorkPointRegistered {
			id: T::VtxDistIdentifier,
			work_points: BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxRewards>,
		},

		/// Vtx staker reward points registered
		VtxRewardPointRegistered {
			id: T::VtxDistIdentifier,
			reward_points: BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxRewards>,
		},

		/// Vtx distribution triggered
		VtxDistributionTriggered { id: T::VtxDistIdentifier },

		/// Vtx distribution triggering
		VtxDistributionTriggering { id: T::VtxDistIdentifier },

		/// Set Vtx total supply
		SetVtxTotalSupply { id: T::VtxDistIdentifier, total_supply: BalanceOf<T> },

		/// Set ConsiderCurrentBalance
		SetConsiderCurrentBalance { value: bool },

		/// Set DisableRedeem
		SetDisableRedeem { value: bool },

		/// Set VtxVaultRedeemAssetList
		SetVtxVaultRedeemAssetList { asset_list: BoundedVec<AssetId, T::MaxAssetPrices> },

		/// Vortex redeemed
		VtxRedeemed { who: T::AccountId, amount: BalanceOf<T> },

		/// Set EnableManualRewardInput
		SetEnableManualRewardInput { value: bool },

		/// Rewards registered
		RewardRegistered {
			id: T::VtxDistIdentifier,
			rewards: BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxRewards>,
		},

		/// Pivot key string is too long and exceeds MaxStringLength
		PivotStringTooLong { id: T::VtxDistIdentifier },
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		// vtx reward calculation
		fn on_idle(_now: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			let calculation_weight = Self::do_reward_calculation(remaining_weight);
			calculation_weight
		}

		// Vtx reward distribution
		fn offchain_worker(now: BlockNumberFor<T>) {
			if let Err(e) = Self::vtx_dist_offchain_worker(now) {
				log::info!(
				  target: "vtx-dist",
				  "offchain worker not triggered at {:?}: {:?}",
				  now,
				  e,
				);
			} else {
				log::debug!(
				  target: "vtx-dist",
				  "offchain worker start at block: {:?} already done!",
				  now,
				);
			}
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Require to be previous admin
		RequireAdmin,

		/// No available Dist id
		VtxDistIdNotAvailable,

		/// Vortex distribution already enabled
		VtxDistAlreadyEnabled,

		/// Vortex distribution disabled
		VtxDistDisabled,

		/// No Vtx asset minted
		NoVtxAssetMinted,

		/// Invalid amount
		InvalidAmount,

		/// ID already in use
		VtxDistIdInUse,

		/// Not a validator
		NotAValidator,

		/// Vortex period not set
		VortexPeriodNotSet,

		/// Pivot string too long
		PivotStringTooLong,

		/// Assets should not include vortex asset
		AssetsShouldNotIncludeVtxAsset,

		/// Vortex distribution is not ready to be triggered
		CannotTrigger,

		/// Vortex distribution is not ready to be redeemed
		CannotRedeem,

		/// Vortex distribution not triggered
		NotTriggered,

		/// out of max reward vecotor bound
		ExceededMaxRewards,

		/// asset (price set) is not in the fee pot assets list
		AssetNotInFeePotList,

		/// vortex price is zero
		VortexPriceIsZero,

		/// root price is zero
		RootPriceIsZero,

		/// Vtx redeem disabled
		VtxRedeemDisabled,

		/// Manual reward input is disabled
		ManualRewardInputDisabled,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_admin())]
		pub fn set_admin(origin: OriginFor<T>, new: AccountIdLookupOf<T>) -> DispatchResult {
			ensure_root(origin)?;

			let new = T::Lookup::lookup(new)?;
			let old_key = AdminAccount::<T>::get();
			AdminAccount::<T>::put(&new);
			Self::deposit_event(Event::AdminAccountChanged { old_key, new_key: new });
			Ok(())
		}

		/// List a vortex distribution
		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_vtx_dist())]
		#[transactional]
		pub fn create_vtx_dist(origin: OriginFor<T>) -> DispatchResult {
			Self::ensure_root_or_admin(origin)?;

			let id = NextVortexId::<T>::get();
			let next_pool_id =
				id.checked_add(&One::one()).ok_or(Error::<T>::VtxDistIdNotAvailable)?;

			NextVortexId::<T>::mutate(|next_id| {
				*next_id = next_pool_id;
			});
			VtxDistStatuses::<T>::insert(id, VtxDistStatus::Enabled);
			Self::deposit_event(Event::VtxDistCreated { id });
			Ok(())
		}

		/// Disable a distribution
		///
		/// `id` - The distribution id
		#[pallet::call_index(2)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::disable_vtx_dist())]
		#[transactional]
		pub fn disable_vtx_dist(origin: OriginFor<T>, id: T::VtxDistIdentifier) -> DispatchResult {
			Self::ensure_root_or_admin(origin)?;
			ensure!(
				VtxDistStatuses::<T>::get(id) != VtxDistStatus::Disabled,
				Error::<T>::VtxDistDisabled
			);
			Self::do_disable_vtx_dist(id);
			Self::deposit_event(Event::VtxDistDisabled { id });
			Ok(())
		}

		/// Set fee pot assets balances
		///
		/// `id` - The distribution id
		/// `assets_balances` - List of asset balances
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::set_fee_pot_asset_balances(assets_balances.len() as u32))]
		pub fn set_fee_pot_asset_balances(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			assets_balances: BoundedVec<(AssetId, BalanceOf<T>), T::MaxAssetPrices>,
		) -> DispatchResultWithPostInfo {
			Self::ensure_root_or_admin(origin)?;
			Self::do_fee_pot_asset_balances_setter(id, assets_balances)
		}

		/// Set vtx vault assets balances
		///
		/// `id` - The distribution id
		/// `assets_balances` - List of asset balances
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::set_vtx_vault_asset_balances(assets_balances.len() as u32))]
		pub fn set_vtx_vault_asset_balances(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			assets_balances: BoundedVec<(AssetId, BalanceOf<T>), T::MaxAssetPrices>,
		) -> DispatchResultWithPostInfo {
			Self::ensure_root_or_admin(origin)?;
			Self::do_vtx_vault_asset_balances_setter(id, assets_balances)
		}

		/// Set vtx total supply for each vortex distribution
		///
		/// `id` - The distribution id
		/// `supply` - Vtx total supply
		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::set_vtx_total_supply())]
		pub fn set_vtx_total_supply(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			supply: BalanceOf<T>,
		) -> DispatchResult {
			Self::ensure_root_or_admin(origin)?;
			VtxTotalSupply::<T>::set(id, supply);

			Self::deposit_event(Event::SetVtxTotalSupply { id, total_supply: supply });
			Ok(())
		}

		/// Register rewards point distribution
		///
		/// `id` - The distribution id
		/// `reward_points` - Reward point list
		#[pallet::call_index(6)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::register_reward_points(reward_points.len() as u32))]
		pub fn register_reward_points(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			reward_points: BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxRewards>,
		) -> DispatchResultWithPostInfo {
			Self::ensure_root_or_admin(origin)?;
			let dst_status = VtxDistStatuses::<T>::get(id);
			ensure!(dst_status == VtxDistStatus::Enabled, Error::<T>::VtxDistDisabled);
			let mut total_reward_points = TotalRewardPoints::<T>::get(id);
			for (account, r_points) in reward_points.clone() {
				let current_r_points = RewardPoints::<T>::get(id, account.clone());
				if current_r_points != Default::default() {
					// means we need to minus the current_r_points and plus r_points from the total_reward_points
					total_reward_points = total_reward_points
						.saturating_sub(current_r_points)
						.saturating_add(r_points);
				} else {
					// just add
					total_reward_points = total_reward_points.saturating_add(r_points);
				}
				RewardPoints::<T>::insert(id, account, r_points);
			}
			TotalRewardPoints::<T>::set(id, total_reward_points);
			Self::deposit_event(Event::VtxRewardPointRegistered { id, reward_points });

			Ok(Pays::No.into())
		}

		/// Register work point distribution
		///
		/// `id` - The distribution id
		/// `work_points` - work point list
		#[pallet::call_index(7)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::register_work_points(work_points.len() as u32))]
		pub fn register_work_points(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			work_points: BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxRewards>,
		) -> DispatchResultWithPostInfo {
			Self::ensure_root_or_admin(origin)?;
			let dst_status = VtxDistStatuses::<T>::get(id);
			ensure!(dst_status == VtxDistStatus::Enabled, Error::<T>::VtxDistDisabled);
			let mut total_work_points = TotalWorkPoints::<T>::get(id);
			for (account, w_points) in work_points.clone() {
				let current_work_points = WorkPoints::<T>::get(id, account.clone());
				if current_work_points != Default::default() {
					// means we need to minus the current_work_points and plus w_points from the total_reward_points
					total_work_points = total_work_points
						.saturating_sub(current_work_points)
						.saturating_add(w_points);
				} else {
					// just add
					total_work_points = total_work_points.saturating_add(w_points);
				}
				WorkPoints::<T>::insert(id, account, w_points);
			}
			TotalWorkPoints::<T>::set(id, total_work_points);
			Self::deposit_event(Event::VtxWorkPointRegistered { id, work_points });

			Ok(Pays::No.into())
		}

		/// Set ConsiderCurrentBalance storage item
		/// If set to true, token balances at the current block will be taken into account for reward calculation
		#[pallet::call_index(8)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_consider_current_balance())]
		pub fn set_consider_current_balance(origin: OriginFor<T>, value: bool) -> DispatchResult {
			Self::ensure_root_or_admin(origin)?;

			ConsiderCurrentBalance::<T>::put(value);
			Self::deposit_event(Event::SetConsiderCurrentBalance { value });
			Ok(())
		}

		/// Set DisableRedeem storage item
		/// If set to true, users would not be able to redeem Vtx tokens
		#[pallet::call_index(9)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_disable_redeem())]
		pub fn set_disable_redeem(origin: OriginFor<T>, value: bool) -> DispatchResult {
			Self::ensure_root_or_admin(origin)?;

			crate::pallet::DisableRedeem::<T>::put(value);
			Self::deposit_event(crate::pallet::Event::SetDisableRedeem { value });
			Ok(())
		}

		/// Set EnableManualRewardInput storage item
		/// If set to true, reward inputs can be given externally, this supports the legacy method
		#[pallet::call_index(10)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_enable_manual_reward_input())]
		pub fn set_enable_manual_reward_input(origin: OriginFor<T>, value: bool) -> DispatchResult {
			Self::ensure_root_or_admin(origin)?;

			EnableManualRewardInput::<T>::put(value);
			Self::deposit_event(crate::pallet::Event::SetEnableManualRewardInput { value });
			Ok(())
		}

		/// Set asset prices
		///
		/// `asset_prices` - List of asset prices
		/// `id` - The distribution id
		#[pallet::call_index(11)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_asset_prices(asset_prices.len() as u32))]
		#[transactional]
		pub fn set_asset_prices(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			asset_prices: BoundedVec<(AssetId, BalanceOf<T>), T::MaxAssetPrices>,
		) -> DispatchResultWithPostInfo {
			Self::ensure_root_or_admin(origin)?;
			Self::do_asset_price_setter(asset_prices, id)
		}

		/// Trigger distribution
		///
		/// `id` - The distribution id
		#[pallet::call_index(12)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::trigger_vtx_distribution())]
		#[transactional]
		pub fn trigger_vtx_distribution(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
		) -> DispatchResultWithPostInfo {
			Self::ensure_root_or_admin(origin)?;

			ensure!(
				VtxDistStatuses::<T>::get(id) == VtxDistStatus::Enabled,
				Error::<T>::CannotTrigger
			);

			Self::do_calculate_vortex_price(id)?;
			Self::do_collate_reward_tokens(id)?;
			// Do the reward calculation if the EnableManualRewardInput is disabled.
			if !EnableManualRewardInput::<T>::get() {
				VtxDistStatuses::<T>::mutate(id, |status| {
					*status = VtxDistStatus::Triggering;
				});
				Self::deposit_event(Event::VtxDistributionTriggering { id });
			} else {
				VtxDistStatuses::<T>::mutate(id, |status| {
					*status = VtxDistStatus::Triggered;
				});
				Self::deposit_event(Event::VtxDistributionTriggered { id });
			}

			Ok(Pays::No.into())
		}

		/// Set vtx vault redeem assets list
		///
		/// `assets_list` - List of assets available to redeem
		#[pallet::call_index(13)]
		#[pallet::weight(<T as Config>::WeightInfo::set_vtx_vault_redeem_asset_list(assets_list.len() as u32))]
		pub fn set_vtx_vault_redeem_asset_list(
			origin: OriginFor<T>,
			assets_list: BoundedVec<AssetId, T::MaxAssetPrices>,
		) -> DispatchResultWithPostInfo {
			Self::ensure_root_or_admin(origin)?;
			VtxVaultRedeemAssetList::<T>::set(assets_list.clone());
			Self::deposit_event(Event::SetVtxVaultRedeemAssetList { asset_list: assets_list });

			Ok(Pays::No.into())
		}

		/// Start distributing vortex
		///
		/// `id` - The distribution id
		#[pallet::call_index(14)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::start_vtx_dist())]
		pub fn start_vtx_dist(origin: OriginFor<T>, id: T::VtxDistIdentifier) -> DispatchResult {
			Self::ensure_root_or_admin(origin)?;
			ensure!(
				VtxDistStatuses::<T>::get(id) == VtxDistStatus::Triggered,
				Error::<T>::NotTriggered
			);

			Self::do_start_vtx_dist(id)?;
			Self::deposit_event(Event::VtxDistStarted { id });
			Ok(())
		}

		/// Unsigned distribution of vortex, called by offchain worker
		///
		/// `id` - The distribution id
		/// `current_block` - Current block number
		#[pallet::call_index(15)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::pay_unsigned().saturating_mul(T::PayoutBatchSize::get().into()))]
		#[transactional]
		pub fn pay_unsigned(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			_current_block: BlockNumberFor<T>,
		) -> DispatchResult {
			ensure_none(origin)?;
			if let VtxDistStatus::Paying = VtxDistStatuses::<T>::get(id) {
				let vtx_held_account = Self::get_vtx_held_account();
				let start_key = VtxDistPayoutPivot::<T>::get(id);
				let payout_pivot: Vec<u8> = start_key.clone().into_inner();

				let mut map_iterator = match VtxDistPayoutPivot::<T>::contains_key(id) {
					true => <VtxDistOrderbook<T>>::iter_prefix_from(id, payout_pivot),
					false => <VtxDistOrderbook<T>>::iter_prefix(id),
				};

				let mut count = 0u32;
				for (who, entry) in map_iterator.by_ref() {
					// if the user is already paid out, skip
					if entry.1 {
						continue;
					}

					let share = entry.0;
					let transfer_result = Self::safe_transfer(
						T::VtxAssetId::get(),
						&vtx_held_account,
						&who,
						share,
						false,
					);

					if transfer_result.is_ok() {
						Self::deposit_event(Event::VtxDistPaidOut {
							id,
							who: who.clone(),
							amount: share,
						});
					}
					VtxDistOrderbook::<T>::mutate(id, who.clone(), |entry| {
						*entry = (entry.0, true);
					});
					count += 1;
					if count > T::PayoutBatchSize::get() {
						break;
					}
				}
				let current_last_raw_key: BoundedVec<u8, T::MaxStringLength> =
					BoundedVec::try_from(map_iterator.last_raw_key().to_vec())
						.map_err(|_| Error::<T>::PivotStringTooLong)?;

				if current_last_raw_key == start_key.clone() {
					VtxDistStatuses::<T>::mutate(id, |status| {
						*status = VtxDistStatus::Done;
					});
					Self::deposit_event(Event::VtxDistDone { id });
				}
				VtxDistPayoutPivot::<T>::insert(id, current_last_raw_key);
			}

			let current_block = <frame_system::Pallet<T>>::block_number();
			let next_unsigned_at = current_block + T::UnsignedInterval::get();
			<NextUnsignedAt<T>>::put(next_unsigned_at);
			Ok(())
		}

		/// Redeem tokens from vault
		///
		/// `id` - The distribution id
		/// `vortex_token_amount` - Amount of vortex to redeem
		#[pallet::call_index(16)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::redeem_tokens_from_vault())]
		#[transactional]
		pub fn redeem_tokens_from_vault(
			origin: OriginFor<T>,
			vortex_token_amount: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(!DisableRedeem::<T>::get(), Error::<T>::VtxRedeemDisabled);
			let vault_account = Self::get_vtx_vault_account();
			let total_vortex = T::MultiCurrency::total_issuance(T::VtxAssetId::get());
			let vortex_balance = vortex_token_amount;
			ensure!(total_vortex > Zero::zero(), Error::<T>::NoVtxAssetMinted);
			ensure!(
				vortex_balance > Zero::zero()
					&& vortex_balance <= T::MultiCurrency::balance(T::VtxAssetId::get(), &who),
				Error::<T>::InvalidAmount
			);

			for asset_id in VtxVaultRedeemAssetList::<T>::get().into_iter() {
				// First, we calculate the ratio between the asset balance and the total vortex
				// issued. then multiply it with the vortex token amount the user wants to redeem to
				// get the resulting asset token amount.
				let asset_balance = T::MultiCurrency::balance(asset_id, &vault_account);
				let redeem_amount = vortex_balance.saturating_mul(asset_balance) / total_vortex;

				Self::safe_transfer(asset_id, &vault_account, &who, redeem_amount, false)?;
			}

			// Burn the vortex token
			T::MultiCurrency::burn_from(
				T::VtxAssetId::get(),
				&who,
				vortex_token_amount,
				Precision::Exact,
				Fortitude::Polite,
			)?;
			Self::deposit_event(Event::VtxRedeemed { who, amount: vortex_balance });

			Ok(())
		}

		/// Register rewards( manual input)
		///
		/// `id` - The distribution id
		/// `rewards` - Rewards list
		#[pallet::call_index(17)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::register_rewards(rewards.len() as u32))]
		pub fn register_rewards(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			rewards: BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxRewards>,
		) -> DispatchResultWithPostInfo {
			Self::ensure_root_or_admin(origin)?;
			ensure!(EnableManualRewardInput::<T>::get(), Error::<T>::ManualRewardInputDisabled);
			let s = VtxDistStatuses::<T>::get(id);
			match s {
				VtxDistStatus::Enabled | VtxDistStatus::Triggered => {
					let mut total_rewards = TotalVortex::<T>::get(id);
					for (who, amount) in rewards.iter() {
						let current_reward = VtxDistOrderbook::<T>::get(id, who.clone()).0;
						if current_reward != Default::default() {
							// means we need to minus the current_reward and plus amount from the total_rewards
							total_rewards = total_rewards
								.saturating_sub(current_reward)
								.saturating_add(*amount);
						} else {
							// just add
							total_rewards = total_rewards.saturating_add(*amount);
						}
						VtxDistOrderbook::<T>::insert(id, who, (*amount, false));
					}
					TotalVortex::<T>::set(id, total_rewards);
					Self::deposit_event(Event::RewardRegistered { id, rewards });
					Ok(Pays::No.into())
				},
				_ => Err(Error::<T>::VtxDistDisabled)?,
			}
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			match call {
				Call::pay_unsigned { id: _, current_block } => {
					// Let's make sure to reject transactions from the future.
					let _current_block = <frame_system::Pallet<T>>::block_number();
					if &_current_block < current_block {
						return InvalidTransaction::Future.into();
					}
					ValidTransaction::with_tag_prefix("VtxDistChainWorker")
						.priority(VTX_DIST_UNSIGNED_PRIORITY)
						.and_provides(current_block)
						.longevity(64_u64)
						.propagate(true)
						.build()
				},
				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	impl<T: Config> Pallet<T> {
		/// Account id of vtx vault asset which will hold the minted vortex
		pub fn get_vtx_held_account() -> T::AccountId {
			T::VtxHeldPotId::get().into_account_truncating()
		}

		/// Account id of vtx asset.
		pub fn get_vtx_vault_account() -> T::AccountId {
			T::VtxDistPotId::get().into_account_truncating()
		}

		/// Get root vault account
		pub fn get_root_vault_account() -> T::AccountId {
			T::RootPotId::get().into_account_truncating()
		}

		/// Get fee vault account
		pub fn get_fee_vault_account() -> T::AccountId {
			T::TxFeePotId::get().into_account_truncating()
		}

		/// disable a distribution
		fn do_disable_vtx_dist(id: T::VtxDistIdentifier) {
			VtxDistStatuses::<T>::mutate(id, |status| {
				*status = VtxDistStatus::Disabled;
			});
		}

		/// start a distribution
		fn do_start_vtx_dist(id: T::VtxDistIdentifier) -> DispatchResult {
			let vtx_held_account = Self::get_vtx_held_account();
			let total_vortex = TotalVortex::<T>::get(id);
			T::MultiCurrency::mint_into(T::VtxAssetId::get(), &vtx_held_account, total_vortex)?;

			VtxDistStatuses::<T>::mutate(id, |status| {
				*status = VtxDistStatus::Paying;
			});
			Ok(())
		}

		/// set fee pot asset balances
		fn do_fee_pot_asset_balances_setter(
			id: T::VtxDistIdentifier,
			assets_balances: BoundedVec<(AssetId, BalanceOf<T>), T::MaxAssetPrices>,
		) -> DispatchResultWithPostInfo {
			for (asset_id, _) in &assets_balances {
				ensure!(
					asset_id != &T::VtxAssetId::get(),
					Error::<T>::AssetsShouldNotIncludeVtxAsset
				);
			}
			FeePotAssetsList::<T>::insert(id, assets_balances.clone());

			Self::deposit_event(Event::SetFeePotAssetBalances { id, assets_balances });
			Ok(Pays::No.into())
		}

		/// set vtx vault asset balances
		fn do_vtx_vault_asset_balances_setter(
			id: T::VtxDistIdentifier,
			assets_balances: BoundedVec<(AssetId, BalanceOf<T>), T::MaxAssetPrices>,
		) -> DispatchResultWithPostInfo {
			for (asset_id, _) in &assets_balances {
				ensure!(
					asset_id != &T::VtxAssetId::get(),
					Error::<T>::AssetsShouldNotIncludeVtxAsset
				);
			}
			VtxVaultAssetsList::<T>::insert(id, assets_balances.clone());

			Self::deposit_event(Event::SetVtxVaultAssetBalances { id, assets_balances });
			Ok(Pays::No.into())
		}

		/// set asset prices
		fn do_asset_price_setter(
			asset_prices: BoundedVec<(AssetId, BalanceOf<T>), T::MaxAssetPrices>,
			id: T::VtxDistIdentifier,
		) -> DispatchResultWithPostInfo {
			for (asset_id, price) in &asset_prices {
				ensure!(
					asset_id != &T::VtxAssetId::get(),
					Error::<T>::AssetsShouldNotIncludeVtxAsset
				);
				ensure!(
					Self::check_asset_exist_in_fee_pot_asset_list(id, asset_id),
					Error::<T>::AssetNotInFeePotList
				);
				AssetPrices::<T>::insert(id, asset_id, price);
			}

			Self::deposit_event(Event::SetAssetPrices { id, asset_prices });
			Ok(Pays::No.into())
		}

		/// Calculate vortex price
		fn do_calculate_vortex_price(id: T::VtxDistIdentifier) -> DispatchResultWithPostInfo {
			let vtx_vault_account = Self::get_vtx_vault_account();

			let mut vtx_vault_asset_value: BalanceOf<T> = 0u64.into();
			for (asset_id, amount) in VtxVaultAssetsList::<T>::get(id).into_iter() {
				let asset_price = AssetPrices::<T>::get(id, asset_id);
				if asset_price == Default::default() {
					continue;
				}
				let asset_balance = match ConsiderCurrentBalance::<T>::get() {
					true => T::MultiCurrency::balance(asset_id, &vtx_vault_account),
					false => amount,
				};
				vtx_vault_asset_value += asset_balance.saturating_mul(asset_price);
			}

			let vtx_existing_supply = match ConsiderCurrentBalance::<T>::get() {
				true => T::MultiCurrency::total_issuance(T::VtxAssetId::get()),
				false => VtxTotalSupply::<T>::get(id),
			};

			let vortex_price = if vtx_existing_supply == Zero::zero() {
				1u64.into() // should be still 1 not matter decimal points (6 decimal)
			} else {
				vtx_vault_asset_value / vtx_existing_supply
			};
			ensure!(vortex_price > Zero::zero(), Error::<T>::VortexPriceIsZero);
			VtxPrice::<T>::set(id, vortex_price);

			Ok(().into())
		}

		// do collate assets into the vtx vault account
		fn do_collate_reward_tokens(id: T::VtxDistIdentifier) -> DispatchResultWithPostInfo {
			let root_price = AssetPrices::<T>::get(id, T::NativeAssetId::get());

			let vtx_vault_account = Self::get_vtx_vault_account();
			let root_vault_account = Self::get_root_vault_account();
			let fee_vault_account = Self::get_fee_vault_account();

			// move gas & network fee to  vtx vault here
			// move all asset in fee_vault to vtx_vault_account based on asset list in FeePotAssetsList
			let mut fee_vault_asset_value: BalanceOf<T> = 0u64.into();
			for (asset_id, amount) in FeePotAssetsList::<T>::get(id).into_iter() {
				let asset_price = AssetPrices::<T>::get(id, asset_id);
				let asset_balance = match ConsiderCurrentBalance::<T>::get() {
					true => T::MultiCurrency::balance(asset_id, &fee_vault_account),
					false => amount,
				};
				fee_vault_asset_value += asset_balance.saturating_mul(asset_price);
				Self::safe_transfer(
					asset_id,
					&fee_vault_account,
					&vtx_vault_account,
					asset_balance,
					false,
				)?;
			}

			// bootstrap - move root token from root_vault to vtx_vault_account
			// TODO: change this to move only the required balance from the root vault account once
			// we let go of the legacy system
			let root_vault_root_token_balance =
				T::MultiCurrency::balance(T::NativeAssetId::get(), &root_vault_account);
			let root_vault_root_value: BalanceOf<T> = root_vault_root_token_balance * root_price;
			Self::safe_transfer(
				T::NativeAssetId::get(),
				&root_vault_account,
				&vtx_vault_account,
				root_vault_root_token_balance,
				false,
			)?;

			// fetch calculated vortex price
			let vortex_price = VtxPrice::<T>::get(id);
			ensure!(vortex_price > Zero::zero(), Error::<T>::VortexPriceIsZero);

			//calculate total rewards
			let total_vortex_network_reward: BalanceOf<T> = fee_vault_asset_value / vortex_price;
			let total_vortex_bootstrap: BalanceOf<T> = root_vault_root_value / vortex_price;
			let total_vortex = total_vortex_network_reward.saturating_add(total_vortex_bootstrap);

			// store TotalVortex only if EnableManualRewardInput is false
			// otherwise in manual mode the TotalVortex will be calculated from the input.
			if !EnableManualRewardInput::<T>::get() {
				TotalVortex::<T>::insert(id, total_vortex);
			}
			TotalNetworkReward::<T>::insert(id, total_vortex_network_reward);
			TotalBootstrapReward::<T>::insert(id, total_vortex_bootstrap);

			Ok(().into())
		}

		fn do_reward_calculation(remaining_weight: Weight) -> Weight {
			// Read: NextVortexId, VtxDistStatuses
			let mut used_weight = DbWeight::get().reads(2);
			if remaining_weight.ref_time() <= DbWeight::get().reads(2).ref_time() {
				return used_weight;
			}
			// get the current vtx distribution id
			let id = NextVortexId::<T>::get().saturating_sub(One::one());

			if let VtxDistStatus::Triggering = VtxDistStatuses::<T>::get(id) {
				// Initial reads and writes for the following:
				// Read: TotalNetworkReward, TotalBootstrapReward, TotalRewardPoints,
				// TotalWorkPoints, VtxRewardCalculationPivot,
				// Write: VtxDistStatuses, VtxRewardCalculationPivot
				let base_process_weight = DbWeight::get().reads_writes(5u64, 2);
				// the weight per transaction is at least two writes
				// Reads: reading map_iterator RewardPoints, WorkPoints,
				// Writes: VtxDistOrderbook
				let min_weight_per_index = DbWeight::get().reads_writes(2, 1);
				// Ensure we have enough weight to perform the initial reads + at least one reward calculation
				if remaining_weight.ref_time()
					<= (base_process_weight + min_weight_per_index).ref_time()
				{
					return used_weight;
				}

				// fetch and calculate reward pool balances
				let total_network_reward = TotalNetworkReward::<T>::get(id);
				let total_bootstrap_reward = TotalBootstrapReward::<T>::get(id);
				// Ref -> https://docs.therootnetwork.com/intro/learn/tokenomics#how-are-rewards-distributed
				let total_staker_pool = total_bootstrap_reward
					.saturating_add(Perbill::from_percent(30) * total_network_reward); // bootstrap + 30% of network rewards
				let total_workpoints_pool = Perbill::from_percent(70) * total_network_reward; // 70% of network rewards
				let total_staker_points = TotalRewardPoints::<T>::get(id);
				let total_work_points = TotalWorkPoints::<T>::get(id);

				// start key
				let start_key = VtxRewardCalculationPivot::<T>::get(id);
				let calculation_pivot: Vec<u8> = start_key.clone().into_inner();

				let mut map_iterator = match start_key.is_empty() {
					true => <RewardPoints<T>>::iter_prefix(id),
					false => <RewardPoints<T>>::iter_prefix_from(id, calculation_pivot),
				};
				used_weight = base_process_weight;

				let mut count = 0u32;
				for (account_id, account_staker_points) in map_iterator.by_ref() {
					// Add weight for reading map_iterator
					used_weight = used_weight.saturating_add(DbWeight::get().reads(1));

					// Add weight for reading WorkPoints
					used_weight = used_weight.saturating_add(DbWeight::get().reads(1));
					let account_work_points: BalanceOf<T> =
						WorkPoints::<T>::get(id, account_id.clone());

					let staker_point_portion =
						Perbill::from_rational(account_staker_points, total_staker_points);
					let work_points_portion =
						Perbill::from_rational(account_work_points, total_work_points);

					let account_work_point_reward = work_points_portion * total_workpoints_pool;
					let account_staker_reward = staker_point_portion * total_staker_pool;
					let final_reward =
						account_work_point_reward.saturating_add(account_staker_reward);

					// Add weight for writing VtxDistOrderbook
					used_weight = used_weight.saturating_add(DbWeight::get().writes(1));
					VtxDistOrderbook::<T>::mutate(id, account_id.clone(), |entry| {
						*entry = (entry.0.saturating_add(final_reward), entry.1);
					});
					count += 1;

					// if no remaining_weight for the next entry iteration, brek
					if remaining_weight.ref_time()
						<= used_weight.saturating_add(min_weight_per_index).ref_time()
					{
						break;
					}
					// if exceeds T::MaxRewards, break
					if count >= T::MaxRewards::get() {
						break;
					}
				}

				let Ok(current_last_raw_key) =
					BoundedVec::try_from(map_iterator.last_raw_key().to_vec())
				else {
					// Unlikely to happen. We can not error here, emit an event and return the consumed weight
					Self::deposit_event(Event::PivotStringTooLong { id });
					return used_weight;
				};
				if current_last_raw_key == start_key.clone() {
					VtxDistStatuses::<T>::mutate(id, |status| {
						*status = VtxDistStatus::Triggered;
					});
					Self::deposit_event(Event::VtxDistributionTriggered { id });
				}
				VtxRewardCalculationPivot::<T>::insert(id, current_last_raw_key);
			}

			used_weight
		}

		/// offchain worker for unsigned tx
		fn vtx_dist_offchain_worker(now: BlockNumberFor<T>) -> Result<(), OffchainErr> {
			if !sp_io::offchain::is_validator() {
				return Err(OffchainErr::NotAValidator);
			}
			let next_unsigned_at = <NextUnsignedAt<T>>::get();
			if next_unsigned_at > now {
				return Err(OffchainErr::TooEarly);
			}

			for (id, vtx_dist_status) in VtxDistStatuses::<T>::iter() {
				match vtx_dist_status {
					VtxDistStatus::Paying => {
						log::info!("start sending unsigned paying tx");
						let call = Call::pay_unsigned { id, current_block: now };
						let _ = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(
							call.into(),
						);
					},
					_ => continue,
				}
			}
			Ok(())
		}

		/// Safe transfer
		pub fn safe_transfer(
			asset_id: AssetId,
			source: &T::AccountId,
			dest: &T::AccountId,
			amount: BalanceOf<T>,
			_keep_live: bool,
		) -> DispatchResult {
			if amount == Zero::zero() {
				return Ok(());
			}
			let transfer_result = T::MultiCurrency::transfer(
				asset_id,
				source,
				dest,
				amount,
				Preservation::Expendable,
			)?;
			ensure!(transfer_result == amount, Error::<T>::InvalidAmount);
			Ok(())
		}

		fn ensure_root_or_admin(
			origin: OriginFor<T>,
		) -> Result<Option<T::AccountId>, DispatchError> {
			match ensure_signed_or_root(origin)? {
				Some(who) => {
					ensure!(
						AdminAccount::<T>::get().map_or(false, |k| who == k),
						Error::<T>::RequireAdmin
					);
					Ok(Some(who))
				},
				None => Ok(None),
			}
		}

		fn check_asset_exist_in_fee_pot_asset_list(
			vtx_id: T::VtxDistIdentifier,
			asset_id: &AssetId,
		) -> bool {
			for (id, _) in FeePotAssetsList::<T>::get(vtx_id).iter() {
				if id == asset_id {
					return true;
				}
			}

			false
		}
	}
}
