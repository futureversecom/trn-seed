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

//! Node-specific RPC methods for interaction with SyloDataPermissions module.
extern crate alloc;

use std::sync::Arc;

use alloc::{string::String, vec::Vec};
use codec::Codec;
use jsonrpsee::{
	core::{async_trait, Error as RpcError, RpcResult},
	proc_macros::rpc,
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::DispatchError;

use pallet_sylo_data_permissions::GetPermissionsResult;
pub use pallet_sylo_data_permissions_rpc_runtime_api::{
	self as runtime_api, SyloDataPermissionsApi as SyloDataPermissionsRuntimeApi,
};

/// SyloDataPermissions RPC methods.
#[rpc(client, server, namespace = "syloDataPermissions")]
pub trait SyloDataPermissionsApi<AccountId> {
	#[method(name = "get_permissions")]
	fn get_permissions(
		&self,
		data_author: AccountId,
		grantee: AccountId,
		data_ids: Vec<String>,
	) -> RpcResult<Result<GetPermissionsResult, DispatchError>>;
}

/// An implementation of SyloDataPermissions specific RPC methods.
pub struct SyloDataPermissions<C, Block> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<Block>,
}

impl<C, Block> SyloDataPermissions<C, Block> {
	/// Create new `SyloDataPermissions` with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		SyloDataPermissions { client, _marker: Default::default() }
	}
}

#[async_trait]
impl<C, Block, AccountId> SyloDataPermissionsApiServer<AccountId> for SyloDataPermissions<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: SyloDataPermissionsRuntimeApi<Block, AccountId>,
	AccountId: Codec,
{
	fn get_permissions(
		&self,
		data_author: AccountId,
		grantee: AccountId,
		data_ids: Vec<String>,
	) -> RpcResult<Result<GetPermissionsResult, DispatchError>> {
		let api = self.client.runtime_api();
		let best = self.client.info().best_hash;
		api.get_permissions(best, data_author, grantee, data_ids)
			.map_err(RpcError::to_call_error)
	}
}
