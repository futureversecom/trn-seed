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

//! RPC API for Ethy.

#![warn(missing_docs)]

use codec::Decode;
use futures::{FutureExt, StreamExt};
use jsonrpsee::{
	core::RpcResult, proc_macros::rpc, types::SubscriptionEmptyError, SubscriptionSink,
};
use log::warn;
use sc_client_api::backend::AuxStore;
use sc_rpc::SubscriptionTaskExecutor;
use sp_api::{BlockId, ProvideRuntimeApi};
use sp_core::{Bytes, H256};
use sp_runtime::traits::{Block, Convert};
use std::{marker::PhantomData, ops::Deref, sync::Arc};

use ethy_gadget::{notification::EthyEventProofStream, EthyEcdsaToEthereum};
use seed_primitives::{
	ethy::{
		EthyApi as EthyRuntimeApi, EthyChainId, EventProof, EventProofId, VersionedEventProof,
		ETHY_ENGINE_ID,
	},
	AccountId20,
};

mod notification;
use notification::{EthEventProofResponse, XrplEventProofResponse};
use seed_primitives::ethy::EthyEcdsaToPublicKey;

/// Provides RPC methods for interacting with Ethy.
#[allow(clippy::needless_return)]
#[rpc(client, server, namespace = "ethy")]
pub trait EthyApi<Notification> {
	/// Returns event proofs generated by Ethy
	#[subscription(name = "subscribeEventProofs" => "eventProofs", unsubscribe = "unsubscribeEventProofs", item = Notification)]
	fn subscribe_event_proofs(&self);

	/// Query a proof for `event_proof_id` and Ethereum chain Id
	///
	/// Returns `null` if missing
	#[method(name = "getEventProof")]
	fn get_event_proof(
		&self,
		event_proof_id: EventProofId,
	) -> RpcResult<Option<EthEventProofResponse>>;

	/// Query a proof for a `event_proof_id` and XRPL chain Id
	///
	/// Returns `null` if missing
	#[method(name = "getXrplTxProof")]
	fn get_xrpl_tx_proof(
		&self,
		event_proof_id: EventProofId,
	) -> RpcResult<Option<XrplEventProofResponse>>;
}

/// Implements the EthyApi RPC trait for interacting with ethy-gadget.
pub struct EthyRpcHandler<C, R, B> {
	event_proof_stream: EthyEventProofStream,
	executor: SubscriptionTaskExecutor,
	/// Handle to a client + backend
	client: Arc<C>,
	runtime: Arc<R>,
	phantom: PhantomData<B>,
}

impl<C, R, B> EthyRpcHandler<C, R, B>
where
	C: AuxStore + Send + Sync + 'static,
	R: ProvideRuntimeApi<B> + Sync + Send + 'static,
	R::Api: EthyRuntimeApi<B>,
	B: Block<Hash = H256>,
{
	/// Creates a new EthyRpcHandler instance.
	pub fn new(
		event_proof_stream: EthyEventProofStream,
		executor: SubscriptionTaskExecutor,
		client: Arc<C>,
		runtime: Arc<R>,
	) -> Self {
		Self { client, event_proof_stream, executor, runtime, phantom: PhantomData }
	}
}

impl<C, R, B> EthyApiServer<EthEventProofResponse> for EthyRpcHandler<C, R, B>
where
	B: Block<Hash = H256>,
	C: AuxStore + Send + Sync + 'static,
	R: ProvideRuntimeApi<B> + Sync + Send + 'static,
	R::Api: EthyRuntimeApi<B>,
{
	fn subscribe_event_proofs(
		&self,
		mut pending: SubscriptionSink,
	) -> Result<(), SubscriptionEmptyError> {
		let runtime_handle = self.runtime.clone();
		let stream = self
			.event_proof_stream
			.subscribe()
			.map(move |p| build_event_proof_response::<R, B>(&runtime_handle, p));

		let fut = async move {
			// asynchronous portion of the function
			pending.pipe_from_stream(stream).await;
		};

		self.executor.spawn("ethy-rpc-subscription", Some("rpc"), fut.boxed());
		Ok(())
	}

	fn get_event_proof(&self, event_id: EventProofId) -> RpcResult<Option<EthEventProofResponse>> {
		if let Ok(maybe_encoded_proof) = self.client.get_aux(
			[
				ETHY_ENGINE_ID.as_slice(),
				&[EthyChainId::Ethereum.into()].as_slice(),
				&event_id.to_be_bytes().as_slice(),
			]
			.concat()
			.as_ref(),
		) {
			if let Some(encoded_proof) = maybe_encoded_proof {
				if let Ok(versioned_proof) = VersionedEventProof::decode(&mut &encoded_proof[..]) {
					let event_proof_response =
						build_event_proof_response::<R, B>(&self.runtime, versioned_proof);
					return Ok(event_proof_response)
				}
			}
		}
		Ok(None)
	}

	fn get_xrpl_tx_proof(
		&self,
		event_id: EventProofId,
	) -> RpcResult<Option<XrplEventProofResponse>> {
		if let Ok(maybe_encoded_proof) = self.client.get_aux(
			[
				ETHY_ENGINE_ID.as_slice(),
				&[EthyChainId::Xrpl.into()].as_slice(),
				&event_id.to_be_bytes().as_slice(),
			]
			.concat()
			.as_ref(),
		) {
			if let Some(encoded_proof) = maybe_encoded_proof {
				if let Ok(versioned_proof) = VersionedEventProof::decode(&mut &encoded_proof[..]) {
					let response =
						build_xrpl_tx_proof_response::<R, B>(&self.runtime, versioned_proof);
					return Ok(response)
				}
			}
		}
		Ok(None)
	}
}

