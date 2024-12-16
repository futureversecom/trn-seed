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

pub use pallet::*;

use frame_support::{
	pallet_prelude::*,
	traits::{
		fungibles::{self, metadata::Inspect as MetadataInspect, Inspect, Mutate},
		tokens::{Fortitude, Precision, Preservation},
	},
	transactional, PalletId,
};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use seed_pallet_common::CreateExt;
use seed_primitives::{AssetId, Balance};
use serde::{Deserialize, Serialize};
use sp_core::{H160, U256};
use sp_runtime::{
	traits::{AccountIdConversion, Zero},
	ArithmeticError, DispatchError, FixedU128, RuntimeDebug, SaturatedConversion,
};
use sp_std::{cmp::min, convert::TryInto, prelude::*, vec};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod types;

pub use types::*;

#[frame_support::pallet]
pub mod pallet {
	use serde::ser;

	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		#[pallet::constant]
		type MaxServiceEndpoints: Get<u32>;

		#[pallet::constant]
		type StringLimit: Get<u32>;

		#[pallet::constant]
		type ResolverMethod: Get<[u8; 9]>;
	}

	#[pallet::storage]
	pub type Resolvers<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BoundedVec<u8, T::StringLimit>,
		Resolver<T::AccountId, T::MaxServiceEndpoints, T::StringLimit>,
	>;

	#[pallet::error]
	pub enum Error<T> {
		/// The Resolver identifier is already in use
		ResolverAlreadyRegistered,
		/// The Resolver has not been registered
		ResolverNotRegistered,
		/// Account is not controller of resolver
		NotController,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		ResolverRegistered {
			id: Vec<u8>,
			controller: T::AccountId,
			service_endpoints: BoundedVec<BoundedVec<u8, T::StringLimit>, T::MaxServiceEndpoints>,
		},
		ResolverUpdated {
			id: Vec<u8>,
			controller: T::AccountId,
			service_endpoints: BoundedVec<BoundedVec<u8, T::StringLimit>, T::MaxServiceEndpoints>,
		},
		ResolverUnregistered {
			id: Vec<u8>,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(1_000)]
		pub fn register_resolver(
			origin: OriginFor<T>,
			identifier: BoundedVec<u8, T::StringLimit>,
			service_endpoints: BoundedVec<ServiceEndpoint<T::StringLimit>, T::MaxServiceEndpoints>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(
				<Resolvers<T>>::get(identifier.clone()).is_none(),
				Error::<T>::ResolverAlreadyRegistered
			);

			let resolver =
				Resolver { controller: who.clone(), service_endpoints: service_endpoints.clone() };

			<Resolvers<T>>::insert(identifier.clone(), resolver);

			Self::deposit_event(Event::ResolverRegistered {
				id: identifier.to_vec(),
				controller: who,
				service_endpoints,
			});

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(1_000)]
		pub fn update_resolver(
			origin: OriginFor<T>,
			identifier: BoundedVec<u8, T::StringLimit>,
			service_endpoints: BoundedVec<ServiceEndpoint<T::StringLimit>, T::MaxServiceEndpoints>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let mut resolver =
				<Resolvers<T>>::get(identifier.clone()).ok_or(Error::<T>::ResolverNotRegistered)?;

			ensure!(who == resolver.controller, Error::<T>::NotController);

			resolver.service_endpoints = service_endpoints.clone();

			<Resolvers<T>>::insert(identifier.clone(), resolver);

			Self::deposit_event(Event::ResolverUpdated {
				id: identifier.to_vec(),
				controller: who,
				service_endpoints,
			});

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(1_000)]
		pub fn unregister_resolver(
			origin: OriginFor<T>,
			identifier: BoundedVec<u8, T::StringLimit>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let resolver =
				<Resolvers<T>>::get(identifier.clone()).ok_or(Error::<T>::ResolverNotRegistered)?;

			ensure!(who == resolver.controller, Error::<T>::NotController);

			<Resolvers<T>>::remove(identifier.clone());

			Self::deposit_event(Event::ResolverUnregistered { id: identifier.to_vec() });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_reserved_resolver_id(
			identifier: BoundedVec<u8, T::StringLimit>,
		) -> ResolverId<T::StringLimit> {
			let method: BoundedVec<u8, T::StringLimit> =
				BoundedVec::try_from(<T as Config>::ResolverMethod::get().to_vec())
					.expect("Failed to convert invalid resolver method config");

			ResolverId { method, identifier }
		}
	}
}
