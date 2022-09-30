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
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_assets_ext::Config {
		type FeeCurrency: Currency<
			Self::AccountId,
			Balance = Balance,
			NegativeImbalance = pallet_assets_ext::NegativeImbalance<Self>,
		>;
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

/// Alias for pallet-assets-ext NegativeImbalance
type FeeNegativeImbalanceOf<T> = pallet_assets_ext::NegativeImbalance<T>;
/// Alias for pallet-balances PositiveImbalance
type FeePositiveImbalanceOf<T> = pallet_assets_ext::PositiveImbalance<T>;
/// Alias for pallet-balances NegativeImbalance
type StakeNegativeImbalanceOf<T> = pallet_balances::NegativeImbalance<T>;

/// Handles imbalances of transaction fee amounts for the transaction fee pot, used to payout
/// stakers

/// On era reward payouts, offset minted tokens from the tx fee pot to maintain total issuance
/// staking pallet calls this to notify it minted `total_rewarded`
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
			log!(error, "💸 era payout was underfunded, please open an issue at https://github.com/futureversecom/seed: {:?}", total_rewarded.peek())
		}
	}
}

/// On tx fee settlement, move funds to tx fee pot address
/// tx payment pallet calls this to notify it burned `amount`
impl<T: Config> OnUnbalanced<FeeNegativeImbalanceOf<T>> for Pallet<T> {
	fn on_nonzero_unbalanced(amount: FeeNegativeImbalanceOf<T>) {
		// this amount was burnt from caller when tx fees were paid (incl. tip), move the funds into
		// the pot
		let note_amount = amount.peek();
		T::FeeCurrency::deposit_creating(&Self::account_id(), note_amount);

		Self::accrue_era_tx_fees(note_amount);
	}
}

/// On era payout remainder
/// staking pallet calls this to notify it has `amount` left over after reward payments
impl<T: Config> OnUnbalanced<StakeNegativeImbalanceOf<T>> for Pallet<T> {
	fn on_nonzero_unbalanced(amount: StakeNegativeImbalanceOf<T>) {
		let note_amount = amount.peek();

		// mint `note_amount` (offsets `amount` imbalance)
		T::FeeCurrency::deposit_creating(&Self::account_id(), note_amount);

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

		// reset the era fee storage, the era payout will be tracked by pallet-staking
		(Self::reset_era_tx_fees(), Zero::zero())
	}
}
