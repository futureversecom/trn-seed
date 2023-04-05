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

//! ethy-gadget types
use seed_primitives::ethy::{EthyChainId, EventProofId, ETHY_ENGINE_ID};

/// Metadata about an Ethy event
pub struct EventMetadata {
	/// The associated chain Id of the event
	pub chain_id: EthyChainId,
	/// The digest data of the event
	// store the digest data rather than the digest, and calculate the digest when required since
	// the xrpl digest is unique per public key
	pub digest_data: Vec<u8>,
	/// The (finalized) block hash where the event proof was made
	pub block_hash: [u8; 32],
}

/// An Ethy proof request
pub struct ProofRequest {
	/// The associated chain Id of the proof request
	pub chain_id: EthyChainId,
	/// data for signing (possibly a digest, depends on bridge protocol for `chain_id`)
	pub data: Vec<u8>,
	/// nonce/event Id of this request
	pub event_id: EventProofId,
	/// Finalized block hash where the proof was requested
	pub block: [u8; 32],
}

/// Make proof storage key
pub fn make_proof_key(chain_id: EthyChainId, event_id: EventProofId) -> Vec<u8> {
	[
		ETHY_ENGINE_ID.as_slice(),
		[Into::<u8>::into(chain_id)].as_slice(),
		event_id.to_be_bytes().as_slice(),
	]
	.concat()
}

// data must be transformed into a 32 byte digest before signing
pub fn data_to_digest(
	chain_id: EthyChainId,
	data: Vec<u8>,
	public_key: [u8; 33],
) -> Option<[u8; 32]> {
	if chain_id == EthyChainId::Xrpl {
		// XRPL has a unique protocol for multi-signing tx `data` where each authority must
		// add its own public key to the data before hashing it
		// the digest is unique per validator
		Some(xrpl_codec::utils::digest_for_multi_signing_pre(data.as_slice(), public_key))
	} else {
		// any other chains e.g. Ethereum, `data` should already be a `keccak256` digest
		data.try_into().ok()
	}
}
