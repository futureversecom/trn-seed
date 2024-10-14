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

use super::*;
#[allow(unused_imports)]
use crate::Pallet as Migration;
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::StorageHasher;
use frame_system::RawOrigin;

benchmarks! {
	// This benchmarks the weight of dispatching migrate to execute 1 `NoopMigraton` step
	#[pov_mode = Measured]
	migrate {
		let weight_limit = T::MaxMigrationWeight::get();
		Status::<T>::put(MigrationStatus::InProgress { steps_done: 0 });
		MigrationEnabled::<T>::put(true);
	}: {
		Migration::<T>::migrate(weight_limit)
	} verify {
		assert_eq!( Status::<T>::get(), MigrationStatus::Completed);
	}

	#[pov_mode = Measured]
	current_migration_step {
		MigrationEnabled::<T>::put(true);
		let mut key = Twox64Concat::hash(&(1 as CollectionUuid).encode());
		let serial_key = Twox64Concat::hash(&(2 as SerialNumber).encode());
		key.extend_from_slice(&serial_key);
		let xls20_token_id: [u8; 64] = "000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d67".as_bytes().try_into().unwrap();
		frame_support::migration::put_storage_value::<[u8; 64]>(b"Xls20", b"Xls20TokenMap", &key, xls20_token_id);
		Status::<T>::put(MigrationStatus::InProgress { steps_done: 0 });
	}: {
		T::CurrentMigration::step(None)
	} verify {
		let new_token = frame_support::migration::get_storage_value::<[u8; 32]>(b"Xls20", b"Xls20TokenMap", &key);
		let xls20_token_id: [u8; 32] = "000b013a95f14b0e44f78a264e41713c".as_bytes().try_into().unwrap();
		assert_eq!(new_token, Some(xls20_token_id));
	}

	enable_migration {
		let enabled = true;
	}: _(RawOrigin::Root, enabled)
	verify {
		assert!(MigrationEnabled::<T>::get());
	}
}

impl_benchmark_test_suite!(
	Migration,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);