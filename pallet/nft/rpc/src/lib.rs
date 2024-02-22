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

//! Node-specific RPC methods for interaction with NFT module.

use std::fmt::Debug;
use std::sync::Arc;

use codec::Codec;
use jsonrpsee::{
	core::{async_trait, Error as RpcError, RpcResult},
	proc_macros::rpc,
};
use pallet_nft::{CollectionInfo, CollectionInformation, Config};
use seed_primitives::types::{CollectionUuid, SerialNumber, TokenCount, TokenId};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use sp_core::{bounded::BoundedVec, ConstU32, Get};
// use pallet_nft::mock::StringLimit;
pub const CollectionNameStringLimit: u32 = 50;
pub use pallet_nft_rpc_runtime_api::{self as runtime_api, NftApi as NftRuntimeApi};
use seed_primitives::{MaxTokensPerCollection, StringLimit};

/// NFT RPC methods.
#[rpc(client, server, namespace = "nft")]
pub trait NftApi<AccountId> where
	AccountId: Debug + PartialEq + Clone
{
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

	#[method(name = "collectionInfo")]
	fn collection_info(&self, collection_id: CollectionUuid) -> RpcResult<CollectionInformation<AccountId, MaxTokensPerCollection, StringLimit>>;
	// fn collection_info(&self, collection_id: CollectionUuid) -> RpcResult<(AccountId,
	// BoundedVec<u8, ConstU32<CollectionNameStringLimit>>, Vec<u8>, Permill, Option<TokenCount>,
	// SerialNumber, TokenCount, bool)>;
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

#[async_trait]
impl<C, Block, AccountId, T> NftApiServer<AccountId> for Nft<C, Block, T>
where
	Block: BlockT,
	T: Config + Send + Sync,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: NftRuntimeApi<Block, AccountId, T>,
	AccountId: Codec + std::clone::Clone + std::cmp::PartialEq + std::fmt::Debug,
{
	fn owned_tokens(
		&self,
		collection_id: CollectionUuid,
		who: AccountId,
		cursor: SerialNumber,
		limit: u16,
	) -> RpcResult<(SerialNumber, TokenCount, Vec<SerialNumber>)> {
		let api = self.client.runtime_api();
		api.owned_tokens(self.client.info().best_hash, collection_id, who, cursor, limit)
			.map_err(|e| RpcError::to_call_error(e))
	}

	fn token_uri(&self, token_id: TokenId) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		api.token_uri(self.client.info().best_hash, token_id)
			.map_err(|e| RpcError::to_call_error(e))
	}

	fn collection_info(&self, collection_id: CollectionUuid) -> RpcResult<CollectionInformation<T::AccountId, T::MaxTokensPerCollection, T::StringLimit>> {
		// RpcResult<(AccountId, BoundedVec<u8, T::StringLimit>, Vec<u8>, Permill,
		// Option<TokenCount>, SerialNumber, TokenCount, bool)> {
		let api = self.client.runtime_api();
		let best = self.client.info().best_hash;
		api.collection_info(best, collection_id).map_err(|e| RpcError::to_call_error(e))
	}
}
