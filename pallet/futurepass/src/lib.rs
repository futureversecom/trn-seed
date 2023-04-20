// SPDX-License-Identifier: Apache-2.0
// This file is part of Frontier.
//
// Copyright (c) 2022 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # EVM chain ID pallet
//!
//! The pallet that stores the numeric Ethereum-style chain id in the runtime.
//! It can simplify setting up multiple networks with different chain ID by configuring the
//! chain spec without requiring changes to the runtime config.
//!
//! **NOTE**: we recommend that the production chains still use the const parameter type, as
//! this extra storage access would imply some performance penalty.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod weights;

use alloc::boxed::Box;
use frame_support::{
	ensure,
	pallet_prelude::{DispatchError, DispatchResult, *},
	traits::{Get, IsSubType, IsType},
};
use frame_system::pallet_prelude::*;
use hex::{encode, FromHex};
use seed_primitives::AccountId;
use sp_core::H160;
use sp_runtime::traits::Dispatchable;
use sp_std::vec::Vec;
pub use weights::WeightInfo;

/// The logging target for this pallet
#[allow(dead_code)]
pub(crate) const LOG_TARGET: &str = "futurepass";

pub trait ProxyProvider<T: Config> {
	fn exists(
		futurepass: &T::AccountId,
		delegate: &T::AccountId,
		proxy_type: Option<T::ProxyType>,
	) -> bool;
	fn delegates(futurepass: &T::AccountId) -> Vec<(T::AccountId, T::ProxyType)>;
	fn add_delegate(
		funder: &T::AccountId,
		futurepass: &T::AccountId,
		delegate: &T::AccountId,
		proxy_type: &T::ProxyType,
	) -> DispatchResult;
	fn remove_delegate(
		receiver: &T::AccountId,
		futurepass: &T::AccountId,
		delegate: &T::AccountId,
	) -> DispatchResult;
	fn proxy_call(
		caller: OriginFor<T>,
		futurepass: T::AccountId,
		call: <T as Config>::Call,
	) -> DispatchResult;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::traits::InstanceFilter;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type FuturepassPrefix: Get<[u8; 4]>;

		type Proxy: ProxyProvider<Self>;

		/// overarching Call type
		type Call: Parameter
			+ Dispatchable<Origin = Self::Origin>
			+ From<frame_system::Call<Self>>
			+ IsSubType<Call<Self>>
			+ IsType<<Self as frame_system::Config>::Call>;

		/// Allowed origins to ease transition to council governance
		type ApproveOrigin: EnsureOrigin<Self::Origin>;

		/// A kind of proxy; specified with the proxy and passed in to the `IsProxyable` fitler.
		/// The instance filter determines whether a given call may be proxied under this type.
		///
		/// IMPORTANT: `Default` must be provided and MUST BE the the *most permissive* value.
		type ProxyType: Parameter
			+ Member
			+ Ord
			+ PartialOrd
			+ InstanceFilter<<Self as Config>::Call>
			+ Default
			+ MaxEncodedLen;

