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

//! Maintains accrued era transaction fees in conjunction with pallet-staking and
//! pallet-transaction-payment
//! The root network stakers are paid out in network tx fees XRP (non-inflationary)
use frame_support::{
	traits::{
		fungible::Inspect, Currency, ExistenceRequirement, Get, Imbalance, OnUnbalanced,
		WithdrawReasons,
	},
	PalletId,
};
use sp_runtime::traits::{AccountIdConversion, Zero};

use seed_pallet_common::log;
use seed_primitives::Balance;

pub use pallet::*;

/// The logging target for this pallet
pub(crate) const LOG_TARGET: &str = "tx-fee-pot";

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::{ValueQuery, *};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_assets_ext::Config {
		type FeeCurrency: Currency<
			Self::AccountId,
			Balance = Balance,
			NegativeImbalance = pallet_assets_ext::NegativeImbalance<Self>,
		>;
		type StakeCurrency: Currency<
			Self::AccountId,
			Balance = Balance,
			NegativeImbalance = pallet_balances::NegativeImbalance<Self>,
		>;
		#[pallet::constant]
		type TxFeePotId: Get<PalletId>;
	}

	/// Accrued transaction fees in the current staking Era
	#[pallet::storage]
	pub type EraTxFees<T> = StorageValue<_, Balance, ValueQuery>;
}

impl<T: Config> Pallet<T> {
	/// Note some `amount` of tx fees were accrued
	pub fn accrue_era_tx_fees(amount: Balance) {
		EraTxFees::<T>::mutate(|total| *total += amount);
	}
	/// Resets the era tx fees to 0 and returns the last amount
	pub fn reset_era_tx_fees() -> Balance {
		EraTxFees::<T>::take()
	}
	/// Get the tx fee pot balance (current era)
	pub fn era_pot_balance() -> Balance {
		EraTxFees::<T>::get()
	}
	/// Get the tx fee pot balance (all eras - any claimed amounts)
	pub fn total_pot_balance() -> Balance {
		pallet_balances::Pallet::<T>::balance(&Self::account_id())
	}
	/// Get the tx fee pot account Id
	pub fn account_id() -> T::AccountId {
		T::TxFeePotId::get().into_account_truncating()
	}
}

/// Alias for pallet-assets-ext NegativeImbalance
type FeeNegativeImbalanceOf<T> = pallet_assets_ext::NegativeImbalance<T>;
/// Alias for pallet-assets-ext PositiveImbalance
type FeePositiveImbalanceOf<T> = pallet_assets_ext::PositiveImbalance<T>;
/// Alias for pallet-balances NegativeImbalance
type StakeNegativeImbalanceOf<T> = pallet_balances::NegativeImbalance<T>;
/// Alias for pallet-balances PositiveImbalance
type StakePositiveImbalanceOf<T> = pallet_balances::PositiveImbalance<T>;

// In our current implementation we have filtered the payout_stakers call so this will never
// be triggered. We have decided to keep the TxFeePot in the case this is overlooked
// to prevent unwanted changes in Root token issuance
impl<T: Config> OnUnbalanced<FeePositiveImbalanceOf<T>> for Pallet<T> {
	fn on_nonzero_unbalanced(total_rewarded: FeePositiveImbalanceOf<T>) {
		// burn `amount` from TxFeePot, reducing total issuance immediately
		// later `total_rewarded` will be dropped keeping total issuance constant
		if let Err(_err) = T::FeeCurrency::withdraw(
			&Self::account_id(),
			total_rewarded.peek(),
			WithdrawReasons::all(),
			ExistenceRequirement::AllowDeath,
		) {
			// tx fee pot did not have enough to reward the amount, this should not happen...
			// there's no way to error out here, just log it
			log!(
				error,
				"💸 era payout was underfunded, please open an issue at https://github.com/futureversecom/seed: {:?}",
				total_rewarded.peek()
			)
		}
	}
}

/// On tx fee settlement, move funds to tx fee pot address.
/// Pallets can call this to signify a "burn" from their perspective.
/// EVM tx base fees are also accrued here - via providing impl for EVMCurrencyAdapter
impl<T: Config> OnUnbalanced<FeeNegativeImbalanceOf<T>> for Pallet<T> {
	fn on_nonzero_unbalanced(amount: FeeNegativeImbalanceOf<T>) {
		// this amount was burnt from caller when tx fees were paid (incl. tip), move the funds into
		// the pot
		let note_amount = amount.peek();
		let _ = T::FeeCurrency::deposit_creating(&Self::account_id(), note_amount);

		Self::accrue_era_tx_fees(note_amount);
	}
}

// In our current implementation we have filtered the payout_stakers call so this will never
// be triggered. We have decided to keep the TxFeePot in the case this is overlooked
// to prevent unwanted changes in Root token issuance
impl<T: Config> OnUnbalanced<StakePositiveImbalanceOf<T>> for Pallet<T> {
	fn on_nonzero_unbalanced(total_rewarded: StakePositiveImbalanceOf<T>) {
		// burn `amount` from TxFeePot, reducing total issuance immediately
		// later `total_rewarded` will be dropped keeping total issuance constant
		if let Err(_err) = T::StakeCurrency::withdraw(
			&Self::account_id(),
			total_rewarded.peek(),
			WithdrawReasons::all(),
			ExistenceRequirement::AllowDeath,
		) {
			// tx fee pot did not have enough to reward the amount, this should not happen...
			// there's no way to error out here, just log it
			log!(
				error,
				"💸 era payout was underfunded, please open an issue at https://github.com/futureversecom/seed: {:?}",
				total_rewarded.peek()
			)
		}
	}
}

/// On era payout remainder
/// Not currently used, see note above. This also does not affect any local storage of tx_fees
/// within the TXFeePot pallet, simply deposits into the account
/// staking pallet calls this to notify it has `amount` left over after reward payments
impl<T: Config> OnUnbalanced<StakeNegativeImbalanceOf<T>> for Pallet<T> {
	fn on_nonzero_unbalanced(amount: StakeNegativeImbalanceOf<T>) {
		let note_amount = amount.peek();

		// mint `note_amount` (offsets `amount` imbalance)
		let _ = T::StakeCurrency::deposit_creating(&Self::account_id(), note_amount);
	}
}

impl<T: Config> pallet_staking::EraPayout<Balance> for Pallet<T> {
	/// Determine the payout for this era.
	///
	/// Returns the amount to be paid to stakers in this era, as well as any indivisible remainder
	fn era_payout(
		_total_staked: Balance,
		_total_issuance: Balance,
		_era_duration_millis: u64,
	) -> (Balance, Balance) {
		// this trait is coupled to the idea of polkadot's inflation schedule
		// on root network we simply redistribute the era's tx fees

		// reset the era fee storage, the era payout will be tracked by pallet-staking
		(Self::reset_era_tx_fees(), Zero::zero())
	}
}
