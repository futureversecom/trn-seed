// This file is part of Substrate.

// Copyright (C) 2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Autogenerated weights for pallet_migration
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-10-29, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `Jasons-Ubuntu`, CPU: `AMD Ryzen 9 7950X 16-Core Processor`
//! EXECUTION: , WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-migration
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./pallet/migration/src/weights.rs
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_migration.
pub trait WeightInfo {
	fn migrate() -> Weight;
	fn current_migration_step() -> Weight;
	fn enable_migration() -> Weight;
	fn set_block_delay() -> Weight;
	fn set_block_limit() -> Weight;
}

/// Weights for pallet_migration using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: `Migration::Status` (r:1 w:1)
	// Proof: `Migration::Status` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `Migration::MigrationEnabled` (r:1 w:0)
	// Proof: `Migration::MigrationEnabled` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `Migration::BlockDelay` (r:1 w:0)
	// Proof: `Migration::BlockDelay` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `Migration::BlockLimit` (r:1 w:0)
	// Proof: `Migration::BlockLimit` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `Migration::LastKey` (r:1 w:1)
	// Proof: `Migration::LastKey` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `Xls20::Xls20TokenMap` (r:1 w:0)
	// Proof: `Xls20::Xls20TokenMap` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	// Storage: UNKNOWN KEY `0x28fc2cbf777640e8e3e472d285713c8d4e7b9012096b41c4eb3aaf947f6ea429` (r:0 w:1)
	// Proof: UNKNOWN KEY `0x28fc2cbf777640e8e3e472d285713c8d4e7b9012096b41c4eb3aaf947f6ea429` (r:0 w:1)
	fn migrate() -> Weight {
		Weight::from_all(13_436_000 as u64)
			.saturating_add(T::DbWeight::get().reads(6 as u64))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
	}
	// Storage: `Xls20::Xls20TokenMap` (r:2 w:1)
	// Proof: `Xls20::Xls20TokenMap` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	fn current_migration_step() -> Weight {
		Weight::from_all(8_647_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: `Migration::MigrationEnabled` (r:0 w:1)
	// Proof: `Migration::MigrationEnabled` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn enable_migration() -> Weight {
		Weight::from_all(5_440_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: `Migration::BlockDelay` (r:0 w:1)
	// Proof: `Migration::BlockDelay` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn set_block_delay() -> Weight {
		Weight::from_all(5_420_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: `Migration::BlockLimit` (r:0 w:1)
	// Proof: `Migration::BlockLimit` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn set_block_limit() -> Weight {
		Weight::from_all(5_491_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: `Migration::Status` (r:1 w:1)
	// Proof: `Migration::Status` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `Migration::MigrationEnabled` (r:1 w:0)
	// Proof: `Migration::MigrationEnabled` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `Migration::BlockDelay` (r:1 w:0)
	// Proof: `Migration::BlockDelay` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `Migration::BlockLimit` (r:1 w:0)
	// Proof: `Migration::BlockLimit` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `Migration::LastKey` (r:1 w:1)
	// Proof: `Migration::LastKey` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `Xls20::Xls20TokenMap` (r:1 w:0)
	// Proof: `Xls20::Xls20TokenMap` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	// Storage: UNKNOWN KEY `0x28fc2cbf777640e8e3e472d285713c8d4e7b9012096b41c4eb3aaf947f6ea429` (r:0 w:1)
	// Proof: UNKNOWN KEY `0x28fc2cbf777640e8e3e472d285713c8d4e7b9012096b41c4eb3aaf947f6ea429` (r:0 w:1)
	fn migrate() -> Weight {
		Weight::from_all(13_436_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(6 as u64))
			.saturating_add(RocksDbWeight::get().writes(3 as u64))
	}
	// Storage: `Xls20::Xls20TokenMap` (r:2 w:1)
	// Proof: `Xls20::Xls20TokenMap` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	fn current_migration_step() -> Weight {
		Weight::from_all(8_647_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(2 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: `Migration::MigrationEnabled` (r:0 w:1)
	// Proof: `Migration::MigrationEnabled` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn enable_migration() -> Weight {
		Weight::from_all(5_440_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: `Migration::BlockDelay` (r:0 w:1)
	// Proof: `Migration::BlockDelay` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn set_block_delay() -> Weight {
		Weight::from_all(5_420_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: `Migration::BlockLimit` (r:0 w:1)
	// Proof: `Migration::BlockLimit` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn set_block_limit() -> Weight {
		Weight::from_all(5_491_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
}

