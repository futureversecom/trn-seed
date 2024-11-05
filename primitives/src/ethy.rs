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

//! Ethereum bridge common types & traits
//! Shared between eth-bridge pallet & ethy-gadget worker

use codec::{Decode, Encode};
use ripemd::{Digest as _, Ripemd160};
use scale_info::TypeInfo;
use sha2::Sha256;
use sp_application_crypto::ByteArray;
use sp_runtime::{traits::Convert, KeyTypeId};
use sp_std::prelude::*;

use self::crypto::{AuthorityId, AuthoritySignature};

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

impl From<EthyChainId> for u8 {
	fn from(value: EthyChainId) -> Self {
		match value {
			EthyChainId::Ethereum => 1_u8,
			EthyChainId::Xrpl => 2_u8,
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
#[derive(Decode, Encode, Debug, PartialEq, Clone, TypeInfo)]
pub struct ValidatorSet<AuthorityId> {
	/// Public keys of the validator set elements
	pub validators: Vec<AuthorityId>,
	/// Identifier of the validator set
	pub id: ValidatorSetId,
	/// Minimum number of validator signatures required for a valid proof (i.e 'm' in 'm-of-n')
	pub proof_threshold: u32,
}

impl Default for ValidatorSet<AuthorityId> {
	fn default() -> Self {
		Self::empty()
	}
}

impl ValidatorSet<AuthorityId> {
	pub fn new<I>(validators: I, id: ValidatorSetId, proof_threshold: u32) -> Self
	where
		I: IntoIterator<Item = AuthorityId>,
	{
		let validators: Vec<AuthorityId> = validators.into_iter().collect();
		Self { validators, id, proof_threshold }
	}
	/// Return an empty validator set with id of 0.
	pub fn empty() -> Self {
		Self { validators: Default::default(), id: Default::default(), proof_threshold: 0 }
	}
	/// Return whether the validator set is empty or not
	pub fn is_empty(&self) -> bool {
		self.validators.is_empty()
	}
	/// Return the authority index of `who` in the validator set
	pub fn authority_index(&self, who: &AuthorityId) -> Option<usize> {
		self.validators.iter().position(|v| v == who)
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
	/// A request from the runtime for ethy-gadget to sign some `data`
	/// The format of `data` is determined by the bridging protocol for a given `chain_id`
	#[codec(index = 3)]
	OpaqueSigningRequest { chain_id: EthyChainId, event_proof_id: EventProofId, data: Vec<u8> },
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
	/// ECDSA signature over `digest`
	pub signature: AuthoritySignature,
	/// proof requested block number
	pub block_number: u64,
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
	/// The length of this `Vec` must match the number of validators in the current set (see
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
	/// Return a full list of signatures, ordered by authority index, with blank values added if a
	/// real signature is missing
	/// `n_signatures` - the total number of signatures that should be returned when expanded (it is
	/// the size of the validator set `validator_set_id`)
	pub fn expanded_signatures(&self, n_signatures: usize) -> Vec<crypto::AuthoritySignature> {
		let empty_sig = AuthoritySignature::from(sp_core::ecdsa::Signature::default());

		// The length of the signatures is expected to be the same as the length of the validators
		// in the current set
		if n_signatures != self.signatures.len() {
			log::warn!(target: "ethy", "💎 The amount of signatures received is not equal to the amount of validators, there may be an unexpected amount of signatures stored/retrieved");
		}

		let mut signatures = vec![empty_sig; n_signatures];

		for (idx, signature) in self.signatures.iter() {
			// Avoid errors by stopping early if there are more signatures than validator addresses
			// stored
			if idx >= &(n_signatures as u32) {
				return signatures;
			}
			signatures[*idx as usize] = signature.clone();
		}
		signatures
	}
}

/// Convert an Ethy secp256k1 public key into an Ethereum address
pub struct EthyEcdsaToEthereum;
impl Convert<&[u8], [u8; 20]> for EthyEcdsaToEthereum {
	fn convert(compressed_key: &[u8]) -> [u8; 20] {
		libsecp256k1::PublicKey::parse_slice(
			compressed_key,
			Some(libsecp256k1::PublicKeyFormat::Compressed),
		)
		// uncompress the key
		.map(|pub_key| pub_key.serialize().to_vec())
		// now convert to Ethereum address
		.map(|uncompressed| {
			sp_io::hashing::keccak_256(&uncompressed[1..])[12..]
				.try_into()
				.expect("32 byte digest")
		})
		.map_err(|_| {
			log::error!(target: "ethy", "💎 invalid ethy public key format");
		})
		.unwrap_or_default()
	}
}

/// Convert an EthyId to an secp256k1 public key
pub struct EthyEcdsaToPublicKey;
impl Convert<AuthorityId, [u8; 33]> for EthyEcdsaToPublicKey {
	fn convert(a: AuthorityId) -> [u8; 33] {
		let compressed_key = a.as_slice();
		libsecp256k1::PublicKey::parse_slice(
			compressed_key,
			Some(libsecp256k1::PublicKeyFormat::Compressed),
		)
		.map(|k| k.serialize_compressed())
		.unwrap_or([0_u8; 33])
	}
}

/// Convert a 33 byte Secp256k1 pub key to an XRPL account ID
pub struct EthyEcdsaToXRPLAccountId;
impl Convert<&[u8], [u8; 20]> for EthyEcdsaToXRPLAccountId {
	fn convert(compressed_key: &[u8]) -> [u8; 20] {
		libsecp256k1::PublicKey::parse_slice(
			compressed_key,
			Some(libsecp256k1::PublicKeyFormat::Compressed),
		)
		.map(|k| k.serialize_compressed())
			.map(|k| Ripemd160::digest(Sha256::digest(k)).into())
		.unwrap_or([0_u8; 20])
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
		/// Return the Ethy validator set (i.e Secp256k1 public keys of the authorized validator set)
		fn validator_set() -> ValidatorSet<AuthorityId>;
		/// Return the (subset) of Ethy validators configured for XRPL signing (i.e Secp256k1 public keys of the authorized validator set)
		fn xrpl_signers() -> ValidatorSet<AuthorityId>;
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use hex_literal::hex;
	use sp_core::ecdsa::Signature;

	#[test]
	fn ethy_pub_key_to_ethereum_address() {
		let address = hex!("dB6B186A0Cf75833903A4cfA0Aa618eDa65793f4");

		assert_eq!(
			EthyEcdsaToEthereum::convert(&hex!(
				"02276503736589d21316da95a46d82b2d5c7aa10b946abbdeb01728d7cb935235e"
			)),
			address,
		);
	}

	#[test]
	fn signature_helpers() {
		let empty_sig = AuthoritySignature::from(Signature::default());
		let proof = EventProof {
			signatures: vec![
				(1, Signature::from_raw([1_u8; 65]).into()),
				(3, Signature::from_raw([3_u8; 65]).into()),
				(4, Signature::from_raw([4_u8; 65]).into()),
			],
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
	fn handles_different_validator_and_signatures_length() {
		let proof = EventProof {
			signatures: vec![
				(1, Signature::from_raw([1_u8; 65]).into()),
				(3, Signature::from_raw([3_u8; 65]).into()),
				(4, Signature::from_raw([4_u8; 65]).into()),
			],
			digest: Default::default(),
			block: Default::default(),
			event_id: 1,
			validator_set_id: 1,
		};

		// Ensures we don't panic with a low amount of validators
		proof.expanded_signatures(1);
	}

	#[test]
	fn ethy_chain_id() {
		assert_eq!(Into::<u8>::into(EthyChainId::Ethereum), 1_u8);
		assert_eq!(Into::<u8>::into(EthyChainId::Xrpl), 2_u8);
	}

	#[test]
	fn ethy_ecdsa_to_xrpl_account_id() {
		// values taken from https://xrpl.org/assign-a-regular-key-pair.html
		let xrpl_account_id = hex!("1620d685fb08d81a70d0b668749cf2e130ea7540");

		assert_eq!(
			EthyEcdsaToXRPLAccountId::convert(&hex!(
				"03AEEFE1E8ED4BBC009DE996AC03A8C6B5713B1554794056C66E5B8D1753C7DD0E"
			)),
			xrpl_account_id,
		);
	}
}
