/* Copyright 2021-2022 Centrality Investments Limited
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

//! Ethereum bridge common types & traits
//! Shared between eth-bridge pallet & ethy-gadget worker

use codec::{Decode, Encode};
use sp_runtime::KeyTypeId;
use sp_std::prelude::*;

use self::crypto::{AuthorityId, AuthoritySignature};

// fixed storage key for offchain config.
// for consistency expect 4 byte key for prefix and 8 byte key for subkeys
/// offchain storage config key for Ethereum HTTP URI
pub const ETH_HTTP_URI: [u8; 8] = *b"ETH_HTTP";

/// The `ConsensusEngineId` of ETHY.
pub const ETHY_ENGINE_ID: sp_runtime::ConsensusEngineId = *b"ETH-";

/// Authority set id starts with zero at genesis
pub const GENESIS_AUTHORITY_SET_ID: u64 = 0;

/// The session key type for Ethereum bridge
pub const ETH_BRIDGE_KEY_TYPE: KeyTypeId = KeyTypeId(*b"eth-");

/// Crypto types for Eth bridge protocol
pub mod crypto {
	mod app_crypto {
		use crate::ethy::ETH_BRIDGE_KEY_TYPE;
		use sp_application_crypto::{app_crypto, ecdsa};
		app_crypto!(ecdsa, ETH_BRIDGE_KEY_TYPE);
	}
	sp_application_crypto::with_pair! {
		/// An eth bridge keypair using ecdsa as its crypto.
		pub type AuthorityPair = app_crypto::Pair;
	}
	/// An eth bridge signature using ecdsa as its crypto.
	pub type AuthoritySignature = app_crypto::Signature;
	/// An eth bridge identifier using ecdsa as its crypto.
	pub type AuthorityId = app_crypto::Public;
}

/// The index of an authority.
pub type AuthorityIndex = u32;

/// An event message for signing
pub type Message = Vec<u8>;

/// Unique nonce for event claim requests
pub type EventClaimId = u64;

/// Unique nonce for event proof requests
pub type EventProofId = u64;

/// A typedef for validator set id.
pub type ValidatorSetId = u64;

/// A set of ETHY authorities, a.k.a. validators.
#[derive(Decode, Encode, Debug, PartialEq, Clone)]
pub struct ValidatorSet<AuthorityId> {
	/// Public keys of the validator set elements
	pub validators: Vec<AuthorityId>,
	/// Identifier of the validator set
	pub id: ValidatorSetId,
	/// Minimum number of validator signatures required for a valid proof (i.e 'm' in 'm-of-n')
	pub proof_threshold: u32,
}

impl<AuthorityId> ValidatorSet<AuthorityId> {
	/// Return an empty validator set with id of 0.
	pub fn empty() -> Self {
		Self { validators: Default::default(), id: Default::default(), proof_threshold: 0 }
	}
}

/// A consensus log item for ETHY.
#[derive(Decode, Encode)]
pub enum ConsensusLog<AuthorityId: Encode + Decode> {
	/// The authorities have changed.
	#[codec(index = 1)]
	AuthoritiesChange(ValidatorSet<AuthorityId>),
	/// Disable the authority with given index.
	#[codec(index = 2)]
	OnDisabled(AuthorityIndex),
	/// A request to sign some data was logged
	/// `Message` is packed bytes e.g. `abi.encodePacked(param0, param1, paramN, validatorSetId,
	/// event_id)`
	#[codec(index = 3)]
	OpaqueSigningRequest((Message, EventProofId)),
	#[codec(index = 4)]
	/// Signal an `AuthoritiesChange` is scheduled for next session
	/// Generate a proof that the current validator set has witnessed the new authority set
	PendingAuthoritiesChange((ValidatorSet<AuthorityId>, EventProofId)),
}

/// ETHY witness message.
///
/// A witness message is a vote created by an ETHY node for a given 'event' combination
/// and is gossiped to its peers.
#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq)]
pub struct Witness {
	/// The event hash: `keccak(abi.encodePacked(param0, param1, paramN, validator_set_id,
	/// event_id))`
	pub digest: [u8; 32],
	/// Event proof nonce (it is unique across all Ethy event proofs)
	pub event_id: EventProofId,
	/// The validator set witnessing the message
	pub validator_set_id: ValidatorSetId,
	/// Node public key (i.e. Ethy session key)
	pub authority_id: AuthorityId,
	/// Node signature
	/// Over `keccak(abi.encodePacked(self.message, self.nonce))`
	/// a 512-bit value, plus 8 bits for recovery ID.
	pub signature: AuthoritySignature,
}

/// A witness with matching GRANDPA validators' signatures.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub struct EventProof {
	/// The event witnessed
	/// The hash of: `keccak(abi.encode(param0, param1, ..,paramN, validator_set_id, event_id))`
	pub digest: [u8; 32],
	/// The witness signatures are collected for this event.
	pub event_id: EventProofId,
	/// The validators set Id that signed the proof
	pub validator_set_id: ValidatorSetId,
	/// GRANDPA validators' signatures for the witness.
	///
	/// The length of this `Vec` must match number of validators in the current set (see
	/// [Witness::validator_set_id]).
	pub signatures: Vec<crypto::AuthoritySignature>,
	/// Block hash of the event
	pub block: [u8; 32],
	/// Metadata tag for the event
	pub tag: Option<Vec<u8>>,
}

impl EventProof {
	/// Return the number of collected signatures.
	pub fn signature_count(&self) -> usize {
		let empty_sig = AuthoritySignature::from(sp_core::ecdsa::Signature::default());
		self.signatures.iter().filter(|x| x != &&empty_sig).count()
	}
}

/// A [EventProof] with a version number. This variant will be appended
/// to the block justifications for the block for which the signed witness
/// has been generated.
#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub enum VersionedEventProof {
	#[codec(index = 1)]
	/// Current active version
	V1(EventProof),
}

sp_api::decl_runtime_apis! {
	/// Runtime API for ETHY validators.
	pub trait EthyApi
	{
		/// Return the active ETHY validator set (i.e Ethy bridge keys of active validator set)
		fn validator_set() -> ValidatorSet<AuthorityId>;
	}
}
