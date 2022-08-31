#![cfg_attr(not(feature = "std"), no_std)]

//! Maintains accrued era transaction fees in conjunction with pallet-staking and
//! pallet-transaction-payment
//! The root network stakers are paid out in network tx fees XRP (non-inflationary)
use frame_support::{
	traits::{
		fungible::{Inspect, Mutate},
		Currency, Get, Imbalance, OnUnbalanced,
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
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: pallet_balances::Config<Balance = Balance> {
		#[pallet::constant]
		type TxFeePotId: Get<PalletId>;
	}

	/// Accrued transaction fees in the current staking Era
	#[pallet::storage]
	#[pallet::getter(fn era_tx_fees)]
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
		Self::era_tx_fees()
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

/// Alias for pallet-balances NegativeImbalance
type NegativeImbalanceOf<T> = pallet_balances::NegativeImbalance<T>;
/// Alias for pallet-balances PositiveImbalance
type PositiveImbalanceOf<T> = pallet_balances::PositiveImbalance<T>;

/// Handles imbalances of transaction fee amounts for the transaction fee pot, used to payout
/// stakers

/// On era reward payouts, offset minted tokens from the tx fee pot to maintain total issuance
impl<T: Config> OnUnbalanced<PositiveImbalanceOf<T>> for Pallet<T> {
	fn on_nonzero_unbalanced(total_rewarded: PositiveImbalanceOf<T>) {
		// burn `amount` from TxFeePot, reducing total issuance immediately
		// later `total_rewarded` will be dropped keeping total issuance constant
		if let Err(_err) =
			pallet_balances::Pallet::<T>::burn_from(&Self::account_id(), total_rewarded.peek())
		{
			// tx fee pot did not have enough to reward the amount, this should not happen...
			// there's no way to error out here, just log it
			log!(error, "💸 era payout was underfunded, please open an issue at https://github.com/futureversecom/seed: {:?}", total_rewarded.peek())
		}
	}
}

/// On tx fee settlement, move funds to tx fee pot address
impl<T: Config> OnUnbalanced<NegativeImbalanceOf<T>> for Pallet<T> {
	fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<T>) {
		// this amount was burnt (`withdraw`) when tx fees were paid (incl. tip)
		let note_amount = amount.peek();
		pallet_balances::Pallet::resolve_creating(&Self::account_id(), amount);
		Self::accrue_era_tx_fees(note_amount);
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
		(Self::reset_era_tx_fees(), Zero::zero())
	}
}
