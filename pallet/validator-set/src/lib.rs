/* Copyright 2021 Centrality Investments Limited
 *
 * Licensed under the LGPL, Version 3.0 (the "License");
 * you may not use this file except in compliance with the License.
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * You may obtain a copy of the License at the root of this project source code,
 * or at:
 *     https://centrality.ai/licenses/gplv3.txt
 *     https://centrality.ai/licenses/lgplv3.txt
 */
#![cfg_attr(not(feature = "std"), no_std)]

use codec::Encode;
use frame_support::codec::{Decode, MaxEncodedLen};
use frame_support::{ensure, fail, traits::Get, weights::Weight, BoundedVec, PalletId};
use frame_support::traits::OneSessionHandler;
use frame_support::traits::schedule::DispatchTime;
use frame_support::traits::schedule::Anon;
pub use pallet::*;
use seed_pallet_common::{EthereumBridge, EthereumEventSubscriber, log, FinalSessionTracker as FinalSessionTrackerT,};
use seed_primitives::{CollectionUuid, SerialNumber};
use sp_core::{H160, U256};
use sp_runtime::{traits::AccountIdConversion, DispatchError, SaturatedConversion};
use sp_runtime::traits::Saturating;
use sp_std::{boxed::Box, vec, vec::Vec};

/// The logging target for this pallet
pub(crate) const LOG_TARGET: &str = "ethy";
pub(crate) const SCHEDULER_PRIORITY: u8 = 63;


#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, transactional};
	use frame_support::traits::schedule::Anon;
	use frame_system::{ensure_signed, pallet_prelude::*};
	use frame_system::offchain::CreateSignedTransaction;
	use sp_runtime::RuntimeAppPublic;
	use seed_primitives::AccountId;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> + CreateSignedTransaction<Call<Self>> {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type PalletId: Get<PalletId>;
		/// The identifier type for an authority in this module (i.e. active validator session key)
		/// 33 byte secp256k1 public key
		type EthyId: Member
		+ Parameter
		+ AsRef<[u8]>
		+ RuntimeAppPublic
		+ Ord
		+ MaybeSerializeDeserialize
		+ MaxEncodedLen;
		/// The duration in blocks of one epoch
		type EpochDuration: Get<u64>;
		/// Length of time the bridge will be paused while the authority set changes
		type AuthorityChangeDelay: Get<Self::BlockNumber>;
		/// Reports the final session of na eras
		type FinalSessionTracker: FinalSessionTrackerT;
		/// The Scheduler.
		type Scheduler: Anon<Self::BlockNumber, <Self as Config>::Call, Self::PalletsOrigin>;
		/// The runtime call type.
		type Call: From<Call<Self>>;
		/// Overarching type of all pallets origins.
		type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>;
	}

	#[pallet::storage]
	#[pallet::getter(fn next_notary_keys)]
	/// Scheduled notary (validator) public keys for the next session
	pub type NextNotaryKeys<T: Config> = StorageValue<_, Vec<T::EthyId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn notary_keys)]
	/// Active notary (validator) public keys
	pub type NotaryKeys<T: Config> =  StorageValue<_, Vec<T::EthyId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn notary_xrpl_keys)]
	/// Active xrpl notary (validator) public keys
	pub type NotaryXrplKeys<T: Config> =  StorageValue<_, Vec<T::EthyId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn next_authority_change)]
	/// The block in which we process the next authority change
	pub type NextAuthorityChange<T: Config> = StorageValue<_, T::BlockNumber>;

	#[pallet::storage]
	#[pallet::getter(fn authorities_changed_this_era)]
	/// Flag to indicate whether authorities have been changed during the current era
	pub type AuthoritiesChangedThisEra<T> = StorageValue<_, bool, ValueQuery>;


	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The schedule to unpause the bridge has failed (scheduled_block)
		FinaliseScheduleFail(T::BlockNumber),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> where
		<T as frame_system::Config>::AccountId: From<sp_core::H160> + Into<sp_core::H160>
	{
	}
}

impl<T: Config> Pallet<T> where <T as frame_system::Config>::AccountId: From<sp_core::H160> {}

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::EthyId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = T::EthyId;

	fn on_genesis_session<'a, I: 'a>(validators: I)
		where
			I: Iterator<Item = (&'a T::AccountId, T::EthyId)>,
	{
		let keys = validators.map(|x| x.1).collect::<Vec<_>>();
		if !keys.is_empty() {
			assert!(NotaryKeys::<T>::decode_len().is_none(), "NotaryKeys are already initialized!");
			NotaryKeys::<T>::put(&keys);
			// Self::update_xrpl_notary_keys(&keys);
		}
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, _validators: I, queued_validators: I)
		where
			I: Iterator<Item = (&'a T::AccountId, T::EthyId)>,
	{
		// Store the keys for usage next session
		let next_queued_authorities = queued_validators.map(|(_, k)| k).collect::<Vec<_>>();
		<NextNotaryKeys<T>>::put(next_queued_authorities);

		if T::FinalSessionTracker::is_active_session_final() {
			// Next authority change is 5 minutes before this session ends
			// (Just before the start of the next epoch)
			// next_block = current_block + epoch_duration - AuthorityChangeDelay
			let epoch_duration: T::BlockNumber = T::EpochDuration::get().saturated_into();
			let next_block: T::BlockNumber = <frame_system::Pallet<T>>::block_number()
				.saturating_add(epoch_duration.saturating_sub(T::AuthorityChangeDelay::get()));
			<NextAuthorityChange<T>>::put(next_block);
		}
	}

	/// A notification for end of the session.
	///
	/// Note it is triggered before any [`SessionManager::end_session`] handlers,
	/// so we can still affect the validator set.
	fn on_before_session_ending() {
		// Re-activate the bridge, allowing claims & proofs again
		if T::FinalSessionTracker::is_active_session_final() {
			// Get the next_notary_keys for the next era
			let next_notary_keys = NextNotaryKeys::<T>::get();

			if !Self::authorities_changed_this_era() {
				// The authorities haven't been changed yet
				// This could be due to a new era being forced before the final session
				// Self::handle_authorities_change();

				// // Schedule an un-pausing of the bridge to give the relayer time to relay the
				// // authority set change.
				// let scheduled_block =
				// 	<frame_system::Pallet<T>>::block_number() + T::AuthorityChangeDelay::get();
				// if T::Scheduler::schedule(
				// 	DispatchTime::At(scheduled_block),
				// 	None,
				// 	SCHEDULER_PRIORITY,
				// 	frame_system::RawOrigin::None.into(),
				// 	Call::finalise_authorities_change { next_notary_keys }.into(),
				// )
				// 	.is_err()
				// {
				// 	// The scheduler failed for some reason, throw a log and event
				// 	Self::deposit_event(Event::<T>::FinaliseScheduleFail(scheduled_block));
				// 	log!(warn, "💎 Unpause bridge schedule failed");
				// }
			} else {
				// Authorities have been changed, finalise those changes immediately
				// Self::do_finalise_authorities_change(next_notary_keys);
			}
		}
	}
	fn on_disabled(_i: u32) {}
}