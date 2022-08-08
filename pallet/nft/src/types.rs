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

//! NFT module types

use crate::Config;

use root_primitives::{AssetId, Balance, BlockNumber, SerialNumber, TokenId};

use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize, Serializer};
use sp_runtime::{PerThing, Permill};
use sp_std::prelude::*;

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

// Information related to a specific collection
#[derive(Debug, Clone, Encode, Decode, PartialEq, TypeInfo)]
pub struct CollectionInformation<AccountId> {
	// The owner of the collection
	pub owner: AccountId,
	// A human friendly name
	pub name: CollectionNameType,
	// Collection metadata reference scheme
	pub metadata_scheme: MetadataScheme,
	// configured royalties schedule
	pub royalties_schedule: Option<RoyaltiesSchedule<AccountId>>,
	// Maximum number of tokens allowed in a collection
	pub max_issuance: Option<TokenCount>,
}

/// Denotes the metadata URI referencing scheme used by a collection
/// Enable token metadata URI construction by clients
#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo)]
pub enum MetadataScheme {
	/// Collection metadata is hosted by an HTTPS server
	/// Inner value is the URI without protocol prefix 'https://' or trailing '/'
	/// full metadata URI construction: `https://<domain>/<path+>/<serial_number>.json`
	/// Https(b"example.com/metadata")
	Https(Vec<u8>),
	/// Collection metadata is hosted by an unsecured HTTP server
	/// Inner value is the URI without protocol prefix 'http://' or trailing '/'
	/// full metadata URI construction: `https://<domain>/<path+>/<serial_number>.json`
	/// Https(b"example.com/metadata")
	Http(Vec<u8>),
	/// Collection metadata is hosted by an IPFS directory
	/// Inner value is the directory's IPFS CID
	/// full metadata URI construction: `ipfs://<directory_CID>/<serial_number>.json`
	/// IpfsDir(b"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi")
	IpfsDir(Vec<u8>),
	/// Collection metadata is hosted by an IPFS directory
	/// Inner value is the shared IPFS CID, each token in the collection shares the same CID
	/// full metadata URI construction: `ipfs://<shared_file_CID>.json`
	IpfsShared(Vec<u8>),
}

impl MetadataScheme {
	/// Returns the protocol prefix for this metadata URI type
	pub fn prefix(&self) -> &'static str {
		match self {
			MetadataScheme::Http(_path) => "http://",
			MetadataScheme::Https(_path) => "https://",
			MetadataScheme::IpfsDir(_path) => "ipfs://",
			MetadataScheme::IpfsShared(_path) => "ipfs://",
		}
	}
	/// Returns a sanitized version of the metadata URI
	pub fn sanitize(&self) -> Result<Self, ()> {
		let prefix = self.prefix();
		let santitize_ = |path: Vec<u8>| {
			if path.is_empty() {
				return Err(())
			}
			// some best effort attempts to sanitize `path`
			let mut path = core::str::from_utf8(&path).map_err(|_| ())?.trim();
			if path.ends_with("/") {
				path = &path[..path.len() - 1];
			}
			if path.starts_with(prefix) {
				path = &path[prefix.len()..];
			}
			Ok(path.as_bytes().to_vec())
		};

		Ok(match self.clone() {
			MetadataScheme::Http(path) => MetadataScheme::Http(santitize_(path)?),
			MetadataScheme::Https(path) => MetadataScheme::Https(santitize_(path)?),
			MetadataScheme::IpfsDir(path) => MetadataScheme::IpfsDir(santitize_(path)?),
			MetadataScheme::IpfsShared(path) => MetadataScheme::IpfsShared(santitize_(path)?),
		})
	}
}

#[cfg(feature = "std")]
pub fn serialize_utf8<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
	let base64_str =
		core::str::from_utf8(v).map_err(|_| serde::ser::Error::custom("Byte vec not UTF-8"))?;
	s.serialize_str(&base64_str)
}

#[cfg(feature = "std")]
pub fn serialize_royalties<S: Serializer, AccountId: Serialize>(
	royalties: &Vec<(AccountId, Permill)>,
	s: S,
) -> Result<S::Ok, S::Error> {
	let royalties: Vec<(&AccountId, String)> = royalties
		.iter()
		.map(|(account_id, per_mill)| {
			let per_mill = format!("{:.6}", per_mill.deconstruct() as f32 / 1000000f32);
			(account_id, per_mill)
		})
		.collect();
	royalties.serialize(s)
}

/// Contains information for a particular token. Returns the attributes and owner
#[derive(Eq, PartialEq, Decode, Encode, Default, Debug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TokenInfo<AccountId> {
	pub owner: AccountId,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_royalties"))]
	pub royalties: Vec<(AccountId, Permill)>,
}

/// Reason for an NFT being locked (un-transferrable)
#[derive(Decode, Encode, Debug, Clone, Eq, PartialEq, TypeInfo)]
pub enum TokenLockReason {
	/// Token is listed for sale
	Listed(ListingId),
}

/// The max. number of entitlements any royalties schedule can have
/// just a sensible upper bound
pub(crate) const MAX_ENTITLEMENTS: usize = 8;

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
#[derive(Default, Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
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

