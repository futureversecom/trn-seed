// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

//! Marketplace pallet types

use crate::{Config, Error};

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{dispatch::DispatchResult, ensure, RuntimeDebugNoBound};
use frame_system::pallet_prelude::BlockNumberFor;
use scale_info::TypeInfo;
use seed_pallet_common::{NFTExt, SFTExt};
use seed_primitives::{
	AssetId, Balance, BlockNumber, CollectionUuid, ListingId, RoyaltiesSchedule, SerialNumber,
	TokenId, TokenLockReason,
};
use sp_runtime::{BoundedVec, DispatchError, Permill};
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

/// The type of tokens included in a marketplace listing, used to specify the type of listing
#[derive(Decode, Encode, RuntimeDebugNoBound, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub enum ListingTokens<T: Config> {
	Nft(NftListing<T>),
	Sft(SftListing<T>),
}

// A group of NFT serial numbers from the same collection
#[derive(Decode, Encode, RuntimeDebugNoBound, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct NftListing<T: Config> {
	pub collection_id: CollectionUuid,
	pub serial_numbers: BoundedVec<SerialNumber, <T as Config>::MaxTokensPerListing>,
}

// A group of SFT serial numbers and balances from the same collection
#[derive(Decode, Encode, RuntimeDebugNoBound, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct SftListing<T: Config> {
	pub collection_id: CollectionUuid,
	pub serial_numbers: BoundedVec<(SerialNumber, Balance), <T as Config>::MaxTokensPerListing>,
}

impl<T: Config> ListingTokens<T> {
	/// Validates the listing tokens by checking the following:
	/// - Ensures the list of tokens is not empty
	/// - SFT tokens all have balance greater than 0
	pub fn validate(&self) -> DispatchResult {
		let serial_numbers: Vec<SerialNumber> = match self {
			ListingTokens::Nft(tokens) => {
				ensure!(!tokens.serial_numbers.is_empty(), Error::<T>::EmptyTokens);
				tokens.clone().serial_numbers.into_inner()
			},
			ListingTokens::Sft(tokens) => {
				ensure!(!tokens.serial_numbers.is_empty(), Error::<T>::EmptyTokens);
				// Ensure the balance is not zero for any token in the listing
				ensure!(
					!tokens.serial_numbers.iter().any(|(_, balance)| *balance == 0),
					Error::<T>::ZeroBalance
				);
				tokens.clone().serial_numbers.into_inner().iter().map(|(sn, _)| *sn).collect()
			},
		};

		let original_length = serial_numbers.len();
		let mut serial_numbers_trimmed = serial_numbers;
		serial_numbers_trimmed.sort_unstable();
		serial_numbers_trimmed.dedup();
		ensure!(serial_numbers_trimmed.len() == original_length, Error::<T>::DuplicateTokens);

		Ok(())
	}

	/// Returns the collection id of the first token in the listing
	pub fn get_collection_id(&self) -> CollectionUuid {
		match self {
			ListingTokens::Nft(tokens) => tokens.collection_id,
			ListingTokens::Sft(tokens) => tokens.collection_id,
		}
	}

	/// Locks a group of tokens before listing for sale
	/// Throws an error if owner does not own all tokens
	pub fn lock_tokens(&self, owner: &T::AccountId, listing_id: ListingId) -> DispatchResult {
		match self {
			ListingTokens::Nft(nfts) => {
				for serial_number in nfts.serial_numbers.iter() {
					let token_id = (nfts.collection_id, *serial_number);
					T::NFTExt::set_token_lock(
						token_id,
						TokenLockReason::Listed(listing_id),
						*owner,
					)?;
				}
			},
			ListingTokens::Sft(sfts) => {
				for (serial_number, balance) in sfts.serial_numbers.iter() {
					let token_id = (sfts.collection_id, *serial_number);
					T::SFTExt::reserve_balance(token_id, *balance, owner)?;
				}
			},
		}
		Ok(())
	}

	/// Removes all token locks and reservations for tokens included in a listing
	pub fn unlock_tokens(&self, owner: &T::AccountId) -> DispatchResult {
		match self {
			ListingTokens::Nft(nfts) => {
				for serial_number in nfts.serial_numbers.iter() {
					let token_id = (nfts.collection_id, *serial_number);
					T::NFTExt::remove_token_lock(token_id)?;
				}
			},
			ListingTokens::Sft(sfts) => {
				for (serial_number, balance) in sfts.serial_numbers.iter() {
					let token_id = (sfts.collection_id, *serial_number);
					T::SFTExt::free_reserved_balance(token_id, *balance, owner)?;
				}
			},
		}
		Ok(())
	}

	/// Unlock the locked tokens and transfer them immediately to the destination address
	/// Called at sale completion
	pub fn unlock_and_transfer(&self, from: &T::AccountId, to: &T::AccountId) -> DispatchResult {
		match self {
			ListingTokens::Nft(nfts) => {
				ensure!(!nfts.serial_numbers.is_empty(), Error::<T>::EmptyTokens);
				for serial_number in nfts.serial_numbers.iter() {
					T::NFTExt::remove_token_lock((nfts.collection_id, *serial_number))?;
				}
				T::NFTExt::do_transfer(
					from,
					nfts.collection_id,
					nfts.serial_numbers.clone().into_inner(),
					to,
				)?;
			},
			ListingTokens::Sft(sfts) => {
				for (serial_number, balance) in sfts.serial_numbers.iter() {
					let token_id = (sfts.collection_id, *serial_number);
					T::SFTExt::free_reserved_balance(token_id, *balance, from)?;
				}
				T::SFTExt::do_transfer(
					*from,
					sfts.collection_id,
					sfts.clone().serial_numbers.into_inner(),
					*to,
				)?;
			},
		}
		Ok(())
	}

	/// Get the royalties schedule for the collection if it exists
	/// Returns None if the collection has no royalties schedule
	/// Returns an error if the collection does not exist
	pub fn get_royalties_schedule(
		&self,
	) -> Result<Option<RoyaltiesSchedule<T::AccountId>>, DispatchError> {
		match self {
			ListingTokens::Nft(tokens) => T::NFTExt::get_royalties_schedule(tokens.collection_id),
			ListingTokens::Sft(tokens) => T::SFTExt::get_royalties_schedule(tokens.collection_id),
		}
	}
}

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
	/// Vendor accepted a buy offer
	OfferAccepted,
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
	pub close: BlockNumberFor<T>,
	/// The seller of the tokens
	pub seller: T::AccountId,
	/// The tokens contained within the listing
	pub tokens: ListingTokens<T>,
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
	/// The requested amount for a successful sale
	pub fixed_price: Balance,
	/// When the listing closes
	pub close: BlockNumberFor<T>,
	/// The authorised buyer. If unset, any buyer is authorised
	pub buyer: Option<T::AccountId>,
	/// The seller of the tokens
	pub seller: T::AccountId,
	/// The tokens contained within the listing
	pub tokens: ListingTokens<T>,
	/// The royalties applicable to this sale
	pub royalties_schedule: RoyaltiesSchedule<T::AccountId>,
	/// The marketplace this is being sold on
	pub marketplace_id: Option<MarketplaceId>,
}
