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
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	pallet_prelude::*,
	traits::IsSubType,
};
use frame_system::pallet_prelude::*;
use pallet_sylo_data_verification::DataId;
use seed_pallet_common::SyloDataVerificationProvider;
use sp_std::{convert::TryInto, vec};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod types;

pub use types::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching call type.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin, PostInfo = PostDispatchInfo>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>
			+ IsSubType<Call<Self>>;

		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type SyloDataVerificationProvider: SyloDataVerificationProvider<
			AccountId = Self::AccountId,
			StringLimit = Self::StringLimit,
		>;

		/// Limit on the number of permissions that can be granted at once
		#[pallet::constant]
		type MaxPermissions: Get<u32>;

		/// The maximum number of tags in a data validation record.
		#[pallet::constant]
		type MaxTags: Get<u32>;

		/// The max length used for data ids
		#[pallet::constant]
		type StringLimit: Get<u32>;
	}

	#[pallet::storage]
	pub type PermissionRecords<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Twox64Concat, T::AccountId>,
			NMapKey<Twox64Concat, DataId<T::StringLimit>>,
			NMapKey<Twox64Concat, T::AccountId>,
		),
		PermissionRecord<T::AccountId, BlockNumberFor<T>>,
	>;

	#[pallet::storage]
	pub type TaggedPermissionRecords<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		T::AccountId,
		TaggedPermissionRecord<BlockNumberFor<T>, T::MaxTags, T::StringLimit>,
	>;

	#[pallet::storage]
	pub type PermissionReferences<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		T::AccountId,
		PermissionReference<T::StringLimit>,
	>;

	#[pallet::error]
	pub enum Error<T> {
		/// Attempted to grant permissions for a data record that does not exist
		DataRecordDoesNotExist,
		/// Cannot revoke a permission that has not been previously granted
		PermissionDoesNotExist,
		/// A permission that is set to irrevocable cannot also set to have an
		/// expiry
		IrrevocableCannotBeExpirable,
		/// Attempted to grant a permission as a delegate without the required
		/// DISTRIBUTE permission
		MissingDistributePermission,
		/// Distribute permissions can only be granted by the data author
		CannotGrantDistributePermission,
		/// An irrevocable permission can not be revoked
		PermissionIrrevocable,
		/// Only the account that granted a permission or the data author
		/// themselves can revoke a permission
		NotPermissionGrantor,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An account has been granted permission for a given data record
		DataPermissionGranted {
			grantor: T::AccountId,
			grantee: T::AccountId,
			data_id: Vec<u8>,
			permission: DataPermission,
			expiry: Option<BlockNumberFor<T>>,
			irrevocable: bool,
		},
		/// An account's permission has been revoked for a given data record
		DataPermissionRevoked { revoker: T::AccountId, grantee: T::AccountId, data_id: Vec<u8> },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(1_000)]
		pub fn grant_data_permissions(
			origin: OriginFor<T>,
			data_author: T::AccountId,
			grantee: T::AccountId,
			data_ids: BoundedVec<DataId<T::StringLimit>, T::MaxPermissions>,
			permission: DataPermission,
			expiry: Option<BlockNumberFor<T>>,
			irrevocable: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let permission_record = PermissionRecord {
				grantor: who.clone(),
				permission,
				block: <frame_system::Pallet<T>>::block_number(),
				expiry,
				irrevocable,
			};

			if irrevocable {
				ensure!(expiry.is_none(), Error::<T>::IrrevocableCannotBeExpirable);
			}

			for data_id in data_ids.iter() {
				let data_id = data_id.clone();

				ensure!(
					T::SyloDataVerificationProvider::validation_record_exists(
						data_author.clone(),
						data_id.clone()
					),
					Error::<T>::DataRecordDoesNotExist
				);

				// if this permission is being granted by an account other than the
				// data author, then ensure the account has been granted the DISTRIBUTE
				// permission
				if who != data_author {
					let distribution_permission =
						<PermissionRecords<T>>::get((&data_author, &data_id, &who))
							.ok_or(Error::<T>::MissingDistributePermission)?;

					ensure!(
						distribution_permission.permission == DataPermission::DISTRIBUTE,
						Error::<T>::MissingDistributePermission,
					);

					ensure!(
						permission != DataPermission::DISTRIBUTE,
						Error::<T>::CannotGrantDistributePermission
					);
				}

				<PermissionRecords<T>>::try_mutate(
					(&data_author, &data_id, &grantee),
					|maybe_record| -> DispatchResult {
						match maybe_record {
							Some(record) if record.irrevocable => {
								// for existing permission records that are irrevocable, we allow the grantor
								// to upgrade the permission, but not downgrade
								ensure!(
									permission >= record.permission,
									Error::<T>::PermissionIrrevocable,
								);

								ensure!(expiry.is_none(), Error::<T>::IrrevocableCannotBeExpirable);
							},
							_ => (),
						}

						*maybe_record = Some(permission_record.clone());

						Ok(())
					},
				)?;

				Self::deposit_event(Event::DataPermissionGranted {
					grantor: who.clone(),
					grantee: grantee.clone(),
					data_id: data_id.clone().to_vec(),
					permission,
					expiry,
					irrevocable,
				});
			}

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(1_000)]
		pub fn revoke_data_permissions(
			origin: OriginFor<T>,
			data_author: T::AccountId,
			grantee: T::AccountId,
			data_ids: BoundedVec<DataId<T::StringLimit>, T::MaxPermissions>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			for data_id in data_ids.iter() {
				let data_id = data_id.clone();

				<PermissionRecords<T>>::try_mutate(
					(&data_author, &data_id, &grantee),
					|maybe_record| -> DispatchResult {
						match maybe_record {
							Some(record) if record.irrevocable => {
								Err(Error::<T>::PermissionIrrevocable)
							},

							// the data author can always revoke a permission, however
							// if this called by a delegate, then they can only revoke
							// a record that they also granted
							Some(record) if record.grantor != who && who != data_author => {
								Err(Error::<T>::NotPermissionGrantor)
							},
							_ => Ok(()),
						}?;

						*maybe_record = None;

						Ok(())
					},
				)?;

				Self::deposit_event(Event::DataPermissionRevoked {
					revoker: who.clone(),
					grantee: grantee.clone(),
					data_id: data_id.clone().to_vec(),
				});
			}

			Ok(())
		}
	}
}
