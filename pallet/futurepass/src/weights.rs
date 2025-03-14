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

//! Autogenerated weights for pallet_futurepass
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
// --pallet=pallet-futurepass
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./pallet/futurepass/src/weights.rs
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_futurepass.
pub trait WeightInfo {
	fn create() -> Weight;
	fn register_delegate_with_signature(p: u32, ) -> Weight;
	fn unregister_delegate(p: u32, ) -> Weight;
	fn transfer_futurepass(p: u32, ) -> Weight;
	fn proxy_extrinsic(p: u32, ) -> Weight;
}

/// Weights for pallet_futurepass using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: `Futurepass::Holders` (r:1 w:1)
	// Proof: `Futurepass::Holders` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	// Storage: `Futurepass::NextFuturepassId` (r:1 w:1)
	// Proof: `Futurepass::NextFuturepassId` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	// Storage: `Proxy::Proxies` (r:1 w:1)
	// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(845), added: 3320, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:1 w:1)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	fn create() -> Weight {
		Weight::from_all(196_195_000_u64)
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}
	// Storage: `Futurepass::Holders` (r:1 w:0)
	// Proof: `Futurepass::Holders` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	// Storage: `Proxy::Proxies` (r:1 w:1)
	// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(845), added: 3320, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:2 w:2)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn register_delegate_with_signature(p: u32, ) -> Weight {
		Weight::from_all(270_996_791_u64)
			// Standard Error: 11_734
			.saturating_add(Weight::from_all(213_467_u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	// Storage: `Futurepass::Holders` (r:2 w:0)
	// Proof: `Futurepass::Holders` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	// Storage: `Proxy::Proxies` (r:1 w:1)
	// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(845), added: 3320, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:2 w:2)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn unregister_delegate(p: u32, ) -> Weight {
		Weight::from_all(194_326_623_u64)
			// Standard Error: 6_500
			.saturating_add(Weight::from_all(270_944_u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads(5_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	// Storage: `Futurepass::Holders` (r:2 w:2)
	// Proof: `Futurepass::Holders` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	// Storage: `Proxy::Proxies` (r:1 w:1)
	// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(845), added: 3320, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:2 w:2)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn transfer_futurepass(p: u32, ) -> Weight {
		Weight::from_all(191_033_203_u64)
			// Standard Error: 97_820
			.saturating_add(Weight::from_all(109_262_745_u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads(5_u64))
			.saturating_add(T::DbWeight::get().writes(5_u64))
	}
	// Storage: `MaintenanceMode::BlockedCalls` (r:2 w:0)
	// Proof: `MaintenanceMode::BlockedCalls` (`max_values`: None, `max_size`: Some(111), added: 2586, mode: `MaxEncodedLen`)
	// Storage: `MaintenanceMode::BlockedPallets` (r:2 w:0)
	// Proof: `MaintenanceMode::BlockedPallets` (`max_values`: None, `max_size`: Some(60), added: 2535, mode: `MaxEncodedLen`)
	// Storage: `Proxy::Proxies` (r:1 w:0)
	// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(845), added: 3320, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn proxy_extrinsic(p: u32, ) -> Weight {
		Weight::from_all(89_612_094_u64)
			// Standard Error: 4_867
			.saturating_add(Weight::from_all(89_411_u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads(5_u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: `Futurepass::Holders` (r:1 w:1)
	// Proof: `Futurepass::Holders` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	// Storage: `Futurepass::NextFuturepassId` (r:1 w:1)
	// Proof: `Futurepass::NextFuturepassId` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	// Storage: `Proxy::Proxies` (r:1 w:1)
	// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(845), added: 3320, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:1 w:1)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	fn create() -> Weight {
		Weight::from_all(196_195_000_u64)
			.saturating_add(RocksDbWeight::get().reads(4_u64))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
	}
	// Storage: `Futurepass::Holders` (r:1 w:0)
	// Proof: `Futurepass::Holders` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	// Storage: `Proxy::Proxies` (r:1 w:1)
	// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(845), added: 3320, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:2 w:2)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn register_delegate_with_signature(p: u32, ) -> Weight {
		Weight::from_all(270_996_791_u64)
			// Standard Error: 11_734
			.saturating_add(Weight::from_all(213_467_u64).saturating_mul(p as u64))
			.saturating_add(RocksDbWeight::get().reads(4_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}
	// Storage: `Futurepass::Holders` (r:2 w:0)
	// Proof: `Futurepass::Holders` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	// Storage: `Proxy::Proxies` (r:1 w:1)
	// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(845), added: 3320, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:2 w:2)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn unregister_delegate(p: u32, ) -> Weight {
		Weight::from_all(194_326_623_u64)
			// Standard Error: 6_500
			.saturating_add(Weight::from_all(270_944_u64).saturating_mul(p as u64))
			.saturating_add(RocksDbWeight::get().reads(5_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}
	// Storage: `Futurepass::Holders` (r:2 w:2)
	// Proof: `Futurepass::Holders` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	// Storage: `Proxy::Proxies` (r:1 w:1)
	// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(845), added: 3320, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:2 w:2)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn transfer_futurepass(p: u32, ) -> Weight {
		Weight::from_all(191_033_203_u64)
			// Standard Error: 97_820
			.saturating_add(Weight::from_all(109_262_745_u64).saturating_mul(p as u64))
			.saturating_add(RocksDbWeight::get().reads(5_u64))
			.saturating_add(RocksDbWeight::get().writes(5_u64))
	}
	// Storage: `MaintenanceMode::BlockedCalls` (r:2 w:0)
	// Proof: `MaintenanceMode::BlockedCalls` (`max_values`: None, `max_size`: Some(111), added: 2586, mode: `MaxEncodedLen`)
	// Storage: `MaintenanceMode::BlockedPallets` (r:2 w:0)
	// Proof: `MaintenanceMode::BlockedPallets` (`max_values`: None, `max_size`: Some(60), added: 2535, mode: `MaxEncodedLen`)
	// Storage: `Proxy::Proxies` (r:1 w:0)
	// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(845), added: 3320, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn proxy_extrinsic(p: u32, ) -> Weight {
		Weight::from_all(89_612_094_u64)
			// Standard Error: 4_867
			.saturating_add(Weight::from_all(89_411_u64).saturating_mul(p as u64))
			.saturating_add(RocksDbWeight::get().reads(5_u64))
	}
}

