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
//! This pallet allows us to pause the chain entirely by enabling maintenance mode or by
//! restricting certain accounts, calls, EVM targets, or pallets from being executed.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{
	dispatch::{CallMetadata, Dispatchable, GetCallMetadata, GetDispatchInfo, PostDispatchInfo},
	pallet_prelude::*,
};
use frame_system::pallet_prelude::*;
use seed_pallet_common::{MaintenanceCheck, MaintenanceCheckEVM};
use sp_core::H160;
use sp_runtime::traits::{DispatchInfoOf, SignedExtension};
use sp_std::{fmt::Debug, prelude::*};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;

mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching call type.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin, PostInfo = PostDispatchInfo>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>;

		/// The overarching event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The maximum length of a pallet or call name, stored on-chain
		#[pallet::constant]
		type StringLimit: Get<u32>;

		// Interface to access weight values
		type WeightInfo: WeightInfo;

		// The sudo pallet to prevent blocking of sudo calls
		type SudoPallet: PalletInfoAccess;

		// The sudo pallet to prevent blocking of timestamp calls
		type TimestampPallet: PalletInfoAccess;

		// The ImOnline pallet to prevent blocking of imOnline calls
		type ImOnlinePallet: PalletInfoAccess;

		// The Ethy pallet to prevent blocking of ethy calls
		type EthyPallet: PalletInfoAccess;

		// The following pallets are used to prevent blocking of calls that are required for governance
		type DemocracyPallet: PalletInfoAccess;
		type PreimagePallet: PalletInfoAccess;
		type CouncilPallet: PalletInfoAccess;
		type SchedulerPallet: PalletInfoAccess;
	}

	/// Determines whether maintenance mode is currently active
	#[pallet::storage]
	pub type MaintenanceModeActive<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Map from account to blocked status
	#[pallet::storage]
	pub type BlockedAccounts<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, bool, ValueQuery>;

	/// Map from EVM target address to blocked status
	#[pallet::storage]
	pub type BlockedEVMAddresses<T: Config> = StorageMap<_, Twox64Concat, H160, bool, ValueQuery>;

	/// Map from call to blocked status
	/// map (PalletNameBytes, FunctionNameBytes) => bool
	#[pallet::storage]
	pub type BlockedCalls<T: Config> = StorageMap<
		_,
		Twox64Concat,
		(BoundedVec<u8, T::StringLimit>, BoundedVec<u8, T::StringLimit>),
		bool,
		ValueQuery,
	>;

	/// Map from pallet to blocked status
	/// map PalletNameBytes => bool
	#[pallet::storage]
	pub type BlockedPallets<T: Config> =
		StorageMap<_, Twox64Concat, BoundedVec<u8, T::StringLimit>, bool, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Maintenance mode was activated
		MaintenanceModeActivated { enabled: bool },
		/// An account was blocked
		AccountBlocked { account: T::AccountId, blocked: bool },
		/// An account was blocked
		EVMTargetBlocked { target_address: H160, blocked: bool },
		/// A Runtime Call was blocked
		CallBlocked {
			pallet_name: BoundedVec<u8, T::StringLimit>,
			call_name: BoundedVec<u8, T::StringLimit>,
			blocked: bool,
		},
		/// A Pallet was blocked
		PalletBlocked { pallet_name: BoundedVec<u8, T::StringLimit>, blocked: bool },
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
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::enable_maintenance_mode())]
		pub fn enable_maintenance_mode(origin: OriginFor<T>, enabled: bool) -> DispatchResult {
			ensure_root(origin)?;

			MaintenanceModeActive::<T>::put(enabled);

			Self::deposit_event(Event::MaintenanceModeActivated { enabled });
			Ok(())
		}

		/// Blocks an account from transacting on the network
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::block_account())]
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
		/// Can be used to block individual precompile addresses or contracts
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::block_evm_target())]
		pub fn block_evm_target(
			origin: OriginFor<T>,
			target_address: H160,
			blocked: bool,
		) -> DispatchResult {
			ensure_root(origin)?;

			match blocked {
				true => BlockedEVMAddresses::<T>::insert(target_address, true),
				false => BlockedEVMAddresses::<T>::remove(target_address),
			}

			Self::deposit_event(Event::EVMTargetBlocked { target_address, blocked });

			Ok(())
		}

		/// Blocks a call from being executed
		/// pallet_name: The name of the pallet as per the runtime file. i.e. FeeProxy
		/// call_name: The snake_case name for the call. i.e. set_fee
		/// Both pallet and call names are not case sensitive
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::block_call())]
		pub fn block_call(
			origin: OriginFor<T>,
			pallet_name: BoundedVec<u8, T::StringLimit>,
			call_name: BoundedVec<u8, T::StringLimit>,
			blocked: bool,
		) -> DispatchResult {
			ensure_root(origin)?;

			// Validate pallet name
			ensure!(!pallet_name.is_empty(), Error::<T>::InvalidPalletName);
			let pallet_name = BoundedVec::truncate_from(pallet_name.to_ascii_lowercase());
			let pallet_name_string =
				core::str::from_utf8(&pallet_name).map_err(|_| Error::<T>::InvalidPalletName)?;
			ensure!(Self::is_pallet_blockable(pallet_name_string), Error::<T>::CannotBlock);

			// Validate call name
			ensure!(!call_name.is_empty(), Error::<T>::InvalidCallName);
			let call_name = BoundedVec::truncate_from(call_name.to_ascii_lowercase());
			let _ = core::str::from_utf8(&call_name).map_err(|_| Error::<T>::InvalidCallName)?;

			match blocked {
				true => BlockedCalls::<T>::insert((&pallet_name, &call_name), true),
				false => BlockedCalls::<T>::remove((&pallet_name, &call_name)),
			}

			Self::deposit_event(Event::CallBlocked { pallet_name, call_name, blocked });

			Ok(())
		}

		/// Blocks an entire pallets calls from being executed
		/// pallet_name: The name of the pallet as per the runtime file. i.e. FeeProxy
		/// Pallet names are not case sensitive
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::block_pallet())]
		pub fn block_pallet(
			origin: OriginFor<T>,
			pallet_name: BoundedVec<u8, T::StringLimit>,
			blocked: bool,
		) -> DispatchResult {
			ensure_root(origin)?;

			// Validate pallet name
			ensure!(!pallet_name.is_empty(), Error::<T>::InvalidPalletName);
			let pallet_name = BoundedVec::truncate_from(pallet_name.to_ascii_lowercase());
			let pallet_name_string =
				core::str::from_utf8(&pallet_name).map_err(|_| Error::<T>::InvalidPalletName)?;
			ensure!(Self::is_pallet_blockable(pallet_name_string), Error::<T>::CannotBlock);

			match blocked {
				true => BlockedPallets::<T>::insert(&pallet_name, true),
				false => BlockedPallets::<T>::remove(&pallet_name),
			}

			Self::deposit_event(Event::PalletBlocked { pallet_name, blocked });

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Checks whether a certain pallet can be blocked. Pallets that cannot be blocked are
	/// defined in the Config individually. This is to prevent the chain from being blocked
	/// These will be checked in the call filter to allow these pallet calls to be executed
	fn is_pallet_blockable(pallet_name: &str) -> bool {
		if pallet_name == <Self as PalletInfoAccess>::name().to_ascii_lowercase() {
			return false;
		}
		if pallet_name == T::SudoPallet::name().to_ascii_lowercase() {
			return false;
		}
		if pallet_name == T::TimestampPallet::name().to_ascii_lowercase() {
			return false;
		}
		if pallet_name == T::ImOnlinePallet::name().to_ascii_lowercase() {
			return false;
		}
		if pallet_name == T::EthyPallet::name().to_ascii_lowercase() {
			return false;
		}
		if pallet_name == T::DemocracyPallet::name().to_ascii_lowercase() {
			return false;
		}
		if pallet_name == T::PreimagePallet::name().to_ascii_lowercase() {
			return false;
		}
		if pallet_name == T::CouncilPallet::name().to_ascii_lowercase() {
			return false;
		}
		if pallet_name == T::SchedulerPallet::name().to_ascii_lowercase() {
			return false;
		}
		true
	}
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct MaintenanceChecker<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> Default for MaintenanceChecker<T> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T: Config> MaintenanceChecker<T> {
	pub fn new() -> Self {
		Self(Default::default())
	}
}

impl<T: frame_system::Config + Config> MaintenanceCheck<T> for MaintenanceChecker<T>
where
	<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
{
	fn call_paused(call: &<T as frame_system::Config>::RuntimeCall) -> bool {
		let CallMetadata { function_name, pallet_name } = call.get_call_metadata();

		// Ensure this pallet is not part of the excluded pallets specified in Config
		if !Pallet::<T>::is_pallet_blockable(&pallet_name.to_ascii_lowercase()) {
			return false;
		}

		let pallet_name = BoundedVec::truncate_from(pallet_name.as_bytes().to_ascii_lowercase());
		let function_name =
			BoundedVec::truncate_from(function_name.as_bytes().to_ascii_lowercase());

		// Check whether call is blocked
		if BlockedCalls::<T>::contains_key((pallet_name.clone(), function_name)) {
			return true;
		}

		// Check whether pallet is blocked
		if BlockedPallets::<T>::contains_key(pallet_name) {
			return true;
		}

		false
	}
}

impl<T: frame_system::Config + Config> MaintenanceCheckEVM<T> for MaintenanceChecker<T> {
	fn validate_evm_call(signer: &<T as frame_system::Config>::AccountId, target: &H160) -> bool {
		// Check if we are in maintenance mode
		if MaintenanceModeActive::<T>::get() {
			return false;
		}
		if BlockedAccounts::<T>::contains_key(signer) {
			return false;
		}
		if BlockedEVMAddresses::<T>::contains_key(target) {
			return false;
		}
		true
	}

	fn validate_evm_create(signer: &<T as frame_system::Config>::AccountId) -> bool {
		// Check if we are in maintenance mode
		if MaintenanceModeActive::<T>::get() {
			return false;
		}
		if BlockedAccounts::<T>::contains_key(signer) {
			return false;
		}
		true
	}
}

impl<T: Config + Send + Sync + Debug> SignedExtension for MaintenanceChecker<T>
where
	<T as Config>::RuntimeCall: GetCallMetadata,
{
	const IDENTIFIER: &'static str = "CheckMaintenanceMode";
	type AccountId = T::AccountId;
	type Call = <T as Config>::RuntimeCall;
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

		let pallet_name = pallet_name.to_ascii_lowercase();

		// Ensure this pallet is not part of the excluded pallets specified in Config
		if !Pallet::<T>::is_pallet_blockable(&pallet_name) {
			return Ok(ValidTransaction::default());
		}

		// Check if we are in maintenance mode
		if <MaintenanceModeActive<T>>::get() {
			return Err(TransactionValidityError::Invalid(InvalidTransaction::Custom(1)));
		}

		if BlockedAccounts::<T>::contains_key(who) {
			return Err(TransactionValidityError::Invalid(InvalidTransaction::Custom(2)));
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
