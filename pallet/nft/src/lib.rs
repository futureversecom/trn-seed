/* Copyright 2019-2021 Centrality Investments Limited
 *
 * Licensed under the LGPL, Version 3.0 (the "License");
 * you may not use this file except in compliance with the License.
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * You may obtain a copy of the License at the root of this project source code,
 * or at:
 *     https://centrality.ai/licenses/gplv3.txt
 *     https://centrality.ai/licenses/lgplv3.txt
 */
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]
//! # NFT Module
//!
//! Provides the basic creation and management of dynamic NFTs (created at runtime).
//!
//! Intended to be used "as is" by dapps and provide basic NFT feature set for smart contracts
//! to extend.
//!
//! *Collection*:
//! Collection are a grouping of tokens- equivalent to an ERC721 contract
//!
//! *Tokens*:
//!  Individual tokens within a collection. Globally identifiable by a tuple of (collection, serial
//! number)

use frame_support::{
	ensure, traits::Get, transactional, weights::constants::RocksDbWeight as DbWeight, PalletId,
};
use seed_pallet_common::{log, Hold, OnTransferSubscriber, TransferExt};
use seed_primitives::{AssetId, Balance, CollectionUuid, ParachainId, SerialNumber, TokenId};
use sp_runtime::{
	traits::{One, Saturating, Zero},
	DispatchResult, PerThing, Permill,
};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod weights;
use weights::WeightInfo;

mod impls;
mod migration;
mod types;

pub use impls::*;
pub use pallet::*;
pub use types::*;

/// The maximum length of valid collection IDs
pub const MAX_COLLECTION_NAME_LENGTH: u8 = 32;
/// The maximum amount of listings to return
pub const MAX_COLLECTION_LISTING_LIMIT: u16 = 100;
/// The logging target for this module
pub(crate) const LOG_TARGET: &str = "nft";

