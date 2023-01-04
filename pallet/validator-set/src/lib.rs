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
use ethabi::Token;
use frame_support::codec::{Decode, MaxEncodedLen};
use frame_support::{ensure, fail, traits::Get, weights::Weight, BoundedVec, PalletId};
use frame_support::metadata::StorageEntryModifier::Default;
use frame_support::pallet_prelude::DispatchResult;
use frame_support::traits::OneSessionHandler;
use frame_support::traits::schedule::DispatchTime;
use frame_support::traits::schedule::Anon;
use frame_system::ensure_none;
use frame_system::pallet_prelude::OriginFor;
use log::{debug, info, trace, warn, error};
pub use pallet::*;
use seed_pallet_common::{EthereumBridge, EthereumEventSubscriber, log, FinalSessionTracker as FinalSessionTrackerT};
use seed_primitives::{CollectionUuid, EthyEcdsaToEthereum, EthyEcdsaToXRPLAccountId, SerialNumber};
use sp_core::{H160, U256};
use sp_runtime::{traits::AccountIdConversion, DispatchError, SaturatedConversion, DigestItem};
use sp_runtime::traits::Saturating;
use sp_std::{boxed::Box, vec, vec::Vec};
use pallet_ethy2::types::Log;
use seed_pallet_common::ethy::State::{Active, Paused};
use seed_pallet_common::validator_set::{ValidatorSetChangeHandler, ValidatorSetChangeInfo};
use seed_pallet_common::xrpl::XRPLBridgeAdapter;
use seed_primitives::ethy::{ConsensusLog, ETHY_ENGINE_ID, EventProofId, ValidatorSet, ValidatorSetId};

/// The logging target for this pallet
pub(crate) const LOG_TARGET: &str = "validator-set";
pub(crate) const SCHEDULER_PRIORITY: u8 = 63;


#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, transactional};
	use frame_support::traits::schedule::Anon;
	use frame_system::{ensure_signed, pallet_prelude::*};
	use frame_system::offchain::CreateSignedTransaction;
	use sp_runtime::RuntimeAppPublic;
	use seed_pallet_common::validator_set::{ValidatorSetChangeHandler, ValidatorSetChangeInfo};
	use seed_pallet_common::xrpl::XRPLBridgeAdapter;
	use seed_primitives::{AccountId, ValidatorId};

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
		/// Length of time the bridge will be paused while the validator set changes
		type ValidatorChangeDelay: Get<Self::BlockNumber>;
		/// Reports the final session of na eras
		type FinalSessionTracker: FinalSessionTrackerT;
		/// The Scheduler.
		type Scheduler: Anon<Self::BlockNumber, <Self as Config>::Call, Self::PalletsOrigin>;
		/// The runtime call type.
		type Call: From<Call<Self>>;
		/// Overarching type of all pallets origins.
		type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>;
		/// Max Xrpl notary (validator) public keys
		type MaxXrplKeys: Get<u8>;
		/// ethy adapter
		type EthyAdapter: ValidatorSetChangeHandler<Self::EthyId>;
		/// XRPL Bridge adapter
		type XRPLBridgeAdapter: XRPLBridgeAdapter<Self::EthyId>;
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

	/// The current validator set id
	#[pallet::storage]
	#[pallet::getter(fn notary_set_id)]
	pub type NotarySetId<T> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn next_validator_set_change)]
	/// The block in which we process the next validator set change
	pub type NextValidatorSetChange<T: Config> = StorageValue<_, T::BlockNumber>;

	#[pallet::storage]
	#[pallet::getter(fn validators_changed_this_era)]
	/// Flag to indicate whether authorities have been changed during the current era
	pub type ValidatorsChangedThisEra<T> = StorageValue<_, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn xrpl_door_signers)]
	/// XRPL Door Signers set by sudo (white listed validators)
	pub type XrplDoorSigners<T: Config> =  StorageMap<_, Twox64Concat,T::EthyId, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn notary_set_proof_id)]
	/// The event proof Id generated by the previous validator set to notarize the current full validator set.
	/// Useful for syncing the latest proof to Ethereum
	pub type NotarySetProofId<T> = StorageValue<_,EventProofId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn xrpl_notary_set_proof_id)]
	/// The event proof Id generated by the previous xrpl validator set to notarize the current xrpl validator set.
	/// Useful for syncing the latest proof to Xrpl
	pub type XrplNotarySetProofId<T> =  StorageValue<_,EventProofId, ValueQuery>;


	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Validator set change successful
		ValidatorSetChangeFinalizeSuccess { validator_set_id: ValidatorSetId },
		/// Validator set change failed
		ValidatorSetChangeFinalizeFailed { validator_set_id: ValidatorSetId },
		/// Validator set change finalize scheduling failed
		ValidatorSetFinalizeSchedulingFailed{ scheduled_at: T::BlockNumber } ,
		/// XRPL notary keys update failed
		XRPLNoratyUpdateFailed { validator_set_id: ValidatorSetId },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> where
		<T as frame_system::Config>::AccountId: From<sp_core::H160> + Into<sp_core::H160>
	{
		/// Finalise validator set change, unpauses bridge and sets new notary keys
		/// Called internally after force new era
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn finalise_validator_set_change(origin: OriginFor<T>, next_notary_keys: Vec<T::EthyId>) -> DispatchResult {
			ensure_none(origin)?;
			Self::do_finalise_validator_set_change(next_notary_keys)
		}
	}
}