/// The listing response and cursor returned with the RPC getCollectionListing
#[derive(Decode, Encode, Debug, Clone, Eq, PartialEq, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ListingResponseWrapper<AccountId> {
	// List of listings to be returned
	pub listings: Vec<ListingResponse<AccountId>>,
	// Cursor pointing to next listing in the collection
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_u128_option"))]
	pub new_cursor: Option<u128>,
}

/// A type to encapsulate both auction listings and fixed price listings for RPC
/// getCollectionListing
#[derive(Decode, Encode, Debug, Clone, Eq, PartialEq, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ListingResponse<AccountId> {
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_u128"))]
	pub id: ListingId,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_utf8"))]
	pub listing_type: Vec<u8>,
	pub payment_asset: AssetId,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_u128"))]
	pub price: Balance,
	pub end_block: BlockNumber,
	pub buyer: Option<AccountId>,
	pub seller: AccountId,
	pub token_ids: Vec<TokenId>,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_royalties"))]
	pub royalties: Vec<(AccountId, Permill)>,
}

#[cfg(feature = "std")]
pub fn serialize_u128<S: Serializer>(val: &u128, s: S) -> Result<S::Ok, S::Error> {
	format!("{}", *val).serialize(s)
}

#[cfg(feature = "std")]
pub fn serialize_u128_option<S: Serializer>(val: &Option<u128>, s: S) -> Result<S::Ok, S::Error> {
	match val {
		Some(v) => format!("{}", *v).serialize(s),
		None => s.serialize_unit(),
	}
}

/// A type of NFT sale listing
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub enum Listing<T: Config> {
	FixedPrice(FixedPriceListing<T>),
	Auction(AuctionListing<T>),
}

/// Information about a marketplace
#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub struct Marketplace<AccountId> {
	/// The marketplace account
	pub account: AccountId,
	/// Royalties to go to the marketplace
	pub entitlement: Permill,
}

/// Information about an auction listing
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct AuctionListing<T: Config> {
	/// The asset to allow bids with
	pub payment_asset: AssetId,
	/// The threshold amount for a succesful bid
	pub reserve_price: Balance,
	/// When the listing closes
	pub close: T::BlockNumber,
	/// The seller of the tokens
	pub seller: T::AccountId,
	/// The token Ids for sale in this listing
	pub tokens: Vec<TokenId>,
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
	/// The token Ids for sale in this listing
	pub tokens: Vec<TokenId>,
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

/// Denotes a quantitiy of tokens
pub type TokenCount = SerialNumber;

#[cfg(test)]
mod test {
	use super::{ListingResponse, MetadataScheme, RoyaltiesSchedule, TokenId, TokenInfo};
	use crate::mock::{AccountId, TestExt};
	use serde_json;
	use sp_runtime::Permill;

	#[test]

	fn metadata_path_sanitize() {
		// empty
		assert_eq!(MetadataScheme::Http(b"".to_vec()).sanitize(), Err(()),);

		// protocol strippred, trailling slash gone
		assert_eq!(
			MetadataScheme::Http(b" http://test.com/".to_vec()).sanitize(),
			Ok(MetadataScheme::Http(b"test.com".to_vec()))
		);
		assert_eq!(
			MetadataScheme::Https(b"https://test.com/ ".to_vec()).sanitize(),
			Ok(MetadataScheme::Https(b"test.com".to_vec()))
		);
		assert_eq!(
			MetadataScheme::IpfsDir(b"ipfs://notarealCIDblah/".to_vec()).sanitize(),
			Ok(MetadataScheme::IpfsDir(b"notarealCIDblah".to_vec()))
		);

		// untouched
		assert_eq!(
			MetadataScheme::Http(b"test.com".to_vec()).sanitize(),
			Ok(MetadataScheme::Http(b"test.com".to_vec()))
		);
	}

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

	#[test]
	fn token_info_should_serialize() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = 1_u64;
			let royalties = RoyaltiesSchedule::<AccountId> {
				entitlements: vec![(3_u64, Permill::from_float(0.2))],
			};

			let token_info =
				TokenInfo { owner: collection_owner, royalties: royalties.entitlements };

			let json_str = "{\
				\"owner\":1,\
				\"royalties\":[\
					[\
						3,\
						\"0.200000\"\
					]\
				]\
			}";

			assert_eq!(serde_json::to_string(&token_info).unwrap(), json_str);
		});
	}

	#[test]
	fn collection_listings_should_serialize() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = 1_u64;
			let buyer = 2_u64;
			let royalties = RoyaltiesSchedule::<AccountId> {
				entitlements: vec![(3_u64, Permill::from_float(0.2))],
			};
			let token_id: TokenId = (0, 0);

			let listing_response = ListingResponse {
				id: 10,
				listing_type: "fixedPrice".as_bytes().to_vec(),
				payment_asset: 10,
				price: 10,
				end_block: 10,
				buyer: Some(buyer),
				seller: collection_owner,
				royalties: royalties.entitlements,
				token_ids: vec![token_id],
			};

			let json_str = "{\
			\"id\":\"10\",\
			\"listing_type\":\"fixedPrice\",\
			\"payment_asset\":10,\
			\"price\":\"10\",\
			\"end_block\":10,\
			\"buyer\":2,\
			\"seller\":1,\
			\"token_ids\":[[0,0]],\
			\"royalties\":[[3,\"0.200000\"]]}\
			";

			assert_eq!(serde_json::to_string(&listing_response).unwrap(), json_str);
		});
	}
}
