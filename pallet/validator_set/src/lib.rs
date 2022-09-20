#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use frame_support::{pallet_prelude::*, traits::OneSessionHandler};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use seed_primitives::ValidatorId;
use sp_runtime::{ArithmeticError, BoundToRuntimeAppPublic};
use sp_std::vec::Vec;
use seed_pallet_common::{
	log,
	FinalSessionTracker as FinalSessionTrackerT,
};

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use sp_runtime::RuntimeAppPublic;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Allowed origins to add/removw the validator
		type ApproveOrigin: EnsureOrigin<Self::Origin>;

		/// The identifier type for an authority in this module (i.e. active validator session key)
		/// 33 byte ECDSA public key
		type XrpValidatorId: Member
			+ Parameter
			+ AsRef<[u8]>
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize;
		/// Reports the final session of na eras
		type FinalSessionTracker: FinalSessionTrackerT;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	#[pallet::storage]
	#[pallet::getter(fn validator_list)]
	/// List of all the validators
	pub type ValidatorList<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	// The pallet's runtime storage items.
	#[pallet::storage]
	#[pallet::getter(fn next_validator_list)]
	/// Next List of all the validators
	pub type NextValidatorList<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	// The pallet's runtime storage items.
	#[pallet::storage]
	#[pallet::getter(fn validator_list_set_id)]
	/// Current validators set id
	pub type ValidatorListSetId<T: Config> = StorageValue<_, u64, ValueQuery>;

	// The pallet's runtime storage items.
	#[pallet::storage]
	#[pallet::getter(fn white_list_validators)]
	/// List of all the white list validators
	pub type WhiteListValidators<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ValidatorId, Blake2_128Concat, T::AccountId, bool>;

	#[pallet::storage]
	#[pallet::getter(fn pool_counter)]
	/// Pool counter
	pub type ValidatorCounter<T: Config> = StorageValue<_, ValidatorId>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New validator added
		/// parameters. [who]
		ValidatorAdded(T::AccountId),

		/// Validator removed
		/// parameters. [who]
		ValidatorRemoved(T::AccountId),
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
		pub initial_validators: Vec<T::AccountId>,
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
		pub fn add_validator(origin: OriginFor<T>, validator_acc: T::AccountId) -> DispatchResult {
			// Check if the sender is an approved origin or not
			T::ApproveOrigin::ensure_origin(origin)?;
			<WhiteListValidators<T>>::insert(validator_id, validator_acc.clone(), true);
			Self::deposit_event(Event::ValidatorAdded(validator_acc));
			Ok(())
		}

		/// Remove a validator from the set.
		/// Emits an event on success else error
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn remove_validator(
			origin: OriginFor<T>,
			validator_id: ValidatorId,
			validator_acc: T::AccountId,
		) -> DispatchResult {
			// Check if the sender is an approved origin or not
			T::ApproveOrigin::ensure_origin(origin)?;
			<WhiteListValidators<T>>::remove(validator_id, validator_acc.clone());
			Self::deposit_event(Event::ValidatorRemoved(validator_acc));
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	#[allow(dead_code)]
	fn initialize_validators(validator_list: &[T::AccountId]) {
		// Add the initial validator_list to the validator set.
		<ValidatorList<T>>::put(validator_list);
	}

	pub fn validator_id_inc() -> Result<ValidatorId, DispatchError> {
		if ValidatorCounter::<T>::get().is_some() {
			let validator_id = ValidatorCounter::<T>::get()
				.unwrap()
				.checked_add(1)
				.ok_or(ArithmeticError::Underflow)?;
			ValidatorCounter::<T>::set(Option::from(validator_id));
			Ok(validator_id)
		} else {
			ValidatorCounter::<T>::set(Some(1));
			Ok(1)
		}
	}

	// To check if the given member is validator or not
	pub fn is_validator(validator_id: ValidatorId, validator_acc: &T::AccountId) -> bool {
		WhiteListValidators::<T>::get(validator_id, validator_acc).unwrap_or(false)
	}
}

impl<T: Config> ValidatorSet<ValidatorId, AccountIdOf<T>> for Pallet<T> {
	fn is_validator(validator_id: ValidatorId, validator_acc: &T::AccountId) -> bool {
		Self::is_validator(validator_id, validator_acc)
	}
}

pub trait ValidatorSet<ValidatorId, AccountId> {
	fn is_validator(validator_id: ValidatorId, validator_acc: &AccountId) -> bool;
}

impl<T: Config> BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::XrpValidatorId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = T::XrpValidatorId;

	fn on_genesis_session<'a, I: 'a>(validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::XrpValidatorId)>,
	{
		let keys = validators.map(|x| x.1).collect::<Vec<_>>();
		if !keys.is_empty() {
			assert!(
				ValidatorList::<T>::decode_len().is_none(),
				"ValidatorList are already initialized!"
			);
			ValidatorList::<T>::put(keys);
		}
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::XrpValidatorId)>,
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
			BridgePaused::kill();
			// Time to update the bridge validator keys.
			let next_notary_keys = NextValidatorList::<T>::take();
			// Store the new keys and increment the validator set id
			// Next notary keys should be unset, until populated by new session logic
			<ValidatorList<T>>::put(&next_notary_keys);
			ValidatorListSetId::mutate(|next_set_id| *next_set_id = next_set_id.wrapping_add(1));
		}
	}

	fn on_disabled(_i: u32) {}
}