impl<T: Config> Pallet<T> {
	pub(crate) fn update_xrpl_notary_keys(validator_list: &Vec<T::EthyId>) -> Result<(), DispatchError> {
		let validators = Self::get_xrpl_notary_keys(validator_list)?;
		<NotaryXrplKeys<T>>::put(&validators);
		Ok(())
	}

	/// Iterate through the given validator_list and extracts the first number of MaxXrplKeys that are in the XrplDoorSigners
	pub(crate) fn get_xrpl_notary_keys(validator_list: &Vec<T::EthyId>) -> Result<Vec<T::EthyId>, DispatchError> {
		let xrpl_door_signers = T::XRPLBridgeAdapter::get_door_signers()?;
		let xrpl_notary_keys = validator_list
			.into_iter()
			.filter(|validator| xrpl_door_signers.contains(&validator))
			.map(|validator| -> T::EthyId { validator.clone() })
			.take(T::MaxXrplKeys::get().into())
			.collect();
		Ok(xrpl_notary_keys)
	}

	/// Handle changes to the validator set
	/// This will be called ValidatorChangeDelay(5) minutes before the end of an era when a natural era change happens.
	/// It should give the bridge enough time to update the keys at the remote side/blockchain
	/// This could also be called at the end of an era when doing a forced era. In this case the actual Notary keys update will be
	/// delayed by ValidatorChangeDelay to give the bridge enough time to update the remote side
	pub(crate) fn handle_validator_set_change() {
		let next_keys = NextNotaryKeys::<T>::get();
		let next_validator_set_id = Self::notary_set_id().wrapping_add(1);
		debug!(target: LOG_TARGET, "handling validator set change for validator set id {:?}", next_validator_set_id);

		let info = ValidatorSetChangeInfo {
			current_validator_set_id: Self::notary_set_id(),
			current_validator_set: Self::notary_keys(),
			next_validator_set_id: next_validator_set_id,
			next_validator_set: next_keys,
		};
		// let know the ethy
		T::EthyAdapter::validator_set_change_in_progress(info);
	}

