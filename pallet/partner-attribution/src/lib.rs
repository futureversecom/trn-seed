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

//! # Partner Attribution pallet
//!
//! The pallet that stores the Futureverse partner attribution data in the runtime.
//! This pallet allows any account to register as a partner with a desired EOA address.
//! Other accounts can then be attributed to each partner.
//! This pallet will allow the managment (creation, update, deletion) of partners accounts.
//! This pallet will also allow management of accounts that want to be attributed to a partner.

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

pub use weights::WeightInfo;

/// The logging target for this pallet
#[allow(dead_code)]
pub(crate) const LOG_TARGET: &str = "partner_attribution";

#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub struct PartnerInformation<AccountId> {
	/// The owner of the partner account
	pub owner: AccountId,
	/// The partner account address to recieve attribution
	pub account: AccountId,
	/// The fee percentage to be paid to the partner
	pub fee_percentage: Option<Permill>,
	/// The accumulated fees by all accounts attributed to this partner
	pub accumulated_fees: Balance,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Allowed origins to ease transition to council givernance
		type ApproveOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		/// Ensure origin is a valid Futurepass account
		type EnsureFuturepass: EnsureOrigin<Self::RuntimeOrigin, Success = H160>;
		/// Interface to access weight values
		type WeightInfo: WeightInfo;
	}

	#[pallet::type_value]
	pub fn DefaultValue() -> u128 {
		1
	}

	/// The next available partner id
	#[pallet::storage]
	pub type NextPartnerId<T> = StorageValue<_, u128, ValueQuery, DefaultValue>;

	/// Partner information
	#[pallet::storage]
	pub type Partners<T: Config> =
		StorageMap<_, Twox64Concat, u128, PartnerInformation<T::AccountId>>;

	/// User-partner attributions
	#[pallet::storage]
	pub type Attributions<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, u128>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		PartnerRegistered {
			partner_id: u128,
			partner: PartnerInformation<T::AccountId>,
		},
		PartnerUpdated {
			partner_id: u128,
			account: T::AccountId,
		},
		PartnerRemoved {
			partner_id: u128,
			account: T::AccountId,
		},
		PartnerUpgraded {
			partner_id: u128,
			account: T::AccountId,
			fee_percentage: Permill,
		},
		AccountAttributed {
			partner_id: u128,
			account: T::AccountId,
		},
		AccountAttributionUpdated {
			old_partner_id: u128,
			new_partner_id: u128,
			account: T::AccountId,
		},
		AccountAttributionRemoved {
			partner_id: u128,
			account: T::AccountId,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// No available ids
		NoAvailableIds,
		/// Partner not found
		PartnerNotFound,
		/// Partner already exists
		PartnerAlreadyExists,
		/// Unauthorized
		Unauthorized,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_chain_id())]
		pub fn set_chain_id(
			origin: OriginFor<T>,
			#[pallet::compact] chain_id: u64,
		) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;
			ChainId::<T>::put(chain_id);
			Self::deposit_event(Event::<T>::ChainIdSet(chain_id));
			Ok(())
		}
	}
}
