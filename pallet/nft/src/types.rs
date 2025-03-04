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

//! NFT module types

use crate::{Config, Error};
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{traits::Get, CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound};
use scale_info::TypeInfo;
use seed_pallet_common::utils::{TokenBurnAuthority, TokenUtilityFlags as TokenFlags};
use seed_primitives::{
	CrossChainCompatibility, IssuanceId, MetadataScheme, OriginChain, RoyaltiesSchedule,
	SerialNumber, TokenCount, TokenLockReason,
};
use serde::{Deserialize, Serialize};
use sp_runtime::{BoundedVec, Permill};
use sp_std::{fmt::Debug, prelude::*};

/// Information related to a specific collection
/// Need for separate collection structure from CollectionInformation for RPC call is cause
/// of complexity of deserialization/serialization BoundedVec
#[derive(
	PartialEqNoBound,
	RuntimeDebugNoBound,
	CloneNoBound,
	Encode,
	Serialize,
	Deserialize,
	Decode,
	TypeInfo,
)]
#[codec(mel_bound(AccountId: MaxEncodedLen))]
pub struct CollectionDetail<AccountId>
where
	AccountId: Debug + PartialEq + Clone,
{
	/// The owner of the collection
	pub owner: AccountId,
	/// A human friendly name
	pub name: Vec<u8>,
	/// Collection metadata reference scheme
	pub metadata_scheme: Vec<u8>,
	/// configured royalties schedule
	pub royalties_schedule: Option<Vec<(AccountId, Permill)>>,
	/// Maximum number of tokens allowed in a collection
	pub max_issuance: Option<TokenCount>,
	/// The chain in which the collection was minted originally
	pub origin_chain: OriginChain,
	/// The next available serial_number
	pub next_serial_number: SerialNumber,
	/// the total count of tokens in this collection
	pub collection_issuance: TokenCount,
	/// This collections compatibility with other chains
	pub cross_chain_compatibility: CrossChainCompatibility,
}

/// Information related to a specific collection
#[derive(
	PartialEqNoBound, RuntimeDebugNoBound, CloneNoBound, Encode, Decode, TypeInfo, MaxEncodedLen,
)]
#[codec(mel_bound(AccountId: MaxEncodedLen))]
#[scale_info(skip_type_params(StringLimit))]
pub struct CollectionInformation<AccountId, StringLimit>
where
	AccountId: Debug + PartialEq + Clone,
	StringLimit: Get<u32>,
{
	/// The owner of the collection
	pub owner: AccountId,
	/// A human friendly name
	pub name: BoundedVec<u8, StringLimit>,
	/// Collection metadata reference scheme
	pub metadata_scheme: MetadataScheme,
	/// configured royalties schedule
	pub royalties_schedule: Option<RoyaltiesSchedule<AccountId>>,
	/// Maximum number of tokens allowed in a collection
	pub max_issuance: Option<TokenCount>,
	/// The chain in which the collection was minted originally
	pub origin_chain: OriginChain,
	/// The next available serial_number
	pub next_serial_number: SerialNumber,
	/// the total count of tokens in this collection
	pub collection_issuance: TokenCount,
	/// This collections compatibility with other chains
	pub cross_chain_compatibility: CrossChainCompatibility,
}

/// Information related to a specific token
#[derive(
	PartialEqNoBound, RuntimeDebugNoBound, CloneNoBound, Encode, Decode, TypeInfo, MaxEncodedLen,
)]
#[codec(mel_bound(AccountId: MaxEncodedLen))]
pub struct TokenInformation<AccountId>
where
	AccountId: Debug + PartialEq + Clone,
{
	/// The owner of the token
	pub owner: AccountId,
	/// Does this token have any locks, i.e. locked for sale
	pub lock_status: Option<TokenLockReason>,
	/// transferable and burn authority flags
	pub utility_flags: TokenFlags,
}

impl<AccountId> TokenInformation<AccountId>
where
	AccountId: Debug + PartialEq + Clone,
{
	/// Creates a new instance of `TokenInformation` with the owner set to the provided account id
	pub fn new(owner: AccountId, utility_flags: TokenFlags) -> Self {
		TokenInformation { owner, lock_status: None, utility_flags }
	}
}

#[derive(
	PartialEqNoBound, RuntimeDebugNoBound, CloneNoBound, Encode, Decode, TypeInfo, MaxEncodedLen,
)]
pub struct PendingIssuance {
	pub issuance_id: IssuanceId,
	pub quantity: u32,
	pub burn_authority: TokenBurnAuthority,
}

