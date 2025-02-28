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

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{traits::Get, CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound};
use scale_info::TypeInfo;
use seed_pallet_common::utils::TokenBurnAuthority;
use seed_primitives::{
	CrossChainCompatibility, IssuanceId, MetadataScheme, OriginChain, RoyaltiesSchedule,
	SerialNumber, TokenCount,
};
use serde::{Deserialize, Serialize};
use sp_runtime::{BoundedVec, Permill};
use sp_std::{fmt::Debug, prelude::*};

#[derive(Decode, Encode, Debug, Clone, Copy, PartialEq, TypeInfo)]
pub enum TokenOwnershipError {
	TokenLimitExceeded,
}

/// Struct that represents the owned serial numbers within a collection of an individual account
#[derive(
	PartialEqNoBound, RuntimeDebugNoBound, Decode, Encode, CloneNoBound, TypeInfo, MaxEncodedLen,
)]
#[codec(mel_bound(AccountId: MaxEncodedLen))]
#[scale_info(skip_type_params(MaxTokensPerCollection))]
pub struct TokenOwnership<AccountId, MaxTokensPerCollection>
where
	AccountId: Debug + PartialEq + Clone,
	MaxTokensPerCollection: Get<u32>,
{
	pub owner: AccountId,
	pub owned_serials: BoundedVec<SerialNumber, MaxTokensPerCollection>,
}

impl<AccountId, MaxTokensPerCollection> TokenOwnership<AccountId, MaxTokensPerCollection>
where
	AccountId: Debug + PartialEq + Clone,
	MaxTokensPerCollection: Get<u32>,
{
	/// Creates a new TokenOwnership with the given owner and serial numbers
	pub fn new(
		owner: AccountId,
		serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection>,
	) -> Self {
		let mut owned_serials = serial_numbers.clone();
		Self { owner, owned_serials }
	}

	/// Adds a serial to owned_serials and sorts the vec
	pub fn add(&mut self, serial_number: SerialNumber) -> Result<(), TokenOwnershipError> {
		self.owned_serials
			.try_push(serial_number)
			.map_err(|_| TokenOwnershipError::TokenLimitExceeded)?;
		Ok(())
	}

	/// Returns true if the serial number is containerd within owned_serials
	pub fn contains_serial(&self, serial_number: &SerialNumber) -> bool {
		self.owned_serials.contains(serial_number)
	}
}

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
#[scale_info(skip_type_params(MaxTokensPerCollection, StringLimit))]
pub struct CollectionInformation<AccountId, MaxTokensPerCollection, StringLimit>
where
	AccountId: Debug + PartialEq + Clone,
	MaxTokensPerCollection: Get<u32>,
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
	/// All serial numbers owned by an account in a collection
	pub owned_tokens:
		BoundedVec<TokenOwnership<AccountId, MaxTokensPerCollection>, MaxTokensPerCollection>,
}

impl<AccountId, MaxTokensPerCollection, StringLimit>
	CollectionInformation<AccountId, MaxTokensPerCollection, StringLimit>
where
	AccountId: Debug + PartialEq + Clone,
	MaxTokensPerCollection: Get<u32>,
	StringLimit: Get<u32>,
{
	/// Check whether a token has been minted in a collection
	pub fn token_exists(&self, serial_number: SerialNumber) -> bool {
		self.owned_tokens
			.iter()
			.any(|token_ownership| token_ownership.contains_serial(&serial_number))
	}

	/// Check whether who is the collection owner
	pub fn is_collection_owner(&self, who: &AccountId) -> bool {
		&self.owner == who
	}

	/// Check whether who owns the serial number in collection_info
	pub fn is_token_owner(&self, who: &AccountId, serial_number: SerialNumber) -> bool {
		self.owned_tokens.iter().any(|token_ownership| {
			if &token_ownership.owner == who {
				token_ownership.contains_serial(&serial_number)
			} else {
				false
			}
		})
	}

	/// Get's the token owner
	pub fn get_token_owner(&self, serial_number: SerialNumber) -> Option<AccountId> {
		let token = self.owned_tokens.iter().find(|x| x.contains_serial(&serial_number))?;
		Some(token.owner.clone())
	}

	/// Adds a list of tokens to a users balance in collection_info
	pub fn add_user_tokens(
		&mut self,
		token_owner: &AccountId,
		serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection>,
	) -> Result<(), TokenOwnershipError> {
		if let Some(token_ownership) =
			self.owned_tokens.iter_mut().find(|p| &p.owner == token_owner)
		{
			// Add new serial numbers to existing owner
			for serial_number in serial_numbers.iter() {
				token_ownership.add(*serial_number)?;
			}
		} else {
			// If token owner doesn't exist, create new entry
			let new_token_ownership = TokenOwnership::new(token_owner.clone(), serial_numbers);
			self.owned_tokens
				.try_push(new_token_ownership)
				.map_err(|_| TokenOwnershipError::TokenLimitExceeded)?;
		}
		Ok(())
	}

	/// Removes a list of tokens from a users balance in collection_info
	pub fn remove_user_tokens(
		&mut self,
		token_owner: &AccountId,
		serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection>,
	) {
		let mut removing_all_tokens: bool = false;
		for token_ownership in self.owned_tokens.iter_mut() {
			if &token_ownership.owner != token_owner {
				continue;
			}
			token_ownership.owned_serials.retain(|serial| !serial_numbers.contains(serial));
			removing_all_tokens = token_ownership.owned_serials.is_empty();
			break;
		}
		// Check whether the owner has any tokens left, if not remove them from the collection
		if removing_all_tokens {
			self.owned_tokens
				.retain(|token_ownership| &token_ownership.owner != token_owner);
		}
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
