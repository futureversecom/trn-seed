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
use seed_pallet_common::{NFTExt, OnTransferSubscriber};
use seed_primitives::{AssetId, Balance, CollectionUuid, TokenId};
use sp_runtime::{traits::Zero, DispatchResult};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub use pallet::*;

mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: frame_system::Config {
		/// NFT ownership interface
		type NFTExt: NFTExt<AccountId = Self::AccountId>;
		/// Provides weights info
		type WeightInfo: WeightInfo;
	}

	// Account with transfer approval for a single NFT
	#[pallet::storage]
	pub type ERC721Approvals<T: Config> = StorageMap<_, Twox64Concat, TokenId, T::AccountId>;

	// Accounts with transfer approval for an NFT collection of another account
	#[pallet::storage]
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
	pub type ERC20Approvals<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		(T::AccountId, AssetId),
		Twox64Concat,
		T::AccountId,
		Balance,
	>;

	// Accounts with transfer approval for an SFT collection of another account
	#[pallet::storage]
	pub type ERC1155ApprovalsForAll<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		(CollectionUuid, T::AccountId),
		bool,
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

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set approval for a single NFT
		/// Mapping from token_id to operator
		/// clears approval on transfer
		/// mapping(uint256 => address) private _tokenApprovals;
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::erc721_approval())]
		pub fn erc721_approval(
			origin: OriginFor<T>,
			caller: T::AccountId,
			operator_account: T::AccountId,
			token_id: TokenId,
		) -> DispatchResult {
			ensure_none(origin)?;
			ensure!(caller != operator_account, Error::<T>::CallerNotOperator);
			// Check that origin owns NFT or is approved_for_all
			let token_owner = match T::NFTExt::get_token_owner(&token_id) {
				Some(owner) => owner,
				None => return Err(Error::<T>::NoToken.into()),
			};

			let is_approved_for_all =
				ERC721ApprovalsForAll::<T>::get(&token_owner, (token_id.0, caller.clone()))
					.unwrap_or_default();
			ensure!(
				token_owner == caller || is_approved_for_all,
				Error::<T>::NotTokenOwnerOrApproved
			);
			ERC721Approvals::<T>::insert(token_id, &operator_account);
			Ok(())
		}

		/// Public method which allows users to remove approvals on a token they own
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::erc721_remove_approval())]
		pub fn erc721_remove_approval(origin: OriginFor<T>, token_id: TokenId) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			ensure!(ERC721Approvals::<T>::contains_key(token_id), Error::<T>::ApprovalDoesntExist);
			ensure!(
				T::NFTExt::get_token_owner(&token_id) == Some(origin),
				Error::<T>::NotTokenOwner
			);
			Self::remove_erc721_approval(&token_id);
			Ok(())
		}

		/// Set approval for an account to transfer an amount of tokens on behalf of the caller
		/// Mapping from caller to spender and amount
		/// mapping(address => mapping(address => uint256)) private _allowances;
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::erc20_approval())]
		pub fn erc20_approval(
			origin: OriginFor<T>,
			caller: T::AccountId,
			spender: T::AccountId,
			asset_id: AssetId,
			amount: Balance,
		) -> DispatchResult {
			ensure_none(origin)?;
			ensure!(caller != spender, Error::<T>::CallerNotOperator);
			ERC20Approvals::<T>::insert((&caller, asset_id), &spender, amount);
			Ok(())
		}

		/// Removes an approval over an account and asset_id
		/// mapping(address => mapping(address => uint256)) private _allowances;
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::erc20_update_approval())]
		pub fn erc20_update_approval(
			origin: OriginFor<T>,
			caller: T::AccountId,
			spender: T::AccountId,
			asset_id: AssetId,
			amount: Balance,
		) -> DispatchResult {
			ensure_none(origin)?;
			let new_approved_amount = ERC20Approvals::<T>::get((&caller, asset_id), &spender)
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
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::erc721_approval_for_all())]
		pub fn erc721_approval_for_all(
			origin: OriginFor<T>,
			caller: T::AccountId,
			operator_account: T::AccountId,
			collection_uuid: CollectionUuid,
			approved: bool,
		) -> DispatchResult {
			ensure_none(origin)?;
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

		/// Set approval for an account (or contract) to transfer any tokens from an SFT collection
		/// mapping(address => mapping(address => bool)) private _operatorApprovals;
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::erc1155_approval_for_all())]
		pub fn erc1155_approval_for_all(
			origin: OriginFor<T>,
			caller: T::AccountId,
			operator_account: T::AccountId,
			collection_uuid: CollectionUuid,
			approved: bool,
		) -> DispatchResult {
			ensure_none(origin)?;
			ensure!(caller != operator_account, Error::<T>::CallerNotOperator);
			if approved {
				ERC1155ApprovalsForAll::<T>::insert(
					caller,
					(collection_uuid, operator_account),
					true,
				);
			} else {
				ERC1155ApprovalsForAll::<T>::remove(caller, (collection_uuid, operator_account));
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
		let token_owner = T::NFTExt::get_token_owner(&token_id);
		if Some(spender.clone()) == token_owner {
			return true;
		}

		// Check approvalForAll
		if let Some(owner) = token_owner {
			if ERC721ApprovalsForAll::<T>::get(owner, (token_id.0, spender.clone()))
				.unwrap_or_default()
			{
				return true;
			}
		}

		// Lastly, Check token approval
		Some(spender) == ERC721Approvals::<T>::get(token_id)
	}
}

impl<T: Config> OnTransferSubscriber for Pallet<T> {
	/// Do anything that needs to be done after an NFT has been transferred
	fn on_nft_transfer(token_id: &TokenId) {
		// Set approval to none
		Self::remove_erc721_approval(token_id);
	}
}