		/// Interface to access weight values
		type WeightInfo: WeightInfo;
	}

	#[pallet::type_value]
	pub fn DefaultValue() -> u128 {
		1
	}

	/// The next available incrementing futurepass id
	#[pallet::storage]
	pub type NextFuturepassId<T> = StorageValue<_, u128, ValueQuery, DefaultValue>;

	/// Futurepass holders (account -> futurepass)
	#[pallet::storage]
	pub type Holders<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::AccountId>;

	/// Accounts which have set futurepass as default proxied on-chain account (delegate ->
	/// futurepass)
	#[pallet::storage]
	pub type DefaultProxy<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::AccountId>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Futurepass creation
		FuturepassCreated {
			futurepass: T::AccountId,
			delegate: T::AccountId,
		},
		/// Delegate registration to Futurepass account
		DelegateRegistered {
			futurepass: T::AccountId,
			delegate: T::AccountId,
			proxy_type: T::ProxyType,
		},
		/// Delegate unregistration from Futurepass account
		DelegateUnregistered {
			futurepass: T::AccountId,
			delegate: T::AccountId,
		},
		/// Futurepass transfer
		FuturepassTransferred {
			old_owner: T::AccountId,
			new_owner: T::AccountId,
			futurepass: T::AccountId,
		},
		DefaultFuturepassSet {
			delegate: T::AccountId,
			futurepass: Option<T::AccountId>,
		},
		/// A proxy call was executed correctly, with the given call
		ProxyExecuted {
			result: DispatchResult,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Account is already futurepass holder
		AccountAlreadyRegistered,
		/// Account is not futurepass delegate
		DelegateNotRegistered,
		/// Account already exists as a delegate
		DelegateAlreadyExists,
		/// Account is not futurepass owner
		NotFuturepassOwner,
		/// Futurepass owner cannot remove themselves
		OwnerCannotUnregister,
		/// Account does not have permission to call this function
		PermissionDenied,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: From<H160>,
	{
		/// Create a futurepass account for the delegator that is able to make calls on behalf of
		/// futurepass.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// Parameters:
		/// - `account`: The delegated account for the futurepass.
		#[pallet::weight(T::WeightInfo::set_chain_id())] // TODO
		pub fn create(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_create_futurepass(who, account)?;
			Ok(())
		}

		/// Register a delegator to an existing futurepass account.
		/// Note: Only futurepass owner account can add more delegates.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// Parameters:
		/// - `futurepass`: Futurepass account to register the account as delegate.
		/// - `proxy_type`: Delegate permission level
		/// - `delegate`: The delegated account for the futurepass.
		#[pallet::weight(T::WeightInfo::set_chain_id())] // TODO
		pub fn register_delegate(
			origin: OriginFor<T>,
			futurepass: T::AccountId,
			delegate: T::AccountId,
			proxy_type: T::ProxyType,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;

			// For V1 - caller must be futurepass holder
			ensure!(
				Holders::<T>::get(&owner.clone()) == Some(futurepass.clone()),
				Error::<T>::NotFuturepassOwner
			);

			ensure!(T::Proxy::exists(&futurepass, &owner, None), Error::<T>::DelegateNotRegistered);
			// for V1, only T::ProxyType::default() is allowed.
			// TODO - update the restriction in V2 as required.
			ensure!(proxy_type == T::ProxyType::default(), Error::<T>::PermissionDenied);
			// delegate should not be an existing proxy of any T::ProxyType
			// This is required here coz pallet_proxy's duplicate check is only for the specific
			// proxy_type
			ensure!(
				!T::Proxy::exists(&futurepass, &delegate, None),
				Error::<T>::DelegateAlreadyExists
			);

			T::Proxy::add_delegate(&owner, &futurepass, &delegate, &proxy_type)?;
			Self::deposit_event(Event::<T>::DelegateRegistered {
				futurepass,
				delegate,
				proxy_type,
			});
			Ok(())
		}

		/// Unregister a delegate from a futurepass account.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// Parameters:
		/// - `futurepass`: Futurepass account to unregister the delegate from.
		/// - `delegate`: The delegated account for the futurepass. Note: if caller is futurepass
		///   holder onwer,
		/// they can remove any delegate (including themselves); otherwise the caller must be the
		/// delegate (can only remove themself).
		#[pallet::weight(T::WeightInfo::set_chain_id())] // TODO
		pub fn unregister_delegate(
			origin: OriginFor<T>,
			futurepass: T::AccountId,
			delegate: T::AccountId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			// Check if the caller is the owner of the futurepass
			let is_owner = Holders::<T>::get(&caller) == Some(futurepass.clone());

			// If provided delegate is the owner themselves, do not allow this action
			ensure!(!(is_owner && caller == delegate), Error::<T>::OwnerCannotUnregister);

			// Check if caller is owner (can remove anyone) or delegate (can remove themsleves) from
			// futurepass
			ensure!(is_owner || caller == delegate, Error::<T>::PermissionDenied);

			// Check if the delegate is registered with the futurepass
			ensure!(
				T::Proxy::exists(&futurepass, &delegate, None),
				Error::<T>::DelegateNotRegistered
			);

			// Remove the delegate from the futurepass
			T::Proxy::remove_delegate(&caller, &futurepass, &delegate)?;

			// If the caller is the owner of the futurepass, remove the ownership
			// if is_owner && caller == delegate { // TODO: validate whether we cant this
			// functionality 	Holders::<T>::remove(&caller);
			// }

			Self::deposit_event(Event::<T>::DelegateUnregistered { futurepass, delegate });
			Ok(())
		}

		/// Transfer ownership of a futurepass to a new account.
		/// The new owner must not already own a futurepass.
		/// This removes all delegates from the futurepass.
		/// The new owner will be the only delegate; they can add more delegates.
		///
		/// The dispatch origin for this call must be _Signed_ and must be the current owner of the
		/// futurepass.
		///
		/// Parameters:
		/// - `new_owner`: The new account that will become the owner of the futurepass.
		#[pallet::weight(T::WeightInfo::set_chain_id())] // TODO
		pub fn transfer_futurepass(
			origin: OriginFor<T>,
			new_owner: T::AccountId,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;

			// Get the current futurepass owner from the `Holders` storage mapping
			let futurepass = Holders::<T>::take(&owner).ok_or(Error::<T>::NotFuturepassOwner)?;

			// Ensure that the new owner does not already own a futurepass
			ensure!(!Holders::<T>::contains_key(&new_owner), Error::<T>::AccountAlreadyRegistered);

			// Remove all proxy delegates from the current futurepass
			let delegates = T::Proxy::delegates(&futurepass);
			for delegate in delegates.iter() {
				T::Proxy::remove_delegate(&owner, &futurepass, &delegate.0)?;
			}

			// Add the current owner as a proxy delegate with most permissive type. i.e
			// T::ProxyType::default()
			T::Proxy::add_delegate(&owner, &futurepass, &new_owner, &T::ProxyType::default())?;

			// Set the new owner as the owner of the futurepass
			Holders::<T>::insert(&new_owner, futurepass.clone());

			Self::deposit_event(Event::<T>::FuturepassTransferred {
				old_owner: owner,
				new_owner,
				futurepass,
			});
			Ok(())
		}

		/// Dispatch the given call through Futurepass account. Transaction fees will be paid by the
		/// Futurepass The dispatch origin for this call must be _Signed_
		///
		/// Parameters:
		/// - `futurepass`: The Futurepass account though which the call is dispatched
		/// - `call`: The Call that needs to be dispatched through the Futurepass account
		#[pallet::weight(T::WeightInfo::set_chain_id())] // TODO
		pub fn proxy_extrinsic(
			origin: OriginFor<T>,
			futurepass: T::AccountId,
			call: Box<<T as Config>::Call>,
		) -> DispatchResult {
			ensure_signed(origin.clone())?;
			let result = T::Proxy::proxy_call(origin, futurepass, *call);
			Self::deposit_event(Event::ProxyExecuted { result: result.map(|_| ()).map_err(|e| e) });
			Ok(())
		}

		// /// Set the default proxy for a delegate, which can be used to proxy all delegate
		// requests /// to a futurepass account.
		// ///
		// /// The dispatch origin for this call must be _Signed_ and must be the delegate that the
		// /// default proxy is being set for.
		// ///
		// /// Parameters:
		// /// - `futurepass`: An optional parameter that specifies the futurepass account that the
		// ///   delegate requests should be proxied to.
		// /// If `Some(futurepass)`, all delegate requests will be proxied through the designated
		// /// futurepass account. If `None`, no delegate requests will be proxied through a
		// futurepass /// account (default behaviour).
		// #[pallet::weight(T::WeightInfo::set_chain_id())] // TODO
		// pub fn proxy_all(origin: OriginFor<T>, futurepass: Option<T::AccountId>) ->
		// DispatchResult { 	let delegate = ensure_signed(origin)?;

		// 	if let Some(futurepass) = &futurepass {
		// 		ensure!(
		// 			T::Proxy::exists(&futurepass, &delegate),
		// 			Error::<T>::DelegateNotRegistered
		// 		);
		// 		// TODO: ensure delegate has permissions?
		// 	}

		// 	DefaultProxy::<T>::set(&delegate, futurepass.clone());
		// 	Self::deposit_event(Event::<T>::DefaultProxySet { delegate, futurepass });
		// 	Ok(())
		// }
	}
}