/// Build an `EthEventProofResponse` from a `VersionedEventProof`
pub fn build_event_proof_response<R, B>(
	runtime: &R,
	versioned_event_proof: VersionedEventProof,
) -> Option<EthEventProofResponse>
where
	B: Block<Hash = H256>,
	R: ProvideRuntimeApi<B>,
	R::Api: EthyRuntimeApi<B>,
{
	match versioned_event_proof {
		VersionedEventProof::V1(event_proof) => {
			let proof_validator_set = runtime
				.runtime_api()
				.validator_set(&BlockId::hash(event_proof.block.into()))
				.ok()?;

			let validator_addresses: Vec<AccountId20> = proof_validator_set
				.validators
				.into_iter()
				.map(|v| EthyEcdsaToEthereum::convert(v.as_ref()))
				.map(Into::into)
				.collect();

			Some(EthEventProofResponse {
				event_id: event_proof.event_id,
				signatures: event_proof
					.expanded_signatures(validator_addresses.len())
					.into_iter()
					.map(|s| Bytes::from(s.deref().to_vec()))
					.collect(),
				validators: validator_addresses,
				validator_set_id: proof_validator_set.id,
				block: event_proof.block.into(),
				tag: None,
			})
		},
	}
}

/// Build an `XrplEventProofResponse` from a `VersionedEventProof`
pub fn build_xrpl_tx_proof_response<R, B>(
	runtime: &R,
	versioned_event_proof: VersionedEventProof,
) -> Option<XrplEventProofResponse>
where
	B: Block<Hash = H256>,
	R: ProvideRuntimeApi<B>,
	R::Api: EthyRuntimeApi<B>,
{
	match versioned_event_proof {
		VersionedEventProof::V1(EventProof { signatures, event_id, block, .. }) => {
			let xrpl_validator_set =
				runtime.runtime_api().xrpl_signers(&BlockId::hash(block.into())).ok()?;

			let validator_set =
				runtime.runtime_api().validator_set(&BlockId::hash(block.into())).ok()?;
			let mut xrpl_signer_set: Vec<Bytes> = Default::default();

			Some(XrplEventProofResponse {
				event_id,
				signatures: signatures
					.into_iter()
					.filter(|(i, _)| {
						let pub_key = validator_set.validators.get(*i as usize);
						if let Some(pub_key) = pub_key {
							// we only care about the availability of the pub_key in
							// xrpl_validator_set or not, doesn't matter the position.
							match xrpl_validator_set.authority_index(pub_key) {
								Some(_) => {
									xrpl_signer_set.push(Bytes::from(
										EthyEcdsaToPublicKey::convert(pub_key.clone()).to_vec(),
									));
									true
								},
								None => false,
							}
						} else {
							false
						}
					})
					.map(|(_, s)| {
						// XRPL requires ECDSA signatures are DER encoded
						// https://github.com/XRPLF/xrpl.js/blob/76b73e16a97e1a371261b462ee1a24f1c01dbb0c/packages/ripple-keypairs/src/i.ts#L58-L60
						let sig_ = s.deref();
						// 0..64, ignore byte 64 (v/recoveryId)
						let mut sig_normalized = libsecp256k1::Signature::parse_standard(
							sig_[..64].try_into().expect("64 byte signature"),
						)
						.expect("valid signature");
						// use 'canonical' S value
						// https://xrpl.org/transaction-malleability.html#alternate-secp256k1-signatures
						// https://github.com/indutny/elliptic/blob/43ac7f230069bd1575e1e4a58394a512303ba803/lib/elliptic/ec/index.js#L146-L150
						sig_normalized.normalize_s();
						sig_normalized.serialize_der()
					})
					.map(|s| Bytes::from(s.as_ref().to_vec()))
					.collect(),
				validators: xrpl_signer_set,
				validator_set_id: xrpl_validator_set.id,
				block: block.into(),
				tag: None,
			})
		},
	}
}
