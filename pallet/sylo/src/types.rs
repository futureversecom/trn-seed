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

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{traits::Get, BoundedVec, CloneNoBound, PartialEqNoBound, RuntimeDebug};
use scale_info::TypeInfo;
use seed_primitives::{AssetId, Balance};
use sp_core::{H160, H256};
use sp_std::default::Default;

pub type ResolverId<StringLimit> = BoundedVec<u8, StringLimit>;

#[derive(Clone, Encode, Decode, RuntimeDebug, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub struct ValidationRecord<AccountId, MaxResolvers, MaxTags, StringLimit>
where
	MaxResolvers: Get<u32>,
	MaxTags: Get<u32>,
	StringLimit: Get<u32>,
{
	pub author: AccountId,
	pub resolvers: BoundedVec<ResolverId<StringLimit>, MaxResolvers>,
	pub data_type: BoundedVec<u8, StringLimit>,
	pub algorithm: BoundedVec<u8, StringLimit>,
	pub tags: BoundedVec<BoundedVec<u8, StringLimit>, MaxTags>,
}
