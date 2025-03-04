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
		tokens::fungibles::{self, Inspect, Mutate},
		Get,
	},
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
	traits::{AccountIdConversion, CheckedAdd, One, Saturating, StaticLookup, Zero},
	Perbill, RuntimeDebug,
};
use sp_staking::EraIndex;
use sp_std::{convert::TryInto, prelude::*};

pub const VTX_DIST_UNSIGNED_PRIORITY: TransactionPriority = TransactionPriority::MAX / 2;

#[derive(
	Clone, Copy, Encode, Decode, RuntimeDebug, PartialEq, PartialOrd, Eq, TypeInfo, MaxEncodedLen,
)]
pub enum VtxDistStatus {
	Disabled,
	Enabled,
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
	use frame_support::{
		traits::tokens::{Fortitude, Precision, Preservation},
		transactional,
	};
	use sp_runtime::traits::AtLeast32BitUnsigned;

	use super::*;

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

		/// Vortex fee pot id
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

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub(super) type AdminAccount<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pallet::storage]
	pub(super) type NextVortexId<T: Config> = StorageValue<_, T::VtxDistIdentifier, ValueQuery>;

	/// Stores status of each vortex distribution
	#[pallet::storage]
	pub type VtxDistStatuses<T: Config> =
		StorageMap<_, Twox64Concat, T::VtxDistIdentifier, VtxDistStatus, ValueQuery>;

	/// Stores start and end eras of each vortex distribution
	#[pallet::storage]
	pub type VtxDistEras<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::VtxDistIdentifier,
		(EraIndex, EraIndex), //start and end era, inclusive
		ValueQuery,
	>;

	/// Stores Vtx total supply for each vortex distribution
	#[pallet::storage]
	pub type VtxTotalSupply<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::VtxDistIdentifier,
		BalanceOf<T>, 
		ValueQuery,
	>;

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

	#[pallet::storage]
	#[pallet::getter(fn effective_balances_work_points)]
	pub type EffectiveBalancesWorkPoints<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Blake2_128Concat, T::VtxDistIdentifier>,
			NMapKey<Blake2_128Concat, EraIndex>,
			NMapKey<Blake2_128Concat, T::AccountId>,
		),
		(BalanceOf<T>, BalanceOf<T>, BalanceOf<T>), //effective balance, workpoints, rates
		ValueQuery,
	>;

	/// Stores penalty effective balances, work points, and rates for each vortex distribution
	#[pallet::storage]
	pub type PenaltyEffectiveBalancesWorkPoints<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Blake2_128Concat, T::VtxDistIdentifier>,
			NMapKey<Blake2_128Concat, EraIndex>,
			NMapKey<Blake2_128Concat, T::AccountId>,
		),
		(BalanceOf<T>, BalanceOf<T>, BalanceOf<T>), /* penalty effective balance, penalty work
		                                             * points, rates */
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn total_nw_reward)]
	pub(super) type TotalNetworkReward<T: Config> =
		StorageMap<_, Twox64Concat, T::VtxDistIdentifier, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn total_bs_reward)]
	pub(super) type TotalBootstrapReward<T: Config> =
		StorageMap<_, Twox64Concat, T::VtxDistIdentifier, BalanceOf<T>, ValueQuery>;

	/// Stores total vortex amount for each distribution
	#[pallet::storage]
	pub(super) type TotalVortex<T: Config> =
		StorageMap<_, Twox64Concat, T::VtxDistIdentifier, BalanceOf<T>, ValueQuery>;

	/// Storing total effective balance for each era
	#[pallet::storage]
	pub(super) type TotalEffectiveBalanceEra<T: Config> =
		StorageMap<_, Twox64Concat, T::VtxDistIdentifier, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	pub(super) type TotalWorkPointsEra<T: Config> =
		StorageMap<_, Twox64Concat, T::VtxDistIdentifier, BalanceOf<T>, ValueQuery>;

	/// Generate code for storing account total effective balance and total work points
	#[pallet::storage]
	pub(super) type AccountTotalEffectiveBalance<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::VtxDistIdentifier,
		Twox64Concat,
		T::AccountId,
		BalanceOf<T>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub(super) type AccountTotalWorkPoints<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::VtxDistIdentifier,
		Twox64Concat,
		T::AccountId,
		BalanceOf<T>,
		ValueQuery,
	>;

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

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Admin Account changed
		AdminAccountChanged { old_key: Option<T::AccountId>, new_key: T::AccountId },

		/// Rewards registered
		RewardRegistered {
			id: T::VtxDistIdentifier,
			rewards: BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxRewards>,
		},

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

		/// Set distribution eras
		SetVtxDistEras { id: T::VtxDistIdentifier, start_era: EraIndex, end_era: EraIndex },

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

		/// Trigger distribution calculation
		TriggerVtxDistribution { id: T::VtxDistIdentifier },

		/// Set Vtx total supply
		SetVtxTotalSupply {
			id: T::VtxDistIdentifier,
			total_supply: BalanceOf<T>,
		},
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		// incentive calculation
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

		/// Invalid end block
		InvalidEndBlock,

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

		/// balances and points vector length not match
		MismatchedBalancesAndPointsLength,

		/// balances and rates vector length not match
		MismatchedBalancesAndRatesLength,

		/// account id list not match
		MismatchedAccountIdLists,

		/// out of max reward vecotor bound
		ExceededMaxRewards,

		/// wrong era
		WrongEra,

		/// asset (price set) is not in assets list
		AssetNotInList,

		/// vortex price is zero
		VortexPriceIsZero,

		/// root price is zero
		RootPriceIsZero,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_admin())]
		pub fn set_admin(origin: OriginFor<T>, new: AccountIdLookupOf<T>) -> DispatchResult {
			ensure_root(origin)?;

			let new = T::Lookup::lookup(new)?;
			AdminAccount::<T>::put(&new);
			Self::deposit_event(Event::AdminAccountChanged {
				old_key: AdminAccount::<T>::get(),
				new_key: new,
			});
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

		/// Start distributing vortex
		///
		/// `id` - The distribution id
		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::start_vtx_dist())]
		#[transactional]
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
		#[pallet::call_index(4)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::pay_unsigned().saturating_mul(T::PayoutBatchSize::get().into()))]
		#[transactional]
		pub fn pay_unsigned(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			_current_block: BlockNumberFor<T>,
		) -> DispatchResult {
			ensure_none(origin)?;
			if let VtxDistStatus::Paying = VtxDistStatuses::<T>::get(id) {
				let vault_account = Self::get_vtx_held_account();
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
						&vault_account,
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
					VtxDistOrderbook::<T>::drain_prefix(id);
					Self::deposit_event(Event::VtxDistDone { id });
				}
				VtxDistPayoutPivot::<T>::insert(id, current_last_raw_key);
			}

			let current_block = <frame_system::Pallet<T>>::block_number();
			let next_unsigned_at = current_block + T::UnsignedInterval::get();
			<NextUnsignedAt<T>>::put(next_unsigned_at);
			Ok(())
		}

		/// Set distribution eras
		///
		/// `id` - The distribution id
		/// `start_era` - Start era
		/// `end_era` - End era
		#[pallet::call_index(5)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_vtx_dist_eras())]
		#[transactional]
		pub fn set_vtx_dist_eras(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			start_era: EraIndex,
			end_era: EraIndex,
		) -> DispatchResult {
			Self::ensure_root_or_admin(origin)?;
			ensure!(start_era <= end_era, Error::<T>::InvalidEndBlock);
			VtxDistEras::<T>::insert(id, (start_era, end_era));

			Self::deposit_event(Event::SetVtxDistEras { id, start_era, end_era });
			Ok(())
		}

		/// Set asset prices
		///
		/// `asset_prices` - List of asset prices
		/// `id` - The distribution id
		#[pallet::call_index(6)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_asset_prices(asset_prices.len() as u32))]
		#[transactional]
		pub fn set_asset_prices(
			origin: OriginFor<T>,
			asset_prices: BoundedVec<(AssetId, BalanceOf<T>), T::MaxAssetPrices>,
			id: T::VtxDistIdentifier,
		) -> DispatchResultWithPostInfo {
			Self::ensure_root_or_admin(origin)?;
			Self::do_asset_price_setter(asset_prices, id)
		}

		/// Register distribution rewards
		///
		/// `id` - The distribution id
		/// `rewards` - Rewards list
		#[pallet::call_index(7)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::register_rewards())]
		pub fn register_rewards(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			rewards: BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxRewards>,
		) -> DispatchResult {
			Self::ensure_root_or_admin(origin)?;

			let s = VtxDistStatuses::<T>::get(id);

			match s {
				VtxDistStatus::Enabled => {
					let mut total_rewards: BalanceOf<T> = Zero::zero();
					for (who, amount) in rewards.iter() {
						total_rewards += *amount;
						VtxDistOrderbook::<T>::mutate(id, who.clone(), |entry| {
							*entry = (*amount, false);
						});
					}
					TotalVortex::<T>::mutate(id, |total_vortex| {
						*total_vortex = total_vortex.saturating_add(total_rewards);
					});
					Self::deposit_event(Event::RewardRegistered { id, rewards });
					Ok(())
				},
				_ => Err(Error::<T>::VtxDistDisabled)?,
			}
		}
		
		/// Trigger distribution
		///
		/// `id` - The distribution id
		#[pallet::call_index(8)]
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

			Self::do_collate_reward_tokens(id)?;
			Self::do_reward_calculation(id)?;

			VtxDistStatuses::<T>::mutate(id, |status| {
				*status = VtxDistStatus::Triggered;
			});
			Self::deposit_event(Event::TriggerVtxDistribution { id });

			Ok(().into())
		}

		/// Redeem tokens from vault
		///
		/// `id` - The distribution id
		/// `vortex_token_amount` - Amount of vortex to redeem
		#[pallet::call_index(9)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::redeem_tokens_from_vault())]
		#[transactional]
		pub fn redeem_tokens_from_vault(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			vortex_token_amount: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let vault_account = Self::get_vtx_vault_account();
			let total_vortex = T::MultiCurrency::total_issuance(T::VtxAssetId::get());
			let vortex_balance = vortex_token_amount;
			ensure!(total_vortex > Zero::zero(), Error::<T>::NoVtxAssetMinted);
			ensure!(
				vortex_balance > Zero::zero()
					&& vortex_balance <= T::MultiCurrency::balance(T::VtxAssetId::get(), &who),
				Error::<T>::InvalidAmount
			);
			ensure!(VtxDistStatuses::<T>::get(id) == VtxDistStatus::Done, Error::<T>::CannotRedeem);

			/*for asset_id in AssetsList::<T>::get(id).into_iter() {
				// First, we calculate the ratio between the asset balance and the total vortex
				// issue. then multiply it with the vortex token amount the user wants to reddem to
				// get the resulting asset token amount.
				let asset_balance = T::MultiCurrency::balance(asset_id, &vault_account);
				let redeem_amount = vortex_balance.saturating_mul(asset_balance) / total_vortex;

				Self::safe_transfer(asset_id, &vault_account, &who, redeem_amount, false)?;
			}*/

			// Burn the vortex token
			T::MultiCurrency::burn_from(
				T::VtxAssetId::get(),
				&who,
				vortex_token_amount,
				Precision::Exact,
				Fortitude::Polite,
			)?;
			Ok(())
		}

		/// Set fee pot assets balances
		///
		/// `id` - The distribution id
		/// `assets_balances` - List of asset balances
		#[pallet::call_index(10)]
		#[pallet::weight(<T as Config>::WeightInfo::set_assets_list(assets_balances.len() as u32))]
		// #[transactional]
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
		#[pallet::call_index(11)]
		#[pallet::weight(<T as Config>::WeightInfo::set_assets_list(assets_balances.len() as u32))]
		// #[transactional]
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
		#[pallet::call_index(12)]
		#[pallet::weight(<T as Config>::WeightInfo::set_assets_list(0 as u32))]
		// #[transactional]
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
		#[pallet::call_index(13)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::register_rewards())]
		pub fn register_reward_points(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			reward_points: BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxRewards>,
		) -> DispatchResult {
			Self::ensure_root_or_admin(origin)?;
			let dst_status = VtxDistStatuses::<T>::get(id);
			ensure!(dst_status == VtxDistStatus::Enabled, Error::<T>::VtxDistDisabled);
			for (account, r_points) in reward_points {
				RewardPoints::<T>::insert(id, account, r_points);
			}

			Ok(())
		}

		/// Register work point distribution
		///
		/// `id` - The distribution id
		/// `work_points` - work point list
		#[pallet::call_index(14)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::register_rewards())]
		pub fn register_work_points(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			work_points: BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxRewards>,
		) -> DispatchResult {
			Self::ensure_root_or_admin(origin)?;
			let dst_status = VtxDistStatuses::<T>::get(id);
			ensure!(dst_status == VtxDistStatus::Enabled, Error::<T>::VtxDistDisabled);
			for (account, w_points) in work_points {
				WorkPoints::<T>::insert(id, account, w_points);
			}

			Ok(())
		}
		
		/// Register effective balances and work points
		/// length of vecotrs should align and with same set of accountid
		#[pallet::call_index(15)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::register_eff_bal_n_wk_pts())]
		#[transactional]
		pub fn register_eff_bal_n_wk_pts(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			era: EraIndex,
			balances: BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxRewards>,
			points: BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxRewards>,
			rates: BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxRewards>,
		) -> DispatchResult {
			Self::ensure_root_or_admin(origin)?;

			let s = VtxDistStatuses::<T>::get(id);

			ensure!(s == VtxDistStatus::Enabled, Error::<T>::VtxDistDisabled);

			//verify balances, points, and rates have the same length
			ensure!(balances.len() == points.len(), Error::<T>::MismatchedBalancesAndPointsLength);
			ensure!(balances.len() == rates.len(), Error::<T>::MismatchedBalancesAndRatesLength);

			// Iterate through balances, points, and rates to ensure they have the same AccountId
			for (((balance_account, _), (point_account, _)), (rate_account, _)) in
			balances.iter().zip(points.iter()).zip(rates.iter())
			{
				ensure!(
					balance_account == point_account && point_account == rate_account,
					Error::<T>::MismatchedAccountIdLists
				);
			}

			//record in storage
			for ((balance, point), rate) in
			balances.into_iter().zip(points.into_iter()).zip(rates.into_iter())
			{
				let penalty =
					EffectiveBalancesWorkPoints::<T>::contains_key((id, era, balance.clone().0));
				if penalty {
					let (effective_balance, work_points, rates) =
						EffectiveBalancesWorkPoints::<T>::get((id, era, balance.clone().0));
					PenaltyEffectiveBalancesWorkPoints::<T>::insert(
						(id, era, balance.clone().0),
						(effective_balance, work_points, rates),
					);
				}

				EffectiveBalancesWorkPoints::<T>::insert(
					(id, era, balance.clone().0),
					(balance.1, point.1, rate.1),
				);
			}

			let mut total_effective_balance_era: BalanceOf<T> =
				TotalEffectiveBalanceEra::<T>::get(id);
			let mut total_work_points_era: BalanceOf<T> = TotalWorkPointsEra::<T>::get(id);

			for (account_id, (effective_balance, work_points, rates)) in
			EffectiveBalancesWorkPoints::<T>::iter_prefix((id, era))
			{
				let mut account_total_effective_balance: BalanceOf<T> =
					AccountTotalEffectiveBalance::<T>::get(id, account_id.clone());
				let mut account_total_work_points: BalanceOf<T> =
					AccountTotalWorkPoints::<T>::get(id, account_id.clone());

				let penalty = PenaltyEffectiveBalancesWorkPoints::<T>::contains_key((
					id,
					era,
					account_id.clone(),
				));
				if penalty {
					let (penalty_effective_balance, penalty_work_points, penalty_rates) =
						PenaltyEffectiveBalancesWorkPoints::<T>::get((id, era, account_id.clone()));
					//reverse total balance and points for the era
					total_effective_balance_era = total_effective_balance_era
						.saturating_sub(penalty_effective_balance.saturating_mul(penalty_rates));
					total_work_points_era =
						total_work_points_era.saturating_sub(penalty_work_points);
					//reverse each account id's balance and points for the era
					account_total_effective_balance = account_total_effective_balance
						.saturating_sub(penalty_effective_balance.saturating_mul(penalty_rates));
					account_total_work_points =
						account_total_work_points.saturating_sub(penalty_work_points);
				}

				//accumulate total balance and points for the era
				total_effective_balance_era = total_effective_balance_era
					.saturating_add(effective_balance.saturating_mul(rates));
				total_work_points_era = total_work_points_era.saturating_add(work_points);
				//accumulate each account id's balance and points for the era
				account_total_effective_balance = account_total_effective_balance
					.saturating_add(effective_balance.saturating_mul(rates));
				account_total_work_points = account_total_work_points.saturating_add(work_points);
				AccountTotalEffectiveBalance::<T>::insert(
					id,
					account_id.clone(),
					account_total_effective_balance,
				);
				AccountTotalWorkPoints::<T>::insert(
					id,
					account_id.clone(),
					account_total_work_points,
				);
			}

			TotalEffectiveBalanceEra::<T>::insert(id, total_effective_balance_era);
			TotalWorkPointsEra::<T>::insert(id, total_work_points_era);

			Ok(())
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
			let vault_account = Self::get_vtx_held_account();
			let total_vortex = TotalVortex::<T>::get(id);
			T::MultiCurrency::mint_into(T::VtxAssetId::get(), &vault_account, total_vortex)?;

			TotalVortex::<T>::remove(id);// spk - why remove?
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
			Ok(().into())
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
			Ok(().into())
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
				ensure!(Self::check_asset_exist_in_fee_pot_asset_list(id, asset_id), Error::<T>::AssetNotInList);
				AssetPrices::<T>::insert(id, asset_id, price);
			}

			Self::deposit_event(Event::SetAssetPrices { id, asset_prices });
			Ok(().into())
		}

		// do collate rewards(assets and root) tokens in to vault account
		fn do_collate_reward_tokens(id: T::VtxDistIdentifier) -> DispatchResultWithPostInfo {
			let root_price = AssetPrices::<T>::get(id, T::NativeAssetId::get());
			ensure!(root_price > Zero::zero(), Error::<T>::RootPriceIsZero);

			let vault_account = Self::get_vtx_vault_account();
			let root_vault_account = Self::get_root_vault_account();
			let fee_vault_account = Self::get_fee_vault_account();

			// move gas & network fee to a vault here
			// move all asset in fee_vault to get_vault_account based on asset list in AssetsList
			let mut fee_vault_asset_value: BalanceOf<T> = 0u64.into();
			let mut vault_asset_value: BalanceOf<T> = 0u64.into();
			/*for asset_id in AssetsList::<T>::get(id).into_iter() {
				let asset_price = AssetPrices::<T>::get(id, asset_id);
				let asset_balance = T::MultiCurrency::balance(asset_id, &fee_vault_account);
				fee_vault_asset_value += asset_balance.saturating_mul(asset_price);
				Self::safe_transfer(
					asset_id,
					&fee_vault_account,
					&vault_account,
					asset_balance,
					false,
				)?;
				let asset_balance_vault = T::MultiCurrency::balance(asset_id, &vault_account);
				vault_asset_value += asset_balance_vault.saturating_mul(asset_price);
			}*/

			// move bootstrap incentive here
			// move root token from fee_vault to vault_account
			let fee_vault_root_token_balance =
				T::MultiCurrency::balance(T::NativeAssetId::get(), &fee_vault_account);
			let fee_vault_root_value: BalanceOf<T> = fee_vault_root_token_balance * root_price;

			if fee_vault_root_token_balance > Zero::zero() {
				Self::safe_transfer(
					T::NativeAssetId::get(),
					&fee_vault_account,
					&vault_account,
					fee_vault_root_token_balance,
					false,
				)?;
			}

			// move root token from root_vault to get_vault_account
			let root_vault_root_token_balance =
				T::MultiCurrency::balance(T::NativeAssetId::get(), &root_vault_account);
			let root_vault_root_value: BalanceOf<T> = root_vault_root_token_balance * root_price;

			Self::safe_transfer(
				T::NativeAssetId::get(),
				&root_vault_account,
				&vault_account,
				root_vault_root_token_balance,
				false,
			)?;
			// let mut vault_root_value: BalanceOf<T> = (fee_vault_root_token_balance +
			// root_vault_root_token_balance) * root_price;
			let vault_root_value: BalanceOf<T> =
				T::MultiCurrency::balance(T::NativeAssetId::get(), &vault_account) * root_price;

			//calculate vortex price
			let vault_total_assets_root_value = vault_asset_value + vault_root_value;
			let existing_vortex_supply = T::MultiCurrency::total_issuance(T::VtxAssetId::get());
			let vortex_price = if existing_vortex_supply == Zero::zero() {
				1u64.into() // should be still 1 not matter decimal points (6 decimal)
			} else {
				vault_total_assets_root_value / existing_vortex_supply
			};

			ensure!(vortex_price > Zero::zero(), Error::<T>::VortexPriceIsZero);
			//calculate total rewards
			let total_vortex_network_reward =
				(fee_vault_asset_value + fee_vault_root_value) / vortex_price;
			let total_vortex_bootstrap = root_vault_root_value / vortex_price;

			let total_vortex = total_vortex_network_reward + total_vortex_bootstrap;
			TotalVortex::<T>::insert(id, total_vortex);
			TotalNetworkReward::<T>::insert(id, total_vortex_network_reward);
			TotalBootstrapReward::<T>::insert(id, total_vortex_bootstrap);

			Ok(().into())
		}

		fn do_reward_calculation(id: T::VtxDistIdentifier) -> DispatchResultWithPostInfo {
			//get era info for this reward cycle
			let (era_start, era_end) = VtxDistEras::<T>::get(id);
			//calc total reward points and total effective balance
			let total_effective_balance = TotalEffectiveBalanceEra::<T>::get(id);
			let total_work_points = TotalWorkPointsEra::<T>::get(id);
			let mut account_ids: BoundedVec<T::AccountId, T::MaxRewards> = BoundedVec::default();
			for era in era_start..=era_end {
				for (account_id, _) in EffectiveBalancesWorkPoints::<T>::iter_prefix((id, era)) {
					if !account_ids.contains(&account_id) {
						account_ids
							.try_push(account_id)
							.map_err(|_| Error::<T>::ExceededMaxRewards)?;
					}
				}
			}

			//calc each account id's reward portion
			for account_id in &account_ids {
				let account_total_effective_balance: BalanceOf<T> =
					AccountTotalEffectiveBalance::<T>::get(id, account_id.clone());
				let account_total_work_points: BalanceOf<T> =
					AccountTotalWorkPoints::<T>::get(id, account_id.clone());

				let balance_portion = Perbill::from_rational(
					account_total_effective_balance,
					total_effective_balance,
				);
				let work_points_portion =
					Perbill::from_rational(account_total_work_points, total_work_points);

				let total_vortex_network_reward = TotalNetworkReward::<T>::get(id);
				let ind_vortex_balance_network_reward =
					balance_portion * Perbill::from_percent(30) * total_vortex_network_reward;
				let ind_vortex_wk_points_network_reward =
					work_points_portion * Perbill::from_percent(70) * total_vortex_network_reward;

				let total_vortex_bootstrap_reward = TotalBootstrapReward::<T>::get(id);
				// let reward_work_points_portion = work_points_portion * Perbill::from_percent(70)
				// * total_bootstrap_reward;

				let ind_vortex_network_reward =
					ind_vortex_balance_network_reward + ind_vortex_wk_points_network_reward;

				let ind_vortex_bootstrap_reward = balance_portion * total_vortex_bootstrap_reward;

				let final_reward = ind_vortex_network_reward + ind_vortex_bootstrap_reward;
				VtxDistOrderbook::<T>::mutate(id, account_id, |entry| {
					*entry = (entry.0.saturating_add(final_reward), entry.1);
				});
			}
			Ok(().into())
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
				if (id == asset_id) {
					return true;
				}
			}
			
			false
		}
	}
}
