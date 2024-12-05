// Copyright 2024-2025 Futureverse Corporation Limited
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

use alloc::boxed::Box;
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo},
	pallet_prelude::*,
	sp_runtime::{
		traits::{AccountIdConversion, Zero},
		SaturatedConversion, Saturating,
	},
	traits::{
		fungibles::{self, Inspect, Mutate},
		tokens::{Fortitude, Precision, Preservation},
		IsSubType,
	},
	transactional, PalletId,
};
use frame_system::{
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::*,
};
use seed_pallet_common::{log, CreateExt, ExtrinsicChecker, InspectExt, NFTExt};
use seed_primitives::{AssetId, Balance, CollectionUuid, OffchainErr, TokenCount};
use sp_std::{vec, vec::Vec};

pub mod types;
use types::*;

mod impls;
mod weights;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// The logging target for this pallet
#[allow(dead_code)]
pub(crate) const LOG_TARGET: &str = "crowdsale";

pub const CROWDSALE_DIST_UNSIGNED_PRIORITY: TransactionPriority = TransactionPriority::MAX / 2;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + SendTransactionTypes<Call<Self>> {
		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The overarching call type.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>
			+ IsSubType<Call<Self>>
			+ IsType<<Self as frame_system::Config>::RuntimeCall>;

		/// This pallet's Id, used for deriving a sovereign account ID
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The maximum length of a intermediary sale voucher asset name and symbol
		#[pallet::constant]
		type StringLimit: Get<u32>;

		/// Futurepass proxy extrinsic inner call blacklist validator
		type ProxyCallValidator: ExtrinsicChecker<
			Call = <Self as pallet::Config>::RuntimeCall,
			Extra = (),
			Result = DispatchResult,
		>;

		/// Currency implementation to deal with assets.
		type MultiCurrency: InspectExt
			+ CreateExt<AccountId = Self::AccountId>
			+ fungibles::Inspect<Self::AccountId, AssetId = AssetId>
			+ fungibles::metadata::Inspect<Self::AccountId>
			+ fungibles::Mutate<Self::AccountId, Balance = Balance>;

		/// NFT Extension, used to retrieve collection data
		type NFTExt: NFTExt<AccountId = Self::AccountId>;

		/// The maximum number of sales that can be queued for completion in a single block
		#[pallet::constant]
		type MaxSalesPerBlock: Get<u32>;

		/// The maximum number of sales that can be active at one time
		#[pallet::constant]
		type MaxConsecutiveSales: Get<u32>;

		/// The maximum number of payments that can be processed in the offchain worker per block
		#[pallet::constant]
		type MaxPaymentsPerBlock: Get<u32>;

		/// The maximum duration of a sale
		#[pallet::constant]
		type MaxSaleDuration: Get<BlockNumberFor<Self>>;

		/// Unsigned transaction interval
		#[pallet::constant]
		type UnsignedInterval: Get<BlockNumberFor<Self>>;

		/// Interface to access weight values
		type WeightInfo: WeightInfo;
	}

	/// The next available sale id
	#[pallet::storage]
	pub type NextSaleId<T: Config> = StorageValue<_, SaleId, ValueQuery>;

	/// Map from sale id to its information
	#[pallet::storage]
	pub type SaleInfo<T: Config> =
		StorageMap<_, Twox64Concat, SaleId, SaleInformation<T::AccountId, BlockNumberFor<T>>>;

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
		BlockNumberFor<T>,
		BoundedVec<SaleId, T::MaxSalesPerBlock>,
		OptionQuery,
	>;

	/// A list of all sales currently being distributed
	#[pallet::storage]
	pub type SaleDistribution<T: Config> =
		StorageValue<_, BoundedVec<SaleId, T::MaxConsecutiveSales>, ValueQuery>;

	/// Stores next unsigned tx block number
	#[pallet::storage]
	pub(super) type NextUnsignedAt<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Crowdsale created
		CrowdsaleCreated { sale_id: SaleId, info: SaleInformation<T::AccountId, BlockNumberFor<T>> },
		/// Call proxied
		VaultCallProxied {
			sale_id: SaleId,
			who: T::AccountId,
			vault: T::AccountId,
			call: Box<<T as Config>::RuntimeCall>,
		},
		/// Crowdsale enabled
		CrowdsaleEnabled {
			sale_id: SaleId,
			info: SaleInformation<T::AccountId, BlockNumberFor<T>>,
			end_block: BlockNumberFor<T>,
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
		CrowdsaleClosed { sale_id: SaleId, info: SaleInformation<T::AccountId, BlockNumberFor<T>> },
		/// Crowdsale distribution was manually triggered
		CrowdsaleManualDistribution {
			sale_id: SaleId,
			info: SaleInformation<T::AccountId, BlockNumberFor<T>>,
			who: T::AccountId,
		},
		/// Crowdsale vouchers claimed
		CrowdsaleVouchersClaimed { sale_id: SaleId, who: T::AccountId, amount: Balance },
		/// Crowdsale distribution has been completed and all vouchers paid out
		CrowdsaleDistributionComplete { sale_id: SaleId, vouchers_distributed: Balance },
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
		/// The soft cap price must be greater than zero
		InvalidSoftCapPrice,
		/// Invalid asset id
		InvalidAsset,
		/// The collection max issuance is too high
		InvalidMaxIssuance,
		/// The amount must not be zero
		InvalidAmount,
		/// Redemption quantity must not be zero
		InvalidQuantity,
		/// The voucher claim could not be completed due to invalid voucher supply
		VoucherClaimFailed,
		/// The NFT collection max issuance is not set
		MaxIssuanceNotSet,
		/// The NFT collection must not contain any minted NFTs
		CollectionIssuanceNotZero,
		/// The NFT collection must not be mintable
		CollectionPublicMintable,
		/// There are too many sales queued for this block, try again on a different block
		TooManySales,
		/// Vouchers have already been claimed
		VouchersAlreadyClaimed,
		/// Automatic trigger of sales distribution has failed
		SaleDistributionFailed,
		/// The sale duration is too long
		SaleDurationTooLong,
		/// Extrinsic not allowed
		ExtrinsicForbidden,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Check and close all expired listings
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			let total_closed: u32 = match Self::close_sales_at(now) {
				Ok(total_closed) => total_closed,
				Err(e) => {
					log!(error, "⛔️ failed to close sales at block {:?}: {:?}", now, e);
					0u32
				},
			};
			// Record weight for closing sales
			if total_closed > 0 {
				log!(debug, "✅ closed {} sales at block {:?}", total_closed, now);
				T::WeightInfo::on_initialize(total_closed)
			} else {
				T::WeightInfo::on_initialize_empty()
			}
		}

		/// Offchain worker processes closed sales to distribute voucher rewards to participants
		fn offchain_worker(now: BlockNumberFor<T>) {
			if !sp_io::offchain::is_validator() {
				log!(
					error,
					"⛔️ offchain worker error at block [{:?}]: {:?}",
					now,
					OffchainErr::NotAValidator
				);
			}

			if <NextUnsignedAt<T>>::get() > now {
				return;
			}
			if !SaleDistribution::<T>::get().is_empty() {
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
					if SaleDistribution::<T>::get().is_empty() {
						return InvalidTransaction::Stale.into();
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
		/// - `payment_asset_id`: The asset_id used for participating in the crowdsale
		/// - `collection_id`: Collection id of the NFTs that will be minted/redeemed to the
		///   participants
		/// - `soft_cap_price`: Number/Ratio of payment_asset tokens that will be required to
		///   purchase vouchers; Note: this does not take into account asset decimals or voucher
		///   decimals
		/// - `sale_duration`: How many blocks will the sale last once enabled
		/// - `voucher_name`: [optional] name for the created voucher asset
		/// - `voucher_symbol`: [optional] symbol for the created voucher asset
		///
		/// Emits `CrowdsaleCreated` event when successful.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::initialize())]
		#[transactional]
		pub fn initialize(
			origin: OriginFor<T>,
			payment_asset_id: AssetId,
			collection_id: CollectionUuid,
			soft_cap_price: Balance,
			sale_duration: BlockNumberFor<T>,
			voucher_name: Option<BoundedVec<u8, T::StringLimit>>,
			voucher_symbol: Option<BoundedVec<u8, T::StringLimit>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// increment the sale id, store it and use it
			let sale_id = NextSaleId::<T>::mutate(|id| -> Result<u64, DispatchError> {
				let current_id = *id;
				*id = id.checked_add(1).ok_or(Error::<T>::NoAvailableIds)?;
				Ok(current_id)
			})?;

			// ensure the asset exists
			if !T::MultiCurrency::exists(payment_asset_id) {
				return Err(Error::<T>::InvalidAsset.into());
			}

			// ensure soft_cap_price is not zero - prevent future div by zero
			ensure!(!soft_cap_price.is_zero(), Error::<T>::InvalidSoftCapPrice);
			// Disallow sale durations that are too long
			ensure!(sale_duration <= T::MaxSaleDuration::get(), Error::<T>::SaleDurationTooLong);

			// create crowdsale vault account which will temporary manage ownership and hold funds
			let vault = Self::vault_account(sale_id);

			// Verify collection max and total issuance
			let (collection_issuance, max_issuance) =
				T::NFTExt::get_collection_issuance(collection_id)?;
			let max_issuance = max_issuance.ok_or(Error::<T>::MaxIssuanceNotSet)?;
			ensure!(collection_issuance.is_zero(), Error::<T>::CollectionIssuanceNotZero);

			// Verify collection public mint is disabled
			if let Ok(mint_info) = T::NFTExt::get_public_mint_info(collection_id) {
				ensure!(!mint_info.enabled, Error::<T>::CollectionPublicMintable);
			}

			// Transfer ownership of the collection to the vault account. This also ensures
			// the caller is the owner of the collection
			// - this is required so collection owner cannot mint/rug to dilute the crowdsale
			T::NFTExt::transfer_collection_ownership(who.clone(), collection_id, vault.clone())?;

			// create voucher asset
			let voucher_asset_id = Self::create_voucher_asset(
				&vault,
				sale_id,
				max_issuance,
				voucher_name.map(|v| v.into()),
				voucher_symbol.map(|v| v.into()),
			)?;

			// store the sale information
			let sale_info = SaleInformation::<T::AccountId, BlockNumberFor<T>> {
				status: SaleStatus::Pending(<frame_system::Pallet<T>>::block_number()),
				admin: who.clone(),
				vault,
				payment_asset_id,
				reward_collection_id: collection_id,
				soft_cap_price,
				funds_raised: 0,
				participant_count: 0,
				voucher_asset_id,
				duration: sale_duration,
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
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::enable())]
		#[transactional]
		pub fn enable(origin: OriginFor<T>, sale_id: SaleId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// update the sale status if the start block is met
			SaleInfo::<T>::try_mutate(sale_id, |sale_info| -> DispatchResult {
				let sale_info = sale_info.as_mut().ok_or(Error::<T>::CrowdsaleNotFound)?;

				// ensure the sale is not already enabled
				ensure!(
					matches!(sale_info.status, SaleStatus::Pending(_)),
					Error::<T>::InvalidCrowdsaleStatus
				);
				ensure!(sale_info.admin == who, Error::<T>::AccessDenied);

				// ensure start block is met and end block is not met
				let current_block = <frame_system::Pallet<T>>::block_number();
				let end_block = sale_info.duration.saturating_add(current_block);

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
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::participate())]
		#[transactional]
		pub fn participate(
			origin: OriginFor<T>,
			sale_id: SaleId,
			amount: Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(amount > 0, Error::<T>::InvalidAmount);

			// update the sale status if the start block is met
			SaleInfo::<T>::try_mutate(sale_id, |sale_info| -> DispatchResult {
				let sale_info = sale_info.as_mut().ok_or(Error::<T>::CrowdsaleNotFound)?;

				// ensure the sale is enabled
				ensure!(
					matches!(sale_info.status, SaleStatus::Enabled(_)),
					Error::<T>::CrowdsaleNotEnabled
				);

				// transfer payment tokens to the crowdsale vault
				T::MultiCurrency::transfer(
					sale_info.payment_asset_id,
					&who,
					&sale_info.vault,
					amount,
					Preservation::Expendable,
				)?;

				// update the sale funds
				sale_info.funds_raised = sale_info.funds_raised.saturating_add(amount);

				// update the user's contribution
				SaleParticipation::<T>::mutate(sale_id, who.clone(), |maybe_contribute| {
					match maybe_contribute {
						Some(contribution) => *contribution = contribution.saturating_add(amount),
						None => {
							sale_info.participant_count =
								sale_info.participant_count.saturating_add(1);
							*maybe_contribute = Some(amount)
						},
					}
				});

				Self::deposit_event(Event::CrowdsaleParticipated {
					sale_id,
					who,
					asset: sale_info.payment_asset_id,
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
		// TODO: update weight based on participants processable
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::distribute_crowdsale_rewards())]
		#[transactional]
		pub fn distribute_crowdsale_rewards(origin: OriginFor<T>) -> DispatchResult {
			ensure_none(origin)?;

			let mut sale_ids: Vec<SaleId> = SaleDistribution::<T>::get().into_inner();

			// Get the first sale_id and process in FiFo order
			let sale_id = *sale_ids.first().ok_or(Error::<T>::CrowdsaleNotFound)?;
			let mut sale_info = SaleInfo::<T>::get(sale_id).ok_or(Error::<T>::CrowdsaleNotFound)?;

			// ensure the sale is in the distribution phase
			let SaleStatus::Distributing(_, mut vouchers_distributed) = sale_info.status else {
				return Err(Error::<T>::InvalidCrowdsaleStatus.into());
			};

			let voucher_max_supply =
				T::NFTExt::get_collection_issuance(sale_info.reward_collection_id)?
					.1
					.ok_or(Error::<T>::MaxIssuanceNotSet)?;

			let mut contributions_iterator = SaleParticipation::<T>::drain_prefix(sale_id);
			let mut payout_complete: bool = false;

			for _ in 0..T::MaxPaymentsPerBlock::get() {
				// End early if we have no more contributions to payout
				let Some((who, contribution)) = contributions_iterator.next() else {
					payout_complete = true;
					break;
				};

				let Ok(claimable_vouchers) = Self::transfer_user_vouchers(
					who.clone(),
					&sale_info,
					contribution,
					voucher_max_supply.into(),
				) else {
					log!(
						error,
						"⛔️ failed to mint voucher rewards for user {:?} in sale {:?}",
						who,
						sale_id,
					);
					continue;
				};

				Self::deposit_event(Event::CrowdsaleVouchersClaimed {
					sale_id,
					who,
					amount: claimable_vouchers,
				});

				vouchers_distributed = vouchers_distributed.saturating_add(claimable_vouchers);
			}

			let block_number = <frame_system::Pallet<T>>::block_number();
			if payout_complete || SaleParticipation::<T>::iter_prefix(sale_id).next().is_none() {
				// Distribution complete
				// Refund admin any remaining vouchers in the vault account
				let vault_balance = T::MultiCurrency::reducible_balance(
					sale_info.voucher_asset_id,
					&sale_info.vault,
					Preservation::Expendable,
					Fortitude::Polite,
				);
				if vault_balance > 0 {
					let _ = T::MultiCurrency::transfer(
						sale_info.voucher_asset_id,
						&sale_info.vault,
						&sale_info.admin,
						vault_balance,
						Preservation::Expendable,
					);
				}
				sale_info.status = SaleStatus::Ended(block_number);
				Self::deposit_event(Event::CrowdsaleDistributionComplete {
					sale_id,
					vouchers_distributed,
				});
				sale_ids = sale_ids.drain(1..).collect();
				SaleDistribution::<T>::put(BoundedVec::truncate_from(sale_ids));
			} else {
				// Update total_contributions
				sale_info.status = SaleStatus::Distributing(block_number, vouchers_distributed);
			}

			let next_unsigned_at = block_number + T::UnsignedInterval::get();
			<NextUnsignedAt<T>>::put(next_unsigned_at);
			SaleInfo::<T>::insert(sale_id, sale_info);
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
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::claim_voucher())]
		#[transactional]
		pub fn claim_voucher(origin: OriginFor<T>, sale_id: SaleId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			SaleInfo::<T>::try_mutate(sale_id, |sale_info| -> DispatchResult {
				let sale_info = sale_info.as_mut().ok_or(Error::<T>::CrowdsaleNotFound)?;

				// ensure the sale is in the distribution phase
				let SaleStatus::Distributing(_, vouchers_distributed) = sale_info.status else {
					return Err(Error::<T>::InvalidCrowdsaleStatus.into());
				};

				// mint vouchers to user based on contribution; remove user from sale
				let contribution = SaleParticipation::<T>::take(sale_id, &who)
					.ok_or(Error::<T>::VouchersAlreadyClaimed)?;

				// get amount of claimable vouchers based on the user's contribution
				let voucher_max_supply =
					T::NFTExt::get_collection_issuance(sale_info.reward_collection_id)?
						.1
						.ok_or(Error::<T>::MaxIssuanceNotSet)?;

				// calculate the claimable vouchers
				let claimable_vouchers = Self::transfer_user_vouchers(
					who.clone(),
					sale_info,
					contribution,
					voucher_max_supply.into(),
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

				let block_number = <frame_system::Pallet<T>>::block_number();
				let vouchers_distributed = vouchers_distributed.saturating_add(claimable_vouchers);
				// Check if we have any more payments to make
				if SaleParticipation::<T>::iter_prefix(sale_id).next().is_none() {
					// Distribution complete
					// Refund admin any remaining vouchers in the vault account
					let vault_balance = T::MultiCurrency::reducible_balance(
						sale_info.voucher_asset_id,
						&sale_info.vault,
						Preservation::Expendable,
						Fortitude::Polite,
					);
					if vault_balance > 0 {
						let _ = T::MultiCurrency::transfer(
							sale_info.voucher_asset_id,
							&sale_info.vault,
							&sale_info.admin,
							vault_balance,
							Preservation::Expendable,
						);
					}
					sale_info.status = SaleStatus::Ended(block_number);
					Self::deposit_event(Event::CrowdsaleDistributionComplete {
						sale_id,
						vouchers_distributed,
					});
					// Clear SaleDistribution storage vec
					let _ = SaleDistribution::<T>::try_mutate(|sales| -> DispatchResult {
						sales.retain(|&id| id != sale_id);
						Ok(())
					});
				} else {
					// Update total_contributions
					sale_info.status = SaleStatus::Distributing(block_number, vouchers_distributed);
				}

				Self::deposit_event(Event::CrowdsaleVouchersClaimed {
					sale_id,
					who,
					amount: claimable_vouchers,
				});

				Ok(())
			})?;

			Ok(())
		}

		/// Redeem the vouchers for the NFTs in a crowdsale which has concluded.
		/// The vouchers are crowdsale specific and can be redeemed for NFTs from the collection.
		/// NFTs are minted to the user's account.
		/// NFTs can be redeemed during or after payment of all vouchers.
		///
		/// Parameters:
		/// - `sale_id`: The id of the sale to redeem the voucher from
		/// - `quantity`: The amount of NFT(s) to redeem
		///
		/// Emits `CrowdsaleNFTRedeemed` event when successful.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::redeem_voucher())]
		#[transactional]
		pub fn redeem_voucher(
			origin: OriginFor<T>,
			sale_id: SaleId,
			quantity: TokenCount,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(quantity > 0, Error::<T>::InvalidQuantity);

			let sale_info = SaleInfo::<T>::get(sale_id).ok_or(Error::<T>::CrowdsaleNotFound)?;

			// ensure the sale has concluded and is being distributed or has been distributed
			ensure!(
				matches!(sale_info.status, SaleStatus::Distributing(_, _))
					|| matches!(sale_info.status, SaleStatus::Ended(_)),
				Error::<T>::InvalidCrowdsaleStatus
			);

			// burn vouchers from the user, will fail if the user does not have enough
			// vouchers since 1:1 mapping between vouchers and NFTs, we can use the quantity
			// * decimals as the amount burned
			let voucher_amount = quantity.saturating_mul(10u32.pow(VOUCHER_DECIMALS as u32));
			T::MultiCurrency::burn_from(
				sale_info.voucher_asset_id,
				&who,
				voucher_amount.into(),
				Precision::Exact,
				Fortitude::Polite,
			)?;

			// mint the NFT(s) to the user
			T::NFTExt::do_mint(
				sale_info.vault.clone(),
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
		}

		/// Caller (crowdsale admin) proxies the `call` to the sale vault account to manage the
		/// assets and NFTs owned by the vault.
		/// Note: Only the asset and nft metadata can be modified by the proxied account.
		///
		/// Parameters:
		/// - `sale_id`: The id of the sale to proxy the call to
		/// - `call`: The call to be proxied
		///
		/// Emits `VaultCallProxied` event when successful.
		#[pallet::call_index(6)]
		#[pallet::weight({
			let call_weight = call.get_dispatch_info().weight;
			T::WeightInfo::proxy_vault_call().saturating_add(call_weight)
		})]
		#[transactional]
		pub fn proxy_vault_call(
			origin: OriginFor<T>,
			sale_id: SaleId,
			call: Box<<T as Config>::RuntimeCall>,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;
			let sale_info = SaleInfo::<T>::get(sale_id).ok_or(Error::<T>::CrowdsaleNotFound)?;

			// ensure the caller is the sale admin
			ensure!(sale_info.admin == who, Error::<T>::AccessDenied);

			// disallow invalid extrinsics
			<T as pallet::Config>::ProxyCallValidator::check_extrinsic(&call, &())?;

			// proxy the call through the vault account
			let vault_origin = frame_system::RawOrigin::Signed(sale_info.vault.clone());
			call.clone().dispatch(vault_origin.into()).map_err(|e| e.error)?;

			Self::deposit_event(Event::VaultCallProxied {
				sale_id,
				who,
				vault: sale_info.vault,
				call,
			});

			Ok(())
		}

		/// In the very unlikely case that a sale was blocked from automatic distribution within
		/// the on_initialise step. This function allows a manual trigger of distribution
		/// callable by anyone to kickstart the sale distribution process.
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::try_force_distribution())]
		pub fn try_force_distribution(origin: OriginFor<T>, sale_id: SaleId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			SaleInfo::<T>::try_mutate(sale_id, |sale_info| -> DispatchResult {
				let sale_info = sale_info.as_mut().ok_or(Error::<T>::CrowdsaleNotFound)?;

				// ensure the sale is in the correct state
				ensure!(
					matches!(sale_info.status, SaleStatus::DistributionFailed(_)),
					Error::<T>::InvalidCrowdsaleStatus
				);

				SaleDistribution::<T>::try_append(sale_id)
					.map_err(|_| Error::<T>::SaleDistributionFailed)?;

				let block_number = <frame_system::Pallet<T>>::block_number();
				if sale_info.funds_raised.is_zero() {
					sale_info.status = SaleStatus::Ended(block_number);
				} else {
					// Mark the sale for distribution
					sale_info.status = SaleStatus::Distributing(block_number, Balance::default());
				}

				Self::deposit_event(Event::CrowdsaleManualDistribution {
					sale_id,
					info: sale_info.clone(),
					who,
				});

				Ok(())
			})?;

			Ok(())
		}
	}
}
