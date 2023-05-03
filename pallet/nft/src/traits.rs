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

use frame_support::traits::Get;
use seed_primitives::{
	CollectionUuid, MetadataScheme, OriginChain, RoyaltiesSchedule, TokenCount, TokenId,
};
use sp_runtime::{BoundedVec, DispatchError, DispatchResult};
use sp_std::fmt::Debug;

use crate::{CollectionInformation, Config};

pub trait NFTExt {
	type AccountId: Debug + PartialEq + Clone;
	type MaxTokensPerCollection: Get<u32>;

	fn do_mint(
		origin: Self::AccountId,
		collection_id: CollectionUuid,
		quantity: TokenCount,
		token_owner: Option<Self::AccountId>,
	) -> DispatchResult;

	fn do_create_collection(
		owner: Self::AccountId,
		name: BoundedVec<u8, <Self::T as Config>::StringLimit>,
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
	) -> Result<CollectionInformation<Self::AccountId, Self::MaxTokensPerCollection>, DispatchError>;

	fn enable_xls20_compatibility(
		who: Self::AccountId,
		collection_id: CollectionUuid,
	) -> DispatchResult;

	fn next_collection_uuid() -> Result<CollectionUuid, DispatchError>;

	fn increment_collection_id() -> DispatchResult;
}
