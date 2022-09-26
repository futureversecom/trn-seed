// Copyright (C) 2020-2022 Parity Technologies (UK) Ltd. and Centrality Investment Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use log::{trace, warn};
use std::collections::HashMap;

use seed_primitives::ethy::{
	crypto::{AuthorityId, AuthoritySignature as Signature},
	AuthorityIndex, EthyChainId, EventProofId, Witness,
};

use crate::types::EventMetadata;

#[derive(PartialEq, Debug)]
pub enum WitnessError {
	/// The digest of the witness/event_id did not match our local digest
	MismatchedDigest,
	/// The witness is from an unknown authority, can't be accepted
	UnknownAuthority,
	/// This witness has been previously seen
	DuplicateWitness,
	/// This witness is for an already completed event
	CompletedEvent,
}

/// Handles tracking witnesses from ethy participants
///
/// Expired/complete witnesses are handled at the gossip layer
#[derive(Default)]
pub struct WitnessRecord {
	/// Metadata about an event
	event_meta: HashMap<EventProofId, EventMetadata>,
	/// Tracks observed witnesses from event -> validator Id
	has_witnessed: HashMap<EventProofId, Vec<AuthorityId>>,
	/// The ECDSA public (session) keys of active validators ORDERED! (managed by pallet-session &
	/// pallet-ethy)
	validators: Vec<AuthorityId>,
	/// The record of witnesses `event_id -> [(validator index, validator signature)]`
	witnesses: HashMap<EventProofId, Vec<(AuthorityIndex, Signature)>>,
	/// completed events
	completed_events: Vec<EventProofId>,
}

impl WitnessRecord {
	/// Set the validator keys
	pub fn set_validators(&mut self, validators: Vec<AuthorityId>) {
		self.validators = validators;
	}
	/// Remove a witness record from memory (typically after it has achieved consensus)
	pub fn mark_complete(&mut self, event_id: EventProofId) {
		self.witnesses.remove(&event_id);
		self.event_meta.remove(&event_id);
		self.has_witnessed.remove(&event_id);

		if let Err(idx) = self.completed_events.binary_search(&event_id) {
			self.completed_events.insert(idx, event_id);
			self.completed_events = compact_sequence(self.completed_events.as_mut_slice()).to_vec();
		}
	}
	/// Return all known signatures for the witness on `event_id`
	// Signatures are stored as tuples of (i-th validator index, validator signature)
	pub fn signatures_for(&self, event_id: EventProofId) -> Vec<(AuthorityIndex, Signature)> {
		match self.witnesses.get(&event_id) {
			Some(witnesses) => witnesses.to_vec(),
			None => Default::default(),
		}
	}
	/// Does the event identified by `event_id` `digest` have >= `threshold` support
	pub fn has_consensus(&self, event_id: EventProofId, threshold: usize) -> bool {
		trace!(target: "ethy", "💎 event {:?}, witnesses: {:?}", event_id, self.witnesses.get(&event_id));
		let maybe_count = self.witnesses.get(&event_id).map(|v| v.len());

		trace!(target: "ethy", "💎 event {:?}, has # support: {:?}", event_id, maybe_count);
		maybe_count.unwrap_or_default() >= threshold
	}
	/// Return event metadata
	pub fn event_metadata(&self, event_id: EventProofId) -> Option<&EventMetadata> {
		self.event_meta.get(&event_id)
	}
	/// Note event metadata
	pub fn note_event_metadata(
		&mut self,
		event_id: EventProofId,
		digest: [u8; 32],
		block_hash: [u8; 32],
		chain_id: EthyChainId,
	) {
		self.event_meta
			.entry(event_id)
			.or_insert(EventMetadata { block_hash, digest, chain_id });
	}
	/// Note a witness if we haven't seen it before
	/// Returns true if the witness was noted, i.e previously unseen
	pub fn note_event_witness(&mut self, witness: &Witness) -> Result<(), WitnessError> {
		// Is the witness for a completed event?
		if let Some(completed_watermark) = self.completed_events.first() {
			if witness.event_id <= *completed_watermark {
				return Err(WitnessError::CompletedEvent)
			}
		}

		if self
			.has_witnessed
			.get(&witness.event_id)
			.map(|votes| votes.binary_search(&witness.authority_id).is_ok())
			.unwrap_or_default()
		{
			trace!(target: "ethy", "💎 witness previously seen: {:?}", witness.event_id);
			return Err(WitnessError::DuplicateWitness)
		}

		if let Some(metadata) = self.event_metadata(witness.event_id) {
			// Witnesses for XRPL are special cases and have unique digests
			if metadata.digest != witness.digest && witness.chain_id != EthyChainId::Xrpl {
				warn!(target: "ethy", "💎 witness has bad digest: {:?} from {:?}", witness.event_id, witness.authority_id);
				return Err(WitnessError::MismatchedDigest)
			}
		}

		// Convert authority ECDSA public key into ordered index
		// this is useful to efficiently generate the full proof later
		let authority_index = self
			.validators
			.iter()
			.position(|v| v == &witness.authority_id)
			.ok_or(WitnessError::UnknownAuthority)? as AuthorityIndex;

		// There are 2 cases:
		// 1) first time observing an event and witness
		// 2) observed event, first time observing a witness
		self.witnesses
			.entry(witness.event_id)
			.and_modify(|witnesses| {
				// case 2
				if let Err(idx) =
					witnesses.binary_search_by_key(&authority_index, |(idx, _sig)| *idx)
				{
					witnesses
						.insert(idx, (authority_index as AuthorityIndex, witness.signature.clone()))
				}
			})
			.or_insert_with(|| vec![(authority_index, witness.signature.clone())]);

		trace!(target: "ethy", "💎 witness recorded: {:?}, {:?}", witness.event_id, witness.authority_id);

		// Mark authority as voted
		match self.has_witnessed.get_mut(&witness.event_id) {
			None => {
				// first vote for this event_id we've seen
				self.has_witnessed.insert(witness.event_id, vec![witness.authority_id.clone()]);
			},
			Some(votes) => {
				// subsequent vote for a known event_id
				if let Err(idx) = votes.binary_search(&witness.authority_id) {
					votes.insert(idx, witness.authority_id.clone());
				}
			},
		}

		Ok(())
	}
}

