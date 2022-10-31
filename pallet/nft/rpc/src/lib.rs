// Copyright 2020-2021 Parity Technologies (UK) Ltd. and Centrality Investments Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Node-specific RPC methods for interaction with NFT module.

use std::sync::Arc;

use codec::Codec;
use jsonrpsee::{
	core::{Error as RpcError, RpcResult},
	proc_macros::rpc,
};
use pallet_nft::Config;
use seed_primitives::types::{BlockNumber, CollectionUuid, TokenId};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

pub use pallet_nft_rpc_runtime_api::{self as runtime_api, NftApi as NftRuntimeApi};

/// NFT RPC methods.
#[rpc(client, server, namespace = "nft")]
pub trait NftApi<AccountId> {
	#[method(name = "collectedTokens")]
	fn collected_tokens(
		&self,
		collection_id: CollectionUuid,
		who: AccountId,
	) -> RpcResult<Vec<TokenId>>;

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
	T: Config<AccountId = AccountId, BlockNumber = BlockNumber> + Send + Sync,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: NftRuntimeApi<Block, AccountId, T>,
	AccountId: Codec,
{
	fn collected_tokens(
		&self,
		collection_id: CollectionUuid,
		who: AccountId,
	) -> RpcResult<Vec<TokenId>> {
		let api = self.client.runtime_api();
		let best = self.client.info().best_hash;
		let at = BlockId::hash(best);
		api.collected_tokens(&at, collection_id, who)
			.map_err(|e| RpcError::to_call_error(e))
	}

	fn token_uri(&self, token_id: TokenId) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let best = self.client.info().best_hash;
		let at = BlockId::hash(best);
		api.token_uri(&at, token_id).map_err(|e| RpcError::to_call_error(e))
	}
}
