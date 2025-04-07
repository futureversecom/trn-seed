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

use alloc::vec::Vec;
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	pallet_prelude::*,
	traits::IsSubType,
};
use frame_system::pallet_prelude::*;
use seed_pallet_common::sylo::*;
use sp_std::{convert::TryInto, vec};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod types;
pub use types::*;

pub mod weights;
pub use weights::WeightInfo;

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

		/// Interface to access weight values
		type WeightInfo: WeightInfo;

		/// Provides functionality to retrieve data validation records
		type SyloDataVerificationProvider: SyloDataVerificationProvider<
			AccountId = Self::AccountId,
			StringLimit = Self::StringLimit,
		>;

		/// Limit on the number of permissions that can be granted at once
		#[pallet::constant]
		type MaxPermissions: Get<u32>;

		/// The maximum number of tags that can be used in a tagged permission
		/// record
		#[pallet::constant]
		type MaxTags: Get<u32>;

		/// The maximum number of tagged permission records that can be granted
		/// to an account
		#[pallet::constant]
		type MaxPermissionRecords: Get<u32>;

		/// The max length used for data ids
		#[pallet::constant]
		type StringLimit: Get<u32>;
	}

	#[pallet::storage]
	pub type NextPermissionRecordId<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, u32, ValueQuery>;

	#[pallet::storage]
	pub type PermissionRecords<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Twox64Concat, T::AccountId>,
			NMapKey<Twox64Concat, DataId<T::StringLimit>>,
			NMapKey<Twox64Concat, T::AccountId>,
		),
		BoundedVec<
			(u32, PermissionRecord<T::AccountId, BlockNumberFor<T>>),
			T::MaxPermissionRecords,
		>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type TaggedPermissionRecords<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		T::AccountId,
		BoundedVec<
			(u32, TaggedPermissionRecord<BlockNumberFor<T>, T::MaxTags, T::StringLimit>),
			T::MaxPermissionRecords,
		>,
		ValueQuery,
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
		/// A permission that is set to irrevocable cannot also set to have an
		/// expiry
		IrrevocableCannotBeExpirable,
		/// Exceeded the maximum number of record permissions granted to a given
		/// account
		ExceededMaxPermissions,
		/// Attempted to grant a permission as a delegate without the required
		/// DISTRIBUTE permission
		MissingDistributePermission,
		/// Distribute permissions can only be granted by the data author
		CannotGrantDistributePermission,
		/// Irrevocable permissions can only be granted by the data author
		CannotGrantIrrevocablePermission,
		/// An irrevocable permission can not be revoked
		PermissionIrrevocable,
		/// Only the account that granted a permission or the data author
		/// themselves can revoke a permission
		NotPermissionGrantor,
		/// Cannot revoke a permission that does not exist
		PermissionNotFound,
		/// An accompanying verification record for the offchain permission does
		/// not exist
		MissingValidationRecord,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An account has been granted permission for a given data record
		DataPermissionGranted {
			data_author: T::AccountId,
			grantor: T::AccountId,
			grantee: T::AccountId,
			data_id: Vec<u8>,
			permission: DataPermission,
			expiry: Option<BlockNumberFor<T>>,
			irrevocable: bool,
		},
		/// An account's permission has been revoked for a given data record
		DataPermissionRevoked {
			revoker: T::AccountId,
			grantee: T::AccountId,
			permission: DataPermission,
			data_id: Vec<u8>,
		},
		/// An account has been granted tagged permissions
		TaggedDataPermissionsGranted {
			grantor: T::AccountId,
			grantee: T::AccountId,
			permission: DataPermission,
			tags: Vec<Vec<u8>>,
			expiry: Option<BlockNumberFor<T>>,
			irrevocable: bool,
		},
		/// One of the tagged permissions for an account has been revoked
		TaggedDataPermissionsRevoked {
			revoker: T::AccountId,
			grantee: T::AccountId,
			permission: DataPermission,
			tags: Vec<Vec<u8>>,
		},
		/// An account has been granted a permission reference
		PermissionReferenceGranted {
			grantor: T::AccountId,
			grantee: T::AccountId,
			permission_record_id: Vec<u8>,
		},
		/// An account's permission reference has been revoked
		PermissionReferenceRevoked {
			grantor: T::AccountId,
			grantee: T::AccountId,
			permission_record_id: Vec<u8>,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight({
			T::WeightInfo::grant_data_permissions(data_ids.len() as u32)
		})]
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

			let block = <frame_system::Pallet<T>>::block_number();

			let permission_record =
				PermissionRecord { grantor: who.clone(), permission, block, expiry, irrevocable };

			if irrevocable {
				ensure!(expiry.is_none(), Error::<T>::IrrevocableCannotBeExpirable);
			}

			for data_id in data_ids.iter() {
				let next_id = <NextPermissionRecordId<T>>::get(&data_author);

				let data_id = data_id.clone();

				ensure!(
					T::SyloDataVerificationProvider::get_validation_record(&data_author, &data_id)
						.is_some(),
					Error::<T>::DataRecordDoesNotExist
				);

				// if this permission is being granted by an account other than the
				// data author, then ensure the account has been granted the DISTRIBUTE
				// permission
				if who != data_author {
					ensure!(
						Self::has_permission(
							&data_author,
							&data_id,
							&who,
							DataPermission::DISTRIBUTE
						),
						Error::<T>::MissingDistributePermission
					);

					ensure!(
						permission != DataPermission::DISTRIBUTE,
						Error::<T>::CannotGrantDistributePermission
					);

					ensure!(!irrevocable, Error::<T>::CannotGrantIrrevocablePermission);
				}

				<PermissionRecords<T>>::try_mutate(
					(&data_author, &data_id, &grantee),
					|records| {
						records
							.try_push((next_id, permission_record.clone()))
							.map_err(|_| Error::<T>::ExceededMaxPermissions)
					},
				)?;

				<NextPermissionRecordId<T>>::mutate(&data_author, |i| *i += 1);

				Self::deposit_event(Event::DataPermissionGranted {
					data_author: who.clone(),
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
		#[pallet::weight({
			T::WeightInfo::revoke_data_permission()
		})]
		pub fn revoke_data_permission(
			origin: OriginFor<T>,
			data_author: T::AccountId,
			permission_id: u32,
			grantee: T::AccountId,
			data_id: DataId<T::StringLimit>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let records = <PermissionRecords<T>>::get((&data_author, &data_id, &grantee));

			let (_, permission_record) = records
				.iter()
				.find(|(id, _)| *id == permission_id)
				.ok_or(Error::<T>::PermissionNotFound)?;

			ensure!(!permission_record.irrevocable, Error::<T>::PermissionIrrevocable);

			// a distributor can only revoke permissions they have granted themselves,
			// though a data author can always revoke a permission
			ensure!(
				permission_record.grantor == who || who == data_author,
				Error::<T>::NotPermissionGrantor
			);

			<PermissionRecords<T>>::mutate((&data_author, &data_id, &grantee), |records| {
				records.retain(|(id, _)| *id != permission_id)
			});

			Self::deposit_event(Event::DataPermissionRevoked {
				revoker: who.clone(),
				grantee: grantee.clone(),
				permission: permission_record.permission,
				data_id: data_id.clone().to_vec(),
			});

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight({
			T::WeightInfo::grant_tagged_permissions(tags.len() as u32)
		})]
		pub fn grant_tagged_permissions(
			origin: OriginFor<T>,
			grantee: T::AccountId,
			permission: DataPermission,
			tags: BoundedVec<BoundedVec<u8, T::StringLimit>, T::MaxTags>,
			expiry: Option<BlockNumberFor<T>>,
			irrevocable: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let tagged_permission_record = TaggedPermissionRecord {
				permission,
				block: <frame_system::Pallet<T>>::block_number(),
				tags: tags.clone(),
				expiry,
				irrevocable,
			};

			let next_id = <NextPermissionRecordId<T>>::get(&who);

			<TaggedPermissionRecords<T>>::try_mutate(
				&who,
				&grantee,
				|tagged_permission_records| {
					tagged_permission_records
						.try_push((next_id, tagged_permission_record))
						.map_err(|_| Error::<T>::ExceededMaxPermissions)
				},
			)?;

			<NextPermissionRecordId<T>>::mutate(&who, |i| *i += 1);

			Self::deposit_event(Event::TaggedDataPermissionsGranted {
				grantor: who,
				grantee,
				permission,
				tags: tags.iter().map(|v| v.to_vec()).collect(),
				expiry,
				irrevocable,
			});

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight({
			T::WeightInfo::revoke_tagged_permissions()
		})]
		pub fn revoke_tagged_permissions(
			origin: OriginFor<T>,
			grantee: T::AccountId,
			permission_id: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let records = <TaggedPermissionRecords<T>>::get(&who, &grantee);

			let (_, tagged_permission_record) = records
				.iter()
				.find(|(id, _)| *id == permission_id)
				.ok_or(Error::<T>::PermissionNotFound)?;

			ensure!(!tagged_permission_record.irrevocable, Error::<T>::PermissionIrrevocable);

			<TaggedPermissionRecords<T>>::mutate(&who, &grantee, |tagged_permission_records| {
				tagged_permission_records.retain(|(id, _)| *id != permission_id)
			});

			Self::deposit_event(Event::TaggedDataPermissionsRevoked {
				revoker: who,
				grantee,
				permission: tagged_permission_record.permission,
				tags: tagged_permission_record.tags.iter().map(|v| v.to_vec()).collect(),
			});

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight({
			T::WeightInfo::grant_permission_reference()
		})]
		pub fn grant_permission_reference(
			origin: OriginFor<T>,
			grantee: T::AccountId,
			permission_record_id: BoundedVec<u8, T::StringLimit>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(
				T::SyloDataVerificationProvider::get_validation_record(&who, &permission_record_id)
					.is_some(),
				Error::<T>::MissingValidationRecord,
			);

			<PermissionReferences<T>>::insert(
				&who,
				&grantee,
				PermissionReference { permission_record_id: permission_record_id.clone() },
			);

			Self::deposit_event(Event::PermissionReferenceGranted {
				grantor: who.clone(),
				grantee: grantee.clone(),
				permission_record_id: permission_record_id.to_vec(),
			});

			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight({
			T::WeightInfo::revoke_permission_reference()
		})]
		pub fn revoke_permission_reference(
			origin: OriginFor<T>,
			grantee: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let record = <PermissionReferences<T>>::get(&who, &grantee)
				.ok_or(Error::<T>::PermissionNotFound)?;

			<PermissionReferences<T>>::remove(&who, &grantee);

			Self::deposit_event(Event::PermissionReferenceRevoked {
				grantor: who.clone(),
				grantee: grantee.clone(),
				permission_record_id: record.permission_record_id.to_vec(),
			});

			Ok(())
		}
	}
}