impl<T: Config> Pallet<T> {
	/// Generate the next Ethereum address (H160) with a custom prefix.
	///
	/// The Ethereum address will have a prefix of "FFFFFFFF" (8 hex digits) followed by the current
	/// value of `NextFuturepassId` (32 hex digits) in hexadecimal representation, resulting in a
	/// 40-hex-digit Ethereum address.
	///
	/// `NextFuturepassId` is a 128-bit unsigned integer - which converts to 32 digit hexadecimal
	/// (16 bytes) ensuring sufficient address space for unique addresses.
	///
	/// This function also increments the `NextFuturepassId` storage value for future use.
	///
	/// # Returns
	/// - `T::AccountId`: A generated Ethereum address (H160) with the desired custom prefix.
	fn generate_futurepass_account() -> T::AccountId
	where
		T::AccountId: From<H160>,
	{
		// Convert the futurepass_id to a byte array and increment the value
		let futurepass_id_bytes = NextFuturepassId::<T>::mutate(|futurepass_id| {
			let bytes = futurepass_id.to_be_bytes();
			*futurepass_id += 1;
			bytes
		});

		let prefix = T::FuturepassPrefix::get();

		// Create a new byte array with the combined length of the prefix and the futurepass_id
		// (bytes)
		let mut address_bytes = [0u8; 20];
		address_bytes[..4].copy_from_slice(&prefix);
		address_bytes[4..].copy_from_slice(&futurepass_id_bytes);

		let address = H160::from_slice(&address_bytes);

		T::AccountId::from(address)
	}

	pub fn do_create_futurepass(
		funder: T::AccountId,
		account: T::AccountId,
	) -> Result<T::AccountId, DispatchError>
	where
		T::AccountId: From<sp_core::H160>,
	{
		ensure!(!Holders::<T>::contains_key(&account), Error::<T>::AccountAlreadyRegistered);
		let futurepass = Self::generate_futurepass_account();
		Holders::<T>::set(&account, Some(futurepass.clone()));
		T::Proxy::add_delegate(&funder, &futurepass, &account, &T::ProxyType::default())?;

		Self::deposit_event(Event::<T>::FuturepassCreated {
			futurepass: futurepass.clone(),
			delegate: account,
		});
		Ok(futurepass)
	}
}

impl<T: Config> seed_pallet_common::AccountProxy<T::AccountId> for Pallet<T> {
	fn primary_proxy(who: &T::AccountId) -> Option<T::AccountId> {
		<DefaultProxy<T>>::get(who)
	}
}
