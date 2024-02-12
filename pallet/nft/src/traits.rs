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

use frame_support::traits::Get;
use seed_primitives::{
	CollectionUuid, MetadataScheme, OriginChain, RoyaltiesSchedule, SerialNumber, TokenCount,
	TokenId, TokenLockReason,
};
use sp_runtime::{BoundedVec, DispatchError, DispatchResult};
use sp_std::{fmt::Debug, vec::Vec};

use crate::CollectionInformation;

pub trait NFTExt {
	type AccountId: Debug + PartialEq + Clone;
	type MaxTokensPerCollection: Get<u32>;
	type StringLimit: Get<u32>;

	fn do_mint(
		origin: Self::AccountId,
		collection_id: CollectionUuid,
		quantity: TokenCount,
		token_owner: Option<Self::AccountId>,
	) -> DispatchResult;

	fn do_transfer(
		origin: Self::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: Vec<SerialNumber>,
		new_owner: Self::AccountId,
	) -> DispatchResult;

	fn do_create_collection(
		owner: Self::AccountId,
		name: BoundedVec<u8, Self::StringLimit>,
		initial_issuance: TokenCount,
		max_issuance: Option<TokenCount>,
		token_owner: Option<Self::AccountId>,
		metadata_scheme: MetadataScheme,
		royalties_schedule: Option<RoyaltiesSchedule<Self::AccountId>>,
		origin_chain: OriginChain,
	) -> Result<CollectionUuid, DispatchError>;

	fn get_token_owner(token_id: &TokenId) -> Option<Self::AccountId>;

	fn get_collection_info(
		collection_id: CollectionUuid,
	) -> Result<
		CollectionInformation<Self::AccountId, Self::MaxTokensPerCollection, Self::StringLimit>,
		DispatchError,
	>;

	fn enable_xls20_compatibility(
		who: Self::AccountId,
		collection_id: CollectionUuid,
	) -> DispatchResult;

	fn next_collection_uuid() -> Result<CollectionUuid, DispatchError>;

	fn increment_collection_id() -> DispatchResult;

	fn get_token_lock(token_id: TokenId) -> Option<TokenLockReason>;

	fn set_token_lock(
		token_id: TokenId,
		lock_reason: TokenLockReason,
		who: Self::AccountId,
	) -> DispatchResult;

	fn remove_token_lock(token_id: TokenId);
}
