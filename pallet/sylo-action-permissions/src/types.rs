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

use core::fmt::Debug;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{traits::Get, CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound};
use scale_info::TypeInfo;
use seed_primitives::Balance;
use sp_runtime::{BoundedBTreeSet, BoundedVec};

#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Clone, PartialEq, Eq)]
pub enum Spender {
	Grantor,
	Grantee,
}

// Tuple of pallet name and extrinsic name which identifies a specific runtime call.
pub type CallId<StringLimit> = (BoundedVec<u8, StringLimit>, BoundedVec<u8, StringLimit>);

#[derive(
	CloneNoBound, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebugNoBound, PartialEqNoBound, Eq,
)]
#[scale_info(skip_type_params(MaxCallIds, StringLimit))]
pub struct DispatchPermission<BlockNumber, MaxCallIds, StringLimit>
where
	BlockNumber: Debug + PartialEq + Clone,
	MaxCallIds: Get<u32>,
	StringLimit: Get<u32>,
{
	// Whether the extrinsic will be paid from the grantor or grantee
	pub spender: Spender,

	// If grantor is spender, then allow grantor to set a limit on the
	// amount the grantee is allowed to spend
	pub spending_balance: Option<Balance>,

	// Optional set of calls (pallet, extrinsic name) that this dispatch
	// permission is valid for. If None, then all extrinsics are allowed.
	pub allowed_calls: BoundedBTreeSet<CallId<StringLimit>, MaxCallIds>,

	// The block number this permission was established
	pub block: BlockNumber,

	// An optional expiry for this permission
	pub expiry: Option<BlockNumber>,
}
