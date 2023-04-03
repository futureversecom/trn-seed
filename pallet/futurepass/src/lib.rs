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

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod weights;

use frame_support::{
	dispatch::Dispatchable,
	ensure,
	pallet_prelude::DispatchResult,
	traits::{Currency, Get, InstanceFilter, IsSubType, IsType, OriginTrait, ReservableCurrency},
	weights::GetDispatchInfo,
	RuntimeDebug,
};
use sp_std::vec::Vec;
pub use weights::WeightInfo;

/// The logging target for this pallet
pub(crate) const LOG_TARGET: &str = "futurepass";

pub trait ProxyProvider<AccountId> {
	// type ProxyType: Parameter
	// 	+ Member
	// 	+ Ord
	// 	+ PartialOrd
	// 	+ InstanceFilter<<Self as ProxyProvider<AccountId>>::Call>
	// 	+ Default;

	fn generate_keyless_account(proxy: &AccountId) -> AccountId;
	fn exists(account: &AccountId, proxy: &AccountId) -> bool;
	fn proxies(account: &AccountId) -> Vec<AccountId>;
	fn add_proxy(account: &AccountId, proxy: AccountId) -> DispatchResult;
	fn remove_proxy(account: &AccountId, proxy: AccountId) -> DispatchResult;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		// type Proxy: ProxyProvider<Self::AccountId, Self::ProxyType>;
		type Proxy: ProxyProvider<Self::AccountId>;

		/// The overarching call type.
		// type Call: Parameter
		// 	+ Dispatchable<Origin = Self::Origin>
		// 	+ GetDispatchInfo
		// 	+ From<frame_system::Call<Self>>
		// 	+ IsSubType<Call<Self>>
		// 	+ IsType<<Self as frame_system::Config>::Call>;

		/// A kind of proxy; specified with the proxy and passed in to the `IsProxyable` fitler.
		/// The instance filter determines whether a given call may be proxied under this type.
		///
		/// IMPORTANT: `Default` must be provided and MUST BE the the *most permissive* value.
		// type ProxyType: Parameter
		// 	+ Member
		// 	+ Ord
		// 	+ PartialOrd
		// 	+ InstanceFilter<<Self as Config>::Call>
		// 	+ Default
		// 	+ MaxEncodedLen;

		// /// The overarching call type.
		// type Call: Parameter
		// + Dispatchable<Origin = Self::Origin>
		// + GetDispatchInfo
		// + From<frame_system::Call<Self>>
		// + IsSubType<Call<Self>>
		// + IsType<<Self as frame_system::Config>::Call>;

		// /// Multicurrency support
		// type Currency: ReservableCurrency<Self::AccountId>;

