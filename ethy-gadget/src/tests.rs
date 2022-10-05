// This file is part of Substrate.

// Copyright (C) 2018-2022 Parity Technologies (UK) Ltd.
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

//! Tests and test helpers for ETHY.

use crate::{
	ethy_protocol_name, notification::EthyEventProofStream, testing::Keyring as EthyKeyring,
};
use futures::{future, stream::FuturesUnordered, Future, StreamExt};
use parking_lot::Mutex;
use sc_chain_spec::{ChainSpec, GenericChainSpec};
use sc_client_api::HeaderBackend;
use sc_consensus::{
	BlockImportParams, BoxJustificationImport, ForkChoiceStrategy, ImportResult, ImportedAux,
};
use sc_keystore::LocalKeystore;
use sc_network_test::{
	Block, BlockImportAdapter, FullPeerConfig, PassThroughVerifier, Peer, PeersClient,
	TestNetFactory,
};
use sc_utils::notification::NotificationReceiver;
use seed_primitives::ethy::{
	crypto::AuthorityId, ConsensusLog, EthyApi, EthyEcdsaToPublicKey, EventProof, EventProofId,
	ValidatorSet, VersionedEventProof, Witness, ETHY_ENGINE_ID, ETHY_KEY_TYPE,
	GENESIS_AUTHORITY_SET_ID,
};
use serde::{Deserialize, Serialize};
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_consensus::BlockOrigin;
use sp_core::H256;
use sp_keystore::{SyncCryptoStore, SyncCryptoStorePtr};
use sp_runtime::{
	codec::Encode,
	generic::BlockId,
	traits::{Header as HeaderT, NumberFor},
	BuildStorage, DigestItem, Justifications, Storage,
};
use std::{collections::HashMap, sync::Arc, task::Poll};
use substrate_test_runtime_client::{runtime::Header, ClientExt};
use tokio::{runtime::Runtime, time::Duration};

pub(crate) const ETHY_PROTOCOL_NAME: &'static str = "/ethy/1";

pub(crate) type EthyValidatorSet = ValidatorSet<AuthorityId>;
pub(crate) type EthyPeer = Peer<PeerData, PeersClient>;

#[derive(Debug, Serialize, Deserialize)]
struct Genesis(std::collections::BTreeMap<String, String>);
impl BuildStorage for Genesis {
	fn assimilate_storage(&self, storage: &mut Storage) -> Result<(), String> {
		storage
			.top
			.extend(self.0.iter().map(|(a, b)| (a.clone().into_bytes(), b.clone().into_bytes())));
		Ok(())
	}
}

#[derive(Clone)]
pub(crate) struct EthyLinkHalf {
	pub event_proof_stream: EthyEventProofStream,
}

#[derive(Default)]
pub(crate) struct PeerData {
	pub(crate) beefy_link_half: Mutex<Option<EthyLinkHalf>>,
}

#[derive(Default)]
pub(crate) struct EthyTestNet {
	peers: Vec<EthyPeer>,
}

impl EthyTestNet {
	pub(crate) fn new(n_authority: usize, n_full: usize) -> Self {
		let mut net = EthyTestNet { peers: Vec::with_capacity(n_authority + n_full) };
		for _ in 0..n_authority {
			net.add_authority_peer();
		}
		for _ in 0..n_full {
			net.add_full_peer();
		}
		net
	}

	pub(crate) fn add_authority_peer(&mut self) {
		self.add_full_peer_with_config(FullPeerConfig {
			notifications_protocols: vec![ETHY_PROTOCOL_NAME.into()],
			is_authority: true,
			..Default::default()
		})
	}

	pub(crate) fn generate_blocks_and_sync(
		&mut self,
		count: usize,
		session_length: u64,
		validator_set: &EthyValidatorSet,
	) {
		self.peer(0).generate_blocks(count, BlockOrigin::File, |builder| {
			let mut block = builder.build().unwrap().block;

			if *block.header.number() % session_length == 0 {
				add_auth_change_digest(&mut block.header, validator_set.clone());
			}

			block
		});
		self.block_until_sync();
	}
}

impl TestNetFactory for EthyTestNet {
	type Verifier = PassThroughVerifier;
	type BlockImport = PeersClient;
	type PeerData = PeerData;

	fn make_verifier(&self, _client: PeersClient, _: &PeerData) -> Self::Verifier {
		PassThroughVerifier::new(false) // use non-instant finality.
	}

	fn make_block_import(
		&self,
		client: PeersClient,
	) -> (
		BlockImportAdapter<Self::BlockImport>,
		Option<BoxJustificationImport<Block>>,
		Self::PeerData,
	) {
		(client.as_block_import(), None, PeerData::default())
	}

	fn peer(&mut self, i: usize) -> &mut EthyPeer {
		&mut self.peers[i]
	}

	fn peers(&self) -> &Vec<EthyPeer> {
		&self.peers
	}

	fn mut_peers<F: FnOnce(&mut Vec<EthyPeer>)>(&mut self, closure: F) {
		closure(&mut self.peers);
	}

	fn add_full_peer(&mut self) {
		self.add_full_peer_with_config(FullPeerConfig {
			notifications_protocols: vec![ETHY_PROTOCOL_NAME.into()],
			is_authority: false,
			..Default::default()
		})
	}
}

