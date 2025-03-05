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

//! Common types across runtimes, pallets, and/or client
#![cfg_attr(not(feature = "std"), no_std)]

pub use opaque::*;
pub use signature::*;
pub use types::*;

pub mod ethy;
pub mod migration;
pub mod nft;
mod signature;
#[cfg(feature = "std")]
pub mod test_utils;

pub use nft::*;

// offchain storage config key for XRP HTTP URI
pub const XRP_HTTP_URI: [u8; 8] = *b"XRP_HTTP";

pub mod types {
	use crate::signature::EthereumSignature;
	use frame_support::dispatch::Weight;
	use sp_runtime::traits::{BlakeTwo256, IdentifyAccount, Verify};
	use sp_runtime::DispatchError;

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
	pub type Nonce = u32;

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

	/// Denotes a quantitiy of tokens
	pub type TokenCount = SerialNumber;

	/// Parachain Identifier
	pub type ParachainId = u32;

	/// The type for identifying the validators
	pub type ValidatorId = u32;

	pub type Timestamp = u64;

	/// Ethereum address type
	pub type EthAddress = sp_core::H160;

	/// Blake2-256 Hash implementation.
	pub type BlakeTwo256Hash = BlakeTwo256;

	/// DispatchResult that includes weight values
	pub type WeightedDispatchResult = Result<Weight, (Weight, DispatchError)>;

	/// Identifier for a pending issuance of a soulbound token
	pub type IssuanceId = u64;
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

/// XRPL primitive types
pub mod xrpl {
	use sp_core::{H160, H512};

	/// An index to a block.
	pub type LedgerIndex = u64;

	/// An XRPL AccountId
	// https://xrpl.org/accounts.html#address-encoding
	pub type XrplAccountId = H160;

	/// An XRPL tx hash
	pub type XrplTxHash = H512;

	/// The type for identifying the XRPL Tx Nonce aka 'Sequence'
	pub type XrplTxNonce = u32;

	/// The type for identifying the XRPL Tx TicketSequence
	pub type XrplTxTicketSequence = u32;

	/// TokenId type for XLS-20 Token Ids
	/// See: https://github.com/XRPLF/XRPL-Standards/discussions/46
	pub type Xls20TokenId = [u8; 32];
}

#[derive(PartialEq)]
pub enum OffchainErr {
	OffchainStore,
	SubmitTransaction,
	NotAValidator,
	OffchainLock,
	TooEarly,
}

impl sp_std::fmt::Debug for OffchainErr {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match *self {
			OffchainErr::OffchainStore => write!(fmt, "Failed to manipulate offchain store"),
			OffchainErr::SubmitTransaction => write!(fmt, "Failed to submit transaction"),
			OffchainErr::NotAValidator => write!(fmt, "Is not validator"),
			OffchainErr::OffchainLock => write!(fmt, "Failed to manipulate offchain lock"),
			OffchainErr::TooEarly => write!(fmt, "Too early to send unsigned transaction"),
		}
	}
}
