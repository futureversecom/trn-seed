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

//! # Pallet Maintenance Mode
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{
	dispatch::{CallMetadata, Dispatchable, GetCallMetadata},
	pallet_prelude::*,
	sp_runtime::{traits::One, SaturatedConversion},
	traits::IsSubType,
	weights::{GetDispatchInfo, PostDispatchInfo},
};
use frame_system::pallet_prelude::*;
use seed_pallet_common::MaintenanceCheck;
use sp_std::prelude::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching call type.
		type Call: Parameter
			+ Dispatchable<Origin = Self::Origin, PostInfo = PostDispatchInfo>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>;

		/// The system event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		// Interface to access weight values
		// type WeightInfo: WeightInfo;
	}

	/// Whether maintenance mode is currently active
	#[pallet::storage]
	pub type MaintenanceModeActive<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Map from account to blocked status
	#[pallet::storage]
	pub type BlockedAccounts<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, bool>;

	/// Map from call to blocked status
	/// map (PalletNameBytes, FunctionNameBytes) => bool
	#[pallet::storage]
	#[pallet::getter(fn paused_transactions)]
	pub type BlockedCalls<T: Config> = StorageMap<_, Twox64Concat, (Vec<u8>, Vec<u8>), bool>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Maintenance mode was activated
		MaintenanceModeActivated { active: bool },
		/// An account was blocked
		AccountBlocked { account: T::AccountId, blocked: bool },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// This account is not authorized to execute this transaction
		NotAuthorized,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Enable maintenance mode which prevents all non sudo calls
		#[pallet::weight(1000)]
		pub fn enable_maintenance_mode(origin: OriginFor<T>, active: bool) -> DispatchResult {
			ensure_root(origin)?;

			MaintenanceModeActive::<T>::put(active);

			Self::deposit_event(Event::MaintenanceModeActivated { active });
			Ok(())
		}

		/// Blocks an account from transacting on the network
		#[pallet::weight(1000)]
		pub fn block_account(
			origin: OriginFor<T>,
			account: T::AccountId,
			blocked: bool,
		) -> DispatchResult {
			ensure_root(origin)?;

			match blocked {
				true => BlockedAccounts::<T>::insert(&account, true),
				false => BlockedAccounts::<T>::remove(&account),
			}

			Self::deposit_event(Event::AccountBlocked { account, blocked });

			Ok(())
		}

		/// Blocks a call from being executed
		#[pallet::weight(1000)]
		pub fn block_call(
			origin: OriginFor<T>,
			pallet_name: Vec<u8>,
			function_name: Vec<u8>,
			blocked: bool,
		) -> DispatchResult {
			ensure_root(origin)?;

			// not allowed to pause calls of this pallet to ensure safe
			// let pallet_name_string = sp_std::str::from_utf8(&pallet_name).map_err(|_|
			// Error::<T>::InvalidCharacter)?; ensure!(
			// 	pallet_name_string != <Self as PalletInfoAccess>::name(),
			// 	Error::<T>::CannotPause
			// );

			let pallet_name = pallet_name.to_ascii_lowercase();
			let function_name = function_name.to_ascii_lowercase();
			match blocked {
				true => BlockedCalls::<T>::insert((pallet_name, function_name), true),
				false => BlockedCalls::<T>::remove((pallet_name, function_name)),
			}
			// println!("{:?}", call.metadata());
			// match blocked {
			// 	true => BlockedCalls::<T>::insert(&call, true),
			// 	false => BlockedCalls::<T>::remove(&call),
			// }

			Ok(())
		}
	}
}

pub struct MaintenanceChecker<T>(sp_std::marker::PhantomData<T>);

impl<T: frame_system::Config + Config> MaintenanceCheck<T> for MaintenanceChecker<T>
where
	<T as frame_system::Config>::Call: GetCallMetadata,
{
	fn can_execute(
		signer: &<T as frame_system::Config>::AccountId,
		call: &<T as frame_system::Config>::Call,
	) -> bool {
		let pallet_name = call.get_call_metadata().pallet_name;

		// Check whether this is a sudo call, we want to enable all sudo calls
		// Regardless of maintenance mode
		// This check is needed here in case we accidentally block the sudo account
		if pallet_name == "Sudo" {
			return true
		}

		// Check if we are in maintenance mode
		if <MaintenanceModeActive<T>>::get() {
			return false
		}

		return !BlockedAccounts::<T>::contains_key(signer)
	}

	fn call_paused(call: &<T as frame_system::Config>::Call) -> bool {
		let CallMetadata { function_name, pallet_name } = call.get_call_metadata();

		// Check whether this is a sudo call, we want to enable all sudo calls
		// Regardless of maintenance mode
		if pallet_name == "Sudo" {
			return false
		}

		// Check whether call is blocked
		BlockedCalls::<T>::contains_key((
			pallet_name.to_ascii_lowercase().as_bytes(),
			function_name.to_ascii_lowercase().as_bytes(),
		))
	}
}
