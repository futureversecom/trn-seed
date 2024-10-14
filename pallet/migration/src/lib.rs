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
pub use pallet::*;

use frame_support::{log, pallet_prelude::*, sp_runtime::traits::Zero};
use frame_system::pallet_prelude::*;
use seed_pallet_common::Migrator;
use seed_primitives::migration::MigrationStep;
use sp_std::prelude::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod weights;
pub use weights::WeightInfo;

#[allow(dead_code)]
pub(crate) const LOG_TARGET: &str = "migration";

/// The result of running the migration.
#[derive(Decode, Encode, RuntimeDebugNoBound, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub enum MigrationStatus {
	/// No migration currently in progress
	NoMigrationInProgress,
	/// A migration is in progress
	InProgress { steps_done: u32 },
	/// All current migrations are completed
	Completed,
}

impl Default for MigrationStatus {
	fn default() -> Self {
		MigrationStatus::NoMigrationInProgress
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The system event type
		type RuntimeEvent: From<Event> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		// /// Interface to access weight values
		// type WeightInfo: WeightInfo;
		type CurrentMigration: MigrationStep;

		/// The maximum weight this pallet can use in on_idle
		#[pallet::constant]
		type MaxMigrationWeight: Get<Weight>;

		/// Interface to access weight values
		type WeightInfo: WeightInfo;
	}

	/// Are we currently migrating data
	#[pallet::storage]
	pub type MigrationEnabled<T> = StorageValue<_, bool, ValueQuery>;

	/// What is the current status of the migration
	#[pallet::storage]
	pub type Status<T> = StorageValue<_, MigrationStatus, ValueQuery>;

	/// The last key that was migrated
	#[pallet::storage]
	pub type LastKey<T> = StorageValue<_, Vec<u8>, OptionQuery>;

	/// The delay between migration blocks
	#[pallet::storage]
	pub type BlockDelay<T> = StorageValue<_, u32, OptionQuery>;

	/// Default value is 100 which is on the conservative side
	#[pallet::type_value]
	pub fn DefaultBlockLimit() -> u32 {
		100
	}

	/// The maximum number of individual items to migrate in a single block
	/// Will still respect maximum weight rules
	#[pallet::storage]
	pub type BlockLimit<T> = StorageValue<_, u32, ValueQuery, DefaultBlockLimit>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event {
		/// Migration has been enabled
		MigrationEnabled,
		/// The current migration has been paused
		MigrationPaused,
		/// The current migration has completed
		MigrationComplete { items_migrated: u32 },
		/// A Migration has started
		MigrationStarted,
	}

	#[pallet::error]
	pub enum Error<T> {
		MigrationInProgress,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_idle(_block: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			Self::migrate(remaining_weight)
		}

