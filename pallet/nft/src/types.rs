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
use seed_primitives::{MetadataScheme, OriginChain, RoyaltiesSchedule, SerialNumber, TokenCount};
use serde::{ser::SerializeStruct, Deserialize, Serialize};
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
		owned_serials.sort();
		Self { owner, owned_serials }
	}

	/// Adds a serial to owned_serials and sorts the vec
	pub fn add(&mut self, serial_number: SerialNumber) -> Result<(), TokenOwnershipError> {
		self.owned_serials
			.try_push(serial_number)
			.map_err(|_| TokenOwnershipError::TokenLimitExceeded)?;
		self.owned_serials.sort();
		Ok(())
	}

	/// Returns true if the serial number is containerd within owned_serials
	pub fn contains_serial(&self, serial_number: &SerialNumber) -> bool {
		self.owned_serials.contains(serial_number)
	}
}

/// Determines compatibility with external chains.
/// If compatible with XRPL, XLS-20 tokens will be minted with every newly minted
/// token on The Root Network
#[derive(Debug, Clone, Encode, Decode, Deserialize, PartialEq, TypeInfo, Copy, MaxEncodedLen)]
pub struct CrossChainCompatibility {
	/// This collection is compatible with the XLS-20 standard on XRPL
	pub xrpl: bool,
}

impl Serialize for CrossChainCompatibility {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let mut s = serializer.serialize_struct("CrossChainCompatibility", 1)?;
		s.serialize_field("xrpl", &self.xrpl)?;
		s.end()
	}
}

impl Default for CrossChainCompatibility {
	fn default() -> Self {
		Self { xrpl: false }
	}
}

/// Information related to a specific collection
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
		let Some(token) = self.owned_tokens.iter().find(|x| x.contains_serial(&serial_number))
		else {
			return None;
		};
		Some(token.owner.clone())
	}

	/// Adds a list of tokens to a users balance in collection_info
	pub fn add_user_tokens(
		&mut self,
		token_owner: &AccountId,
		serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection>,
	) -> Result<(), TokenOwnershipError> {
		if self
			.owned_tokens
			.iter()
			.any(|token_ownership| &token_ownership.owner == token_owner)
		{
			for token_ownership in self.owned_tokens.iter_mut() {
				if &token_ownership.owner != token_owner {
					continue;
				}
				// Add new serial numbers to existing owner
				for serial_number in serial_numbers.iter() {
					token_ownership.add(*serial_number)?;
				}
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
