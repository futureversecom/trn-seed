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
use frame_support::{
	parameter_types, traits::Get, BoundedVec, CloneNoBound, EqNoBound, PartialEqNoBound,
	RuntimeDebug, RuntimeDebugNoBound,
};
use scale_info::TypeInfo;
use seed_primitives::{AssetId, Balance, Block};
use sp_core::{hexdisplay::AsBytesRef, H160, H256};
use sp_std::{fmt::Debug, prelude::*};

#[derive(
	CloneNoBound,
	RuntimeDebugNoBound,
	Encode,
	Decode,
	PartialEqNoBound,
	EqNoBound,
	TypeInfo,
	MaxEncodedLen,
)]
#[scale_info(skip_type_params(StringLimit))]
pub struct ResolverId<StringLimit>
where
	StringLimit: Get<u32>,
{
	pub method: BoundedVec<u8, StringLimit>,
	pub identifier: BoundedVec<u8, StringLimit>,
}

impl<T: Get<u32>> ResolverId<T> {
	pub fn to_did(&self) -> Vec<u8> {
		return Vec::new();
		// let method = self.method.to_vec();
		// let method = String::from_utf8_lossy(method.as_bytes_ref());

		// let identifier = self.identifier.to_vec();
		// let identifier = String::from_utf8_lossy(identifier.as_bytes_ref());

		// format!("did:{method}:{identifier}").as_bytes().to_vec()
	}
}

pub type ServiceEndpoint<StringLimit> = BoundedVec<u8, StringLimit>;

#[derive(
	Clone, Encode, Decode, RuntimeDebugNoBound, PartialEqNoBound, Eq, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxServiceEndpoints, StringLimit))]
pub struct Resolver<AccountId, MaxServiceEndpoints, StringLimit>
where
	AccountId: Debug + PartialEq + Clone,
	MaxServiceEndpoints: Get<u32>,
	StringLimit: Get<u32>,
{
	pub controller: AccountId,
	pub service_endpoints: BoundedVec<ServiceEndpoint<StringLimit>, MaxServiceEndpoints>,
}

#[derive(
	Clone, Encode, Decode, RuntimeDebugNoBound, PartialEqNoBound, Eq, TypeInfo, MaxEncodedLen,
)]
pub struct ValidationEntry<BlockNumber>
where
	BlockNumber: Debug + PartialEq + Clone,
{
	pub checksum: H256,
	pub block: BlockNumber,
}

#[derive(
	Clone, Encode, Decode, RuntimeDebugNoBound, PartialEqNoBound, Eq, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxResolvers, MaxTags, MaxEntries, StringLimit))]
pub struct ValidationRecord<AccountId, BlockNumber, MaxResolvers, MaxTags, MaxEntries, StringLimit>
where
	AccountId: Debug + PartialEq + Clone,
	BlockNumber: Debug + PartialEq + Clone,
	MaxResolvers: Get<u32>,
	MaxTags: Get<u32>,
	MaxEntries: Get<u32>,
	StringLimit: Get<u32>,
{
	pub author: AccountId,
	pub resolvers: BoundedVec<ResolverId<StringLimit>, MaxResolvers>,
	pub data_type: BoundedVec<u8, StringLimit>,
	pub tags: BoundedVec<BoundedVec<u8, StringLimit>, MaxTags>,
	pub entries: BoundedVec<ValidationEntry<BlockNumber>, MaxEntries>,
}
