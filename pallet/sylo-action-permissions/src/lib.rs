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
extern crate alloc;

pub use pallet::*;

use alloc::boxed::Box;
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo},
	pallet_prelude::*,
	traits::{CallMetadata, GetCallMetadata, IsSubType},
};
use frame_system::pallet_prelude::*;
use seed_primitives::Balance;
use sp_core::H160;
use sp_runtime::BoundedBTreeSet;

pub mod types;
pub use types::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {

	use std::collections::BTreeSet;

	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config
	where
		<Self as frame_system::Config>::RuntimeCall: GetCallMetadata,
		<Self as frame_system::Config>::AccountId: From<H160>,
	{
		/// The overarching call type.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			+ GetDispatchInfo
			+ GetCallMetadata
			+ From<frame_system::Call<Self>>
			+ IsSubType<Call<Self>>
			+ IsType<<Self as frame_system::Config>::RuntimeCall>;

		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Interface to access weight values
		// type WeightInfo: WeightInfo;

		/// The maximum number of modules allowed in a dispatch permission.
		#[pallet::constant]
		type MaxCallIds: Get<u32>;

		/// The maximum number of modules allowed in a dispatch permission.
		#[pallet::constant]
		type StringLimit: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {
		PermissionNotGranted,
		NotAuthorizedCall,
	}

	#[pallet::storage]
	// #[pallet::getter(fn permissions)]
	pub type DispatchPermissions<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId, // Grantor
		Blake2_128Concat,
		T::AccountId, // Grantee
		DispatchPermission<BlockNumberFor<T>, T::MaxCallIds, T::StringLimit>,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config>
	where
		<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
		<T as frame_system::Config>::AccountId: From<H160>, {}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
		<T as frame_system::Config>::AccountId: From<H160>,
	{
		#[pallet::call_index(0)]
		#[pallet::weight(10_000)]
		pub fn grant_action_permission(
			origin: OriginFor<T>,
			grantee: T::AccountId,
			spender: Spender,
			spending_balance: Option<Balance>,
			allowed_calls: BoundedBTreeSet<CallId<T::StringLimit>, T::MaxCallIds>,
			expiry: Option<BlockNumberFor<T>>,
		) -> DispatchResult {
			let grantor = ensure_signed(origin)?;

			// normalize the pallet and function names to lowercase
			let allowed_calls = BoundedBTreeSet::try_from(
				allowed_calls
					.into_iter()
					.map(|(pallet, function)| {
						let pallet_name: BoundedVec<u8, T::StringLimit> =
							BoundedVec::truncate_from(pallet.to_ascii_lowercase());
						let function_name: BoundedVec<u8, T::StringLimit> =
							BoundedVec::truncate_from(function.to_ascii_lowercase());
						(pallet_name, function_name)
					})
					.collect::<BTreeSet<_>>(),
			)
			.unwrap(); // safe unwrap as the size is already bounded

			let permission_record = DispatchPermission {
				spender,
				spending_balance,
				allowed_calls,
				block: frame_system::Pallet::<T>::block_number(),
				expiry,
			};

			DispatchPermissions::<T>::insert(&grantor, &grantee, permission_record);
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(1000)]
		pub fn transact(
			origin: OriginFor<T>,
			grantor: T::AccountId,
			call: Box<<T as Config>::RuntimeCall>,
		) -> DispatchResult {
			let grantee = ensure_signed(origin.clone())?;

			let permission_record = DispatchPermissions::<T>::get(&grantor, &grantee)
				.ok_or(Error::<T>::PermissionNotGranted)?;

			ensure!(
				Self::is_call_allowed(&*call, permission_record.allowed_calls),
				Error::<T>::NotAuthorizedCall
			);

			// Dispatch the call directly
			call.dispatch(frame_system::RawOrigin::Signed(grantor).into())
				.map_err(|e| e.error)?;

			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
		<T as frame_system::Config>::AccountId: From<H160>,
	{
		fn is_call_allowed(
			call: &<T as Config>::RuntimeCall,
			allowed_calls: BoundedBTreeSet<CallId<T::StringLimit>, T::MaxCallIds>,
		) -> bool {
			let CallMetadata { function_name, pallet_name } = call.get_call_metadata();

			println!("{} {}", function_name, pallet_name);

			let pallet_name: BoundedVec<u8, T::StringLimit> =
				BoundedVec::truncate_from(pallet_name.as_bytes().to_ascii_lowercase());
			let function_name: BoundedVec<u8, T::StringLimit> =
				BoundedVec::truncate_from(function_name.as_bytes().to_ascii_lowercase());

			let wildcard: BoundedVec<u8, T::StringLimit> = BoundedVec::truncate_from(b"*".to_vec());

			allowed_calls.iter().any(|(pallet, function)| {
				if pallet == &pallet_name || pallet == &wildcard {
					if function == &function_name || function == &wildcard {
						return true;
					}
				}

				false
			})
		}
	}
}
