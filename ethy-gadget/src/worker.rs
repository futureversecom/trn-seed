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

use codec::{Codec, Decode, Encode};
use futures::StreamExt;
use log::{debug, error, info, trace, warn};
use sc_client_api::{Backend, FinalityNotification};
use sc_network_gossip::GossipEngine;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::SyncOracle;
use sp_runtime::{
	generic::OpaqueDigestItemId,
	traits::{Block, Convert, Header, One},
};
use std::{sync::Arc, time::Duration};

use seed_primitives::ethy::{
	crypto::AuthorityId as Public, ConsensusLog, EthyApi, EthyEcdsaToPublicKey, EventProof,
	EventProofId, ValidatorSet, VersionedEventProof, Witness, ETHY_ENGINE_ID,
	GENESIS_AUTHORITY_SET_ID,
};

use crate::{
	gossip::{topic, GossipValidator},
	keystore::EthyKeystore,
	metric_inc, metric_set,
	metrics::Metrics,
	notification,
	types::{data_to_digest, make_proof_key, EventMetadata, ProofRequest},
	witness_record::WitnessRecord,
	Client,
};
pub(crate) struct WorkerParams<B, BE, C, R, SO>
where
	B: Block,
	BE: Backend<B>,
{
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub key_store: EthyKeystore,
	pub event_proof_sender: notification::EthyEventProofSender,
	pub gossip_engine: GossipEngine<B>,
	pub gossip_validator: Arc<GossipValidator<B, BE>>,
	pub metrics: Option<Metrics>,
	pub sync_oracle: SO,
}

/// An ETHY worker plays the ETHY protocol
pub(crate) struct EthyWorker<B, C, BE, R, SO>
where
	B: Block,
	BE: Backend<B>,
	R: ProvideRuntimeApi<B>,
	R::Api: EthyApi<B>,
	C: Client<B, BE>,
	SO: SyncOracle + Send + Sync + Clone + 'static,
{
	client: Arc<C>,
	backend: Arc<BE>,
	runtime: Arc<R>,
	key_store: EthyKeystore,
	event_proof_sender: notification::EthyEventProofSender,
	gossip_engine: GossipEngine<B>,
	gossip_validator: Arc<GossipValidator<B, BE>>,
	metrics: Option<Metrics>,
	/// Tracks on-going witnesses
	witness_record: WitnessRecord,
	/// Best block we received a GRANDPA notification for
	best_grandpa_block_header: <B as Block>::Header,
	/// Current validator set
	validator_set: ValidatorSet<Public>,
	/// Handle to the sync oracle
	sync_oracle: SO,
}

impl<B, C, BE, R, SO> EthyWorker<B, C, BE, R, SO>
where
	B: Block + Codec,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: EthyApi<B>,
	SO: SyncOracle + Send + Sync + Clone + 'static,
{
	/// Return a new ETHY worker instance.
	///
	/// Note that a ETHY worker is only fully functional if a corresponding
	/// ETHY pallet has been deployed on-chain.
	///
	/// The ETHY pallet is needed in order to keep track of the ETHY authority set.
	pub(crate) fn new(worker_params: WorkerParams<B, BE, C, R, SO>) -> Self {
		let WorkerParams {
			client,
			backend,
			runtime,
			key_store,
			event_proof_sender,
			gossip_engine,
			gossip_validator,
			metrics,
			sync_oracle,
		} = worker_params;

		let last_finalized_header = client
			.expect_header(client.info().finalized_hash)
			.expect("latest block always has header available; qed.");

		EthyWorker {
			client,
			backend,
			runtime,
			key_store,
			event_proof_sender,
			gossip_engine,
			gossip_validator,
			metrics,
			best_grandpa_block_header: last_finalized_header,
			validator_set: ValidatorSet::empty(),
			witness_record: Default::default(),
			sync_oracle,
		}
	}
}

