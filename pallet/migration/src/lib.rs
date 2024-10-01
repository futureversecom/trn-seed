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
    pallet_prelude::*,
    sp_runtime::{traits::One, SaturatedConversion},
};
use frame_support::traits::OnRuntimeUpgrade;
use frame_system::pallet_prelude::*;
use sp_std::prelude::*;
use seed_pallet_common::Migrator;
use seed_primitives::migration::{IsFinished, MigrationStep};

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
    use frame_support::weights::RuntimeDbWeight;
    use seed_primitives::{CollectionUuid, SerialNumber, TokenId};
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
        fn on_idle(_block: BlockNumberFor<T>, mut remaining_weight: Weight) -> Weight {
            Self::migrate(remaining_weight)
        }

        fn on_runtime_upgrade() -> Weight {
            if T::CurrentMigration::version_check() {
                LastKey::<T>::kill();
                Status::<T>::put(MigrateStatus::NoMigrationInProgress);
                // TODO this causes the upgrade to fail due to exhausting block weights, why?
                // return T::DbWeight::get().writes(2);
            } else {
                // Ensure that a migration is not already in progress. This is to prevent data loss
                // in the case where an update is performed before the previous migration is completed.
                if !Self::migration_in_progress() {
                    Status::<T>::put(MigrateStatus::InProgress { steps_done: 0 });
                    Self::deposit_event(Event::MigrationStarted);
                    // return T::DbWeight::get().writes(1);
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
    }
}

impl<T: Config> Pallet<T> {
    pub fn migrate(mut weight_limit: Weight) -> Weight {
        // Check if there is enough weight to perform the migration
        let mut weight_left = weight_limit;
        let migration_weight = Weight::zero(); // TODO Change this
        if weight_left.checked_reduce(migration_weight).is_none() {
            return Weight::zero()
        }

        // CHeck if there is a migration in progress and it is not paused
        let status = Status::<T>::get();
        if status == MigrateStatus::NoMigrationInProgress {
            return Weight::zero();
        }
        if !MigrationEnabled::<T>::get() {
            return Weight::zero();
        }

        let mut steps_done = match status {
            MigrateStatus::InProgress { steps_done } => steps_done,
            _ => 0,
        };
        let mut last_key = LastKey::<T>::get();
        while weight_left.all_gt(weight_limit) {
            let (result, step_weight, last) = T::CurrentMigration::step(last_key);
            last_key = last;
            weight_left.saturating_reduce(step_weight);
            steps_done += 1;

            if result == IsFinished::Yes {
                Self::post_migration();
                return weight_limit.saturating_sub(weight_left);
            }
        }

        Status::<T>::put(MigrateStatus::InProgress { steps_done });
        if let Some(last_key) = last_key {
            LastKey::<T>::put(last_key);
        } else {
            LastKey::<T>::kill();
        }

        weight_limit.saturating_sub(weight_left)
    }

    fn post_migration() {
        Status::<T>::put(MigrateStatus::Completed);
        LastKey::<T>::kill();
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
