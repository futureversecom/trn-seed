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

use crate::{Config, Error};

use codec::{Decode, Encode};
use frame_support::dispatch::DispatchResult;
use scale_info::TypeInfo;
use seed_primitives::{
	AssetId, Balance, BlockNumber, CollectionUuid, MetadataScheme, SerialNumber, TokenId,
};
use sp_runtime::{BoundedVec, PerThing, Permill};
use sp_std::prelude::*;

//pub type SftBalance = u128;

/// Struct that represents the owned serial numbers within a collection of an individual account
#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ERC1155Ownership<T: Config> {
	pub owner: T::AccountId,
	// Owned serials maps to the number of each semi fungible token owned
	pub owned_serials:
		BoundedVec<(SerialNumber, TokenCount), <T as Config>::MaxTokensPerCollection>,
}

impl<T: Config> ERC1155Ownership<T> {
	/// Creates a new ERC721Ownership with the given owner and serial numbers
	pub fn new(
		owner: T::AccountId,
		serial_numbers: BoundedVec<(SerialNumber, TokenCount), T::MaxTokensPerCollection>,
	) -> Self {
		let mut owned_serials = serial_numbers.clone();
		owned_serials.sort();
		Self { owner, owned_serials }
	}

	/// Adds a serial to owned_serials and sorts the vec
	pub fn add(&mut self, serial_number: SerialNumber) -> DispatchResult {
		self.owned_serials
			.try_push(serial_number)
			.map_err(|_| Error::<T>::TokenLimitExceeded)?;
		self.owned_serials.sort();
		Ok(())
	}

	/// Returns true if the serial number is containerd within owned_serials
	pub fn contains_serial(&self, serial_number: &SerialNumber) -> bool {
		self.owned_serials.map(|(serial, quantity)| serial).contains(serial_number)
	}
}

/// Information related to a specific collection
#[derive(Debug, Clone, Encode, Decode, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct SftCollectionInformation<T: Config> {
	/// The owner of the collection
	pub owner: T::AccountId,
	/// A human friendly name
	pub name: CollectionNameType,
	/// Collection metadata reference scheme
	pub metadata_scheme: MetadataScheme,
	/// configured royalties schedule
	pub royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
	/// The chain in which the collection was minted originally
	pub origin_chain: OriginChain,
	/// The next available serial_number
	pub next_serial_number: SerialNumber,
	/// This collections compatibility with other chains, SFT remains incompatible
	pub cross_chain_compatibility: CrossChainCompatibility,
	/// Information on tokens within this collection
	pub token_information: BoundedVec<
		(SerialNumber, SftTokenInformation<T>),
		<T as Config>::MaxTokensPerSftCollection,
	>,
}

pub struct SftTokenInformation<T: Config> {
	pub name: CollectionNameType,
	/// Maximum number of this token allowed
	pub max_issuance: Option<TokenCount>,
	/// the total count of tokens in this collection
	pub token_issuance: TokenCount,
	/// Map from account to tokens owned by that account
	pub owned_tokens:
		BoundedVec<(T::AccountId, SftTokenBalance), <T as Config>::MaxOwnersPerSftToken>,
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
pub struct SftTokenBalance {
	// The balance currently available
	pub free_balance: Balance,
	// The reserved balance, not transferable unless unlocked
	pub reserved_balance: Balance,
}

impl SftTokenBalance {
	/// Returns the total balance
	pub fn total_balance(&self) -> Balance {
		self.free_balance + self.reserved_balance
	}

	/// Adds some balance to the free balance
	pub fn add_balance(&mut self, amount: Balance) -> DispatchResult {
		self.free_balance = self.free_balance.checked_add(amount).ok_or(Error::<T>::Overflow)?;
		Ok(())
	}

	/// Removes some balance from the free balance
	pub fn remove_balance(&mut self, amount: Balance) -> DispatchResult {
		if self.free_balance < amount {
			return Err(Error::<T>::InsufficientBalance.into())
		}
		self.free_balance -= amount;
		Ok(())
	}

	/// Reserves some balance
	pub fn place_reserve(&mut self, amount: Balance) -> DispatchResult {
		if self.free_balance < amount {
			return Err(Error::<T>::InsufficientBalance.into())
		}
		self.free_balance -= amount;
		self.reserved_balance =
			self.reserved_balance.checked_add(amount).ok_or(Error::<T>::Overflow)?;
		Ok(())
	}

	/// Removes some balance from reserved
	pub fn remove_reserve(&mut self, amount: Balance) -> DispatchResult {
		if self.reserved_balance < amount {
			return Err(Error::<T>::InsufficientBalance.into())
		}
		self.reserved_balance -= amount;
		self.free_balance = self.free_balance.checked_add(amount).ok_or(Error::<T>::Overflow)?;
		Ok(())
	}
}
