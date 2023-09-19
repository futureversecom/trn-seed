// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

//! Marketplace pallet types

use crate::Config;

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use seed_primitives::{
	AssetId, Balance, BlockNumber, CollectionUuid, RoyaltiesSchedule, SerialNumber, TokenId,
};
use sp_runtime::{BoundedVec, Permill};
use sp_std::prelude::*;

/// The logging target for this module
pub(crate) const LOG_TARGET: &str = "marketplace";

// Time before auction ends that auction is extended if a bid is placed
pub const AUCTION_EXTENSION_PERIOD: BlockNumber = 40;

/// OfferId type used to distinguish different offers on NFTs
pub type OfferId = u64;

/// Auto-incrementing Uint
/// Uniquely identifies a registered marketplace
pub type MarketplaceId = u32;

/// Holds information relating to NFT offers
#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo, MaxEncodedLen)]
pub struct SimpleOffer<AccountId> {
	pub token_id: TokenId,
	pub asset_id: AssetId,
	pub amount: Balance,
	pub buyer: AccountId,
	pub marketplace_id: Option<MarketplaceId>,
}

#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound(AccountId: MaxEncodedLen))]
pub enum OfferType<AccountId> {
	Simple(SimpleOffer<AccountId>),
}

/// Reasons for an auction closure
#[derive(Decode, Encode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub enum AuctionClosureReason {
	/// Auction expired with no bids
	ExpiredNoBids,
	/// Auction should have happened but settlement failed due to payment issues
	SettlementFailed,
	/// Auction was cancelled by the vendor
	VendorCancelled,
}

/// Reason for a fixed price closure
#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo)]
pub enum FixedPriceClosureReason {
	/// Listing was cancelled by the vendor
	VendorCancelled,
	/// Listing expired
	Expired,
}

/// Information about a marketplace
#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub struct Marketplace<AccountId> {
	/// The marketplace account
	pub account: AccountId,
	/// Royalties to go to the marketplace
	pub entitlement: Permill,
}

/// A type of NFT sale listing
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub enum Listing<T: Config> {
	FixedPrice(FixedPriceListing<T>),
	Auction(AuctionListing<T>),
}

/// Information about an auction listing
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct AuctionListing<T: Config> {
	/// The asset to allow bids with
	pub payment_asset: AssetId,
	/// The threshold amount for a successful bid
	pub reserve_price: Balance,
	/// When the listing closes
	pub close: T::BlockNumber,
	/// The seller of the tokens
	pub seller: T::AccountId,
	/// The listing collection id
	pub collection_id: CollectionUuid,
	/// The serial numbers for sale in this listing
	pub serial_numbers: BoundedVec<SerialNumber, <T as Config>::MaxTokensPerListing>,
	/// The royalties applicable to this auction
	pub royalties_schedule: RoyaltiesSchedule<T::AccountId>,
	/// The marketplace this is being sold on
	pub marketplace_id: Option<MarketplaceId>,
}

/// Information about a fixed price listing
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct FixedPriceListing<T: Config> {
	/// The asset to allow bids with
	pub payment_asset: AssetId,
	/// The requested amount for a succesful sale
	pub fixed_price: Balance,
	/// When the listing closes
	pub close: T::BlockNumber,
	/// The authorised buyer. If unset, any buyer is authorised
	pub buyer: Option<T::AccountId>,
	/// The seller of the tokens
	pub seller: T::AccountId,
	/// The listing collection id
	pub collection_id: CollectionUuid,
	/// The serial numbers for sale in this listing
	pub serial_numbers: BoundedVec<SerialNumber, <T as Config>::MaxTokensPerListing>,
	/// The royalties applicable to this sale
	pub royalties_schedule: RoyaltiesSchedule<T::AccountId>,
	/// The marketplace this is being sold on
	pub marketplace_id: Option<MarketplaceId>,
}
