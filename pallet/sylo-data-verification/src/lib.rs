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
use seed_primitives::AssetId;
use sp_core::H256;
use sp_std::prelude::*;
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

		/// Allowed origins to set payment asset and reversed sylo method
		type ApproveOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Interface to access weight values
		type WeightInfo: WeightInfo;

		/// The maximim number of resolvers in a validation record.
		#[pallet::constant]
		type MaxResolvers: Get<u32>;

		/// The maximum number of tags in a validation record.
		#[pallet::constant]
		type MaxTags: Get<u32>;

		/// The maximum number of validation entries in a record.
		#[pallet::constant]
		type MaxEntries: Get<u32>;

		/// The maximum number of service endpoints for a registered resolver.
		#[pallet::constant]
		type MaxServiceEndpoints: Get<u32>;

		/// The max length of strings used within the Sylo Pallet. This limits
		/// the maximum size for resolver identifiers, data identifier, service
		/// endpoint strings, and tag strings.
		#[pallet::constant]
		type StringLimit: Get<u32>;
	}

	/// The default string used as the reserved method for sylo resolvers
	#[pallet::type_value]
	pub fn DefaultReservedSyloResolvedMethod<T: Config>() -> BoundedVec<u8, T::StringLimit> {
		BoundedVec::truncate_from(b"sylo-data".to_vec())
	}

	#[pallet::storage]
	pub type SyloAssetId<T: Config> = StorageValue<_, AssetId, OptionQuery>;

	#[pallet::storage]
	pub type SyloResolverMethod<T: Config> = StorageValue<
		_,
		BoundedVec<u8, T::StringLimit>,
		ValueQuery,
		DefaultReservedSyloResolvedMethod<T>,
	>;

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
		DataId<T::StringLimit>,
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
		NoValidationRecord,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The asset used to for extrinsics has been set
		PaymentAssetSet { asset_id: AssetId },
		/// The string reserved for the method used by sylo resolvers has been set
		SyloResolverMethodSet { method: Vec<u8> },
		/// A new resolver has been registered and set in storage
		ResolverRegistered {
			id: Vec<u8>,
			controller: T::AccountId,
			service_endpoints: BoundedVec<BoundedVec<u8, T::StringLimit>, T::MaxServiceEndpoints>,
		},
		/// An existing resolver has had it's service endpoints updated
		ResolverUpdated {
			id: Vec<u8>,
			controller: T::AccountId,
			service_endpoints: BoundedVec<BoundedVec<u8, T::StringLimit>, T::MaxServiceEndpoints>,
		},
		/// An existing resolver has been deregistered and removed from storage
		ResolverDeregistered { id: Vec<u8> },
		/// A new validation record has been created and set in storage
		ValidationRecordCreated { author: T::AccountId, id: Vec<u8> },
		/// An entry of an existing validation record has been added
		ValidationEntryAdded { author: T::AccountId, id: Vec<u8>, checksum: H256 },
		/// An existing validation record has had its fields updated
		ValidationRecordUpdated {
			author: T::AccountId,
			id: Vec<u8>,
			resolvers: Option<Vec<Vec<u8>>>,
			data_type: Option<Vec<u8>>,
			tags: Option<Vec<Vec<u8>>>,
		},
		/// An existing validation record has been deleted and removed from
		/// storage
		ValidationRecordDeleted { author: T::AccountId, id: Vec<u8> },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set the asset used to pay for sylo extrinsics.
		///
		/// This operation requires root access.
		#[pallet::call_index(0)]
		#[pallet::weight({
			T::WeightInfo::set_payment_asset()
		})]
		pub fn set_payment_asset(origin: OriginFor<T>, payment_asset: AssetId) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;
			<SyloAssetId<T>>::put(payment_asset);
			Self::deposit_event(Event::PaymentAssetSet { asset_id: payment_asset });
			Ok(())
		}

		/// Set the string used as the reserved sylo resolver method.
		///
		/// This operation requires root access.
		#[pallet::call_index(1)]
		#[pallet::weight({
			T::WeightInfo::set_sylo_resolver_method()
		})]
		pub fn set_sylo_resolver_method(
			origin: OriginFor<T>,
			resolver_method: BoundedVec<u8, T::StringLimit>,
		) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;
			<SyloResolverMethod<T>>::put(&resolver_method);
			Self::deposit_event(Event::SyloResolverMethodSet { method: resolver_method.to_vec() });
			Ok(())
		}

		/// Register a new resolver.
		///
		/// The caller will be set as the controller of the resolver.
		#[pallet::call_index(2)]
		#[pallet::weight({
			T::WeightInfo::register_resolver(service_endpoints.len() as u32)
		})]
		pub fn register_resolver(
			origin: OriginFor<T>,
			identifier: BoundedVec<u8, T::StringLimit>,
			service_endpoints: BoundedVec<ServiceEndpoint<T::StringLimit>, T::MaxServiceEndpoints>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(
				!<Resolvers<T>>::contains_key(&identifier),
				Error::<T>::ResolverAlreadyRegistered
			);

			let resolver =
				Resolver { controller: who.clone(), service_endpoints: service_endpoints.clone() };

			<Resolvers<T>>::insert(&identifier, resolver);

			Self::deposit_event(Event::ResolverRegistered {
				id: identifier.to_vec(),
				controller: who,
				service_endpoints,
			});

			Ok(())
		}

		/// Update the the service endpoints of an existing the resolver.
		///
		/// Caller must be the controller of the resolver.
		#[pallet::call_index(3)]
		#[pallet::weight({
			T::WeightInfo::update_resolver(service_endpoints.len() as u32)
		})]
		pub fn update_resolver(
			origin: OriginFor<T>,
			identifier: BoundedVec<u8, T::StringLimit>,
			service_endpoints: BoundedVec<ServiceEndpoint<T::StringLimit>, T::MaxServiceEndpoints>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			<Resolvers<T>>::try_mutate(&identifier, |resolver| -> DispatchResult {
				let resolver = resolver.as_mut().ok_or(Error::<T>::ResolverNotRegistered)?;

				ensure!(who == resolver.controller, Error::<T>::NotController);

				resolver.service_endpoints = service_endpoints.clone();

				Self::deposit_event(Event::ResolverUpdated {
					id: identifier.to_vec(),
					controller: who,
					service_endpoints,
				});

				Ok(())
			})?;

			Ok(())
		}

		/// Deregister an existing resolver.
		///
		/// Caller must be the controller of the resolver.
		#[pallet::call_index(4)]
		#[pallet::weight({
			T::WeightInfo::deregister_resolver()
		})]
		pub fn deregister_resolver(
			origin: OriginFor<T>,
			identifier: BoundedVec<u8, T::StringLimit>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let resolver =
				<Resolvers<T>>::get(&identifier).ok_or(Error::<T>::ResolverNotRegistered)?;

			ensure!(who == resolver.controller, Error::<T>::NotController);

			<Resolvers<T>>::remove(&identifier);

			Self::deposit_event(Event::ResolverDeregistered { id: identifier.to_vec() });

			Ok(())
		}

		/// Create a new validation record.
		///
		/// The caller will be set as the record's author.
		///
		/// For any specified resolvers which use the reserved sylo resolver
		/// method, those resolvers must already be registered and exist in storage.
		///
		/// The initial record entry will use the current system block for the
		/// block value.
		#[pallet::call_index(5)]
		#[pallet::weight({
			T::WeightInfo::create_validation_record(resolvers.len() as u32, tags.len() as u32)
		})]
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
				!<ValidationRecords<T>>::contains_key(&who, &data_id),
				Error::<T>::RecordAlreadyCreated
			);

			Self::validate_sylo_resolvers(&resolvers)?;

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

			<ValidationRecords<T>>::insert(&who, &data_id, record);

			Self::deposit_event(Event::ValidationRecordCreated {
				author: who,
				id: data_id.to_vec(),
			});

			Ok(())
		}

		/// Add a new entry to an existing validation record.
		///
		/// The current block will be used as the entry's block number.
		///
		/// Caller must be the author of the record.
		#[pallet::call_index(6)]
		#[pallet::weight({
			T::WeightInfo::add_validation_record_entry()
		})]
		pub fn add_validation_record_entry(
			origin: OriginFor<T>,
			data_id: BoundedVec<u8, T::StringLimit>,
			checksum: H256,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			<ValidationRecords<T>>::try_mutate(&who, &data_id, |record| -> DispatchResult {
				let record = record.as_mut().ok_or(Error::<T>::NoValidationRecord)?;

				record.entries.force_push(ValidationEntry {
					checksum,
					block: <frame_system::Pallet<T>>::block_number(),
				});

				Self::deposit_event(Event::ValidationEntryAdded {
					author: who.clone(),
					id: data_id.to_vec(),
					checksum,
				});

				Ok(())
			})?;

			Ok(())
		}

		/// Update a validation record's fields. The call takes in an Option
		/// value for the fields: resolvers, data_type, and tags.
		///
		/// Setting those fields to Some value will update the field in storage,
		/// whilst setting to None will be a no-op.
		///
		/// Caller must be the author of the record.
		#[pallet::call_index(7)]
		#[pallet::weight({
			T::WeightInfo::update_validation_record(
				resolvers.as_ref().map_or(0, |v| v.len() as u32),
				tags.as_ref().map_or(0, |v| v.len() as u32)
			)
		})]
		pub fn update_validation_record(
			origin: OriginFor<T>,
			data_id: BoundedVec<u8, T::StringLimit>,
			resolvers: Option<BoundedVec<ResolverId<T::StringLimit>, T::MaxResolvers>>,
			data_type: Option<BoundedVec<u8, T::StringLimit>>,
			tags: Option<BoundedVec<BoundedVec<u8, T::StringLimit>, T::MaxTags>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			<ValidationRecords<T>>::try_mutate(&who, &data_id, |record| -> DispatchResult {
				let record = record.as_mut().ok_or(Error::<T>::NoValidationRecord)?;

				if let Some(ref new_resolvers) = resolvers {
					Self::validate_sylo_resolvers(new_resolvers)?;
					record.resolvers = new_resolvers.clone();
				}

				if let Some(ref new_data_type) = data_type {
					record.data_type = new_data_type.clone();
				}

				if let Some(ref new_tags) = tags {
					record.tags = new_tags.clone();
				}

				Self::deposit_event(Event::ValidationRecordUpdated {
					author: who.clone(),
					id: data_id.to_vec(),
					resolvers: resolvers
						.map(|r| r.iter().map(|resolver| resolver.to_did()).collect()),
					data_type: data_type.map(|dt| dt.to_vec()),
					tags: tags.map(|t| t.iter().map(|tag| tag.to_vec()).collect()),
				});

				Ok(())
			})?;

			Ok(())
		}

		/// Delete an existing validation record.
		///
		/// Caller must be the author of the record.
		#[pallet::call_index(8)]
		#[pallet::weight({
			T::WeightInfo::delete_validation_record()
		})]
		pub fn delete_validation_record(
			origin: OriginFor<T>,
			data_id: BoundedVec<u8, T::StringLimit>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(
				<ValidationRecords<T>>::contains_key(&who, &data_id),
				Error::<T>::NoValidationRecord
			);

			<ValidationRecords<T>>::remove(&who, &data_id);

			Self::deposit_event(Event::ValidationRecordDeleted {
				author: who,
				id: data_id.to_vec(),
			});

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn validate_sylo_resolvers(
			resolvers: &BoundedVec<ResolverId<T::StringLimit>, T::MaxResolvers>,
		) -> DispatchResult {
			let reserved_method = <SyloResolverMethod<T>>::get();

			// Ensure any sylo data resolvers are already registered
			resolvers
				.iter()
				.filter(|resolver| resolver.method == reserved_method)
				.try_for_each(|resolver| -> DispatchResult {
					ensure!(
						<Resolvers<T>>::contains_key(&resolver.identifier),
						Error::<T>::ResolverNotRegistered
					);
					Ok(())
				})?;

			Ok(())
		}
	}
}
