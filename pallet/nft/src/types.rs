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

//! NFT module types

use crate::{Config, Error};

use codec::{Decode, Encode};
use frame_support::dispatch::DispatchResult;
use scale_info::TypeInfo;
use seed_primitives::{
	AssetId, Balance, BlockNumber, CollectionUuid, MetadataScheme, SerialNumber, TokenCount,
	TokenId,
};
use sp_runtime::{BoundedVec, PerThing, Permill};
use sp_std::prelude::*;

/// The max. number of entitlements any royalties schedule can have
/// just a sensible upper bound
pub(crate) const MAX_ENTITLEMENTS: usize = 8;

// Time before auction ends that auction is extended if a bid is placed
pub const AUCTION_EXTENSION_PERIOD: BlockNumber = 40;

/// OfferId type used to distinguish different offers on NFTs
pub type OfferId = u64;

/// Holds information relating to NFT offers
#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo)]
pub struct SimpleOffer<AccountId> {
	pub token_id: TokenId,
	pub asset_id: AssetId,
	pub amount: Balance,
	pub buyer: AccountId,
	pub marketplace_id: Option<MarketplaceId>,
}

#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo)]
pub enum OfferType<AccountId> {
	Simple(SimpleOffer<AccountId>),
}

#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo)]
/// Describes the chain that the bridged resource originated from
pub enum OriginChain {
	Ethereum,
	Root,
}

/// Struct that represents the owned serial numbers within a collection of an individual account
#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct TokenOwnership<T: Config> {
	pub owner: T::AccountId,
	pub owned_serials: BoundedVec<SerialNumber, <T as Config>::MaxTokensPerCollection>,
}

impl<T: Config> TokenOwnership<T> {
	/// Creates a new TokenOwnership with the given owner and serial numbers
	pub fn new(
		owner: T::AccountId,
		serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection>,
	) -> Self {
		let mut owned_serials = serial_numbers.clone();
		owned_serials.sort();
		Self { owner, owned_serials }
	}

	/// Adds a serial to owned_serials and sorts the vec
	pub fn add(&mut self, serial_number: SerialNumber) -> DispatchResult {
		self.owned_serials
			.try_push(serial_number)
			.map_err(|_| Error::<T>::TokenLimitExceeded)?;
		self.owned_serials.sort();
		Ok(())
	}

	/// Returns true if the serial number is containerd within owned_serials
	pub fn contains_serial(&self, serial_number: &SerialNumber) -> bool {
		self.owned_serials.contains(serial_number)
	}
}

/// Determines compatibility with external chains.
/// If compatible with XRPL, XLS-20 tokens will be minted with every newly minted
/// token on The Root Network
#[derive(Debug, Clone, Encode, Decode, PartialEq, TypeInfo, Copy)]
pub struct CrossChainCompatibility {
	/// This collection is compatible with the XLS-20 standard on XRPL
	pub xrpl: bool,
}

impl Default for CrossChainCompatibility {
	fn default() -> Self {
		Self { xrpl: false }
	}
}

/// Information related to a specific collection
#[derive(Debug, Clone, Encode, Decode, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct CollectionInformation<T: Config> {
	/// The owner of the collection
	pub owner: T::AccountId,
	/// A human friendly name
	pub name: CollectionNameType,
	/// Collection metadata reference scheme
	pub metadata_scheme: MetadataScheme,
	/// configured royalties schedule
	pub royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
	/// Maximum number of tokens allowed in a collection
	pub max_issuance: Option<TokenCount>,
	/// The chain in which the collection was minted originally
	pub origin_chain: OriginChain,
	/// The next available serial_number
	pub next_serial_number: SerialNumber,
	/// the total count of tokens in this collection
	pub collection_issuance: TokenCount,
	/// This collections compatibility with other chains
	pub cross_chain_compatibility: CrossChainCompatibility,
	/// All serial numbers owned by an account in a collection
	pub owned_tokens: BoundedVec<TokenOwnership<T>, <T as Config>::MaxTokensPerCollection>,
}

impl<T: Config> CollectionInformation<T> {
	/// Check whether a token has been minted in a collection
	pub fn token_exists(&self, serial_number: SerialNumber) -> bool {
		self.owned_tokens
			.iter()
			.any(|token_ownership| token_ownership.contains_serial(&serial_number))
	}

	/// Check whether who is the collection owner
	pub fn is_collection_owner(&self, who: &T::AccountId) -> bool {
		&self.owner == who
	}

	/// Check whether who owns the serial number in collection_info
	pub fn is_token_owner(&self, who: &T::AccountId, serial_number: SerialNumber) -> bool {
		self.owned_tokens.iter().any(|token_ownership| {
			if &token_ownership.owner == who {
				token_ownership.contains_serial(&serial_number)
			} else {
				false
			}
		})
	}

	/// Get's the token owner
	pub fn get_token_owner(&self, serial_number: SerialNumber) -> Option<T::AccountId> {
		let Some(token) = self.owned_tokens.iter().find(|x| x.contains_serial(&serial_number)) else {
			return None
		};
		Some(token.owner.clone())
	}

