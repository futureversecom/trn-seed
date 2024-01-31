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

//! Autogenerated weights for pallet_doughnut
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-01-23, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `Jasons-Ubuntu`, CPU: `AMD Ryzen 9 7950X 16-Core Processor`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_doughnut
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./pallet/doughnut/src/weights.rs
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_doughnut.
pub trait WeightInfo {
	fn transact() -> Weight;
	fn revoke_doughnut() -> Weight;
	fn revoke_holder() -> Weight;
}

/// Weights for pallet_fee_control using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: Doughnut BlockedDoughnuts (r:1 w:0)
	// Storage: Doughnut BlockedHolders (r:1 w:0)
	// Storage: MaintenanceMode BlockedCalls (r:1 w:0)
	// Storage: MaintenanceMode BlockedPallets (r:1 w:0)
	fn transact() -> Weight {
		Weight::from_ref_time(192_694_000 as u64)
			.saturating_add(T::DbWeight::get().reads(4 as u64))
	}
	// Storage: Doughnut BlockedDoughnuts (r:0 w:1)
	fn revoke_doughnut() -> Weight {
		Weight::from_ref_time(13_706_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Doughnut BlockedHolders (r:0 w:1)
	fn revoke_holder() -> Weight {
		Weight::from_ref_time(4_678_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: Doughnut BlockedDoughnuts (r:1 w:0)
	// Storage: Doughnut BlockedHolders (r:1 w:0)
	// Storage: MaintenanceMode BlockedCalls (r:1 w:0)
	// Storage: MaintenanceMode BlockedPallets (r:1 w:0)
	fn transact() -> Weight {
		Weight::from_ref_time(192_694_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(4 as u64))
	}
	// Storage: Doughnut BlockedDoughnuts (r:0 w:1)
	fn revoke_doughnut() -> Weight {
		Weight::from_ref_time(13_706_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Doughnut BlockedHolders (r:0 w:1)
	fn revoke_holder() -> Weight {
		Weight::from_ref_time(4_678_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
}