impl<B, C, BE, R, SO> EthyWorker<B, C, BE, R, SO>
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: EthyApi<B>,
	SO: SyncOracle + Send + Sync + Clone + 'static,
{
	/// Query the runtime state for validator set
	///
	/// Note that the validator set could be `None`. This is the case if we can't fetch the
	/// authority set from the Ethy on-chain state.
	/// Such a failure is usually an indication that the Ethy pallet has not been deployed (yet).
	fn validator_set(&self, header: &B::Header) -> Option<ValidatorSet<Public>> {
		// queries the Ethy pallet to get the active validator set public keys
		let validator_set = self.runtime.runtime_api().validator_set(header.hash()).ok();

		info!(target: "ethy", "💎 active validator set: {:?}", validator_set);
		validator_set
	}

	/// Return the signers authorized for signing XRPL messages
	/// It is always a subset of the total ethy `validator_set`
	///
	/// note: XRPL cannot make use of the total signer set and is limited to 8 total signers
	///
	/// Always query the chain state incase the authorized list changed
	fn xrpl_validator_set(&self, header: &B::Header) -> Option<ValidatorSet<Public>> {
		let xrpl_signers = self.runtime.runtime_api().xrpl_signers(header.hash()).ok();
		info!(target: "ethy", "💎 xrpl validator set: {:?}", xrpl_signers);

		xrpl_signers
	}

	/// Handle finality notification for non-signers (no locally available validator keys)
	fn handle_finality_notification_passive(&mut self, notification: FinalityNotification<B>) {
		for ProofRequest { chain_id, event_id, data, block } in
			extract_proof_requests::<B>(&notification.header).into_iter()
		{
			debug!(target: "ethy", "💎 noting event metadata: {:?}", event_id);
			self.witness_record.note_event_metadata(event_id, data, block, chain_id);
			// with the event metadata available we may be able to make a proof (provided there's
			// enough witnesses ready)
			self.try_make_proof(event_id);
		}
	}

	/// Handle processing finalized block events for active validators (requires local active, ethy
	/// key)
	fn handle_finality_notification_active(
		&mut self,
		notification: FinalityNotification<B>,
		authority_id: Public,
	) {
		let authority_public_key = EthyEcdsaToPublicKey::convert(authority_id.clone());
		// Search block header for ethy signing requests
		// Then sign and broadcast a witness
		for ProofRequest { chain_id, event_id, data, block } in
			extract_proof_requests::<B>(&notification.header).into_iter()
		{
			debug!(target: "ethy", "💎 got event proof request. chain_id: {:?}. event id: {:?}, data: {:?}", chain_id, event_id, hex::encode(&data));

			// `data` must be transformed into a 32 byte digest before signing
			let digest = match data_to_digest(chain_id, data.clone(), authority_public_key) {
				Some(d) => d,
				None => {
					error!(target: "ethy", "💎 error making digest: {:?}", event_id);
					continue;
				},
			};
			let signature = match self.key_store.sign_prehashed(&authority_id, &digest) {
				Ok(sig) => sig,
				Err(err) => {
					error!(target: "ethy", "💎 error signing witness: {:?}", err);
					continue;
				},
			};

			debug!(target: "ethy", "💎 signed event id: {:?}, validator set: {:?},\nsignature: {:?}", event_id, self.validator_set.id, hex::encode(&signature));

			let witness = Witness {
				chain_id,
				digest,
				validator_set_id: self.validator_set.id,
				event_id,
				authority_id: authority_id.clone(),
				signature,
				block_number: (*notification.header.number()).try_into().unwrap_or_default(),
			};

			metric_inc!(self, ethy_witness_sent);

			// process the witness
			self.witness_record.note_event_metadata(event_id, data, block, chain_id);
			self.handle_witness(witness.clone());
			debug!(target: "ethy", "💎 Sent witness: {:?}", witness);
		}
	}

	/// Check finalized blocks for proof requests
	fn handle_finality_notification(&mut self, notification: FinalityNotification<B>) {
		debug!(target: "ethy", "💎 finality notification: {:?}", notification);
		let new_header = notification.header.clone();
		let number = *new_header.number();

		// On start-up ignore old finality notifications that we're not interested in.
		if number <= *self.best_grandpa_block_header.number() {
			debug!(target: "ethy", "💎 unexpected finality for old block #{:?}", number);
			return;
		}

		// block finality notifications are un-reliable and may skip block numbers but ethy requires
		// all blocks are processed. ensure we backfill all blocks between the the last processed
		// block by ethy and the new finalized block notification
		if number > *self.best_grandpa_block_header.number() + One::one() {
			debug!(target: "ethy", "💎 finality notification for non-sequential future block #{:?}", number);
			match self.backend.blockchain().header(*new_header.parent_hash()) {
				Ok(Some(parent_header)) => {
					let mut n = notification.clone();
					n.hash = parent_header.hash();
					n.header = parent_header.clone();
					n.tree_route = Arc::new([]);
					n.stale_heads = Arc::new([]);

					self.handle_finality_notification(n);
				},
				Ok(None) => {
					error!(target: "ethy", "💎 missing prior block #{:?}", number - One::one())
				},
				Err(err) => {
					error!(target: "ethy", "💎 error fetching prior block #{:?}. {:?}", number - One::one(), err)
				},
			}
		}

		// Check the block for any validator set changes or the ethy-gadget validator set is empty
		if find_authorities_change::<B>(&new_header).is_some() || self.validator_set.is_empty() {
			match self.validator_set(&new_header) {
				Some(active) => {
					// if the validator set id is different or equal to the GENESIS_AUTHORITY_SET_ID
					// and local validator set is empty
					if active.id != self.validator_set.id
						|| (active.id == GENESIS_AUTHORITY_SET_ID && self.validator_set.is_empty())
					{
						info!(target: "ethy", "💎 new active validator set: {:?}", active);
						info!(target: "ethy", "💎 old validator set: {:?}", self.validator_set);
						metric_set!(self, ethy_validator_set_id, active.id);
						self.gossip_validator.set_active_validators(active.validators.clone());
						self.witness_record.set_validators(
							active.clone(),
							self.xrpl_validator_set(&new_header).unwrap_or_default(),
						);
						self.validator_set = active;
					}
				},
				None => {
					warn!(target: "ethy", "💎 Validator set is empty");
				},
			}
		}

		// Process proof requests
		if let Some(authority_id) =
			self.key_store.authority_id(self.validator_set.validators.as_slice())
		{
			trace!(target: "ethy", "💎 Local authority id: {:?}", authority_id);
			self.handle_finality_notification_active(notification, authority_id)
		} else {
			trace!(target: "ethy", "💎 No authority id - can't witness events in: {:?}", new_header.hash());
			self.handle_finality_notification_passive(notification)
		};

		self.best_grandpa_block_header = new_header;
	}

	/// Note an individual witness for a message
	fn handle_witness(&mut self, witness: Witness) {
		// The aggregated signed witness here could be different to another validators.
		// As long as we have threshold of signatures the proof is valid.
		info!(target: "ethy", "💎 got witness: {:?}", witness);

		// only share if it's the first time witnessing the event
		if let Err(err) = self.witness_record.note_event_witness(&witness) {
			warn!(target: "ethy", "💎 failed to note witness: {:?}, {:?}", witness, err);
			return;
		}

		// gossip the witness. will gossip even if we don't have event metadata yet. This would
		// increase the network activity, but gives more room for validator Witnesses to spread
		// across the network.
		trace!(target: "ethy", "💎 gossiping witness: {:?}", witness.event_id);
		self.gossip_engine.gossip_message(topic::<B>(), witness.encode(), false);

		// Try to make proof
		self.try_make_proof(witness.event_id);
	}

	/// Try to make an event proof
	///
	/// For a proof to be made successfully requires event metadata has been retrieved from a
	/// finalized block header and enough valid, corroborating witnesses are known
	///
	/// Process of making the proof is:
	/// 1) Assemble the aggregated witness' (proof)
	/// 2) Store proof in DB
	/// 3) Notify listeners of the new proof
	fn try_make_proof(&mut self, event_id: EventProofId) {
		let event_metadata = self.witness_record.event_metadata(event_id);
		if event_metadata.is_none() {
			debug!(target: "ethy", "💎 missing event metadata: {:?}, can't make proof yet", event_id);
			return;
		}

		// process any unverified witnesses, received before event metadata was known
		self.witness_record.process_unverified_witnesses(event_id);
		let EventMetadata { chain_id, block_hash, digest_data } =
			self.witness_record.event_metadata(event_id).unwrap();

		if self.witness_record.has_consensus(event_id, *chain_id) {
			let signatures = self.witness_record.signatures_for(event_id);
			info!(target: "ethy", "💎 generating proof for event: {:?}, signatures: {:?}, validator set: {:?}", event_id, signatures, self.validator_set.id);
			// NOTE: XRPL digest is unique per pubkey. For Ethereum it's the same as digest_data.
			// Anyway EventProof.digest is not used by any other part of the code
			let Some(digest) = data_to_digest(*chain_id, digest_data.clone(), [0_u8; 33]) else {
				error!(target: "ethy", "💎 error creating digest");
				return;
			};
			let event_proof = EventProof {
				digest,
				event_id,
				validator_set_id: self.validator_set.id,
				block: *block_hash,
				signatures: signatures.clone(),
			};

			let versioned_event_proof = VersionedEventProof::V1(event_proof.clone());

			// Add proof to the DB that this event has been notarized specifically by the
			// given threshold of validators
			// DB key is (engine_id + chain_id + proof_id)
			let proof_key = make_proof_key(*chain_id, event_proof.event_id);

			if let Err(err) = Backend::insert_aux(
				self.backend.as_ref(),
				&[(proof_key.as_ref(), versioned_event_proof.encode().as_ref())],
				&[],
			) {
				// this is a warning for now, because until the round lifecycle is improved, we will
				// conclude certain rounds multiple times.
				error!(target: "ethy", "💎 failed to store proof: {:?} for key [{:?}, {:?}]. Error received: {:?}", event_proof, proof_key, versioned_event_proof.encode(), err);
			}

			// Notify an subscribers that we've got a witness for a new message e.g. open RPC
			// subscriptions
			self.event_proof_sender
				.notify(|| Ok::<_, ()>(versioned_event_proof))
				.expect("forwards closure result; the closure always returns Ok; qed.");
			// Remove from memory
			self.witness_record.mark_complete(event_id);
			self.gossip_validator.mark_complete(event_id);
		} else {
			let debug_proof_key = make_proof_key(*chain_id, event_id);
			trace!(target: "ethy", "💎 no consensus for event: {:?}, can't make proof yet. Likely did not store proof for key {:?}", event_id, debug_proof_key);
		}
	}

	/// Main loop for Ethy worker.
	pub(crate) async fn run(mut self) {
		info!(target: "ethy", "💎 run Ethy worker, best finalized block: #{:?}.", self.best_grandpa_block_header.number());

		// wait for sync to complete before accepting ethy messages...
		while self.sync_oracle.is_major_syncing() {
			debug!(target: "ethy", "💎 Waiting for major sync to complete...");
			futures_timer::Delay::new(Duration::from_secs(4)).await;
		}

		let mut finality_notifications = self.client.finality_notification_stream().fuse();
		let mut witnesses = Box::pin(self.gossip_engine.messages_for(topic::<B>()).filter_map(
			|notification| async move {
				trace!(target: "ethy", "💎 got witness: {:?}", notification);

				Witness::decode(&mut &notification.message[..]).ok()
			},
		))
		.fuse();

		loop {
			while self.sync_oracle.is_major_syncing() {
				debug!(target: "ethy", "💎 Waiting for major sync to complete...");
				futures_timer::Delay::new(Duration::from_secs(4)).await;
			}

			let mut gossip_engine = &mut self.gossip_engine;
			futures::select! {
				notification = finality_notifications.next() => {
					if let Some(notification) = notification {
						self.handle_finality_notification(notification);
					} else {
						return;
					}
				},
				witness = witnesses.next() => {
					if let Some(witness) = witness {
						self.handle_witness(witness);
					} else {
						return;
					}
				},
				_ = gossip_engine => {
					error!(target: "ethy", "💎 Gossip engine has terminated.");
					return;
				}
			}
		}
	}
}

