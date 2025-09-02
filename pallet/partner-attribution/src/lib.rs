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

use alloc::vec::Vec;
use frame_support::{pallet_prelude::*, sp_runtime::Permill, transactional};
use frame_system::pallet_prelude::*;
use seed_pallet_common::{AttributionProvider, FuturepassProvider};
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

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Allowed origin to perform privileged calls
		type ApproveOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		/// Ensure origin is a valid Futurepass account
		type EnsureFuturepass: EnsureOrigin<Self::RuntimeOrigin, Success = H160>;
		/// The futurepass creation interface
		type FuturepassCreator: FuturepassProvider<AccountId = Self::AccountId>;
		/// Interface to access weight values
		type WeightInfo: WeightInfo;
		#[cfg(feature = "runtime-benchmarks")]
		/// Handles a multi-currency fungible asset system for benchmarking.
		type MultiCurrency: frame_support::traits::fungibles::Inspect<
				Self::AccountId,
				AssetId = seed_primitives::AssetId,
			> + frame_support::traits::fungibles::Mutate<Self::AccountId>;
		/// The maximum number of partners
		type MaxPartners: Get<u32>;
	}

	#[pallet::type_value]
	pub fn DefaultValue() -> u128 {
		1
	}

	/// The next available partner id
	#[pallet::storage]
	pub type NextPartnerId<T> = StorageValue<_, u128, ValueQuery, DefaultValue>;

	/// Current number of partners
	#[pallet::storage]
	pub type PartnerCount<T> = StorageValue<_, u32, ValueQuery>;

	/// Partner information
	#[pallet::storage]
	pub type Partners<T: Config> =
		StorageMap<_, Twox64Concat, u128, PartnerInformation<T::AccountId>>;

	/// User-partner attributions
	#[pallet::storage]
	pub type Attributions<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, u128>;

	/// Admin account for Attribution Percentage operations
	#[pallet::storage]
	pub(super) type AdminAccount<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

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
		/// Admin Account changed
		AdminAccountChanged {
			old_key: Option<T::AccountId>,
			new_key: T::AccountId,
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
		/// Caller is not a futurepass account
		CallerNotFuturepass,
		/// Account already attributed to another partner
		AccountAlreadyAttributed,
		/// Maximum number of partners exceeded
		MaxPartnersExceeded,
		/// Caller must be admin account
		RequireAdmin,
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

			// Ensure we don't exceed the maximum number of partners
			ensure!(
				PartnerCount::<T>::get() < T::MaxPartners::get(),
				Error::<T>::MaxPartnersExceeded
			);

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

			// Increment partner count
			PartnerCount::<T>::mutate(|count| *count = count.saturating_add(1));

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
		/// - `partner_account`: Updates the partner's account
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::update_partner_account())]
		pub fn update_partner_account(
			origin: OriginFor<T>,
			#[pallet::compact] partner_id: u128,
			partner_account: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Get partner and verify ownership in one read
			let partner = Partners::<T>::get(partner_id).ok_or(Error::<T>::PartnerNotFound)?;
			ensure!(partner.owner == who, Error::<T>::Unauthorized);

			// Update partner account
			Partners::<T>::mutate(partner_id, |maybe_partner| {
				if let Some(partner) = maybe_partner {
					partner.account = partner_account.clone();
				}
			});
			Self::deposit_event(Event::PartnerUpdated { partner_id, account: partner_account });

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
			Self::ensure_root_or_admin(origin)?;

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

		/// Create a futurepass account and attribute it to a partner permanently
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// Parameters:
		/// - `partner_id`: The partner id to attribute the account to.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::create_futurepass_with_partner())]
		#[transactional]
		pub fn create_futurepass_with_partner(
			origin: OriginFor<T>,
			partner_id: u128,
			account: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Ensure partner exists
			ensure!(Partners::<T>::contains_key(partner_id), Error::<T>::PartnerNotFound);

			// Create the futurepass account
			let futurepass = T::FuturepassCreator::create_futurepass(who.clone(), account)?;

			// Ensure account is not already attributed
			ensure!(
				!Attributions::<T>::contains_key(&futurepass),
				Error::<T>::AccountAlreadyAttributed
			);

			// Attribute the new futurepass account to the partner
			Attributions::<T>::insert(&futurepass, partner_id);

			Self::deposit_event(Event::AccountAttributed { partner_id, account: futurepass });

			Ok(())
		}

		/// Remove a partner (privileged call)
		///
		/// This is a privileged call that can only be called by an authorized futureverse account.
		///
		/// Parameters:
		/// - `partner_id`: The partner id to remove.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::remove_partner())]
		pub fn remove_partner(
			origin: OriginFor<T>,
			#[pallet::compact] partner_id: u128,
		) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;

			// Ensure partner exists
			let partner = Partners::<T>::get(partner_id).ok_or(Error::<T>::PartnerNotFound)?;

			// Remove the partner
			Partners::<T>::remove(partner_id);

			// Decrement partner count
			PartnerCount::<T>::mutate(|count| *count = count.saturating_sub(1));

			Self::deposit_event(Event::PartnerRemoved { partner_id, account: partner.account });

			Ok(())
		}

		/// Set the admin account for DEX operations
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::set_admin())]
		pub fn set_admin(origin: OriginFor<T>, new: T::AccountId) -> DispatchResult {
			Self::ensure_root_or_admin(origin)?;

			let old_key = AdminAccount::<T>::get();
			AdminAccount::<T>::put(new.clone());
			Self::deposit_event(Event::AdminAccountChanged { old_key, new_key: new });
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn ensure_root_or_admin(origin: OriginFor<T>) -> Result<Option<T::AccountId>, DispatchError> {
		match ensure_signed_or_root(origin)? {
			Some(who) => {
				ensure!(
					AdminAccount::<T>::get().map_or(false, |k| who == k),
					Error::<T>::RequireAdmin
				);
				Ok(Some(who))
			},
			None => Ok(None),
		}
	}
}

impl<T: Config> AttributionProvider<T::AccountId> for Pallet<T> {
	fn get_attributions() -> Vec<(T::AccountId, Balance, Option<Permill>)> {
		Partners::<T>::iter()
			.filter(|(_id, partner)| {
				partner.fee_percentage.is_some() && partner.accumulated_fees != 0
			})
			.map(|(_id, partner)| {
				(partner.account.clone(), partner.accumulated_fees, partner.fee_percentage)
			})
			.collect()
	}

	fn reset_balances() {
		Partners::<T>::iter_keys().for_each(|id| {
			Partners::<T>::mutate(id, |maybe_partner| {
				if let Some(partner) = maybe_partner {
					partner.accumulated_fees = 0;
				}
			});
		});
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn set_attributions(attributions: Vec<(T::AccountId, Balance, Option<Permill>)>) {
		// Clear existing partners first
		let _ = Partners::<T>::clear(1000, None);
		NextPartnerId::<T>::put(1);
		PartnerCount::<T>::put(0);

		// Set up new partners from the provided attributions
		for (account, accumulated_fees, fee_percentage) in attributions.clone() {
			let partner_id = NextPartnerId::<T>::mutate(|id| {
				let current_id = *id;
				*id = id.saturating_add(1);
				current_id
			});

			let partner = PartnerInformation::<T::AccountId> {
				owner: account.clone(),
				account,
				fee_percentage,
				accumulated_fees,
			};
			Partners::<T>::insert(partner_id, partner);
		}

		// Update partner count
		PartnerCount::<T>::put(attributions.len() as u32);
	}
}
