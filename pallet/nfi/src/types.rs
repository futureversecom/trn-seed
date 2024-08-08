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
use frame_support::{traits::Get, BoundedVec, CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound};
use scale_info::TypeInfo;
use seed_primitives::{AssetId, Balance};
use sp_core::H256;
use sp_std::default::Default;

/// Categorise the NFI sub type. This is to futureproof the pallet and to allow for multiple
/// pieces of data to be stored per token
#[derive(Decode, Encode, Copy, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub enum NFISubType {
	NFI,
	Jen,
}

// What data type is stored for each SubType?
#[derive(
	RuntimeDebugNoBound, CloneNoBound, PartialEqNoBound, Eq, Decode, Encode, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxDataLength))]
pub enum NFIDataType<MaxDataLength: Get<u32>> {
	NFI(NFIMatrix<MaxDataLength>),
	Jen(u64),
}

impl<MaxDataLength: Get<u32>> From<NFIDataType<MaxDataLength>> for NFISubType {
	fn from(data: NFIDataType<MaxDataLength>) -> Self {
		match data {
			NFIDataType::NFI(_) => NFISubType::NFI,
			NFIDataType::Jen(_) => NFISubType::Jen,
		}
	}
}

#[derive(
	RuntimeDebugNoBound,
	Default,
	CloneNoBound,
	PartialEqNoBound,
	Eq,
	Decode,
	Encode,
	TypeInfo,
	MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxDataLength))]
pub struct NFIMatrix<MaxDataLength: Get<u32>> {
	pub metadata_link: BoundedVec<u8, MaxDataLength>,
	pub verification_hash: H256,
}

#[derive(Decode, Encode, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound(AccountId: MaxEncodedLen))]
pub struct FeeDetails<AccountId> {
	pub asset_id: AssetId,
	pub amount: Balance,
	pub receiver: AccountId,
}