/// Extract event proof requests from a digest in the given header, if any.
/// Returns (digest for signing, event id, optional tag)
fn extract_proof_requests<B>(header: &B::Header) -> Vec<ProofRequest>
where
	B: Block,
{
	let block_hash = header.hash().as_ref().try_into().unwrap_or_default();
	header
		.digest()
		.logs()
		.iter()
		.flat_map(|log| {
			if let Some(ConsensusLog::OpaqueSigningRequest { chain_id, event_proof_id, data }) =
				log.try_to::<ConsensusLog<Public>>(OpaqueDigestItemId::Consensus(&ETHY_ENGINE_ID))
			{
				Some(ProofRequest { chain_id, event_id: event_proof_id, data, block: block_hash })
			} else {
				None
			}
		})
		.collect()
}

/// Scan the `header` digest log for an Ethy validator set change. Return either the new
/// validator set or `None` in case no validator set change has been signaled.
fn find_authorities_change<B>(header: &B::Header) -> Option<ValidatorSet<Public>>
where
	B: Block,
{
	let id = OpaqueDigestItemId::Consensus(&ETHY_ENGINE_ID);

	let filter = |log: ConsensusLog<Public>| match log {
		ConsensusLog::AuthoritiesChange(validator_set) => Some(validator_set),
		_ => None,
	};

	header.digest().convert_first(|l| l.try_to(id).and_then(filter))
}

