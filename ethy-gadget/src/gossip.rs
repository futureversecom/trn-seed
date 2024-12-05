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

use codec::Decode;
use log::{debug, error, info, trace, warn};
use parking_lot::{Mutex, RwLock};
use sc_client_api::Backend;
use sc_network::PeerId;
use sc_network_gossip::{MessageIntent, ValidationResult, Validator, ValidatorContext};
use sp_api::{BlockT, HeaderT};
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::{Block, Hash, Header};
use std::{
	collections::{BTreeMap, VecDeque},
	sync::Arc,
	time::{Duration, Instant},
};

use seed_primitives::ethy::{crypto::AuthorityId as Public, EventProofId, Witness};

use crate::keystore::EthyKeystore;

/// Gossip engine messages topic
pub(crate) fn topic<B>() -> B::Hash
where
	B: Block,
{
	<<B::Header as Header>::Hashing as Hash>::hash(b"ethy")
}

/// Number of recent complete events to keep in memory
// Theoretically This buffer should hold completed events until they go out of live window.
// rough theoretical value of 2520 is suitable at the expense of increased search time. Should not
// be problematic. can change as the network grows if required.
const MAX_COMPLETE_EVENT_CACHE: usize = 500;

// Timeout for rebroadcasting messages.
const REBROADCAST_AFTER: Duration = Duration::from_secs(60 * 3);

// Window size in blocks within which we expect the request to reach terminal state.
// We take the WINDOW_SIZE approximately as 6 mins. This gives at-least another rebroadcast before
// going out of live window.
#[cfg(not(test))]
const WINDOW_SIZE: u64 = 90;
#[cfg(test)]
const WINDOW_SIZE: u64 = 5;

/// ETHY gossip validator
///
/// Validate ETHY gossip messages
///
/// All messaging is handled in a single ETHY global topic.
pub(crate) struct GossipValidator<B, BE>
where
	B: Block,
	BE: Backend<B>,
{
	topic: B::Hash,
	known_votes: RwLock<BTreeMap<EventProofId, Vec<Public>>>,
	/// Pruned list of recently completed events
	complete_events: RwLock<VecDeque<EventProofId>>,
	/// Public (ECDSA session) keys of active ethy validators
	active_validators: RwLock<Vec<Public>>,
	/// Scheduled time for re-broadcasting event witnesses
	next_rebroadcast: Mutex<Instant>,
	/// client backend
	backend: Arc<BE>,
}

impl<B, BE> GossipValidator<B, BE>
where
	B: Block,
	BE: Backend<B>,
{
	pub fn new(active_validators: Vec<Public>, backend: Arc<BE>) -> GossipValidator<B, BE> {
		GossipValidator {
			topic: topic::<B>(),
			known_votes: RwLock::new(BTreeMap::new()),
			active_validators: RwLock::new(active_validators),
			complete_events: RwLock::new(Default::default()),
			next_rebroadcast: Mutex::new(Instant::now() + REBROADCAST_AFTER),
			backend,
		}
	}

	/// Wheher the gossip validator is tracking an event
	#[cfg(test)]
	fn is_tracking_event(&self, event_id: &EventProofId) -> bool {
		self.known_votes.read().get(event_id).is_some()
	}

	/// Make a vote for an event as complete
	pub fn mark_complete(&self, event_id: EventProofId) {
		let mut known_votes = self.known_votes.write();
		known_votes.remove(&event_id);
		let mut complete_events = self.complete_events.write();
		if complete_events.len() >= MAX_COMPLETE_EVENT_CACHE {
			complete_events.pop_front();
		}
		match complete_events.binary_search(&event_id) {
			Ok(_idx) => {
				error!(target: "ethy", "💎 double event complete: {:?} in {:?}", event_id, complete_events);
			},
			Err(idx) => {
				complete_events.insert(idx, event_id);
			},
		}
	}

	pub fn set_active_validators(&self, new_active_validators: Vec<Public>) {
		let mut active_validators = self.active_validators.write();
		let _old = std::mem::replace(&mut *active_validators, new_active_validators);
		info!(target: "ethy", "💎 set gossip active validators: {:?}", active_validators);
	}
}

