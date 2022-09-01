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
use seed_pallet_common::{IsTokenOwner, OnTransferSubscriber};
use seed_primitives::{AssetId, Balance, TokenId};
use sp_runtime::{traits::Zero, DispatchResult};

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
		type IsTokenOwner: IsTokenOwner<AccountId = Self::AccountId>;
	}

	// Account with transfer approval for a single NFT
	#[pallet::storage]
	#[pallet::getter(fn erc721_approvals)]
	pub type ERC721Approvals<T: Config> = StorageMap<_, Twox64Concat, TokenId, T::AccountId>;

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
		/// The account is not the owner of the token
		NotTokenOwner,
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
		#[pallet::weight(125_000_000)]
		pub fn erc721_approval(
			origin: OriginFor<T>,
			caller: T::AccountId,
			operator_account: T::AccountId,
			token_id: TokenId,
		) -> DispatchResult {
			let _ = ensure_none(origin)?;
			ensure!(caller != operator_account, Error::<T>::CallerNotOperator);
			// Check that origin owns NFT
			ensure!(T::IsTokenOwner::is_owner(&caller, &token_id), Error::<T>::NotTokenOwner);
			ERC721Approvals::<T>::insert(token_id, &operator_account);
			Ok(())
		}

		/// Set approval for an account to transfer an amount of tokens on behalf of the caller
		/// Mapping from caller to spender and amount
		/// mapping(address => mapping(address => uint256)) private _allowances;
		#[pallet::weight(100_000_000)]
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
		#[pallet::weight(100_000_000)]
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

		/// Public method which allows users to remove approvals on a token they own
		#[pallet::weight(100_000_000)]
		pub fn erc721_remove_approval(origin: OriginFor<T>, token_id: TokenId) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			ensure!(ERC721Approvals::<T>::contains_key(token_id), Error::<T>::ApprovalDoesntExist);
			ensure!(T::IsTokenOwner::is_owner(&origin, &token_id), Error::<T>::NotTokenOwner);
			Self::remove_erc721_approval(&token_id);
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
}

impl<T: Config> OnTransferSubscriber for Pallet<T> {
	/// Do anything that needs to be done after an NFT has been transferred
	fn on_nft_transfer(token_id: &TokenId) {
		// Set approval to none
		Self::remove_erc721_approval(token_id);
	}
}