#[cfg(test)]
pub(crate) mod test {
	use super::*;
	use crate::{
		notification::EthyEventProofTracingKey,
		testing::Keyring,
		tests::{
			create_ethy_keystore, make_ethy_ids, two_validators::TestApi, EthyPeer, EthyTestNet,
			ETHY_PROTOCOL_NAME,
		},
		witness_record::test::create_witness,
	};
	use sc_client_api::{AuxStore, FinalizeSummary};
	use sc_network_sync::service::chain_sync::SyncingService;
	use sc_network_test::{PeersFullClient, TestNetFactory};
	use sc_utils::{mpsc::tracing_unbounded, notification::NotificationStream};
	use seed_primitives::ethy::{crypto::AuthorityId, EthyChainId, ValidatorSet};
	use sp_api::HeaderT;
	use substrate_test_runtime_client::{
		runtime::{Block, Digest, DigestItem, Header},
		Backend,
	};

	fn get_proof(
		event_id: EventProofId,
		chain_id: EthyChainId,
		worker: &EthyWorker<Block, PeersFullClient, Backend, TestApi, Arc<SyncingService<Block>>>,
	) -> Option<EventProof> {
		if let Ok(maybe_encoded_proof) = worker.client.get_aux(
			[
				ETHY_ENGINE_ID.as_slice(),
				([chain_id.into()].as_slice()),
				(event_id.to_be_bytes().as_slice()),
			]
			.concat()
			.as_ref(),
		) {
			if let Some(encoded_proof) = maybe_encoded_proof {
				if let Ok(versioned_proof) = VersionedEventProof::decode(&mut &encoded_proof[..]) {
					match versioned_proof {
						VersionedEventProof::V1(event_proof) => return Some(event_proof),
					}
				}
			}
		}
		None
	}

