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
use sp_core::{H160, H256, U256};
use sp_runtime::{
	traits::{AccountIdConversion, Zero},
	ArithmeticError, DispatchError, FixedU128, RuntimeDebug, SaturatedConversion,
};
use sp_std::{cmp::min, convert::TryInto, prelude::*, vec};

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
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Interface to access weight values
		type WeightInfo: WeightInfo;

		#[pallet::constant]
		type MaxResolvers: Get<u32>;

		#[pallet::constant]
		type MaxTags: Get<u32>;

		#[pallet::constant]
		type MaxEntries: Get<u32>;

		#[pallet::constant]
		type MaxServiceEndpoints: Get<u32>;

		#[pallet::constant]
		type StringLimit: Get<u32>;

		#[pallet::constant]
		type SyloResolverMethod: Get<[u8; 9]>;
	}

	#[pallet::storage]
	pub type Resolvers<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BoundedVec<u8, T::StringLimit>,
		Resolver<T::AccountId, T::MaxServiceEndpoints, T::StringLimit>,
	>;

	#[pallet::storage]
	pub type ValidationRecords<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		BoundedVec<u8, T::StringLimit>,
		ValidationRecord<
			T::AccountId,
			BlockNumberFor<T>,
			T::MaxResolvers,
			T::MaxTags,
			T::MaxEntries,
			T::StringLimit,
		>,
	>;

	#[pallet::error]
	pub enum Error<T> {
		/// The Resolver identifier is already in use
		ResolverAlreadyRegistered,
		/// The Resolver has not been registered
		ResolverNotRegistered,
		/// Account is not controller of resolver
		NotController,
		/// A validation record with the given data id has already been created
		RecordAlreadyCreated,
		/// The validation record to be updated has not been created
		RecordNotCreated,
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
		ValidationRecordCreated {
			author: T::AccountId,
			id: Vec<u8>,
		},
		ValidationEntryAdded {
			author: T::AccountId,
			id: Vec<u8>,
			checksum: H256,
		},
		ValidationRecordUpdated {
			author: T::AccountId,
			id: Vec<u8>,
			resolvers: Option<Vec<Vec<u8>>>,
			data_type: Option<Vec<u8>>,
			tags: Option<Vec<Vec<u8>>>,
		},
		ValidationRecordDeleted {
			author: T::AccountId,
			id: Vec<u8>,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight({
			T::WeightInfo::register_resolver(<T::StringLimit>::get(), <T::StringLimit>::get())
		})]
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

		#[pallet::call_index(3)]
		#[pallet::weight(1_000)]
		pub fn create_validation_record(
			origin: OriginFor<T>,
			data_id: BoundedVec<u8, T::StringLimit>,
			resolvers: BoundedVec<ResolverId<T::StringLimit>, T::MaxResolvers>,
			data_type: BoundedVec<u8, T::StringLimit>,
			tags: BoundedVec<BoundedVec<u8, T::StringLimit>, T::MaxTags>,
			checksum: H256,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(
				<ValidationRecords<T>>::get(who.clone(), data_id.clone()).is_none(),
				Error::<T>::RecordAlreadyCreated
			);

			Self::validate_sylo_resolvers(resolvers.clone())?;

			let current_block = <frame_system::Pallet<T>>::block_number();

			let record = ValidationRecord {
				author: who.clone(),
				resolvers,
				data_type,
				tags,
				entries: BoundedVec::truncate_from(vec![ValidationEntry {
					checksum,
					block: current_block,
				}]),
			};

			<ValidationRecords<T>>::insert(who.clone(), data_id.clone(), record);

			Self::deposit_event(Event::ValidationRecordCreated {
				author: who,
				id: data_id.to_vec(),
			});

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(1_000)]
		pub fn add_validation_record_entry(
			origin: OriginFor<T>,
			data_id: BoundedVec<u8, T::StringLimit>,
			checksum: H256,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let mut record = <ValidationRecords<T>>::get(who.clone(), data_id.clone())
				.ok_or(Error::<T>::RecordNotCreated)?;

			record.entries.force_push(ValidationEntry {
				checksum: checksum.clone(),
				block: <frame_system::Pallet<T>>::block_number(),
			});

			<ValidationRecords<T>>::insert(who.clone(), data_id.clone(), record);

			Self::deposit_event(Event::ValidationEntryAdded {
				author: who,
				id: data_id.to_vec(),
				checksum,
			});

			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(1_000)]
		pub fn update_validation_record(
			origin: OriginFor<T>,
			data_id: BoundedVec<u8, T::StringLimit>,
			resolvers: Option<BoundedVec<ResolverId<T::StringLimit>, T::MaxResolvers>>,
			data_type: Option<BoundedVec<u8, T::StringLimit>>,
			tags: Option<BoundedVec<BoundedVec<u8, T::StringLimit>, T::MaxTags>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let mut record = <ValidationRecords<T>>::get(who.clone(), data_id.clone())
				.ok_or(Error::<T>::RecordNotCreated)?;

			if let Some(resolvers) = resolvers.clone() {
				Self::validate_sylo_resolvers(resolvers.clone())?;
				record.resolvers = resolvers;
			}

			if let Some(data_type) = data_type.clone() {
				record.data_type = data_type;
			}

			if let Some(tags) = tags.clone() {
				record.tags = tags;
			}

			<ValidationRecords<T>>::insert(who.clone(), data_id.clone(), record);

			Self::deposit_event(Event::ValidationRecordUpdated {
				author: who,
				id: data_id.to_vec(),
				resolvers: resolvers
					.map(|resolvers| resolvers.iter().map(|resolver| resolver.to_did()).collect()),
				data_type: data_type.map(|data_type| data_type.to_vec()),
				tags: tags.map(|tags| tags.iter().map(|tag| tag.to_vec()).collect()),
			});

			Ok(())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(1_000)]
		pub fn delete_validation_record(
			origin: OriginFor<T>,
			data_id: BoundedVec<u8, T::StringLimit>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(
				<ValidationRecords<T>>::get(who.clone(), data_id.clone()).is_some(),
				Error::<T>::RecordNotCreated
			);

			<ValidationRecords<T>>::remove(who.clone(), data_id.clone());

			Self::deposit_event(Event::ValidationRecordDeleted {
				author: who,
				id: data_id.to_vec(),
			});

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn validate_sylo_resolvers(
			resolvers: BoundedVec<ResolverId<T::StringLimit>, T::MaxResolvers>,
		) -> DispatchResult {
			let reserved_method: BoundedVec<u8, T::StringLimit> =
				BoundedVec::try_from(<T as Config>::SyloResolverMethod::get().to_vec())
					.expect("Failed to convert invalid resolver method config");

			// Ensure any sylo data resolvers are already registered
			for resolver in resolvers {
				if resolver.method == reserved_method {
					ensure!(
						<Resolvers<T>>::get(resolver.identifier).is_some(),
						Error::<T>::ResolverNotRegistered
					);
				}
			}

			Ok(())
		}
	}
}
