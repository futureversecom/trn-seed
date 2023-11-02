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
	Balance, CollectionUuid, SerialNumber, TokenCount, TokenId, TokenLockReason,
};
use sp_runtime::{BoundedVec, DispatchError, DispatchResult};
use sp_std::{fmt::Debug, vec::Vec};

pub trait SFTExt {
	type AccountId: Debug + PartialEq + Clone;
	type MaxSerialsPerMint: Get<u32>;

	fn do_transfer(
		origin: Self::AccountId,
		collection_id: CollectionUuid,
		serial_numbers: BoundedVec<(SerialNumber, Balance), Self::MaxSerialsPerMint>,
		new_owner: Self::AccountId,
	) -> DispatchResult;

	fn reserve_balance(token_id: TokenId, amount: Balance, who: &Self::AccountId)
		-> DispatchResult;

	fn free_reserved_balance(
		token_id: TokenId,
		amount: Balance,
		who: &Self::AccountId,
	) -> DispatchResult;
}