/// Compact a sorted vec of IDs by replacing a monotonic sequence of IDs with the last ID in the
/// sequence
fn compact_sequence(completed_events: &mut [EventProofId]) -> &[EventProofId] {
	if completed_events.len() < 2 {
		return completed_events
	}

	let mut watermark_idx = 0;
	for i in 0..completed_events.len() - 1 {
		if completed_events[i] + 1 as EventProofId == completed_events[i + 1] {
			watermark_idx = i + 1;
			continue
		} else {
			break
		}
	}

	return completed_events.split_at(watermark_idx).1
}

#[cfg(test)]
mod test {
	use sp_application_crypto::Pair;

	use seed_primitives::ethy::{crypto::AuthorityPair, AuthorityIndex, Witness};

	use super::{compact_sequence, Signature, WitnessError, WitnessRecord};

	fn dev_signers() -> Vec<AuthorityPair> {
		let alice_pair = AuthorityPair::from_string("//Alice", None).unwrap();
		let bob_pair = AuthorityPair::from_string("//Bob", None).unwrap();
		let charlie_pair = AuthorityPair::from_string("//Charlie", None).unwrap();
		vec![alice_pair, bob_pair, charlie_pair]
	}

	#[test]
	fn proof_signatures_ordered_by_validator_index() {
		let validator_keys = dev_signers();
		let mut witness_record = WitnessRecord {
			// this determines the validator indexes as (0, alice), (1, bob), (2, charlie), etc.
			validators: validator_keys.iter().map(|x| x.public()).collect(),
			..Default::default()
		};

		let event_id = 5_u64;
		let digest = [1_u8; 32];
		// note signatures in reverse order
		for validator_key in validator_keys.iter().rev() {
			assert!(witness_record
				.note_event_witness(&Witness {
					digest,
					event_id,
					validator_set_id: 5_u64,
					authority_id: validator_key.public(),
					signature: validator_key.sign(&digest),
				})
				.is_ok());
		}

		// signature returned in order
		assert_eq!(
			witness_record.signatures_for(event_id),
			validator_keys
				.into_iter()
				.enumerate()
				.map(|(idx, p)| (idx as u32, p.sign(&digest)))
				.collect::<Vec<(AuthorityIndex, Signature)>>(),
		);
	}

	#[test]
	fn note_event_witness_duplicate_witness() {
		let validator_keys = dev_signers();
		let mut witness_record = WitnessRecord {
			// this determines the validator indexes as (0, alice), (1, bob), (2, charlie), etc.
			validators: validator_keys.iter().map(|x| x.public()).collect(),
			..Default::default()
		};

		let digest = [1_u8; 32];
		let alice_validator = &validator_keys[0];
		let witness = &Witness {
			digest,
			event_id: 5_u64,
			validator_set_id: 5_u64,
			authority_id: alice_validator.public(),
			signature: alice_validator.sign(&digest),
		};

		assert!(witness_record.note_event_witness(witness).is_ok());
		assert_eq!(witness_record.note_event_witness(witness), Err(WitnessError::DuplicateWitness));

		let bob_validator = &validator_keys[1];
		let witness = &Witness {
			digest,
			event_id: 5_u64,
			validator_set_id: 5_u64,
			authority_id: bob_validator.public(),
			signature: bob_validator.sign(&digest),
		};

		assert!(witness_record.note_event_witness(witness).is_ok());
		assert_eq!(witness_record.note_event_witness(witness), Err(WitnessError::DuplicateWitness));
	}

