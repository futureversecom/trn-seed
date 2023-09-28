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
//! # Marketplace Module
//!
//! Provides marketplace functionality for NFT and SFT pallets
//!
//! Allows users to buy or sell tokens, register as a marketplace and distribute royalties
//! per sale.
//! Also allows for offers on these tokens, which can be accepted by the owner of the token.

use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::fungibles::{Mutate, Transfer},
	transactional, PalletId,
};
pub use pallet::*;
use pallet_nft::traits::NFTExt;
use seed_pallet_common::{CreateExt, Hold, TransferExt};
use seed_primitives::{
	AccountId, AssetId, Balance, CollectionUuid, ListingId, SerialNumber, TokenId, TokenLockReason,
};
use sp_runtime::{DispatchResult, Permill};
use sp_std::vec::Vec;

mod benchmarking;
mod impls;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;
pub mod types;
use types::*;
pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::{DispatchResult, *};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::AccountIdConversion;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
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
			NextMarketplaceId::<T>::put(1 as MarketplaceId);
			NextListingId::<T>::put(1 as ListingId);
			NextOfferId::<T>::put(1 as OfferId);
		}
	}

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		/// The overarching call type.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin, PostInfo = PostDispatchInfo>
			+ GetDispatchInfo;
		/// Default auction / sale length in blocks
		#[pallet::constant]
		type DefaultListingDuration: Get<Self::BlockNumber>;
		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The default account which collects network fees from marketplace sales
		#[pallet::constant]
		type DefaultFeeTo: Get<Option<PalletId>>;
		/// Handles a multi-currency fungible asset system
		type MultiCurrency: TransferExt<AccountId = Self::AccountId>
			+ Hold<AccountId = Self::AccountId>
			+ Mutate<Self::AccountId, AssetId = AssetId>
			+ CreateExt<AccountId = Self::AccountId>
			+ Transfer<Self::AccountId, Balance = Balance>;
		/// NFT Extension, used to retrieve nextCollectionUuid
		type NFTExt: NFTExt<AccountId = Self::AccountId>;
		/// This pallet's Id, used for deriving a sovereign account ID
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// Percentage of sale price to charge for network fee
		type NetworkFeePercentage: Get<Permill>;
		/// Provides the public call to weight mapping
		type WeightInfo: WeightInfo;
		/// Max tokens that can be sold in one listing
		type MaxTokensPerListing: Get<u32>;
		/// The maximum number of offers allowed on a collection
		type MaxOffers: Get<u32>;
	}

	#[pallet::type_value]
	pub fn DefaultFeeTo<T: Config>() -> Option<T::AccountId> {
		T::DefaultFeeTo::get().map(|v| v.into_account_truncating())
	}

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

	/// The pallet id for the tx fee pot
	#[pallet::storage]
	pub type FeeTo<T: Config> = StorageValue<_, Option<T::AccountId>, ValueQuery, DefaultFeeTo<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
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
		/// The network fee receiver address has been updated
		FeeToSet { account: Option<T::AccountId> },
	}

	#[pallet::error]
	pub enum Error<T> {
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
		/// Cannot make an offer on a token up for auction
		TokenOnAuction,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Check and close all expired listings
		fn on_initialize(now: T::BlockNumber) -> Weight {
			// TODO: this is unbounded and could become costly
			// https://github.com/cennznet/cennznet/issues/444
			let removed_count = Self::close_listings_at(now);
			// 'buy' weight is comparable to successful closure of an auction
			<T as Config>::WeightInfo::buy().mul(removed_count as u64)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
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
			Self::do_register_marketplace(who, marketplace_account, entitlement)?;
			Ok(().into())
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
		pub fn sell_nft(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerListing>,
			buyer: Option<T::AccountId>,
			payment_asset: AssetId,
			fixed_price: Balance,
			duration: Option<T::BlockNumber>,
			marketplace_id: Option<MarketplaceId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_sell_nft(
				who,
				collection_id,
				serial_numbers,
				buyer,
				payment_asset,
				fixed_price,
				duration,
				marketplace_id,
			)?;
			Ok(().into())
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
			Self::do_update_fixed_price(who, listing_id, new_price)
		}

		/// Buy a token listing for its specified price
		#[pallet::weight(T::WeightInfo::buy())]
		#[transactional]
		pub fn buy(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_buy(who, listing_id)?;
			Ok(().into())
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
		pub fn auction_nft(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerListing>,
			payment_asset: AssetId,
			reserve_price: Balance,
			duration: Option<T::BlockNumber>,
			marketplace_id: Option<MarketplaceId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_auction_nft(
				who,
				collection_id,
				serial_numbers,
				payment_asset,
				reserve_price,
				duration,
				marketplace_id,
			)?;
			Ok(().into())
		}

		/// Place a bid on an open auction
		/// - `amount` to bid (in the seller's requested payment asset)
		#[pallet::weight(T::WeightInfo::bid())]
		#[transactional]
		pub fn bid(origin: OriginFor<T>, listing_id: ListingId, amount: Balance) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_bid(who, listing_id, amount)
		}

		/// Close a sale or auction returning tokens
		/// Requires no successful bids have been made for an auction.
		/// Caller must be the listed seller
		#[pallet::weight(T::WeightInfo::cancel_sale())]
		#[transactional]
		pub fn cancel_sale(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_cancel_sale(who, listing_id)
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
			Self::do_make_simple_offer(who, token_id, amount, asset_id, marketplace_id)?;
			Ok(().into())
		}

		/// Cancels an offer on a token
		/// Caller must be the offer buyer
		#[pallet::weight(T::WeightInfo::cancel_offer())]
		pub fn cancel_offer(origin: OriginFor<T>, offer_id: OfferId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_cancel_offer(who, offer_id)
		}

		/// Accepts an offer on a token
		/// Caller must be token owner
		#[pallet::weight(T::WeightInfo::accept_offer())]
		#[transactional]
		pub fn accept_offer(origin: OriginFor<T>, offer_id: OfferId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_accept_offer(who, offer_id)
		}

		/// Set the `FeeTo` account
		/// This operation requires root access
		#[pallet::weight(T::WeightInfo::set_fee_to())]
		pub fn set_fee_to(origin: OriginFor<T>, fee_to: Option<T::AccountId>) -> DispatchResult {
			ensure_root(origin)?;
			Self::do_set_fee_to(fee_to)
		}
	}
}
