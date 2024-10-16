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

use frame_support::storage::storage_prefix;
use frame_support::StorageHasher;
use super::*;
use crate::mock::{old::TestMap as TestMapOld, Migration, NewType, System, Test};
use crate::Config;
use seed_pallet_common::test_prelude::*;

// Setup some fake data for testing by inserting into the map as the old type (u32)
fn setup_old_data(count: u32) {
	for i in 0..count {
		TestMapOld::<Test>::insert(i, i * 100);
	}
}

/// Verify that the correct amount of data has been migrated by counting up the total
/// number of converted items in the new storage map
/// If `converted_count` is None, it will be assumed that all items have been converted
fn verify_new_data(total_count: u32, converted_count: Option<u32>) {
	let mut converted: u32 = 0;
	for i in 0..total_count {
		if let Some(value) = get::<NewType>(i) {
			if value == (i * 100 + 1).to_string() {
				converted += 1;
			}
		}
	}
	assert_eq!(converted, converted_count.unwrap_or(total_count));
}

/// Return the value of the item in storage under `key`, or `None` if there is no explicit entry.
/// This is a helper function that can unsafely decode a value from storage.
/// Used to avoid the corrupted state error handling in the frame support storage module.
fn get<T: Decode + Sized>(key: u32) -> Option<T> {
	let key_raw = Twox64Concat::hash(&(key).encode());
	let storage_prefix = storage_prefix(b"Migration", b"TestMap");
	let mut key = vec![0u8; 32 + key_raw.len()];
	key[..32].copy_from_slice(&storage_prefix);
	key[32..].copy_from_slice(&key_raw);

	sp_io::storage::get(&key).and_then(|val| {
		Decode::decode(&mut &val[..]).map(Some).unwrap_or_default()
	})
}

mod enable_migration {
	use super::*;

	#[test]
	fn enable_migration_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_ok!(Migration::enable_migration(RawOrigin::Root.into(), true));
			assert!(MigrationEnabled::<Test>::get());
			System::assert_has_event(Event::MigrationEnabled.into());

			assert_ok!(Migration::enable_migration(RawOrigin::Root.into(), false));
			assert!(!MigrationEnabled::<Test>::get());
			System::assert_has_event(Event::MigrationDisabled.into());
		});
	}

	#[test]
	fn enable_migration_not_sudo_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_noop!(
				Migration::enable_migration(RawOrigin::Signed(create_account(1)).into(), true),
				BadOrigin
			);
		});
	}
}

mod set_block_delay {
	use super::*;

	#[test]
	fn set_block_delay_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let block_delay = Some(10);
			assert_ok!(Migration::set_block_delay(RawOrigin::Root.into(), block_delay));
			assert_eq!(BlockDelay::<Test>::get(), block_delay);
			System::assert_has_event(Event::BlockDelaySet { block_delay }.into());

			let block_delay = None;
			assert_ok!(Migration::set_block_delay(RawOrigin::Root.into(), block_delay));
			assert_eq!(BlockDelay::<Test>::get(), block_delay);
			System::assert_has_event(Event::BlockDelaySet { block_delay }.into());
		});
	}

	#[test]
	fn set_block_delay_not_sudo_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let block_delay = Some(10);
			assert_noop!(
				Migration::set_block_delay(
					RawOrigin::Signed(create_account(1)).into(),
					block_delay
				),
				BadOrigin
			);
		});
	}

	// TODO actual migration with block delay set
}

mod set_block_limit {
	use super::*;

	#[test]
	fn set_block_limit_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let block_limit = 100;
			assert_ok!(Migration::set_block_limit(RawOrigin::Root.into(), block_limit));
			assert_eq!(BlockLimit::<Test>::get(), block_limit);
			System::assert_has_event(Event::BlockLimitSet { block_limit }.into());
		});
	}

	#[test]
	fn set_block_limit_not_sudo_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_noop!(
				Migration::set_block_limit(RawOrigin::Signed(create_account(1)).into(), 10),
				BadOrigin
			);
		});
	}
}

mod on_runtime_upgrade {
	use super::*;

	#[test]
	fn on_runtime_upgrade_starts_migration() {
		TestExt::<Test>::default().build().execute_with(|| {
			// sanity check, version check should return false
			assert!(!<Test as Config>::CurrentMigration::version_check());
			let used_weight = Migration::on_runtime_upgrade();
			// Check storage updated
			assert_eq!(Status::<Test>::get(), MigrationStatus::InProgress { steps_done: 0 });
			System::assert_has_event(Event::MigrationStarted.into());
			assert_eq!(used_weight, DbWeight::get().reads_writes(1, 1));
			// Ensure migrated should fail as the migration is now in progress
			assert_noop!(Pallet::<Test>::ensure_migrated(), Error::<Test>::MigrationInProgress);
		});
	}

