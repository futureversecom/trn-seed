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

use crate::keystore::EthyKeystore;
use log::{debug, error, trace, warn};
use seed_primitives::ethy::{
	crypto::{AuthorityId, AuthoritySignature as Signature},
	AuthorityIndex, EthyChainId, EthyEcdsaToPublicKey, EventProofId, ValidatorSet, Witness,
};
use sp_runtime::traits::Convert;
use std::collections::HashMap;

use crate::types::{data_to_digest, EventMetadata};

/// Status after processing a witness
#[derive(PartialEq, Debug)]
pub enum WitnessStatus {
	/// The witness digest needs verifying
	DigestUnverified,
	/// Its all ok
	Verified,
}

#[derive(PartialEq, Debug)]
pub enum WitnessError {
	/// This witness is for an already completed event
	CompletedEvent,
	/// This witness has been previously seen
	DuplicateWitness,
	/// The digest of the witness/event_id did not match our local digest
	MismatchedDigest,
	/// Failed to create the digest
	DigestCreationFailed,
	/// The signature supplied with the witness could not be verified
	SignatureVerificationFailed,
	/// The witness is from an unknown authority, can't be accepted
	UnknownAuthority,
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
	/// The secp256k1 public (session) keys of all active validators ORDERED! (managed by
	/// pallet-session & pallet-ethy)
	validators: ValidatorSet<AuthorityId>,
	/// The secp256k1 public (session) keys of the XRPL validators (subset of all validators)
	xrpl_validators: ValidatorSet<AuthorityId>,
	/// The record of witnesses `event_id -> [(validator index, validator signature)]`
	witnesses: HashMap<EventProofId, Vec<(AuthorityIndex, Signature)>>,
	/// The record of unverified witnesses `event_id -> [(validator index, validator signature)]`
	unverified_witnesses: HashMap<EventProofId, Vec<Witness>>,
	/// completed events
	completed_events: Vec<EventProofId>,
}

