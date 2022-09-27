// Copyright (C) 2020-2021 Parity Technologies (UK) Ltd. & Centrality Investments Ltd
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

//! RPC API for Ethy.

#![warn(missing_docs)]

use codec::Decode;
use futures::{FutureExt, StreamExt};
use jsonrpsee::{core::RpcResult, proc_macros::rpc, PendingSubscription};
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
use notification::{EventProofResponse, XrplTxProofResponse};

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
	fn get_event_proof(&self, event_proof_id: EventProofId) -> RpcResult<Option<Notification>>;

	/// Query a proof for a `event_proof_id` and XRPL chain Id
	///
	/// Returns `null` if missing
	#[method(name = "getXrplTxProof")]
	fn get_xrpl_tx_proof(
		&self,
		event_proof_id: EventProofId,
	) -> RpcResult<Option<XrplTxProofResponse>>;
}

/// Implements the EthyApi RPC trait for interacting with ethy-gadget.
pub struct EthyRpcHandler<C, B> {
	event_proof_stream: EthyEventProofStream,
	executor: SubscriptionTaskExecutor,
	/// Handle to a client + backend
	client: Arc<C>,
	phantom: PhantomData<B>,
}

impl<C, B> EthyRpcHandler<C, B>
where
	B: Block<Hash = H256>,
	C: ProvideRuntimeApi<B> + AuxStore + Send + Sync + 'static,
	C::Api: EthyRuntimeApi<B>,
{
	/// Creates a new EthyRpcHandler instance.
	pub fn new(
		event_proof_stream: EthyEventProofStream,
		executor: SubscriptionTaskExecutor,
		client: Arc<C>,
	) -> Self {
		Self { client, event_proof_stream, executor, phantom: PhantomData }
	}
}

impl<C, B> EthyApiServer<EventProofResponse> for EthyRpcHandler<C, B>
where
	B: Block<Hash = H256>,
	C: ProvideRuntimeApi<B> + AuxStore + Send + Sync + 'static,
	C::Api: EthyRuntimeApi<B>,
{
	fn subscribe_event_proofs(&self, pending: PendingSubscription) {
		let client_handle = self.client.clone();
		let stream = self
			.event_proof_stream
			.subscribe()
			.map(move |p| build_event_proof_response::<C, B>(&client_handle, p));

		let fut = async move {
			// asynchronous portion of the function
			if let Some(mut sink) = pending.accept() {
				sink.pipe_from_stream(stream).await;
			}
		};

		self.executor.spawn("ethy-rpc-subscription", Some("rpc"), fut.boxed());
	}

	fn get_event_proof(&self, event_id: EventProofId) -> RpcResult<Option<EventProofResponse>> {
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
						build_event_proof_response::<C, B>(&self.client, versioned_proof);
					return Ok(event_proof_response)
				}
			}
		}
		Ok(None)
	}

	fn get_xrpl_tx_proof(&self, event_id: EventProofId) -> RpcResult<Option<XrplTxProofResponse>> {
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
					let response = build_xrpl_tx_proof_response(versioned_proof);
					return Ok(response)
				}
			}
		}
		Ok(None)
	}
}

/// Build an `EventProofResponse` from a `VersionedEventProof`
pub fn build_event_proof_response<C, B>(
	client: &C,
	versioned_event_proof: VersionedEventProof,
) -> Option<EventProofResponse>
where
	B: Block<Hash = H256>,
	C: ProvideRuntimeApi<B> + Send + Sync + 'static,
	C::Api: EthyRuntimeApi<B>,
{
	match versioned_event_proof {
		VersionedEventProof::V1(event_proof) => {
			let proof_validator_set = client
				.runtime_api()
				.validator_set(&BlockId::hash(event_proof.block.into()))
				.ok()?;

			let validator_addresses: Vec<AccountId20> = proof_validator_set
				.validators
				.into_iter()
				.map(|v| EthyEcdsaToEthereum::convert(v.as_ref()))
				.map(Into::into)
				.collect();

			Some(EventProofResponse {
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

/// Build an `XrplTxProofResponse` from a `VersionedEventProof`
pub fn build_xrpl_tx_proof_response(
	versioned_event_proof: VersionedEventProof,
) -> Option<XrplTxProofResponse> {
	match versioned_event_proof {
		VersionedEventProof::V1(EventProof { signatures, event_id, block, .. }) =>
			Some(XrplTxProofResponse {
				event_id,
				signatures: signatures
					.into_iter()
					.map(|(i, s)| {
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
						(i, sig_normalized.serialize_der())
					})
					.map(|(i, s)| (i, Bytes::from(s.as_ref().to_vec())))
					.collect(),
				block: block.into(),
			}),
	}
}
