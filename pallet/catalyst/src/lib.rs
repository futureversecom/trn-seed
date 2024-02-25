#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

use frame_support::{dispatch::DispatchResult, log, pallet_prelude::*, PalletId};
use frame_system::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode, HasCompact};
use plug_primitives::{AssetId, Balance};
use scale_info::TypeInfo;
use sp_io::hashing::blake2_256;
use sp_runtime::{
	traits::{AccountIdConversion, CheckedDiv, One, Saturating, Zero},
	FixedPointNumber, FixedU128, RuntimeDebug,
};

use frame_system::offchain::{SendTransactionTypes, SubmitTransaction};
pub use plug_utils::NATIVE_TOKEN_ASSET_ID as NATIVE_TOKEN_OTTO_ASSET_ID;
use sp_core::U256;
use sp_std::prelude::*;

pub const IEO_UNSIGNED_PRIORITY: TransactionPriority = TransactionPriority::max_value() / 2;
const PLUG_OTTOT_RATE: u128 = 25;

#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, PartialEq, Eq, TypeInfo)]
pub enum IEOStatus<BlockNumber, Balance> {
	NotEnabled,
	Enabled(BlockNumber),
	Paying(Balance),
	Done,
}

impl<BlockNumber, Balance> Default for IEOStatus<BlockNumber, Balance> {
	fn default() -> Self {
		Self::NotEnabled
	}
}

#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, PartialEq, Eq, TypeInfo)]
pub enum CatBinarySearchResult<T, Balance> {
	ExactIntegerAtCatNumber(T),
	WithFractionAtCatNumber((T, Balance)),
	Error,
}

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq, TypeInfo)]
pub struct CATParameters<Balance, BlockNumber> {
	offered_asset: AssetId,
	required_asset: AssetId,
	offered_amount: Balance,
	start_block: BlockNumber,
	end_block: BlockNumber,
	next_price: FixedU128,
	next_time_diminishing: FixedU128,
	next_cat_number: u128,
	price: Vec<(u128, u128, FixedU128, FixedU128)>, /* token starting number, cat token ending
	                                                 * number,
	                                                 * price_incremental, time_diminishing) */
}

#[derive(Clone, Encode, Decode, RuntimeDebug, PartialEq, Eq, TypeInfo)]
pub enum CATStatus<Balance, BlockNumber> {
	NotEnabled,
	Enabled(CATParameters<Balance, BlockNumber>),
	Done,
}

impl<Balance, BlockNumber> Default for CATStatus<Balance, BlockNumber> {
	fn default() -> Self {
		Self::NotEnabled
	}
}

#[frame_support::pallet]
pub mod pallet {
	use core::cmp::Ordering;

	use frame_support::transactional;
	use weights::WeightInfo;

	use super::*;