impl WitnessRecord {
	/// Set the active `ValidatorSet` for ethy and the XRPL subset
	pub fn set_validators(
		&mut self,
		validators: ValidatorSet<AuthorityId>,
		xrpl_validators: ValidatorSet<AuthorityId>,
	) {
		self.validators = validators;
		self.xrpl_validators = xrpl_validators;
	}
	/// Remove a witness record from memory (typically after it has achieved consensus)
	pub fn mark_complete(&mut self, event_id: EventProofId) {
		self.witnesses.remove(&event_id);
		self.event_meta.remove(&event_id);
		self.has_witnessed.remove(&event_id);
		self.unverified_witnesses.remove(&event_id);

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
	pub fn has_consensus(&self, event_id: EventProofId, chain_id: EthyChainId) -> bool {
		trace!(target: "ethy", "💎 event {:?}, witnesses: {:?}", event_id, self.witnesses.get(&event_id));

		let proof_threshold = match chain_id {
			EthyChainId::Ethereum => self.validators.proof_threshold as usize,
			EthyChainId::Xrpl => self.xrpl_validators.proof_threshold as usize,
		};

		let witness_count = match chain_id {
			EthyChainId::Ethereum => self.witnesses.get(&event_id).map(|w| w.len()),
			EthyChainId::Xrpl => self.witnesses.get(&event_id).map(|w| {
				// ethy tracks all witnesses but only a subset are able to be submitted to XRPL
				// count signatures from the XRPL authorized signers only
				w.iter()
					.filter(|(idx, _sig)| {
						let ethy_pub_key = self.validators.validators.get(*idx as usize);
						if let Some(ethy_pub_key) = ethy_pub_key {
							self.xrpl_validators.authority_index(ethy_pub_key).is_some()
						} else {
							false
						}
					})
					.count()
			}),
		}
		.unwrap_or(0_usize);

		trace!(target: "ethy", "💎 event {:?}, has # support: {:?}, proof threshold: {:?}", event_id, witness_count, proof_threshold);
		witness_count >= proof_threshold
	}

	/// Return event metadata
	pub fn event_metadata(&self, event_id: EventProofId) -> Option<&EventMetadata> {
		self.event_meta.get(&event_id)
	}
	/// Process any unverified witnesses for `event_id`
	/// Unverified witnesses can exist if metadata for an event was unknown locally when the
	/// witnesses were originally received by the network
	pub fn process_unverified_witnesses(&mut self, event_id: EventProofId) {
		if let Some(unverified) = self.unverified_witnesses.remove(&event_id) {
			for w in unverified {
				if let Err(err) = self.note_event_witness(&w) {
					warn!(target: "ethy", "💎 failed to note (unverified) witness: {:?}, {:?}", w, err);
				}
			}
		}
	}
	/// Note event metadata
	/// This must exist in order to locally verify witnesses
	pub fn note_event_metadata(
		&mut self,
		event_id: EventProofId,
		digest_data: Vec<u8>,
		block_hash: [u8; 32],
		chain_id: EthyChainId,
	) {
		self.event_meta.entry(event_id).or_insert(EventMetadata {
			block_hash,
			digest_data,
			chain_id,
		});
	}
	/// Note a witness if we haven't seen it before
	/// Returns true if the witness was noted, i.e previously unseen
	pub fn note_event_witness(&mut self, witness: &Witness) -> Result<WitnessStatus, WitnessError> {
		// Check if the witness is for a completed event, based on the pruned completed_events vec
		// First check if the event_id is contained within completed_events
		if self.completed_events.iter().any(|id| id == &witness.event_id) {
			return Err(WitnessError::CompletedEvent);
		} else {
			// If we have only 1 event, and it's not in completed events, that means the
			// completed_events id is 1 and the new event_id is 0, so comparing with the lowest
			// won't work
			if self.completed_events.len() > 1 {
				if let Some(completed_watermark) = self.completed_events.first() {
					if witness.event_id <= *completed_watermark {
						return Err(WitnessError::CompletedEvent);
					}
				}
			}
		}

		if self
			.has_witnessed
			.get(&witness.event_id)
			.map(|seen| seen.binary_search(&witness.authority_id).is_ok())
			.unwrap_or_default()
		{
			trace!(target: "ethy", "💎 witness previously seen: {:?}", witness.event_id);
			return Err(WitnessError::DuplicateWitness);
		}

		// witness metadata may not be available at this point
		// if so we can't fully verify `witness` is for the correct `digest` yet (i.e. validator
		// didn't sign a different message) store `witness` as unconfirmed for verification later
		if let Some(metadata) = self.event_metadata(witness.event_id) {
			// verify the signature against locally found digest info from metadata
			let Some(digest) = data_to_digest(
				metadata.chain_id,
				metadata.digest_data.clone(),
				EthyEcdsaToPublicKey::convert(witness.authority_id.clone()),
			) else {
				error!(target: "ethy", "💎 error creating digest");
				return Err(WitnessError::DigestCreationFailed);
			};
			// check if digests match
			if digest != witness.digest {
				warn!(target: "ethy", "💎 witness has bad digest: {:?} from {:?}", witness.event_id, witness.authority_id);
				debug!(target: "ethy", "💎 witness has bad digest: witness: {:?} local digest: {:?}", witness, digest);
				return Err(WitnessError::MismatchedDigest);
			}
			// signature verify
			if !EthyKeystore::verify_prehashed(&witness.authority_id, &witness.signature, &digest) {
				warn!(target: "ethy", "💎 witness digest signature verification failed: {:?} from {:?}", witness.event_id, witness.authority_id);
				debug!(target: "ethy", "💎 witness digest signature verification failed. witness: {:?}, local digest: {:?} ", witness, digest);
				return Err(WitnessError::SignatureVerificationFailed);
			}
		} else {
			// store witness for re-verification later
			debug!(target: "ethy", "💎 witness recorded (digest unverified): {:?}, {:?}", witness.event_id, witness.authority_id);
			self.unverified_witnesses
				.entry(witness.event_id)
				.and_modify(|witnesses| witnesses.push(witness.clone()))
				.or_insert_with(|| vec![witness.clone()]);
			return Ok(WitnessStatus::DigestUnverified);
		};

		// Convert authority secp256k1 public key into ordered index
		// this is useful to efficiently generate the full proof later
		let authority_index = self
			.validators
			.authority_index(&witness.authority_id)
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
		debug!(target: "ethy", "💎 witness recorded: {:?}, {:?}", witness.event_id, witness.authority_id);

		// Mark authority as voted
		match self.has_witnessed.get_mut(&witness.event_id) {
			None => {
				// first vote for this event id we've seen
				self.has_witnessed.insert(witness.event_id, vec![witness.authority_id.clone()]);
			},
			Some(seen) => {
				// subsequent witness for a known event id
				if let Err(idx) = seen.binary_search(&witness.authority_id) {
					seen.insert(idx, witness.authority_id.clone());
				}
			},
		}

		Ok(WitnessStatus::Verified)
	}

	#[cfg(test)]
	pub fn get_validator_set(&self) -> ValidatorSet<AuthorityId> {
		self.validators.clone()
	}
	#[cfg(test)]
	pub fn get_xrpl_validator_set(&self) -> ValidatorSet<AuthorityId> {
		self.validators.clone()
	}
}

/// Compact a sorted vec of IDs by replacing a monotonic sequence of IDs with the last ID in the
/// sequence
fn compact_sequence(completed_events: &mut [EventProofId]) -> &[EventProofId] {
	// Note: (JasonT) We keep at least 2 events in completed events to handle the first two events
	// (0,1) being processed in the incorrect order
	if completed_events.len() < 3 {
		return completed_events;
	}

	let mut watermark_idx = 0;
	for i in 0..completed_events.len() - 2 {
		if completed_events[i] + 1 as EventProofId == completed_events[i + 1] {
			watermark_idx = i + 1;
			continue;
		} else {
			break; // Note - fix the algo
		}
	}

	return completed_events.split_at(watermark_idx).1;
}

#[cfg(test)]
pub(crate) mod test {
	use crate::types::data_to_digest;

