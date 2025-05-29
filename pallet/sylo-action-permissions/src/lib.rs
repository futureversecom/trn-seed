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

use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo},
	pallet_prelude::*,
	traits::{Currency, InstanceFilter, IsSubType},
};
use frame_system::pallet_prelude::*;
use sp_core::H160;

pub mod types;
pub use types::*;

pub trait CreateProxyAccount<AccountId> {
	fn create_proxy_account(grantor: &AccountId, grantee: &AccountId) -> AccountId;
}

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
		<Self as frame_system::Config>::AccountId: From<H160>,
	{
		type Currency: Currency<Self::AccountId>;

		/// The overarching call type.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>
			+ IsSubType<Call<Self>>
			+ IsType<<Self as frame_system::Config>::RuntimeCall>;

		/// A kind of proxy; specified with the proxy and passed in to the `IsProxyable` filter.
		/// The instance filter determines whether a given call may be proxied under this type.
		///
		/// IMPORTANT: `Default` must be provided and MUST BE the *most permissive* value.
		type ProxyType: Parameter
			+ Member
			+ Ord
			+ PartialOrd
			+ InstanceFilter<<Self as Config>::RuntimeCall>
			+ Default
			+ MaxEncodedLen
			+ Into<u8>;

		/// The maximum number of modules allowed in a dispatch permission.
		#[pallet::constant]
		type ModuleLimit: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {
		PermissionNotGranted,
		NotAuthorized,
	}

	#[pallet::storage]
	#[pallet::getter(fn permissions)]
	pub type Permissions<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId, // Grantor
		Blake2_128Concat,
		T::AccountId,                                              // Grantee
		ActionPermissionRecord<T::ModuleLimit, BlockNumberFor<T>>, // Value
		OptionQuery,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<H160>,
	{
		#[pallet::call_index(0)]
		#[pallet::weight(10_000)]
		pub fn grant_action_permission(
			origin: OriginFor<T>,
			grantee: T::AccountId,
		) -> DispatchResult {
			let grantor = ensure_signed(origin)?;

			let permission_record = ActionPermissionRecord {
				permission: DispatchPermission {
					spender: Spender::Grantor,
					spending_balance: None,
					modules: None,
				},
				block: frame_system::Pallet::<T>::block_number(),
				expiry: None,
			};

			Permissions::<T>::insert(&grantor, &grantee, permission_record);
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(10_000)]
		pub fn execute_action(
			origin: OriginFor<T>,
			grantor: T::AccountId,
			call: Box<<T as Config>::RuntimeCall>,
		) -> DispatchResult {
			let grantee = ensure_signed(origin.clone())?;
			let permission_record = Permissions::<T>::get(&grantor, &grantee)
				.ok_or(Error::<T>::PermissionNotGranted)?;

			ensure!(
				permission_record.permission.spender == Spender::Grantor,
				Error::<T>::NotAuthorized
			);

			// Dispatch the call directly
			call.dispatch(frame_system::RawOrigin::Signed(grantor).into())
				.map_err(|e| e.error)?;

			Ok(())
		}
	}
}
