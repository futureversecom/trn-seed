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
//! DATE: 2024-05-23, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-102-147`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: , WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-doughnut
// --extrinsic=*
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
	fn update_whitelisted_holders() -> Weight;
}

/// Weights for pallet_doughnut using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: `Doughnut::WhitelistedHolders` (r:1 w:0)
	// Proof: `Doughnut::WhitelistedHolders` (`max_values`: None, `max_size`: Some(29), added: 2504, mode: `MaxEncodedLen`)
	// Storage: `Doughnut::BlockedDoughnuts` (r:1 w:0)
	// Proof: `Doughnut::BlockedDoughnuts` (`max_values`: None, `max_size`: Some(41), added: 2516, mode: `MaxEncodedLen`)
	// Storage: `Doughnut::BlockedHolders` (r:1 w:0)
	// Proof: `Doughnut::BlockedHolders` (`max_values`: None, `max_size`: Some(57), added: 2532, mode: `MaxEncodedLen`)
	// Storage: `MaintenanceMode::BlockedCalls` (r:1 w:0)
	// Proof: `MaintenanceMode::BlockedCalls` (`max_values`: None, `max_size`: Some(111), added: 2586, mode: `MaxEncodedLen`)
	// Storage: `MaintenanceMode::BlockedPallets` (r:1 w:0)
	// Proof: `MaintenanceMode::BlockedPallets` (`max_values`: None, `max_size`: Some(60), added: 2535, mode: `MaxEncodedLen`)
	fn transact() -> Weight {
		Weight::from_all(514_937_000_u64)
			.saturating_add(T::DbWeight::get().reads(5_u64))
	}
	// Storage: `Doughnut::BlockedDoughnuts` (r:0 w:1)
	// Proof: `Doughnut::BlockedDoughnuts` (`max_values`: None, `max_size`: Some(41), added: 2516, mode: `MaxEncodedLen`)
	fn revoke_doughnut() -> Weight {
		Weight::from_all(54_500_000_u64)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	// Storage: `Doughnut::BlockedHolders` (r:0 w:1)
	// Proof: `Doughnut::BlockedHolders` (`max_values`: None, `max_size`: Some(57), added: 2532, mode: `MaxEncodedLen`)
	fn revoke_holder() -> Weight {
		Weight::from_all(27_483_000_u64)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	// Storage: `Doughnut::WhitelistedHolders` (r:0 w:1)
	// Proof: `Doughnut::WhitelistedHolders` (`max_values`: None, `max_size`: Some(29), added: 2504, mode: `MaxEncodedLen`)
	fn update_whitelisted_holders() -> Weight {
		Weight::from_all(26_157_000_u64)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: `Doughnut::WhitelistedHolders` (r:1 w:0)
	// Proof: `Doughnut::WhitelistedHolders` (`max_values`: None, `max_size`: Some(29), added: 2504, mode: `MaxEncodedLen`)
	// Storage: `Doughnut::BlockedDoughnuts` (r:1 w:0)
	// Proof: `Doughnut::BlockedDoughnuts` (`max_values`: None, `max_size`: Some(41), added: 2516, mode: `MaxEncodedLen`)
	// Storage: `Doughnut::BlockedHolders` (r:1 w:0)
	// Proof: `Doughnut::BlockedHolders` (`max_values`: None, `max_size`: Some(57), added: 2532, mode: `MaxEncodedLen`)
	// Storage: `MaintenanceMode::BlockedCalls` (r:1 w:0)
	// Proof: `MaintenanceMode::BlockedCalls` (`max_values`: None, `max_size`: Some(111), added: 2586, mode: `MaxEncodedLen`)
	// Storage: `MaintenanceMode::BlockedPallets` (r:1 w:0)
	// Proof: `MaintenanceMode::BlockedPallets` (`max_values`: None, `max_size`: Some(60), added: 2535, mode: `MaxEncodedLen`)
	fn transact() -> Weight {
		Weight::from_all(514_937_000_u64)
			.saturating_add(RocksDbWeight::get().reads(5_u64))
	}
	// Storage: `Doughnut::BlockedDoughnuts` (r:0 w:1)
	// Proof: `Doughnut::BlockedDoughnuts` (`max_values`: None, `max_size`: Some(41), added: 2516, mode: `MaxEncodedLen`)
	fn revoke_doughnut() -> Weight {
		Weight::from_all(54_500_000_u64)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	// Storage: `Doughnut::BlockedHolders` (r:0 w:1)
	// Proof: `Doughnut::BlockedHolders` (`max_values`: None, `max_size`: Some(57), added: 2532, mode: `MaxEncodedLen`)
	fn revoke_holder() -> Weight {
		Weight::from_all(27_483_000_u64)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	// Storage: `Doughnut::WhitelistedHolders` (r:0 w:1)
	// Proof: `Doughnut::WhitelistedHolders` (`max_values`: None, `max_size`: Some(29), added: 2504, mode: `MaxEncodedLen`)
	fn update_whitelisted_holders() -> Weight {
		Weight::from_all(26_157_000_u64)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
}

