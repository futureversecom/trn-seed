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
use frame_system::pallet_prelude::*;
use sp_std::prelude::*;
use seed_pallet_common::Migration;

#[frame_support::pallet]
pub mod pallet {
    use seed_primitives::{CollectionUuid, TokenId};
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The system event type
        type RuntimeEvent: From<Event> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        // /// Interface to access weight values
        // type WeightInfo: WeightInfo;
        type CurrentMigration: Migration;
    }

    /// Are we currently migrating data
    #[pallet::storage]
    pub type MigrationEnabled<T> = StorageValue<_, bool, ValueQuery>;


    /// Are we currently migrating data
    #[pallet::storage]
    pub type MigrationComplete<T> = StorageValue<_, bool, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event {
        /// Migration has been enabled
        MigrationEnabled,
        /// Migration has been paused
        MigrationPaused,
        /// Migration has been completed
        MigrationComplete,
    }

    #[pallet::error]
    pub enum Error<T> {
        Errnogo,
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
        pub fn manual_migrate_next(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?;
            T::CurrentMigration::migrate_next();
            Ok(())
        }
    }
}