impl<B, BE> Validator<B> for GossipValidator<B, BE>
where
	B: Block,
	BE: Backend<B>,
	<<B as BlockT>::Header as HeaderT>::Number: Into<u64>,
{
	fn validate(
		&self,
		_context: &mut dyn ValidatorContext<B>,
		sender: &PeerId,
		mut data: &[u8],
	) -> ValidationResult<B::Hash> {
		if let Ok(Witness {
			authority_id,
			event_id,
			validator_set_id,
			digest,
			signature,
			block_number,
			..
		}) = Witness::decode(&mut data)
		{
			trace!(target: "ethy", "💎 witness from: {:?}, validator set: {:?}, event: {:?}", authority_id, validator_set_id, event_id);

			let mut known_votes = self.known_votes.write();
			let maybe_known = known_votes.get(&event_id).map(|v| v.binary_search(&authority_id));
			if let Some(Ok(_)) = maybe_known {
				trace!(target: "ethy", "💎 witness from: {:?}, event: {:?} is already known", &authority_id, event_id);
				return ValidationResult::Discard;
			}

			if !self.active_validators.read().iter().any(|v| *v == authority_id) {
				trace!(target: "ethy", "💎 witness from: {:?}, event: {:?} is not an active authority", &authority_id, event_id);
				return ValidationResult::Discard;
			}

			// verify witness is a valid signature for `digest`, this does NOT guarantee digest is
			// correct i.e malicious or buggy validators could sign anything as digest this will be
			// verified later to match the locally extracted digest from finalized block headers
			if EthyKeystore::verify_prehashed(&authority_id, &signature, &digest) {
				// Make the vote as seen
				trace!(target: "ethy", "💎 verify prehashed OK, waiting lock: {:?}, event: {:?}", &authority_id, event_id);
				match maybe_known {
					Some(Err(index)) => {
						// we've seen this nonce and need to add the new vote
						// insert_index is guaranteed to be `Err` as it has not been recorded yet
						if let Some(v) = known_votes.get_mut(&event_id) {
							v.insert(index, authority_id.clone())
						}
					},
					None => {
						// we haven't seen this nonce yet
						known_votes.insert(event_id, vec![authority_id.clone()]);
					},
					Some(Ok(_)) => (), // dropped/checked prior as duplicate
				}

				trace!(target: "ethy", "💎 valid witness: {:?}, event: {:?}", &authority_id, event_id);
				let finalized_number = self.backend.blockchain().info().finalized_number;
				if block_number < finalized_number.into().saturating_sub(WINDOW_SIZE) {
					info!(target: "ethy", "💎 witness: {:?}, event: {:?} sender: {:?} out of live window. mark as discard.", &authority_id, event_id, sender);
					return ValidationResult::Discard;
				}

				return ValidationResult::ProcessAndKeep(self.topic);
			} else {
				// TODO: decrease peer reputation
				warn!(target: "ethy", "💎 bad signature: {:?}, event: {:?}", authority_id, event_id);
			}
		}

		trace!(target: "ethy", "💎 invalid witness from sender: {:?}, could not decode: {:?}", sender, data);
		ValidationResult::Discard
	}

	fn message_expired<'a>(&'a self) -> Box<dyn FnMut(B::Hash, &[u8]) -> bool + 'a> {
		let complete_events = self.complete_events.read();
		Box::new(move |_topic, mut data| {
			let witness = match Witness::decode(&mut data) {
				Ok(w) => w,
				Err(_) => return true,
			};

			let finalized_number = self.backend.blockchain().info().finalized_number;
			if witness.block_number < finalized_number.into().saturating_sub(WINDOW_SIZE) {
				debug!(target: "ethy", "💎 Message for event #{} is out of live window. marked as expired: {}", witness.event_id, true);
				return true;
			}

			let expired = complete_events.binary_search(&witness.event_id).is_ok(); // spk
			trace!(target: "ethy", "💎 Message for event #{} expired: {}", witness.event_id, expired);

			expired
		})
	}

	#[allow(clippy::type_complexity)]
	fn message_allowed<'a>(
		&'a self,
	) -> Box<dyn FnMut(&PeerId, MessageIntent, &B::Hash, &[u8]) -> bool + 'a> {
		let do_rebroadcast = {
			let now = Instant::now();
			let mut next_rebroadcast = self.next_rebroadcast.lock();
			if now >= *next_rebroadcast {
				*next_rebroadcast = now + REBROADCAST_AFTER;
				true
			} else {
				false
			}
		};

		let complete_events = self.complete_events.read();
		Box::new(move |_who, intent, _topic, mut data| {
			if let MessageIntent::PeriodicRebroadcast = intent {
				return do_rebroadcast;
			}

			let witness = match Witness::decode(&mut data) {
				Ok(w) => w,
				Err(_) => return false,
			};

			let finalized_number = self.backend.blockchain().info().finalized_number;
			if witness.block_number < finalized_number.into().saturating_sub(WINDOW_SIZE) {
				debug!(target: "ethy", "💎 Message for event #{} is out of live window. marked as allowed: {}", witness.event_id, false);
				return false;
			}
			// Check if message is incomplete
			let allowed = complete_events.binary_search(&witness.event_id).is_err();

			trace!(target: "ethy", "💎 Message for round #{} allowed: {}", &witness.event_id, allowed);

			allowed
		})
	}
}

