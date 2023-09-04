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
	weights::{GetDispatchInfo, PostDispatchInfo},
};
use frame_system::pallet_prelude::*;
use seed_pallet_common::{MaintenanceCheck, MaintenanceCheckEVM};
use sp_core::H160;
use sp_runtime::traits::{DispatchInfoOf, SignedExtension};
use sp_std::{fmt::Debug, prelude::*, vec::Vec};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;

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

	/// Map from EVM target address to blocked status
	#[pallet::storage]
	pub type BlockedEVMAddresses<T: Config> = StorageMap<_, Twox64Concat, H160, bool>;

	/// Map from call to blocked status
	/// map (PalletNameBytes, FunctionNameBytes) => bool
	#[pallet::storage]
	pub type BlockedCalls<T: Config> = StorageMap<_, Twox64Concat, (Vec<u8>, Vec<u8>), bool>;

	/// Map from pallet to blocked status
	/// map PalletNameBytes => bool
	#[pallet::storage]
	pub type BlockedPallets<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, bool>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Maintenance mode was activated
		MaintenanceModeActivated { enabled: bool },
		/// An account was blocked
		AccountBlocked { account: T::AccountId, blocked: bool },
		/// An account was blocked
		EVMTargetBlocked { target_address: H160, blocked: bool },
		/// An account was blocked
		CallBlocked { pallet_name: Vec<u8>, call_name: Vec<u8>, blocked: bool },
		/// An account was blocked
		PalletBlocked { pallet_name: Vec<u8>, blocked: bool },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// This account is not authorized to execute this transaction
		AccountBlocked,
		/// This call is disabled as the chain is in maintenance mode
		MaintenanceModeActive,
		/// The pallet name is not valid utf-8 characters
		InvalidPalletName,
		/// The call name is not valid utf-8 characters
		InvalidCallName,
		/// This pallet or call cannot be blocked
		CannotBlock,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Enable maintenance mode which prevents all non sudo calls
		#[pallet::weight(1000)]
		pub fn enable_maintenance_mode(origin: OriginFor<T>, enabled: bool) -> DispatchResult {
			ensure_root(origin)?;

			MaintenanceModeActive::<T>::put(enabled);

			Self::deposit_event(Event::MaintenanceModeActivated { enabled });
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

		/// Blocks an account from transacting on the network
		#[pallet::weight(1000)]
		pub fn block_evm_target(
			origin: OriginFor<T>,
			target_address: H160,
			blocked: bool,
		) -> DispatchResult {
			ensure_root(origin)?;

			match blocked {
				true => BlockedEVMAddresses::<T>::insert(&target_address, true),
				false => BlockedEVMAddresses::<T>::remove(&target_address),
			}

			Self::deposit_event(Event::EVMTargetBlocked { target_address, blocked });

			Ok(())
		}

		/// Blocks a call from being executed
		#[pallet::weight(1000)]
		pub fn block_call(
			origin: OriginFor<T>,
			pallet_name: Vec<u8>,
			call_name: Vec<u8>,
			blocked: bool,
		) -> DispatchResult {
			ensure_root(origin)?;

			// Validate pallet name
			ensure!(!pallet_name.is_empty(), Error::<T>::InvalidPalletName);
			let pallet_name = pallet_name.to_ascii_lowercase();
			let pallet_name_string =
				core::str::from_utf8(&pallet_name).map_err(|_| Error::<T>::InvalidPalletName)?;
			// Ensure this pallet cannot be blocked
			ensure!(
				pallet_name_string != <Self as PalletInfoAccess>::name().to_ascii_lowercase(),
				Error::<T>::CannotBlock
			);

			// Validate call name
			ensure!(!call_name.is_empty(), Error::<T>::InvalidCallName);
			let call_name = call_name.to_ascii_lowercase();
			let _ = core::str::from_utf8(&call_name).map_err(|_| Error::<T>::InvalidCallName)?;

			match blocked {
				true => BlockedCalls::<T>::insert((&pallet_name, &call_name), true),
				false => BlockedCalls::<T>::remove((&pallet_name, &call_name)),
			}

			Self::deposit_event(Event::CallBlocked { pallet_name, call_name, blocked });

			Ok(())
		}

		/// Blocks an entire pallets calls from being executed
		#[pallet::weight(1000)]
		pub fn block_pallet(
			origin: OriginFor<T>,
			pallet_name: Vec<u8>,
			blocked: bool,
		) -> DispatchResult {
			ensure_root(origin)?;

			// Validate pallet name
			ensure!(!pallet_name.is_empty(), Error::<T>::InvalidPalletName);
			let pallet_name = pallet_name.to_ascii_lowercase();
			let pallet_name_string =
				core::str::from_utf8(&pallet_name).map_err(|_| Error::<T>::InvalidPalletName)?;
			// Ensure this pallet cannot be blocked
			ensure!(
				pallet_name_string != <Self as PalletInfoAccess>::name().to_ascii_lowercase(),
				Error::<T>::CannotBlock
			);

			match blocked {
				true => BlockedPallets::<T>::insert(&pallet_name, true),
				false => BlockedPallets::<T>::remove(&pallet_name),
			}

			Self::deposit_event(Event::PalletBlocked { pallet_name, blocked });

			Ok(())
		}
	}
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct MaintenanceChecker<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> MaintenanceChecker<T> {
	pub fn new() -> Self {
		Self(Default::default())
	}
}