	use super::{compact_sequence, Signature, WitnessError, WitnessRecord, WitnessStatus};
	use crate::{keystore::EthyKeystore, testing::Keyring, tests::create_ethy_keystore};
	use seed_primitives::ethy::{
		AuthorityIndex, EthyChainId, EthyEcdsaToPublicKey, EventProofId, ValidatorSet, Witness,
	};
	use sp_runtime::traits::Convert;

	fn dev_signers() -> Vec<Keyring> {
		// we return Alice, Bob, Charlie
		vec![Keyring::Alice, Keyring::Bob, Keyring::Charlie]
	}

	fn dev_signers_xrpl() -> Vec<Keyring> {
		// Alice, Bob only
		vec![Keyring::Alice, Keyring::Bob]
	}

	/// Helper function for creating a Witness
	pub(crate) fn create_witness(
		validator: &Keyring,
		event_id: EventProofId,
		chain_id: EthyChainId,
		digest_data: [u8; 32],
	) -> Witness {
		let compatible_public = EthyEcdsaToPublicKey::convert(validator.public());
		let digest = data_to_digest(chain_id, digest_data.to_vec(), compatible_public).unwrap();
		let keystore: EthyKeystore = Some(create_ethy_keystore(*validator)).into();

		Witness {
			digest,
			chain_id,
			event_id,
			validator_set_id: 5_u64,
			authority_id: validator.public(),
			signature: keystore.sign_prehashed(&validator.public(), &digest).unwrap(),
			block_number: 0,
		}
	}