	/// Finalise validator set changes
	pub fn do_finalise_validator_set_change(next_notary_keys: Vec<T::EthyId>) -> Result<(), DispatchError> {
		info!(target: LOG_TARGET, "finalise validator set change");
		// A proof should've been generated now so we can reactivate the bridge with the new
		// validator set
		ValidatorsChangedThisEra::<T>::kill();
		// Store the new keys and increment the validator set id
		// Next notary keys should be unset, until populated by new session logic
		<NotaryKeys<T>>::put(&next_notary_keys);
		Self::update_xrpl_notary_keys(&next_notary_keys)?;
		NotarySetId::<T>::mutate(|next_set_id| *next_set_id = next_set_id.wrapping_add(1));
		Ok(())
	}

}

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
			if let Err(e) = Self::update_xrpl_notary_keys(&keys) {
				Self::deposit_event(
					Event::<T>::XRPLNoratyUpdateFailed{ validator_set_id: Self::notary_set_id().wrapping_add(1).into() }
				);
				error!( target: LOG_TARGET, "Update XRPL notary keys failed. error: {:?}", Into::<&str>::into(e));
			}
		}
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, _validators: I, queued_validators: I)
		where
			I: Iterator<Item = (&'a T::AccountId, T::EthyId)>,
	{
		// Store the keys for usage next session
		let next_queued_validators = queued_validators.map(|(_, k)| k).collect::<Vec<_>>();
		<NextNotaryKeys<T>>::put(next_queued_validators);

		if T::FinalSessionTracker::is_active_session_final() {
			// Next authority change is AuthorityChangeDelay(5 minutes) before this session ends
			// (Just before the start of the next epoch)
			let epoch_duration: T::BlockNumber = T::EpochDuration::get().saturated_into();
			let next_block: T::BlockNumber = <frame_system::Pallet<T>>::block_number()
				.saturating_add(epoch_duration.saturating_sub(T::ValidatorChangeDelay::get()));
			<NextValidatorSetChange<T>>::put(next_block);
		}
	}

	/// A notification for end of the session.
	///
	/// Note it is triggered before any [`SessionManager::end_session`] handlers,
	/// so we can still affect the validator set.
	fn on_before_session_ending() {
		if !T::FinalSessionTracker::is_active_session_final() {
			// no work here. return
			return
		}
		// Get the next_notary_keys for the next era
		let next_notary_keys = NextNotaryKeys::<T>::get();
		if !Self::validators_changed_this_era() {
			// The validator set change haven't been started yet
			// This could be due to a new era being forced before the final session
			Self::handle_validator_set_change();
			// Schedule the finalizing of the new validator set into the future to give the ethy a sufficient time
			let scheduled_block =
				<frame_system::Pallet<T>>::block_number() + T::ValidatorChangeDelay::get();
			if T::Scheduler::schedule(
				DispatchTime::At(scheduled_block),
				None,
				SCHEDULER_PRIORITY,
				frame_system::RawOrigin::None.into(),
				Call::finalise_validator_set_change { next_notary_keys }.into(),
			)
				.is_err()
			{
				// The scheduler failed for some reason, throw an event and log
				Self::deposit_event(Event::<T>::ValidatorSetFinalizeSchedulingFailed { scheduled_at: scheduled_block });
				error!(target: LOG_TARGET, "Scheduling finalize validator set change failed");
			}
		} else {
			// validators change have been started, finalise the changes
			match Self::do_finalise_validator_set_change(next_notary_keys) {
				Ok(_) => {
					Self::deposit_event(
						Event::<T>::ValidatorSetChangeFinalizeSuccess{ validator_set_id: Self::notary_set_id() }
					);
					info!(target: LOG_TARGET, "Validator set change finalize successful. set Id: {:?}", Self::notary_set_id());
				},
				Err(e) => {
					Self::deposit_event(
						Event::<T>::ValidatorSetChangeFinalizeFailed{ validator_set_id: Self::notary_set_id()}
					);
					error!(target: LOG_TARGET, "Validator set change finalize failed. set Id: {:?}, error: {:?}", Self::notary_set_id(), Into::<&str>::into(e));
				}
			}
		}
	}
	fn on_disabled(_i: u32) {}
}