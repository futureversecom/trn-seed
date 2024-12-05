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

//! 'Ethy' is the CENNZnet event proving protocol
//! It is based on the same architecture as substrate's 'BEEFY' protocol.
//!
//! Active validators receive requests to witness events from runtime messages added to blocks.
//! Validators then sign the event and share with peers
//! Once a threshold hold of votes have been assembled a proof is generated, stored in auxiliary db
//! storage and
// shared over RPC to subscribers.
//!
//! The current implementation simply assembles signatures from individual validators.

use std::sync::Arc;

use log::debug;
use substrate_prometheus_endpoint::Registry;

use sc_client_api::{Backend, BlockchainEvents, Finalizer};
use sc_network::ProtocolName;
use sc_network_gossip::{GossipEngine, Network as GossipNetwork, Syncing as GossipSyncing};
use seed_primitives::ethy::EthyApi;
use sp_api::{BlockT, HeaderT, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_consensus::SyncOracle;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Block;

mod error;
mod gossip;
mod keystore;
mod metrics;
#[cfg(test)]
mod testing;
mod types;
mod witness_record;
mod worker;

pub mod notification;
#[cfg(test)]
mod tests;

pub use ethy_protocol_name::standard_name as protocol_standard_name;
pub use keystore::EthyEcdsaToEthereum;
pub use types::data_to_digest;

pub(crate) mod ethy_protocol_name {
	use sc_chain_spec::ChainSpec;
	use sc_network::ProtocolName;

	pub const NAME: &str = "/ethy/1";
	/// Name of the notifications protocol used by Ethy.
	///
	/// Must be registered towards the networking in order for Ethy to properly function.
	pub fn standard_name<Hash: AsRef<[u8]>>(
		genesis_hash: &Hash,
		chain_spec: &Box<dyn ChainSpec>,
	) -> ProtocolName {
		let chain_prefix = match chain_spec.fork_id() {
			Some(fork_id) => format!("/{}/{}", hex::encode(genesis_hash), fork_id),
			None => format!("/{}", hex::encode(genesis_hash)),
		};
		ProtocolName::OnHeap(format!("{}{}", chain_prefix, NAME).into())
	}
}

/// Returns the configuration value to put in
/// [`sc_network::config::NetworkConfiguration::extra_sets`].
pub fn ethy_peers_set_config(
	protocol_name: ProtocolName,
) -> sc_network::config::NonDefaultSetConfig {
	let mut cfg = sc_network::config::NonDefaultSetConfig::new(protocol_name, 1024 * 1024);
	cfg.allow_non_reserved(25, 25);
	cfg
}

/// A convenience ETHY client trait that defines all the type bounds a ETHY client
/// has to satisfy. Ideally that should actually be a trait alias. Unfortunately as
/// of today, Rust does not allow a type alias to be used as a trait bound. Tracking
/// issue is <https://github.com/rust-lang/rust/issues/41517>.
pub trait Client<B, BE>:
	BlockchainEvents<B> + HeaderBackend<B> + Finalizer<B, BE> + ProvideRuntimeApi<B> + Send + Sync
where
	B: Block,
	BE: Backend<B>,
{
	// empty
}

impl<B, BE, T> Client<B, BE> for T
where
	B: Block,
	BE: Backend<B>,
	T: BlockchainEvents<B>
		+ HeaderBackend<B>
		+ Finalizer<B, BE>
		+ ProvideRuntimeApi<B>
		+ Send
		+ Sync,
{
	// empty
}

/// ETHY gadget initialization parameters.
pub struct EthyParams<B, BE, C, R, N, S>
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: EthyApi<B>,
	N: GossipNetwork<B> + Clone + Send + 'static,
	S: GossipSyncing<B> + SyncOracle + Send + Clone + 'static,
{
	/// ETHY client
	pub client: Arc<C>,
	/// Client Backend
	pub backend: Arc<BE>,
	/// Runtime
	pub runtime: Arc<R>,
	/// Local key store
	pub key_store: Option<KeystorePtr>,
	/// Gossip network
	pub network: N,
	/// Gossip network
	pub sync_service: S,
	/// ETHY signed witness sender
	pub event_proof_sender: notification::EthyEventProofSender,
	/// Prometheus metric registry
	pub prometheus_registry: Option<Registry>,
	/// Chain specific Ethy protocol name. See [`ethy_protocol_name::standard_name`].
	pub protocol_name: ProtocolName,
	pub _phantom: std::marker::PhantomData<B>,
}

/// Start the ETHY gadget.
///
/// This is a thin shim around running and awaiting a ETHY worker.
pub async fn start_ethy_gadget<B, BE, C, R, N, S>(ethy_params: EthyParams<B, BE, C, R, N, S>)
where
	B: Block,
	BE: Backend<B> + 'static,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: EthyApi<B>,
	N: GossipNetwork<B> + Clone + Sync + Send + 'static,
	S: GossipSyncing<B> + SyncOracle + Send + Clone + 'static,
	<<B as BlockT>::Header as HeaderT>::Number: Into<u64>,
{
	let EthyParams {
		client,
		backend,
		runtime,
		key_store,
		network,
		sync_service,
		event_proof_sender,
		prometheus_registry,
		protocol_name,
		_phantom: std::marker::PhantomData,
	} = ethy_params;

	let sync_oracle = sync_service.clone();
	let gossip_validator =
		Arc::new(gossip::GossipValidator::new(Default::default(), backend.clone()));
	let gossip_engine =
		GossipEngine::new(network, sync_service, protocol_name, gossip_validator.clone(), None);

	let metrics =
		prometheus_registry.as_ref().map(metrics::Metrics::register).and_then(
			|result| match result {
				Ok(metrics) => {
					debug!(target: "ethy", "💎 Registered metrics");
					Some(metrics)
				},
				Err(err) => {
					debug!(target: "ethy", "💎 Failed to register metrics: {:?}", err);
					None
				},
			},
		);

	let worker_params = worker::WorkerParams {
		client,
		backend,
		runtime,
		key_store: key_store.into(),
		event_proof_sender,
		gossip_engine,
		gossip_validator,
		metrics,
		sync_oracle,
	};

	let worker = worker::EthyWorker::<_, _, _, _, _>::new(worker_params);

	worker.run().await
}