		/// Allowed origins to ease transition to council governance
		type ApproveOrigin: EnsureOrigin<Self::Origin>;
		/// The default chain ID to use if not set in the chain spec
		type DefaultChainId: Get<u64>;
		/// Interface to access weight values
		type WeightInfo: WeightInfo;
	}

	#[pallet::type_value]
	pub fn DefaultChainId<T: Config>() -> u64 {
		T::DefaultChainId::get()
	}

	#[pallet::storage]
	pub type Holders<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::AccountId>; // account -> futurepass

	#[pallet::storage]
	pub type DefaultProxy<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::AccountId>; // delegate -> futurepass

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Futurepass creation
		FuturepassCreated {
			futurepass: T::AccountId,
			delegate: T::AccountId,
		},
		/// Futurepass registration
		FuturepassRegistered {
			futurepass: T::AccountId,
			delegate: T::AccountId,
		},
		/// Futurepass delegate unregister
		FuturepassUnregistered {
			futurepass: T::AccountId,
			delegate: T::AccountId,
		},
		/// Futurepass transfer
		FuturepassTransferred {
			old_owner: T::AccountId,
			new_owner: T::AccountId,
			futurepass: T::AccountId,
		},
		DefaultProxySet {
			delegate: T::AccountId,
			futurepass: Option<T::AccountId>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Account is already futurepass holder
		AccountAlreadyRegistered,
		/// Account is not futurepass delegate
		DelegateNotRegistered,
		/// Account is not futurepass owner
		NotFuturepassOwner,
		/// Account does not have permission to call this function
		PermissionDenied,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a futurepass account for the delegator that is able to make calls on behalf of
		/// futurepass.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// Parameters:
		/// - `account`: The delegated account for the futurepass.
		#[pallet::weight(T::WeightInfo::set_chain_id())] // TODO
		pub fn create(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// TODO: ensure account is not a futurepass (must be EOA)

			ensure!(!Holders::<T>::contains_key(&account), Error::<T>::AccountAlreadyRegistered);

			let futurepass = T::Proxy::generate_keyless_account(&account);

			Holders::<T>::set(&account, Some(futurepass.clone()));
			T::Proxy::add_proxy(&futurepass, account.clone())?;
			Self::deposit_event(Event::<T>::FuturepassCreated { futurepass, delegate: account });
			Ok(())
		}

		/// Register a delegator to an existing futurepass account.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// Parameters:
		/// - `futurepass`: futurepass account to register the account as delegate.
		/// - `delegate`: The delegated account for the futurepass.
		#[pallet::weight(T::WeightInfo::set_chain_id())] // TODO
		pub fn register(
			origin: OriginFor<T>,
			futurepass: T::AccountId,
			delegate: T::AccountId,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;

			// caller must be futurepass holder
			// TODO: or they can have any permission (sufficient permissions) to add other
			// delegators
			ensure!(
				Holders::<T>::get(&owner) == Some(futurepass.clone()),
				Error::<T>::NotFuturepassOwner
			);

			// maybe we can check here if caller/owner has sufficient permissions to add the other
			// delegate?
			ensure!(T::Proxy::exists(&futurepass, &owner), Error::<T>::DelegateNotRegistered);

			T::Proxy::add_proxy(&futurepass, delegate.clone())?;
			Self::deposit_event(Event::<T>::FuturepassRegistered { futurepass, delegate });
			Ok(())
		}

		/// Unregister a delegate from a futurepass account.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// Parameters:
		/// - `futurepass`: futurepass account to unregister the delegate from.
		/// - `delegate`: The delegated account for the futurepass. Note: if caller is futurepass
		///   holder onwer,
		/// they can remove any delegate (including themselves); otherwise the caller must be the
		/// delegate (can only remove themself).
		#[pallet::weight(T::WeightInfo::set_chain_id())] // TODO
		pub fn unregister(
			origin: OriginFor<T>,
			futurepass: T::AccountId,
			delegate: T::AccountId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			// Check if the caller is the owner of the futurepass
			let is_owner = Holders::<T>::get(&caller) == Some(futurepass.clone());

			// Check if the caller is the delegate or the owner of the futurepass
			ensure!(is_owner || caller == delegate, Error::<T>::PermissionDenied);

			// Check if the delegate is registered with the futurepass
			ensure!(T::Proxy::exists(&futurepass, &delegate), Error::<T>::DelegateNotRegistered);

			// Remove the delegate from the futurepass
			T::Proxy::remove_proxy(&futurepass, delegate.clone())?;

			// If the caller is the owner of the futurepass, remove the ownership
			if is_owner {
				Holders::<T>::remove(&caller);
			}

			Self::deposit_event(Event::<T>::FuturepassUnregistered { futurepass, delegate });
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
			let proxies = T::Proxy::proxies(&futurepass);
			for proxy in proxies.iter() {
				T::Proxy::remove_proxy(&futurepass, proxy.clone())?;
			}

			// Add the current owner as a proxy delegate
			T::Proxy::add_proxy(&futurepass, new_owner.clone())?;

			// Set the new owner as the owner of the futurepass
			Holders::<T>::insert(&new_owner, futurepass.clone());

			Self::deposit_event(Event::<T>::FuturepassTransferred {
				old_owner: owner,
				new_owner,
				futurepass,
			});
			Ok(())
		}

		// /// Set the default proxy for a delegate, which can be used to proxy all delegate requests
		// /// to a futurepass account.
		// ///
		// /// The dispatch origin for this call must be _Signed_ and must be the delegate that the
		// /// default proxy is being set for.
		// ///
		// /// Parameters:
		// /// - `futurepass`: An optional parameter that specifies the futurepass account that the
		// ///   delegate requests should be proxied to.
		// /// If `Some(futurepass)`, all delegate requests will be proxied through the designated
		// /// futurepass account. If `None`, no delegate requests will be proxied through a futurepass
		// /// account (default behaviour).
		// #[pallet::weight(T::WeightInfo::set_chain_id())] // TODO
		// pub fn proxy_all(origin: OriginFor<T>, futurepass: Option<T::AccountId>) -> DispatchResult {
		// 	let delegate = ensure_signed(origin)?;

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

impl<T: Config> seed_pallet_common::AccountProxy<T::AccountId> for Pallet<T> {
	fn primary_proxy(who: &T::AccountId) -> Option<T::AccountId> {
		<DefaultProxy<T>>::get(who)
	}
}
