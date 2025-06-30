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
use seed_pallet_common::ExtrinsicChecker;
use seed_primitives::Balance;
use sp_core::{H160, U256};
use sp_runtime::traits::StaticLookup;
use sp_runtime::BoundedBTreeSet;
use sp_std::vec;

pub mod types;
pub use types::*;

pub mod weights;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
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
		<Self as frame_system::Config>::AccountId: From<H160> + Into<H160>,
	{
		/// The overarching call type.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			+ GetDispatchInfo
			+ GetCallMetadata
			+ From<frame_system::Call<Self>>
			+ IsSubType<Call<Self>>
			+ IsType<<Self as frame_system::Config>::RuntimeCall>;

		/// Provides a way to check if a call is blacklisted.
		type BlacklistedCallProvider: ExtrinsicChecker<
			Call = <Self as pallet::Config>::RuntimeCall,
			Extra = (),
			Result = bool,
		>;

		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Interface to access weight values
		type WeightInfo: WeightInfo;

		/// The maximum number of modules allowed in a transact permission.
		#[pallet::constant]
		type MaxCallIds: Get<u32>;

		/// The maximum number of modules allowed in a transact permission.
		#[pallet::constant]
		type StringLimit: Get<u32>;

		/// The maximum bounded length for the XRPL signed message/transaction.
		#[pallet::constant]
		type XrplMaxMessageLength: Get<u32>;

		/// The maximum bounded length for the XRPL signature.
		#[pallet::constant]
		type XrplMaxSignatureLength: Get<u32>;

		/// A lookup to get the futurepass account id for a futurepass holder.
		type FuturepassLookup: StaticLookup<Source = H160, Target = H160>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The permission does not exist or has not been granted.
		PermissionNotGranted,
		/// The call is not authorized under the granted permission.
		NotAuthorizedCall,
		/// The permission has expired and is no longer valid.
		PermissionExpired,
		/// The provided expiry block is in the past.
		InvalidExpiry,
		/// A permission already exists and has not yet expired.
		PermissionAlreadyExists,
		/// The specified spending balance is not allowed.
		InvalidSpendingBalance,
		/// The provided token signature is invalid or cannot be verified.
		InvalidTokenSignature,
		/// The grantee in the token does not match the caller.
		GranteeDoesNotMatch,
		/// The nonce provided in the token has already been used.
		NonceAlreadyUsed,
		/// The futurepass in the token is not owned by the grantor.
		InvalidFuturepassInToken,
		/// The spending balance is insufficient to cover the transaction fee.
		InsufficientSpendingBalance,
	}

	/// Holds transact permission records.
	#[pallet::storage]
	pub type TransactPermissions<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId, // Grantor
		Blake2_128Concat,
		T::AccountId, // Grantee
		TransactPermission<BlockNumberFor<T>, T::MaxCallIds, T::StringLimit>,
		OptionQuery,
	>;

	/// Nonces that have already been used for token signatures.
	#[pallet::storage]
	pub type TokenSignatureNonces<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		U256, // Nonce
		bool,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config>
	where
		<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
		<T as frame_system::Config>::AccountId: From<H160> + Into<H160>,
	{
		/// A transact permission was granted.
		TransactPermissionGranted {
			grantor: T::AccountId,
			grantee: T::AccountId,
			spender: Spender,
			spending_balance: Option<Balance>,
			allowed_calls: Vec<CallId<T::StringLimit>>,
			expiry: Option<BlockNumberFor<T>>,
		},
		/// A permissioned transaction was executed.
		PermissionTransactExecuted { grantor: T::AccountId, grantee: T::AccountId },
		/// A transact permission was updated.
		TransactPermissionUpdated {
			grantor: T::AccountId,
			grantee: T::AccountId,
			spender: Spender,
			spending_balance: Option<Balance>,
			allowed_calls: Vec<CallId<T::StringLimit>>,
			expiry: Option<BlockNumberFor<T>>,
		},
		/// A transact permission was revoked.
		TransactPermissionRevoked { grantor: T::AccountId, grantee: T::AccountId },
		/// A transact permission was accepted.
		TransactPermissionAccepted { grantor: T::AccountId, grantee: T::AccountId },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
		<T as frame_system::Config>::AccountId: From<H160> + Into<H160>,
	{
		/// Will create ands store a new transact permission to for the given
		/// grantor and grantee.
		///
		/// Results in an error if the permission already exists and has not expired.
		/// If the spending balance is specified, the spender must be the grantor.
		#[pallet::call_index(0)]
		#[pallet::weight({
			T::WeightInfo::grant_transact_permission(allowed_calls.len() as u32)
		})]
		pub fn grant_transact_permission(
			origin: OriginFor<T>,
			grantee: T::AccountId,
			spender: Spender,
			spending_balance: Option<Balance>,
			allowed_calls: BoundedBTreeSet<CallId<T::StringLimit>, T::MaxCallIds>,
			expiry: Option<BlockNumberFor<T>>,
		) -> DispatchResult {
			let grantor = ensure_signed(origin)?;

			let block = frame_system::Pallet::<T>::block_number();

			// Check if a non-expired permission already exists
			if let Some(existing_permission) = TransactPermissions::<T>::get(&grantor, &grantee) {
				if let Some(existing_expiry) = existing_permission.expiry {
					ensure!(block > existing_expiry, Error::<T>::PermissionAlreadyExists);
				} else {
					Err(Error::<T>::PermissionAlreadyExists)?;
				}
			}

			Self::do_grant_transact_permission(
				grantor,
				grantee,
				spender,
				spending_balance,
				allowed_calls,
				expiry,
			)
		}

		/// Updates an existing transact permission record.
		///
		/// This must be called by the grantor of the permission.
		#[pallet::call_index(1)]
		#[pallet::weight({
			T::WeightInfo::update_transact_permission(allowed_calls.as_ref().map(|a| a.len() as u32).unwrap_or(0))
		})]
		pub fn update_transact_permission(
			origin: OriginFor<T>,
			grantee: T::AccountId,
			spender: Option<Spender>,
			spending_balance: Option<Option<Balance>>,
			allowed_calls: Option<BoundedBTreeSet<CallId<T::StringLimit>, T::MaxCallIds>>,
			expiry: Option<Option<BlockNumberFor<T>>>,
		) -> DispatchResult {
			let grantor = ensure_signed(origin)?;

			// Update the permission record
			TransactPermissions::<T>::try_mutate(&grantor, &grantee, |permission| {
				let permission_record =
					permission.as_mut().ok_or(Error::<T>::PermissionNotGranted)?;

				// Update spender if provided
				if let Some(new_spender) = spender {
					permission_record.spender = new_spender;
				}

				// Ensure spending_balance is only specified if spender is Grantor
				if let Some(Some(_)) = spending_balance {
					ensure!(
						matches!(permission_record.spender, Spender::GRANTOR),
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
				Self::deposit_event(Event::TransactPermissionUpdated {
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

		/// Revokes a transact permission for a grantee.
		///
		/// This must be called by the grantor of the permission.
		#[pallet::call_index(2)]
		#[pallet::weight({
			T::WeightInfo::revoke_transact_permission()
		})]
		pub fn revoke_transact_permission(
			origin: OriginFor<T>,
			grantee: T::AccountId,
		) -> DispatchResult {
			let grantor = ensure_signed(origin)?;

			// Remove the permission if it exists
			let removed = TransactPermissions::<T>::take(&grantor, &grantee);
			ensure!(removed.is_some(), Error::<T>::PermissionNotGranted);

			Self::deposit_event(Event::TransactPermissionRevoked { grantor, grantee });

			Ok(())
		}

		/// Accepts a transact permission using a signed token.
		///
		/// This is intended to by called by the grantee of the permission. The caller
		/// must provide a permission token that contains the details of the permission,
		/// which has also been signed by the grantor.
		///
		/// Accepts a signature either in the form of a EIP191 or XRPL signed message.
		#[pallet::call_index(3)]
		#[pallet::weight({
			T::WeightInfo::accept_transact_permission()
		})]
		pub fn accept_transact_permission(
			origin: OriginFor<T>,
			permission_token: TransactPermissionToken<
				T::AccountId,
				BlockNumberFor<T>,
				T::MaxCallIds,
				T::StringLimit,
			>,
			token_signature: TransactPermissionTokenSignature<
				T::XrplMaxMessageLength,
				T::XrplMaxSignatureLength,
			>,
		) -> DispatchResult {
			let grantee = ensure_signed(origin)?;

			// Verify the signature
			let token_signer = token_signature
				.verify_signature(&permission_token)
				.map_err(|_| Error::<T>::InvalidTokenSignature)?;

			let mut grantor = token_signer;

			// Check if the futurepass field is specified, and if the grantor
			// is the owner of the futurepass, set the grantor to the futurepass account
			if permission_token.use_futurepass {
				let futurepass = T::FuturepassLookup::lookup(grantor.clone().into())
					.map_err(|_| Error::<T>::InvalidFuturepassInToken)?;

				grantor = futurepass.into();
			}

			// Ensure the origin is the grantee
			ensure!(grantee == permission_token.grantee, Error::<T>::GranteeDoesNotMatch);

			// Validate the nonce
			ensure!(
				!TokenSignatureNonces::<T>::contains_key(permission_token.nonce),
				Error::<T>::NonceAlreadyUsed
			);

			// Grant the transact permission.
			// This will overwrite any existing permission allowing the grantor/grantee
			// to update the permission by calling this again
			Self::do_grant_transact_permission(
				grantor.clone(),
				grantee.clone(),
				permission_token.spender,
				permission_token.spending_balance,
				permission_token.allowed_calls,
				permission_token.expiry,
			)?;

			// Mark the nonce as used
			TokenSignatureNonces::<T>::insert(permission_token.nonce, true);

			// Emit event
			Self::deposit_event(Event::TransactPermissionAccepted { grantor, grantee });

			Ok(())
		}

		/// Executes a permissioned transaction on behalf of the grantor.
		#[pallet::call_index(4)]
		#[pallet::weight({
			let dispatch_info = call.get_dispatch_info();
			T::WeightInfo::transact().saturating_add(dispatch_info.weight)
		})]
		pub fn transact(
			origin: OriginFor<T>,
			grantor: T::AccountId,
			call: Box<<T as Config>::RuntimeCall>,
		) -> DispatchResult {
			let grantee = ensure_signed(origin.clone())?;

			let permission_record = TransactPermissions::<T>::get(&grantor, &grantee)
				.ok_or(Error::<T>::PermissionNotGranted)?;

			// Check if the permission has expired
			if let Some(expiry) = permission_record.expiry {
				let current_block = frame_system::Pallet::<T>::block_number();
				ensure!(current_block <= expiry, Error::<T>::PermissionExpired);
			}

			ensure!(
				Self::is_call_allowed(&*call, permission_record.allowed_calls.clone()),
				Error::<T>::NotAuthorizedCall
			);

			// Dispatch the call directly
			call.dispatch(frame_system::RawOrigin::Signed(grantor.clone()).into())
				.map_err(|e| e.error)?;

			// Emit event
			Self::deposit_event(Event::PermissionTransactExecuted { grantor, grantee });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
		<T as frame_system::Config>::AccountId: From<H160> + Into<H160>,
	{
		fn is_call_allowed(
			call: &<T as Config>::RuntimeCall,
			allowed_calls: BoundedBTreeSet<CallId<T::StringLimit>, T::MaxCallIds>,
		) -> bool {
			// Deny if the call is blacklisted
			if T::BlacklistedCallProvider::check_extrinsic(call, &()) {
				return false;
			}

			let CallMetadata { function_name, pallet_name } = call.get_call_metadata();

			let pallet_name: BoundedVec<u8, T::StringLimit> =
				BoundedVec::truncate_from(pallet_name.as_bytes().to_ascii_lowercase());
			let function_name: BoundedVec<u8, T::StringLimit> =
				BoundedVec::truncate_from(function_name.as_bytes().to_ascii_lowercase());

			let wildcard: BoundedVec<u8, T::StringLimit> = BoundedVec::truncate_from(b"*".to_vec());

			// Check if the call is allowed
			allowed_calls.iter().any(|(pallet, function)| {
				if pallet == &pallet_name || pallet == &wildcard {
					if function == &function_name || function == &wildcard {
						return true;
					}
				}

				false
			})
		}

		pub fn do_grant_transact_permission(
			grantor: T::AccountId,
			grantee: T::AccountId,
			spender: Spender,
			spending_balance: Option<Balance>,
			allowed_calls: BoundedBTreeSet<CallId<T::StringLimit>, T::MaxCallIds>,
			expiry: Option<BlockNumberFor<T>>,
		) -> DispatchResult {
			// Ensure spending_balance is only specified if spender is Grantor
			if let Some(_) = spending_balance {
				ensure!(matches!(spender, Spender::GRANTOR), Error::<T>::InvalidSpendingBalance);
			}

			let block = frame_system::Pallet::<T>::block_number();

			// Ensure expiry is not in the past
			if let Some(expiry_block) = expiry {
				ensure!(expiry_block >= block, Error::<T>::InvalidExpiry);
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

			let permission_record = TransactPermission {
				spender,
				spending_balance,
				allowed_calls: normalized_allowed_calls.clone(),
				block,
				expiry,
			};

			TransactPermissions::<T>::insert(&grantor, &grantee, permission_record);

			// Emit event
			Self::deposit_event(Event::TransactPermissionGranted {
				grantor,
				grantee,
				spender,
				spending_balance,
				allowed_calls: normalized_allowed_calls.into_iter().collect(),
				expiry,
			});

			Ok(())
		}

		pub fn validate_spending_balance<'a>(
			grantor: &'a T::AccountId,
			grantee: &'a T::AccountId,
			fee: Balance,
		) -> Result<&'a T::AccountId, Error<T>> {
			let mut spender = grantee;
			TransactPermissions::<T>::try_mutate(grantor, grantee, |maybe_permission| {
				if let Some(permission) = maybe_permission {
					if permission.spender == Spender::GRANTOR {
						// Set the spender as the grantor of the permission. This allows the
						// fee to be deducted from the grantor's balance.
						spender = grantor;

						// if the fee payer is the grantor, we should detract from
						// the spending balance if it exists
						if let Some(spending_balance) = permission.spending_balance.as_mut() {
							if *spending_balance < fee.into() {
								return Err(Error::<T>::InsufficientSpendingBalance);
							}

							*spending_balance = spending_balance.saturating_sub(fee.into());
						}
					}
				}

				Ok::<(), Error<T>>(())
			})?;

			Ok(spender)
		}
	}
}
