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
use sp_consensus::SyncOracle;
use sp_runtime::{
	generic::OpaqueDigestItemId,
	traits::{Block, Convert, Header},
};

use seed_primitives::{
	ethy::{
		crypto::AuthorityId as Public, ConsensusLog, EthyApi, EventProof, EventProofId,
		PendingAuthorityChange, ValidatorSet, ValidatorSetId, VersionedEventProof, Witness,
		ETHY_ENGINE_ID, GENESIS_AUTHORITY_SET_ID,
	},
	AccountId,
};

use crate::{
	gossip::{topic, GossipValidator},
	keystore::{EthyEcdsaToEthereum, EthyKeystore},
	metric_inc, metric_set,
	metrics::Metrics,
	notification,
	witness_record::{EventMetadata, WitnessRecord},
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
			client: client.clone(),
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

	/// Check finalized blocks for proof requests
	fn handle_finality_notification(&mut self, notification: FinalityNotification<B>) {
		debug!(target: "ethy", "💎 finality notification: {:?}", notification);
		let number = *notification.header.number();

		// On start-up ignore old finality notifications that we're not interested in.
		if number <= *self.best_grandpa_block_header.number() {
			debug!(target: "ethy", "💎 Got unexpected finality for old block #{:?}", number);
			return
		}

		if let Some(active) = self.validator_set(&notification.header) {
			// Authority set change or genesis set id triggers new voting rounds
			// this block has a different validator set id to the one we know about OR
			// it's the first block
			if active.id != self.validator_set.id ||
				(active.id == GENESIS_AUTHORITY_SET_ID &&
					self.validator_set.validators.is_empty())
			{
				debug!(target: "ethy", "💎 new active validator set: {:?}", active);
				debug!(target: "ethy", "💎 old validator set: {:?}", self.validator_set);
				metric_set!(self, ethy_validator_set_id, active.id);
				self.gossip_validator.set_active_validators(active.validators.clone());
				self.witness_record.set_validators(active.validators.clone());
				self.validator_set = active;
			}
		}

		let authority_id = if let Some(id) =
			self.key_store.authority_id(self.validator_set.validators.as_slice())
		{
			trace!(target: "ethy", "💎 Local authority id: {:?}", id);
			id
		} else {
			trace!(target: "ethy", "💎 No authority id - can't vote for events in: {:?}", notification.header.hash());
			for ProofRequest { message, event_id, tag, block } in
				extract_proof_requests::<B>(&notification.header, self.validator_set.id).into_iter()
			{
				trace!(target: "ethy", "💎 noting event metadata: {:?}", event_id);
				// it's possible this event already has a proof stored due to differences in block
				// propagation times.
				// update the proof block hash and tag
				let proof_key = [&ETHY_ENGINE_ID[..], &event_id.to_be_bytes()[..]].concat();

				if let Ok(Some(encoded_proof)) =
					Backend::get_aux(self.backend.as_ref(), proof_key.as_ref())
				{
					if let Ok(VersionedEventProof::V1 { 0: mut proof }) =
						VersionedEventProof::decode(&mut &encoded_proof[..])
					{
						proof.block = block;
						proof.tag = tag;

						if Backend::insert_aux(
							self.backend.as_ref(),
							&[
								// DB key is (engine_id + proof_id)
								(
									[&ETHY_ENGINE_ID[..], &event_id.to_be_bytes()[..]]
										.concat()
										.as_ref(),
									VersionedEventProof::V1(proof).encode().as_ref(),
								),
							],
							&[],
						)
						.is_err()
						{
							// this is a warning for now, because until the round lifecycle is
							// improved, we will conclude certain rounds multiple times.
							error!(target: "ethy", "💎 failed to store proof: {:?}", event_id);
						}
					} else {
						error!(target: "ethy", "💎 failed decoding event proof v1: {:?}", event_id);
					}
				} else {
					// no proof is known for this event yet
					let event_digest = sp_core::keccak_256(message.as_ref());
					self.witness_record.note_event_metadata(event_id, event_digest, block, tag);
				}
			}

			// full node can't vote, we're done
			return
		};

		// Search from (self.best_grandpa_block_header - notification.block) to find all signing
		// requests Sign and broadcast a witness
		for ProofRequest { message, event_id, tag, block } in
			extract_proof_requests::<B>(&notification.header, self.validator_set.id).into_iter()
		{
			debug!(target: "ethy", "💎 got event proof request. event id: {:?}, message: {:?}", event_id, hex::encode(&message));
			// `message = abi.encode(param0, param1,.., paramN, nonce)`
			let signature = match self.key_store.sign(&authority_id, message.as_ref()) {
				Ok(sig) => sig,
				Err(err) => {
					error!(target: "ethy", "💎 error signing witness: {:?}", err);
					return
				},
			};
			debug!(target: "ethy", "💎 signed event id: {:?}, validator set: {:?},\nsignature: {:?}", event_id, self.validator_set.id, hex::encode(&signature));
			let event_digest = sp_core::keccak_256(message.as_ref());
			let witness = Witness {
				digest: event_digest,
				validator_set_id: self.validator_set.id,
				event_id,
				authority_id: authority_id.clone(),
				signature,
			};
			let broadcast_witness = witness.encode();

			metric_inc!(self, ethy_witness_sent);
			debug!(target: "ethy", "💎 Sent witness: {:?}", witness);

			// process the witness
			self.witness_record.note_event_metadata(event_id, event_digest, block, tag);
			self.handle_witness(witness.clone());

			// broadcast the witness
			self.gossip_engine.gossip_message(topic::<B>(), broadcast_witness, false);
			debug!(target: "ethy", "💎 gossiped witness for event: {:?}", witness.event_id);
		}

		self.best_grandpa_block_header = notification.header;
	}

	/// Note an individual witness for a message
	/// If the witness means consensus is reached on a message then;
	/// 1) Assemble the aggregated witness (proof)
	/// 2) Add proof to DB
	/// 3) Notify listeners of the proof
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

		let proof_threshold = self.validator_set.proof_threshold as usize;
		if proof_threshold < self.validator_set.validators.len() / 2 {
			// safety check, < 50% doesn't make sense
			error!(target: "ethy", "💎 Ethy proof threshold too low!: {:?}, validator set: {:?}", proof_threshold, self.validator_set.validators.len());
			return
		}

		if self.witness_record.has_consensus(witness.event_id, proof_threshold) {
			let signatures = self.witness_record.signatures_for(witness.event_id);
			info!(target: "ethy", "💎 generating proof for event: {:?}, signatures: {:?}, validator set: {:?}", witness.event_id, signatures, self.validator_set.id);

			let maybe_event_metadata = self.witness_record.event_metadata(witness.event_id);
			if maybe_event_metadata.is_none() {
				error!(target: "ethy", "💎 missing event metadata: {:?}", witness.event_id);
				return
			}
			let EventMetadata { tag, block_hash, .. } = maybe_event_metadata.unwrap();

			let event_proof = EventProof {
				digest: witness.digest,
				event_id: witness.event_id,
				validator_set_id: self.validator_set.id,
				block: *block_hash,
				tag: tag.clone(),
				signatures,
			};
			let versioned_event_proof = VersionedEventProof::V1(event_proof.clone());

			// Add proof to the DB that this event has been notarized specifically by the
			// given threshold of validators
			if Backend::insert_aux(
				self.backend.as_ref(),
				&[
					// DB key is (engine_id + proof_id)
					(
						[&ETHY_ENGINE_ID[..], &event_proof.event_id.to_be_bytes()[..]]
							.concat()
							.as_ref(),
						versioned_event_proof.encode().as_ref(),
					),
				],
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
			self.witness_record.mark_complete(witness.event_id);
			self.gossip_validator.mark_complete(witness.event_id);
		} else {
			trace!(target: "ethy", "💎 no consensus yet for event: {:?}", witness.event_id);
		}
	}

	/// Main loop for Ethy worker.
	pub(crate) async fn run(mut self) {
		debug!(target: "Ethy", "💎 run Ethy worker, best finalized block: #{:?}.", self.best_grandpa_block_header.number());

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

pub struct ProofRequest {
	/// raw message for signing
	message: Vec<u8>,
	/// nonce/event Id of this request
	event_id: EventProofId,
	/// metadata tag about the proof e.g. denotes the proof is for an authority set change
	tag: Option<Vec<u8>>,
	/// Finalized block hash when the proof was requested
	block: [u8; 32],
}
/// Extract event proof requests from a digest in the given header, if any.
/// Returns (digest for signing, event id, optional tag)
fn extract_proof_requests<B>(
	header: &B::Header,
	active_validator_set_id: ValidatorSetId,
) -> Vec<ProofRequest>
where
	B: Block,
{
	let block_hash = header.hash().as_ref().try_into().unwrap_or_default();
	header
		.digest()
		.logs()
		.iter()
		.flat_map(|log| {
			let res: Option<ProofRequest> = match log
				.try_to::<ConsensusLog<Public>>(OpaqueDigestItemId::Consensus(&ETHY_ENGINE_ID))
			{
				Some(ConsensusLog::OpaqueSigningRequest((message, event_id))) =>
					Some(ProofRequest { message, event_id, tag: None, block: block_hash }),
				// Note: we also handle this in `find_authorities_change` to update the validator
				// set here we want to convert it into an 'OpaqueSigningRequest` to create a proof
				// of the validator set change we must do this before the validators officially
				// change next session
				Some(ConsensusLog::PendingAuthoritiesChange(PendingAuthorityChange {
					source,
					destination,
					next_validator_set,
					event_proof_id,
				})) => {
					let message = eth_abi_encode_validator_set_change(
						source,
						destination,
						&next_validator_set,
						active_validator_set_id,
						event_proof_id,
					);
					Some(ProofRequest {
						message,
						event_id: event_proof_id,
						tag: Some(b"sys:authority-change".to_vec()),
						block: block_hash,
					})
				},
				_ => None,
			};
			res
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

/// Ethereum ABI encode a validator set change message
/// - `bridge_pallet_address` The ethy pallet address (source)
/// - `bridge_contract_address` The ethereum bridge contract address (destination)
/// - `next_validator_set` Ordered list of validator public keys (secp256k1)
/// - `proof_validator_set_id ` the id of the current validator set (acting as witnesses)
/// - `proof_event_id` Id of this outgoing event
fn eth_abi_encode_validator_set_change(
	bridge_pallet_address: AccountId,
	bridge_contract_address: AccountId,
	next_validator_set: &ValidatorSet<Public>,
	proof_validator_set_id: ValidatorSetId,
	proof_event_id: EventProofId,
) -> Vec<u8> {
	// Convert the validator ECDSA pub keys to addresses
	let next_validator_addresses: Vec<ethabi::Token> = next_validator_set
		.validators
		.iter()
		.map(|ecdsa_pub| EthyEcdsaToEthereum::convert(ecdsa_pub.clone()))
		.map(|a| ethabi::Token::Address(a.into()))
		.collect();

	// bridge contract specific message
	let app_message = ethabi::encode(&[
		ethabi::Token::Array(next_validator_addresses),
		ethabi::Token::Uint(next_validator_set.id.into()),
	]);

	// wrap event message
	ethabi::encode(&[
		// event source address
		ethabi::Token::Address(bridge_pallet_address.into()),
		// event destination address
		ethabi::Token::Address(bridge_contract_address.into()),
		// event data
		ethabi::Token::Bytes(app_message),
		// proof parameters
		ethabi::Token::Uint(proof_validator_set_id.into()),
		ethabi::Token::Uint(proof_event_id.into()),
	])
}

#[cfg(test)]
pub(crate) mod test {
	use super::*;
	use sp_application_crypto::ByteArray;
	use sp_core::H160;
	use substrate_test_runtime_client::runtime::{Block, Digest, DigestItem};

	use crate::testing::Keyring;
	use seed_primitives::ethy::ValidatorSet;

	pub(crate) fn make_ethy_ids(keys: &[Keyring]) -> Vec<Public> {
		keys.iter().map(|key| key.clone().public().into()).collect()
	}

	#[test]
	fn encode_validator_set_change() {
		let abi_encoded =
			eth_abi_encode_validator_set_change(
				H160::from_low_u64_be(111_u64).into(),
				H160::from_low_u64_be(222_u64).into(),
				&ValidatorSet::<Public> {
					validators: vec![
					Public::from_slice(
						// `//Alice` ECDSA public key
						&hex::decode(b"0204dad6fc9c291c68498de501c6d6d17bfe28aee69cfbf71b2cc849caafcb0159").unwrap(),
					)
					.unwrap(),
					Public::from_slice(
						// `//Alice` ECDSA public key
						&hex::decode(b"0204dad6fc9c291c68498de501c6d6d17bfe28aee69cfbf71b2cc849caafcb0159").unwrap(),
					)
					.unwrap(),
				],
					id: 598,
					proof_threshold: 2,
				},
				599,
				1_234_567,
			);
		assert_eq!(
			hex::encode(abi_encoded),
			"000000000000000000000000000000000000000000000000000000000000006f00000000000000000000000000000000000000000000000000000000000000de00000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000257000000000000000000000000000000000000000000000000000000000012d68700000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000256000000000000000000000000000000000000000000000000000000000000000200000000000000000000000058dad74c38e9c4738bf3471f6aac6124f862faf500000000000000000000000058dad74c38e9c4738bf3471f6aac6124f862faf5"
		);
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