#[cfg(test)]
mod tests {
	use codec::Encode;
	use sc_network::PeerId;
	use sc_network_gossip::{MessageIntent, ValidationResult, Validator, ValidatorContext};
	use sc_network_test::{Block, Hash, TestNetFactory};
	use sp_core::keccak_256;

	use seed_primitives::ethy::{EthyChainId, Witness};

	use super::{GossipValidator, MAX_COMPLETE_EVENT_CACHE};
	use crate::{assert_validation_result, gossip::topic, testing::Keyring, tests::EthyTestNet};

	#[macro_export]
	/// sc_network_gossip::ValidationResult is missing Eq impl
	macro_rules! assert_validation_result {
		($l:pat, $r:ident) => {
			if let $l = $r {
				assert!(true);
			} else {
				assert!(false);
			}
		};
	}

	struct NoopContext;
	impl ValidatorContext<Block> for NoopContext {
		fn broadcast_topic(&mut self, _: Hash, _: bool) {}
		fn broadcast_message(&mut self, _: Hash, _: Vec<u8>, _: bool) {}
		fn send_message(&mut self, _: &PeerId, _: Vec<u8>) {}
		fn send_topic(&mut self, _: &PeerId, _: Hash, _: bool) {}
	}

	fn mock_signers() -> Vec<Keyring> {
		vec![Keyring::Alice, Keyring::Bob, Keyring::Charlie]
	}

	#[tokio::test]
	async fn verify_event_witness() {
		let validators = mock_signers();
		let alice = &validators[0];
		let mut context = NoopContext {};
		let sender_peer_id = PeerId::random();
		let mut net = EthyTestNet::new(1, 0);
		let backend = net.peer(0).client().as_backend();
		let gv = GossipValidator::new(vec![], backend);

		let event_id = 5;
		let message = b"hello world";
		let witness = Witness {
			digest: sp_core::keccak_256(message),
			chain_id: EthyChainId::Ethereum,
			event_id,
			validator_set_id: 123,
			authority_id: alice.public(),
			// 	fn sign(&self, message: &[u8]) -> Signature {
			// self.sign_prehashed(&blake2_256(message))
			signature: alice.sign(message),
			block_number: 0,
		}
		.encode();

		// check the witness, not a validator, discard
		let result = gv.validate(&mut context, &sender_peer_id, witness.as_ref());
		assert_validation_result!(ValidationResult::Discard, result);

		// set validtors, check witness again, ok
		gv.set_active_validators(validators.into_iter().map(|x| x.public()).collect());
		let result = gv.validate(&mut context, &sender_peer_id, witness.as_ref());
		assert_validation_result!(ValidationResult::ProcessAndKeep(_), result);
		assert!(gv.is_tracking_event(&event_id));

		// check the witness again, duplicate, discard
		let result = gv.validate(&mut context, &sender_peer_id, witness.as_ref());
		assert_validation_result!(ValidationResult::Discard, result);
	}

