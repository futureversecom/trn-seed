use seed_primitives::SerialNumber;

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::H160;
use sp_runtime::{PerThing, Permill};
use sp_std::prelude::*;

/// NFT collection moniker
pub type CollectionNameType = Vec<u8>;

/// Denotes a quantitiy of tokens
pub type TokenCount = SerialNumber;

/// The max. number of entitlements any royalties schedule can have
/// just a sensible upper bound
pub const MAX_ENTITLEMENTS: usize = 8;

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

#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo)]
/// Describes the chain that the bridged resource originated from
pub enum OriginChain {
	Ethereum,
	Root,
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
	// The chain in which the collection was minted originally
	pub origin_chain: OriginChain,
}
