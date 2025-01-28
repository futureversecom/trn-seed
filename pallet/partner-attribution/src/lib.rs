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
extern crate alloc;

pub use pallet::*;

use frame_support::{pallet_prelude::*, sp_runtime::Permill};
use frame_system::pallet_prelude::*;
use seed_primitives::Balance;
use sp_core::H160;

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
		PartnerRegistered { partner_id: u128, partner: PartnerInformation<T::AccountId> },
		PartnerUpdated { partner_id: u128, account: T::AccountId },
		PartnerRemoved { partner_id: u128, account: T::AccountId },
		PartnerUpgraded { partner_id: u128, account: T::AccountId, fee_percentage: Permill },
		AccountAttributed { partner_id: u128, account: T::AccountId },
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
		/// Caller is not a futurepass account
		CallerNotFuturepass,
		/// Account already attributed to another partner
		AccountAlreadyAttributed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register as a partner
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// Parameters:
		/// - `account`: The account to register as a partner.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::register_partner_account())]
		pub fn register_partner_account(
			origin: OriginFor<T>,
			account: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// increment the partner id, store it and use it
			let partner_id = NextPartnerId::<T>::mutate(|id| -> Result<u128, DispatchError> {
				let current_id = *id;
				*id = id.checked_add(1).ok_or(Error::<T>::NoAvailableIds)?;
				Ok(current_id)
			})?;

			let partner = PartnerInformation::<T::AccountId> {
				owner: who,
				account,
				fee_percentage: None,
				accumulated_fees: 0,
			};
			Partners::<T>::insert(partner_id, partner.clone());

			Self::deposit_event(Event::PartnerRegistered { partner_id, partner });
			Ok(())
		}

		/// Update or remove a partner account
		///
		/// The dispatch origin for this call must be _Signed_ and the caller must be the owner
		/// of the partner account.
		///
		/// Parameters:
		/// - `partner_id`: The ID of the partner to update
		/// - `partner_account`: If Some, updates the partner's account. If None, removes the partner entirely
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::update_partner_account())]
		pub fn update_partner_account(
			origin: OriginFor<T>,
			#[pallet::compact] partner_id: u128,
			partner_account: Option<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Get partner and verify ownership in one read
			let partner = Partners::<T>::get(partner_id).ok_or(Error::<T>::PartnerNotFound)?;
			ensure!(partner.owner == who, Error::<T>::Unauthorized);

			match partner_account {
				Some(account) => {
					// Update partner account
					Partners::<T>::mutate(partner_id, |maybe_partner| {
						if let Some(partner) = maybe_partner {
							partner.account = account.clone();
						}
					});
					Self::deposit_event(Event::PartnerUpdated { partner_id, account });
				},
				None => {
					// Remove partner entirely
					Partners::<T>::remove(partner_id);
					Self::deposit_event(Event::PartnerRemoved {
						partner_id,
						account: partner.account,
					});
				},
			}

			Ok(())
		}

		/// Attribute an account to a partner permanently
		///
		/// The dispatch origin for this call must be _Signed_.
		/// The dispatch origin must be a futurepass account.
		///
		/// Parameters:
		/// - `partner_id`: The partner id to attribute the account to.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::attribute_account())]
		pub fn attribute_account(origin: OriginFor<T>, partner_id: u128) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			// Ensure the caller is a futurepass account
			let _ = <T as Config>::EnsureFuturepass::try_origin(origin)
				.map_err(|_| Error::<T>::CallerNotFuturepass)?;

			// Ensure partner exists
			let _ = Partners::<T>::get(partner_id).ok_or(Error::<T>::PartnerNotFound)?;

			// Ensure the account is not already attributed to another partner
			if Attributions::<T>::get(&who).is_some() {
				return Err(Error::<T>::AccountAlreadyAttributed.into());
			}

			// Attribute the account to the partner
			Attributions::<T>::insert(&who, partner_id);
			Self::deposit_event(Event::AccountAttributed { partner_id, account: who });

			Ok(())
		}

		/// Update a partner account's fee percentage
		///
		/// This is a privileged call that can only be called by an authorized futureverse account.
		///
		/// Parameters:
		/// - `partner_id`: The partner id to update.
		/// - `fee_percentage`: The new fee percentage to set for the partner.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::upgrade_partner())]
		pub fn upgrade_partner(
			origin: OriginFor<T>,
			#[pallet::compact] partner_id: u128,
			#[pallet::compact] fee_percentage: Permill,
		) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;

			Partners::<T>::mutate(partner_id, |maybe_partner| {
				let Some(ref mut partner) = maybe_partner else {
					return Err(Error::<T>::PartnerNotFound);
				};
				partner.fee_percentage = Some(fee_percentage);
				Self::deposit_event(Event::PartnerUpgraded {
					partner_id,
					account: partner.clone().account,
					fee_percentage,
				});
				Ok(())
			})?;
			Ok(())
		}
	}
}
