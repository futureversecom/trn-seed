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
use frame_support::{
    log,
    pallet_prelude::*,
    sp_runtime::{traits::One},
};
use frame_support::traits::OnRuntimeUpgrade;
use frame_system::pallet_prelude::*;
use sp_std::prelude::*;
use seed_pallet_common::Migrator;
use seed_primitives::migration::{MigrationStep};
use seed_primitives::{CollectionUuid, SerialNumber};

#[allow(dead_code)]
pub(crate) const LOG_TARGET: &str = "migration";

/// The result of running the migration.
#[derive(Decode, Encode, RuntimeDebugNoBound, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub enum MigrateStatus {
    /// No migration currently in progress
    NoMigrationInProgress,
    /// A migration is in progress
    InProgress { steps_done: u32 },
    /// All current migrations are completed
    Completed,
}

impl Default for MigrateStatus {
    fn default() -> Self {
        MigrateStatus::NoMigrationInProgress
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The system event type
        type RuntimeEvent: From<Event> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        // /// Interface to access weight values
        // type WeightInfo: WeightInfo;
        type CurrentMigration: MigrationStep<StorageKey=(CollectionUuid, SerialNumber)>;

        /// The maximum weight this pallet can use in on_idle
        #[pallet::constant]
        type MaxMigrationWeight: Get<Weight>;
    }

    /// Are we currently migrating data
    #[pallet::storage]
    pub type MigrationEnabled<T> = StorageValue<_, bool, ValueQuery>;


    /// Are we currently migrating data
    #[pallet::storage]
    pub type Status<T> = StorageValue<_, MigrateStatus, ValueQuery>;

    /// The last key that was migrated if any
    #[pallet::storage]
    pub type LastKey<T> = StorageValue<_, (CollectionUuid, SerialNumber), OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event {
        /// Migration has been enabled
        MigrationEnabled,
        /// Migration has been paused
        MigrationPaused,
        /// Migration has been completed
        MigrationComplete,

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
            if T::CurrentMigration::version_check() {
                LastKey::<T>::kill();
                Status::<T>::put(MigrateStatus::NoMigrationInProgress);
                log::debug!(target: LOG_TARGET, "🦆 No multi-block migration in progress");
                return T::DbWeight::get().writes(2);
            } else {
                // Ensure that a migration is not already in progress. This is to prevent data loss
                // in the case where an update is performed before the previous migration is completed.
                if !Self::migration_in_progress() {
                    Status::<T>::put(MigrateStatus::InProgress { steps_done: 0 });
                    Self::deposit_event(Event::MigrationStarted);
                    log::debug!(target: LOG_TARGET, "🦆 A new multi-block migration has started");
                    return T::DbWeight::get().writes(1);
                } else {
                    log::debug!(target: LOG_TARGET, "🦆 A multi-block migration is already in progress");
                }
            }
            Weight::zero()
        }

        // fn integrity_test() {
        //     T::CurrentMigration::integrity_test();
        // }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(0)]
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
        #[pallet::weight(0)]
        pub fn manual_trigger(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?;
            MigrationEnabled::<T>::put(true);
            Status::<T>::put(MigrateStatus::InProgress { steps_done: 0 });
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
        let base_weight = T::DbWeight::get().reads_writes(3, 2);

        // Check we have enough weight to perform at least one step
        if weight_limit.all_lt(base_weight.saturating_add(max_step_weight)) {
            return Weight::zero()
        }

        // Check if there is a migration in progress and it is not paused
        if !MigrationEnabled::<T>::get() {
            return T::DbWeight::get().reads(1);
        }
        let previous_steps = match Status::<T>::get() {
            MigrateStatus::InProgress { steps_done } => steps_done,
            _ => return T::DbWeight::get().reads(2),
        };
        let mut last_key = LastKey::<T>::get();
        let mut step_counter: u32 = 0;
        used_weight = used_weight.saturating_add(base_weight);

        while used_weight.all_lt(weight_limit) {
            // Perform one migration step on the current migration
            let step_result = T::CurrentMigration::step(last_key);
            last_key = step_result.last_key;
            used_weight = used_weight.saturating_add(step_result.weight_consumed);
            step_counter = step_counter.saturating_add(1);

            if step_result.is_finished() {
                Self::post_migration();
                return used_weight;
            }
        }
        let block_number = frame_system::Pallet::<T>::block_number();
        log::debug!(target: LOG_TARGET, "🦆 Block: {:?} Migrated {} items, total: {}", block_number, step_counter, previous_steps.saturating_add(step_counter));

        // Weight of these writes is accounted for in base_weight
        Status::<T>::put(MigrateStatus::InProgress { steps_done: previous_steps.saturating_add(step_counter) });
        if let Some(last_key) = last_key {
            LastKey::<T>::put(last_key);
        } else {
            LastKey::<T>::kill();
        }

        used_weight
    }

    fn post_migration() {
        Status::<T>::put(MigrateStatus::Completed);
        LastKey::<T>::kill();
        log::debug!(target: LOG_TARGET, "🦆 Migration completed successfully");
        Self::deposit_event(Event::MigrationComplete);
    }

    /// Returns whether a migration is in progress
    fn migration_in_progress() -> bool {
        match Status::<T>::get() {
            MigrateStatus::Completed => false,
            MigrateStatus::NoMigrationInProgress => false,
            _ => true,
        }
    }
}

impl<T: Config> Migrator for Pallet<T> {
    fn ensure_migrated() -> DispatchResult {
        ensure!(!Self::migration_in_progress(), Error::<T>::MigrationInProgress);
        Ok(())
    }
}