	fn create_ethy_worker(
		peer: &EthyPeer,
		key: &Keyring,
		validators: Vec<AuthorityId>,
	) -> EthyWorker<Block, PeersFullClient, Backend, TestApi, Arc<SyncingService<Block>>> {
		let keystore = create_ethy_keystore(*key);
		let api = Arc::new(TestApi {});
		let network = peer.network_service().clone();
		let sync_oracle = peer.sync_service().clone();
		let gossip_validator =
			Arc::new(crate::gossip::GossipValidator::new(validators, peer.client().as_backend()));
		let gossip_engine = GossipEngine::new(
			network,
			sync_oracle.clone(),
			ETHY_PROTOCOL_NAME,
			gossip_validator.clone(),
			None,
		);
		let (sender, _receiver) = NotificationStream::<_, EthyEventProofTracingKey>::channel();

		let worker_params = crate::worker::WorkerParams {
			client: peer.client().as_client(),
			backend: peer.client().as_backend(),
			runtime: api,
			key_store: Some(keystore).into(),
			event_proof_sender: sender,
			gossip_engine,
			gossip_validator,
			metrics: None,
			sync_oracle,
		};
		EthyWorker::<_, _, _, _, _>::new(worker_params)
	}

	#[tokio::test]
	async fn handle_witness_works() {
		let keys = &[Keyring::Alice, Keyring::Bob];
		let validators = make_ethy_ids(keys);
		let mut net = EthyTestNet::new(1, 0);
		let mut worker = create_ethy_worker(net.peer(0), &keys[0], validators.clone());

		// Create validator set with proof threshold of 2
		let validator_set = ValidatorSet { validators, id: 1, proof_threshold: 2 };
		worker.witness_record.set_validators(validator_set.clone(), validator_set);

		let event_id: EventProofId = 5;
		let chain_id = EthyChainId::Ethereum;
		let digest = [1_u8; 32];

		// Create witness for Alice
		let witness_1 = create_witness(&keys[0], event_id, chain_id, digest);

		// Manually enter event metadata
		// TODO, find a way for the worker to do this, rather than injecting metadata manually
		worker
			.witness_record
			.note_event_metadata(event_id, digest.to_vec(), [2_u8; 32], chain_id);

		// Handle the witness
		worker.handle_witness(witness_1);
		// Check we have 1 signature
		assert_eq!(worker.witness_record.signatures_for(event_id).len(), 1);

		let witness_2 = create_witness(&keys[1], event_id, chain_id, digest);
		worker.handle_witness(witness_2);

		// Check we have 0 signatures. The event should have reached consensus and  witness
		// signatures removed
		assert_eq!(worker.witness_record.signatures_for(event_id).len(), 0);

		// check for proof in the aux store
		let proof = get_proof(event_id, EthyChainId::Ethereum, &worker);
		assert_eq!(proof.unwrap().event_id, event_id);
	}