	#[test]
	fn proof_signatures_ordered_by_validator_index() {
		let validator_keys = dev_signers();
		let mut witness_record = WitnessRecord {
			// this determines the validator indexes as (0, alice), (1, bob), (2, charlie), etc.
			validators: ValidatorSet {
				validators: validator_keys.iter().map(|x| x.public()).collect(),
				..Default::default()
			},
			..Default::default()
		};

		let event_id = 5_u64;
		let digest = [1_u8; 32];
		witness_record.note_event_metadata(
			event_id,
			digest.to_vec(),
			Default::default(),
			EthyChainId::Ethereum,
		);

		// note signatures in reverse order
		for validator_key in validator_keys.iter().rev() {
			let witness = &create_witness(validator_key, event_id, EthyChainId::Ethereum, digest);
			assert!(witness_record.note_event_witness(witness).is_ok());
		}

		// signature returned in order
		assert_eq!(
			witness_record.signatures_for(event_id),
			validator_keys
				.into_iter()
				.enumerate()
				.map(|(idx, p)| {
					let keystore: EthyKeystore = Some(create_ethy_keystore(p)).into();
					(idx as u32, keystore.sign_prehashed(&p.public(), &digest).unwrap())
				})
				.collect::<Vec<(AuthorityIndex, Signature)>>(),
		);
	}

	#[test]
	fn note_event_witness_duplicate_witness() {
		let validator_keys = dev_signers();
		let mut witness_record = WitnessRecord {
			validators: ValidatorSet {
				validators: validator_keys.iter().map(|x| x.public()).collect(),
				..Default::default()
			},
			..Default::default()
		};
		let digest = [1_u8; 32];
		let event_id = 5_u64;

		witness_record.note_event_metadata(
			event_id,
			digest.to_vec(),
			Default::default(),
			EthyChainId::Ethereum,
		);

		// Create witness with Alice key and attempt to note witness twice
		let witness = &create_witness(&validator_keys[0], event_id, EthyChainId::Ethereum, digest);
		assert_eq!(witness_record.note_event_witness(witness), Ok(WitnessStatus::Verified));
		assert_eq!(witness_record.note_event_witness(witness), Err(WitnessError::DuplicateWitness));

		// Create witness with Bob key and attempt to note witness twice
		let witness = &create_witness(&validator_keys[1], event_id, EthyChainId::Ethereum, digest);
		assert_eq!(witness_record.note_event_witness(witness), Ok(WitnessStatus::Verified));
		assert_eq!(witness_record.note_event_witness(witness), Err(WitnessError::DuplicateWitness));
	}

	#[test]
	fn note_event_witness_mismatched_digest() {
		let validator_keys = dev_signers();
		let mut witness_record = WitnessRecord {
			validators: ValidatorSet {
				validators: validator_keys.iter().map(|x| x.public()).collect(),
				..Default::default()
			},
			..Default::default()
		};

		let event_id = 5_u64;
		let digest = [1_u8; 32];
		let witness = &create_witness(&validator_keys[0], event_id, EthyChainId::Ethereum, digest);

		witness_record.note_event_metadata(
			event_id,
			[2_u8; 32].to_vec(), // Digest created in create_witness() is [1_u8; 32]
			Default::default(),
			EthyChainId::Ethereum,
		);
		assert_eq!(witness_record.note_event_witness(witness), Err(WitnessError::MismatchedDigest));
	}

	#[test]
	fn note_event_witness_mismatched_digest_xrpl() {
		let validator_keys = dev_signers();
		let mut witness_record = WitnessRecord {
			validators: ValidatorSet {
				validators: validator_keys.iter().map(|x| x.public()).collect(),
				..Default::default()
			},
			xrpl_validators: ValidatorSet {
				validators: validator_keys.iter().map(|x| x.public()).collect(),
				..Default::default()
			},
			..Default::default()
		};

		let event_id = 5_u64;
		let digest = [1_u8; 32];
		let witness = &create_witness(&validator_keys[0], event_id, EthyChainId::Xrpl, digest);

		witness_record.note_event_metadata(
			event_id,
			[2_u8; 32].to_vec(), // Digest data passed to the create_witness() was [1_u8; 32]
			Default::default(),
			EthyChainId::Xrpl,
		);
		assert_eq!(witness_record.note_event_witness(witness), Err(WitnessError::MismatchedDigest));
	}

