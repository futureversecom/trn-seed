#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
pub use pallet::*;
use seed_primitives::ValidatorId;
use sp_runtime::ArithmeticError;
use sp_std::vec::Vec;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

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
	#[pallet::getter(fn validators)]
	/// List of all the validators
	pub type Validators<T: Config> =
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
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(8,7))]
		pub fn add_validator(origin: OriginFor<T>, validator_acc: T::AccountId) -> DispatchResult {
			// Check if the sender is an approved origin or not
			T::ApproveOrigin::ensure_origin(origin)?;

			let mut validator_list = Self::validator_list();
			// check of the given validator is already exist in the set before adding
			match validator_list.binary_search(&validator_acc) {
				Ok(_) => Err(Error::<T>::DuplicateValidator)?,
				Err(pos) => {
					let validator_id = Self::validator_id_inc().expect("Validator id inc falied");
					// Add the validator to the set
					validator_list.insert(pos, validator_acc.clone());
					// Store the set
					<ValidatorList<T>>::put(validator_list);
					<Validators<T>>::insert(validator_id, validator_acc.clone(), true);
					// Emit an event
					Self::deposit_event(Event::ValidatorAdded(validator_acc));
					Ok(())
				},
			}
		}

		/// Remove a validator from the set.
		/// Emits an event on success else error
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(7,7))]
		pub fn remove_validator(
			origin: OriginFor<T>,
			validator_id: ValidatorId,
			validator_acc: T::AccountId,
		) -> DispatchResult {
			// Check if the sender is an approved origin or not
			T::ApproveOrigin::ensure_origin(origin)?;

			let mut validator_list = Self::validator_list();
			match validator_list.binary_search(&validator_acc) {
				Ok(pos) => {
					// Remove the validator from the set
					validator_list.remove(pos);
					// Store the set
					<ValidatorList<T>>::put(validator_list);
					<Validators<T>>::remove(validator_id, validator_acc.clone());
					// Emit an event
					Self::deposit_event(Event::ValidatorRemoved(validator_acc));
					Ok(())
				},
				Err(_) => Err(Error::<T>::ValidatorNotFound)?,
			}
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
		Validators::<T>::get(validator_id, validator_acc).unwrap_or(false)
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
