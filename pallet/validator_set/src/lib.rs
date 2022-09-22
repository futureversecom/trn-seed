#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod helpers;

use crate::helpers::{ConsensusLog, EventProofId, PendingAuthorityChange, ValidatorSet};
use frame_support::{pallet_prelude::*, traits::OneSessionHandler, PalletId};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use seed_pallet_common::{log, FinalSessionTracker as FinalSessionTrackerT};
use sp_core::H160;
use sp_runtime::{
	traits::AccountIdConversion, BoundToRuntimeAppPublic, DigestItem, Percent, RuntimeAppPublic,
};
use sp_std::vec::Vec;

pub type ValidatorIdOf<T> = <T as Config>::ValidatorId;
pub(crate) const LOG_TARGET: &str = "validator_set";
pub const ENGINE_ID: sp_runtime::ConsensusEngineId = *b"EGN-";

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Allowed origins to add/removw the validator
		type ApproveOrigin: EnsureOrigin<Self::Origin>;

		/// The identifier type for an authority in this module (i.e. active validator session key)
		/// 33 byte ECDSA public key
		type ValidatorId: Member
			+ Parameter
			+ AsRef<[u8]>
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize;
		/// Reports the final session of na eras
		type FinalSessionTracker: FinalSessionTrackerT;
		/// The pallet bridge address (destination for incoming messages, source for outgoing)
		type BridgePalletId: Get<PalletId>;
		/// The bridge contract address
		type BridgeContractAddress: Get<H160>;
		/// The threshold of notarizations required to approve an event
		type NotarizationThreshold: Get<Percent>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	#[pallet::storage]
	#[pallet::getter(fn validator_list)]
	/// List of all the validators
	pub type ValidatorList<T: Config> = StorageValue<_, Vec<T::ValidatorId>, ValueQuery>;

	// The pallet's runtime storage items.
	#[pallet::storage]
	#[pallet::getter(fn next_validator_list)]
	/// Next List of all the validators
	pub type NextValidatorList<T: Config> = StorageValue<_, Vec<T::ValidatorId>, ValueQuery>;

	// The pallet's runtime storage items.
	#[pallet::storage]
	#[pallet::getter(fn validator_list_set_id)]
	/// Current validators set id
	pub type ValidatorListSetId<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn bridge_paused)]
	/// Current validators set id
	pub type BridgePaused<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn notary_set_id)]
	/// Current validators set id
	pub type NotarySetId<T: Config> = StorageValue<_, EventProofId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn notary_set_proof_id)]
	/// Current validators set id
	pub type NotarySetProofId<T: Config> = StorageValue<_, EventProofId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn next_event_proof_id)]
	/// Current validators set id
	pub type NextEventProofId<T: Config> = StorageValue<_, EventProofId, ValueQuery>;

	// The pallet's runtime storage items.
	#[pallet::storage]
	#[pallet::getter(fn white_list_validators)]
	/// List of all the white list validators
	pub type WhiteListValidators<T: Config> = StorageMap<_, Blake2_128Concat, T::ValidatorId, bool>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New validator added
		/// parameters. [who]
		ValidatorAdded(T::ValidatorId),

		/// Validator removed
		/// parameters. [who]
		ValidatorRemoved(T::ValidatorId),
		/// A notary (validator) set change is in motion (event_id, new_validator_set_id)
		/// A proof for the change will be generated with the given `event_id`
		AuthoritySetChange(EventProofId, u64),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Validator is already in the validator set.
		DuplicateValidator,
		/// Validator not found in the validator set.
		ValidatorNotFound,
		/// The bridge is paused pending validator set changes (once every era / 24 hours)
		/// It will reactive after ~10 minutes
		BridgePaused,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		/// The initial validator set.
		pub initial_validators: Vec<T::ValidatorId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { initial_validators: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::initialize_validators(&self.initial_validators);
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add a new validator to the set.
		/// New validator's session keys should be set in Session pallet before
		/// calling this.
		/// Emits an event on success else error
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(0,1))]
		pub fn add_validator(
			origin: OriginFor<T>,
			validator_id: T::ValidatorId,
		) -> DispatchResultWithPostInfo {
			// Check if the sender is an approved origin or not
			T::ApproveOrigin::ensure_origin(origin)?;
			if <WhiteListValidators<T>>::contains_key(&validator_id) {
				Err(Error::<T>::DuplicateValidator.into())
			} else {
				<WhiteListValidators<T>>::insert(&validator_id, true);
				Self::deposit_event(Event::ValidatorAdded(validator_id));
				Ok(().into())
			}
		}

		/// Remove a validator from the set.
		/// Emits an event on success else error
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn remove_validator(
			origin: OriginFor<T>,
			validator_id: T::ValidatorId,
		) -> DispatchResultWithPostInfo {
			// Check if the sender is an approved origin or not
			T::ApproveOrigin::ensure_origin(origin)?;
			if <WhiteListValidators<T>>::contains_key(&validator_id) {
				<WhiteListValidators<T>>::remove(&validator_id);
				Self::deposit_event(Event::ValidatorRemoved(validator_id));
				Ok(().into())
			} else {
				Err(Error::<T>::ValidatorNotFound.into())
			}
		}
	}
}

impl<T: Config> Pallet<T> {
	#[allow(dead_code)]
	fn initialize_validators(validator_list: &[T::ValidatorId]) {
		// Add the initial validator_list to WhiteList Validators.
		for validator in validator_list {
			<WhiteListValidators<T>>::insert(validator, true);
		}
	}

	// To check if the given member is validator or not
	pub fn is_validator(validator_id: &T::ValidatorId) -> bool {
		WhiteListValidators::<T>::get(validator_id).unwrap_or(false)
	}

