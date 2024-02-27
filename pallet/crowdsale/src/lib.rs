// Copyright 2023-2024 Futureverse Corporation Limited
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

//! # Pallet Crowdsale
//!
//! A pallet which enables anyone to create a crowdsale for an NFT collection.
//! A softcap is set at the start of the crowdsale; vouchers (asset/erc20) are distributed
//! to participants once the crowdsale ends.
//! If softcap is not reached, the difference is refunded to the crowdsale creator.
//! The vouchers can be used to redeemed for NFTs from the collection.
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;

use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{AccountIdConversion, Zero},
		SaturatedConversion, Saturating,
	},
	traits::fungibles::{self, Mutate, Transfer},
	transactional, PalletId,
};
use frame_system::{
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::*,
};
use pallet_nft::traits::NFTExt;
use seed_pallet_common::{log, CreateExt, InspectExt};
use seed_primitives::{AssetId, Balance, CollectionUuid, OffchainErr, TokenCount};
use sp_std::vec;

pub mod types;
use types::*;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;
mod impls;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
// mod weights;

// pub use weights::WeightInfo;

/// The logging target for this pallet
#[allow(dead_code)]
pub(crate) const LOG_TARGET: &str = "crowdsale";

pub const CROWDSALE_DIST_UNSIGNED_PRIORITY: TransactionPriority =
	TransactionPriority::max_value() / 2;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + SendTransactionTypes<Call<Self>> {
		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// This pallet's Id, used for deriving a sovereign account ID
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Currency implementation to deal with assets.
		type MultiCurrency: InspectExt
			+ CreateExt<AccountId = Self::AccountId>
			+ fungibles::Transfer<Self::AccountId, Balance = Balance>
			+ fungibles::Inspect<Self::AccountId, AssetId = AssetId>
			+ fungibles::InspectMetadata<Self::AccountId>
			+ fungibles::Mutate<Self::AccountId>;

		/// NFT Extension, used to retrieve collection data
		type NFTExt: pallet_nft::traits::NFTExt<AccountId = Self::AccountId>;

		/// The maximum number of sales that can be queued for completion in a single block
		type MaxSalesPerBlock: Get<u32>;

		/// The maximum number of sales that can be active at one time
		type MaxConsecutiveSales: Get<u32>;

		/// The maximum number of payments that can be processed in the offchain worker per block
		type MaxPaymentsPerBlock: Get<u32>;

		/// The maximum duration of a sale
		type MaxSaleDuration: Get<Self::BlockNumber>;

		// / Interface to access weight values
		// type WeightInfo: WeightInfo;
	}

	/// The next available sale id
	#[pallet::storage]
	pub type NextSaleId<T: Config> = StorageValue<_, SaleId, ValueQuery>;

	/// Map from sale id to its information
	#[pallet::storage]
	pub type SaleInfo<T: Config> =
		StorageMap<_, Twox64Concat, SaleId, SaleInformation<T::AccountId, T::BlockNumber>>;

	/// User participation in the sale
	/// sale_id -> user -> payment_asset contribution amount
	#[pallet::storage]
	pub type SaleParticipation<T: Config> =
		StorageDoubleMap<_, Twox64Concat, SaleId, Twox64Concat, T::AccountId, Balance, OptionQuery>;

	/// Map from block number to the sales that will end at that block
	/// The tuple represents the sale id and the current sale participant distribution index
	#[pallet::storage]
	pub type SaleEndBlocks<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::BlockNumber,
		BoundedVec<SaleId, T::MaxSalesPerBlock>,
		OptionQuery,
	>;

	/// A list of all sales currently being distributed
	#[pallet::storage]
	pub type DistributingSales<T: Config> =
		StorageValue<_, BoundedVec<SaleId, T::MaxConsecutiveSales>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Crowdsale created
		CrowdsaleCreated { sale_id: SaleId, info: SaleInformation<T::AccountId, T::BlockNumber> },
		/// Crowdsale enabled
		CrowdsaleEnabled {
			sale_id: SaleId,
			info: SaleInformation<T::AccountId, T::BlockNumber>,
			end_block: T::BlockNumber,
		},
		/// Crowdsale participated
		CrowdsaleParticipated {
			sale_id: SaleId,
			who: T::AccountId,
			asset: AssetId,
			amount: Balance,
		},
		/// Crowdsale NFT redeemed
		CrowdsaleNFTRedeemed {
			sale_id: SaleId,
			who: T::AccountId,
			collection_id: CollectionUuid,
			quantity: TokenCount,
		},
		/// Crowdsale closed
		CrowdsaleClosed { sale_id: SaleId, info: SaleInformation<T::AccountId, T::BlockNumber> },
		/// Crowdsale vouchers claimed
		CrowdsaleVouchersClaimed { sale_id: SaleId, who: T::AccountId, amount: Balance },
		/// Crowdsale distribution has been completed and all vouchers paid out
		CrowdsaleDistributionComplete { sale_id: SaleId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Access denied
		AccessDenied,
		/// There are no remaining sale ids
		NoAvailableIds,
		/// The start block is greater than the end block
		InvalidBlockRange,
		/// Crowdsale was not found
		CrowdsaleNotFound,
		/// Invalid crowdsale status
		InvalidCrowdsaleStatus,
		/// Crowdsale is not enabled
		CrowdsaleNotEnabled,
		/// Invalid asset id
		InvalidAsset,
		/// The voucher claim could not be completed due to invalid voucher supply
		VoucherClaimFailed,
		/// Failed to create voucher asset
		CreateAssetFailed,
		/// Asset transfer failed
		AssetTransferFailed,
		/// The NFT collection max issuance is not set
		MaxIssuanceNotSet,
		/// The NFT collection must not contain any minted NFTs
		CollectionIssuanceNotZero,
		/// There are too many sales queued for this block, try again on a different block
		TooManySales,
		/// Vouchers have already been claimed
		VouchersAlreadyClaimed,
		/// Automatic trigger of sales distribution has failed
		DistributingSaleFailed,
		/// The sale duration is too long
		SaleDurationTooLong,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Check and close all expired listings
		fn on_initialize(now: T::BlockNumber) -> Weight {
			match Self::close_sales_at(now) {
				Ok(total_closed) =>
					log!(info, "✅ closed {} sales at block {:?}", total_closed, now),
				Err(e) => log!(error, "⛔️ failed to close sales at block {:?}: {:?}", now, e),
			};
			// TODO Benchmark this
			// <T as Config>::WeightInfo::close().mul(total_closed as u64)
			// total_closed == 1 read + 1 write per close
			// + 1 read + write for SaleEndBlocks
			Weight::zero()
		}

		/// Offchain worker processes closed sales to distribute voucher rewards to participants
		fn offchain_worker(now: T::BlockNumber) {
			if !sp_io::offchain::is_validator() {
				log!(
					error,
					"⛔️ offchain worker error at block [{:?}]: {:?}",
					now,
					OffchainErr::NotAValidator
				);
			}

			if !DistributingSales::<T>::get().is_empty() {
				log!(info, "⭐️ distributing rewards for crowdsales");
				let call = Call::distribute_crowdsale_rewards {};
				let _ = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());
			}
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			match call {
				Call::distribute_crowdsale_rewards {} => {
					// reject crowdsale distribution tx which have already been processed
					let now = <frame_system::Pallet<T>>::block_number();
					if DistributingSales::<T>::get().is_empty() {
						return InvalidTransaction::Stale.into()
					}
					ValidTransaction::with_tag_prefix("CrowdsaleDistOffchainWorker")
						.priority(CROWDSALE_DIST_UNSIGNED_PRIORITY)
						.and_provides(now)
						.longevity(64_u64)
						.propagate(true)
						.build()
				},
				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Initialize a new crowdsale with the given parameters.
		/// The provided collection max_issuance must be set and the collection must not have minted
		/// any NFTs.
		///
		/// Parameters:
		/// - `payment_asset`: Asset id of the token that will be used to redeem the NFTs at the end
		///   of the sale
		/// - `collection_id`: Collection id of the NFTs that will be minted/redeemed to the
		///   participants
		/// - `soft_cap_price`: Number/Ratio of payment_asset tokens that will be required to
		///   purchase vouchers; Note: this does not take into account asset decimals or voucher
		///   decimals
		/// - `vouchers_per_nft`: Number of vouchers required to redeem for a single NFT; Note: this
		///   does not take into account voucher decimals
		/// - `sale_duration`: How many blocks will the sale last once enabled
		///
		/// Emits `CrowdsaleCreated` event when successful.
		#[pallet::weight(0)]
		// #[pallet::weight(T::WeightInfo::initialize())]
		#[transactional]
		pub fn initialize(
			origin: OriginFor<T>,
			payment_asset: AssetId,
			collection_id: CollectionUuid,
			soft_cap_price: Balance,
			sale_duration: T::BlockNumber,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// increment the sale id, store it and use it
			let sale_id = NextSaleId::<T>::mutate(|id| -> Result<u64, DispatchError> {
				let current_id = *id;
				*id = id.checked_add(1).ok_or(Error::<T>::NoAvailableIds)?;
				Ok(current_id)
			})?;

			// ensure the asset exists
			if !T::MultiCurrency::exists(payment_asset) {
				return Err(Error::<T>::InvalidAsset.into())
			}

			// ensure soft_cap_price is not zero - prevent future div by zero
			ensure!(!soft_cap_price.is_zero(), Error::<T>::InvalidAsset);

			// TODO
			ensure!(sale_duration <= T::MaxSaleDuration::get(), Error::<T>::SaleDurationTooLong);

			// create crowdsale vault account which will temporary manage ownership and hold funds
			let vault = Self::vault_account(sale_id);

			// TODO: pass NFT collection ownership to the vault account
			// - this is required so collection owner cannot mint/rug to dilute the crowdsale

			let collection_info = T::NFTExt::get_collection_info(collection_id)?;
			ensure!(collection_info.max_issuance.is_some(), Error::<T>::MaxIssuanceNotSet);
			ensure!(
				collection_info.collection_issuance.is_zero(),
				Error::<T>::CollectionIssuanceNotZero
			);

			// create voucher asset
			let voucher_asset_id = Self::create_voucher_asset(&vault, sale_id)?;

			// store the sale information
			let sale_info = SaleInformation::<T::AccountId, T::BlockNumber> {
				status: SaleStatus::Pending(<frame_system::Pallet<T>>::block_number()),
				admin: who.clone(),
				vault,
				payment_asset,
				reward_collection_id: collection_id,
				soft_cap_price,
				funds_raised: 0,
				voucher: voucher_asset_id,
				sale_duration,
			};
			SaleInfo::<T>::insert(sale_id, sale_info.clone());

			Self::deposit_event(Event::CrowdsaleCreated { sale_id, info: sale_info });
			Ok(())
		}

		/// Enable a crowdsale for user participation.
		/// Only the crowdsale admin can call this function to enable the sale.
		/// This will enable the sale to be participated in by any user which has required
		/// payment_asset. The sale will be closed automatically once the sale_duration is met; the
		/// sale end block/time is based on current block + sale_duration.
		///
		/// Parameters:
		/// - `sale_id`: The id of the sale to enable
		///
		/// Emits `CrowdsaleEnabled` event when successful.
		#[pallet::weight(0)]
		#[transactional]
		pub fn enable(origin: OriginFor<T>, sale_id: SaleId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// update the sale status if the start block is met
			SaleInfo::<T>::try_mutate(sale_id, |sale_info| -> DispatchResult {
				let Some(sale_info) = sale_info else {
					return Err(Error::<T>::CrowdsaleNotFound.into());
				};

				// ensure the sale is not already enabled
				ensure!(
					matches!(sale_info.status, SaleStatus::Pending(_)),
					Error::<T>::InvalidCrowdsaleStatus
				);
				ensure!(sale_info.admin == who, Error::<T>::AccessDenied);

				// ensure start block is met and end block is not met
				let current_block = <frame_system::Pallet<T>>::block_number();
				let end_block = sale_info.sale_duration.saturating_add(current_block);

				// Append end block to SaleEndBlocks
				SaleEndBlocks::<T>::try_mutate(end_block, |sales| -> DispatchResult {
					if let Some(sales) = sales {
						sales.try_push(sale_id).map_err(|_| Error::<T>::TooManySales)?;
					} else {
						let new_sales = BoundedVec::truncate_from(vec![sale_id]);
						*sales = Some(new_sales);
					}
					Ok(())
				})?;

				// update the sale details
				sale_info.status = SaleStatus::Enabled(<frame_system::Pallet<T>>::block_number());

				Self::deposit_event(Event::CrowdsaleEnabled {
					sale_id,
					info: sale_info.clone(),
					end_block,
				});

				Ok(())
			})?;

			Ok(())
		}

		/// Participate in the crowdsale.
		/// Any user can call this function to participate in the crowdsale
		/// assuming the sale is enabled and the user has enough payment_asset tokens to
		/// participate. The tokens required to participate are transferred to the pallet account.
		///
		/// Parameters:
		/// - `sale_id`: The id of the sale to participate in
		/// - `amount`: The amount of tokens to participate with
		///
		/// Emits `CrowdsaleParticipated` event when successful.
		#[pallet::weight(0)]
		#[transactional]
		pub fn participate(
			origin: OriginFor<T>,
			sale_id: SaleId,
			amount: Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// update the sale status if the start block is met
			SaleInfo::<T>::try_mutate(sale_id, |sale_info: &mut Option<SaleInformation<_, _>>| {
				let Some(sale_info) = sale_info else {
					return Err(Error::<T>::CrowdsaleNotFound);
				};

				// ensure the sale is enabled
				ensure!(
					matches!(sale_info.status, SaleStatus::Enabled(_)),
					Error::<T>::CrowdsaleNotEnabled
				);

				// transfer payment tokens to the crowdsale vault
				T::MultiCurrency::transfer(
					sale_info.payment_asset,
					&who,
					&sale_info.vault,
					amount,
					false,
				)
				.map_err(|_| Error::<T>::AssetTransferFailed)?;

				// update the sale funds
				sale_info.funds_raised = sale_info.funds_raised.saturating_add(amount);

				// update the user's contribution
				SaleParticipation::<T>::mutate(sale_id, who.clone(), |maybe_contribute| {
					match maybe_contribute {
						Some(contribution) => *contribution = contribution.saturating_add(amount),
						None => *maybe_contribute = Some(amount),
					}
				});

				Self::deposit_event(Event::CrowdsaleParticipated {
					sale_id,
					who,
					asset: sale_info.payment_asset,
					amount,
				});

				Ok(())
			})?;

			Ok(())
		}

		/// Distribute vouchers for a given crowdsale - based on the amount of funds raised, the NFT
		/// collection max issuance and respective participant contributions.
		/// The crowdsale must be closed (automated on chain via `on_initialize`).
		/// The extrinsic is automatically called by offchain worker once the sale is closed (end
		/// block reached). The extrinsic can also manually be called by anyone.
		///
		/// Parameters:
		/// - `sale_id`: The id of the sale to distribute the vouchers for
		///
		/// Emits `CrowdsaleVouchersDistributed` event when successful.
		#[pallet::weight(0)]
		#[transactional]
		pub fn distribute_crowdsale_rewards(origin: OriginFor<T>) -> DispatchResult {
			ensure_none(origin)?;

			let mut sale_ids: Vec<SaleId> = DistributingSales::<T>::get().into_inner();

			// Get the first sale_id and process in FiFo order
			let sale_id = *sale_ids.first().ok_or(Error::<T>::CrowdsaleNotFound)?;
			let mut sale_info = SaleInfo::<T>::get(sale_id).ok_or(Error::<T>::CrowdsaleNotFound)?;

			// ensure the sale is in the distribution phase
			let SaleStatus::Distributing(_, mut total_paid_contributions, mut voucher_current_supply) = sale_info.status else {
				return Err(Error::<T>::InvalidCrowdsaleStatus.into());
			};

			// get amount of claimable vouchers based on the user's contribution
			let collection_info = T::NFTExt::get_collection_info(sale_info.reward_collection_id)?;
			let voucher_max_supply =
				collection_info.max_issuance.ok_or(Error::<T>::MaxIssuanceNotSet)?;

			let mut contributions_iterator = SaleParticipation::<T>::drain_prefix(sale_id);
			let mut payout_complete: bool = false;

			for _ in 0..T::MaxPaymentsPerBlock::get() {
				// Note. There is a very small chance that this payout_complete check will not
				// execute on the last iteration, this is because the for loop may end before
				// it realizes it is the last item in the iterator. Due to the iterator being
				// a drain_prefix, it is safest to ignore this and end the distribution
				// the next time this function is called
				// The chance of this happening is 1 / MaxPaymentsPerBlock
				let Some((who, contribution)) = contributions_iterator.next() else {
					payout_complete = true;
					break;
				};

				let Ok(claimable_vouchers) = Self::mint_user_vouchers(
					who.clone(),
					sale_id,
					&sale_info,
					contribution.into(),
					voucher_max_supply.into(),
					voucher_current_supply,
					total_paid_contributions.into(),
				) else {
					log!(
						error,
						"⛔️ failed to mint voucher rewards for user {:?} in sale {:?}",
						who,
						sale_id,
					);
					continue;
				};

				voucher_current_supply = voucher_current_supply.saturating_add(claimable_vouchers);
				total_paid_contributions = total_paid_contributions.saturating_add(contribution);
			}

			let block_number = <frame_system::Pallet<T>>::block_number();
			if payout_complete {
				// Distribution complete
				sale_info.status = SaleStatus::Ended(block_number, voucher_current_supply);
				Self::deposit_event(Event::CrowdsaleDistributionComplete { sale_id });
				// TODO Verify this:
				sale_ids = sale_ids.drain(1..).collect();
				DistributingSales::<T>::put(BoundedVec::truncate_from(sale_ids));
			} else {
				// Update total_contributions
				sale_info.status = SaleStatus::Distributing(
					block_number,
					total_paid_contributions,
					voucher_current_supply,
				);
			}

			Ok(())
		}

		/// Claim the vouchers after a sale has concluded, based on caller's contribution.
		/// The vouchers are redeemable 1:1 with the NFTs in the collection (excluding decimals).
		/// A successful claim will remove the user's contribution from the sale and mint the
		/// vouchers to the user.
		///
		/// Parameters:
		/// - `sale_id`: The id of the sale to claim the vouchers from
		///
		/// Emits `CrowdsaleVouchersClaimed` event when successful.
		#[pallet::weight(0)]
		#[transactional]
		pub fn claim_voucher(origin: OriginFor<T>, sale_id: SaleId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			SaleInfo::<T>::try_mutate(sale_id, |sale_info| -> DispatchResult {
				let Some(sale_info) = sale_info else {
					return Err(Error::<T>::CrowdsaleNotFound.into());
				};

				// ensure the sale is in the distribution phase
				let SaleStatus::Distributing(_, total_paid_contributions, voucher_current_supply) = sale_info.status else {
					return Err(Error::<T>::InvalidCrowdsaleStatus.into());
				};

				// mint vouchers to user based on contribution; remove user from sale
				let contribution = SaleParticipation::<T>::take(sale_id, &who)
					.ok_or(Error::<T>::VouchersAlreadyClaimed)?;
				// TODO maybe set stale status to distributed here if it is the last claim

				// get amount of claimable vouchers based on the user's contribution
				let collection_info =
					T::NFTExt::get_collection_info(sale_info.reward_collection_id)?;
				let voucher_max_supply =
					collection_info.max_issuance.ok_or(Error::<T>::MaxIssuanceNotSet)?;

				// calculate the claimable vouchers
				let claimable_vouchers = Self::mint_user_vouchers(
					who.clone(),
					sale_id,
					sale_info,
					contribution.into(),
					voucher_max_supply.into(),
					voucher_current_supply,
					total_paid_contributions.into(),
				)
				.map_err(|_| {
					log!(
						error,
						"⛔️ failed to mint voucher rewards for user {:?} in sale: {:?}",
						who,
						sale_id,
					);
					Error::<T>::VoucherClaimFailed
				})?;

				let voucher_max_supply: Balance = (voucher_max_supply as u128)
					.saturating_mul(10u128.pow(VOUCHER_DECIMALS as u32));
				let block_number = <frame_system::Pallet<T>>::block_number();
				let voucher_current_supply =
					voucher_current_supply.saturating_add(claimable_vouchers);

				if voucher_current_supply >= voucher_max_supply {
					// Distribution complete
					sale_info.status = SaleStatus::Ended(block_number, voucher_max_supply);
					Self::deposit_event(Event::CrowdsaleDistributionComplete { sale_id });
				} else {
					// Update total_contributions
					sale_info.status = SaleStatus::Distributing(
						block_number,
						total_paid_contributions.saturating_add(contribution),
						voucher_current_supply,
					);
				}

				Ok(())
			})?;

			Ok(())
		}

		/// Redeem the vouchers for the NFTs in a crowdsale which has concluded.
		/// The vouchers are crowdsale specific and can be redeemed for NFTs from the collection.
		/// NFTs are minted to the user's account.
		/// NFTs can be redeemed during or after payment of all vouchers
		///
		/// Parameters:
		/// - `sale_id`: The id of the sale to redeem the voucher from
		/// - `quantity`: The amount of NFT(s) to redeem
		///
		/// Emits `CrowdsaleNFTRedeemed` event when successful.
		#[pallet::weight(0)]
		#[transactional]
		pub fn redeem_voucher(
			origin: OriginFor<T>,
			sale_id: SaleId,
			quantity: TokenCount,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			SaleInfo::<T>::try_mutate(sale_id, |sale_info| -> DispatchResult {
				let Some(sale_info) = sale_info else {
					return Err(Error::<T>::CrowdsaleNotFound.into());
				};

				// ensure the sale has concluded and is being distributed or has been distributed
				ensure!(
					matches!(sale_info.status, SaleStatus::Distributing(_, _, _)) ||
						matches!(sale_info.status, SaleStatus::Ended(_, _)),
					Error::<T>::InvalidCrowdsaleStatus
				);

				// burn vouchers from the user, will fail if the user does not have enough
				// vouchers since 1:1 mapping between vouchers and NFTs, we can use the quantity
				// * decimals as the amount burned
				let voucher_amount = quantity.saturating_mul(10u32.pow(VOUCHER_DECIMALS as u32));
				T::MultiCurrency::burn_from(sale_info.voucher, &who, voucher_amount.into())?;

				// mint the NFT(s) to the user
				T::NFTExt::do_mint(
					T::PalletId::get().into_account_truncating(),
					sale_info.reward_collection_id,
					quantity,
					Some(who.clone()),
				)?;

				Self::deposit_event(Event::CrowdsaleNFTRedeemed {
					sale_id,
					who,
					collection_id: sale_info.reward_collection_id,
					quantity,
				});

				Ok(())
			})?;

			Ok(())
		}

		/// In the very unlikely case that a sale was blocked from automatic distribution within
		/// the on_initialise step. This function allows a manual trigger of distribution
		/// callable by the admin of the sale
		#[pallet::weight(0)]
		pub fn claim_blocked_sale(origin: OriginFor<T>, sale_id: SaleId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			SaleInfo::<T>::try_mutate(sale_id, |sale_info| -> DispatchResult {
				let Some(sale_info) = sale_info else {
					return Err(Error::<T>::CrowdsaleNotFound.into());
				};

				// ensure the sale is in the correct state
				ensure!(
					matches!(sale_info.status, SaleStatus::DistributionFailed(_)),
					Error::<T>::InvalidCrowdsaleStatus
				);

				ensure!(sale_info.admin == who, Error::<T>::AccessDenied);

				DistributingSales::<T>::try_append(sale_id)
					.map_err(|_| Error::<T>::DistributingSaleFailed)?;

				let block_number = <frame_system::Pallet<T>>::block_number();
				if sale_info.funds_raised.is_zero() {
					sale_info.status = SaleStatus::Ended(block_number, Balance::default());
				} else {
					// Mark the sale for distribution
					sale_info.status = SaleStatus::Distributing(
						block_number,
						Balance::default(),
						Balance::default(),
					);
				}

				Self::deposit_event(Event::CrowdsaleClosed { sale_id, info: sale_info.clone() });

				Ok(())
			})?;

			Ok(())
		}
	}
}