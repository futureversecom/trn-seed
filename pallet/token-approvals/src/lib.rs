/* Copyright 2021 Centrality Investments Limited
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
#![cfg_attr(not(feature = "std"), no_std)]

//! Seed token approvals
//!
//! Module for handling approvals on Seed network to allow for ERC-721 and ERC-20 crossover
//!
//! Ethereum standards allow for token transfers of accounts on behalf of the token owner
//! to allow for easier precompiling of ERC-721 and ERC-20 tokens, this module handles approvals on
//! Seed for token transfers.

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use seed_pallet_common::{GetTokenOwner, OnTransferSubscriber};
use seed_primitives::{AssetId, Balance, CollectionUuid, TokenId};
use sp_runtime::{traits::Zero, DispatchResult};

mod migration;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: frame_system::Config {
		/// NFT ownership interface
		type GetTokenOwner: GetTokenOwner<AccountId = Self::AccountId>;
	}

	// Account with transfer approval for a single NFT
	#[pallet::storage]
	#[pallet::getter(fn erc721_approvals)]
	pub type ERC721Approvals<T: Config> = StorageMap<_, Twox64Concat, TokenId, T::AccountId>;

	// Accounts with transfer approval for an NFT collection of another account
	#[pallet::storage]
	#[pallet::getter(fn erc721_approvals_for_all)]
	pub type ERC721ApprovalsForAll<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		(CollectionUuid, T::AccountId),
		bool,
	>;

	// Mapping from account/ asset_id to an approved balance of another account
	#[pallet::storage]
	#[pallet::getter(fn erc20_approvals)]
	pub type ERC20Approvals<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		(T::AccountId, AssetId),
		Twox64Concat,
		T::AccountId,
		Balance,
	>;

	#[pallet::error]
	pub enum Error<T> {
		/// The token doesn't exist
		NoToken,
		/// The account is not the owner of the token
		NotTokenOwner,
		/// The account is not the owner of the token or an approved account
		NotTokenOwnerOrApproved,
		/// The caller account can't be the same as the operator account
		CallerNotOperator,
		/// The caller is not approved for the requested amount
		ApprovedAmountTooLow,
		/// The caller isn't approved for any amount
		CallerNotApproved,
		/// Address is already approved
		AlreadyApproved,
		/// There is no approval set for this token
		ApprovalDoesntExist,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_runtime_upgrade() -> Weight {
			use frame_support::{
				weights::constants::RocksDbWeight as DbWeight, IterableStorageDoubleMap,
			};
			use migration::v1_storage;
			use sp_std::vec::Vec;

			if StorageVersion::get::<Self>() == 0 {
				StorageVersion::new(1).put::<Self>();

				// Get values from old storage
				let old_approvals_for_all: Vec<(T::AccountId, CollectionUuid, T::AccountId)> =
					v1_storage::ERC721ApprovalsForAll::<T>::iter().collect();
				let weight = old_approvals_for_all.len() as Weight;

				for (owner, collection_id, spender) in old_approvals_for_all {
					// Insert values into new storage
					ERC721ApprovalsForAll::<T>::insert(owner, (collection_id, spender), true);
				}

				return 6_000_000 as Weight
					+ DbWeight::get().reads_writes(weight as Weight + 1, weight as Weight + 1);
			} else {
				Zero::zero()
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set approval for a single NFT
		/// Mapping from token_id to operator
		/// clears approval on transfer
		/// mapping(uint256 => address) private _tokenApprovals;
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn erc721_approval(
			origin: OriginFor<T>,
			caller: T::AccountId,
			operator_account: T::AccountId,
			token_id: TokenId,
		) -> DispatchResult {
			let _ = ensure_none(origin)?;
			ensure!(caller != operator_account, Error::<T>::CallerNotOperator);
			// Check that origin owns NFT or is approved_for_all
			let token_owner = match T::GetTokenOwner::get_owner(&token_id) {
				Some(owner) => owner,
				None => return Err(Error::<T>::NoToken.into()),
			};

			let is_approved_for_all =
				Self::erc721_approvals_for_all(&token_owner, (token_id.0, caller.clone()))
					.unwrap_or_default();
			ensure!(
				token_owner == caller || is_approved_for_all,
				Error::<T>::NotTokenOwnerOrApproved
			);
			ERC721Approvals::<T>::insert(token_id, &operator_account);
			Ok(())
		}

		/// Public method which allows users to remove approvals on a token they own
		#[pallet::weight(T::DbWeight::get().reads_writes(2, 1))]
		pub fn erc721_remove_approval(origin: OriginFor<T>, token_id: TokenId) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			ensure!(ERC721Approvals::<T>::contains_key(token_id), Error::<T>::ApprovalDoesntExist);
			ensure!(
				T::GetTokenOwner::get_owner(&token_id) == Some(origin),
				Error::<T>::NotTokenOwner
			);
			Self::remove_erc721_approval(&token_id);
			Ok(())
		}

		/// Set approval for an account to transfer an amount of tokens on behalf of the caller
		/// Mapping from caller to spender and amount
		/// mapping(address => mapping(address => uint256)) private _allowances;
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn erc20_approval(
			origin: OriginFor<T>,
			caller: T::AccountId,
			spender: T::AccountId,
			asset_id: AssetId,
			amount: Balance,
		) -> DispatchResult {
			let _ = ensure_none(origin)?;
			ensure!(caller != spender, Error::<T>::CallerNotOperator);
			ERC20Approvals::<T>::insert((&caller, asset_id), &spender, amount);
			Ok(())
		}

		/// Removes an approval over an account and asset_id
		/// mapping(address => mapping(address => uint256)) private _allowances;
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn erc20_update_approval(
			origin: OriginFor<T>,
			caller: T::AccountId,
			spender: T::AccountId,
			asset_id: AssetId,
			amount: Balance,
		) -> DispatchResult {
			let _ = ensure_none(origin)?;
			let new_approved_amount = Self::erc20_approvals((&caller, asset_id), &spender)
				.ok_or(Error::<T>::CallerNotApproved)?
				.checked_sub(amount)
				.ok_or(Error::<T>::ApprovedAmountTooLow)?;
			if new_approved_amount.is_zero() {
				ERC20Approvals::<T>::remove((&caller, asset_id), &spender);
			} else {
				ERC20Approvals::<T>::insert((&caller, asset_id), &spender, new_approved_amount);
			}
			Ok(())
		}

		/// Set approval for an account (or contract) to transfer any tokens from a collection
		/// mapping(address => mapping(address => bool)) private _operatorApprovals;
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn erc721_approval_for_all(
			origin: OriginFor<T>,
			caller: T::AccountId,
			operator_account: T::AccountId,
			collection_uuid: CollectionUuid,
			approved: bool,
		) -> DispatchResult {
			let _ = ensure_none(origin)?;
			ensure!(caller != operator_account, Error::<T>::CallerNotOperator);
			if approved {
				ERC721ApprovalsForAll::<T>::insert(
					caller,
					(collection_uuid, operator_account),
					true,
				);
			} else {
				ERC721ApprovalsForAll::<T>::remove(caller, (collection_uuid, operator_account));
			}
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Removes the approval of a single NFT
	/// Triggered by transferring the token
	pub fn remove_erc721_approval(token_id: &TokenId) {
		// Check that origin owns NFT
		ERC721Approvals::<T>::remove(token_id);
	}

	/// Mimics the following Solidity function
	/// https://github.com/OpenZeppelin/openzeppelin-contracts/blob/a1948250ab8c441f6d327a65754cb20d2b1b4554/contracts/token/ERC721/ERC721.sol#L239
	pub fn is_approved_or_owner(token_id: TokenId, spender: T::AccountId) -> bool {
		// Check if spender is owner
		let token_owner = T::GetTokenOwner::get_owner(&token_id);
		if Some(spender.clone()) == token_owner {
			return true;
		}

		// Check approvalForAll
		if let Some(owner) = token_owner {
			if Self::erc721_approvals_for_all(owner, (token_id.0, spender.clone()))
				.unwrap_or_default()
			{
				return true;
			}
		}

		// Lastly, Check token approval
		Some(spender) == Self::erc721_approvals(token_id)
	}
}

impl<T: Config> OnTransferSubscriber for Pallet<T> {
	/// Do anything that needs to be done after an NFT has been transferred
	fn on_nft_transfer(token_id: &TokenId) {
		// Set approval to none
		Self::remove_erc721_approval(token_id);
	}
}
