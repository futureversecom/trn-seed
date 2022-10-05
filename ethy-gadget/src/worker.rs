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

use std::{sync::Arc, time::Duration};

use codec::{Codec, Decode, Encode};
use futures::StreamExt;
use log::{debug, error, info, trace, warn};
use sc_client_api::{Backend, FinalityNotification};
use sc_network_gossip::GossipEngine;
use sp_api::BlockId;
use sp_blockchain::HeaderBackend;
use sp_consensus::SyncOracle;
use sp_runtime::{
	generic::OpaqueDigestItemId,
	traits::{Block, Convert, Header, One},
};

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
pub(crate) struct WorkerParams<B, BE, C, SO>
where
	B: Block,
{
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub key_store: EthyKeystore,
	pub event_proof_sender: notification::EthyEventProofSender,
	pub gossip_engine: GossipEngine<B>,
	pub gossip_validator: Arc<GossipValidator<B>>,
	pub metrics: Option<Metrics>,
	pub sync_oracle: SO,
}

/// An ETHY worker plays the ETHY protocol
pub(crate) struct EthyWorker<B, C, BE, SO>
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
{
	client: Arc<C>,
	backend: Arc<BE>,
	key_store: EthyKeystore,
	event_proof_sender: notification::EthyEventProofSender,
	gossip_engine: GossipEngine<B>,
	gossip_validator: Arc<GossipValidator<B>>,
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

impl<B, C, BE, SO> EthyWorker<B, C, BE, SO>
where
	B: Block + Codec,
	BE: Backend<B>,
	C: Client<B, BE>,
	C::Api: EthyApi<B>,
	SO: SyncOracle + Send + Sync + Clone + 'static,
{
	/// Return a new ETHY worker instance.
	///
	/// Note that a ETHY worker is only fully functional if a corresponding
	/// ETHY pallet has been deployed on-chain.
	///
	/// The ETHY pallet is needed in order to keep track of the ETHY authority set.
	pub(crate) fn new(worker_params: WorkerParams<B, BE, C, SO>) -> Self {
		let WorkerParams {
			client,
			backend,
			key_store,
			event_proof_sender,
			gossip_engine,
			gossip_validator,
			metrics,
			sync_oracle,
		} = worker_params;

		let last_finalized_header = client
			.expect_header(BlockId::number(client.info().finalized_number))
			.expect("latest block always has header available; qed.");

		EthyWorker {
			client,
			backend,
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

impl<B, C, BE, SO> EthyWorker<B, C, BE, SO>
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	C::Api: EthyApi<B>,
	SO: SyncOracle + Send + Sync + Clone + 'static,
{
	/// Return the current active validator set at header `header`.
	///
	/// Note that the validator set could be `None`. This is the case if we don't find
	/// a ETHY authority set change and we can't fetch the authority set from the
	/// ETHY on-chain state.
	///
	/// Such a failure is usually an indication that the ETHY pallet has not been deployed (yet).
	fn validator_set(&self, header: &B::Header) -> Option<ValidatorSet<Public>> {
		let new = if let Some(new) = find_authorities_change::<B>(header) {
			Some(new)
		} else {
			let at = BlockId::hash(header.hash());
			// queries the Ethy pallet to get the active validator set public keys
			self.client.runtime_api().validator_set(&at).ok()
		};

		trace!(target: "ethy", "💎 active validator set: {:?}", new);

		new
	}

	/// Handle finality notification for non-signers (no locally available validator keys)
	fn handle_finality_notification_passive(&mut self, notification: FinalityNotification<B>) {
		for ProofRequest { chain_id, event_id, data, block } in
			extract_proof_requests::<B>(&notification.header).into_iter()
		{
			trace!(target: "ethy", "💎 noting event metadata: {:?}", event_id);

			// it's possible the event already has a proof stored e.g.
			// ethy protocol completed by validators for the event and broadcast prior to the
			// finalized block being imported locally if so update the proof's block hash
			let proof_key = make_proof_key(chain_id, event_id);
			let get_proof = Backend::get_aux(self.backend.as_ref(), proof_key.as_ref());

			// Try update the existing proof if it exists
			if let Ok(Some(encoded_proof)) = get_proof {
				if let Ok(VersionedEventProof::V1 { 0: mut proof }) =
					VersionedEventProof::decode(&mut &encoded_proof[..])
				{
					proof.block = block;
					if let Err(err) = Backend::insert_aux(
						self.backend.as_ref(),
						&[(proof_key.as_ref(), VersionedEventProof::V1(proof).encode().as_ref())],
						&[],
					) {
						error!(target: "ethy", "💎 failed to update existing proof: {:?}, {:?}", event_id, err);
						continue
					}
				} else {
					error!(target: "ethy", "💎 failed decoding event proof v1: {:?}", event_id);
					continue
				}
			}

			let digest = match data_to_digest(chain_id, data, [0_u8; 33]) {
				Some(d) => d,
				None => {
					error!(target: "ethy", "💎 error making digest: {:?}", event_id);
					continue
				},
			};

			self.witness_record.note_event_metadata(event_id, digest, block, chain_id);
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
			let digest = match data_to_digest(chain_id, data, authority_public_key) {
				Some(d) => d,
				None => {
					error!(target: "ethy", "💎 error making digest: {:?}", event_id);
					continue
				},
			};
			let signature = match self.key_store.sign_prehashed(&authority_id, &digest) {
				Ok(sig) => sig,
				Err(err) => {
					error!(target: "ethy", "💎 error signing witness: {:?}", err);
					continue
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
			};
			let broadcast_witness = witness.encode();

			metric_inc!(self, ethy_witness_sent);
			debug!(target: "ethy", "💎 Sent witness: {:?}", witness);

			// process the witness
			self.witness_record.note_event_metadata(event_id, digest, block, chain_id);
			self.handle_witness(witness.clone());

			// broadcast the witness
			self.gossip_engine.gossip_message(topic::<B>(), broadcast_witness, false);
			debug!(target: "ethy", "💎 gossiped witness for event: {:?}", witness.event_id);
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
			return
		}

		// block finality notifications are un-reliable and may skip block numbers but ethy requires
		// all blocks are processed. ensure we backfill all blocks between the the last processed
		// block by ethy and the new finalized block notification
		if number > *self.best_grandpa_block_header.number() + One::one() {
			debug!(target: "ethy", "💎 finality notification for non-sequential future block #{:?}", number);
			match self.backend.blockchain().header(BlockId::Number(number - One::one())) {
				Ok(Some(parent_header)) => {
					let n = FinalityNotification {
						hash: parent_header.hash(),
						header: parent_header.clone(),
						// these fields are unused by ethy
						tree_route: Arc::new([]),
						stale_heads: Arc::new([]),
					};
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

		// Check the block for any validator set changes and update local view
		if let Some(active) = self.validator_set(&new_header) {
			// Authority set change or genesis set id triggers new voting rounds
			// this block has a different validator set id to the one we know about OR
			// it's the first block
			if self.validator_set.validators.is_empty() ||
				active.id != self.validator_set.id ||
				active.id == GENESIS_AUTHORITY_SET_ID && self.validator_set.validators.is_empty()
			{
				debug!(target: "ethy", "💎 new active validator set: {:?}", active);
				debug!(target: "ethy", "💎 old validator set: {:?}", self.validator_set);
				metric_set!(self, ethy_validator_set_id, active.id);
				self.gossip_validator.set_active_validators(active.validators.clone());
				self.witness_record.set_validators(active.validators.clone());
				self.validator_set = active;
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
			return
		}

		self.gossip_engine.gossip_message(topic::<B>(), witness.encode(), false);
		// after processing `witness` there may now be enough info to make a proof
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
		{
			let event_metadata = self.witness_record.event_metadata(event_id);
			if event_metadata.is_none() {
				debug!(target: "ethy", "💎 missing event metadata: {:?}, can't make proof yet", event_id);
				return
			}
		}

		// process any unverified witnesses, received before event metadata was known
		self.witness_record.process_unverified_witnesses(event_id);
		let EventMetadata { chain_id, block_hash, digest } =
			self.witness_record.event_metadata(event_id).unwrap();

		let proof_threshold = self.validator_set.proof_threshold as usize;
		if proof_threshold < self.validator_set.validators.len() / 2 {
			// safety check, < 50% doesn't make sense
			error!(target: "ethy", "💎 Ethy proof threshold too low!: {:?}, validator set: {:?}", proof_threshold, self.validator_set.validators.len());
			return
		}

		// TODO: if chain_id is XRPL this must be a majority of the XRPL validators only, not any
		// majority
		if self.witness_record.has_consensus(event_id, proof_threshold) {
			let signatures = self.witness_record.signatures_for(event_id);
			info!(target: "ethy", "💎 generating proof for event: {:?}, signatures: {:?}, validator set: {:?}", event_id, signatures, self.validator_set.id);

			let event_proof = EventProof {
				digest: *digest,
				event_id,
				validator_set_id: self.validator_set.id,
				block: *block_hash,
				signatures,
			};
			let versioned_event_proof = VersionedEventProof::V1(event_proof.clone());

			// Add proof to the DB that this event has been notarized specifically by the
			// given threshold of validators
			// DB key is (engine_id + chain_id + proof_id)
			let proof_key = make_proof_key(*chain_id, event_proof.event_id);
			if Backend::insert_aux(
				self.backend.as_ref(),
				&[(proof_key.as_ref(), versioned_event_proof.encode().as_ref())],
				&[],
			)
			.is_err()
			{
				// this is a warning for now, because until the round lifecycle is improved, we will
				// conclude certain rounds multiple times.
				warn!(target: "ethy", "💎 failed to store proof: {:?}", event_proof);
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
			trace!(target: "ethy", "💎 no consensus for event: {:?}, can't make proof yet", event_id);
		}
	}

	/// Main loop for Ethy worker.
	pub(crate) async fn run(mut self) {
		debug!(target: "Ethy", "💎 run Ethy worker, best finalized block: #{:?}.", self.best_grandpa_block_header.number());

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
	use substrate_test_runtime_client::runtime::{Block, Digest, DigestItem};

	use crate::testing::Keyring;
	use seed_primitives::ethy::ValidatorSet;

	pub(crate) fn make_ethy_ids(keys: &[Keyring]) -> Vec<Public> {
		keys.iter().map(|key| key.clone().public().into()).collect()
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
}
