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

use crate as pallet_migration;
use crate::Config;
use frame_support::pallet_prelude::ValueQuery;
use frame_support::{storage_alias, Twox64Concat};
use seed_pallet_common::test_prelude::*;
use seed_primitives::migration::{MigrationStep, MigrationStepResult};
use std::marker::PhantomData;

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Assets: pallet_assets,
		Balances: pallet_balances,
		Migration: pallet_migration,
	}
);

impl_frame_system_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_balance_config!(Test);

pub const WEIGHT_PER_MIGRATION: u64 = 1000;

pub type OldType = u32;
pub type NewType = String;

pub mod old {
	use super::*;

	#[storage_alias]
	pub type TestMap<Test: Config> = StorageMap<crate::Pallet<Test>, Twox64Concat, u32, OldType>;
}
#[storage_alias]
pub type TestMap<Test: Config> = StorageMap<crate::Pallet<Test>, Twox64Concat, u32, NewType>;

#[storage_alias]
pub type TestVersion<Test: Config> = StorageValue<crate::Pallet<Test>, u16, ValueQuery>;

/// A mock migration to test the migration pallet
pub struct MockMigration<T: Config> {
	phantom: PhantomData<T>,
}

impl<T: Config> MigrationStep for MockMigration<T> {
	const TARGET_VERSION: u16 = 1;

	fn version_check() -> bool {
		TestVersion::<T>::get() == Self::TARGET_VERSION
	}

	fn on_complete() {
		TestVersion::<T>::put(Self::TARGET_VERSION);
	}

	fn max_step_weight() -> Weight {
		Weight::from_all(WEIGHT_PER_MIGRATION)
	}

	fn step(last_key: Option<Vec<u8>>) -> MigrationStepResult {
		let mut iter = if let Some(last_key) = last_key {
			old::TestMap::<T>::iter_from(last_key)
		} else {
			old::TestMap::<T>::iter()
		};

		if let Some((key, value)) = iter.next() {
			let new_value = (value + 1).to_string();
			TestMap::<T>::insert(key, new_value);
			let last_key = old::TestMap::<T>::hashed_key_for(key);
			MigrationStepResult::continue_step(Self::max_step_weight(), last_key)
		} else {
			MigrationStepResult::finish_step(Self::max_step_weight())
		}
	}
}

parameter_types! {
	// Allow max 1000 migrations per block based on weight
	pub MaxMigrationWeight: Weight = Weight::from_all(u64::MAX);
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type CurrentMigration = MockMigration<Test>;
	type MaxMigrationWeight = MaxMigrationWeight;
	type WeightInfo = ();
}
