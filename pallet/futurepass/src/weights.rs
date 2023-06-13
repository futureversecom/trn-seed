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
//! DATE: 2023-06-05, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `fedora`, CPU: `13th Gen Intel(R) Core(TM) i7-13700K`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_futurepass
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./output/pallet_futurepass_weights.rs
// --template
// ./scripts/pallet_template.hbs
// --output
// ./output

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_futurepass.
pub trait WeightInfo {
	fn create() -> Weight;
	fn register_delegate_with_signature(p: u32) -> Weight;
	fn unregister_delegate(p: u32) -> Weight;
	fn transfer_futurepass(p: u32,) -> Weight;
	fn proxy_extrinsic(p: u32,) -> Weight;
}

/// Weights for pallet_futurepass using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: Futurepass Holders (r:1 w:1)
	// Storage: Futurepass NextFuturepassId (r:1 w:1)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	fn create() -> Weight {
		Weight::from_ref_time(46_228_000 as u64)
			.saturating_add(T::DbWeight::get().reads(4 as u64))
			.saturating_add(T::DbWeight::get().writes(4 as u64))
	}
	// Storage: Futurepass Holders (r:1 w:0)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	fn register_delegate_with_signature(p: u32, ) -> Weight {
		(70_000_000 as Weight)
			.saturating_add((76_000 as Weight).saturating_mul(p as u64))
			.saturating_add(RocksDbWeight::get().reads(4 as u64))
			.saturating_add(RocksDbWeight::get().writes(3 as u64))
	}
	// Storage: Futurepass Holders (r:2 w:0)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	fn unregister_delegate(p: u32,) -> Weight {
		Weight::from_ref_time(46_361_000 as u64)
			.saturating_add(T::DbWeight::get().reads(5 as u64))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
	}
	// Storage: Futurepass Holders (r:2 w:2)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	fn transfer_futurepass(p: u32,) -> Weight {
		Weight::from_ref_time(66_911_000 as u64)
			.saturating_add(T::DbWeight::get().reads(5 as u64))
			.saturating_add(T::DbWeight::get().writes(5 as u64))
	}
	// Storage: Proxy Proxies (r:1 w:0)
	fn proxy_extrinsic(p: u32,) -> Weight {
		Weight::from_ref_time(19_615_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: Futurepass Holders (r:1 w:1)
	// Storage: Futurepass NextFuturepassId (r:1 w:1)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	fn create() -> Weight {
		Weight::from_ref_time(46_228_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(4 as u64))
			.saturating_add(RocksDbWeight::get().writes(4 as u64))
	}
	// Storage: Futurepass Holders (r:1 w:0)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	fn register_delegate_with_signature(p: u32, ) -> Weight {
		(70_000_000 as Weight)
			.saturating_add((76_000 as Weight).saturating_mul(p as u64))
			.saturating_add(RocksDbWeight::get().reads(4 as u64))
			.saturating_add(RocksDbWeight::get().writes(3 as u64))
	}
	// Storage: Futurepass Holders (r:1 w:0)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	fn unregister_delegate(p: u32, ) -> Weight {
		(69_000_000 as Weight)
			.saturating_add((76_000 as Weight).saturating_mul(p as u64))
			.saturating_add(RocksDbWeight::get().reads(4 as u64))
			.saturating_add(RocksDbWeight::get().writes(3 as u64))
	}
	// Storage: Futurepass Holders (r:2 w:2)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	fn transfer_futurepass(p: u32, ) -> Weight {
		Weight::from_ref_time(66_911_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(5 as u64))
			.saturating_add(RocksDbWeight::get().writes(5 as u64))
	}
	// Storage: Proxy Proxies (r:1 w:0)
	fn proxy_extrinsic(p: u32, ) -> Weight {
		Weight::from_ref_time(19_615_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
	}
}
