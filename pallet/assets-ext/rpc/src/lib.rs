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

//! Node-specific RPC methods for interaction with AssetExt module.
extern crate alloc;

use alloc::string::String;
use std::sync::Arc;

use codec::Codec;
use jsonrpsee::{
	core::{async_trait, Error as RpcError, RpcResult},
	proc_macros::rpc,
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;

pub use pallet_assets_ext_rpc_runtime_api::{
	self as runtime_api, AssetsExtApi as AssetsExtRuntimeApi,
};
use seed_primitives::AssetId;

/// AssetsExt RPC methods.
#[rpc(client, server, namespace = "assetsExt")]
pub trait AssetsExtApi<AccountId> {
	#[method(name = "freeBalance")]
	fn free_balance(&self, asset_id: AssetId, who: AccountId) -> RpcResult<String>;
}

/// An implementation of AssetsExt specific RPC methods.
pub struct AssetsExt<C, Block> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<Block>,
}

impl<C, Block> AssetsExt<C, Block> {
	/// Create new `AssetsExt` with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		AssetsExt { client, _marker: Default::default() }
	}
}

#[async_trait]
impl<C, Block, AccountId> AssetsExtApiServer<AccountId> for AssetsExt<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: AssetsExtRuntimeApi<Block, AccountId>,
	AccountId: Codec,
{
	fn free_balance(&self, asset_id: AssetId, who: AccountId) -> RpcResult<String> {
		let api = self.client.runtime_api();
		let best = self.client.info().best_hash;
		api.free_balance(best, asset_id, who, false).map_err(RpcError::to_call_error)
	}
}
