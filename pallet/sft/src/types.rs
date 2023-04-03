/* Copyright 2019-2021 Centrality Investments Limited
 *
 * Licensed under the LGPL, Version 3.0 (the "License");
 * you may not use this file except in compliance with the License.
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * You may obtain a copy of the License at the root of this project source code,
 * or at:
 *     https://centrality.ai/licenses/gplv3.txt
 *     https://centrality.ai/licenses/lgplv3.txt
 */

//! SFT module types

use crate::Config;

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use seed_primitives::{
	Balance, CollectionNameType, MetadataScheme, OriginChain, RoyaltiesSchedule, SerialNumber,
	TokenCount,
};
use sp_runtime::BoundedVec;
use sp_std::prelude::*;

// TODO
// pub type CollectionNameType = BoundedVec<u8>;

/// Information related to a specific collection
#[derive(Debug, Clone, Encode, Decode, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct SftCollectionInformation<T: Config> {
	/// The owner of the collection
	pub collection_owner: T::AccountId,
	/// A human friendly name
	//pub name: CollectionNameType,
	/// Collection metadata reference scheme
	pub metadata_scheme: MetadataScheme,
	/// configured royalties schedule
	pub royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
	/// The chain in which the collection was minted originally
	pub origin_chain: OriginChain,
	/// The next available serial_number
	pub next_serial_number: SerialNumber,
}

pub trait TokenInformation<T: Config> {
	/// Check whether who is the collection owner
	fn is_collection_owner(&self, who: &T::AccountId) -> bool;
}

// TODO Add a common trait for both SFT and NFT that shares a lot of the functionality
// that is implemented on each struct. i.e. is_collection_owner()

impl<T: Config> TokenInformation<T> for SftCollectionInformation<T> {
	/// Check whether who is the collection owner
	fn is_collection_owner(&self, who: &T::AccountId) -> bool {
		&self.collection_owner == who
	}
}

#[derive(Debug, Clone, Encode, Decode, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct SftTokenInformation<T: Config> {
	/// The owner of the token
	pub token_owner: T::AccountId,
	/// A human friendly name
	pub name: CollectionNameType,
	/// Maximum number of this token allowed
	pub max_issuance: Option<u128>,
	/// the total count of tokens in this collection
	pub token_issuance: u128,
	/// Map from account to tokens owned by that account
	pub owned_tokens:
		BoundedVec<(T::AccountId, SftTokenBalance<T>), <T as Config>::MaxOwnersPerSftToken>,
}

impl<T: Config> SftTokenInformation<T> {
	/// Returns the total balance of a token owned by who
	pub fn free_balance_of(&self, who: &T::AccountId) -> Balance {
		self.owned_tokens
			.iter()
			.find(|(account, _)| account == who)
			.map(|(_, balance)| balance.free_balance)
			.unwrap_or_default()
	}
}

/// Holds information about a users balance of a specific token
/// An amount of SFT balance can be reserved when listed for sale
#[derive(Debug, Clone, Encode, Decode, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct SftTokenBalance<T: Config> {
	_phantom: sp_std::marker::PhantomData<T>,
	// The balance currently available
	pub free_balance: Balance,
	// The reserved balance, not transferable unless unlocked
	pub reserved_balance: Balance,
}

// TODO Go back to using pallet error
pub enum TokenBalanceError {
	InsufficientBalance,
	Overflow,
}

impl<T: Config> SftTokenBalance<T> {
	pub fn new(free_balance: u128, reserved_balance: u128) -> Self {
		SftTokenBalance { _phantom: Default::default(), free_balance, reserved_balance }
	}
	/// Returns the total balance
	pub fn total_balance(&self) -> Balance {
		self.free_balance + self.reserved_balance
	}

	/// Adds some balance to the free balance
	pub fn add_balance(&mut self, amount: Balance) -> Result<(), TokenBalanceError> {
		// self.free_balance =
		// 	self.free_balance.checked_add(amount).ok_or(Err(TokenBalanceError::Overflow))?;
		Ok(())
	}

	/// Removes some balance from the free balance
	pub fn remove_balance(&mut self, amount: Balance) -> Result<(), TokenBalanceError> {
		if self.free_balance < amount {
			return Err(TokenBalanceError::InsufficientBalance)
		}
		self.free_balance -= amount;
		Ok(())
	}

	/// Reserves some balance
	pub fn place_reserve(&mut self, amount: Balance) -> Result<(), TokenBalanceError> {
		if self.free_balance < amount {
			return Err(TokenBalanceError::InsufficientBalance)
		}
		self.free_balance -= amount;
		self.reserved_balance =
			self.reserved_balance.checked_add(amount).ok_or(TokenBalanceError::Overflow)?;
		Ok(())
	}

	/// Removes some balance from reserved
	pub fn remove_reserve(&mut self, amount: Balance) -> Result<(), TokenBalanceError> {
		if self.reserved_balance < amount {
			return Err(TokenBalanceError::InsufficientBalance)
		}
		self.reserved_balance -= amount;
		self.free_balance =
			self.free_balance.checked_add(amount).ok_or(TokenBalanceError::Overflow)?;
		Ok(())
	}
}