	#[test]
	fn note_event_witness_signature_verification_fails() {
		let validator_keys = dev_signers();
		let mut witness_record = WitnessRecord {
			validators: ValidatorSet {
				validators: validator_keys.iter().map(|x| x.public()).collect(),
				..Default::default()
			},
			xrpl_validators: ValidatorSet {
				validators: validator_keys.iter().map(|x| x.public()).collect(),
				..Default::default()
			},
			..Default::default()
		};
		let event_id = 5_u64;
		let digest = [1_u8; 32];
		let witness =
			&mut create_witness(&validator_keys[0], event_id, EthyChainId::Ethereum, digest);
		// Add invalid signature
		witness.signature = validator_keys[0].sign(b"wrong message!");

		witness_record.note_event_metadata(
			event_id,
			[1_u8; 32].to_vec(),
			Default::default(),
			EthyChainId::Ethereum,
		);

		assert_eq!(
			witness_record.note_event_witness(witness),
			Err(WitnessError::SignatureVerificationFailed)
		);
	}

	#[test]
	fn note_event_witness_signature_verification_fails_xrpl() {
		let validator_keys = dev_signers();
		let mut witness_record = WitnessRecord {
			validators: ValidatorSet {
				validators: validator_keys.iter().map(|x| x.public()).collect(),
				..Default::default()
			},
			xrpl_validators: ValidatorSet {
				validators: validator_keys.iter().map(|x| x.public()).collect(),
				..Default::default()
			},
			..Default::default()
		};
		let event_id = 5_u64;
		let digest = [1_u8; 32];
		let witness = &mut create_witness(&validator_keys[0], event_id, EthyChainId::Xrpl, digest);
		// Add invalid signature
		witness.signature = validator_keys[0].sign(b"wrong message!");

		witness_record.note_event_metadata(
			event_id,
			[1_u8; 32].to_vec(),
			Default::default(),
			EthyChainId::Xrpl,
		);

		assert_eq!(
			witness_record.note_event_witness(witness),
			Err(WitnessError::SignatureVerificationFailed)
		);
	}

	#[test]
	fn note_event_witness_unknown_authority() {
		let dave_pair = Keyring::Dave;
		let mut witness_record = WitnessRecord::default();
		let event_id = 5_u64;
		let digest = [1_u8; 32];

		let witness = &create_witness(&dave_pair, event_id, EthyChainId::Ethereum, digest);

		witness_record.note_event_metadata(
			event_id,
			digest.to_vec(),
			Default::default(),
			EthyChainId::Ethereum,
		);
		assert_eq!(witness_record.note_event_witness(witness), Err(WitnessError::UnknownAuthority));
	}

	#[test]
	fn note_event_witness_completed_event() {
		let validator_keys = dev_signers();
		let mut witness_record = WitnessRecord {
			// this determines the validator indexes as (0, alice), (1, bob), (2, charlie), etc.
			validators: ValidatorSet {
				validators: validator_keys.iter().map(|x| x.public()).collect(),
				..Default::default()
			},
			..Default::default()
		};
		let event_id = 5_u64;
		let digest = [1_u8; 32];

		let witness = &create_witness(&validator_keys[0], event_id, EthyChainId::Ethereum, digest);
		assert!(witness_record.note_event_witness(witness).is_ok());

		let witness = &create_witness(&validator_keys[2], event_id, EthyChainId::Ethereum, digest);
		assert!(witness_record.note_event_witness(witness).is_ok());

		// event complete
		witness_record.mark_complete(event_id);
		assert_eq!(witness_record.note_event_witness(witness), Err(WitnessError::CompletedEvent));

		// memory cleared
		assert!(witness_record.event_meta.get(&event_id).is_none());
		assert!(witness_record.has_witnessed.get(&event_id).is_none());
		assert!(witness_record.witnesses.get(&event_id).is_none());
		assert_eq!(witness_record.completed_events, vec![event_id]);
	}