	/// Tests whether the migration updates Status to NoMigrationInProgress if version check passes
	#[test]
	fn on_runtime_upgrade_migration_complete_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Call on complete to update version
			<Test as Config>::CurrentMigration::on_complete();
			assert!(<Test as Config>::CurrentMigration::version_check());
			let used_weight = Migration::on_runtime_upgrade();
			assert_eq!(Status::<Test>::get(), MigrationStatus::NoMigrationInProgress);
			assert_eq!(used_weight, DbWeight::get().writes(1));
			// Ensure migrated should pass as the migration is now complete
			assert_ok!(Pallet::<Test>::ensure_migrated());
		});
	}

	#[test]
	fn on_runtime_upgrade_migration_in_progress() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Fake the migration status to in progress
			Status::<Test>::put(MigrationStatus::InProgress { steps_done: 100 });
			let used_weight = Migration::on_runtime_upgrade();
			// Status un changed
			assert_eq!(Status::<Test>::get(), MigrationStatus::InProgress { steps_done: 100 });
			assert_eq!(used_weight, DbWeight::get().reads(1));
			// Ensure migrated should fail as the migration is now in progress
			assert_noop!(Pallet::<Test>::ensure_migrated(), Error::<Test>::MigrationInProgress);
		});
	}
}

mod migrate {
	use super::*;

	#[test]
	fn migrate_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let data_count: u32 = 10;
			// Setup fake data
			setup_old_data(data_count);
			// Initialise migration
			Migration::on_runtime_upgrade();
			// Enable migration
			assert_ok!(Migration::enable_migration(RawOrigin::Root.into(), true));

			// Call on_idle with plenty of weight to complete the migration
			let block_number = System::block_number() + 1;
			let used_weight = Migration::on_idle(block_number, Weight::from_all(u64::MAX));
			// Expected weight is the weight for each step, + 1 for the finishing step
			// + the weight for the migration function
			let mut expected_weight =
				<Test as Config>::CurrentMigration::max_step_weight() * (data_count as u64 + 1);
			expected_weight += <Test as Config>::WeightInfo::migrate();
			assert_eq!(used_weight, expected_weight);