	/// Adds a list of tokens to a users balance in collection_info
	pub fn add_user_tokens(
		&mut self,
		token_owner: &T::AccountId,
		serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection>,
	) -> DispatchResult {
		if self
			.owned_tokens
			.iter()
			.any(|token_ownership| &token_ownership.owner == token_owner)
		{
			for token_ownership in self.owned_tokens.iter_mut() {
				if &token_ownership.owner != token_owner {
					continue
				}
				// Add new serial numbers to existing owner
				for serial_number in serial_numbers.iter() {
					token_ownership.add(*serial_number)?;
				}
			}
		} else {
			// If token owner doesn't exist, create new entry
			let new_token_ownership = TokenOwnership::new(token_owner.clone(), serial_numbers);
			self.owned_tokens
				.try_push(new_token_ownership)
				.map_err(|_| Error::<T>::TokenLimitExceeded)?;
		}
		Ok(())
	}

	/// Removes a list of tokens from a users balance in collection_info
	pub fn remove_user_tokens(
		&mut self,
		token_owner: &T::AccountId,
		serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection>,
	) {
		let mut removing_all_tokens: bool = false;
		for token_ownership in self.owned_tokens.iter_mut() {
			if &token_ownership.owner != token_owner {
				continue
			}
			token_ownership.owned_serials.retain(|serial| !serial_numbers.contains(serial));
			removing_all_tokens = token_ownership.owned_serials.is_empty();
			break
		}
		// Check whether the owner has any tokens left, if not remove them from the collection
		if removing_all_tokens {
			self.owned_tokens
				.retain(|token_ownership| &token_ownership.owner != token_owner);
		}
	}
}

/// Reason for an NFT being locked (un-transferrable)
#[derive(Decode, Encode, Debug, Clone, Eq, PartialEq, TypeInfo)]
pub enum TokenLockReason {
	/// Token is listed for sale
	Listed(ListingId),
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

/// Describes the royalty scheme for secondary sales for an NFT collection/token
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub struct RoyaltiesSchedule<AccountId> {
	/// Entitlements on all secondary sales, (beneficiary, % of sale price)
	pub entitlements: Vec<(AccountId, Permill)>,
}

impl<AccountId> RoyaltiesSchedule<AccountId> {
	/// True if entitlements are within valid parameters
	/// - not overcommitted (> 100%)
	/// - < MAX_ENTITLEMENTS
	pub fn validate(&self) -> bool {
		!self.entitlements.is_empty() &&
			self.entitlements.len() <= MAX_ENTITLEMENTS &&
			self.entitlements
				.iter()
				.map(|(_who, share)| share.deconstruct() as u32)
				.sum::<u32>() <= Permill::ACCURACY
	}
	/// Calculate the total % entitled for royalties
	/// It will return `0` if the `entitlements` are overcommitted
	pub fn calculate_total_entitlement(&self) -> Permill {
		// if royalties are in a strange state
		if !self.validate() {
			return Permill::zero()
		}
		Permill::from_parts(
			self.entitlements.iter().map(|(_who, share)| share.deconstruct()).sum::<u32>(),
		)
	}
}

impl<AccountId> Default for RoyaltiesSchedule<AccountId> {
	fn default() -> Self {
		Self { entitlements: vec![] }
	}
}

/// Information about a marketplace
#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub struct Marketplace<AccountId> {
	/// The marketplace account
	pub account: AccountId,
	/// Royalties to go to the marketplace
	pub entitlement: Permill,
}

/// A type of NFT sale listing
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub enum Listing<T: Config> {
	FixedPrice(FixedPriceListing<T>),
	Auction(AuctionListing<T>),
}

/// Information about an auction listing
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
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
	pub serial_numbers: BoundedVec<SerialNumber, <T as Config>::MaxTokensPerCollection>,
	/// The royalties applicable to this auction
	pub royalties_schedule: RoyaltiesSchedule<T::AccountId>,
	/// The marketplace this is being sold on
	pub marketplace_id: Option<MarketplaceId>,
}

/// Information about a fixed price listing
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
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
	pub serial_numbers: BoundedVec<SerialNumber, <T as Config>::MaxTokensPerCollection>,
	/// The royalties applicable to this sale
	pub royalties_schedule: RoyaltiesSchedule<T::AccountId>,
	/// The marketplace this is being sold on
	pub marketplace_id: Option<MarketplaceId>,
}

/// NFT collection moniker
pub type CollectionNameType = Vec<u8>;

/// Auto-incrementing Uint
/// Uniquely identifies a registered marketplace
pub type MarketplaceId = u32;

/// Unique Id for a listing
pub type ListingId = u128;

#[cfg(test)]
mod test {
	use super::RoyaltiesSchedule;
	use sp_runtime::Permill;

	#[test]
	fn valid_royalties_plan() {
		assert!(RoyaltiesSchedule::<u32> { entitlements: vec![(1_u32, Permill::from_float(0.1))] }
			.validate());

		// explicitally specifying zero royalties is odd but fine
		assert!(RoyaltiesSchedule::<u32> { entitlements: vec![(1_u32, Permill::from_float(0.0))] }
			.validate());

		let plan = RoyaltiesSchedule::<u32> {
			entitlements: vec![
				(1_u32, Permill::from_float(1.01)), // saturates at 100%
			],
		};
		assert_eq!(plan.entitlements[0].1, Permill::one());
		assert!(plan.validate());
	}

	#[test]
	fn invalid_royalties_plan() {
		// overcommits > 100% to royalties
		assert!(!RoyaltiesSchedule::<u32> {
			entitlements: vec![
				(1_u32, Permill::from_float(0.2)),
				(2_u32, Permill::from_float(0.81)),
			],
		}
		.validate());
	}
}