	#[tokio::test]
	async fn handle_witness_first_two_events() {
		let keys = &[Keyring::Alice, Keyring::Bob];
		let validators = make_ethy_ids(keys);
		let mut net = EthyTestNet::new(1, 0);
		let mut worker = create_ethy_worker(net.peer(0), &keys[0], validators.clone());

		// Create validator set with proof threshold of 2
		let validator_set = ValidatorSet { validators, id: 1, proof_threshold: 2 };
		worker.witness_record.set_validators(validator_set.clone(), validator_set);

		// First event to be processed is event id 2, to simulate out of sync events with XRPL and
		// Ethereum Events
		let event_id_2: EventProofId = 2;
		let chain_id = EthyChainId::Ethereum;
		let digest = [1_u8; 32];

		// Create witness for Alice
		let witness_1 = create_witness(&keys[0], event_id_2, chain_id, digest);
		worker.witness_record.note_event_metadata(
			event_id_2,
			digest.to_vec(),
			[2_u8; 32],
			chain_id,
		);
		worker.handle_witness(witness_1);
		assert_eq!(worker.witness_record.signatures_for(event_id_2).len(), 1);

		// Create witness for Bob
		let witness_2 = create_witness(&keys[1], event_id_2, chain_id, digest);
		worker.handle_witness(witness_2);
		assert_eq!(worker.witness_record.signatures_for(event_id_2).len(), 0);

		// Second event to be processed is event id 1, an XRPL event
		let event_id_1: EventProofId = 1;
		let chain_id = EthyChainId::Xrpl;
		let digest = [2_u8; 32];

		// Create witness for Alice
		let witness_1 = create_witness(&keys[0], event_id_1, chain_id, digest);
		worker.witness_record.note_event_metadata(
			event_id_1,
			digest.to_vec(),
			[2_u8; 32],
			chain_id,
		);
		worker.handle_witness(witness_1);
		assert_eq!(worker.witness_record.signatures_for(event_id_1).len(), 1);

		// Create witness for Bob
		let witness_2 = create_witness(&keys[1], event_id_1, chain_id, digest);
		worker.handle_witness(witness_2);

		// Check we have 0 signatures. The event should have reached consensus and  witness
		// signatures removed
		assert_eq!(worker.witness_record.signatures_for(event_id_1).len(), 0);

		// Now we attempt to process event 0, which should not go through as completed_events now
		// contains events [1,2]
		let event_id_0: EventProofId = 0;
		let chain_id = EthyChainId::Ethereum;
		let digest = [3_u8; 32];

		// Create witness for Alice
		let witness_1 = create_witness(&keys[0], event_id_0, chain_id, digest);
		worker.witness_record.note_event_metadata(
			event_id_0,
			digest.to_vec(),
			[2_u8; 32],
			chain_id,
		);
		worker.handle_witness(witness_1);
		assert_eq!(worker.witness_record.signatures_for(event_id_0).len(), 0);

		// check for proof in the aux store
		let proof = get_proof(event_id_1, EthyChainId::Xrpl, &worker);
		assert_eq!(proof.unwrap().event_id, event_id_1);
		let proof = get_proof(event_id_2, EthyChainId::Ethereum, &worker);
		assert_eq!(proof.unwrap().event_id, event_id_2);
		let proof = get_proof(event_id_0, EthyChainId::Ethereum, &worker);
		assert_eq!(proof, None);
	}