	#[test]
	/// This test checks the edge case where the first two events (0 and 1) are noted in the
	/// incorrect order. Both should be processed and completed once and only once
	fn note_event_witness_completed_event_first_two_incorrect_order() {
		let validator_keys = dev_signers();
		let mut witness_record = WitnessRecord {
			// this determines the validator indexes as (0, alice), (1, bob), (2, charlie), etc.
			validators: ValidatorSet {
				validators: validator_keys.iter().map(|x| x.public()).collect(),
				..Default::default()
			},
			..Default::default()
		};
		let event_id_0 = 0_u64;
		let event_id_1 = 1_u64;
		let event_id_2 = 2_u64;
		let digest = [1_u8; 32];
		let chain_id = EthyChainId::Xrpl;

		// Note and complete event_id 1
		let witness_1 = &create_witness(&validator_keys[0], event_id_1, chain_id, digest);
		assert!(witness_record.note_event_witness(witness_1).is_ok());
		witness_record.mark_complete(event_id_1);
		// Further witness on this event should fail
		assert_eq!(witness_record.note_event_witness(witness_1), Err(WitnessError::CompletedEvent));
		// completed_events should contain event_id 1
		assert_eq!(witness_record.completed_events, vec![event_id_1]);

		// Note and complete event_id 0 (Ethereum event)
		let chain_id = EthyChainId::Ethereum;
		let witness_0 = &create_witness(&validator_keys[1], event_id_0, chain_id, digest);
		assert!(witness_record.note_event_witness(witness_0).is_ok());
		witness_record.mark_complete(event_id_0);
		// Further witness on this event should fail
		assert_eq!(witness_record.note_event_witness(witness_0), Err(WitnessError::CompletedEvent));
		// completed_events should contain event_id 0 and 1 (Not pruned)
		assert_eq!(witness_record.completed_events, vec![event_id_0, event_id_1]);

		// Note and complete event_id 2
		let witness_2 = &create_witness(&validator_keys[2], event_id_2, chain_id, digest);
		assert!(witness_record.note_event_witness(witness_2).is_ok());
		witness_record.mark_complete(event_id_2);
		// completed_events should contain event_id 2 (0 now pruned), we keep both 1 and 2
		assert_eq!(witness_record.completed_events, vec![event_id_1, event_id_2]);

		// Further witness on all three events should fail
		assert_eq!(witness_record.note_event_witness(witness_0), Err(WitnessError::CompletedEvent));
		assert_eq!(witness_record.note_event_witness(witness_1), Err(WitnessError::CompletedEvent));
		assert_eq!(witness_record.note_event_witness(witness_2), Err(WitnessError::CompletedEvent));
	}