#[frame_support::pallet]
pub mod pallet {
	use super::{DispatchResult, *};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::AccountIdConversion;
	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		_phantom: sp_std::marker::PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { _phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			NextCollectionId::<T>::put(1_u32);
			NextMarketplaceId::<T>::put(1 as MarketplaceId);
			NextListingId::<T>::put(1 as ListingId);
			NextOfferId::<T>::put(1 as OfferId);
			StorageVersion::<T>::put(Releases::V1);
		}
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Default auction / sale length in blocks
		#[pallet::constant]
		type DefaultListingDuration: Get<Self::BlockNumber>;
		/// The system event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Handles a multi-currency fungible asset system
		type MultiCurrency: TransferExt<AccountId = Self::AccountId>
			+ Hold<AccountId = Self::AccountId>;
		/// Handler for when an NFT has been transferred
		type OnTransferSubscription: OnTransferSubscriber;
		/// This pallet's Id, used for deriving a sovereign account ID
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// The parachain_id being used by this parachain
		type ParachainId: Get<ParachainId>;
		/// Provides the public call to weight mapping
		type WeightInfo: WeightInfo;
	}

	/// Map from collection to its information
	#[pallet::storage]
	#[pallet::getter(fn collection_info)]
	pub type CollectionInfo<T: Config> =
		StorageMap<_, Twox64Concat, CollectionUuid, CollectionInformation<T::AccountId>>;

	/// Map from a collection to its total issuance
	#[pallet::storage]
	#[pallet::getter(fn collection_issuance)]
	pub type CollectionIssuance<T> = StorageMap<_, Twox64Concat, CollectionUuid, TokenCount>;

	/// The next available incrementing collection id
	#[pallet::storage]
	pub type NextCollectionId<T> = StorageValue<_, u32, ValueQuery>;

	/// The next available serial number in a given collection
	#[pallet::storage]
	#[pallet::getter(fn next_serial_number)]
	pub type NextSerialNumber<T> = StorageMap<_, Twox64Concat, CollectionUuid, SerialNumber>;

	/// Map from a token to lock status if any
	#[pallet::storage]
	#[pallet::getter(fn token_locks)]
	pub type TokenLocks<T> = StorageMap<_, Twox64Concat, TokenId, TokenLockReason>;

	/// Map from a token to its owner
	#[pallet::storage]
	#[pallet::getter(fn token_owner)]
	pub type TokenOwner<T: Config> =
		StorageDoubleMap<_, Twox64Concat, CollectionUuid, Twox64Concat, SerialNumber, T::AccountId>;

	/// Count of tokens owned by an address, supports ERC721 `balanceOf`
	#[pallet::storage]
	#[pallet::getter(fn token_balance)]
	pub type TokenBalance<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BTreeMap<CollectionUuid, TokenCount>>;

	/// The next available marketplace id
	#[pallet::storage]
	#[pallet::getter(fn next_marketplace_id)]
	pub type NextMarketplaceId<T> = StorageValue<_, MarketplaceId, ValueQuery>;

	/// Map from marketplace account_id to royalties schedule
	#[pallet::storage]
	#[pallet::getter(fn registered_marketplaces)]
	pub type RegisteredMarketplaces<T: Config> =
		StorageMap<_, Twox64Concat, MarketplaceId, Marketplace<T::AccountId>>;

	/// NFT sale/auction listings keyed by listing id
	#[pallet::storage]
	#[pallet::getter(fn listings)]
	pub type Listings<T: Config> = StorageMap<_, Twox64Concat, ListingId, Listing<T>>;

	/// The next available listing Id
	#[pallet::storage]
	#[pallet::getter(fn next_listing_id)]
	pub type NextListingId<T> = StorageValue<_, ListingId, ValueQuery>;

	/// Map from collection to any open listings
	#[pallet::storage]
	#[pallet::getter(fn open_collection_listings)]
	pub type OpenCollectionListings<T> =
		StorageDoubleMap<_, Twox64Concat, CollectionUuid, Twox64Concat, ListingId, bool>;

	/// Winning bids on open listings.
	#[pallet::storage]
	#[pallet::getter(fn listing_winning_bid)]
	pub type ListingWinningBid<T: Config> =
		StorageMap<_, Twox64Concat, ListingId, (T::AccountId, Balance)>;

	/// Block numbers where listings will close. Value is `true` if at block number `listing_id` is
	/// scheduled to close.
	#[pallet::storage]
	#[pallet::getter(fn listing_end_schedule)]
	pub type ListingEndSchedule<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::BlockNumber, Twox64Concat, ListingId, bool>;

	/// Map from offer_id to the information related to the offer
	#[pallet::storage]
	#[pallet::getter(fn offers)]
	pub type Offers<T: Config> = StorageMap<_, Twox64Concat, OfferId, OfferType<T::AccountId>>;

	/// Maps from token_id to a vector of offer_ids on that token
	#[pallet::storage]
	#[pallet::getter(fn token_offers)]
	pub type TokenOffers<T> = StorageMap<_, Twox64Concat, TokenId, Vec<OfferId>>;

	/// The next available offer_id
	#[pallet::storage]
	#[pallet::getter(fn next_offer_id)]
	pub type NextOfferId<T> = StorageValue<_, OfferId, ValueQuery>;

	/// Version of this module's storage schema
	#[pallet::storage]
	pub type StorageVersion<T> = StorageValue<_, Releases, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new collection of tokens was created
		CollectionCreate {
			collection_uuid: CollectionUuid,
			token_count: TokenCount,
			owner: T::AccountId,
		},
		/// Token(s) were minted
		Mint {
			collection_id: CollectionUuid,
			first_serial_number: SerialNumber,
			token_count: TokenCount,
			owner: T::AccountId,
		},
		/// A new owner was set
		OwnerSet { collection_id: CollectionUuid, new_owner: T::AccountId },
		/// A token was transferred
		Transfer {
			previous_owner: T::AccountId,
			collection_id: CollectionUuid,
			serial_number: SerialNumber,
			new_owner: T::AccountId,
		},
		/// A token was burned
		Burn { collection_id: CollectionUuid, serial_number: SerialNumber },
		/// A fixed price sale has been listed
		FixedPriceSaleList {
			tokens: Vec<TokenId>,
			listing_id: ListingId,
			marketplace_id: Option<MarketplaceId>,
			price: Balance,
			payment_asset: AssetId,
			seller: T::AccountId,
		},
		/// A fixed price sale has completed
		FixedPriceSaleComplete {
			tokens: Vec<TokenId>,
			listing_id: ListingId,
			price: Balance,
			payment_asset: AssetId,
			buyer: T::AccountId,
			seller: T::AccountId,
		},
		/// A fixed price sale has closed without selling
		FixedPriceSaleClose {
			tokens: Vec<TokenId>,
			listing_id: ListingId,
			reason: FixedPriceClosureReason,
		},
		/// A fixed price sale has had its price updated
		FixedPriceSalePriceUpdate {
			tokens: Vec<TokenId>,
			listing_id: ListingId,
			new_price: Balance,
		},
		/// An auction has opened
		AuctionOpen {
			tokens: Vec<TokenId>,
			payment_asset: AssetId,
			reserve_price: Balance,
			listing_id: ListingId,
			marketplace_id: Option<MarketplaceId>,
			seller: T::AccountId,
		},
		/// An auction has sold
		AuctionSold {
			collection_id: CollectionUuid,
			listing_id: ListingId,
			payment_asset: AssetId,
			hammer_price: Balance,
			winner: T::AccountId,
		},
		/// An auction has closed without selling
		AuctionClose {
			collection_id: CollectionUuid,
			listing_id: ListingId,
			reason: AuctionClosureReason,
		},
		/// A new highest bid was placed
		Bid { tokens: Vec<TokenId>, listing_id: ListingId, amount: Balance, bidder: T::AccountId },
		/// An account has been registered as a marketplace
		MarketplaceRegister {
			account: T::AccountId,
			entitlement: Permill,
			marketplace_id: MarketplaceId,
		},
		/// An offer has been made on an NFT
		Offer {
			offer_id: OfferId,
			amount: Balance,
			asset_id: AssetId,
			marketplace_id: Option<MarketplaceId>,
			buyer: T::AccountId,
		},
		/// An offer has been cancelled
		OfferCancel { offer_id: OfferId, token_id: TokenId },
		/// An offer has been cancelled
		OfferAccept { offer_id: OfferId, token_id: TokenId, amount: Balance, asset_id: AssetId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Given collection name is invalid (invalid utf-8, too long, empty)
		CollectionNameInvalid,
		/// No more Ids are available, they've been exhausted
		NoAvailableIds,
		/// origin does not have permission for the operation (the token may not exist)
		NoPermission,
		/// The token does not exist
		NoToken,
		/// The token is not listed for fixed price sale
		NotForFixedPriceSale,
		/// The token is not listed for auction sale
		NotForAuction,
		/// Cannot operate on a listed NFT
		TokenLocked,
		/// Internal error during payment
		InternalPayment,
		/// Total royalties would exceed 100% of sale or an empty vec is supplied
		RoyaltiesInvalid,
		/// Auction bid was lower than reserve or current highest bid
		BidTooLow,
		/// Selling tokens from different collection is not allowed
		MixedBundleSale,
		/// The account_id hasn't been registered as a marketplace
		MarketplaceNotRegistered,
		/// The collection does not exist
		NoCollection,
		/// The metadata path is invalid (non-utf8 or empty)
		InvalidMetadataPath,
		/// No offer exists for the given OfferId
		InvalidOffer,
		/// The caller is not the buyer
		NotBuyer,
		/// The caller owns the token and can't make an offer
		IsTokenOwner,
		/// Offer amount needs to be greater than 0
		ZeroOffer,
		/// Cannot make an offer on a token up for auction
		TokenOnAuction,
		/// Max issuance needs to be greater than 0 and initial_issuance
		InvalidMaxIssuance,
		/// The collection max issuance has been reached and no more tokens can be minted
		MaxIssuanceReached,
		/// Attemped to mint a token that was bridged from a different chain
		AttemptedMintOnBridgedToken,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Check and close all expired listings
		fn on_initialize(now: T::BlockNumber) -> Weight {
			// TODO: this is unbounded and could become costly
			// https://github.com/cennznet/cennznet/issues/444
			let removed_count = Self::close_listings_at(now);
			// 'buy' weight is comparable to successful closure of an auction
			T::WeightInfo::buy() * removed_count as Weight
		}

		fn on_runtime_upgrade() -> Weight {
			use frame_support::IterableStorageMap;
			use migration::v1_storage;

			if <StorageVersion<T>>::get() == Releases::V0 {
				<StorageVersion<T>>::put(Releases::V1);

				let old_collection_info: Vec<(
					CollectionUuid,
					v1_storage::CollectionInformation<T::AccountId>,
				)> = v1_storage::CollectionInfo::<T>::iter().collect();

				let weight = old_collection_info.len() as Weight;
				for (collection_id, info) in old_collection_info {
					let collection_info_migrated = types::CollectionInformation {
						owner: info.owner,
						name: info.name,
						metadata_scheme: info.metadata_scheme,
						royalties_schedule: info.royalties_schedule,
						max_issuance: info.max_issuance,
						source_chain: OriginChain::Root,
					};

					<CollectionInfo<T>>::insert(collection_id, collection_info_migrated);
				}

				log!(warn, "🃏 collection info migrated");
				return 6_000_000 as Weight +
					DbWeight::get().reads_writes(weight as Weight + 1, weight as Weight + 1)
			} else {
				Zero::zero()
			}
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn do_create_collection(
			owner: T::AccountId,
			name: CollectionNameType,
			initial_issuance: TokenCount,
			max_issuance: Option<TokenCount>,
			token_owner: Option<T::AccountId>,
			metadata_scheme: MetadataScheme,
			royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
			source_chain: OriginChain,
		) -> Result<u32, DispatchError> {
			// Check we can issue the new tokens
			let collection_uuid = Self::next_collection_uuid()?;

			// Check max issuance is valid
			if let Some(max_issuance) = max_issuance {
				ensure!(max_issuance > Zero::zero(), Error::<T>::InvalidMaxIssuance);
				ensure!(initial_issuance <= max_issuance, Error::<T>::InvalidMaxIssuance);
			}

			// Validate collection attributes
			ensure!(
				!name.is_empty() && name.len() <= MAX_COLLECTION_NAME_LENGTH as usize,
				Error::<T>::CollectionNameInvalid
			);
			ensure!(core::str::from_utf8(&name).is_ok(), Error::<T>::CollectionNameInvalid);
			let metadata_scheme =
				metadata_scheme.sanitize().map_err(|_| Error::<T>::InvalidMetadataPath)?;
			if let Some(royalties_schedule) = royalties_schedule.clone() {
				ensure!(royalties_schedule.validate(), Error::<T>::RoyaltiesInvalid);
			}

			<CollectionInfo<T>>::insert(
				collection_uuid,
				CollectionInformation {
					owner: owner.clone(),
					name,
					metadata_scheme,
					royalties_schedule: None,
					max_issuance: None,
					source_chain,
				},
			);

			// Now mint the collection tokens
			let token_owner = token_owner.unwrap_or(owner);
			if initial_issuance > Zero::zero() {
				let token_ids: Vec<SerialNumber> = (0..initial_issuance).collect();
				Self::do_mint(&token_owner, collection_uuid, token_ids, source_chain)?;
			}
			// will not overflow, asserted prior qed.
			<NextCollectionId<T>>::mutate(|i| *i += u32::one());

			Self::deposit_event(Event::<T>::CollectionCreate {
				collection_uuid,
				token_count: initial_issuance,
				owner: token_owner,
			});
			Ok(collection_uuid)
		}

		pub fn do_burn(
			who: &T::AccountId,
			collection_id: CollectionUuid,
			serial_number: &SerialNumber,
		) -> DispatchResult {
			ensure!(
				!<TokenLocks<T>>::contains_key((collection_id, serial_number)),
				Error::<T>::TokenLocked
			);
			ensure!(
				Self::token_owner(collection_id, serial_number) == Some(who.clone()),
				Error::<T>::NoPermission
			);
			<TokenOwner<T>>::remove(collection_id, serial_number);

			let _ = <TokenBalance<T>>::try_mutate::<_, (), Error<T>, _>(who, |mut balances| {
				match &mut balances {
					Some(balances) => {
						match (balances).get_mut(&collection_id) {
							Some(balance) => {
								let new_balance = balance.saturating_sub(1);
								if new_balance.is_zero() {
									balances.remove(&collection_id);
								} else {
									*balance = new_balance;
								}
								Ok(())
							},
							None => return Err(Error::NoToken.into()), // should not happen
						}
					},
					None => return Err(Error::NoToken.into()), // should not happen
				}
			})?;

			if let Some(collection_issuance) = Self::collection_issuance(collection_id) {
				if collection_issuance.saturating_sub(1).is_zero() {
					// this is the last of the tokens
					<CollectionInfo<T>>::remove(collection_id);
					<CollectionIssuance<T>>::remove(collection_id);
				} else {
					<CollectionIssuance<T>>::mutate(collection_id, |mut q| {
						if let Some(q) = &mut q {
							*q = q.saturating_sub(1)
						}
					});
				}
			}

			Ok(())
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10000)]
		/// Bridged collections from Ethereum will initially lack an owner. These collections will
		/// be assigned to the pallet. This allows for claiming those collections assuming they were
		/// assigned to the pallet
		pub fn claim_unowned_collection(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			new_owner: T::AccountId,
		) -> DispatchResult {
			let _who = ensure_root(origin);

			if let Some(mut collection_info) = Self::collection_info(collection_id) {
				ensure!(
					collection_info.owner == T::PalletId::get().into_account_truncating(),
					Error::<T>::NoPermission
				);
				collection_info.owner = new_owner;
			};
			Ok(())
		}

		/// Set the owner of a collection
		/// Caller must be the current collection owner
		#[pallet::weight(T::WeightInfo::set_owner())]
		pub fn set_owner(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			new_owner: T::AccountId,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			if let Some(mut collection_info) = Self::collection_info(collection_id) {
				ensure!(collection_info.owner == origin, Error::<T>::NoPermission);
				collection_info.owner = new_owner.clone();
				<CollectionInfo<T>>::insert(collection_id, collection_info);
				Self::deposit_event(Event::<T>::OwnerSet { collection_id, new_owner });
				Ok(())
			} else {
				Err(Error::<T>::NoCollection.into())
			}
		}

		/// Flag an account as a marketplace
		///
		/// `marketplace_account` - if specified, this account will be registered
		/// `entitlement` - Permill, percentage of sales to go to the marketplace
		/// If no marketplace is specified the caller will be registered
		#[pallet::weight(16_000_000)]
		pub fn register_marketplace(
			origin: OriginFor<T>,
			marketplace_account: Option<T::AccountId>,
			entitlement: Permill,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			ensure!(
				entitlement.deconstruct() as u32 <= Permill::ACCURACY,
				Error::<T>::RoyaltiesInvalid
			);
			let marketplace_account = marketplace_account.unwrap_or(origin);
			let marketplace_id = Self::next_marketplace_id();
			let marketplace = Marketplace { account: marketplace_account.clone(), entitlement };
			let next_marketplace_id = <NextMarketplaceId<T>>::get();
			ensure!(
				next_marketplace_id.checked_add(One::one()).is_some(),
				Error::<T>::NoAvailableIds
			);
			<RegisteredMarketplaces<T>>::insert(&marketplace_id, marketplace);
			Self::deposit_event(Event::<T>::MarketplaceRegister {
				account: marketplace_account,
				entitlement,
				marketplace_id,
			});
			<NextMarketplaceId<T>>::mutate(|i| *i += 1);
			Ok(())
		}

		/// Create a new collection
		/// Additional tokens can be minted via `mint_additional`
		///
		/// `name` - the name of the collection
		/// `initial_issuance` - number of tokens to mint now
		/// `max_issuance` - maximum number of tokens allowed in collection
		/// `owner` - the token owner, defaults to the caller
		/// `metadata_scheme` - The off-chain metadata referencing scheme for tokens in this
		/// collection `royalties_schedule` - defacto royalties plan for secondary sales, this will
		/// apply to all tokens in the collection by default.
		#[pallet::weight(T::WeightInfo::mint_collection(*initial_issuance))]
		#[transactional]
		pub fn create_collection(
			origin: OriginFor<T>,
			name: CollectionNameType,
			initial_issuance: TokenCount,
			max_issuance: Option<TokenCount>,
			token_owner: Option<T::AccountId>,
			metadata_scheme: MetadataScheme,
			royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			Self::do_create_collection(
				origin,
				name,
				initial_issuance,
				max_issuance,
				token_owner,
				metadata_scheme,
				royalties_schedule,
				OriginChain::Root,
			)?;
			Ok(())
		}

		/// Mint tokens for an existing collection
		///
		/// `collection_id` - the collection to mint tokens in
		/// `quantity` - how many tokens to mint
		/// `token_owner` - the token owner, defaults to the caller if unspecified
		/// Caller must be the collection owner
		/// -----------
		/// Weight is O(N) where N is `quantity`
		#[pallet::weight(T::WeightInfo::mint_additional(*quantity))]
		#[transactional]
		pub fn mint(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			quantity: TokenCount,
			token_owner: Option<T::AccountId>,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			let serial_number = Self::next_serial_number(collection_id).unwrap_or_default();
			ensure!(serial_number.checked_add(quantity).is_some(), Error::<T>::NoAvailableIds);

			// Permission and existence check
			if let Some(collection_info) = Self::collection_info(collection_id) {
				ensure!(collection_info.owner == origin, Error::<T>::NoPermission);
				// Cannot mint for a token that was bridged from Ethereum
				ensure!(
					collection_info.source_chain == OriginChain::Root,
					Error::<T>::AttemptedMintOnBridgedToken
				);

				if let Some(max_issuance) = collection_info.max_issuance {
					ensure!(
						max_issuance >= serial_number.saturating_add(quantity),
						Error::<T>::MaxIssuanceReached
					);
				}
			} else {
				return Err(Error::<T>::NoCollection.into())
			}

			let owner = token_owner.unwrap_or(origin);

			let token_ids: Vec<SerialNumber> = (serial_number..quantity).collect();
			Self::do_mint(&owner, collection_id, token_ids, OriginChain::Root)?;

			Self::deposit_event(Event::<T>::Mint {
				collection_id,
				first_serial_number: serial_number,
				token_count: quantity,
				owner,
			});

			Ok(())
		}

		/// Transfer ownership of an NFT
		/// Caller must be the token owner
		#[pallet::weight(T::WeightInfo::transfer())]
		#[transactional]
		pub fn transfer(
			origin: OriginFor<T>,
			token_id: TokenId,
			new_owner: T::AccountId,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			ensure!(!<TokenLocks<T>>::contains_key(token_id), Error::<T>::TokenLocked);
			ensure!(
				Self::token_owner(token_id.0, token_id.1) == Some(origin.clone()),
				Error::<T>::NoPermission
			);
			let _ = Self::do_transfer_unchecked(token_id.clone(), &origin, &new_owner)?;

			Self::deposit_event(Event::<T>::Transfer {
				previous_owner: origin,
				collection_id: token_id.0,
				serial_number: token_id.1,
				new_owner,
			});
			Ok(())
		}

		/// Burn a token 🔥
		///
		/// Caller must be the token owner
		#[pallet::weight(T::WeightInfo::burn())]
		#[transactional]
		pub fn burn(origin: OriginFor<T>, token_id: TokenId) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			let (collection_id, serial_number) = token_id;

			Self::do_burn(&origin, collection_id, &serial_number)?;
			Self::deposit_event(Event::<T>::Burn { collection_id, serial_number });
			Ok(())
		}

		/// Sell a bundle of tokens at a fixed price
		/// - Tokens must be from the same collection
		/// - Tokens with individual royalties schedules cannot be sold with this method
		///
		/// `buyer` optionally, the account to receive the NFT. If unspecified, then any account may
		/// purchase `asset_id` fungible asset Id to receive as payment for the NFT
		/// `fixed_price` ask price
		/// `duration` listing duration time in blocks from now
		/// Caller must be the token owner
		#[pallet::weight(
		T::WeightInfo::sell()
		.saturating_add(
		T::DbWeight::get().reads_writes(2, 1).saturating_mul(tokens.len() as Weight)
		)
		)]
		#[transactional]
		pub fn sell(
			origin: OriginFor<T>,
			tokens: Vec<TokenId>,
			buyer: Option<T::AccountId>,
			payment_asset: AssetId,
			fixed_price: Balance,
			duration: Option<T::BlockNumber>,
			marketplace_id: Option<MarketplaceId>,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			if tokens.is_empty() {
				return Err(Error::<T>::NoToken.into())
			}

			let royalties_schedule = Self::check_bundle_royalties(&tokens, marketplace_id)?;

			let listing_id = Self::next_listing_id();
			ensure!(listing_id.checked_add(One::one()).is_some(), Error::<T>::NoAvailableIds);

			// use the first token's collection as representative of the bundle
			let (bundle_collection_id, _serial_number) = tokens[0];
			for (collection_id, serial_number) in tokens.iter() {
				ensure!(
					!<TokenLocks<T>>::contains_key((collection_id, serial_number)),
					Error::<T>::TokenLocked
				);
				ensure!(
					Self::token_owner(collection_id, serial_number) == Some(origin.clone()),
					Error::<T>::NoPermission
				);
				<TokenLocks<T>>::insert(
					(collection_id, serial_number),
					TokenLockReason::Listed(listing_id),
				);
			}

			let listing_end_block = <frame_system::Pallet<T>>::block_number()
				.saturating_add(duration.unwrap_or_else(T::DefaultListingDuration::get));
			<ListingEndSchedule<T>>::insert(listing_end_block, listing_id, true);
			let listing = Listing::<T>::FixedPrice(FixedPriceListing::<T> {
				payment_asset,
				fixed_price,
				close: listing_end_block,
				tokens: tokens.clone(),
				buyer: buyer.clone(),
				seller: origin.clone(),
				royalties_schedule,
				marketplace_id,
			});

			<OpenCollectionListings<T>>::insert(bundle_collection_id, listing_id, true);
			<Listings<T>>::insert(listing_id, listing);
			<NextListingId<T>>::mutate(|i| *i += 1);

			Self::deposit_event(Event::<T>::FixedPriceSaleList {
				tokens,
				listing_id,
				marketplace_id,
				price: fixed_price,
				payment_asset,
				seller: origin,
			});
			Ok(())
		}

		/// Buy a token listing for its specified price
		#[pallet::weight(T::WeightInfo::buy())]
		#[transactional]
		pub fn buy(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			if let Some(Listing::FixedPrice(listing)) = Self::listings(listing_id) {
				// if buyer is specified in the listing, then `origin` must be buyer
				if let Some(buyer) = &listing.buyer {
					ensure!(&origin == buyer, Error::<T>::NoPermission);
				}

				let collection_id = listing.tokens.get(0).ok_or_else(|| Error::<T>::NoToken)?.0;
				let payouts = Self::calculate_royalty_payouts(
					listing.seller.clone(),
					listing.royalties_schedule,
					listing.fixed_price,
				);
				// Make split transfer
				T::MultiCurrency::split_transfer(
					&origin,
					listing.payment_asset,
					payouts.as_slice(),
				)?;

				<OpenCollectionListings<T>>::remove(collection_id, listing_id);

				for token_id in listing.tokens.clone() {
					<TokenLocks<T>>::remove(token_id);
					let _ = Self::do_transfer_unchecked(token_id, &listing.seller, &origin)?;
				}
				Self::remove_fixed_price_listing(listing_id);

				Self::deposit_event(Event::<T>::FixedPriceSaleComplete {
					tokens: listing.tokens,
					listing_id,
					price: listing.fixed_price,
					payment_asset: listing.payment_asset,
					buyer: origin,
					seller: listing.seller,
				});
			} else {
				return Err(Error::<T>::NotForFixedPriceSale.into())
			}
			Ok(())
		}

		/// Auction a bundle of tokens on the open market to the highest bidder
		/// - Tokens must be from the same collection
		/// - Tokens with individual royalties schedules cannot be sold in bundles
		///
		/// Caller must be the token owner
		/// - `payment_asset` fungible asset Id to receive payment with
		/// - `reserve_price` winning bid must be over this threshold
		/// - `duration` length of the auction (in blocks), uses default duration if unspecified
		#[pallet::weight(
		T::WeightInfo::sell()
		.saturating_add(
		T::DbWeight::get().reads_writes(2, 1).saturating_mul(tokens.len() as Weight)
		)
		)]
		#[transactional]
		pub fn auction(
			origin: OriginFor<T>,
			tokens: Vec<TokenId>,
			payment_asset: AssetId,
			reserve_price: Balance,
			duration: Option<T::BlockNumber>,
			marketplace_id: Option<MarketplaceId>,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			if tokens.is_empty() {
				return Err(Error::<T>::NoToken.into())
			}

			let royalties_schedule = Self::check_bundle_royalties(&tokens, marketplace_id)?;

			let listing_id = Self::next_listing_id();
			ensure!(listing_id.checked_add(One::one()).is_some(), Error::<T>::NoAvailableIds);

			// use the first token's collection as representative of the bundle
			let (bundle_collection_id, _serial_number) = tokens[0];
			for (collection_id, serial_number) in tokens.iter() {
				ensure!(
					!<TokenLocks<T>>::contains_key((collection_id, serial_number)),
					Error::<T>::TokenLocked
				);
				ensure!(
					Self::token_owner(collection_id, serial_number) == Some(origin.clone()),
					Error::<T>::NoPermission
				);
				<TokenLocks<T>>::insert(
					(collection_id, serial_number),
					TokenLockReason::Listed(listing_id),
				);
			}

			let listing_end_block = <frame_system::Pallet<T>>::block_number()
				.saturating_add(duration.unwrap_or_else(T::DefaultListingDuration::get));
			<ListingEndSchedule<T>>::insert(listing_end_block, listing_id, true);
			let listing = Listing::<T>::Auction(AuctionListing::<T> {
				payment_asset,
				reserve_price,
				close: listing_end_block,
				tokens: tokens.clone(),
				seller: origin.clone(),
				royalties_schedule,
				marketplace_id,
			});

			<OpenCollectionListings<T>>::insert(bundle_collection_id, listing_id, true);
			<Listings<T>>::insert(listing_id, listing);
			<NextListingId<T>>::mutate(|i| *i += 1);

			Self::deposit_event(Event::<T>::AuctionOpen {
				tokens,
				payment_asset,
				reserve_price,
				listing_id,
				marketplace_id,
				seller: origin,
			});
			Ok(())
		}

		/// Place a bid on an open auction
		/// - `amount` to bid (in the seller's requested payment asset)
		#[pallet::weight(T::WeightInfo::bid())]
		#[transactional]
		pub fn bid(origin: OriginFor<T>, listing_id: ListingId, amount: Balance) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			if let Some(Listing::Auction(mut listing)) = Self::listings(listing_id) {
				if let Some(current_bid) = Self::listing_winning_bid(listing_id) {
					ensure!(amount > current_bid.1, Error::<T>::BidTooLow);
				} else {
					// first bid
					ensure!(amount >= listing.reserve_price, Error::<T>::BidTooLow);
				}

				// try lock funds
				T::MultiCurrency::place_hold(
					T::PalletId::get(),
					&origin,
					listing.payment_asset,
					amount,
				)?;

				<ListingWinningBid<T>>::mutate(listing_id, |maybe_current_bid| {
					if let Some(current_bid) = maybe_current_bid {
						// replace old bid
						let _ = T::MultiCurrency::release_hold(
							T::PalletId::get(),
							&current_bid.0,
							listing.payment_asset,
							current_bid.1,
						);
					}
					*maybe_current_bid = Some((origin.clone(), amount))
				});

				// Auto extend auction if bid is made within certain amount of time of auction
				// duration
				let listing_end_block = listing.close;
				let current_block = <frame_system::Pallet<T>>::block_number();
				let blocks_till_close = listing_end_block - current_block;
				let new_closing_block =
					current_block + T::BlockNumber::from(AUCTION_EXTENSION_PERIOD);
				if blocks_till_close <= T::BlockNumber::from(AUCTION_EXTENSION_PERIOD) {
					ListingEndSchedule::<T>::remove(listing_end_block, listing_id);
					ListingEndSchedule::<T>::insert(new_closing_block, listing_id, true);
					listing.close = new_closing_block;
					Listings::<T>::insert(listing_id, Listing::Auction(listing.clone()));
				}

				Self::deposit_event(Event::<T>::Bid {
					tokens: listing.tokens,
					listing_id,
					amount,
					bidder: origin,
				});
				Ok(())
			} else {
				return Err(Error::<T>::NotForAuction.into())
			}
		}

		/// Close a sale or auction returning tokens
		/// Requires no successful bids have been made for an auction.
		/// Caller must be the listed seller
		#[pallet::weight(T::WeightInfo::cancel_sale())]
		pub fn cancel_sale(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			match Self::listings(listing_id) {
				Some(Listing::<T>::FixedPrice(sale)) => {
					ensure!(sale.seller == origin, Error::<T>::NoPermission);
					Listings::<T>::remove(listing_id);
					ListingEndSchedule::<T>::remove(sale.close, listing_id);
					for token_id in sale.tokens.iter() {
						<TokenLocks<T>>::remove(token_id);
					}
					let collection_id = sale.tokens[0].0;
					<OpenCollectionListings<T>>::remove(collection_id, listing_id);

					Self::deposit_event(Event::<T>::FixedPriceSaleClose {
						tokens: sale.tokens,
						listing_id,
						reason: FixedPriceClosureReason::VendorCancelled,
					});
				},
				Some(Listing::<T>::Auction(auction)) => {
					ensure!(auction.seller == origin, Error::<T>::NoPermission);
					ensure!(
						Self::listing_winning_bid(listing_id).is_none(),
						Error::<T>::TokenLocked
					);
					Listings::<T>::remove(listing_id);
					ListingEndSchedule::<T>::remove(auction.close, listing_id);
					for token_id in auction.tokens.iter() {
						<TokenLocks<T>>::remove(token_id);
					}
					let collection_id = auction.tokens[0].0;
					<OpenCollectionListings<T>>::remove(collection_id, listing_id);

					Self::deposit_event(Event::<T>::AuctionClose {
						collection_id,
						listing_id,
						reason: AuctionClosureReason::VendorCancelled,
					});
				},
				None => {},
			}
			Ok(())
		}

		/// Update fixed price for a single token sale
		///
		/// `listing_id` id of the fixed price listing
		/// `new_price` new fixed price
		/// Caller must be the token owner
		#[pallet::weight(T::WeightInfo::update_fixed_price())]
		pub fn update_fixed_price(
			origin: OriginFor<T>,
			listing_id: ListingId,
			new_price: Balance,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			match Self::listings(listing_id) {
				Some(Listing::<T>::FixedPrice(mut sale)) => {
					ensure!(sale.seller == origin, Error::<T>::NoPermission);

					sale.fixed_price = new_price;

					<Listings<T>>::insert(listing_id, Listing::<T>::FixedPrice(sale.clone()));
					Self::deposit_event(Event::<T>::FixedPriceSalePriceUpdate {
						tokens: sale.tokens,
						listing_id,
						new_price,
					});
					Ok(())
				},
				Some(Listing::<T>::Auction(_)) => Err(Error::<T>::NotForFixedPriceSale.into()),
				None => Err(Error::<T>::NotForFixedPriceSale.into()),
			}
		}

		/// Create an offer on a token
		/// Locks funds until offer is accepted, rejected or cancelled
		/// An offer can't be made on a token currently in an auction
		/// (This follows the behaviour of Opensea and forces the buyer to bid rather than create an
		/// offer)
		#[pallet::weight(T::WeightInfo::make_simple_offer())]
		#[transactional]
		pub fn make_simple_offer(
			origin: OriginFor<T>,
			token_id: TokenId,
			amount: Balance,
			asset_id: AssetId,
			marketplace_id: Option<MarketplaceId>,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			ensure!(!amount.is_zero(), Error::<T>::ZeroOffer);
			ensure!(
				Self::token_owner(token_id.0, token_id.1) != Some(origin.clone()),
				Error::<T>::IsTokenOwner
			);
			let offer_id = Self::next_offer_id();
			ensure!(offer_id.checked_add(One::one()).is_some(), Error::<T>::NoAvailableIds);

			// ensure the token_id is not currently in an auction
			if let Some(TokenLockReason::Listed(listing_id)) = Self::token_locks(token_id) {
				match Self::listings(listing_id) {
					Some(Listing::<T>::Auction(_)) => return Err(Error::<T>::TokenOnAuction.into()),
					None | Some(Listing::<T>::FixedPrice(_)) => (),
				}
			}

			// try lock funds
			T::MultiCurrency::place_hold(T::PalletId::get(), &origin, asset_id, amount)?;
			<TokenOffers<T>>::append(token_id, offer_id);
			let new_offer = OfferType::<T::AccountId>::Simple(SimpleOffer {
				token_id,
				asset_id,
				amount,
				buyer: origin.clone(),
				marketplace_id,
			});
			<Offers<T>>::insert(offer_id, new_offer);
			<NextOfferId<T>>::mutate(|i| *i += 1);

			Self::deposit_event(Event::<T>::Offer {
				offer_id,
				amount,
				asset_id,
				marketplace_id,
				buyer: origin,
			});
			Ok(())
		}

		/// Cancels an offer on a token
		/// Caller must be the offer buyer
		#[pallet::weight(T::WeightInfo::cancel_offer())]
		pub fn cancel_offer(origin: OriginFor<T>, offer_id: OfferId) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			if let Some(offer_type) = Self::offers(offer_id) {
				match offer_type {
					OfferType::Simple(offer) => {
						ensure!(offer.buyer == origin, Error::<T>::NotBuyer);
						T::MultiCurrency::release_hold(
							T::PalletId::get(),
							&origin,
							offer.asset_id,
							offer.amount,
						)?;
						Offers::<T>::remove(offer_id);
						if let Some(offers) = Self::token_offers(offer.token_id) {
							if offers.len() == 1 {
								// this is the last of the token offers
								<TokenOffers<T>>::remove(offer.token_id);
							} else {
								<TokenOffers<T>>::mutate(offer.token_id, |mut offers| {
									if let Some(offers) = &mut offers {
										offers
											.binary_search(&offer_id)
											.map(|idx| offers.remove(idx))
											.unwrap();
									}
								});
							}
						};
						Self::deposit_event(Event::<T>::OfferCancel {
							offer_id,
							token_id: offer.token_id,
						});
						Ok(())
					},
				}
			} else {
				Err(Error::<T>::InvalidOffer.into())
			}
		}

		/// Accepts an offer on a token
		/// Caller must be token owner
		#[pallet::weight(T::WeightInfo::accept_offer())]
		#[transactional]
		pub fn accept_offer(origin: OriginFor<T>, offer_id: OfferId) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			if let Some(offer_type) = Self::offers(offer_id) {
				match offer_type {
					OfferType::Simple(offer) => {
						let token_id = offer.token_id;
						ensure!(
							Self::token_owner(token_id.0, token_id.1) == Some(origin.clone()),
							Error::<T>::NoPermission
						);

						let royalties_schedule =
							Self::check_bundle_royalties(&vec![token_id], offer.marketplace_id)?;
						Self::process_payment_and_transfer(
							&offer.buyer,
							&origin,
							offer.asset_id,
							vec![offer.token_id],
							offer.amount,
							royalties_schedule,
						)?;

						// Clean storage
						Offers::<T>::remove(offer_id);
						if let Some(offers) = Self::token_offers(token_id) {
							if offers.len() == 1 {
								// this is the last of the token offers
								<TokenOffers<T>>::remove(token_id);
							} else {
								<TokenOffers<T>>::mutate(token_id, |mut offers| {
									if let Some(offers) = &mut offers {
										offers
											.binary_search(&offer_id)
											.map(|idx| offers.remove(idx))
											.unwrap();
									}
								});
							}
						};
						Self::deposit_event(Event::<T>::OfferAccept {
							offer_id,
							token_id: offer.token_id,
							amount: offer.amount,
							asset_id: offer.asset_id,
						});
						Ok(())
					},
				}
			} else {
				Err(Error::<T>::InvalidOffer.into())
			}
		}
	}
}
