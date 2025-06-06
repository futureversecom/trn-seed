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

use alloc::{boxed::Box, collections::BTreeSet, vec::Vec};
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
		PermissionExpired,
		InvalidExpiry,
		PermissionAlreadyExists,
		InvalidSpendingBalance,
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
		<T as frame_system::Config>::AccountId: From<H160>,
	{
		/// A dispatch permission was granted.
		DispatchPermissionGranted {
			grantor: T::AccountId,
			grantee: T::AccountId,
			spender: Spender,
			spending_balance: Option<Balance>,
			allowed_calls: Vec<CallId<T::StringLimit>>,
			expiry: Option<BlockNumberFor<T>>,
		},
		/// A permissioned transaction was executed.
		PermissionTransactExecuted { grantor: T::AccountId, grantee: T::AccountId },
		/// A dispatch permission was updated.
		DispatchPermissionUpdated {
			grantor: T::AccountId,
			grantee: T::AccountId,
			spender: Spender,
			spending_balance: Option<Balance>,
			allowed_calls: Vec<CallId<T::StringLimit>>,
			expiry: Option<BlockNumberFor<T>>,
		},
		/// A dispatch permission was revoked.
		DispatchPermissionRevoked { grantor: T::AccountId, grantee: T::AccountId },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
		<T as frame_system::Config>::AccountId: From<H160>,
	{
		#[pallet::call_index(0)]
		#[pallet::weight(10_000)]
		pub fn grant_dispatch_permission(
			origin: OriginFor<T>,
			grantee: T::AccountId,
			spender: Spender,
			spending_balance: Option<Balance>,
			allowed_calls: BoundedBTreeSet<CallId<T::StringLimit>, T::MaxCallIds>,
			expiry: Option<BlockNumberFor<T>>,
		) -> DispatchResult {
			let grantor = ensure_signed(origin)?;

			// Ensure spending_balance is only specified if spender is Grantor
			if let Some(_) = spending_balance {
				ensure!(matches!(spender, Spender::Grantor), Error::<T>::InvalidSpendingBalance);
			}

			let block = frame_system::Pallet::<T>::block_number();

			// Ensure expiry is not in the past
			if let Some(expiry_block) = expiry {
				ensure!(expiry_block >= block, Error::<T>::InvalidExpiry);
			}

			// Check if a non-expired permission already exists
			if let Some(existing_permission) = DispatchPermissions::<T>::get(&grantor, &grantee) {
				if let Some(existing_expiry) = existing_permission.expiry {
					ensure!(block > existing_expiry, Error::<T>::PermissionAlreadyExists);
				}
			}

			// Normalize the pallet and function names to lowercase
			let normalized_allowed_calls = BoundedBTreeSet::try_from(
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
			.unwrap(); // Safe unwrap as the size is already bounded

			let permission_record = DispatchPermission {
				spender,
				spending_balance,
				allowed_calls: normalized_allowed_calls.clone(),
				block: frame_system::Pallet::<T>::block_number(),
				expiry,
			};

			DispatchPermissions::<T>::insert(&grantor, &grantee, permission_record);

			// Emit event
			Self::deposit_event(Event::DispatchPermissionGranted {
				grantor,
				grantee,
				spender,
				spending_balance,
				allowed_calls: normalized_allowed_calls.into_iter().collect(),
				expiry,
			});

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(10_000)]
		pub fn update_dispatch_permission(
			origin: OriginFor<T>,
			grantee: T::AccountId,
			spender: Option<Spender>,
			spending_balance: Option<Option<Balance>>,
			allowed_calls: Option<BoundedBTreeSet<CallId<T::StringLimit>, T::MaxCallIds>>,
			expiry: Option<Option<BlockNumberFor<T>>>,
		) -> DispatchResult {
			let grantor = ensure_signed(origin)?;

			// Update the permission record
			DispatchPermissions::<T>::try_mutate(&grantor, &grantee, |permission| {
				let permission_record =
					permission.as_mut().ok_or(Error::<T>::PermissionNotGranted)?;

				// Update spender if provided
				if let Some(new_spender) = spender {
					permission_record.spender = new_spender;
				}

				// Ensure spending_balance is only specified if spender is Grantor
				if let Some(Some(_)) = spending_balance {
					ensure!(
						matches!(permission_record.spender, Spender::Grantor),
						Error::<T>::InvalidSpendingBalance
					);
				}

				// Update fields if provided
				if let Some(new_spending_balance) = spending_balance {
					permission_record.spending_balance = new_spending_balance;
				}
				if let Some(new_allowed_calls) = allowed_calls {
					permission_record.allowed_calls = new_allowed_calls;
				}
				if let Some(new_expiry) = expiry {
					if let Some(expiry_block) = new_expiry {
						let current_block = frame_system::Pallet::<T>::block_number();
						ensure!(expiry_block >= current_block, Error::<T>::InvalidExpiry);
					}
					permission_record.expiry = new_expiry;
				}

				// Emit event with updated fields
				Self::deposit_event(Event::DispatchPermissionUpdated {
					grantor: grantor.clone(),
					grantee: grantee.clone(),
					spender: permission_record.spender.clone(),
					spending_balance: permission_record.spending_balance,
					allowed_calls: permission_record.allowed_calls.clone().into_iter().collect(),
					expiry: permission_record.expiry,
				});

				Ok(())
			})
		}

		#[pallet::call_index(2)]
		#[pallet::weight(1000)]
		pub fn transact(
			origin: OriginFor<T>,
			grantor: T::AccountId,
			call: Box<<T as Config>::RuntimeCall>,
		) -> DispatchResult {
			let grantee = ensure_signed(origin.clone())?;

			let permission_record = DispatchPermissions::<T>::get(&grantor, &grantee)
				.ok_or(Error::<T>::PermissionNotGranted)?;

			// Check if the permission has expired
			if let Some(expiry) = permission_record.expiry {
				let current_block = frame_system::Pallet::<T>::block_number();
				ensure!(current_block <= expiry, Error::<T>::PermissionExpired);
			}

			ensure!(
				Self::is_call_allowed(&*call, permission_record.allowed_calls),
				Error::<T>::NotAuthorizedCall
			);

			// Dispatch the call directly
			call.dispatch(frame_system::RawOrigin::Signed(grantor.clone()).into())
				.map_err(|e| e.error)?;

			// Emit event
			Self::deposit_event(Event::PermissionTransactExecuted { grantor, grantee });

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(10_000)]
		pub fn revoke_dispatch_permission(
			origin: OriginFor<T>,
			grantee: T::AccountId,
		) -> DispatchResult {
			let grantor = ensure_signed(origin)?;

			// Remove the permission if it exists
			let removed = DispatchPermissions::<T>::take(&grantor, &grantee);
			ensure!(removed.is_some(), Error::<T>::PermissionNotGranted);

			Self::deposit_event(Event::DispatchPermissionRevoked { grantor, grantee });

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
