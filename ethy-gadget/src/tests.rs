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

//! Tests and test helpers for ETHY.

use crate::{notification::EthyEventProofStream, testing::Keyring as EthyKeyring};
use parking_lot::Mutex;
use sc_consensus::BoxJustificationImport;
use sc_keystore::LocalKeystore;
use sc_network_test::{
	Block, BlockImportAdapter, FullPeerConfig, PassThroughVerifier, Peer, PeersClient,
	TestNetFactory,
};
use seed_primitives::ethy::{crypto::AuthorityId, EthyApi, ValidatorSet, ETHY_KEY_TYPE};
use serde::{Deserialize, Serialize};
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::{BuildStorage, Storage};
use std::sync::Arc;

pub(crate) const ETHY_PROTOCOL_NAME: &str = "/ethy/1";

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
	pub _event_proof_stream: EthyEventProofStream,
}

#[derive(Default)]
pub(crate) struct PeerData {
	pub(crate) _beefy_link_half: Mutex<Option<EthyLinkHalf>>,
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

	/* 	pub(crate) fn generate_blocks_and_sync(
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
	} */
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

	fn peers_mut(&mut self) -> &mut Vec<EthyPeer> {
		&mut self.peers
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
						let validators = make_ethy_ids(&[$($inits),+]);
						EthyValidatorSet::new(make_ethy_ids(&[$($inits),+]), 0, validators.len() as u32)
					}
					fn xrpl_signers() -> EthyValidatorSet {
						let validators = make_ethy_ids(&[$($inits),+]);
						EthyValidatorSet::new(make_ethy_ids(&[$($inits),+]), 0, validators.len() as u32)
					}
				}
			}
		}
	};
}

create_test_api!(two_validators, EthyKeyring::Alice, EthyKeyring::Bob);

/* fn add_auth_change_digest(header: &mut Header, new_auth_set: EthyValidatorSet) {
	header.digest_mut().push(DigestItem::Consensus(
		ETHY_ENGINE_ID,
		ConsensusLog::<AuthorityId>::AuthoritiesChange(new_auth_set).encode(),
	));
} */

pub(crate) fn make_ethy_ids(keys: &[EthyKeyring]) -> Vec<AuthorityId> {
	keys.iter().map(|key| key.public()).collect()
}

pub(crate) fn create_ethy_keystore(authority: EthyKeyring) -> KeystorePtr {
	let keystore = Arc::new(LocalKeystore::in_memory());
	Keystore::ecdsa_generate_new(&*keystore, ETHY_KEY_TYPE, Some(&authority.to_seed()))
		.expect("Creates authority key");
	keystore
}
