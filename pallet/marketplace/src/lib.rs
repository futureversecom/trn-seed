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
	traits::fungibles::Mutate,
	transactional, PalletId,
};
pub use pallet::*;
use seed_pallet_common::{CreateExt, Hold, NFTExt, SFTExt, TransferExt};
use seed_primitives::{
	AccountId, AssetId, Balance, CollectionUuid, ListingId, SerialNumber, TokenId, TokenLockReason,
};
use sp_runtime::{DispatchResult, Permill};

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
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		_phantom: sp_std::marker::PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { _phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
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
		type DefaultListingDuration: Get<BlockNumberFor<Self>>;
		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The default account which collects network fees from marketplace sales
		#[pallet::constant]
		type DefaultFeeTo: Get<Option<PalletId>>;
		/// Handles a multi-currency fungible asset system
		type MultiCurrency: TransferExt<AccountId = Self::AccountId>
			+ Hold<AccountId = Self::AccountId>
			+ Mutate<Self::AccountId, AssetId = AssetId, Balance = Balance>
			+ CreateExt<AccountId = Self::AccountId>;
		/// NFT Extension
		type NFTExt: NFTExt<AccountId = Self::AccountId>;
		/// SFT Extension
		type SFTExt: SFTExt<AccountId = Self::AccountId>;
		/// This pallet's Id, used for deriving a sovereign account ID
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// Percentage of sale price to charge for network fee
		type NetworkFeePercentage: Get<Permill>;
		/// Provides the public call to weight mapping
		type WeightInfo: WeightInfo;
		/// Max tokens that can be sold in one listing
		type MaxTokensPerListing: Get<u32>;
		/// Max listings per single multi_buy call
		type MaxListingsPerMultiBuy: Get<u32>;
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
		StorageDoubleMap<_, Twox64Concat, BlockNumberFor<T>, Twox64Concat, ListingId, bool>;

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
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A fixed price sale has been listed
		FixedPriceSaleList {
			tokens: ListingTokens<T>,
			listing_id: ListingId,
			marketplace_id: Option<MarketplaceId>,
			price: Balance,
			payment_asset: AssetId,
			seller: T::AccountId,
			close: BlockNumberFor<T>,
		},
		/// A fixed price sale has completed
		FixedPriceSaleComplete {
			tokens: ListingTokens<T>,
			listing_id: ListingId,
			marketplace_id: Option<MarketplaceId>,
			price: Balance,
			payment_asset: AssetId,
			buyer: T::AccountId,
			seller: T::AccountId,
		},
		/// A fixed price sale has closed without selling
		FixedPriceSaleClose {
			tokens: ListingTokens<T>,
			listing_id: ListingId,
			marketplace_id: Option<MarketplaceId>,
			reason: FixedPriceClosureReason,
		},
		/// A fixed price sale has had its price updated
		FixedPriceSalePriceUpdate {
			tokens: ListingTokens<T>,
			listing_id: ListingId,
			marketplace_id: Option<MarketplaceId>,
			new_price: Balance,
		},
		/// An auction has opened
		AuctionOpen {
			tokens: ListingTokens<T>,
			listing_id: ListingId,
			marketplace_id: Option<MarketplaceId>,
			payment_asset: AssetId,
			reserve_price: Balance,
			seller: T::AccountId,
			close: BlockNumberFor<T>,
		},
		/// An auction has sold
		AuctionSold {
			tokens: ListingTokens<T>,
			listing_id: ListingId,
			marketplace_id: Option<MarketplaceId>,
			payment_asset: AssetId,
			hammer_price: Balance,
			winner: T::AccountId,
		},
		/// An auction has closed without selling
		AuctionClose {
			tokens: ListingTokens<T>,
			listing_id: ListingId,
			marketplace_id: Option<MarketplaceId>,
			reason: AuctionClosureReason,
		},
		/// A new highest bid was placed
		Bid {
			tokens: ListingTokens<T>,
			listing_id: ListingId,
			marketplace_id: Option<MarketplaceId>,
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
		OfferCancel { offer_id: OfferId, marketplace_id: Option<MarketplaceId>, token_id: TokenId },
		/// An offer has been accepted
		OfferAccept {
			offer_id: OfferId,
			marketplace_id: Option<MarketplaceId>,
			token_id: TokenId,
			amount: Balance,
			asset_id: AssetId,
		},
		/// The network fee receiver address has been updated
		FeeToSet { account: Option<T::AccountId> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// No more Ids are available, they've been exhausted
		NoAvailableIds,
		/// Origin does not own the NFT
		NotTokenOwner,
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
		/// The balance of tokens within the listing must be greater than zero
		ZeroBalance,
		/// Cannot make an offer on a token up for auction
		TokenOnAuction,
		/// No tokens were specified in the listing
		EmptyTokens,
		/// The token does not exist
		NoToken,
		/// The listing duration is too short
		DurationTooShort,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Check and close all expired listings
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
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
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::register_marketplace())]
		pub fn register_marketplace(
			origin: OriginFor<T>,
			marketplace_account: Option<T::AccountId>,
			entitlement: Permill,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_register_marketplace(who, marketplace_account, entitlement)?;
			Ok(())
		}

		/// Deprecated, use `sell` instead
		/// Sell a bundle of tokens at a fixed price
		/// - Tokens must be from the same collection
		/// - Tokens with individual royalties schedules cannot be sold with this method
		///
		/// `buyer` optionally, the account to receive the NFT. If unspecified, then any account may
		/// purchase `asset_id` fungible asset Id to receive as payment for the NFT
		/// `fixed_price` ask price
		/// `duration` listing duration time in blocks from now
		/// Caller must be the token owner
		#[pallet::call_index(1)]
		#[pallet::weight({
			T::WeightInfo::sell_nft(serial_numbers.len() as u32)
		})]
		pub fn sell_nft(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerListing>,
			buyer: Option<T::AccountId>,
			payment_asset: AssetId,
			fixed_price: Balance,
			duration: Option<BlockNumberFor<T>>,
			marketplace_id: Option<MarketplaceId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let tokens = ListingTokens::Nft(NftListing { collection_id, serial_numbers });
			Self::do_sell(
				who,
				tokens,
				buyer,
				payment_asset,
				fixed_price,
				duration,
				marketplace_id,
			)?;
			Ok(())
		}

		/// Sell a bundle of SFTs or NFTs at a fixed price
		/// - Tokens must be from the same collection
		/// - Tokens with individual royalties schedules cannot be sold with this method
		///
		/// `buyer` optionally, the account to receive the tokens. If unspecified, then any account
		/// may purchase `asset_id` fungible asset Id to receive as payment for the NFT
		/// `fixed_price` ask price
		/// `duration` listing duration time in blocks from now
		/// Caller must be the token owner
		#[pallet::call_index(2)]
		#[pallet::weight({
			match &tokens {
				ListingTokens::Nft(nft) => T::WeightInfo::sell_nft(nft.serial_numbers.len() as u32),
				ListingTokens::Sft(sft) => T::WeightInfo::sell_sft(sft.serial_numbers.len() as u32),
			}
		})]
		pub fn sell(
			origin: OriginFor<T>,
			tokens: ListingTokens<T>,
			buyer: Option<T::AccountId>,
			payment_asset: AssetId,
			fixed_price: Balance,
			duration: Option<BlockNumberFor<T>>,
			marketplace_id: Option<MarketplaceId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_sell(
				who,
				tokens,
				buyer,
				payment_asset,
				fixed_price,
				duration,
				marketplace_id,
			)?;
			Ok(())
		}

		/// Update fixed price for a single token sale
		///
		/// `listing_id` id of the fixed price listing
		/// `new_price` new fixed price
		/// Caller must be the token owner
		#[pallet::call_index(3)]
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
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::buy())]
		#[transactional]
		pub fn buy(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_buy(who, listing_id)?;
			Ok(())
		}

		/// Buy multiple listings, each for their respective price
		#[pallet::call_index(5)]
		#[pallet::weight({
			T::WeightInfo::buy_multi(listing_ids.len() as u32)
		})]
		#[transactional]
		pub fn buy_multi(
			origin: OriginFor<T>,
			listing_ids: BoundedVec<ListingId, T::MaxListingsPerMultiBuy>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			for listing_id in listing_ids.iter() {
				Self::do_buy(who, *listing_id)?;
			}
			Ok(())
		}

		/// Deprecated, use `auction` instead
		/// Auction a bundle of tokens on the open market to the highest bidder
		///
		/// - Tokens must be from the same collection
		/// - Tokens with individual royalties schedules cannot be sold in bundles
		/// - `payment_asset` fungible asset Id to receive payment with
		/// - `reserve_price` winning bid must be over this threshold
		/// - `duration` length of the auction (in blocks), uses default duration if unspecified
		/// Caller must be the token owner
		#[pallet::call_index(6)]
		#[pallet::weight({
			T::WeightInfo::auction_nft(serial_numbers.len() as u32)
		})]
		pub fn auction_nft(
			origin: OriginFor<T>,
			collection_id: CollectionUuid,
			serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerListing>,
			payment_asset: AssetId,
			reserve_price: Balance,
			duration: Option<BlockNumberFor<T>>,
			marketplace_id: Option<MarketplaceId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let tokens = ListingTokens::Nft(NftListing { collection_id, serial_numbers });
			Self::do_auction(who, tokens, payment_asset, reserve_price, duration, marketplace_id)?;
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
		#[pallet::call_index(7)]
		#[pallet::weight({
			match &tokens {
				ListingTokens::Nft(nft) => T::WeightInfo::auction_nft(nft.serial_numbers.len() as u32),
				ListingTokens::Sft(sft) => T::WeightInfo::auction_sft(sft.serial_numbers.len() as u32),
			}
		})]
		pub fn auction(
			origin: OriginFor<T>,
			tokens: ListingTokens<T>,
			payment_asset: AssetId,
			reserve_price: Balance,
			duration: Option<BlockNumberFor<T>>,
			marketplace_id: Option<MarketplaceId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_auction(who, tokens, payment_asset, reserve_price, duration, marketplace_id)?;
			Ok(())
		}

		/// Place a bid on an open auction
		/// - `amount` to bid (in the seller's requested payment asset)
		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::bid())]
		#[transactional]
		pub fn bid(origin: OriginFor<T>, listing_id: ListingId, amount: Balance) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_bid(who, listing_id, amount)
		}

		/// Close a sale or auction returning tokens
		/// Requires no successful bids have been made for an auction.
		/// Caller must be the listed seller
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::cancel_sale())]
		#[transactional]
		pub fn cancel_sale(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_cancel_sale(who, listing_id)
		}

		/// Create an offer on a single NFT
		/// Locks funds until offer is accepted, rejected or cancelled
		/// An offer can't be made on a token currently in an auction
		/// (This follows the behaviour of Opensea and forces the buyer to bid rather than create an
		/// offer)
		#[pallet::call_index(10)]
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
			Ok(())
		}

		/// Cancels an offer on a token
		/// Caller must be the offer buyer
		#[pallet::call_index(11)]
		#[pallet::weight(T::WeightInfo::cancel_offer())]
		pub fn cancel_offer(origin: OriginFor<T>, offer_id: OfferId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_cancel_offer(who, offer_id)
		}

		/// Accepts an offer on a token
		/// Caller must be token owner
		#[pallet::call_index(12)]
		#[pallet::weight(T::WeightInfo::accept_offer())]
		#[transactional]
		pub fn accept_offer(origin: OriginFor<T>, offer_id: OfferId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_accept_offer(who, offer_id)
		}

		/// Set the `FeeTo` account
		/// This operation requires root access
		#[pallet::call_index(13)]
		#[pallet::weight(T::WeightInfo::set_fee_to())]
		pub fn set_fee_to(origin: OriginFor<T>, fee_to: Option<T::AccountId>) -> DispatchResult {
			ensure_root(origin)?;
			Self::do_set_fee_to(fee_to)
		}
	}
}