	fn update_validators(validator_list: Vec<T::ValidatorId>) {
		// Filter validator_list from WhiteList Validators.
		let mut validators: Vec<T::ValidatorId> = Vec::new();
		for validator in validator_list {
			if <WhiteListValidators<T>>::contains_key(&validator) {
				validators.push(validator);
				//ValidatorList::<T>::append(validator);
			}
		}
		<ValidatorList<T>>::put(validators);
	}

	/// Handle changes to the authority set
	/// This could be called when validators rotate their keys, we don't want to
	/// change this until the era has changed to avoid generating proofs for small set changes or
	/// too frequently
	/// - `new`: The validator set that is active right now
	/// - `queued`: The validator set that will activate next session
	pub(crate) fn handle_authorities_change(new: Vec<T::ValidatorId>, queued: Vec<T::ValidatorId>) {
		// ### Session life cycle
		// block on_initialize if ShouldEndSession(n)
		//  rotate_session
		//    before_end_session
		//    end_session (end just been)
		//    start_session (start now)
		//    new_session (start now + 1)
		//   -> on_new_session <- this function is CALLED here

		let log_notary_change = |next_keys: &[T::ValidatorId]| {
			// Store the keys for usage next session
			<NextValidatorList<T>>::put(next_keys);
			// Signal the Event Id that will be used for the proof of validator set change.
			// Any observer can subscribe to this event and submit the resulting proof to keep the
			// validator set on the bridge contract updated.
			let event_proof_id = <NextEventProofId<T>>::get();
			let next_validator_set_id = Self::notary_set_id().wrapping_add(1);
			Self::deposit_event(Event::<T>::AuthoritySetChange(
				event_proof_id,
				next_validator_set_id,
			));
			<NotarySetProofId<T>>::put(event_proof_id);
			<NextEventProofId<T>>::put(event_proof_id.wrapping_add(1));
			let log: DigestItem = DigestItem::Consensus(
				ENGINE_ID,
				ConsensusLog::PendingAuthoritiesChange(PendingAuthorityChange {
					source: T::BridgePalletId::get().into_account_truncating(),
					destination: T::BridgeContractAddress::get().into(),
					next_validator_set: ValidatorSet {
						validators: next_keys.to_vec(),
						id: next_validator_set_id,
						proof_threshold: T::NotarizationThreshold::get()
							.mul_ceil(next_keys.len() as u32),
					},
					event_proof_id,
				})
				.encode(),
			);
			<frame_system::Pallet<T>>::deposit_log(log);
		};

		// signal 1 session early about the `queued` validator set change for the next era so
		// there's time to generate a proof
		if T::FinalSessionTracker::is_next_session_final() {
			log!(trace, "💎 next session final");
			log_notary_change(queued.as_ref());
		} else if T::FinalSessionTracker::is_active_session_final() {
			// Pause bridge claim/proofs
			// Prevents claims/proofs being partially processed and failing if the validator set
			// changes significantly
			// Note: the bridge will be reactivated at the end of the session
			log!(trace, "💎 active session final");
			<BridgePaused<T>>::put(true);

			if Self::next_validator_list().is_empty() {
				// if we're here the era was forced, we need to generate a proof asap
				log!(warn, "💎 urgent notary key rotation");
				log_notary_change(new.as_ref());
			}
		}
	}
}

impl<T: Config> ValidatorWhiteList<ValidatorIdOf<T>> for Pallet<T> {
	fn is_validator(validator_id: &T::ValidatorId) -> bool {
		Self::is_validator(validator_id)
	}
}

pub trait ValidatorWhiteList<ValidatorId> {
	fn is_validator(validator_id: &ValidatorId) -> bool;
}

impl<T: Config> BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::ValidatorId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = T::ValidatorId;

	fn on_genesis_session<'a, I: 'a>(validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::ValidatorId)>,
	{
		let keys = validators.map(|x| x.1).collect::<Vec<_>>();
		if !keys.is_empty() {
			assert!(
				ValidatorList::<T>::decode_len().is_none(),
				"Validator List is already initialized!"
			);
			Self::update_validators(keys);
		}
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::ValidatorId)>,
	{
		// Only run change process at the end of an era
		if T::FinalSessionTracker::is_next_session_final() ||
			T::FinalSessionTracker::is_active_session_final()
		{
			// Record authorities for the new session.
			let next_authorities = validators.map(|(_, k)| k).collect::<Vec<_>>();
			let next_queued_authorities = queued_validators.map(|(_, k)| k).collect::<Vec<_>>();

			Self::handle_authorities_change(next_authorities, next_queued_authorities);
		}
	}

	/// A notification for end of the session.
	///
	/// Note it is triggered before any [`SessionManager::end_session`] handlers,
	/// so we can still affect the validator set.
	fn on_before_session_ending() {
		// Re-activate the bridge, allowing claims & proofs again
		if T::FinalSessionTracker::is_active_session_final() {
			log!(trace, "💎 session & era ending, set new validator keys");
			// A proof should've been generated now so we can reactivate the bridge with the new
			// validator set
			<BridgePaused<T>>::kill();
			// Time to update the bridge validator keys.
			let next_notary_keys = NextValidatorList::<T>::take();
			// Store the new keys and increment the validator set id
			// Next notary keys should be unset, until populated by new session logic
			Self::update_validators(next_notary_keys);
			<ValidatorListSetId<T>>::mutate(|next_set_id| {
				*next_set_id = next_set_id.wrapping_add(1)
			});
		}
	}

	fn on_disabled(_i: u32) {}
}