pub enum PendingIssuanceError {
	PendingIssuanceLimitExceeded,
}

impl<T: Config> From<PendingIssuanceError> for Error<T> {
	fn from(val: PendingIssuanceError) -> Error<T> {
		match val {
			PendingIssuanceError::PendingIssuanceLimitExceeded => {
				Error::<T>::PendingIssuanceLimitExceeded
			},
		}
	}
}

/// The state of a collection's pending issuances
#[derive(
	PartialEqNoBound, RuntimeDebugNoBound, CloneNoBound, Encode, Decode, TypeInfo, MaxEncodedLen,
)]
#[codec(mel_bound(AccountId: MaxEncodedLen))]
#[scale_info(skip_type_params(MaxPendingIssuances))]
pub struct CollectionPendingIssuances<AccountId, MaxPendingIssuances: Get<u32>>
where
	AccountId: Debug + PartialEq + Clone,
	MaxPendingIssuances: Get<u32>,
{
	pub next_issuance_id: IssuanceId,
	pub pending_issuances: BoundedVec<
		(AccountId, BoundedVec<PendingIssuance, MaxPendingIssuances>),
		MaxPendingIssuances,
	>,
}

impl<AccountId, MaxPendingIssuances> Default
	for CollectionPendingIssuances<AccountId, MaxPendingIssuances>
where
	AccountId: Debug + PartialEq + Clone,
	MaxPendingIssuances: Get<u32>,
{
	fn default() -> Self {
		CollectionPendingIssuances { next_issuance_id: 0, pending_issuances: BoundedVec::new() }
	}
}

impl<AccountId, MaxPendingIssuances> CollectionPendingIssuances<AccountId, MaxPendingIssuances>
where
	AccountId: Debug + PartialEq + Clone,
	MaxPendingIssuances: Get<u32>,
{
	/// Creates a new instance of `CollectionPendingIssuances` with the next
	/// issuance id set to 0, and an empty list of pending issuances
	pub fn new() -> Self {
		CollectionPendingIssuances { next_issuance_id: 0, pending_issuances: BoundedVec::new() }
	}

	/// Inserts a new pending issuance for a token owner
	pub fn insert_pending_issuance(
		&mut self,
		token_owner: &AccountId,
		quantity: u32,
		burn_authority: TokenBurnAuthority,
	) -> Result<IssuanceId, PendingIssuanceError> {
		let issuance_id = self.next_issuance_id;
		let pending_issuance = PendingIssuance { issuance_id, quantity, burn_authority };

		if let Some(account_pending_issuances) =
			self.pending_issuances.iter_mut().find(|p| &p.0 == token_owner)
		{
			account_pending_issuances
				.1
				.try_push(pending_issuance)
				.map_err(|_| PendingIssuanceError::PendingIssuanceLimitExceeded)?;
		} else {
			// create new entry
			let mut new_account_issuance = BoundedVec::new();
			new_account_issuance.force_push(pending_issuance);

			self.pending_issuances
				.try_push((token_owner.clone(), new_account_issuance))
				.map_err(|_| PendingIssuanceError::PendingIssuanceLimitExceeded)?;
		}

		self.next_issuance_id = self.next_issuance_id.saturating_add(1);

		Ok(issuance_id)
	}

	/// Gets the pending issuance by the token owner and issuance id
	pub fn get_pending_issuance(
		&self,
		token_owner: &AccountId,
		issuance_id: IssuanceId,
	) -> Option<PendingIssuance> {
		let account_pending_issuances = self
			.pending_issuances
			.iter()
			.find(|pending_issuance| &pending_issuance.0 == token_owner)?;

		let pending_issuance =
			account_pending_issuances.1.iter().find(|p| p.issuance_id == issuance_id)?;

		Some(pending_issuance.clone())
	}

	/// Removes a pending issuance for a token owner
	pub fn remove_pending_issuance(&mut self, token_owner: &AccountId, issuance_id: IssuanceId) {
		for account_pending_issuance in self.pending_issuances.iter_mut() {
			if &account_pending_issuance.0 != token_owner {
				continue;
			}

			account_pending_issuance.1.retain(|p| p.issuance_id != issuance_id);
			break;
		}
	}

	/// Gets all pending issuances for a token owner
	pub fn get_pending_issuances(&self, token_owner: &AccountId) -> Vec<PendingIssuance> {
		if let Some(account_pending_issuances) = self
			.pending_issuances
			.iter()
			.find(|pending_issuance| &pending_issuance.0 == token_owner)
		{
			return account_pending_issuances.1.to_vec();
		}

		vec![]
	}
}
