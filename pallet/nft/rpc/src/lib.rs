// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

//! Node-specific RPC methods for interaction with NFT module.

use std::sync::Arc;

use codec::Codec;
use jsonrpsee::{
	core::{Error as RpcError, RpcResult},
	proc_macros::rpc,
};
use pallet_nft::Config;
use seed_primitives::types::{BlockNumber, CollectionUuid, SerialNumber, TokenCount, TokenId};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

pub use pallet_nft_rpc_runtime_api::{self as runtime_api, NftApi as NftRuntimeApi};

/// NFT RPC methods.
#[rpc(client, server, namespace = "nft")]
pub trait NftApi<AccountId> {
	#[method(name = "ownedTokens")]
	fn owned_tokens(
		&self,
		collection_id: CollectionUuid,
		who: AccountId,
		cursor: SerialNumber,
		limit: u16,
	) -> RpcResult<(SerialNumber, TokenCount, Vec<SerialNumber>)>;

	#[method(name = "tokenUri")]
	fn token_uri(&self, token_id: TokenId) -> RpcResult<Vec<u8>>;
}

/// An implementation of NFT specific RPC methods.
pub struct Nft<C, Block, T: Config> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<(Block, T)>,
}

impl<C, Block, T: Config> Nft<C, Block, T> {
	/// Create new `Nft` with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Nft { client, _marker: Default::default() }
	}
}

impl<C, Block, AccountId, T> NftApiServer<AccountId> for Nft<C, Block, T>
where
	Block: BlockT,
	T: Config<BlockNumber = BlockNumber> + Send + Sync,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: NftRuntimeApi<Block, AccountId, T>,
	AccountId: Codec,
{
	fn owned_tokens(
		&self,
		collection_id: CollectionUuid,
		who: AccountId,
		cursor: SerialNumber,
		limit: u16,
	) -> RpcResult<(SerialNumber, TokenCount, Vec<SerialNumber>)> {
		let api = self.client.runtime_api();
		let best = self.client.info().best_hash;
		let at = BlockId::hash(best);
		api.owned_tokens(&at, collection_id, who, cursor, limit)
			.map_err(|e| RpcError::to_call_error(e))
	}

	fn token_uri(&self, token_id: TokenId) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let best = self.client.info().best_hash;
		let at = BlockId::hash(best);
		api.token_uri(&at, token_id).map_err(|e| RpcError::to_call_error(e))
	}
}