	#[tokio::test]
	async fn witness_bad_signature_discarded() {
		let validators = mock_signers();
		let alice = &validators[0];
		let bob = &validators[1];
		let mut net = EthyTestNet::new(1, 0);
		let backend = net.peer(0).client().as_backend();
		let gv =
			GossipValidator::new(validators.iter().map(|x| x.public().clone()).collect(), backend);

		let event_id = 5;
		let message = b"hello world";
		let witness = Witness {
			digest: keccak_256(message),
			chain_id: EthyChainId::Ethereum,
			event_id,
			validator_set_id: 123,
			authority_id: alice.public(),
			signature: bob.sign(message), // signed by bob
			block_number: 0,
		}
		.encode();

		// check the witness, not a validator, discard
		let result = gv.validate(&mut NoopContext {}, &PeerId::random(), witness.as_ref());
		assert_validation_result!(ValidationResult::Discard, result);
		assert!(!gv.is_tracking_event(&event_id));
	}

	#[tokio::test]
	async fn keeps_most_recent_events() {
		let mut net = EthyTestNet::new(1, 0);
		let backend = net.peer(0).client().as_backend();
		let gv = GossipValidator::new(vec![], backend);
		for event_id in 1..=MAX_COMPLETE_EVENT_CACHE {
			gv.mark_complete(event_id as u64);
		}
		gv.mark_complete(MAX_COMPLETE_EVENT_CACHE as u64 + 1);
		assert_eq!(gv.complete_events.read()[0], 2_u64);
		gv.mark_complete(MAX_COMPLETE_EVENT_CACHE as u64 + 2);
		assert_eq!(gv.complete_events.read()[0], 3_u64);

		assert_eq!(gv.complete_events.read().len(), MAX_COMPLETE_EVENT_CACHE);
	}

	#[tokio::test]
	async fn witness_validate_events_outside_live_window_discarded() {
		let validators = mock_signers();
		let alice = &validators[0];
		let mut context = NoopContext {};
		let sender_peer_id = PeerId::random();
		let mut net = EthyTestNet::new(1, 0);
		let backend = net.peer(0).client().as_backend();
		let gv = GossipValidator::new(vec![], backend);

		let event_id = 5;
		let message = b"hello world";
		let mut witness = Witness {
			digest: sp_core::keccak_256(message),
			chain_id: EthyChainId::Ethereum,
			event_id,
			validator_set_id: 123,
			authority_id: alice.public(),
			signature: alice.sign(message),
			block_number: 0,
		};

		// set validtors
		gv.set_active_validators(validators.into_iter().map(|x| x.public()).collect());

		// finalized number is 0 atm. validate now, should pass
		let result = gv.validate(&mut context, &sender_peer_id, witness.clone().encode().as_ref());
		assert_validation_result!(ValidationResult::ProcessAndKeep(_), result);
		assert!(gv.is_tracking_event(&event_id));

		// set the finalized block number to 7. try to validate now. should fail since out of live
		// window. i.e. WINDOW_SIZE = 5
		let block_hashes = net.peer(0).push_blocks(7, false);
		net.run_until_sync().await;

		assert_eq!(net.peer(0).client().justifications(block_hashes[6]).unwrap(), None);
		let just = (*b"FRNK", Vec::new());
		net.peer(0)
			.client()
			.finalize_block(block_hashes[6], Some(just.clone()), true)
			.unwrap();
		assert_eq!(
			net.peer(0).client().info().finalized_number,
			7,
			"Peer #{} finalized block number is not 7",
			0
		);

		// modify the event_id to avoid duplicate check
		witness.event_id += 1;
		// now validate, should fail since out of live window.
		let result = gv.validate(&mut context, &sender_peer_id, witness.clone().encode().as_ref());
		assert_validation_result!(ValidationResult::Discard, result);
	}

