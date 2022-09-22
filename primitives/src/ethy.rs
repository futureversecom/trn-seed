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
use scale_info::TypeInfo;
use sp_runtime::KeyTypeId;
use sp_std::prelude::*;

use self::crypto::{AuthorityId, AuthoritySignature};
use crate::AccountId;

// fixed storage key for offchain config.
// for consistency expect 4 byte key for prefix and 8 byte key for subkeys
/// offchain storage config key for Ethereum HTTP URI
pub const ETH_HTTP_URI: [u8; 8] = *b"ETH_HTTP";

/// The `ConsensusEngineId` of Ethy.
pub const ETHY_ENGINE_ID: sp_runtime::ConsensusEngineId = *b"ETHY";

/// Authority set id starts with zero at genesis
pub const GENESIS_AUTHORITY_SET_ID: u64 = 0;

/// The session key type for Ethy
pub const ETHY_KEY_TYPE: KeyTypeId = KeyTypeId(*b"eth-");

/// Crypto types for Ethy protocol
pub mod crypto {
	mod app_crypto {
		use crate::ethy::ETHY_KEY_TYPE;
		use sp_application_crypto::{app_crypto, ecdsa};
		app_crypto!(ecdsa, ETHY_KEY_TYPE);
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

/// An ethy specific identifier for a bridged network
#[derive(Encode, Decode, Debug, Eq, PartialEq, TypeInfo, Copy, Clone)]
pub enum EthyChainId {
	/// The Chain Id given to Ethereum by ethy
	Ethereum = 1,
	/// The Chain Id given to Xrpl by ethy
	Xrpl = 2,
}

impl Into<u8> for EthyChainId {
	fn into(self) -> u8 {
		match self {
			Self::Ethereum => 1_u8,
			Self::Xrpl => 2_u8,
		}
	}
}

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

/// Authority change data
#[derive(Decode, Encode)]
pub struct PendingAuthorityChange<AuthorityId: Encode + Decode> {
	/// The source of the change
	pub source: AccountId,
	/// The destination for the change
	pub destination: AccountId,
	/// The next validator set (ordered)
	pub next_validator_set: ValidatorSet<AuthorityId>,
	/// The event proof Id for this request
	pub event_proof_id: EventProofId,
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
	/// A request from the runtime for ethy-gadget to sign some prehashed data (`digest`)
	#[codec(index = 3)]
	OpaqueSigningRequest { chain_id: EthyChainId, event_proof_id: EventProofId, digest: [u8; 32] },
	#[codec(index = 4)]
	/// Signal an `AuthoritiesChange` is scheduled for next session
	/// Generate a proof that the current validator set has witnessed the new authority set
	PendingAuthoritiesChange(PendingAuthorityChange<AuthorityId>),
}

/// Ethy witness message.
///
/// A witness message is a vote created by an Ethy node for a given 'event' combination
/// and is gossiped to its peers.
#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq)]
pub struct Witness {
	/// The event digest (the hash function may differ based on chain Id)
	pub digest: [u8; 32],
	/// The associated chainId for this witness
	pub chain_id: EthyChainId,
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

/// An Ethy event proof with validator signatures.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub struct EventProof {
	/// The event digest (hash function may differ based on chain Id)
	pub digest: [u8; 32],
	/// The event proof Id.
	pub event_id: EventProofId,
	/// The validators set Id that signed the proof
	pub validator_set_id: ValidatorSetId,
	/// Signatures for the proof.
	///
	/// The length of this `Vec` must match number of validators in the current set (see
	/// [Witness::validator_set_id]).
	pub signatures: Vec<(AuthorityIndex, crypto::AuthoritySignature)>,
	/// Finalized block hash of the event (when it was requested)
	pub block: [u8; 32],
}

impl EventProof {
	/// Return the number of collected signatures.
	pub fn signature_count(&self) -> usize {
		let empty_sig = AuthoritySignature::from(sp_core::ecdsa::Signature::default());
		self.signatures.iter().filter(|(_id, sig)| sig != &empty_sig).count()
	}
	/// Return a full list or signatures, ordered by authority index, with blank values added if a
	/// real signature is missing
	/// `n_signatures` - the total number of signatures that should be returned when expanded (it is
	/// the size of the validator set `validator_set_id`)
	pub fn expanded_signatures(&self, n_signatures: usize) -> Vec<crypto::AuthoritySignature> {
		let empty_sig = AuthoritySignature::from(sp_core::ecdsa::Signature::default());
		let mut signatures = vec![empty_sig; n_signatures];
		for (idx, signature) in self.signatures.iter() {
			signatures[*idx as usize] = signature.clone();
		}
		signatures
	}
}

/// An `EventProof` with a version number. This variant will be appended
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
		/// Return the Ethy validator set for `chain_id` (i.e Secp256k1 public keys of the authorized validator set)
		fn validator_set() -> ValidatorSet<AuthorityId>;
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use sp_core::ecdsa::Signature;

	#[test]
	fn signature_helpers() {
		let empty_sig = AuthoritySignature::from(Signature::default());
		let proof = EventProof {
			signatures: vec![
				(1, Signature::from_raw([1_u8; 65]).into()),
				(3, Signature::from_raw([3_u8; 65]).into()),
				(4, Signature::from_raw([4_u8; 65]).into()),
			],
			tag: None,
			digest: Default::default(),
			block: Default::default(),
			event_id: 1,
			validator_set_id: 1,
		};
		assert_eq!(
			proof.expanded_signatures(6_usize),
			vec![
				empty_sig.clone(),
				Signature::from_raw([1_u8; 65]).into(),
				empty_sig.clone(),
				Signature::from_raw([3_u8; 65]).into(),
				Signature::from_raw([4_u8; 65]).into(),
				empty_sig.clone(),
			]
		);
		assert_eq!(proof.signature_count(), 3);
	}

	#[test]
	fn ethy_chain_id() {
		assert_eq!(EthyChainId::Ethereum.into(), 1_u8);
		assert_eq!(EthyChainId::Xrpl.into(), 2_u8);
	}
}