	#[test]
	fn extract_authorities_change_digest() {
		let mut header = Header::new(
			1u32.into(),
			Default::default(),
			Default::default(),
			Default::default(),
			Digest::default(),
		);

		// verify empty digest shows nothing
		assert!(find_authorities_change::<Block>(&header).is_none());

		let id = 42;
		let validators = make_ethy_ids(&[Keyring::Alice, Keyring::Bob]);
		let validator_set = ValidatorSet { validators, id, proof_threshold: 2 };
		header.digest_mut().push(DigestItem::Consensus(
			ETHY_ENGINE_ID,
			ConsensusLog::<Public>::AuthoritiesChange(validator_set.clone()).encode(),
		));

		// verify validator set is correctly extracted from digest
		let extracted = find_authorities_change::<Block>(&header);
		assert_eq!(extracted, Some(validator_set));
	}

	#[tokio::test]
	async fn extract_validators_from_the_runtime_and_not_from_header() {
		let keys = &[Keyring::Alice, Keyring::Bob];
		let runtime_validators = make_ethy_ids(keys);
		let header_validators = make_ethy_ids(&[Keyring::Alice, Keyring::Bob, Keyring::Charlie]);
		let mut net = EthyTestNet::new(1, 0);
		let mut worker = create_ethy_worker(net.peer(0), &keys[0], runtime_validators.clone());

		let mut header = Header::new(
			1u32.into(),
			Default::default(),
			Default::default(),
			Default::default(),
			Digest::default(),
		);
		let header_validator_set =
			ValidatorSet { validators: header_validators, id: 1_u64, proof_threshold: 3 };
		header.digest_mut().push(DigestItem::Consensus(
			ETHY_ENGINE_ID,
			ConsensusLog::<Public>::AuthoritiesChange(header_validator_set.clone()).encode(),
		));
		let (sink, _stream) = tracing_unbounded("test_sink", 100_000);
		let summary = FinalizeSummary {
			header: header.clone(),
			finalized: vec![header.hash()],
			stale_heads: vec![],
		};
		let finality_notification = FinalityNotification::from_summary(summary, sink);

		worker.handle_finality_notification(finality_notification);

		// stored validator set should be extracted from runtime. not from the block header. i.e
		// same as value in runtime_validators above
		assert_eq!(
			worker.witness_record.get_validator_set(),
			ValidatorSet { validators: runtime_validators.clone(), id: 0_u64, proof_threshold: 2 }
		);
		assert_eq!(
			worker.witness_record.get_xrpl_validator_set(),
			ValidatorSet { validators: runtime_validators, id: 0_u64, proof_threshold: 2 }
		);
	}
}
