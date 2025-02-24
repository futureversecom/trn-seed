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

use crate::Config;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{traits::Get, CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound};
use scale_info::TypeInfo;
use seed_primitives::{
	CrossChainCompatibility, MetadataScheme, OriginChain, RoyaltiesSchedule, SerialNumber,
	TokenCount,
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

#[derive(Decode, Encode, Debug, Clone, Copy, PartialEq, TypeInfo)]
pub enum TokenOwnershipError {
	TokenLimitExceeded,
}

impl<T: Config> From<TokenOwnershipError> for crate::Error<T> {
	fn from(val: TokenOwnershipError) -> crate::Error<T> {
		match val {
			TokenOwnershipError::TokenLimitExceeded => crate::Error::<T>::TokenLimitExceeded,
		}
	}
}

/// Type to denote an account and it's owned serial numbers within a collection
pub type OwnedTokens<AccountId, MaxTokensPerCollection> =
	(AccountId, BoundedVec<SerialNumber, MaxTokensPerCollection>);

/// Contains ownership info for all tokens within a collection
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
	/// List of all token owners
	pub owned_tokens:
		BoundedVec<OwnedTokens<AccountId, MaxTokensPerCollection>, MaxTokensPerCollection>,
}

impl<AccountId, MaxTokensPerCollection> Default
	for TokenOwnership<AccountId, MaxTokensPerCollection>
where
	AccountId: Debug + PartialEq + Clone,
	MaxTokensPerCollection: Get<u32>,
{
	fn default() -> Self {
		Self { owned_tokens: BoundedVec::default() }
	}
}

impl<AccountId, MaxTokensPerCollection> TokenOwnership<AccountId, MaxTokensPerCollection>
where
	AccountId: Debug + PartialEq + Clone,
	MaxTokensPerCollection: Get<u32>,
{
	pub fn new(
		account: AccountId,
		serials: BoundedVec<SerialNumber, MaxTokensPerCollection>,
	) -> Self {
		let owned_tokens = BoundedVec::truncate_from(vec![(account, serials)]);
		Self { owned_tokens }
	}
	/// Check whether who owns the serial number in collection_info
	pub fn is_token_owner(&self, who: &AccountId, serial_number: SerialNumber) -> bool {
		self.owned_tokens.iter().any(|(owner, owned_serials)| {
			if owner == who {
				owned_serials.contains(&serial_number)
			} else {
				false
			}
		})
	}

	/// Retrieve the token owner of a specified serial number
	pub fn get_token_owner(&self, serial_number: SerialNumber) -> Option<AccountId> {
		let Some((token_owner, _)) = self
			.owned_tokens
			.iter()
			.find(|(_, owned_serials)| owned_serials.contains(&serial_number))
		else {
			return None;
		};
		Some(token_owner.clone())
	}

	/// Check whether a token has been minted in a collection
	pub fn token_exists(&self, serial_number: SerialNumber) -> bool {
		self.owned_tokens
			.iter()
			.any(|(_, owned_serials)| owned_serials.contains(&serial_number))
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
			.any(|token_ownership| &token_ownership.0 == token_owner)
		{
			for (owner, owned_serials) in self.owned_tokens.iter_mut() {
				if owner != token_owner {
					continue;
				}
				// Add new serial numbers to existing owner
				for serial_number in serial_numbers.iter() {
					owned_serials
						.try_push(*serial_number)
						.map_err(|_| TokenOwnershipError::TokenLimitExceeded)?;
					owned_serials.sort();
				}
			}
		} else {
			// If token owner doesn't exist, create new entry
			let new_owned_tokens = (token_owner.clone(), serial_numbers);
			self.owned_tokens
				.try_push(new_owned_tokens)
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
		for (owner, owned_serials) in self.owned_tokens.iter_mut() {
			if owner != token_owner {
				continue;
			}
			owned_serials.retain(|serial| !serial_numbers.contains(serial));
			removing_all_tokens = owned_serials.is_empty();
			break;
		}
		// Check whether the owner has any tokens left, if not remove them from the collection
		if removing_all_tokens {
			self.owned_tokens.retain(|(owner, _)| owner != token_owner);
		}
	}
}