		fn on_runtime_upgrade() -> Weight {
			// Check if we are in the middle of a migration
			if T::CurrentMigration::version_check() {
				// Update Status to NoMigrationInProgress to signify that since the last runtime
				// upgrade there was no multi-block migration
				Status::<T>::put(MigrationStatus::NoMigrationInProgress);
				log::debug!(target: LOG_TARGET, "🦆 No multi-block migration in progress");
				return T::DbWeight::get().writes(1);
			} else {
				// Ensure that a migration is not already in progress. This is to prevent data loss
				// in the case where a runtime update is performed before the previous migration is
				// completed.
				if !Self::migration_in_progress() {
					Status::<T>::put(MigrationStatus::InProgress { steps_done: 0 });
					Self::deposit_event(Event::MigrationStarted);
					log::debug!(target: LOG_TARGET, "🦆 A new multi-block migration has started");
					return T::DbWeight::get().writes(1);
				} else {
					log::debug!(target: LOG_TARGET, "🦆 A multi-block migration is already in progress");
				}
			}
			Weight::zero()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::enable_migration())]
		pub fn enable_migration(origin: OriginFor<T>, enabled: bool) -> DispatchResult {
			ensure_root(origin)?;
			MigrationEnabled::<T>::put(enabled);
			match enabled {
				true => Self::deposit_event(Event::MigrationEnabled),
				false => Self::deposit_event(Event::MigrationPaused),
			}
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::enable_migration())]
		pub fn set_block_delay(origin: OriginFor<T>, delay: Option<u32>) -> DispatchResult {
			ensure_root(origin)?;
			match delay {
				Some(delay) => BlockDelay::<T>::put(delay),
				None => BlockDelay::<T>::kill(),
			}
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::enable_migration())]
		pub fn set_block_limit(origin: OriginFor<T>, limit: u32) -> DispatchResult {
			ensure_root(origin)?;
			BlockLimit::<T>::put(limit);
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn migrate(weight_limit: Weight) -> Weight {
		let weight_limit = weight_limit.min(T::MaxMigrationWeight::get());
		// Check if there is enough weight to perform the migration
		let mut used_weight = Weight::zero();
		// Maximum weight for one migration step
		let max_step_weight = T::CurrentMigration::max_step_weight();
		// Reads: MigrationEnabled, Status, LastKey
		// Writes: Status, LastKey
		let base_weight = T::WeightInfo::migrate();

		// Check we have enough weight to perform at least one step
		if weight_limit.all_lt(base_weight.saturating_add(max_step_weight)) {
			return Weight::zero();
		}

		// Check if there is a migration in progress and it is not paused
		let previous_steps = match Status::<T>::get() {
			MigrationStatus::InProgress { steps_done } => steps_done,
			_ => return T::DbWeight::get().reads(1),
		};

		if !MigrationEnabled::<T>::get() {
			return T::DbWeight::get().reads(2);
		}

		let mut last_key = LastKey::<T>::get();
		let mut step_counter: u32 = 0;
		used_weight = used_weight.saturating_add(base_weight);

		let block_number = frame_system::Pallet::<T>::block_number();
		// let number: BlockNumber = block_number.into();
		if let Some(delay) = BlockDelay::<T>::get() {
			let delay: BlockNumberFor<T> = delay.into();
			if block_number % delay != BlockNumberFor::<T>::zero() {
				log::debug!(target: LOG_TARGET, "🦆 Skipping multi-block migration for block {:?}", block_number);
				return used_weight;
			}
		}
		log::debug!(target: LOG_TARGET, "🦆 Starting multi-block migration for block {:?}", block_number);
		let block_limit: u32 = BlockLimit::<T>::get();
		while used_weight.all_lt(weight_limit) && step_counter < block_limit {
			// Perform one migration step on the current migration
			let step_result = T::CurrentMigration::step(last_key);
			last_key = step_result.last_key.clone();
			used_weight = used_weight.saturating_add(step_result.weight_consumed);
			if step_counter.checked_add(1).is_none() {
				log::debug!(target: LOG_TARGET, "🦆 Step counter overflowed, stopping migration");
				break;
			}
			step_counter = step_counter.saturating_add(1);

			if step_result.is_finished() {
				Self::complete_migration(previous_steps.saturating_add(step_counter));
				return used_weight;
			}
		}
		log::debug!(target: LOG_TARGET, "🦆 Block {:?} Successfully migrated {} items, total: {}",
			block_number,
			step_counter,
			previous_steps.saturating_add(step_counter)
		);

		// Weight of these writes is accounted for in base_weight
		Status::<T>::put(MigrationStatus::InProgress {
			steps_done: previous_steps.saturating_add(step_counter),
		});
		if let Some(last_key) = last_key {
			LastKey::<T>::put(last_key);
		} else {
			LastKey::<T>::kill();
		}

		used_weight
	}

	/// Perform post migration operations and clean up storage
	fn complete_migration(total_steps: u32) {
		Status::<T>::put(MigrationStatus::Completed);
		LastKey::<T>::kill();
		T::CurrentMigration::on_complete();
		log::debug!(target: LOG_TARGET, "🦆 Migration completed successfully");
		log::debug!(target: LOG_TARGET, "🦆 Total items migrated: {}", total_steps);
		Self::deposit_event(Event::MigrationComplete { items_migrated: total_steps });
	}

	/// Returns whether a migration is in progress
	fn migration_in_progress() -> bool {
		match Status::<T>::get() {
			MigrationStatus::Completed => false,
			MigrationStatus::NoMigrationInProgress => false,
			_ => true,
		}
	}
}

/// Called by external pallets to check on the migration process
impl<T: Config> Migrator for Pallet<T> {
	fn ensure_migrated() -> DispatchResult {
		ensure!(!Self::migration_in_progress(), Error::<T>::MigrationInProgress);
		Ok(())
	}
}