	#[tokio::test]
	async fn witness_expired_events_outside_live_window_discarded() {
		let validators = mock_signers();
		let alice = &validators[0];
		let mut net = EthyTestNet::new(1, 0);
		let backend = net.peer(0).client().as_backend();
		let gv = GossipValidator::new(vec![], backend);

		let event_id = 5;
		let message = b"hello world";
		let mut witness = Witness {
			digest: sp_core::keccak_256(message),
			chain_id: EthyChainId::Ethereum,
			event_id,
			validator_set_id: 123,
			authority_id: alice.public(),
			signature: alice.sign(message),
			block_number: 0,
		};

		// finalized number is 0 atm. check now, should give false
		let result = gv.message_expired()(topic::<Block>(), witness.clone().encode().as_ref());
		assert!(!result);

		// set the finalized block number to 7. try to validate now. should fail since out of live
		// window. i.e. WINDOW_SIZE = 5
		let block_hashes = net.peer(0).push_blocks(7, false);
		net.run_until_sync().await;

		assert_eq!(net.peer(0).client().justifications(block_hashes[6]).unwrap(), None);
		let just = (*b"FRNK", Vec::new());
		net.peer(0)
			.client()
			.finalize_block(block_hashes[6], Some(just.clone()), true)
			.unwrap();
		assert_eq!(
			net.peer(0).client().info().finalized_number,
			7,
			"Peer #{} finalized block number is not 7",
			0
		);

		// modify the event_id to avoid duplicate check
		witness.event_id += 1;
		// check now, should give true since out of live window.
		let result = gv.message_expired()(topic::<Block>(), witness.clone().encode().as_ref());
		assert!(result);
	}

	#[tokio::test]
	async fn witness_allowed_events_outside_live_window_discarded() {
		let validators = mock_signers();
		let alice = &validators[0];
		let mut net = EthyTestNet::new(1, 0);
		let backend = net.peer(0).client().as_backend();
		let gv = GossipValidator::new(vec![], backend);

		let event_id = 5;
		let message = b"hello world";
		let mut witness = Witness {
			digest: sp_core::keccak_256(message),
			chain_id: EthyChainId::Ethereum,
			event_id,
			validator_set_id: 123,
			authority_id: alice.public(),
			signature: alice.sign(message),
			block_number: 0,
		};

		// finalized number is 0 atm. check now, should give true
		let result = gv.message_allowed()(
			&PeerId::random(),
			MessageIntent::Broadcast,
			&topic::<Block>(),
			witness.clone().encode().as_ref(),
		);
		assert!(result);

		// set the finalized block number to 7. try to validate now. should fail since out of live
		// window. i.e. WINDOW_SIZE = 5
		let block_hashes = net.peer(0).push_blocks(7, false);
		net.run_until_sync().await;
		assert_eq!(net.peer(0).client().justifications(block_hashes[6]).unwrap(), None);
		let just = (*b"FRNK", Vec::new());
		net.peer(0)
			.client()
			.finalize_block(block_hashes[6], Some(just.clone()), true)
			.unwrap();
		assert_eq!(
			net.peer(0).client().info().finalized_number,
			7,
			"Peer #{} finalized block number is not 7",
			0
		);

		// modify the event_id to avoid duplicate check
		witness.event_id += 1;
		// check now, should give false since out of live window.
		let result = gv.message_allowed()(
			&PeerId::random(),
			MessageIntent::Broadcast,
			&topic::<Block>(),
			witness.clone().encode().as_ref(),
		);
		assert!(!result);
	}
}