	#[pallet::config]
	pub trait Config:
		frame_system::Config
		+ plug_utils::Config
		+ pallet_catalyst_reward::Config
		+ SendTransactionTypes<Call<Self>>
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		#[pallet::constant]
		type CATPalletId: Get<PalletId>;
		#[pallet::constant]
		type PLUGAssetId: Get<AssetId>;
		#[pallet::constant]
		type CatalystAssetId: Get<AssetId>;
		#[pallet::constant]
		type CatalystVoucherAssetId: Get<AssetId>;
		#[pallet::constant]
		type UnsignedInterval: Get<BlockNumberFor<Self>>;
		#[pallet::constant]
		type PayoutBatchSize: Get<u32>;
		#[pallet::constant]
		type TimeDiminishingNo: Get<u128>;
		#[pallet::constant]
		type TimeDiminishingBase: Get<FixedU128>;
		#[pallet::constant]
		type TimeDiminishingFactor: Get<FixedU128>;
		type CATAdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		type CATIdentifier: Member + Parameter + Default + Copy + HasCompact + MaxEncodedLen;
		/// Verified participants
		type VerifiedUserOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn ieo_statuses)]
	pub type IEOStatuses<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::CATIdentifier,
		IEOStatus<BlockNumberFor<T>, Balance>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn ieo_orderbook)]
	pub(super) type IEOOrderbook<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::CATIdentifier,
		Blake2_128Concat,
		T::AccountId,
		(Balance, bool),
		ValueQuery,
		GetDefault,
		ConstU32<{ u32::MAX }>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn next_unsigned_at)]
	pub(super) type NextUnsignedAt<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn ieo_payout_pivot)]
	pub(super) type IEOPayoutPivot<T: Config> =
		StorageMap<_, Twox64Concat, T::CATIdentifier, Vec<u8>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn ieo_total_gathered)]
	pub(super) type IEOTotalGathered<T: Config> =
		StorageMap<_, Twox64Concat, T::CATIdentifier, Balance, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn cat_statuses)]
	pub type CATStatuses<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::CATIdentifier,
		CATStatus<Balance, BlockNumberFor<T>>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		OfferReceived(T::CATIdentifier, T::AccountId, Balance, Balance),
		CATEnabled(
			T::CATIdentifier,
			BlockNumberFor<T>,
			BlockNumberFor<T>,
			AssetId,
			Balance,
			AssetId,
		),
		CATDisabled(T::CATIdentifier),
		CATPriceSet(T::CATIdentifier, FixedU128, Vec<(u128, u128, FixedU128, FixedU128)>),
		PlugOfferReceived(T::AccountId, Balance),
		PlugCataIEOEnabled(BlockNumberFor<T>, BlockNumberFor<T>, AssetId, AssetId),
		PlugCataIEODone(),
		PlugCataIEOPaidOut(T::AccountId, Balance),
		PlugCataIEODisabled(T::CATIdentifier),
		PlugCataIEOOrderBookClear(T::CATIdentifier),
		PlugCataInconsistentDeposit(),
		PlugCataVoucherRedeemed(T::AccountId, Balance),
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> frame_support::weights::Weight {
			let mut consumed_weight: Weight = Weight::from_parts(0, 0);
			// for (id, status) in IEOStatuses::<T>::iter() {
			// 	match status {
			// 		IEOStatus::<_, _>::Enabled(end_block) => {
			// 			match end_block {
			// 				x if x == now => {
			// 					consumed_weight += <T as Config>::WeightInfo::participate();
			// 					let vault_account = Self::get_vault_account(id).unwrap();
			// 					let total_gathered = Self::ieo_total_gathered(id);
			// 					let vault_balance = plug_utils::Pallet::<T>::asset_balance(
			// 						T::PLUGAssetId::get(),
			// 						&vault_account,
			// 					);
			// 					// we really should not see this coming. Serious fraud
			// 					log::warn!("total gathered {:?}", total_gathered);
			// 					log::warn!("vault_balance {:?}", vault_balance);
			// 					if (vault_balance < total_gathered) || total_gathered.is_zero() {
			// 						Self::do_disable_ieo(id);
			// 						Self::deposit_event(Event::PlugCataIEODisabled(id));
			// 						Self::deposit_event(Event::PlugCataInconsistentDeposit());
			// 					}
			//
			// 					if let CATStatus::Enabled(parameters) = Self::cat_statuses(id) {
			// 						let CATParameters {
			// 							offered_asset,
			// 							required_asset,
			// 							offered_amount,
			// 							start_block,
			// 							end_block,
			// 							next_cat_number: old_next_cat_number,
			// 							next_price,
			// 							next_time_diminishing,
			// 							price,
			// 							..
			// 						} = parameters;
			//
			// 						let (
			// 							available_cata_voucher,
			// 							next_price,
			// 							next_time_diminishing,
			// 							next_cat_number,
			// 						) = Self::do_finalize_ieo(
			// 							total_gathered,
			// 							next_price,
			// 							old_next_cat_number,
			// 							next_time_diminishing,
			// 							&price,
			// 						);
			//
			// 						IEOStatuses::<T>::mutate(id, |status| {
			// 							*status = IEOStatus::Paying(available_cata_voucher);
			// 						});
			//
			// 						let current_cat_price =
			// 							Self::get_cat_price_at(next_cat_number - 1, &price);
			//
			// 						plug_utils::Pallet::<T>::token_mint_into(
			// 							T::CatalystAssetId::get(),
			// 							&vault_account,
			// 							next_cat_number - old_next_cat_number,
			// 						)
			// 						.unwrap();
			//
			// 						let new_parameter = CATParameters {
			// 							offered_asset,
			// 							required_asset,
			// 							offered_amount,
			// 							start_block,
			// 							end_block,
			// 							next_price,
			// 							next_time_diminishing,
			// 							next_cat_number,
			// 							price,
			// 						};
			// 						CATStatuses::<T>::mutate(id, |status| {
			// 							*status = CATStatus::Enabled(new_parameter);
			// 						});
			//
			// 						Self::update_reward_storage(id, current_cat_price, next_time_diminishing).unwrap();
			// 					}
			// 				},
			// 				x if x < now => {
			// 					// just in case, this shouldn't happen
			// 					Self::do_disable_ieo(id);
			// 					Self::deposit_event(Event::PlugCataIEODisabled(id));
			// 				},
			// 				_ => {},
			// 			}
			// 		},
			// 		_ => {},
			// 	}
			// }
			consumed_weight
		}

		// fn offchain_worker(now: BlockNumberFor<T>) {
		// 	if let Err(e) = Self::ieo_offchain_worker(now) {
		// 		log::info!(
		// 		  target: "ieo offchain worker",
		// 		  "error happened in offchain worker at {:?}: {:?}",
		// 		  now,
		// 		  e,
		// 		);
		// 	} else {
		// 		log::debug!(
		// 		  target: "ieo offchain worker",
		// 		  "offchain worker start at block: {:?} already done!",
		// 		  now,
		// 		);
		// 	}
		// }
	}

	#[pallet::error]
	pub enum Error<T> {
		CatAlreadyEnabled,
		CatIsNotEnabled,
		CatIsNotStarted,
		InvalidStartblock,
		InvalidEndblock,
		InvalidOfferedAmount,
		InvalidPrice,
		InvalidPurchaseAmount,
		InvalidAssetId,
		UnableToCalculateAssetRate,
		CatIdInUse,
		CatIdNotSet,
		IeoAlreadyEnabled,
		IeoAlreadyDone,
		IeoIsNotEnabled,
		IeoIsNotProvisioning,
		IeoIsNotDone,
		IeoIsNotDisabled,
		PlugIsDisabledWhenIeoEnabled,
		InvalidCatNumber,
		InvalidAmount,
		// offchain
		NotValidator,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::list_ieo().ref_time())]
		#[transactional]
		pub fn list_ieo(
			origin: OriginFor<T>,
			id: T::CATIdentifier,
			end_block: BlockNumberFor<T>,
		) -> DispatchResult {
			T::CATAdminOrigin::ensure_origin(origin)?;
			Self::check_cata_status(id)?;

			let ieo_status = Self::ieo_statuses(id);
			ensure!(
				ieo_status == IEOStatus::NotEnabled || ieo_status == IEOStatus::Done,
				Error::<T>::IeoAlreadyEnabled
			);
			let now = frame_system::Pallet::<T>::block_number();

			ensure!(end_block > now, Error::<T>::InvalidEndblock);

			IEOStatuses::<T>::insert(id, IEOStatus::Enabled(end_block));
			Self::deposit_event(Event::PlugCataIEOEnabled(
				now,
				end_block,
				T::PLUGAssetId::get(),
				T::CatalystVoucherAssetId::get(),
			));
			Ok(())
		}
		//
		// #[pallet::call_index(1)]
		// #[pallet::weight(<T as pallet::Config>::WeightInfo::disable_ieo().ref_time())]
		// #[transactional]
		// pub fn disable_ieo(origin: OriginFor<T>, id: T::CATIdentifier) -> DispatchResult {
		// 	T::CATAdminOrigin::ensure_origin(origin)?;
		// 	Self::do_disable_ieo(id);
		// 	Self::deposit_event(Event::PlugCataIEODisabled(id));
		// 	Ok(())
		// }
		//
		// #[pallet::call_index(2)]
		// #[pallet::weight(<T as pallet::Config>::WeightInfo::clear_orderbook().ref_time())]
		// #[transactional]
		// pub fn clear_orderbook(origin: OriginFor<T>, id: T::CATIdentifier) -> DispatchResult {
		// 	T::CATAdminOrigin::ensure_origin(origin)?;
		// 	ensure!(
		// 		matches!(Self::ieo_statuses(id), IEOStatus::NotEnabled),
		// 		Error::<T>::IeoIsNotDisabled
		// 	);
		//
		// 	let _ = IEOOrderbook::<T>::drain_prefix(id).collect::<Vec<_>>();
		// 	IEOTotalGathered::<T>::remove(id);
		// 	IEOPayoutPivot::<T>::remove(id);
		// 	NextUnsignedAt::<T>::kill();
		//
		// 	Self::deposit_event(Event::PlugCataIEOOrderBookClear(id));
		// 	Ok(())
		// }
		//
		// #[pallet::call_index(3)]
		// #[pallet::weight(<T as pallet::Config>::WeightInfo::deposit_plug().ref_time())]
		// #[transactional]
		// pub fn deposit_plug(
		// 	origin: OriginFor<T>,
		// 	id: T::CATIdentifier,
		// 	amount: Balance,
		// ) -> DispatchResult {
		// 	let who = <T as pallet::Config>::VerifiedUserOrigin::ensure_origin(origin)?;
		// 	Self::check_cata_status(id)?;
		//
		// 	if let IEOStatus::<_, _>::Enabled(end_block) = Self::ieo_statuses(id) {
		// 		let now = frame_system::Pallet::<T>::block_number();
		// 		ensure!(end_block > now, Error::<T>::IeoAlreadyDone);
		//
		// 		let vault_account = Self::get_vault_account(id).unwrap();
		// 		let offer_to_vault_account = plug_utils::Pallet::<T>::asset_token_transfer(
		// 			T::PLUGAssetId::get(),
		// 			&who,
		// 			&vault_account,
		// 			amount,
		// 			false,
		// 		)?;
		// 		IEOOrderbook::<T>::mutate(id.clone(), who.clone(), |entry| {
		// 			*entry = (entry.0.saturating_add(amount), entry.1);
		// 		});
		// 		IEOTotalGathered::<T>::mutate(id.clone(), |gathered| {
		// 			*gathered = gathered.saturating_add(offer_to_vault_account)
		// 		});
		// 		Self::deposit_event(Event::PlugOfferReceived(who, amount));
		// 		Ok(())
		// 	} else {
		// 		return Err(Error::<T>::IeoIsNotEnabled.into())
		// 	}
		// }
		//
		// #[pallet::call_index(4)]
		// #[pallet::weight(<T as pallet::Config>::WeightInfo::redeem_cata().ref_time())]
		// #[transactional]
		// pub fn redeem_cata(
		// 	origin: OriginFor<T>,
		// 	id: T::CATIdentifier,
		// 	cata_voucher_amount: Balance,
		// ) -> DispatchResult {
		// 	let who = <T as pallet::Config>::VerifiedUserOrigin::ensure_origin(origin)?;
		//
		// 	// check ieo status should be done
		// 	ensure!(
		// 		matches!(Self::ieo_statuses(id), IEOStatus::<_, _>::Done),
		// 		Error::<T>::IeoIsNotDone
		// 	);
		//
		// 	// check cata_voucher amount has no decimals.
		// 	let cata_voucher_decimals =
		// 		plug_utils::Pallet::<T>::asset_decimals(T::CatalystVoucherAssetId::get()) as u32;
		// 	ensure!(
		// 		cata_voucher_amount % ((10 as Balance).pow(cata_voucher_decimals)) == 0,
		// 		Error::<T>::InvalidAmount
		// 	);
		//
		// 	if let CATStatus::Enabled(_parameters) = Self::cat_statuses(id) {
		// 		let vault_account = Self::get_vault_account(id).unwrap();
		// 		let cat_amount = TryInto::<Balance>::try_into(
		// 			Self::rate_between_assets(
		// 				T::CatalystAssetId::get(),
		// 				T::CatalystVoucherAssetId::get(),
		// 				cata_voucher_amount,
		// 			)?
		// 			.into_inner() / FixedU128::DIV,
		// 		)
		// 		.unwrap_or(0);
		//
		// 		plug_utils::Pallet::<T>::token_burn_from(
		// 			T::CatalystVoucherAssetId::get(),
		// 			&who,
		// 			cata_voucher_amount,
		// 		)?;
		// 		plug_utils::Pallet::<T>::asset_token_transfer(
		// 			T::CatalystAssetId::get(),
		// 			&vault_account,
		// 			&who,
		// 			cat_amount,
		// 			false,
		// 		)?;
		// 	} else {
		// 		return Err(Error::<T>::CatIsNotEnabled.into())
		// 	}
		//
		// 	Self::deposit_event(Event::PlugCataVoucherRedeemed(who, cata_voucher_amount));
		//
		// 	Ok(())
		// }
		//
		// #[pallet::call_index(5)]
		// #[pallet::weight(<T as pallet::Config>::WeightInfo::list_cat().ref_time())]
		// #[transactional]
		// pub fn list_cat(
		// 	origin: OriginFor<T>,
		// 	id: T::CATIdentifier, //@@dexter should be one id only but don't limit at code level
		// 	start_block: BlockNumberFor<T>,
		// 	end_block: BlockNumberFor<T>,
		// 	offered_asset: AssetId, //@@dexter catalyst token but don't limit at code level
		// 	required_asset: AssetId, //@@dexter otto token but don't limit at code level
		// 	offered_amount: Balance,
		// 	next_price: FixedU128,
		// 	next_time_diminishing: FixedU128,
		// 	next_cat_number: u128,
		// 	price: Vec<(u128, u128, FixedU128, FixedU128)>, /* cat token starting number, cat
		// 	                                                 * token ending
		// 	                                                 * number,
		// 	                                                 * price_incremental,
		// 	                                                 * time_diminishing) */
		// ) -> DispatchResult {
		// 	T::CATAdminOrigin::ensure_origin(origin)?;
		// 	ensure!(!CATStatuses::<T>::contains_key(id), Error::<T>::CatIdInUse);
		// 	ensure!(
		// 		matches!(Self::cat_statuses(id.clone()), CATStatus::NotEnabled),
		// 		Error::<T>::CatAlreadyEnabled
		// 	);
		// 	let now = frame_system::Pallet::<T>::block_number();
		//
		// 	ensure!(start_block >= now, Error::<T>::InvalidStartblock);
		// 	ensure!(end_block > start_block, Error::<T>::InvalidEndblock);
		//
		// 	ensure!(
		// 		!offered_amount.is_zero() && offered_amount >= next_cat_number,
		// 		Error::<T>::InvalidOfferedAmount
		// 	);
		// 	for (start_num, end_num, _, _) in price.iter() {
		// 		ensure!(
		// 			start_num <= end_num && *end_num <= offered_amount,
		// 			Error::<T>::InvalidPrice
		// 		);
		// 	}
		// 	// only when need to specify total amount
		//
		// 	CATStatuses::<T>::insert(
		// 		id,
		// 		CATStatus::Enabled(CATParameters {
		// 			offered_asset,
		// 			required_asset,
		// 			offered_amount,
		// 			start_block,
		// 			end_block,
		// 			next_price,
		// 			next_time_diminishing,
		// 			next_cat_number,
		// 			price,
		// 		}),
		// 	);
		// 	Self::deposit_event(Event::CATEnabled(
		// 		id,
		// 		start_block,
		// 		end_block,
		// 		offered_asset,
		// 		offered_amount,
		// 		required_asset,
		// 	));
		// 	Ok(())
		// }
		//
		// #[pallet::call_index(6)]
		// #[pallet::weight(<T as pallet::Config>::WeightInfo::disable_cat().ref_time())]
		// #[transactional]
		// pub fn disable_cat(origin: OriginFor<T>, id: T::CATIdentifier) -> DispatchResult {
		// 	T::CATAdminOrigin::ensure_origin(origin)?;
		// 	Self::do_disable_cat(id);
		// 	Self::deposit_event(Event::CATDisabled(id));
		// 	Ok(())
		// }
		//
		// #[pallet::call_index(7)]
		// #[pallet::weight(<T as pallet::Config>::WeightInfo::participate().ref_time())]
		// #[transactional]
		// pub fn participate(
		// 	origin: OriginFor<T>,
		// 	id: T::CATIdentifier,
		// 	cat_amount: Balance,
		// 	required_asset: AssetId,
		// ) -> DispatchResult {
		// 	let who = <T as pallet::Config>::VerifiedUserOrigin::ensure_origin(origin)?;
		// 	let s = Self::cat_statuses(id);
		// 	let now = <frame_system::Pallet<T>>::block_number();
		//
		// 	// When plug ieo enables, won't accept new cata mint with plug token
		// 	if let IEOStatus::Enabled(_end_block) = Self::ieo_statuses(id) {
		// 		ensure!(
		// 			required_asset != T::PLUGAssetId::get(),
		// 			Error::<T>::PlugIsDisabledWhenIeoEnabled
		// 		);
		// 	}
		//
		// 	match s {
		// 		CATStatus::<_, _>::Enabled(parameters) => {
		// 			let start_block = parameters.start_block;
		// 			ensure!(start_block <= now, Error::<T>::CatIsNotStarted);
		//
		// 			let vault_account = Self::get_vault_account(id).unwrap();
		// 			// find the price range
		// 			let mut next_price = parameters.next_price;
		// 			let mut next_time_diminishing = parameters.next_time_diminishing;
		// 			let mut next_cat_number = parameters.next_cat_number;
		// 			let mut target_cat_amount = cat_amount;
		// 			let mut accumulate_cost = FixedU128::zero();
		// 			let offered_asset = parameters.offered_asset;
		// 			let required_asset_rate = Self::get_asset_rate(required_asset)?;
		// 			let offered_amount = parameters.offered_amount;
		// 			let end_block = parameters.end_block;
		// 			let price = parameters.price;
		// 			for (_, end_number, price_inc, time_diminishing) in price.iter() {
		// 				if target_cat_amount == 0 {
		// 					break
		// 				}
		// 				if next_cat_number > *end_number {
		// 					continue
		// 				}
		//
		// 				let tokens_in_range = if next_cat_number + target_cat_amount <= *end_number
		// 				{
		// 					target_cat_amount
		// 				} else {
		// 					end_number - next_cat_number + 1
		// 				};
		// 				accumulate_cost = accumulate_cost +
		// 					next_price
		// 						.saturating_mul(required_asset_rate)
		// 						.saturating_mul(
		// 							price_inc.saturating_pow(tokens_in_range as usize) -
		// 								FixedU128::one(),
		// 						)
		// 						.checked_div(&price_inc.saturating_sub(FixedU128::one()))
		// 						.unwrap_or(FixedU128::zero());
		// 				next_price = next_price
		// 					.saturating_mul(price_inc.saturating_pow(tokens_in_range as usize));
		// 				next_cat_number += tokens_in_range;
		// 				target_cat_amount -= tokens_in_range;
		// 				next_time_diminishing = *time_diminishing;
		// 			}
		//
		// 			//check next_cat_number whether exceed max supploy of catalyst token
		// 			ensure!(
		// 				next_cat_number <= offered_amount + 1,
		// 				Error::<T>::InvalidPurchaseAmount
		// 			);
		//
		// 			plug_utils::Pallet::<T>::asset_token_transfer(
		// 				required_asset,
		// 				&who,
		// 				&vault_account,
		// 				accumulate_cost.into_inner(),
		// 				false,
		// 			)?;
		//
		// 			plug_utils::Pallet::<T>::token_mint_into(
		// 				parameters.offered_asset,
		// 				&who,
		// 				cat_amount,
		// 			)?;
		//
		// 			let current_cat_price = Self::get_cat_price_at(next_cat_number - 1, &price);
		//
		// 			let new_parameter = CATParameters {
		// 				offered_asset,
		// 				required_asset,
		// 				offered_amount,
		// 				start_block,
		// 				end_block,
		// 				next_price,
		// 				next_time_diminishing,
		// 				next_cat_number,
		// 				price,
		// 			};
		// 			CATStatuses::<T>::mutate(id, |status| {
		// 				*status = CATStatus::Enabled(new_parameter);
		// 			});
		// 			Self::update_reward_storage(id, current_cat_price, next_time_diminishing)?;
		//
		// 			Self::deposit_event(Event::OfferReceived(
		// 				id,
		// 				who,
		// 				cat_amount,
		// 				accumulate_cost.into_inner(),
		// 			));
		// 			Ok(())
		// 		},
		// 		_ => Err(Error::<T>::CatIsNotEnabled)?,
		// 	}
		// }
		//
		// #[pallet::call_index(8)]
		// #[pallet::weight(<T as pallet::Config>::WeightInfo::set_price_parameters().ref_time())]
		// #[transactional]
		// pub fn set_price_parameters(
		// 	origin: OriginFor<T>,
		// 	id: T::CATIdentifier,
		// 	next_price: FixedU128,
		// 	next_time_diminishing: FixedU128,
		// 	price: Vec<(u128, u128, FixedU128, FixedU128)>, /* cat token starting number, cat
		// 	                                                 * token ending
		// 	                                                 * number,
		// 	                                                 * price_incremental,
		// 	                                                 * time_diminishing) */
		// ) -> DispatchResult {
		// 	T::CATAdminOrigin::ensure_origin(origin)?;
		// 	ensure!(CATStatuses::<T>::contains_key(id), Error::<T>::CatIdNotSet);
		//
		// 	let s = Self::cat_statuses(id);
		// 	match s {
		// 		CATStatus::<_, _>::Enabled(parameters) => {
		// 			let offered_amount = parameters.offered_amount;
		// 			for (start_num, end_num, _, _) in price.iter() {
		// 				ensure!(
		// 					start_num <= end_num && *end_num <= offered_amount,
		// 					Error::<T>::InvalidPrice
		// 				);
		// 			}
		// 			let offered_asset = parameters.offered_asset;
		// 			let required_asset = parameters.required_asset;
		// 			let start_block = parameters.start_block;
		// 			let end_block = parameters.end_block;
		// 			let next_cat_number = parameters.next_cat_number;
		//
		// 			let new_parameter = CATParameters {
		// 				offered_asset,
		// 				required_asset,
		// 				offered_amount,
		// 				start_block,
		// 				end_block,
		// 				next_price,
		// 				next_time_diminishing,
		// 				next_cat_number,
		// 				price,
		// 			};
		//
		// 			let current_cat_price =
		// 				Self::get_cat_price_at(next_cat_number - 1, &new_parameter.price);
		//
		// 			CATStatuses::<T>::mutate(id, |status| {
		// 				*status = CATStatus::Enabled(new_parameter.clone());
		// 			});
		// 			Self::update_reward_storage(id, current_cat_price, next_time_diminishing)?;
		//
		// 			Self::deposit_event(Event::CATPriceSet(
		// 				id,
		// 				new_parameter.next_price,
		// 				new_parameter.price,
		// 			));
		// 		},
		// 		_ => Err(Error::<T>::CatIsNotEnabled)?,
		// 	}
		// 	Ok(())
		// }
		//
		// #[pallet::call_index(9)]
		// #[pallet::weight(<T as pallet::Config>::WeightInfo::pay_unsigned().ref_time())]
		// #[transactional]
		// pub fn pay_unsigned(
		// 	origin: OriginFor<T>,
		// 	_current_block: BlockNumberFor<T>,
		// ) -> DispatchResult {
		// 	ensure_none(origin)?;
		// 	// TODO: Check
		// 	log::warn!("start processing the payouts");
		// 	let mut count = 0u32;
		// 	for (id, status) in IEOStatuses::<T>::iter() {
		// 		if let IEOStatus::<_, _>::Paying(available_voucher) = status {
		// 			let start_key = Self::ieo_payout_pivot(id);
		// 			let mut map_iterator = match IEOPayoutPivot::<T>::contains_key(id) {
		// 				true => <IEOOrderbook<T>>::iter_from(start_key.clone()),
		// 				false => <IEOOrderbook<T>>::iter(),
		// 			};
		// 			let total_gathered = Self::ieo_total_gathered(id);
		// 			log::warn!("total gathered {:?}", total_gathered);
		// 			let denominator: U256 = U256::from(total_gathered);
		//
		// 			while let Some((id, who, entry)) = map_iterator.next() {
		// 				if entry.1 {
		// 					continue
		// 				}
		// 				let numerator: U256 =
		// 					U256::from(available_voucher).saturating_mul(U256::from(entry.0));
		// 				let share = numerator
		// 					.checked_div(denominator)
		// 					.and_then(|n| TryInto::<Balance>::try_into(n).ok())
		// 					.unwrap_or_else(Zero::zero);
		// 				let transfer_result = plug_utils::Pallet::<T>::token_mint_into(
		// 					T::CatalystVoucherAssetId::get(),
		// 					&who,
		// 					share,
		// 				);
		// 				if transfer_result.is_ok() {
		// 					Self::deposit_event(Event::PlugCataIEOPaidOut(who.clone(), share));
		// 				}
		// 				// TODO: what if transfer_result is not true?
		// 				IEOOrderbook::<T>::mutate(id.clone(), who.clone(), |entry| {
		// 					*entry = (entry.0, true);
		// 				});
		// 				count += 1;
		// 				if count > T::PayoutBatchSize::get() {
		// 					break
		// 				}
		// 			}
		// 			let current_last_raw_key = map_iterator.last_raw_key().to_vec();
		// 			if current_last_raw_key == start_key.clone() {
		// 				IEOStatuses::<T>::mutate(id, |status| {
		// 					*status = IEOStatus::Done;
		// 				});
		// 				let _ = IEOOrderbook::<T>::drain_prefix(id).collect::<Vec<_>>();
		// 				IEOTotalGathered::<T>::remove(id);
		// 				IEOPayoutPivot::<T>::remove(id);
		// 				NextUnsignedAt::<T>::kill();
		// 				Self::deposit_event(Event::PlugCataIEODone());
		//
		// 				return Ok(())
		// 			}
		// 			IEOPayoutPivot::<T>::mutate(id, |last_raw_key| {
		// 				*last_raw_key = current_last_raw_key;
		// 			});
		// 		}
		// 	}
		//
		// 	let current_block = <frame_system::Pallet<T>>::block_number();
		// 	log::warn!("current block {:?}", current_block);
		// 	let next_unsigned_at = current_block + T::UnsignedInterval::get();
		// 	<NextUnsignedAt<T>>::put(next_unsigned_at);
		// 	log::warn!("proposed next unsigned at {:?}", next_unsigned_at);
		// 	Ok(())
		// }
	}

	// #[pallet::validate_unsigned]
	// impl<T: Config> ValidateUnsigned for Pallet<T> {
	// 	type Call = Call<T>;
	//
	// 	fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
	// 		match call {
	// 			Call::pay_unsigned { current_block } => {
	// 				let _current_block = <frame_system::Pallet<T>>::block_number();
	// 				if &_current_block < current_block {
	// 					return InvalidTransaction::Future.into()
	// 				}
	// 				ValidTransaction::with_tag_prefix("IEOChainWorker")
	// 					.priority(IEO_UNSIGNED_PRIORITY)
	// 					.and_provides(current_block)
	// 					.longevity(64_u64)
	// 					.propagate(true)
	// 					.build()
	// 			},
	// 			_ => InvalidTransaction::Call.into(),
	// 		}
	// 	}
	// }

	impl<T: Config> Pallet<T> {
		pub fn account_id() -> T::AccountId {
			T::CATPalletId::get().into_account_truncating()
		}
		//
		// pub fn get_total_cost(
		// 	id: T::CATIdentifier,
		// 	cat_amount: Balance,
		// 	required_asset: AssetId,
		// ) -> Result<Balance, DispatchError> {
		// 	let s = Self::cat_statuses(id);
		//
		// 	match s {
		// 		CATStatus::<_, _>::Enabled(parameters) => {
		// 			let mut next_price = parameters.next_price;
		// 			let mut next_cat_number = parameters.next_cat_number;
		// 			let required_asset_rate = Self::get_asset_rate(required_asset)?;
		// 			let mut target_cat_amount = cat_amount;
		// 			let mut accumulate_cost = FixedU128::zero();
		// 			for (_start_number, end_number, price_inc, _) in parameters.price.iter() {
		// 				if target_cat_amount == 0 {
		// 					break
		// 				}
		// 				if next_cat_number > *end_number {
		// 					continue
		// 				}
		//
		// 				log::info!("next cat number {:?}", next_cat_number);
		// 				let tokens_in_range = if next_cat_number + target_cat_amount <= *end_number
		// 				{
		// 					target_cat_amount
		// 				} else {
		// 					end_number - next_cat_number + 1
		// 				};
		// 				log::info!("tokens in range {:?}", tokens_in_range);
		// 				log::info!("accumulated cost before {:?}", accumulate_cost);
		// 				log::info!("next price before {:?}", next_price);
		// 				accumulate_cost = accumulate_cost +
		// 					next_price
		// 						.saturating_mul(required_asset_rate)
		// 						.saturating_mul(
		// 							price_inc.saturating_pow(tokens_in_range as usize) -
		// 								FixedU128::one(),
		// 						)
		// 						.checked_div(&price_inc.saturating_sub(FixedU128::one()))
		// 						.unwrap_or(FixedU128::zero());
		// 				log::info!("accumulated cost after {:?}", accumulate_cost);
		// 				next_price = next_price
		// 					.saturating_mul(price_inc.saturating_pow(tokens_in_range as usize));
		// 				log::info!("next price after {:?}", next_price);
		// 				next_cat_number += tokens_in_range;
		// 				log::info!("next cat number {:?}", next_cat_number);
		// 				target_cat_amount -= tokens_in_range;
		// 				log::info!("target cat amount {:?}", target_cat_amount);
		// 			}
		// 			Ok(accumulate_cost.into_inner())
		// 		},
		// 		_ => Ok(0),
		// 	}
		// }
		//
		// pub fn get_vault_account(cat_id: T::CATIdentifier) -> Option<T::AccountId> {
		// 	// use cat module account id and offered asset id as entropy to generate reward vault
		// 	// id.
		// 	let entropy =
		// 		(b"modlpy/palletcat", Self::account_id(), cat_id).using_encoded(blake2_256);
		// 	if let Ok(cat_vault_account) = T::AccountId::decode(&mut &entropy[..]) {
		// 		return Some(cat_vault_account)
		// 	}
		// 	None
		// }
		//
		// fn do_disable_cat(id: T::CATIdentifier) {
		// 	CATStatuses::<T>::mutate(id, |status| {
		// 		*status = CATStatus::NotEnabled;
		// 	});
		// }
		//
		// fn update_reward_storage(
		// 	id: T::CATIdentifier,
		// 	current_cat_price: FixedU128,
		// 	next_time_diminishing: FixedU128,
		// ) -> DispatchResult {
		// 	let s = Self::cat_statuses(id);
		// 	match s {
		// 		CATStatus::<_, _>::Enabled(parameters) => {
		// 			let current_cat_number = parameters.next_cat_number.saturating_sub(1);
		// 			let mut real_next_time_diminishing = next_time_diminishing;
		// 			let cat_number = T::TimeDiminishingNo::get();
		// 			let base_time_diminishing = T::TimeDiminishingBase::get();
		// 			let time_diminishing_factor = T::TimeDiminishingFactor::get();
		//
		// 			if current_cat_number >= cat_number {
		// 				real_next_time_diminishing = base_time_diminishing.saturating_mul(
		// 					time_diminishing_factor.saturating_pow(
		// 						(current_cat_number - cat_number).try_into().unwrap(),
		// 					),
		// 				);
		// 			}
		//
		// 			pallet_catalyst_reward::Pallet::<T>::update_cat_reward_storage(
		// 				current_cat_price,
		// 				real_next_time_diminishing,
		// 			)?; //update reward pallet
		// 		},
		// 		_ => Err(Error::<T>::CatIsNotEnabled)?,
		// 	}
		// 	Ok(())
		// }
		//
		// pub fn rate_between_assets(
		// 	asset_a: AssetId,
		// 	asset_b: AssetId,
		// 	base_rate: u128,
		// ) -> Result<FixedU128, DispatchError> {
		// 	let asset_a_decimals = plug_utils::Pallet::<T>::asset_decimals(asset_a) as u32;
		// 	let asset_b_decimals = plug_utils::Pallet::<T>::asset_decimals(asset_b) as u32;
		//
		// 	match asset_a_decimals.cmp(&asset_b_decimals) {
		// 		Ordering::Greater =>
		// 			Ok((base_rate * 10u128.pow(asset_a_decimals - asset_b_decimals)).into()),
		// 		Ordering::Less => {
		// 			let base_rate = FixedU128::from(base_rate);
		// 			let decimals_rate = FixedU128::one()
		// 				.checked_div(&FixedU128::from(
		// 					10u128.pow(asset_b_decimals - asset_a_decimals),
		// 				))
		// 				.ok_or(Error::<T>::UnableToCalculateAssetRate)?;
		// 			return Ok(base_rate.saturating_mul(decimals_rate))
		// 		},
		// 		Ordering::Equal => Ok(base_rate.into()),
		// 	}
		// }
		//
		// fn get_asset_rate(asset: AssetId) -> Result<FixedU128, DispatchError> {
		// 	match asset {
		// 		NATIVE_TOKEN_OTTO_ASSET_ID => Ok(1.into()),
		// 		_ if asset == T::PLUGAssetId::get() =>
		// 			Self::rate_between_assets(asset, NATIVE_TOKEN_OTTO_ASSET_ID, PLUG_OTTOT_RATE),
		// 		_ => Err(Error::<T>::InvalidAssetId)?,
		// 	}
		// }
		//
		// fn do_disable_ieo(id: T::CATIdentifier) {
		// 	IEOStatuses::<T>::mutate(id, |status| {
		// 		*status = IEOStatus::NotEnabled;
		// 	});
		// }
		//
		// fn check_cata_status(id: T::CATIdentifier) -> DispatchResult {
		// 	if let CATStatus::<_, _>::Enabled(parameters) = Self::cat_statuses(id) {
		// 		let start_block = parameters.start_block;
		// 		let now = <frame_system::Pallet<T>>::block_number();
		// 		ensure!(start_block <= now, Error::<T>::CatIsNotStarted);
		// 		Ok(())
		// 	} else {
		// 		return Err(Error::<T>::CatIsNotEnabled.into())
		// 	}
		// }
		//
		// fn ieo_offchain_worker(now: BlockNumberFor<T>) -> Result<(),
		// plug_primitives::OffchainErr> { 	if !sp_io::offchain::is_validator() {
		// 		return Err(plug_primitives::OffchainErr::NotValidator)
		// 	}
		// 	let next_unsigned_at = <NextUnsignedAt<T>>::get();
		// 	if next_unsigned_at > now {
		// 		return Err(plug_primitives::OffchainErr::TooEarly)
		// 	}
		//
		// 	// if detect any ieo is at paying stage, kicks off pay unsigned flow.
		// 	for (_id, status) in IEOStatuses::<T>::iter() {
		// 		match status {
		// 			IEOStatus::<_, _>::Paying(_available_cata) => {
		// 				log::info!("start sending unsigned paying tx");
		// 				let call = Call::pay_unsigned { current_block: now };
		// 				SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
		// 					.unwrap_or(());
		// 				break
		// 			},
		// 			_ => {},
		// 		}
		// 	}
		// 	Ok(())
		// }
		//
		// /// (cata_voucher, next_price, next_time_diminishing, next_cat_number)
		// pub fn do_finalize_ieo(
		// 	total_gather: Balance,
		// 	next_price: FixedU128,
		// 	next_cat_number: u128,
		// 	next_time_diminishing: FixedU128,
		// 	price: &Vec<(u128, u128, FixedU128, FixedU128)>,
		// ) -> (Balance, FixedU128, FixedU128, u128) {
		// 	let mut accumulate_cost;
		// 	// next_price
		// 	let mut next_price = next_price;
		// 	let mut next_time_diminishing = next_time_diminishing;
		// 	let mut next_cat_number = next_cat_number;
		// 	let mut range_start_number = next_cat_number;
		// 	let mut cata_voucher: Balance = 0;
		// 	let mut cross_range_integer = 0u128;
		//
		// 	let decimals_diff =
		// 		Self::get_asset_rate(T::PLUGAssetId::get()).unwrap_or(FixedU128::one());
		//
		// 	let mut remaining_asset_amount = FixedU128::from_inner(total_gather)
		// 		.checked_div(&decimals_diff)
		// 		.unwrap_or(FixedU128::zero());
		//
		// 	for (start_number, end_number, price_inc, time_diminishing) in price.iter() {
		// 		if end_number < &range_start_number {
		// 			continue
		// 		}
		//
		// 		let start_number = start_number.max(&range_start_number);
		// 		let tokens_in_range = (end_number + 1 - start_number) as usize;
		//
		// 		let range_max_accumulate_cost =
		// 			Self::get_cost(next_price, FixedU128::zero(), *price_inc, tokens_in_range);
		//
		// 		if remaining_asset_amount <= range_max_accumulate_cost {
		// 			let (index, remaining_asset_amount) = Self::search_upper_target_catalyst_index(
		// 				*start_number,
		// 				*end_number,
		// 				remaining_asset_amount,
		// 				*price_inc,
		// 				next_price,
		// 			);
		//
		// 			next_price = next_price
		// 				.saturating_mul(price_inc.saturating_pow((index - start_number) as usize));
		// 			next_time_diminishing = *time_diminishing;
		// 			cata_voucher = Self::calculate_cata_voucher(
		// 				next_price,
		// 				index - start_number + cross_range_integer,
		// 				remaining_asset_amount,
		// 			);
		//
		// 			next_cat_number = index;
		//
		// 			break
		// 		} else {
		// 			accumulate_cost = range_max_accumulate_cost;
		// 			remaining_asset_amount = remaining_asset_amount.saturating_sub(accumulate_cost);
		// 			cross_range_integer += tokens_in_range as u128;
		// 		}
		// 		range_start_number = end_number + 1;
		// 		next_price = next_price.saturating_mul(price_inc.saturating_pow(tokens_in_range));
		// 		next_time_diminishing = *time_diminishing;
		// 	}
		//
		// 	(cata_voucher, next_price, next_time_diminishing, next_cat_number)
		// }
		//
		// fn calculate_cata_voucher(
		// 	base_price: FixedU128,
		// 	integer_cata_amount: u128,
		// 	fraction_asset_amount: FixedU128,
		// ) -> Balance {
		// 	let fraction_cata_amount = fraction_asset_amount
		// 		.checked_div(&base_price) // TODO: asset rate
		// 		.unwrap_or(FixedU128::zero());
		//
		// 	let cata_voucher_decimals =
		// 		plug_utils::Pallet::<T>::asset_decimals(T::CatalystVoucherAssetId::get()) as u32;
		//
		// 	let cata_voucher_amount = TryInto::<Balance>::try_into(
		// 		FixedU128::from(integer_cata_amount)
		// 			.saturating_add(fraction_cata_amount)
		// 			.into_inner() / FixedU128::DIV
		// 			.checked_div((10 as Balance).pow(cata_voucher_decimals))
		// 			.unwrap_or(0),
		// 	)
		// 	.unwrap_or(0);
		//
		// 	// remove fraction parts
		// 	cata_voucher_amount - (cata_voucher_amount % (10 as Balance).pow(cata_voucher_decimals))
		// }
		//
		// fn get_cost(
		// 	base_price: FixedU128,
		// 	base_total: FixedU128,
		// 	price_inc: FixedU128,
		// 	tokens_in_range: usize,
		// ) -> FixedU128 {
		// 	base_total +
		// 		base_price
		// 			.saturating_mul(
		// 				price_inc.saturating_pow(tokens_in_range).saturating_sub(FixedU128::one()),
		// 			)
		// 			.checked_div(&price_inc.saturating_sub(FixedU128::one()))
		// 			.unwrap_or(FixedU128::zero())
		// }
		//
		// fn search_upper_target_catalyst_index(
		// 	low: u128,
		// 	high: u128,
		// 	target: FixedU128,
		// 	price_inc: FixedU128,
		// 	base_price: FixedU128,
		// ) -> (u128, FixedU128) {
		// 	let mut low = low;
		// 	let start = low;
		// 	let mut high = high;
		//
		// 	while low < high {
		// 		let mid = low + (high + 1 - low) / 2;
		// 		let range_cost = Self::get_cost(
		// 			base_price,
		// 			FixedU128::zero(),
		// 			price_inc,
		// 			(mid - start) as usize,
		// 		);
		//
		// 		if range_cost <= target {
		// 			low = mid;
		// 		} else {
		// 			high = mid - 1;
		// 		}
		// 	}
		//
		// 	let remaining = target.saturating_sub(Self::get_cost(
		// 		base_price,
		// 		FixedU128::zero(),
		// 		price_inc,
		// 		(low - start) as usize,
		// 	));
		//
		// 	(low, remaining)
		// }
		//
		// pub fn get_cat_price_at(
		// 	target_cat_number: u128,
		// 	price: &Vec<(u128, u128, FixedU128, FixedU128)>,
		// ) -> FixedU128 {
		// 	let mut next_price = FixedU128::one()
		// 		.checked_div(&FixedU128::from(10u128.pow(6)))
		// 		.unwrap_or(FixedU128::zero());
		// 	for (start_number, end_number, price_inc, _) in price.iter() {
		// 		if target_cat_number == 0 {
		// 			break
		// 		}
		//
		// 		if &target_cat_number < start_number {
		// 			break
		// 		}
		//
		// 		let tokens_in_range =
		// 			(end_number.min(&(target_cat_number - 1)) + 1 - start_number) as usize;
		//
		// 		next_price = next_price.saturating_mul(price_inc.saturating_pow(tokens_in_range));
		// 	}
		//
		// 	next_price
		// }
	}
}
