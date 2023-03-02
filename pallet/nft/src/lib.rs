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
	ensure,
	traits::{tokens::fungibles::Mutate, Get},
	transactional, PalletId,
};
use seed_pallet_common::{
	CreateExt, Hold, OnNewAssetSubscriber, OnTransferSubscriber, TransferExt,
};
use seed_primitives::{AssetId, Balance, CollectionUuid, ParachainId, SerialNumber, TokenId};
use sp_runtime::{
	traits::{AccountIdConversion, One, Saturating, Zero},
	DispatchResult, PerThing, Permill,
};
use sp_std::prelude::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;
mod weights;
pub use weights::WeightInfo;

mod impls;
mod migration;
pub mod traits;
mod types;

pub use impls::*;
pub use pallet::*;
pub use types::*;

/// The maximum length of valid collection IDs
pub const MAX_COLLECTION_NAME_LENGTH: u8 = 32;
/// The maximum amount of listings to return
pub const MAX_COLLECTION_LISTING_LIMIT: u16 = 100;
/// The maximum amount of listings to return
pub const MAX_OWNED_TOKENS_LIMIT: u16 = 500;
/// The logging target for this module
pub(crate) const LOG_TARGET: &str = "nft";

#[frame_support::pallet]
pub mod pallet {
	use super::{DispatchResult, *};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
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
		}
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Default auction / sale length in blocks
		#[pallet::constant]
		type DefaultListingDuration: Get<Self::BlockNumber>;
		/// The system event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The maximum number of offers allowed on a collection
		type MaxOffers: Get<u32>;
		/// Max tokens that a collection can contain
		type MaxTokensPerCollection: Get<u32>;
		/// Handles a multi-currency fungible asset system
		type MultiCurrency: TransferExt<AccountId = Self::AccountId>
			+ Hold<AccountId = Self::AccountId>
			+ Mutate<Self::AccountId, AssetId = AssetId>
			+ CreateExt<AccountId = Self::AccountId>;
		/// Handler for when an NFT has been transferred
		type OnTransferSubscription: OnTransferSubscriber;
		/// Handler for when an NFT collection has been created
		type OnNewAssetSubscription: OnNewAssetSubscriber<CollectionUuid>;
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
		StorageMap<_, Twox64Concat, CollectionUuid, CollectionInformation<T>>;

	/// The next available incrementing collection id
	#[pallet::storage]
	pub type NextCollectionId<T> = StorageValue<_, u32, ValueQuery>;

	/// Map from a token to lock status if any
	#[pallet::storage]
	#[pallet::getter(fn token_locks)]
	pub type TokenLocks<T> = StorageMap<_, Twox64Concat, TokenId, TokenLockReason>;

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
	pub type TokenOffers<T: Config> =
		StorageMap<_, Twox64Concat, TokenId, BoundedVec<OfferId, T::MaxOffers>>;

	/// The next available offer_id
	#[pallet::storage]
	#[pallet::getter(fn next_offer_id)]
	pub type NextOfferId<T> = StorageValue<_, OfferId, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new collection of tokens was created
		CollectionCreate {
			collection_uuid: CollectionUuid,
			initial_issuance: TokenCount,
			max_issuance: Option<TokenCount>,
			collection_owner: T::AccountId,
			metadata_scheme: MetadataScheme,
			name: CollectionNameType,
			royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
			origin_chain: OriginChain,
		},
		/// Token(s) were minted
		Mint {
			collection_id: CollectionUuid,
			start: SerialNumber,
			end: SerialNumber,
			owner: T::AccountId,
		},
		/// A new owner was set
		OwnerSet { collection_id: CollectionUuid, new_owner: T::AccountId },
		/// Max issuance was set
		MaxIssuanceSet { collection_id: CollectionUuid, max_issuance: TokenCount },
		/// Base URI was set
		BaseUriSet { collection_id: CollectionUuid, base_uri: Vec<u8> },
		/// A token was transferred
		Transfer {
			previous_owner: T::AccountId,
			collection_id: CollectionUuid,
			serial_numbers: Vec<SerialNumber>,
			new_owner: T::AccountId,
		},
		/// A token was burned
		Burn { collection_id: CollectionUuid, serial_number: SerialNumber },
		/// A fixed price sale has been listed
		FixedPriceSaleList {
			collection_id: CollectionUuid,
			serial_numbers: Vec<SerialNumber>,
			listing_id: ListingId,
			marketplace_id: Option<MarketplaceId>,
			price: Balance,
			payment_asset: AssetId,
			seller: T::AccountId,
		},
		/// A fixed price sale has completed
		FixedPriceSaleComplete {
			collection_id: CollectionUuid,
			serial_numbers: Vec<SerialNumber>,
			listing_id: ListingId,
			price: Balance,
			payment_asset: AssetId,
			buyer: T::AccountId,
			seller: T::AccountId,
		},
		/// A fixed price sale has closed without selling
		FixedPriceSaleClose {
			collection_id: CollectionUuid,
			serial_numbers: Vec<SerialNumber>,
			listing_id: ListingId,
			reason: FixedPriceClosureReason,
		},
		/// A fixed price sale has had its price updated
		FixedPriceSalePriceUpdate {
			collection_id: CollectionUuid,
			serial_numbers: Vec<SerialNumber>,
			listing_id: ListingId,
			new_price: Balance,
		},
		/// An auction has opened
		AuctionOpen {
			collection_id: CollectionUuid,
			serial_numbers: Vec<SerialNumber>,
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
		Bid {
			collection_id: CollectionUuid,
			serial_numbers: Vec<SerialNumber>,
			listing_id: ListingId,
			amount: Balance,
			bidder: T::AccountId,
		},
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
		/// An offer has been accepted
		OfferAccept { offer_id: OfferId, token_id: TokenId, amount: Balance, asset_id: AssetId },
		/// Collection has been claimed
		CollectionClaimed { account: T::AccountId, collection_id: CollectionUuid },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Given collection name is invalid (invalid utf-8, too long, empty)
		CollectionNameInvalid,
		/// No more Ids are available, they've been exhausted
		NoAvailableIds,
		/// Origin does not own the NFT
		NotTokenOwner,
		/// The token does not exist
		NoToken,
		/// The token is not listed for fixed price sale
		NotForFixedPriceSale,
		/// The token is not listed for auction sale
		NotForAuction,
		/// Origin is not the collection owner and is not permitted to perform the operation
		NotCollectionOwner,
		/// The token is not listed for sale
		TokenNotListed,
		/// The maximum number of offers on this token has been reached
		MaxOffersReached,
		/// Cannot operate on a listed NFT
		TokenLocked,
		/// Total royalties would exceed 100% of sale or an empty vec is supplied
		RoyaltiesInvalid,
		/// Auction bid was lower than reserve or current highest bid
		BidTooLow,
		/// Selling tokens from different collection is not allowed
		MixedBundleSale,
		/// The account_id hasn't been registered as a marketplace
		MarketplaceNotRegistered,
		/// The collection does not exist
		NoCollectionFound,
		/// The metadata path is invalid (non-utf8 or empty)
		InvalidMetadataPath,
		/// No offer exists for the given OfferId
		InvalidOffer,
		/// The caller is not the specified buyer
		NotBuyer,
		/// The caller is not the seller of the NFT
		NotSeller,
		/// The caller owns the token and can't make an offer
		IsTokenOwner,
		/// Offer amount needs to be greater than 0
		ZeroOffer,
		/// The number of tokens have exceeded the max tokens allowed
		TokenLimitExceeded,
		/// Cannot make an offer on a token up for auction
		TokenOnAuction,
		/// Max issuance needs to be greater than 0 and initial_issuance
		/// Cannot exceed MaxTokensPerCollection
		InvalidMaxIssuance,
		/// The max issuance has already been set and can't be changed
		MaxIssuanceAlreadySet,
		/// The collection max issuance has been reached and no more tokens can be minted
		MaxIssuanceReached,
		/// Attemped to mint a token that was bridged from a different chain
		AttemptedMintOnBridgedToken,
		/// Cannot claim already claimed collections
		CannotClaimNonClaimableCollections,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<(), &'static str> {
			migration::v2::pre_upgrade::<T>()?;

			Ok(())
		}

		/// Perform runtime upgrade
		fn on_runtime_upgrade() -> Weight {
			let mut weight = migration::try_migrate::<T>();
			weight += migration::v2::migrate::<T>();
			weight
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade() -> Result<(), &'static str> {
			migration::v2::post_upgrade::<T>()?;

			Ok(())
		}

		/// Check and close all expired listings
		fn on_initialize(now: T::BlockNumber) -> Weight {
			// TODO: this is unbounded and could become costly
			// https://github.com/cennznet/cennznet/issues/444
			let removed_count = Self::close_listings_at(now);
			// 'buy' weight is comparable to successful closure of an auction
			T::WeightInfo::buy() * removed_count as Weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::claim_unowned_collection())]
		/// Bridged collections from Ethereum will initially lack an owner. These collections will
		/// be assigned to the pallet. This allows for claiming those collections assuming they were
		/// assigned to the pallet
		pub fn claim_unowned_collection(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			new_owner: T::AccountId,
		) -> DispatchResult {
			let _who = ensure_root(origin)?;

			CollectionInfo::<T>::try_mutate(collection_id, |maybe_collection| -> DispatchResult {
				let collection = maybe_collection.as_mut().ok_or(Error::<T>::NoCollectionFound)?;
				ensure!(
					collection.owner == Self::account_id(),
					Error::<T>::CannotClaimNonClaimableCollections
				);

				collection.owner = new_owner.clone();
				Ok(())
			})?;
			let event = Event::<T>::CollectionClaimed { account: new_owner, collection_id };
			Self::deposit_event(event);

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
			let who = ensure_signed(origin)?;
			let mut collection_info =
				Self::collection_info(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
			ensure!(collection_info.owner == who, Error::<T>::NotCollectionOwner);
			collection_info.owner = new_owner.clone();
			<CollectionInfo<T>>::insert(collection_id, collection_info);
			Self::deposit_event(Event::<T>::OwnerSet { collection_id, new_owner });
			Ok(())
		}

		/// Set the max issuance of a collection
		/// Caller must be the current collection owner
		#[pallet::weight(T::WeightInfo::set_owner())] // TODO - weights
		pub fn set_max_issuance(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			max_issuance: TokenCount,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut collection_info =
				Self::collection_info(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
			ensure!(!max_issuance.is_zero(), Error::<T>::InvalidMaxIssuance);
			ensure!(collection_info.owner == who, Error::<T>::NotCollectionOwner);
			ensure!(collection_info.max_issuance.is_none(), Error::<T>::MaxIssuanceAlreadySet);
			ensure!(
				collection_info.collection_issuance <= max_issuance,
				Error::<T>::InvalidMaxIssuance
			);

			match collection_info.max_issuance {
				// cannot set - if already set
				Some(_) => return Err(Error::<T>::InvalidMaxIssuance.into()),
				// if not set, ensure that the max issuance is greater than the current issuance
				None => ensure!(
					collection_info.collection_issuance <= max_issuance,
					Error::<T>::InvalidMaxIssuance
				),
			}

			collection_info.max_issuance = Some(max_issuance);
			<CollectionInfo<T>>::insert(collection_id, collection_info);
			Self::deposit_event(Event::<T>::MaxIssuanceSet { collection_id, max_issuance });
			Ok(())
		}

		/// Set the base URI of a collection
		/// Caller must be the current collection owner
		#[pallet::weight(T::WeightInfo::set_owner())] // TODO - weights
		pub fn set_base_uri(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			base_uri: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut collection_info =
				Self::collection_info(collection_id).ok_or(Error::<T>::NoCollectionFound)?;
			ensure!(collection_info.owner == who, Error::<T>::NotCollectionOwner);

			collection_info.metadata_scheme =
				base_uri.clone().try_into().map_err(|_| Error::<T>::InvalidMetadataPath)?;

			<CollectionInfo<T>>::insert(collection_id, collection_info);
			Self::deposit_event(Event::<T>::BaseUriSet { collection_id, base_uri });
			Ok(())
		}

		/// Flag an account as a marketplace
		///
		/// `marketplace_account` - if specified, this account will be registered
		/// `entitlement` - Permill, percentage of sales to go to the marketplace
		/// If no marketplace is specified the caller will be registered
		#[pallet::weight(T::WeightInfo::register_marketplace())]
		pub fn register_marketplace(
			origin: OriginFor<T>,
			marketplace_account: Option<T::AccountId>,
			entitlement: Permill,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				entitlement.deconstruct() as u32 <= Permill::ACCURACY,
				Error::<T>::RoyaltiesInvalid
			);
			let marketplace_account = marketplace_account.unwrap_or(who);
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
		/// `token_owner` - the token owner, defaults to the caller
		/// `metadata_scheme` - The off-chain metadata referencing scheme for tokens in this
		/// `royalties_schedule` - defacto royalties plan for secondary sales, this will
		/// apply to all tokens in the collection by default.
		#[pallet::weight(T::WeightInfo::create_collection())]
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
			let who = ensure_signed(origin)?;
			Self::do_create_collection(
				who,
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
		#[pallet::weight(T::WeightInfo::mint())]
		#[transactional]
		pub fn mint(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			quantity: TokenCount,
			token_owner: Option<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(quantity > Zero::zero(), Error::<T>::NoToken);

			let mut collection_info =
				Self::collection_info(collection_id).ok_or(Error::<T>::NoCollectionFound)?;

			// Caller must be collection_owner
			ensure!(collection_info.owner == who, Error::<T>::NotCollectionOwner);

			// Check we don't exceed the token limit
			ensure!(
				collection_info.collection_issuance.saturating_add(quantity) <
					T::MaxTokensPerCollection::get(),
				Error::<T>::TokenLimitExceeded
			);

			// Cannot mint for a token that was bridged from Ethereum
			ensure!(
				collection_info.origin_chain == OriginChain::Root,
				Error::<T>::AttemptedMintOnBridgedToken
			);

			let next_serial_number = collection_info.next_serial_number;
			// Increment next serial number
			collection_info.next_serial_number =
				next_serial_number.checked_add(quantity).ok_or(Error::<T>::NoAvailableIds)?;

			// Check early that we won't exceed the BoundedVec limit
			ensure!(
				collection_info.next_serial_number <= T::MaxTokensPerCollection::get(),
				Error::<T>::TokenLimitExceeded
			);

			// Can't mint more than specified max_issuance
			if let Some(max_issuance) = collection_info.max_issuance {
				ensure!(
					max_issuance >= collection_info.next_serial_number,
					Error::<T>::MaxIssuanceReached
				);
			}

			let owner = token_owner.unwrap_or(who);
			let serial_numbers_unbounded: Vec<SerialNumber> =
				(next_serial_number..collection_info.next_serial_number).collect();
			let serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection> =
				BoundedVec::try_from(serial_numbers_unbounded)
					.map_err(|_| Error::<T>::TokenLimitExceeded)?;
			Self::do_mint(collection_id, collection_info, &owner, &serial_numbers)?;

			// throw event, listing starting and endpoint token ids (sequential mint)
			Self::deposit_event(Event::<T>::Mint {
				collection_id,
				start: *serial_numbers.first().ok_or(Error::<T>::NoToken)?,
				end: *serial_numbers.last().ok_or(Error::<T>::NoToken)?,
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
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection>,
			new_owner: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Self::do_transfer(collection_id, serial_numbers, &who, &new_owner)
		}

		/// Burn a token 🔥
		///
		/// Caller must be the token owner
		#[pallet::weight(T::WeightInfo::burn())]
		#[transactional]
		pub fn burn(origin: OriginFor<T>, token_id: TokenId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let (collection_id, serial_number) = token_id;

			Self::do_burn(&who, collection_id, serial_number)?;
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
		#[pallet::weight(T::WeightInfo::sell())]
		#[transactional]
		pub fn sell(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection>,
			buyer: Option<T::AccountId>,
			payment_asset: AssetId,
			fixed_price: Balance,
			duration: Option<T::BlockNumber>,
			marketplace_id: Option<MarketplaceId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(!serial_numbers.is_empty(), Error::<T>::NoToken);
			let royalties_schedule =
				Self::calculate_bundle_royalties(collection_id, marketplace_id)?;
			let listing_id = Self::next_listing_id();

			// use the first token's collection as representative of the bundle
			Self::lock_tokens_for_listing(collection_id, &serial_numbers, &who, listing_id)?;

			let listing_end_block = <frame_system::Pallet<T>>::block_number()
				.saturating_add(duration.unwrap_or_else(T::DefaultListingDuration::get));
			let listing = Listing::<T>::FixedPrice(FixedPriceListing::<T> {
				payment_asset,
				fixed_price,
				close: listing_end_block,
				collection_id,
				serial_numbers: serial_numbers.clone(),
				buyer: buyer.clone(),
				seller: who.clone(),
				royalties_schedule,
				marketplace_id,
			});

			<ListingEndSchedule<T>>::insert(listing_end_block, listing_id, true);
			<OpenCollectionListings<T>>::insert(collection_id, listing_id, true);
			<Listings<T>>::insert(listing_id, listing);
			<NextListingId<T>>::mutate(|i| *i += 1);

			Self::deposit_event(Event::<T>::FixedPriceSaleList {
				collection_id,
				serial_numbers: serial_numbers.into_inner(),
				listing_id,
				marketplace_id,
				price: fixed_price,
				payment_asset,
				seller: who,
			});
			Ok(())
		}

		/// Buy a token listing for its specified price
		#[pallet::weight(T::WeightInfo::buy())]
		#[transactional]
		pub fn buy(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			if let Some(Listing::FixedPrice(listing)) = Self::listings(listing_id) {
				// if buyer is specified in the listing, then `who` must be buyer
				if let Some(buyer) = &listing.buyer {
					ensure!(&who == buyer, Error::<T>::NotBuyer);
				}

				let payouts = Self::calculate_royalty_payouts(
					listing.seller.clone(),
					listing.royalties_schedule,
					listing.fixed_price,
				);
				// Make split transfer
				T::MultiCurrency::split_transfer(&who, listing.payment_asset, payouts.as_slice())?;

				<OpenCollectionListings<T>>::remove(listing.collection_id, listing_id);

				for serial_number in listing.serial_numbers.iter() {
					<TokenLocks<T>>::remove((listing.collection_id, *serial_number));
				}
				// Transfer the tokens
				let _ = Self::do_transfer(
					listing.collection_id,
					listing.serial_numbers.clone(),
					&listing.seller,
					&who,
				)?;

				Self::remove_fixed_price_listing(listing_id);

				Self::deposit_event(Event::<T>::FixedPriceSaleComplete {
					collection_id: listing.collection_id,
					serial_numbers: listing.serial_numbers.into_inner(),
					listing_id,
					price: listing.fixed_price,
					payment_asset: listing.payment_asset,
					buyer: who,
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
		#[pallet::weight(T::WeightInfo::auction())]
		#[transactional]
		pub fn auction(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection>,
			payment_asset: AssetId,
			reserve_price: Balance,
			duration: Option<T::BlockNumber>,
			marketplace_id: Option<MarketplaceId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			if serial_numbers.is_empty() {
				return Err(Error::<T>::NoToken.into())
			}
			let royalties_schedule =
				Self::calculate_bundle_royalties(collection_id, marketplace_id)?;

			let listing_id = Self::next_listing_id();
			ensure!(listing_id.checked_add(One::one()).is_some(), Error::<T>::NoAvailableIds);

			Self::lock_tokens_for_listing(collection_id, &serial_numbers, &who, listing_id)?;

			let listing_end_block = <frame_system::Pallet<T>>::block_number()
				.saturating_add(duration.unwrap_or_else(T::DefaultListingDuration::get));
			let listing = Listing::<T>::Auction(AuctionListing::<T> {
				payment_asset,
				reserve_price,
				close: listing_end_block,
				collection_id,
				serial_numbers: serial_numbers.clone(),
				seller: who.clone(),
				royalties_schedule,
				marketplace_id,
			});

			<ListingEndSchedule<T>>::insert(listing_end_block, listing_id, true);
			<OpenCollectionListings<T>>::insert(collection_id, listing_id, true);
			<Listings<T>>::insert(listing_id, listing);
			<NextListingId<T>>::mutate(|i| *i += 1);

			Self::deposit_event(Event::<T>::AuctionOpen {
				collection_id,
				serial_numbers: serial_numbers.into_inner(),
				payment_asset,
				reserve_price,
				listing_id,
				marketplace_id,
				seller: who,
			});
			Ok(())
		}

		/// Place a bid on an open auction
		/// - `amount` to bid (in the seller's requested payment asset)
		#[pallet::weight(T::WeightInfo::bid())]
		#[transactional]
		pub fn bid(origin: OriginFor<T>, listing_id: ListingId, amount: Balance) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let mut listing = match Self::listings(listing_id) {
				Some(Listing::Auction(listing)) => listing,
				_ => return Err(Error::<T>::NotForAuction.into()),
			};

			if let Some(current_bid) = Self::listing_winning_bid(listing_id) {
				ensure!(amount > current_bid.1, Error::<T>::BidTooLow);
			} else {
				// first bid
				ensure!(amount >= listing.reserve_price, Error::<T>::BidTooLow);
			}

			// try lock funds
			T::MultiCurrency::place_hold(T::PalletId::get(), &who, listing.payment_asset, amount)?;

			<ListingWinningBid<T>>::try_mutate(
				listing_id,
				|maybe_current_bid| -> DispatchResult {
					if let Some(current_bid) = maybe_current_bid {
						// replace old bid
						let _ = T::MultiCurrency::release_hold(
							T::PalletId::get(),
							&current_bid.0,
							listing.payment_asset,
							current_bid.1,
						)?;
					}
					*maybe_current_bid = Some((who.clone(), amount));
					Ok(())
				},
			)?;

			// Auto extend auction if bid is made within certain amount of time of auction
			// duration
			let listing_end_block = listing.close;
			let current_block = <frame_system::Pallet<T>>::block_number();
			let blocks_till_close = listing_end_block - current_block;
			let new_closing_block = current_block + T::BlockNumber::from(AUCTION_EXTENSION_PERIOD);
			if blocks_till_close <= T::BlockNumber::from(AUCTION_EXTENSION_PERIOD) {
				ListingEndSchedule::<T>::remove(listing_end_block, listing_id);
				ListingEndSchedule::<T>::insert(new_closing_block, listing_id, true);
				listing.close = new_closing_block;
				Listings::<T>::insert(listing_id, Listing::Auction(listing.clone()));
			}

			Self::deposit_event(Event::<T>::Bid {
				collection_id: listing.collection_id,
				serial_numbers: listing.serial_numbers.into_inner(),
				listing_id,
				amount,
				bidder: who,
			});
			Ok(())
		}

		/// Close a sale or auction returning tokens
		/// Requires no successful bids have been made for an auction.
		/// Caller must be the listed seller
		#[pallet::weight(T::WeightInfo::cancel_sale())]
		pub fn cancel_sale(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let listing = Self::listings(listing_id).ok_or(Error::<T>::TokenNotListed)?;

			match listing {
				Listing::<T>::FixedPrice(sale) => {
					ensure!(sale.seller == who, Error::<T>::NotSeller);
					Listings::<T>::remove(listing_id);
					ListingEndSchedule::<T>::remove(sale.close, listing_id);
					for serial_number in sale.serial_numbers.iter() {
						<TokenLocks<T>>::remove((sale.collection_id, serial_number));
					}
					<OpenCollectionListings<T>>::remove(sale.collection_id, listing_id);

					Self::deposit_event(Event::<T>::FixedPriceSaleClose {
						collection_id: sale.collection_id,
						serial_numbers: sale.serial_numbers.into_inner(),
						listing_id,
						reason: FixedPriceClosureReason::VendorCancelled,
					});
				},
				Listing::<T>::Auction(auction) => {
					ensure!(auction.seller == who, Error::<T>::NotSeller);
					ensure!(
						Self::listing_winning_bid(listing_id).is_none(),
						Error::<T>::TokenLocked
					);
					Listings::<T>::remove(listing_id);
					ListingEndSchedule::<T>::remove(auction.close, listing_id);
					for serial_number in auction.serial_numbers.iter() {
						<TokenLocks<T>>::remove((auction.collection_id, serial_number));
					}
					<OpenCollectionListings<T>>::remove(auction.collection_id, listing_id);

					Self::deposit_event(Event::<T>::AuctionClose {
						collection_id: auction.collection_id,
						listing_id,
						reason: AuctionClosureReason::VendorCancelled,
					});
				},
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
			let who = ensure_signed(origin)?;

			match Self::listings(listing_id) {
				Some(Listing::<T>::FixedPrice(mut sale)) => {
					ensure!(sale.seller == who, Error::<T>::NotSeller);

					sale.fixed_price = new_price;

					<Listings<T>>::insert(listing_id, Listing::<T>::FixedPrice(sale.clone()));
					Self::deposit_event(Event::<T>::FixedPriceSalePriceUpdate {
						collection_id: sale.collection_id,
						serial_numbers: sale.serial_numbers.into_inner(),
						listing_id,
						new_price,
					});
					Ok(())
				},
				_ => Err(Error::<T>::NotForFixedPriceSale.into()),
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
			let who = ensure_signed(origin)?;
			ensure!(!amount.is_zero(), Error::<T>::ZeroOffer);
			let collection_info =
				Self::collection_info(token_id.0).ok_or(Error::<T>::NoCollectionFound)?;
			ensure!(!collection_info.is_token_owner(&who, token_id.1), Error::<T>::IsTokenOwner);
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
			T::MultiCurrency::place_hold(T::PalletId::get(), &who, asset_id, amount)?;
			<TokenOffers<T>>::try_append(token_id, offer_id)
				.map_err(|_| Error::<T>::MaxOffersReached)?;
			let new_offer = OfferType::<T::AccountId>::Simple(SimpleOffer {
				token_id,
				asset_id,
				amount,
				buyer: who.clone(),
				marketplace_id,
			});
			<Offers<T>>::insert(offer_id, new_offer);
			<NextOfferId<T>>::mutate(|i| *i += 1);

			Self::deposit_event(Event::<T>::Offer {
				offer_id,
				amount,
				asset_id,
				marketplace_id,
				buyer: who,
			});
			Ok(())
		}

		/// Cancels an offer on a token
		/// Caller must be the offer buyer
		#[pallet::weight(T::WeightInfo::cancel_offer())]
		pub fn cancel_offer(origin: OriginFor<T>, offer_id: OfferId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let offer_type = Self::offers(offer_id).ok_or(Error::<T>::InvalidOffer)?;
			match offer_type {
				OfferType::Simple(offer) => {
					ensure!(offer.buyer == who, Error::<T>::NotBuyer);
					T::MultiCurrency::release_hold(
						T::PalletId::get(),
						&who,
						offer.asset_id,
						offer.amount,
					)?;
					let _ = Self::remove_offer(offer_id, offer.token_id)?;
					Self::deposit_event(Event::<T>::OfferCancel {
						offer_id,
						token_id: offer.token_id,
					});
					Ok(())
				},
			}
		}

		/// Accepts an offer on a token
		/// Caller must be token owner
		#[pallet::weight(T::WeightInfo::accept_offer())]
		#[transactional]
		pub fn accept_offer(origin: OriginFor<T>, offer_id: OfferId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let offer_type = Self::offers(offer_id).ok_or(Error::<T>::InvalidOffer)?;
			match offer_type {
				OfferType::Simple(offer) => {
					let (collection_id, serial_number) = offer.token_id;

					let royalties_schedule =
						Self::calculate_bundle_royalties(collection_id, offer.marketplace_id)?;
					let serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection> =
						BoundedVec::try_from(vec![serial_number])
							.map_err(|_| Error::<T>::TokenLimitExceeded)?;

					Self::process_payment_and_transfer(
						&offer.buyer,
						&who,
						offer.asset_id,
						collection_id,
						serial_numbers,
						offer.amount,
						royalties_schedule,
					)?;

					let _ = Self::remove_offer(offer_id, offer.token_id)?;
					Self::deposit_event(Event::<T>::OfferAccept {
						offer_id,
						token_id: offer.token_id,
						amount: offer.amount,
						asset_id: offer.asset_id,
					});
					Ok(())
				},
			}
		}
	}
}
