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
		tokens::fungibles::{self, Inspect, Mutate, Transfer},
		Get,
	},
	PalletId,
};
use frame_system::{
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::*,
};
use pallet_staking::{BalanceOf, RewardPoint};
use scale_info::TypeInfo;
use seed_pallet_common::CreateExt;
use seed_primitives::{AssetId, OffchainErr};
use sp_runtime::{
	traits::{AccountIdConversion, CheckedAdd, One, Saturating, Zero},
	RuntimeDebug,
};
use sp_staking::EraIndex;
use sp_std::{convert::TryInto, prelude::*};

pub const VTX_DIST_UNSIGNED_PRIORITY: TransactionPriority = TransactionPriority::max_value() / 2;

#[derive(
	Clone, Copy, Encode, Decode, RuntimeDebug, PartialEq, PartialOrd, Eq, TypeInfo, MaxEncodedLen,
)]
pub enum VtxDistStatus {
	NotEnabled,
	Enabled,
	Triggered,
	Paying,
	Done,
}

impl Default for VtxDistStatus {
	fn default() -> Self {
		Self::NotEnabled
	}
}

#[frame_support::pallet]
pub mod pallet {

	use frame_support::transactional;
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
			+ fungibles::Transfer<Self::AccountId, Balance = BalanceOf<Self>>
			+ fungibles::Inspect<Self::AccountId, AssetId = AssetId>
			+ fungibles::InspectMetadata<Self::AccountId>
			+ fungibles::Mutate<Self::AccountId>;

		/// The native token asset Id (managed by pallet-balances)
		#[pallet::constant]
		type NativeAssetId: Get<AssetId>;

		/// Vortex token asset Id
		#[pallet::constant]
		type VtxAssetId: Get<AssetId>;

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
		type UnsignedInterval: Get<Self::BlockNumber>;

		/// Payout batch size
		#[pallet::constant]
		type PayoutBatchSize: Get<u32>;

		/// Max asset prices items
		type MaxAssetPrices: Get<u32>;

		/// Max rewards items
		type MaxRewards: Get<u32>;

		/// Max pivot string length
		type MaxStringLength: Get<u32>;

		/// Vortex distribution admin origin
		type VtxDistAdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

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
		type HistoryDepth: Get<u32>; //TODO: need to set to same value as pallet_staking
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

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

	/// Stores stake and roles reward points for each vortex distribution
	#[pallet::storage]
	pub(super) type StakeRewardsPoints<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::VtxDistIdentifier,
		Blake2_128Concat,
		T::AccountId,
		(BalanceOf<T>, RewardPoint), //30% and 70% part
		ValueQuery,
		GetDefault,
		ConstU32<{ u32::MAX }>,
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

	/// Stores total vortex amount for each distribution
	#[pallet::storage]
	pub(super) type TotalVortex<T: Config> =
		StorageMap<_, Twox64Concat, T::VtxDistIdentifier, BalanceOf<T>, ValueQuery>;

	/// Stores next unsigned tx block number
	#[pallet::storage]
	pub(super) type NextUnsignedAt<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

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
		/// Rewards registered
		RewardRegistered {
			id: T::VtxDistIdentifier,
			rewards: BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxRewards>,
		},

		/// Distribution enabled
		VtxDistEnabled { id: T::VtxDistIdentifier },

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

		/// Set asset prices
		SetAssetPrices {
			id: T::VtxDistIdentifier,
			asset_prices: BoundedVec<(AssetId, BalanceOf<T>), T::MaxAssetPrices>,
		},

