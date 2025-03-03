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
use seed_primitives::CollectionUuid;
use sp_runtime::DispatchError;
use sp_std::fmt::Debug;

use crate::{CollectionInformation};

pub trait NFTCollectionInfo {
	type AccountId: Debug + PartialEq + Clone;
	type StringLimit: Get<u32>;

	fn get_collection_info(
		collection_id: CollectionUuid,
	) -> Result<CollectionInformation<Self::AccountId, Self::StringLimit>, DispatchError>;
}
