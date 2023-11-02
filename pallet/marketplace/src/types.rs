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

use crate::{Config, OpenCollectionListings};

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{dispatch::DispatchResult, traits::fungibles::Transfer, RuntimeDebugNoBound};
use pallet_nft::traits::NFTExt;
use pallet_sft::traits::SFTExt;
use scale_info::TypeInfo;
use seed_pallet_common::Hold;
use seed_primitives::{
	AssetId, Balance, BlockNumber, CollectionUuid, ListingId, RoyaltiesSchedule, SerialNumber,
	TokenId, TokenLockReason,
};
use sp_core::Get;
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

// /// The type of tokens included in a marketplace listing, used to specify the type of listing
// #[derive(Decode, Encode, RuntimeDebugNoBound, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
// #[scale_info(skip_type_params(T))]
// pub enum ListingTokens<T: Config> {
// 	Nft(BoundedVec<TokenId, T::MaxTokensPerListing>),
// 	Sft(BoundedVec<(TokenId, Balance), T::MaxTokensPerListing>),
// 	Bundle(TokenBundle<T>),
// }

// impl<T: Config> ListingTokens<T> {
// 	pub fn create_bundle(&self) -> TokenBundle<T> {
// 		match self {
// 			ListingTokens::Nft(nft_token_ids) =>
// 				TokenBundle::new(Some(nft_token_ids.clone()), None, None),
// 			ListingTokens::Sft(sft_token_balances) =>
// 				TokenBundle::new(None, Some(sft_token_balances.clone()), None),
// 			ListingTokens::Bundle(token_bundle) => token_bundle.clone(),
// 		}
// 	}
// }

// Wrapper type to allow for a bundle of NFT, SFT and asset tokens
#[derive(Decode, Encode, RuntimeDebugNoBound, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct TokenBundle<T: Config> {
	pub nft_token_ids: Option<BoundedVec<TokenId, <T as Config>::MaxTokensPerListing>>,
	pub sft_token_balances:
		Option<BoundedVec<(TokenId, Balance), <T as Config>::MaxTokensPerListing>>,
	pub asset_balances: Option<BoundedVec<(AssetId, Balance), <T as Config>::MaxTokensPerListing>>,
}

impl<T: Config> TokenBundle<T> {
	/// Creates a new bundle
	pub fn new(
		nft_token_ids: Option<BoundedVec<TokenId, <T as Config>::MaxTokensPerListing>>,
		sft_token_balances: Option<
			BoundedVec<(TokenId, Balance), <T as Config>::MaxTokensPerListing>,
		>,
		asset_balances: Option<BoundedVec<(AssetId, Balance), <T as Config>::MaxTokensPerListing>>,
	) -> Self {
		Self { nft_token_ids, sft_token_balances, asset_balances }
	}

	// Checks to see whether the token bundle contains at least some NFT or SFT tokens
	// Note: a bundle with just asset_ids does not count as having tokens that can be sold on the
	// marketplace
	pub fn has_tokens(&self) -> bool {
		let has_nfts = match &self.nft_token_ids {
			Some(nft_token_ids) => !nft_token_ids.is_empty(),
			None => false,
		};
		let has_sfts = match &self.sft_token_balances {
			Some(sft_token_balances) => !sft_token_balances.is_empty(),
			None => false,
		};
		has_nfts || has_sfts
	}

