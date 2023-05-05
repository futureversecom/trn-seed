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
use frame_support::{traits::Get, CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound};
use scale_info::TypeInfo;
use seed_primitives::{Balance, MetadataScheme, OriginChain, RoyaltiesSchedule, SerialNumber};
use sp_runtime::BoundedVec;
use sp_std::{fmt::Debug, prelude::*};

/// Information related to a specific collection
#[derive(PartialEqNoBound, RuntimeDebugNoBound, CloneNoBound, Encode, Decode, TypeInfo)]
#[codec(mel_bound(AccountId: MaxEncodedLen))]
#[scale_info(skip_type_params(StringLimit))]
pub struct SftCollectionInformation<AccountId, StringLimit>
where
	AccountId: Debug + PartialEq + Clone,
	StringLimit: Get<u32>,
{
	/// The owner of the collection
	pub collection_owner: AccountId,
	/// A human friendly name
	pub collection_name: BoundedVec<u8, StringLimit>,
	/// Collection metadata reference scheme
	pub metadata_scheme: MetadataScheme,
	/// configured royalties schedule
	pub royalties_schedule: Option<RoyaltiesSchedule<AccountId>>,
	/// The chain in which the collection was minted originally
	pub origin_chain: OriginChain,
	/// The next available serial_number
	pub next_serial_number: SerialNumber,
}

pub trait TokenInformation<AccountId> {
	/// Check whether who is the collection owner
	fn is_collection_owner(&self, who: &AccountId) -> bool;
}

impl<AccountId, StringLimit> TokenInformation<AccountId>
	for SftCollectionInformation<AccountId, StringLimit>
where
	AccountId: Debug + PartialEq + Clone,
	StringLimit: Get<u32>,
{
	/// Check whether who is the collection owner
	fn is_collection_owner(&self, who: &AccountId) -> bool {
		&self.collection_owner == who
	}
}

#[derive(PartialEqNoBound, RuntimeDebugNoBound, CloneNoBound, Encode, Decode, TypeInfo)]
#[codec(mel_bound(AccountId: MaxEncodedLen))]
#[scale_info(skip_type_params(StringLimit, MaxOwnersPerSftToken))]
pub struct SftTokenInformation<AccountId, StringLimit, MaxOwnersPerSftToken>
where
	AccountId: Debug + PartialEq + Clone,
	StringLimit: Get<u32>,
	MaxOwnersPerSftToken: Get<u32>,
{
	/// A human friendly name
	pub token_name: BoundedVec<u8, StringLimit>,
	/// Maximum number of this token allowed
	pub max_issuance: Option<Balance>,
	/// the total count of tokens in this collection
	pub token_issuance: Balance,
	/// Map from account to tokens owned by that account
	pub owned_tokens: BoundedVec<(AccountId, SftTokenBalance), MaxOwnersPerSftToken>,
}

impl<AccountId, StringLimit, MaxOwnersPerSftToken>
	SftTokenInformation<AccountId, StringLimit, MaxOwnersPerSftToken>
where
	AccountId: Debug + PartialEq + Clone,
	StringLimit: Get<u32>,
	MaxOwnersPerSftToken: Get<u32>,
{
	/// Returns the total balance of a token owned by who
	pub fn free_balance_of(&self, who: &AccountId) -> Balance {
		self.owned_tokens
			.iter()
			.find(|(account, _)| account == who)
			.map(|(_, balance)| balance.free_balance)
			.unwrap_or_default()
	}

	/// Adds some balance into an account
	pub fn add_balance(
		&mut self,
		who: &AccountId,
		amount: Balance,
	) -> Result<(), TokenBalanceError> {
		let existing_owner = self.owned_tokens.iter_mut().find(|(account, _)| account == who);

		if let Some((_, balance)) = existing_owner {
			// Owner already exists, add to their balance
			balance.add_free_balance(amount)?;
		} else {
			// The owner doesn't exist for this SerialNumber, add them
			let new_token_balance = SftTokenBalance::new(amount, 0);
			self.owned_tokens
				.try_push((who.clone(), new_token_balance))
				.map_err(|_| TokenBalanceError::MaxOwnersReached)?;
		}
		Ok(())
	}

	/// Removes some balance from an account
	pub fn remove_balance(
		&mut self,
		who: &AccountId,
		amount: Balance,
	) -> Result<(), TokenBalanceError> {
		let Some((_, existing_balance)) = self.owned_tokens.iter_mut().find(|(account, _)| account == who) else {
			return Err(TokenBalanceError::InsufficientBalance.into());
		};

		existing_balance.remove_free_balance(amount)?;

		if existing_balance.total_balance() == 0 {
			// Remove the owner if they have no balance
			self.owned_tokens.retain(|(account, _)| account != who);
		}

		Ok(())
	}

	/// Transfers some balance from one account to another
	pub fn transfer_balance(
		&mut self,
		from: &AccountId,
		to: &AccountId,
		amount: Balance,
	) -> Result<(), TokenBalanceError> {
		self.remove_balance(from, amount)?;
		self.add_balance(to, amount)?;
		Ok(())
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

pub enum TokenBalanceError {
	InsufficientBalance,
	Overflow,
	MaxOwnersReached,
}

impl<T: Config> From<TokenBalanceError> for Error<T> {
	fn from(error: TokenBalanceError) -> Self {
		match error {
			TokenBalanceError::InsufficientBalance => Error::<T>::InsufficientBalance.into(),
			TokenBalanceError::Overflow => Error::<T>::Overflow.into(),
			TokenBalanceError::MaxOwnersReached => Error::<T>::MaxOwnersReached.into(),
		}
	}
}

impl Default for SftTokenBalance {
	fn default() -> Self {
		SftTokenBalance { free_balance: 0, reserved_balance: 0 }
	}
}

impl SftTokenBalance {
	/// Creates a news instance of SftTokenBalance
	pub fn new(free_balance: Balance, reserved_balance: Balance) -> Self {
		SftTokenBalance { free_balance, reserved_balance }
	}
	/// Returns the total balance
	pub fn total_balance(&self) -> Balance {
		self.free_balance + self.reserved_balance
	}

	/// Adds some balance to the free balance
	pub fn add_free_balance(&mut self, amount: Balance) -> Result<(), TokenBalanceError> {
		self.free_balance =
			self.free_balance.checked_add(amount).ok_or(TokenBalanceError::Overflow)?;
		Ok(())
	}

	/// Removes some balance from the free balance
	pub fn remove_free_balance(&mut self, amount: Balance) -> Result<(), TokenBalanceError> {
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
