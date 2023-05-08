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

use jsonrpsee::{
	core::{Error as RpcError, RpcResult},
	proc_macros::rpc,
};
use pallet_dex::{types::WrappedBalance, Config, TradingPairStatus};
pub use pallet_dex_rpc_runtime_api::{self as runtime_api, DexApi as DexRuntimeApi};
use seed_primitives::types::{AssetId, Balance, BlockNumber};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT, DispatchError};

/// Dex RPC methods.
#[rpc(client, server, namespace = "dex")]
pub trait DexApi {
	#[method(name = "quote")]
	fn quote(
		&self,
		amount_a: u128,
		reserve_a: u128,
		reserve_b: u128,
	) -> RpcResult<Result<u128, DispatchError>>;

	#[method(name = "getAmountsOut")]
	fn get_amounts_out(
		&self,
		amount_in: WrappedBalance,
		path: Vec<AssetId>,
	) -> RpcResult<Result<Vec<Balance>, DispatchError>>;

	#[method(name = "getAmountsIn")]
	fn get_amounts_in(
		&self,
		amount_out: WrappedBalance,
		path: Vec<AssetId>,
	) -> RpcResult<Result<Vec<Balance>, DispatchError>>;

	#[method(name = "getLPTokenID")]
	fn get_lp_token_id(
		&self,
		asset_id_a: AssetId,
		asset_id_b: AssetId,
	) -> RpcResult<Result<AssetId, DispatchError>>;

	#[method(name = "getLiquidity")]
	fn get_liquidity(
		&self,
		asset_id_a: AssetId,
		asset_id_b: AssetId,
	) -> RpcResult<(Balance, Balance)>;

	#[method(name = "getTradingPairStatus")]
	fn get_trading_pair_status(
		&self,
		asset_id_a: AssetId,
		asset_id_b: AssetId,
	) -> RpcResult<TradingPairStatus>;
}

/// An implementation of Dex specific RPC methods.
pub struct Dex<C, Block, T: Config>
where
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
{
	client: Arc<C>,
	_marker: std::marker::PhantomData<(Block, T)>,
}

impl<C, Block, T: Config> Dex<C, Block, T>
where
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
{
	/// Create new `Dex` with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Dex { client, _marker: Default::default() }
	}
}

impl<C, Block, T> DexApiServer for Dex<C, Block, T>
where
	Block: BlockT,
	T: Config<BlockNumber = BlockNumber> + Send + Sync,
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: DexRuntimeApi<Block, T>,
{
	fn quote(
		&self,
		amount_a: u128,
		reserve_a: u128,
		reserve_b: u128,
	) -> RpcResult<Result<u128, DispatchError>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		api.quote(&at, amount_a, reserve_a, reserve_b)
			.map_err(|e| RpcError::to_call_error(e))
	}

	fn get_amounts_out(
		&self,
		amount_in: WrappedBalance,
		path: Vec<AssetId>,
	) -> RpcResult<Result<Vec<Balance>, DispatchError>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		api.get_amounts_out(&at, amount_in.0.into(), path)
			.map_err(|e| RpcError::to_call_error(e))
	}

	fn get_amounts_in(
		&self,
		amount_out: WrappedBalance,
		path: Vec<AssetId>,
	) -> RpcResult<Result<Vec<Balance>, DispatchError>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		api.get_amounts_in(&at, amount_out.0.into(), path)
			.map_err(|e| RpcError::to_call_error(e))
	}

	fn get_lp_token_id(
		&self,
		asset_id_a: AssetId,
		asset_id_b: AssetId,
	) -> RpcResult<Result<AssetId, DispatchError>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		api.get_lp_token_id(&at, asset_id_a, asset_id_b)
			.map_err(|e| RpcError::to_call_error(e))
	}

	fn get_liquidity(
		&self,
		asset_id_a: AssetId,
		asset_id_b: AssetId,
	) -> RpcResult<(Balance, Balance)> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		api.get_liquidity(&at, asset_id_a, asset_id_b)
			.map_err(|e| RpcError::to_call_error(e))
	}

	fn get_trading_pair_status(
		&self,
		asset_id_a: AssetId,
		asset_id_b: AssetId,
	) -> RpcResult<TradingPairStatus> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);
		api.get_trading_pair_status(&at, asset_id_a, asset_id_b)
			.map_err(|e| RpcError::to_call_error(e))
	}
}

#[test]
fn wrapped_balance_can_deserialize_integer_or_hex() {
	let info = WrappedBalance(u64::MAX.into());
	let json_str = r#"{"value":18446744073709551615}"#;

	assert_eq!(serde_json::to_string(&info).unwrap(), String::from(json_str));
	assert_eq!(serde_json::from_str::<WrappedBalance>("18446744073709551615").unwrap(), info);

	let info = WrappedBalance { 0: u128::MAX };
	let json_str = r#"{"value":340282366920938463463374607431768211455}"#;

	assert_eq!(serde_json::to_string(&info).unwrap(), String::from(json_str));
	assert_eq!(
		serde_json::from_str::<WrappedBalance>(r#""0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF""#).unwrap(),
		info
	);
}