	// Lock all tokens within the bundle to prevent transfer while a listing is open
	pub fn lock_bundle_tokens(&self, listing_id: ListingId, who: &T::AccountId) -> DispatchResult {
		// Collect a list of all unique collection ids within the bundle
		let mut unique_collection_ids: Vec<CollectionUuid> = Vec::new();

		// Lock all NFT tokens in bundle
		if let Some(nft_token_ids) = &self.nft_token_ids {
			for token_id in nft_token_ids.iter() {
				T::NFTExt::set_token_lock(
					*token_id,
					Some(TokenLockReason::Listed(listing_id)),
					who.clone(),
				)?;
				// Add collection id to list of unique collection ids
				if !unique_collection_ids.contains(&token_id.0) {
					unique_collection_ids.push(token_id.0);
				}
			}
		}

		if let Some(sft_token_balances) = &self.sft_token_balances {
			let mut unique_collection_ids: Vec<CollectionUuid> = Vec::new();
			// Lock all SFT balances in bundle
			for (token_id, balance) in sft_token_balances.iter() {
				T::SFTExt::reserve_balance(*token_id, *balance, who)?;
				if !unique_collection_ids.contains(&token_id.0) {
					unique_collection_ids.push(token_id.0);
				}
			}
		}

		// Add all collection ids to open collection listings
		for collection_id in unique_collection_ids.iter() {
			<OpenCollectionListings<T>>::insert(collection_id, listing_id, true);
		}

		if let Some(asset_balances) = &self.asset_balances {
			// Place hold on all assets in bundle
			for (asset_id, balance) in asset_balances.iter() {
				T::MultiCurrency::place_hold(T::PalletId::get(), who, *asset_id, *balance)?;
			}
		}

		Ok(())
	}

	// Unlock all tokens within the bundle, called when a sale is cancelled or completed
	pub fn unlock_bundle_tokens(
		&self,
		who: &T::AccountId,
		listing_id: ListingId,
	) -> DispatchResult {
		// Collect a list of all unique collection ids within the bundle
		let mut unique_collection_ids: Vec<CollectionUuid> = Vec::new();

		// Unlock all NFT tokens in bundle
		if let Some(nft_token_ids) = &self.nft_token_ids {
			for token_id in nft_token_ids.iter() {
				T::NFTExt::set_token_lock(*token_id, None, who.clone())?;
				if !unique_collection_ids.contains(&token_id.0) {
					unique_collection_ids.push(token_id.0);
				}
			}
		}

		// Unlock all SFT balances in bundle
		if let Some(sft_token_balances) = &self.sft_token_balances {
			// Unlock all SFT balances in bundle
			for (token_id, balance) in sft_token_balances.iter() {
				T::SFTExt::free_reserved_balance(*token_id, *balance, who)?;
				if !unique_collection_ids.contains(&token_id.0) {
					unique_collection_ids.push(token_id.0);
				}
			}
		}

		// Remove all collection ids to open collection listings
		for collection_id in unique_collection_ids.iter() {
			<OpenCollectionListings<T>>::remove(collection_id, listing_id);
		}

		// Remove hold on all assets in bundle
		if let Some(asset_balances) = &self.asset_balances {
			// Remove hold on all assets in bundle
			for (asset_id, balance) in asset_balances.iter() {
				T::MultiCurrency::release_hold(T::PalletId::get(), who, *asset_id, *balance)?;
			}
		}
		Ok(())
	}

	pub fn transfer_bundle_tokens(
		&self,
		seller: &T::AccountId,
		buyer: &T::AccountId,
	) -> DispatchResult {
		if let Some(nft_token_ids) = &self.nft_token_ids {
			// Transfer all NFT tokens in bundle
			for token_id in nft_token_ids.iter() {
				T::NFTExt::do_transfer(*seller, token_id.0, vec![token_id.1], *buyer)?;
			}
		}

		if let Some(sft_token_balances) = &self.sft_token_balances {
			// Transfer all SFT balances in bundle
			for (token_id, balance) in sft_token_balances.iter() {
				let serial_numbers = BoundedVec::truncate_from(vec![(token_id.1, *balance)]);
				T::SFTExt::do_transfer(*seller, token_id.0, serial_numbers, *buyer)?;
			}
		}

		// Transfer all assets in bundle
		if let Some(asset_balances) = &self.asset_balances {
			// Remove hold on all assets in bundle
			for (asset_id, balance) in asset_balances.iter() {
				T::MultiCurrency::transfer(*asset_id, seller, buyer, *balance, false)?;
			}
		}
		Ok(())
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
	/// The tokens contained within the listing
	pub tokens: TokenBundle<T>,
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
	/// The tokens contained within the listing
	pub tokens: TokenBundle<T>,
	/// The royalties applicable to this sale
	pub royalties_schedule: RoyaltiesSchedule<T::AccountId>,
	/// The marketplace this is being sold on
	pub marketplace_id: Option<MarketplaceId>,
}