macro_rules! create_test_api {
    ( $api_name:ident, $($inits:expr),+ ) => {
		pub(crate) mod $api_name {
			use super::*;

			#[derive(Clone, Default)]
			pub(crate) struct TestApi {}

			// compiler gets confused and warns us about unused inner
			#[allow(dead_code)]
			pub(crate) struct RuntimeApi {
				inner: TestApi,
			}

			impl ProvideRuntimeApi<Block> for TestApi {
				type Api = RuntimeApi;
				fn runtime_api<'a>(&'a self) -> ApiRef<'a, Self::Api> {
					RuntimeApi { inner: self.clone() }.into()
				}
			}
			sp_api::mock_impl_runtime_apis! {
				impl EthyApi<Block> for RuntimeApi {
					fn validator_set() -> EthyValidatorSet {
						EthyValidatorSet::new(make_ethy_ids(&[$($inits),+]), 0)
					}
				}
			}
		}
	};
}

create_test_api!(two_validators, EthyKeyring::Alice, EthyKeyring::Bob);
create_test_api!(
	four_validators,
	EthyKeyring::Alice,
	EthyKeyring::Bob,
	EthyKeyring::Charlie,
	EthyKeyring::Dave
);
create_test_api!(
	bad_four_validators,
	EthyKeyring::Alice,
	EthyKeyring::Bob,
	EthyKeyring::Charlie,
	EthyKeyring::Dave
);

fn add_auth_change_digest(header: &mut Header, new_auth_set: EthyValidatorSet) {
	header.digest_mut().push(DigestItem::Consensus(
		ETHY_ENGINE_ID,
		ConsensusLog::<AuthorityId>::AuthoritiesChange(new_auth_set).encode(),
	));
}

// pub(crate) fn make_ethy_ids(keys: &[EthyKeyring]) -> Vec<AuthorityId> {
// 	keys.iter().map(|&key| key.public().into()).collect()
// }
pub(crate) fn make_ethy_ids(keys: &[EthyKeyring]) -> Vec<AuthorityId> {
	keys.iter().map(|key| key.clone().public().into()).collect()
}

pub(crate) fn create_beefy_keystore(authority: EthyKeyring) -> SyncCryptoStorePtr {
	let keystore = Arc::new(LocalKeystore::in_memory());
	SyncCryptoStore::ecdsa_generate_new(&*keystore, ETHY_KEY_TYPE, Some(&authority.to_seed()))
		.expect("Creates authority key");
	keystore
}

// // Spawns beefy voters. Returns a future to spawn on the runtime.
// fn initialize_beefy<API>(
// 	net: &mut EthyTestNet,
// 	peers: Vec<(usize, &EthyKeyring, Arc<API>)>,
// ) -> impl Future<Output = ()>
// where
// 	API: ProvideRuntimeApi<Block> + Default + Sync + Send,
// 	API::Api: EthyApi<Block>,
// {
// 	let voters = FuturesUnordered::new();
//
// 	for (peer_id, key, api) in peers.into_iter() {
// 		let peer = &net.peers[peer_id];
//
// 		let keystore = create_beefy_keystore(*key);
//
// 		let (_, _, peer_data) = net.make_block_import(peer.client().clone());
// 		let PeerData { beefy_link_half } = peer_data;
// 		let event_proof_stream = beefy_link_half.into_inner().unwrap().event_proof_stream;
// 		let beefy_voter_links = beefy_voter_links.lock().take();
// 		*peer.data.beefy_rpc_links.lock() = beefy_rpc_links.lock().take();
// 		*peer.data.beefy_voter_links.lock() = beefy_voter_links.clone();
//
// 		let ethy_params = crate::EthyParams {
// 			client: peer.client().as_client(),
// 			backend: peer.client().as_backend(),
// 			key_store: Some(keystore),
// 			network: peer.network_service().clone(),
// 			prometheus_registry: None,
// 			protocol_name: ETHY_PROTOCOL_NAME.into(),
// 			event_proof_sender: (),
// 			_phantom: Default::default()
// 		};
// 		let gadget = crate::start_ethy_gadget::<_, _, _, _>(ethy_params);
//
// 		fn assert_send<T: Send>(_: &T) {}
// 		assert_send(&gadget);
// 		voters.push(gadget);
// 	}
//
// 	voters.for_each(|_| async move {})
// }

fn block_until(future: impl Future + Unpin, net: &Arc<Mutex<EthyTestNet>>, runtime: &mut Runtime) {
	let drive_to_completion = futures::future::poll_fn(|cx| {
		net.lock().poll(cx);
		Poll::<()>::Pending
	});
	runtime.block_on(future::select(future, drive_to_completion));
}

fn run_for(duration: Duration, net: &Arc<Mutex<EthyTestNet>>, runtime: &mut Runtime) {
	let sleep = runtime.spawn(async move { tokio::time::sleep(duration).await });
	block_until(sleep, net, runtime);
}

// pub(crate) fn get_beefy_streams(
// 	net: &mut EthyTestNet,
// 	peers: &[BeefyKeyring],
// ) -> (Vec<NotificationReceiver<H256>>, Vec<NotificationReceiver<BeefySignedCommitment<Block>>>) {
// 	let mut best_block_streams = Vec::new();
// 	let mut signed_commitment_streams = Vec::new();
// 	for peer_id in 0..peers.len() {
// 		let beefy_link_half =
// 			net.peer(peer_id).data.beefy_link_half.lock().as_ref().unwrap().clone();
// 		let EthyLinkHalf { signed_commitment_stream, beefy_best_block_stream } = beefy_link_half;
// 		best_block_streams.push(beefy_best_block_stream.subscribe());
// 		signed_commitment_streams.push(signed_commitment_stream.subscribe());
// 	}
// 	(best_block_streams, signed_commitment_streams)
// }