	#[test]
	/// This test checks the edge case where the first two events (0 and 1) are noted in the correct
	/// order. Both should be processed and completed once and only once
	fn note_event_witness_completed_event_first_two_correct_order() {
		let validator_keys = dev_signers();
		let mut witness_record = WitnessRecord {
			// this determines the validator indexes as (0, alice), (1, bob), (2, charlie), etc.
			validators: ValidatorSet {
				validators: validator_keys.iter().map(|x| x.public()).collect(),
				..Default::default()
			},
			..Default::default()
		};
		let event_id_0 = 0_u64;
		let event_id_1 = 1_u64;
		let event_id_2 = 2_u64;
		let digest = [1_u8; 32];
		let chain_id = EthyChainId::Ethereum;

		// Note and complete event_id 0
		let witness_0 = &create_witness(&validator_keys[2], event_id_0, chain_id, digest);
		assert!(witness_record.note_event_witness(witness_0).is_ok());
		witness_record.mark_complete(event_id_0);
		// Further witness on this event should fail
		assert_eq!(witness_record.note_event_witness(witness_0), Err(WitnessError::CompletedEvent));
		// completed_events should contain event_id 0
		assert_eq!(witness_record.completed_events, vec![event_id_0]);

		// Note and complete event_id 1 (XRPL event)
		let chain_id = EthyChainId::Xrpl;
		let witness_1 = &create_witness(&validator_keys[0], event_id_1, chain_id, digest);
		assert!(witness_record.note_event_witness(witness_1).is_ok());
		witness_record.mark_complete(event_id_1);
		// Further witness on this event should fail
		assert_eq!(witness_record.note_event_witness(witness_1), Err(WitnessError::CompletedEvent));
		// completed_events should contain event_id 0 and 1 (not pruned)
		assert_eq!(witness_record.completed_events, vec![event_id_0, event_id_1]);

		// Note and complete event_id 2 (XRPL event)
		let witness_2 = &create_witness(&validator_keys[1], event_id_2, chain_id, digest);
		assert!(witness_record.note_event_witness(witness_2).is_ok());
		witness_record.mark_complete(event_id_2);
		// completed_events should contain event_id 2 (0 now pruned), we keep both 1 and 2
		assert_eq!(witness_record.completed_events, vec![event_id_1, event_id_2]);

		// Further witness on all three events should fail
		assert_eq!(witness_record.note_event_witness(witness_0), Err(WitnessError::CompletedEvent));
		assert_eq!(witness_record.note_event_witness(witness_1), Err(WitnessError::CompletedEvent));
		assert_eq!(witness_record.note_event_witness(witness_2), Err(WitnessError::CompletedEvent));
	}

	#[test]
	fn has_consensus() {
		let validator_keys = dev_signers();
		let mut witness_record = WitnessRecord {
			// this determines the validator indexes as (0, alice), (1, bob), (2, charlie), etc.
			validators: ValidatorSet {
				validators: validator_keys.iter().map(|x| x.public()).collect(),
				proof_threshold: 2,
				..Default::default()
			},
			..Default::default()
		};
		let chain_id = EthyChainId::Ethereum;
		let event_id = 5_u64;
		let digest = [1_u8; 32];

		let witness = &create_witness(&validator_keys[0], event_id, chain_id, digest);
		assert!(witness_record.note_event_witness(witness).is_ok());
		assert!(!witness_record.has_consensus(event_id, chain_id));

		let witness = &create_witness(&validator_keys[1], event_id, chain_id, digest);
		assert!(witness_record.note_event_witness(witness).is_ok());

		// unverified
		assert!(!witness_record.has_consensus(event_id, chain_id));

		witness_record.note_event_metadata(event_id, digest.to_vec(), Default::default(), chain_id);
		witness_record.process_unverified_witnesses(event_id);

		assert!(witness_record.has_consensus(event_id, chain_id));
		assert!(witness_record.has_consensus(event_id, chain_id));
	}

	#[test]
	fn has_consensus_xrpl() {
		let xrpl_validator_keys = dev_signers_xrpl();
		let validator_keys = dev_signers();
		let validator_set_id = 1_u64;
		let mut witness_record = WitnessRecord {
			validators: ValidatorSet {
				validators: validator_keys.iter().map(|x| x.public()).collect(),
				proof_threshold: 3,
				id: validator_set_id,
			},
			xrpl_validators: ValidatorSet {
				validators: xrpl_validator_keys.iter().map(|x| x.public()).collect(),
				proof_threshold: 2,
				id: validator_set_id,
			},
			..Default::default()
		};
		let chain_id = EthyChainId::Xrpl;
		let digest = [1_u8; 32];
		let event_id = 5_u64;
		let witness = &create_witness(&validator_keys[0], event_id, chain_id, digest);

		witness_record.note_event_metadata(event_id, digest.to_vec(), Default::default(), chain_id);
		assert_eq!(witness_record.note_event_witness(witness), Ok(WitnessStatus::Verified));
		assert!(!witness_record.has_consensus(event_id, chain_id));

		// charlie is not an XRPL signer so cannot affect consensus
		let witness = &create_witness(&validator_keys[2], event_id, chain_id, digest);
		assert_eq!(witness_record.note_event_witness(witness), Ok(WitnessStatus::Verified));
		assert!(!witness_record.has_consensus(event_id, chain_id));

		// bob signs and we have consensus
		let witness = &create_witness(&validator_keys[1], event_id, chain_id, digest);
		assert_eq!(witness_record.note_event_witness(witness), Ok(WitnessStatus::Verified));
		witness_record.process_unverified_witnesses(event_id);

		assert!(witness_record.has_consensus(event_id, chain_id));
	}

