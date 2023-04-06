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
use hex;
use std::sync::Arc;

use jsonrpsee::{
	core::{Error as RpcError, RpcResult},
	proc_macros::rpc,
};
use pallet_dex::Config;
use seed_primitives::types::{AssetId, Balance, BlockNumber};
use serde::{Deserialize, Deserializer, Serialize};
use sp_api::ProvideRuntimeApi;
use sp_arithmetic::traits::SaturatedConversion;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT, DispatchError};

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

#[derive(Debug, PartialEq)]
// A balance type for receiving over RPC
pub struct WrappedBalance(u128);
#[derive(Debug, Default, Serialize, Deserialize)]
/// Private, used to help serde handle `WrappedBalance`
/// https://github.com/serde-rs/serde/issues/751#issuecomment-277580700
struct WrappedBalanceHelper {
	value: u128,
}
impl Serialize for WrappedBalance {
	fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		WrappedBalanceHelper { value: self.0 }.serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for WrappedBalance {
	fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer
			.deserialize_any(WrappedBalanceVisitor)
			.map_err(|_| serde::de::Error::custom("deserialize failed"))
	}
}

/// Implements custom serde visitor for decoding balance inputs as integer or hex
struct WrappedBalanceVisitor;

impl<'de> serde::de::Visitor<'de> for WrappedBalanceVisitor {
	type Value = WrappedBalance;
	fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(formatter, "an integer or hex-string")
	}

	fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		Ok(WrappedBalance(v.saturated_into()))
	}

	fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		//remove the first two chars as we are expecting a string prefixed with '0x'
		let decoded_string = hex::decode(&s[2..])
			.map_err(|_| serde::de::Error::custom("expected hex encoded string"))?;
		let fixed_16_bytes: [u8; 16] = decoded_string
			.try_into()
			.map_err(|_| serde::de::Error::custom("parse big int as u128 failed"))?;
		Ok(WrappedBalance(u128::from_be_bytes(fixed_16_bytes)))
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
}
