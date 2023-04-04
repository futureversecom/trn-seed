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

use jsonrpsee::{
	core::{Error as RpcError, RpcResult},
	proc_macros::rpc,
};
use pallet_dex::Config;
use seed_primitives::types::{AssetId, Balance, BlockNumber};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

pub use pallet_dex_rpc_runtime_api::{self as runtime_api, DexApi as DexRuntimeApi};

/// Dex RPC methods.
#[rpc(client, server, namespace = "dex")]
pub trait DexApi {
	#[method(name = "quote")]
	fn quote(
		&self,
		amount_a: u128,
		reserve_a: u128,
		reserve_b: u128,
	) -> RpcResult<u128>;

	#[method(name = "getAmountsOut")]
	fn get_amounts_out(
		&self,
		amount_in: Balance,
		path: Vec<AssetId>,
	) -> RpcResult<Vec<Balance>>;

	#[method(name = "getAmountsIn")]
	fn get_amounts_in(
		&self,
		amount_out: Balance,
		path: Vec<AssetId>,
	) -> RpcResult<Vec<Balance>>;
}

/// An implementation of Dex specific RPC methods.
pub struct Dex<C, Block, T: Config> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<(Block, T)>,
}

impl<C, Block, T: Config> Dex<C, Block, T> {
	/// Create new `Dex` with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Dex { client, _marker: Default::default() }
	}
}

impl<C, Block, T> DexApiServer for Dex<C, Block, T>
where
	Block: BlockT,
	T: Config<BlockNumber = BlockNumber> + Send + Sync,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: DexRuntimeApi<Block, T>,
{
	fn quote(
		&self,
		amount_a: u128,
		reserve_a: u128,
		reserve_b: u128,
	) -> RpcResult<u128> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		api.quote(&at, amount_a, reserve_a, reserve_b)
			.map_err(|e| RpcError::to_call_error(e))
	}

	fn get_amounts_out(
		&self,
		amount_in: Balance,
		path: Vec<AssetId>,
	) -> RpcResult<Vec<Balance>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		api.get_amounts_out(&at, amount_in, path)
			.map_err(|e| RpcError::to_call_error(e))
	}

	fn get_amounts_in(
		&self,
		amount_out: Balance,
		path: Vec<AssetId>,
	) -> RpcResult<Vec<Balance>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		api.get_amounts_in(&at, amount_out, path)
			.map_err(|e| RpcError::to_call_error(e))
	}
}
