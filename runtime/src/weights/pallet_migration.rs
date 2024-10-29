
//! Autogenerated weights for `pallet_migration`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-10-29, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `Jasons-Ubuntu`, CPU: `AMD Ryzen 9 7950X 16-Core Processor`
//! EXECUTION: ``, WASM-EXECUTION: `Compiled`, CHAIN: `Some("dev")`, DB CACHE: 1024

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
// ./runtime/src/weights/pallet_migration.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_migration`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_migration::WeightInfo for WeightInfo<T> {
	/// Storage: `Migration::Status` (r:1 w:1)
	/// Proof: `Migration::Status` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Migration::MigrationEnabled` (r:1 w:0)
	/// Proof: `Migration::MigrationEnabled` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Migration::BlockDelay` (r:1 w:0)
	/// Proof: `Migration::BlockDelay` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Migration::BlockLimit` (r:1 w:0)
	/// Proof: `Migration::BlockLimit` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Migration::LastKey` (r:1 w:1)
	/// Proof: `Migration::LastKey` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Xls20::Xls20TokenMap` (r:1 w:0)
	/// Proof: `Xls20::Xls20TokenMap` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// Storage: UNKNOWN KEY `0x28fc2cbf777640e8e3e472d285713c8d4e7b9012096b41c4eb3aaf947f6ea429` (r:0 w:1)
	/// Proof: UNKNOWN KEY `0x28fc2cbf777640e8e3e472d285713c8d4e7b9012096b41c4eb3aaf947f6ea429` (r:0 w:1)
	fn migrate() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `98`
		//  Estimated: `3521`
		// Minimum execution time: 12_664_000 picoseconds.
		Weight::from_parts(13_225_000, 0)
			.saturating_add(Weight::from_parts(0, 3521))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Xls20::Xls20TokenMap` (r:2 w:1)
	/// Proof: `Xls20::Xls20TokenMap` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	fn current_migration_step() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `120`
		//  Estimated: `6052`
		// Minimum execution time: 8_286_000 picoseconds.
		Weight::from_parts(8_536_000, 0)
			.saturating_add(Weight::from_parts(0, 6052))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Migration::MigrationEnabled` (r:0 w:1)
	/// Proof: `Migration::MigrationEnabled` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn enable_migration() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 5_240_000 picoseconds.
		Weight::from_parts(5_490_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Migration::BlockDelay` (r:0 w:1)
	/// Proof: `Migration::BlockDelay` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn set_block_delay() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 5_240_000 picoseconds.
		Weight::from_parts(5_521_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Migration::BlockLimit` (r:0 w:1)
	/// Proof: `Migration::BlockLimit` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn set_block_limit() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 5_430_000 picoseconds.
		Weight::from_parts(5_531_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
