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

use std::sync::Arc;

use codec::Codec;
use jsonrpsee::{
	core::{Error as RpcError, RpcResult},
	proc_macros::rpc,
};
use pallet_assets_ext::Config;
use seed_primitives::types::BlockNumber;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

pub use pallet_assets_ext_rpc_runtime_api::{
	self as runtime_api, AssetsExtApi as AssetsExtRuntimeApi,
};
use seed_primitives::{AssetId, Balance};

/// AssetsExt RPC methods.
#[rpc(client, server, namespace = "assets-ext")]
pub trait AssetsExtApi<AccountId> {
	#[method(name = "assetBalance")]
	fn asset_balance(&self, asset_id: AssetId, who: AccountId) -> RpcResult<Balance>;
}

/// An implementation of AssetsExt specific RPC methods.
pub struct AssetsExt<C, Block, T: Config> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<(Block, T)>,
}

impl<C, Block, T: Config> AssetsExt<C, Block, T> {
	/// Create new `AssetsExt` with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		AssetsExt { client, _marker: Default::default() }
	}
}

impl<C, Block, AccountId, T> AssetsExtApiServer<AccountId> for AssetsExt<C, Block, T>
where
	Block: BlockT,
	T: Config<BlockNumber = BlockNumber> + Send + Sync,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: AssetsExtRuntimeApi<Block, AccountId, T>,
	AccountId: Codec,
{
	fn asset_balance(&self, asset_id: AssetId, who: AccountId) -> RpcResult<Balance> {
		let api = self.client.runtime_api();
		let best = self.client.info().best_hash;
		let at = BlockId::hash(best);
		api.asset_balance(&at, asset_id, who).map_err(|e| RpcError::to_call_error(e))
	}
}