	#[test]
	fn note_event_witness_out_of_order_event() {
		let validator_keys = dev_signers();
		let mut witness_record = WitnessRecord {
			// this determines the validator indexes as (0, alice), (1, bob), (2, charlie), etc.
			validators: ValidatorSet {
				validators: validator_keys.iter().map(|x| x.public()).collect(),
				..Default::default()
			},
			..Default::default()
		};

		// ids 1, 2 & 4 are complete
		witness_record.mark_complete(1);
		witness_record.mark_complete(2);
		witness_record.mark_complete(4);

		// id 3 should be accepted
		let digest = [1_u8; 32];
		let witness = &create_witness(&validator_keys[0], 3, EthyChainId::Ethereum, digest);
		assert!(witness_record.note_event_witness(witness).is_ok());
	}

	#[test]
	/// Compact sequence should leave at least the lowest two events
	fn compact_sequence_works() {
		assert_eq!(compact_sequence(&mut [1]), [1]);
		assert_eq!(compact_sequence(&mut [0, 1]), [0, 1]);
		assert_eq!(compact_sequence(&mut [0, 1, 2]), [1, 2]);
		assert_eq!(compact_sequence(&mut [1, 2, 3, 4, 5]), [4, 5]);
		assert_eq!(compact_sequence(&mut [1, 2, 3, 8, 9]), [3, 8, 9]);
		assert_eq!(compact_sequence(&mut [1, 2, 3, 4, 9]), [4, 9]);
	}

	#[test]
	fn witness_signature_verification_xrpl() {
		let validator_keys = dev_signers();
		let validator = &validator_keys[0]; // Alice
		let chain_id = EthyChainId::Xrpl;
		let event_id = 5;
		let compatible_public = EthyEcdsaToPublicKey::convert(validator.public());
		let digest_data = [2_u8; 32];

		let witness = create_witness(validator, event_id, chain_id, digest_data);
		let correct_digest =
			data_to_digest(chain_id, digest_data.to_vec(), compatible_public).unwrap();
		assert!(EthyKeystore::verify_prehashed(
			&validator.public(),
			&witness.signature,
			&correct_digest
		));

		let wrong_digest = data_to_digest(chain_id, vec![3_u8; 32], compatible_public).unwrap();
		assert!(!EthyKeystore::verify_prehashed(
			&validator.public(),
			&witness.signature,
			&wrong_digest
		),);
	}

	#[test]
	fn witness_signature_verification_ethereum() {
		let validator_keys = dev_signers();
		let validator = &validator_keys[0];
		let chain_id = EthyChainId::Ethereum;
		let event_id = 5;
		let compatible_public = EthyEcdsaToPublicKey::convert(validator.public());
		let digest_data = [2_u8; 32];

		let witness = create_witness(validator, event_id, chain_id, digest_data);
		let correct_digest =
			data_to_digest(chain_id, digest_data.to_vec(), compatible_public).unwrap();
		assert!(EthyKeystore::verify_prehashed(
			&validator.public(),
			&witness.signature,
			&correct_digest
		));

		let wrong_digest = data_to_digest(chain_id, vec![3_u8; 32], compatible_public).unwrap();
		assert!(!EthyKeystore::verify_prehashed(
			&validator.public(),
			&witness.signature,
			&wrong_digest
		),);
	}
}