		/// Trigger distribution calculation
		TriggerVtxDistribution { id: T::VtxDistIdentifier },
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		// incentive calculation
		fn offchain_worker(now: T::BlockNumber) {
			if let Err(e) = Self::vtx_dist_offchain_worker(now) {
				log::info!(
				  target: "vtx-dist",
				  "error happened in offchain worker at {:?}: {:?}",
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
		/// No available Dist id
		VtxDistIdNotAvailable,

		/// Vortex distribution already enabled
		VtxDistAlreadyEnabled,

		/// Vortex distribution not enabled
		VtxDistNotEnabled,

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

		/// Vortex distribution already triggered
		AlreadyTriggered,

		/// Vortex distribution not triggered
		NotTriggered,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// List a vortex distribution
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_vtx_dist())]
		#[transactional]
		pub fn create_vtx_dist(origin: OriginFor<T>) -> DispatchResult {
			T::VtxDistAdminOrigin::ensure_origin(origin)?;

			let id = NextVortexId::<T>::get();
			let next_pool_id =
				id.checked_add(&One::one()).ok_or(Error::<T>::VtxDistIdNotAvailable)?;

			NextVortexId::<T>::mutate(|next_id| {
				*next_id = next_pool_id;
			});
			VtxDistStatuses::<T>::insert(id, VtxDistStatus::Enabled);
			Self::deposit_event(Event::VtxDistEnabled { id });
			Ok(())
		}

		/// Disable a distribution
		///
		/// `id` - The distribution id
		#[pallet::weight(<T as pallet::Config>::WeightInfo::disable_vtx_dist())]
		#[transactional]
		pub fn disable_vtx_dist(origin: OriginFor<T>, id: T::VtxDistIdentifier) -> DispatchResult {
			T::VtxDistAdminOrigin::ensure_origin(origin)?;
			ensure!(
				VtxDistStatuses::<T>::get(id.clone()) != VtxDistStatus::NotEnabled,
				Error::<T>::VtxDistNotEnabled
			);
			Self::do_disable_vtx_dist(id);
			Self::deposit_event(Event::VtxDistDisabled { id });
			Ok(())
		}

		/// Start distributing vortex
		///
		/// `id` - The distribution id
		#[pallet::weight(<T as pallet::Config>::WeightInfo::start_vtx_dist())]
		#[transactional]
		pub fn start_vtx_dist(origin: OriginFor<T>, id: T::VtxDistIdentifier) -> DispatchResult {
			T::VtxDistAdminOrigin::ensure_origin(origin)?;

			ensure!(
				VtxDistStatuses::<T>::get(id.clone()) == VtxDistStatus::Triggered,
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
		#[pallet::weight(<T as pallet::Config>::WeightInfo::pay_unsigned() * 99)]
		#[transactional]
		pub fn pay_unsigned(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			_current_block: T::BlockNumber,
		) -> DispatchResult {
			ensure_none(origin)?;
			if let VtxDistStatus::Paying = VtxDistStatuses::<T>::get(id) {
				let vault_account = Self::get_vtx_vault_account();
				let start_key = VtxDistPayoutPivot::<T>::get(id);
				let payout_pivot: Vec<u8> =
					start_key.clone().try_into().map_err(|_| Error::<T>::PivotStringTooLong)?;

				let mut map_iterator = match VtxDistPayoutPivot::<T>::contains_key(id) {
					true => <VtxDistOrderbook<T>>::iter_prefix_from(id, payout_pivot),
					false => <VtxDistOrderbook<T>>::iter_prefix(id),
				};

				let mut count = 0u32;
				while let Some((who, entry)) = map_iterator.next() {
					// if the user is already paid out, skip
					if entry.1 {
						continue
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
						break
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
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_vtx_dist_eras())]
		#[transactional]
		pub fn set_vtx_dist_eras(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			start_era: EraIndex,
			end_era: EraIndex,
		) -> DispatchResult {
			T::VtxDistAdminOrigin::ensure_origin(origin)?;
			ensure!(start_era <= end_era, Error::<T>::InvalidEndBlock);
			VtxDistEras::<T>::insert(id, (start_era, end_era));

			Self::deposit_event(Event::SetVtxDistEras { id, start_era, end_era });
			Ok(())
		}

		/// Set asset prices
		///
		/// `asset_prices` - List of asset prices
		/// `id` - The distribution id
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_asset_prices(asset_prices.len() as u32))]
		#[transactional]
		pub fn set_asset_prices(
			origin: OriginFor<T>,
			asset_prices: BoundedVec<(AssetId, BalanceOf<T>), T::MaxAssetPrices>,
			id: T::VtxDistIdentifier,
		) -> DispatchResultWithPostInfo {
			T::VtxDistAdminOrigin::ensure_origin(origin)?;
			Self::do_asset_price_setter(asset_prices, id)
		}

		/// Register distribution rewards
		///
		/// `id` - The distribution id
		/// `rewards` - Rewards list
		#[pallet::weight(<T as pallet::Config>::WeightInfo::register_rewards())]
		pub fn register_rewards(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
			rewards: BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxRewards>,
		) -> DispatchResult {
			T::VtxDistAdminOrigin::ensure_origin(origin)?;
			let s = VtxDistStatuses::<T>::get(id);

			match s {
				VtxDistStatus::Enabled => {
					let mut total_rewards: BalanceOf<T> = Zero::zero();
					for (who, amount) in rewards.iter() {
						total_rewards += *amount;
						VtxDistOrderbook::<T>::mutate(id, who.clone(), |entry| {
							*entry = (entry.0.saturating_add(*amount), entry.1);
						});
					}
					TotalVortex::<T>::mutate(id, |total_vortex| {
						*total_vortex = total_vortex.saturating_add(total_rewards);
					});
					Self::deposit_event(Event::RewardRegistered { id, rewards });
					Ok(())
				},
				_ => Err(Error::<T>::VtxDistNotEnabled)?,
			}
		}

		/// Trigger distribution
		///
		/// `id` - The distribution id
		#[pallet::weight(<T as pallet::Config>::WeightInfo::trigger_vtx_distribution())]
		pub fn trigger_vtx_distribution(
			origin: OriginFor<T>,
			id: T::VtxDistIdentifier,
		) -> DispatchResultWithPostInfo {
			T::VtxDistAdminOrigin::ensure_origin(origin)?;

			ensure!(
				VtxDistStatuses::<T>::get(id.clone()) < VtxDistStatus::Triggered,
				Error::<T>::AlreadyTriggered
			);

			Self::do_vtx_distribution_trigger(id)
		}

		/// Redeem tokens from vault
		///
		/// `id` - The distribution id
		/// `vortex_token_amount` - Amount of vortex to redeem
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
				vortex_balance > Zero::zero() &&
					vortex_balance <= T::MultiCurrency::balance(T::VtxAssetId::get(), &who),
				Error::<T>::InvalidAmount
			);

			for (asset_id, _) in AssetPrices::<T>::iter_prefix(id) {
				// First we calculate the ratio between the asset balance and the total vortex
				// issue. then multiply it with the vortex token amount the user wants to reddem to
				// get the resulting asset token amount.
				let asset_balance = T::MultiCurrency::balance(asset_id, &vault_account);
				let redeem_amount = vortex_balance.saturating_mul(asset_balance) / total_vortex;

				Self::safe_transfer(asset_id, &vault_account, &who, redeem_amount, false)?;
			}

			// Add root token in the redeem token
			let root_token_id = T::NativeAssetId::get();
			let root_token_balance = T::MultiCurrency::balance(root_token_id, &vault_account);
			let redeem_amount = vortex_balance.saturating_mul(root_token_balance) / total_vortex;

			// Transfer native token from Vault to user
			Self::safe_transfer(root_token_id, &vault_account, &who, redeem_amount, false)?;
			// Burn the vortex token
			T::MultiCurrency::burn_from(T::VtxAssetId::get(), &who, vortex_token_amount)?;
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
						return InvalidTransaction::Future.into()
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
				*status = VtxDistStatus::NotEnabled;
			});
		}

		/// start a distribution
		fn do_start_vtx_dist(id: T::VtxDistIdentifier) -> DispatchResult {
			let vault_account = Self::get_vtx_vault_account();
			let total_vortex = TotalVortex::<T>::get(id);
			T::MultiCurrency::mint_into(T::VtxAssetId::get(), &vault_account, total_vortex)?;

			TotalVortex::<T>::remove(id);
			VtxDistStatuses::<T>::mutate(id, |status| {
				*status = VtxDistStatus::Paying;
			});
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
				AssetPrices::<T>::insert(id, asset_id, price);
			}

			Self::deposit_event(Event::SetAssetPrices { id, asset_prices });
			Ok(().into())
		}

		/// trigger a distribution
		fn do_vtx_distribution_trigger(id: T::VtxDistIdentifier) -> DispatchResultWithPostInfo {
			let vault_account = Self::get_vtx_vault_account();

			let root_vault_account = Self::get_root_vault_account();
			let fee_vault_account = Self::get_fee_vault_account();

			// move gas & network fee to a vault here
			// move all asset in fee_vault to get_vault_account based on asset list in AssetPrices
			for (asset_id, _) in AssetPrices::<T>::iter_prefix(id) {
				let asset_balance = T::MultiCurrency::balance(asset_id, &fee_vault_account);
				Self::safe_transfer(
					asset_id,
					&fee_vault_account,
					&vault_account,
					asset_balance,
					false,
				)?;
			}
			// move bootstrap incenive here
			// move root token from root_vault to get_vault_account
			let root_token_balance =
				T::MultiCurrency::balance(T::NativeAssetId::get(), &root_vault_account);
			Self::safe_transfer(
				T::NativeAssetId::get(),
				&root_vault_account,
				&vault_account,
				root_token_balance,
				false,
			)?;

			VtxDistStatuses::<T>::mutate(id, |status| {
				*status = VtxDistStatus::Triggered;
			});
			Self::deposit_event(Event::TriggerVtxDistribution { id });

			Ok(().into())
		}

		/// offchain worker for unsigned tx
		fn vtx_dist_offchain_worker(now: T::BlockNumber) -> Result<(), OffchainErr> {
			if !sp_io::offchain::is_validator() {
				return Err(OffchainErr::NotAValidator)
			}
			let next_unsigned_at = <NextUnsignedAt<T>>::get();
			if next_unsigned_at > now {
				return Err(OffchainErr::TooEarly)
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
			let transfer_result =
				T::MultiCurrency::transfer(asset_id, source, dest, amount, false)?;
			ensure!(transfer_result == amount, Error::<T>::InvalidAmount);
			Ok(())
		}
	}
}
