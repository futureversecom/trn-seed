//! ethy-gadget types
use seed_primitives::ethy::{EthyChainId, EventProofId, ETHY_ENGINE_ID};

/// Metadata about an Ethy event
pub struct EventMetadata {
	/// The associated chain Id of the event
	pub chain_id: EthyChainId,
	/// The digest of the event
	pub digest: [u8; 32],
	/// The (finalized) block hash where the event proof was made
	pub block_hash: [u8; 32],
}

/// An Ethy proof request
pub struct ProofRequest {
	/// The associated chain Id of the proof request
	pub chain_id: EthyChainId,
	/// hashed message for signing (hash function determined by destination chain Id)
	pub digest: [u8; 32],
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
		&event_id.to_be_bytes().as_slice(),
	]
	.concat()
}
