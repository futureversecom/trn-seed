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

use crate::{Config, Error};

use codec::{Decode, Encode};
use core::fmt::Write;
use frame_support::dispatch::DispatchResult;
use scale_info::TypeInfo;
use seed_primitives::{AssetId, Balance, BlockNumber, CollectionUuid, SerialNumber, TokenId};
use sp_core::H160;
use sp_runtime::{BoundedVec, PerThing, Permill};
use sp_std::prelude::*;

/// The max. number of entitlements any royalties schedule can have
/// just a sensible upper bound
pub(crate) const MAX_ENTITLEMENTS: usize = 8;

// Time before auction ends that auction is extended if a bid is placed
pub const AUCTION_EXTENSION_PERIOD: BlockNumber = 40;

/// OfferId type used to distinguish different offers on NFTs
pub type OfferId = u64;

pub type OwnedTokens<T> = BoundedVec<
	(
		<T as frame_system::Config>::AccountId,
		BoundedVec<SerialNumber, <T as Config>::MaxTokensPerCollection>,
	),
	<T as Config>::MaxTokensPerCollection,
>;

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
	/// All serial numbers owned by an account in a collection
	pub owned_tokens: OwnedTokens<T>,
}

impl<T: Config> CollectionInformation<T> {
	/// Check whether a token has been minted in a collection
	pub fn token_exists(&self, serial_number: SerialNumber) -> bool {
		self.owned_tokens
			.iter()
			.any(|(_, tokens)| tokens.clone().into_inner().contains(&serial_number))
	}

	/// Check whether who owns the serial number in collection_info
	pub fn is_token_owner(&self, who: &T::AccountId, serial_number: SerialNumber) -> bool {
		self.owned_tokens.iter().any(|(account, tokens)| {
			if account == who {
				tokens.clone().into_inner().contains(&serial_number)
			} else {
				false
			}
		})
	}

	/// Adds a list of tokens to a users balance in collection_info
	pub fn add_user_tokens(
		&mut self,
		token_owner: &T::AccountId,
		serial_numbers: BoundedVec<SerialNumber, T::MaxTokensPerCollection>,
	) -> DispatchResult {
		if self.owned_tokens.iter().any(|(owner, _)| owner == token_owner) {
			for (owner, owned_serial_numbers) in self.owned_tokens.iter_mut() {
				if owner != token_owner {
					continue
				}
				// Add new serial numbers to existing owner
				for serial_number in serial_numbers.iter() {
					owned_serial_numbers
						.try_push(*serial_number)
						.map_err(|_| Error::<T>::TokenLimitExceeded)?;
					owned_serial_numbers.sort();
				}
			}
		} else {
			// If token owner doesn't exist, create new entry
			self.owned_tokens
				.try_push((
					token_owner.clone(),
					BoundedVec::try_from(serial_numbers.to_vec())
						.map_err(|_| Error::<T>::TokenLimitExceeded)?,
				))
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
		for (owner, owned_serial_numbers) in self.owned_tokens.iter_mut() {
			if owner != token_owner {
				continue
			}
			owned_serial_numbers.retain(|serial| !serial_numbers.contains(serial));
			removing_all_tokens = owned_serial_numbers.is_empty();
		}
		// Check whether the owner has any tokens left, if not remove them from the collection
		if removing_all_tokens {
			self.owned_tokens.retain(|(owner, _)| owner != token_owner);
		}
	}
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
	// Collection metadata is located on Ethereum in the relevant field on the source token
	// ethereum://<contractaddress>/<originalid>
	Ethereum(H160),
}

impl MetadataScheme {
	/// Returns the protocol prefix for this metadata URI type
	pub fn prefix(&self) -> &'static str {
		match self {
			MetadataScheme::Http(_path) => "http://",
			MetadataScheme::Https(_path) => "https://",
			MetadataScheme::IpfsDir(_path) => "ipfs://",
			MetadataScheme::IpfsShared(_path) => "ipfs://",
			MetadataScheme::Ethereum(_path) => "ethereum://",
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
			// Ethereum inner value is an H160 and does not need sanitizing
			MetadataScheme::Ethereum(address) => MetadataScheme::Ethereum(address),
		})
	}
	/// Returns a MetadataScheme from an index and metadata_path
	pub fn from_index(index: u8, metadata_path: Vec<u8>) -> Result<Self, ()> {
		match index {
			0 => Ok(MetadataScheme::Https(metadata_path)),
			1 => Ok(MetadataScheme::Http(metadata_path)),
			2 => Ok(MetadataScheme::IpfsDir(metadata_path)),
			3 => Ok(MetadataScheme::IpfsShared(metadata_path)),
			_ => return Err(()),
		}
	}
	/// Returns the full token_uri for a token
	pub fn construct_token_uri(&self, serial_number: SerialNumber) -> Vec<u8> {
		let mut token_uri = sp_std::Writer::default();
		match self {
			MetadataScheme::Http(path) => {
				let path = core::str::from_utf8(&path).unwrap_or("");
				write!(&mut token_uri, "http://{}/{}.json", path, serial_number)
					.expect("Not written");
			},
			MetadataScheme::Https(path) => {
				let path = core::str::from_utf8(&path).unwrap_or("");
				write!(&mut token_uri, "https://{}/{}.json", path, serial_number)
					.expect("Not written");
			},
			MetadataScheme::IpfsDir(dir_cid) => {
				write!(
					&mut token_uri,
					"ipfs://{}/{}.json",
					core::str::from_utf8(&dir_cid).unwrap_or(""),
					serial_number
				)
				.expect("Not written");
			},
			MetadataScheme::IpfsShared(shared_cid) => {
				write!(
					&mut token_uri,
					"ipfs://{}.json",
					core::str::from_utf8(&shared_cid).unwrap_or("")
				)
				.expect("Not written");
			},
			MetadataScheme::Ethereum(contract_address) => {
				write!(&mut token_uri, "ethereum://{:?}/{}", contract_address, serial_number)
					.expect("Not written");
			},
		}
		token_uri.inner().clone()
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
	/// The threshold amount for a succesful bid
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

/// Denotes a quantitiy of tokens
pub type TokenCount = SerialNumber;

#[cfg(test)]
mod test {
	use super::{MetadataScheme, RoyaltiesSchedule};
	use sp_core::H160;
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

		assert_eq!(
			MetadataScheme::Ethereum(H160::from_low_u64_be(123)).sanitize(),
			Ok(MetadataScheme::Ethereum(H160::from_low_u64_be(123)))
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
}
