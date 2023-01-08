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

use codec::Encode;
use frame_support::{ensure, fail, traits::Get, weights::Weight, BoundedVec, PalletId};
pub use pallet::*;
use seed_pallet_common::{EthereumBridge, EthereumEventSubscriber};
use seed_primitives::{CollectionUuid, SerialNumber};
use sp_core::{H160, U256};
use sp_runtime::{traits::AccountIdConversion, DispatchError, SaturatedConversion};
use sp_std::{boxed::Box, vec, vec::Vec};

pub(crate) const LOG_TARGET: &str = "eth-bridge";

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, transactional};
	use frame_support::traits::fungibles::Transfer;
	use frame_system::{ensure_signed, pallet_prelude::*};
	use log::info;
	use seed_pallet_common::ethy::EthySigningRequest;
	use seed_pallet_common::Hold;
	use seed_primitives::{AssetId, Balance};
	use seed_primitives::ethy::EventProofId;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type PalletId: Get<PalletId>;
		/// Bond required for an account to act as relayer
		type RelayerBond: Get<Balance>;
		/// The native token asset Id (managed by pallet-balances)
		type NativeAssetId: Get<AssetId>;
		/// Handles a multi-currency fungible asset system
		type MultiCurrency: Transfer<Self::AccountId> + Hold<AccountId = Self::AccountId>;

	}

	#[pallet::storage]
	#[pallet::getter(fn relayer_paid_bond)]
	/// Map from relayer account to their paid bond amount
	pub type RelayerPaidBond<T: Config> =  StorageMap<_, Twox64Concat, T::AccountId, Balance, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn relayer)]
	/// The permissioned relayer
	pub type Relayer<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;



	#[pallet::error]
	pub enum Error<T> {
		/// The relayer hasn't paid the relayer bond
		NoBondPaid,
		/// The relayer already has a bonded amount
		CantBondRelayer,
		/// The relayer is active and cant unbond the specified amount
		CantUnbondRelayer,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new relayer has been set
		RelayerSet { relayer_account: T::AccountId },
		/// An account has deposited a relayer bond
		RelayerBondDeposited { account_id: T::AccountId, amount: Balance},
		/// An account has withdrawn a relayer bond
		RelayerBondWithdrawn { account_id: T::AccountId, amount: Balance},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> where
		<T as frame_system::Config>::AccountId: From<sp_core::H160> + Into<sp_core::H160>
	{
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		/// Set the relayer address
		pub fn set_relayer(origin: OriginFor<T>, relayer: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;
			// Ensure relayer has bonded more than relayer bond amount
			ensure!(Self::relayer_paid_bond(&relayer) >= T::RelayerBond::get(), Error::<T>::NoBondPaid);
			Relayer::<T>::put(&relayer);
			info!(target: LOG_TARGET, "relayer set. Account Id: {:?}", relayer);
			Self::deposit_event(Event::<T>::RelayerSet{ relayer_account: relayer });
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		/// Submit bond for relayer account
		pub fn deposit_relayer_bond(origin: OriginFor<T>) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			// Ensure relayer doesn't already have a bond set
			ensure!(Self::relayer_paid_bond(&origin) == 0, Error::<T>::CantBondRelayer);

			let relayer_bond = T::RelayerBond::get();
			// Attempt to place a hold from the relayer account
			T::MultiCurrency::place_hold(
				T::PalletId::get(),
				&origin,
				T::NativeAssetId::get(),
				relayer_bond,
			)?;
			RelayerPaidBond::<T>::insert(&origin, relayer_bond);
			Self::deposit_event(Event::<T>::RelayerBondDeposited{ account_id: origin, amount: relayer_bond});
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		/// Withdraw relayer bond amount
		pub fn withdraw_relayer_bond(origin: OriginFor<T>) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			// Ensure account is not the current relayer
			if Self::relayer() == Some(origin.clone()) {
				// spk - check this logic
				ensure!(Self::relayer() != Some(origin.clone()), Error::<T>::CantUnbondRelayer);
			};
			let relayer_paid_bond = Self::relayer_paid_bond(&origin);
			ensure!(relayer_paid_bond > 0, Error::<T>::CantUnbondRelayer);

			// Attempt to release the relayers hold
			T::MultiCurrency::release_hold(
				T::PalletId::get(),
				&origin,
				T::NativeAssetId::get(),
				relayer_paid_bond,
			)?;
			RelayerPaidBond::<T>::remove(&origin);

			Self::deposit_event(Event::<T>::RelayerBondWithdrawn{ account_id: origin, amount: relayer_paid_bond });
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> where <T as frame_system::Config>::AccountId: From<sp_core::H160> {}
