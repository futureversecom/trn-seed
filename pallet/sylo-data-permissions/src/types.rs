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

use super::*;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{traits::Get, BoundedVec, CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_std::{fmt::Debug, prelude::*};

#[derive(
	Clone, Encode, Decode, RuntimeDebugNoBound, PartialEqNoBound, Eq, TypeInfo, MaxEncodedLen,
)]
pub struct PermissionRecord<AccountId, BlockNumber>
where
	AccountId: Debug + PartialEq + Clone,
	BlockNumber: Debug + PartialEq + Clone,
{
	pub grantor: AccountId,
	pub permission: DataPermission,
	pub block: BlockNumber,
	pub expiry: Option<BlockNumber>,
	pub irrevocable: bool,
}

#[derive(
	CloneNoBound, Encode, Decode, RuntimeDebugNoBound, PartialEqNoBound, Eq, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxTags, StringLimit))]
pub struct TaggedPermissionRecord<BlockNumber, MaxTags, StringLimit>
where
	BlockNumber: Debug + PartialEq + Clone,
	MaxTags: Get<u32>,
	StringLimit: Get<u32>,
{
	pub permission: DataPermission,
	pub tags: BoundedVec<BoundedVec<u8, StringLimit>, MaxTags>,
	pub block: BlockNumber,
	pub expiry: Option<BlockNumber>,
	pub irrevocable: bool,
}

#[derive(
	CloneNoBound, Encode, Decode, RuntimeDebugNoBound, PartialEqNoBound, Eq, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(StringLimit))]
pub struct PermissionReference<StringLimit>
where
	StringLimit: Get<u32>,
{
	pub permission_record_id: BoundedVec<u8, StringLimit>,
}

#[derive(
	Clone,
	Encode,
	Decode,
	Serialize,
	Deserialize,
	RuntimeDebugNoBound,
	PartialEqNoBound,
	Eq,
	TypeInfo,
)]
pub struct PermissionReferenceRecord {
	pub permission_record_id: String,
	pub resolvers: Vec<(String, Vec<String>)>,
}

#[derive(
	Clone,
	Encode,
	Decode,
	Serialize,
	Deserialize,
	RuntimeDebugNoBound,
	PartialEqNoBound,
	Eq,
	TypeInfo,
)]
pub struct HasPermissionQueryResult {
	pub onchain: bool,
	pub permission_reference: Option<PermissionReferenceRecord>,
}
