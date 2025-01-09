// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

//! Node-specific RPC methods for interaction with SFT module.

use std::sync::Arc;

use jsonrpsee::{
	core::{async_trait, Error as RpcError, RpcResult},
	proc_macros::rpc,
};
use pallet_sft::Config;
use seed_primitives::types::TokenId;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;

pub use pallet_sft_rpc_runtime_api::{self as runtime_api, SftApi as SftRuntimeApi};

/// SFT RPC methods.
#[rpc(client, server, namespace = "sft")]
pub trait SftApi {
	#[method(name = "tokenUri")]
	fn token_uri(&self, token_id: TokenId) -> RpcResult<Vec<u8>>;
}

/// An implementation of SFT specific RPC methods.
pub struct Sft<C, Block, T: Config> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<(Block, T)>,
}

impl<C, Block, T: Config> Sft<C, Block, T> {
	/// Create new `Sft` with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Sft { client, _marker: Default::default() }
	}
}

#[async_trait]
impl<C, Block, T> SftApiServer for Sft<C, Block, T>
where
	Block: BlockT,
	T: Config + Send + Sync,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: SftRuntimeApi<Block, T>,
{
	fn token_uri(&self, token_id: TokenId) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		api.token_uri(self.client.info().best_hash, token_id)
			.map_err(RpcError::to_call_error)
	}
}