impl<T: Config> SyloDataPermissionsProvider for Pallet<T> {
	type AccountId = T::AccountId;
	type StringLimit = T::StringLimit;

	fn has_permission(
		data_author: &T::AccountId,
		data_id: &DataId<T::StringLimit>,
		grantee: &T::AccountId,
		permission: DataPermission,
	) -> bool {
		// both the MODIFY and DISTRIBUTE permissions also imply
		// having the VIEW permission
		let is_sufficient_permission =
			|p: DataPermission| p == permission || permission == DataPermission::VIEW;

		let block = <frame_system::Pallet<T>>::block_number();

		let permissions = <PermissionRecords<T>>::get((data_author, data_id, grantee));

		// try find a direct permission that is valid
		let has_direct_permission = permissions
			.iter()
			.find(|(_, record)| {
				// check for expiry
				if let Some(expiry) = record.expiry {
					if expiry <= block {
						return false;
					}
				}

				return is_sufficient_permission(record.permission);
			})
			.is_some();

		if has_direct_permission {
			return true;
		}

		// check for tagged permissions
		if let Some(verification_record) =
			T::SyloDataVerificationProvider::get_validation_record(&data_author, &data_id)
		{
			let tagged_permissions = <TaggedPermissionRecords<T>>::get(data_author, grantee);

			return tagged_permissions
				.iter()
				.find(|(_, record)| {
					// check for expiry
					if let Some(expiry) = record.expiry {
						if expiry <= block {
							return false;
						}
					}

					// check if any of the tags in the permission record
					// matches any of the tags in data record
					return record
						.tags
						.iter()
						.find(|permission_tag| {
							verification_record
								.tags
								.iter()
								.find(|record_tag| permission_tag == record_tag)
								.is_some()
						})
						.is_some() && is_sufficient_permission(record.permission);
				})
				.is_some();
		}

		false
	}
}