impl<T: frame_system::Config + Config> MaintenanceCheck<T> for MaintenanceChecker<T>
where
	<T as frame_system::Config>::Call: GetCallMetadata,
{
	fn call_paused(call: &<T as frame_system::Config>::Call) -> bool {
		let CallMetadata { function_name, pallet_name } = call.get_call_metadata();

		// Check whether this is a sudo call, we want to enable all sudo calls
		// Regardless of maintenance mode
		if pallet_name == "Sudo" {
			return false
		}

		let pallet_name = pallet_name.to_ascii_lowercase();
		let function_name = function_name.to_ascii_lowercase();

		// Check whether call is blocked
		if BlockedCalls::<T>::contains_key((pallet_name.as_bytes(), function_name.as_bytes())) {
			return true
		}

		// Check whether pallet is blocked
		if BlockedPallets::<T>::contains_key(pallet_name.as_bytes()) {
			return true
		}

		return false
	}
}

impl<T: frame_system::Config + Config> MaintenanceCheckEVM<T> for MaintenanceChecker<T> {
	fn validate_evm_transaction(
		signer: &<T as frame_system::Config>::AccountId,
		target: &H160,
	) -> bool {
		// Check if we are in maintenance mode
		if MaintenanceModeActive::<T>::get() {
			return false
		}

		if BlockedEVMAddresses::<T>::contains_key(target) {
			return false
		}

		if BlockedAccounts::<T>::contains_key(signer) {
			return false
		}
		return true
	}
}

impl<T: Config + Send + Sync + Debug> SignedExtension for MaintenanceChecker<T>
where
	<T as Config>::Call: GetCallMetadata,
{
	const IDENTIFIER: &'static str = "CheckMaintenanceMode";
	type AccountId = T::AccountId;
	type Call = <T as Config>::Call;
	type AdditionalSigned = ();
	type Pre = ();

	fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> {
		Ok(())
	}

	fn validate(
		&self,
		who: &Self::AccountId,
		call: &Self::Call,
		_info: &DispatchInfoOf<Self::Call>,
		_len: usize,
	) -> TransactionValidity {
		let pallet_name = call.get_call_metadata().pallet_name;

		// Check whether this is a sudo call, we want to enable all sudo calls
		// Regardless of maintenance mode
		// This check is needed here in case we accidentally block the sudo account
		if pallet_name == "Sudo" {
			return Ok(ValidTransaction::default())
		}

		// Check if we are in maintenance mode
		if <MaintenanceModeActive<T>>::get() {
			return Err(TransactionValidityError::Invalid(InvalidTransaction::Custom(1)))
		}

		if BlockedAccounts::<T>::contains_key(who) {
			return Err(TransactionValidityError::Invalid(InvalidTransaction::Custom(1)))
		}

		Ok(ValidTransaction::default())
	}

	fn pre_dispatch(
		self,
		who: &Self::AccountId,
		call: &Self::Call,
		info: &DispatchInfoOf<Self::Call>,
		len: usize,
	) -> Result<Self::Pre, TransactionValidityError> {
		self.validate(who, call, info, len).map(|_| ())
	}
}