	#[test]
	fn note_event_witness_mismatched_digest() {
		let validator_keys = dev_signers();
		let mut witness_record = WitnessRecord {
			// this determines the validator indexes as (0, alice), (1, bob), (2, charlie), etc.
			validators: validator_keys.iter().map(|x| x.public()).collect(),
			..Default::default()
		};

		let alice_validator = &validator_keys[0];
		let digest = [1_u8; 32];
		let event_id = 5_u64;
		let witness = &Witness {
			digest,
			event_id,
			validator_set_id: 5_u64,
			authority_id: alice_validator.public(),
			signature: alice_validator.sign(&digest),
		};

		witness_record.note_event_metadata(event_id, [2_u8; 32], Default::default(), None);
		assert_eq!(witness_record.note_event_witness(witness), Err(WitnessError::MismatchedDigest));
	}

	#[test]
	fn note_event_witness_unknown_authority() {
		let dave_pair = AuthorityPair::from_string("//Dave", None).unwrap();
		let mut witness_record = WitnessRecord::default();
		let witness = &Witness {
			digest: [1_u8; 32],
			event_id: 5_u64,
			validator_set_id: 5_u64,
			authority_id: dave_pair.public(),
			signature: dave_pair.sign(&[1u8; 32]),
		};
		assert_eq!(witness_record.note_event_witness(witness), Err(WitnessError::UnknownAuthority));
	}

	#[test]
	fn note_event_witness_completed_event() {
		let validator_keys = dev_signers();
		let mut witness_record = WitnessRecord {
			// this determines the validator indexes as (0, alice), (1, bob), (2, charlie), etc.
			validators: validator_keys.iter().map(|x| x.public()).collect(),
			..Default::default()
		};

		let event_id = 5_u64;
		let digest = [1_u8; 32];
		let alice_validator = &validator_keys[0];
		let witness = &Witness {
			digest,
			event_id,
			validator_set_id: 5_u64,
			authority_id: alice_validator.public(),
			signature: alice_validator.sign(&digest),
		};
		assert!(witness_record.note_event_witness(witness).is_ok());

		let bob_validator = &validator_keys[2];
		let witness = &Witness {
			digest,
			event_id,
			validator_set_id: 5_u64,
			authority_id: bob_validator.public(),
			signature: bob_validator.sign(&digest),
		};
		assert!(witness_record.note_event_witness(witness).is_ok());

		// event complete
		witness_record.mark_complete(event_id);
		assert_eq!(witness_record.note_event_witness(witness), Err(WitnessError::CompletedEvent));

		// memory cleared
		assert!(witness_record.event_meta.get(&event_id).is_none());
		assert!(witness_record.has_witnessed.get(&event_id).is_none());
		assert!(witness_record.witnesses.get(&event_id).is_none());
		assert!(witness_record.completed_events.iter().any(|x| *x == event_id));
	}

	#[test]
	fn has_consensus() {
		let validator_keys = dev_signers();
		let mut witness_record = WitnessRecord {
			// this determines the validator indexes as (0, alice), (1, bob), (2, charlie), etc.
			validators: validator_keys.iter().map(|x| x.public()).collect(),
			..Default::default()
		};

		let event_id = 5_u64;
		let digest = [1_u8; 32];
		let alice_validator = &validator_keys[0];
		let witness = &Witness {
			digest,
			event_id,
			validator_set_id: 5_u64,
			authority_id: alice_validator.public(),
			signature: alice_validator.sign(&digest),
		};

		assert!(witness_record.note_event_witness(witness).is_ok());
		assert!(!witness_record.has_consensus(event_id, 2));

		let bob_validator = &validator_keys[1];
		let witness = &Witness {
			digest,
			event_id,
			validator_set_id: 5_u64,
			authority_id: bob_validator.public(),
			signature: bob_validator.sign(&digest),
		};

		assert!(witness_record.note_event_witness(witness).is_ok());
		assert!(witness_record.has_consensus(event_id, 2));
	}

	#[test]
	fn note_event_witness_out_of_order_event() {
		let validator_keys = dev_signers();
		let mut witness_record = WitnessRecord {
			// this determines the validator indexes as (0, alice), (1, bob), (2, charlie), etc.
			validators: validator_keys.iter().map(|x| x.public()).collect(),
			..Default::default()
		};

		// ids 1, 2 & 4 are complete
		witness_record.mark_complete(1);
		witness_record.mark_complete(2);
		witness_record.mark_complete(4);

		// id 3 should be accepted
		let digest = [1_u8; 32];
		let alice_validator = &validator_keys[0];
		let witness = &Witness {
			digest,
			event_id: 3,
			validator_set_id: 5_u64,
			authority_id: alice_validator.public(),
			signature: alice_validator.sign(&digest),
		};
		assert!(witness_record.note_event_witness(witness).is_ok());
	}

	#[test]
	fn compact_sequence_works() {
		assert_eq!(compact_sequence(&mut [1]), [1]);
		assert_eq!(compact_sequence(&mut [0, 1]), [1]);
		assert_eq!(compact_sequence(&mut [0, 1, 2]), [2]);
		assert_eq!(compact_sequence(&mut [1, 2, 3, 8, 9]), [3, 8, 9]);
	}
}
