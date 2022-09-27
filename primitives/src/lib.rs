//! Common types across runtimes, pallets, and/or client
#![cfg_attr(not(feature = "std"), no_std)]

pub use opaque::*;
pub use signature::*;
pub use types::*;

pub mod ethy;
mod signature;

// offchain storage config key for XRP HTTP URI
pub const XRP_HTTP_URI: [u8; 8] = *b"XRP_HTTP";

pub mod types {
	use crate::signature::EthereumSignature;
	use sp_core::{H160, H512};
	use sp_runtime::traits::{IdentifyAccount, Verify};

	/// An index to a block.
	pub type BlockNumber = u32;

	/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
	pub type Signature = EthereumSignature;

	/// Some way of identifying an account on the chain. We intentionally make it equivalent
	/// to the public key of our transaction signing scheme.
	pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

	/// The chain address type
	pub type Address = AccountId;

	/// Balance of an account.
	pub type Balance = u128;

	/// Index of a transaction in the chain.
	pub type Index = u32;

	/// A hash of some data used by the chain.
	pub type Hash = sp_core::H256;

	/// Digest item type.
	pub type DigestItem = sp_runtime::generic::DigestItem;

	// Babe consensus authority.
	pub type BabeId = sp_consensus_babe::AuthorityId;

	// Id used for identifying assets.
	pub type AssetId = u32;

	/// Uniquely identifies a collection across parachains
	/// Made up of ParachainId (10 bits) CollectionId (22 bits)
	///
	/// example:
	/// world: 100, collection: 1234
	/// 0x00000000000000000000000000134864
	/// 0b00000000000100110100100001100100
	pub type CollectionUuid = u32;

	/// Auto-incrementing Uint
	/// Uniquely identifies a token within a collection
	pub type SerialNumber = u32;

	/// Global unique token identifier
	pub type TokenId = (CollectionUuid, SerialNumber);

	/// Parachain Identifier
	pub type ParachainId = u32;

	/// The type for identifying the validators
	pub type ValidatorId = u32;

	pub type Timestamp = u64;

	/// An index to a block.
	pub type LedgerIndex = u64;

	pub type XrplTxHash = H512;

	pub type XrplAddress = H160;

	/// The type for identifying the Withdraw Tx Nonce
	pub type XrplWithdrawTxNonce = u32;
	/// Unique nonce for event proof requests
	pub type EventId = u64;

	/// Ethereum address type
	pub type EthAddress = sp_core::H160;
}

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;
	use sp_runtime::{generic, traits::BlakeTwo256};

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
}