			// Verify that all data has been migrated and storage is updated
			verify_new_data(data_count, None);
			assert_eq!(Status::<Test>::get(), MigrationStatus::Completed);
			assert_ok!(Pallet::<Test>::ensure_migrated());
			assert!(LastKey::<Test>::get().is_none());
			assert!(<Test as Config>::CurrentMigration::version_check());
		});
	}

	#[test]
	fn migrate_not_enough_weight_is_noop() {
		TestExt::<Test>::default().build().execute_with(|| {
			let data_count: u32 = 10;
			setup_old_data(data_count);
			Migration::on_runtime_upgrade();
			assert_ok!(Migration::enable_migration(RawOrigin::Root.into(), true));

			// Call on_idle with not enough weight to perform one migration step
			let block_number = System::block_number() + 1;
			let base_weight = <Test as Config>::WeightInfo::migrate()
				+ <Test as Config>::CurrentMigration::max_step_weight();
			let used_weight = Migration::on_idle(block_number, base_weight - 1.into());
			assert_eq!(used_weight, Weight::zero());

			// Verify that no data has been migrated
			verify_new_data(data_count, Some(0));
			assert_eq!(Status::<Test>::get(), MigrationStatus::InProgress { steps_done: 0 });
		});
	}

	#[test]
	fn migrate_not_enabled_is_noop() {
		TestExt::<Test>::default().build().execute_with(|| {
			let data_count: u32 = 10;
			setup_old_data(data_count);
			Migration::on_runtime_upgrade();
			// Disable migration
			assert_ok!(Migration::enable_migration(RawOrigin::Root.into(), false));

			// Call on_idle with enough weight, but migration is not enabled
			let block_number = System::block_number() + 1;
			let used_weight = Migration::on_idle(block_number, Weight::from_all(u64::MAX));
			assert_eq!(used_weight, DbWeight::get().reads(2));

			// Verify that no data has been migrated
			verify_new_data(data_count, Some(0));
			assert_eq!(Status::<Test>::get(), MigrationStatus::InProgress { steps_done: 0 });
		});
	}

	#[test]
	fn migrate_no_migration_active_is_noop() {
		TestExt::<Test>::default().build().execute_with(|| {
			// No migration active
			assert_eq!(Status::<Test>::get(), MigrationStatus::NoMigrationInProgress);

			// Call on_idle with enough weight, but no migration active
			let block_number = System::block_number() + 1;
			let used_weight = Migration::on_idle(block_number, Weight::from_all(u64::MAX));
			assert_eq!(used_weight, DbWeight::get().reads(1));

			assert_eq!(Status::<Test>::get(), MigrationStatus::NoMigrationInProgress);
		});
	}

	#[test]
	fn migrate_with_block_limit() {
		TestExt::<Test>::default().build().execute_with(|| {
			let data_count: u32 = 1000;
			setup_old_data(data_count);
			Migration::on_runtime_upgrade();
			assert_ok!(Migration::enable_migration(RawOrigin::Root.into(), true));

			// Call on_idle with BlockLimit set to 10
			let block_limit: u32 = 10;
			assert_ok!(Migration::set_block_limit(RawOrigin::Root.into(), block_limit));
			let block_number = System::block_number() + 1;
			let used_weight = Migration::on_idle(block_number, Weight::from_all(u64::MAX));
			let expected_weight = <Test as Config>::WeightInfo::migrate()
				+ <Test as Config>::CurrentMigration::max_step_weight() * block_limit as u64;
			assert_eq!(used_weight, expected_weight);
			assert_eq!(
				Status::<Test>::get(),
				MigrationStatus::InProgress { steps_done: block_limit }
			);
			assert!(LastKey::<Test>::get().is_some());
			assert_noop!(Pallet::<Test>::ensure_migrated(), Error::<Test>::MigrationInProgress);
			assert!(!<Test as Config>::CurrentMigration::version_check());
			verify_new_data(data_count, Some(block_limit));

			// Call on_idle with BlockLimit set to 100
			let block_limit: u32 = 100;
			assert_ok!(Migration::set_block_limit(RawOrigin::Root.into(), block_limit));
			let block_number = System::block_number() + 2;
			let used_weight = Migration::on_idle(block_number, Weight::from_all(u64::MAX));
			let expected_weight = <Test as Config>::WeightInfo::migrate()
				+ <Test as Config>::CurrentMigration::max_step_weight() * block_limit as u64;
			assert_eq!(used_weight, expected_weight);
			// Verify total completed is 100 + 10
			assert_eq!(Status::<Test>::get(), MigrationStatus::InProgress { steps_done: 110 });
			assert!(LastKey::<Test>::get().is_some());
			assert_noop!(Pallet::<Test>::ensure_migrated(), Error::<Test>::MigrationInProgress);
			assert!(!<Test as Config>::CurrentMigration::version_check());
			verify_new_data(data_count, Some(110));

			// Call on_idle with BlockLimit set to 1000, allowing the migration to complete
			let block_limit: u32 = 1000;
			assert_ok!(Migration::set_block_limit(RawOrigin::Root.into(), block_limit));
			let block_number = System::block_number() + 3;
			let used_weight = Migration::on_idle(block_number, Weight::from_all(u64::MAX));
			let expected_weight = <Test as Config>::WeightInfo::migrate()
				+ <Test as Config>::CurrentMigration::max_step_weight() * 891;
			assert_eq!(used_weight, expected_weight);
			assert_eq!(Status::<Test>::get(), MigrationStatus::Completed);
			assert!(LastKey::<Test>::get().is_none());
			assert_ok!(Pallet::<Test>::ensure_migrated());
			assert!(<Test as Config>::CurrentMigration::version_check());
			verify_new_data(data_count, None);
		});
	}

	#[test]
	fn migrate_with_block_delay() {
		TestExt::<Test>::default().build().execute_with(|| {
			let data_count: u32 = 10;
			setup_old_data(data_count);
			Migration::on_runtime_upgrade();
			assert_ok!(Migration::enable_migration(RawOrigin::Root.into(), true));

			// Set block_limit to 1 so we don't complete the migration in one go
			let block_limit: u32 = 1;
			assert_ok!(Migration::set_block_limit(RawOrigin::Root.into(), block_limit));

			// Set block delay to 5, so we should only process the migration on every 5th block
			let block_delay: u32 = 5;
			assert_ok!(Migration::set_block_delay(RawOrigin::Root.into(), Some(block_delay)));

			// No migration for blocks 1 to 4
			for block_number in 1..5 {
				let used_weight = Migration::on_idle(block_number, Weight::from_all(u64::MAX));
				assert_eq!(used_weight, DbWeight::get().reads(3));
				assert_eq!(Status::<Test>::get(), MigrationStatus::InProgress { steps_done: 0 });
				verify_new_data(data_count, Some(0));
			}

			// Migration should process one step on block 5
			let block_number = 5;
			let used_weight = Migration::on_idle(block_number, Weight::from_all(u64::MAX));
			let expected_weight = <Test as Config>::WeightInfo::migrate()
				+ <Test as Config>::CurrentMigration::max_step_weight();
			assert_eq!(used_weight, expected_weight);
			assert_eq!(
				Status::<Test>::get(),
				MigrationStatus::InProgress { steps_done: block_limit }
			);
			verify_new_data(data_count, Some(block_limit));

			// No migration for blocks 6 to 9
			for block_number in 6..10 {
				let used_weight = Migration::on_idle(block_number, Weight::from_all(u64::MAX));
				assert_eq!(used_weight, DbWeight::get().reads(3));
				assert_eq!(
					Status::<Test>::get(),
					MigrationStatus::InProgress { steps_done: block_limit }
				);
				verify_new_data(data_count, Some(block_limit));
			}

			// Migration should process one step on block 10
			let block_number = 10;
			let used_weight = Migration::on_idle(block_number, Weight::from_all(u64::MAX));
			assert_eq!(used_weight, expected_weight);
			assert_eq!(
				Status::<Test>::get(),
				MigrationStatus::InProgress { steps_done: block_limit * 2 }
			);
			verify_new_data(data_count, Some(block_limit * 2));
		});
	}
}
