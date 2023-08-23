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

//! # Pallet Echo
//!
//! A simple utility pallet for testing Ethereum bridging.
//! Users can call the ping() extrinsic which will submit an event to Ethereum
//! The pallet will subscribe to EthereumEventSubscriber so it can verify that the ping was received
//! on Ethereum
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{
	pallet_prelude::*,
	sp_runtime::{traits::One, SaturatedConversion},
};
use frame_system::pallet_prelude::*;
use seed_pallet_common::{MaintenanceCheck};
use sp_std::prelude::*;


#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The system event type
		type Event: From<Event> + IsType<<Self as frame_system::Config>::Event>;

		// Interface to access weight values
		// type WeightInfo: WeightInfo;
	}

	/// Maintenance Account
	#[pallet::storage]
	pub type MaintenanceAccount<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// Whether maintenance mode is currently active
	#[pallet::storage]
	pub type MaintenanceModeActive<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event {
		/// Maintenance mode was activated
		MaintenanceModeActivated { active: bool },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// This account is not authorized to execute this transaction
		NotAuthorized,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Enable maintenance mode which prevents all non sudo calls
		/// TODO may need to try call sudo pallet directly
		#[pallet::weight(1000)]
		pub fn set_maintenance_account(origin: OriginFor<T>, new_account: Option<T::AccountId>) -> DispatchResult {
			ensure_root(origin)?;

			match new_account {
				Some(account) => MaintenanceAccount::<T>::put(account),
				None => MaintenanceAccount::<T>::kill()
			}

			Ok(())
		}

		/// Enable maintenance mode which prevents all non sudo calls
		#[pallet::weight(1000)]
		pub fn enable_maintenance_mode(origin: OriginFor<T>, active: bool) -> DispatchResult {
			// Both sudo and the maintenance account can enable maintenance mode
			if ensure_root(origin.clone()).is_err() {
                let who = ensure_signed(origin)?;
				ensure!(Some(who) == MaintenanceAccount::<T>::get(), Error::<T>::NotAuthorized);
            }

			MaintenanceModeActive::<T>::put(active);

			// Deposit runtime event
			Self::deposit_event(Event::MaintenanceModeActivated {
				active,
			});
			Ok(())
		}
	}
}

pub struct MaintenanceChecker<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> MaintenanceCheck<T::AccountId> for MaintenanceChecker<T> {
	fn can_execute(signer: &T::AccountId) -> bool {
		match <MaintenanceModeActive<T>>::get() {
			true => {
				let maintenance_account = MaintenanceAccount::<T>::get();
				match maintenance_account {
					Some(account) => &account == signer,
					None => false
				}
			},
			false => return true
		}
	}
}
