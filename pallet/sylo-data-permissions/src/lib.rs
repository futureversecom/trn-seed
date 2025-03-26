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
		PermissionRecord<BlockNumberFor<T>>,
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
		/// Attempted to grant permissions for a data record when not the author
		NotAuthor,
		/// Attempted to grant permission for a data record when a DISTRIBUTION
		/// permission does not exist
		NoDistributionPermissionExists,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An account has been granted permission for a given data
		DataPermissionGranted {
			grantor: T::AccountId,
			grantee: T::AccountId,
			data_id: Vec<u8>,
			permission: DataPermission,
			expiry: Option<BlockNumberFor<T>>,
		},
		/// An account's permission has been revoked for a given data
		DataPermissionRevoked {
			grantor: T::AccountId,
			grantee: T::AccountId,
			data_id: Vec<u8>,
			expiry: Option<BlockNumberFor<T>>,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(1_000)]
		pub fn grant_data_permissions(
			origin: OriginFor<T>,
			grantee: T::AccountId,
			data_ids: BoundedVec<DataId<T::StringLimit>, T::MaxPermissions>,
			permission: DataPermission,
			expiry: Option<BlockNumberFor<T>>,
			irrevocable: Option<bool>,
		) -> DispatchResult {
			Ok(())
		}
	}
}
