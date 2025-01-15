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

//! Autogenerated weights for pallet_sylo
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2025-01-15, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `Johns-Macbook-Pro.local`, CPU: `<UNKNOWN>`
//! EXECUTION: , WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-sylo
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./pallet/sylo/src/weights.rs
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_sylo.
pub trait WeightInfo {
	fn set_payment_asset() -> Weight;
	fn set_sylo_resolver_method() -> Weight;
	fn register_resolver(p: u32, q: u32, ) -> Weight;
	fn update_resolver(p: u32, q: u32, ) -> Weight;
	fn deregister_resolver() -> Weight;
	fn create_validation_record(p: u32, q: u32, r: u32, ) -> Weight;
	fn add_validation_record_entry() -> Weight;
	fn update_validation_record(p: u32, q: u32, r: u32, ) -> Weight;
	fn delete_validation_record() -> Weight;
}

/// Weights for pallet_sylo using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: `Sylo::SyloAssetId` (r:0 w:1)
	// Proof: `Sylo::SyloAssetId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn set_payment_asset() -> Weight {
		Weight::from_all(9_000_000)
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `Sylo::SyloResolverMethod` (r:0 w:1)
	// Proof: `Sylo::SyloResolverMethod` (`max_values`: Some(1), `max_size`: Some(502), added: 997, mode: `MaxEncodedLen`)
	fn set_sylo_resolver_method() -> Weight {
		Weight::from_all(9_000_000)
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `Sylo::Resolvers` (r:1 w:1)
	// Proof: `Sylo::Resolvers` (`max_values`: None, `max_size`: Some(5551), added: 8026, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 250]`.
	/// The range of component `q` is `[1, 10]`.
	fn register_resolver(p: u32, q: u32, ) -> Weight {
		Weight::from_all(10_245_832)
			// Standard Error: 280
			.saturating_add(Weight::from_all(12_290_u64).saturating_mul(p as u64))
			// Standard Error: 7_321
			.saturating_add(Weight::from_all(682_226_u64).saturating_mul(q as u64))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `Sylo::Resolvers` (r:1 w:1)
	// Proof: `Sylo::Resolvers` (`max_values`: None, `max_size`: Some(5551), added: 8026, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 250]`.
	/// The range of component `q` is `[1, 10]`.
	fn update_resolver(p: u32, q: u32, ) -> Weight {
		Weight::from_all(11_883_855)
			// Standard Error: 308
			.saturating_add(Weight::from_all(12_707_u64).saturating_mul(p as u64))
			// Standard Error: 8_037
			.saturating_add(Weight::from_all(670_881_u64).saturating_mul(q as u64))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `Sylo::Resolvers` (r:1 w:1)
	// Proof: `Sylo::Resolvers` (`max_values`: None, `max_size`: Some(5551), added: 8026, mode: `MaxEncodedLen`)
	fn deregister_resolver() -> Weight {
		Weight::from_all(16_000_000)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `Sylo::ValidationRecords` (r:1 w:1)
	// Proof: `Sylo::ValidationRecords` (`max_values`: None, `max_size`: Some(19724), added: 22199, mode: `MaxEncodedLen`)
	// Storage: `Sylo::SyloResolverMethod` (r:1 w:0)
	// Proof: `Sylo::SyloResolverMethod` (`max_values`: Some(1), `max_size`: Some(502), added: 997, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 250]`.
	/// The range of component `q` is `[1, 10]`.
	/// The range of component `r` is `[1, 10]`.
	fn create_validation_record(p: u32, q: u32, r: u32, ) -> Weight {
		Weight::from_all(12_583_999)
			// Standard Error: 325
			.saturating_add(Weight::from_all(18_414_u64).saturating_mul(p as u64))
			// Standard Error: 8_482
			.saturating_add(Weight::from_all(839_885_u64).saturating_mul(q as u64))
			// Standard Error: 8_482
			.saturating_add(Weight::from_all(376_512_u64).saturating_mul(r as u64))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `Sylo::ValidationRecords` (r:1 w:1)
	// Proof: `Sylo::ValidationRecords` (`max_values`: None, `max_size`: Some(19724), added: 22199, mode: `MaxEncodedLen`)
	fn add_validation_record_entry() -> Weight {
		Weight::from_all(17_000_000)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `Sylo::ValidationRecords` (r:1 w:1)
	// Proof: `Sylo::ValidationRecords` (`max_values`: None, `max_size`: Some(19724), added: 22199, mode: `MaxEncodedLen`)
	// Storage: `Sylo::SyloResolverMethod` (r:1 w:0)
	// Proof: `Sylo::SyloResolverMethod` (`max_values`: Some(1), `max_size`: Some(502), added: 997, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 250]`.
	/// The range of component `q` is `[1, 10]`.
	/// The range of component `r` is `[1, 10]`.
	fn update_validation_record(p: u32, q: u32, r: u32, ) -> Weight {
		Weight::from_all(2_813_514)
			// Standard Error: 544
			.saturating_add(Weight::from_all(57_067_u64).saturating_mul(p as u64))
			// Standard Error: 14_176
			.saturating_add(Weight::from_all(2_829_596_u64).saturating_mul(q as u64))
			// Standard Error: 14_176
			.saturating_add(Weight::from_all(670_440_u64).saturating_mul(r as u64))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `Sylo::ValidationRecords` (r:1 w:1)
	// Proof: `Sylo::ValidationRecords` (`max_values`: None, `max_size`: Some(19724), added: 22199, mode: `MaxEncodedLen`)
	fn delete_validation_record() -> Weight {
		Weight::from_all(16_000_000)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: `Sylo::SyloAssetId` (r:0 w:1)
	// Proof: `Sylo::SyloAssetId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn set_payment_asset() -> Weight {
		Weight::from_all(9_000_000)
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `Sylo::SyloResolverMethod` (r:0 w:1)
	// Proof: `Sylo::SyloResolverMethod` (`max_values`: Some(1), `max_size`: Some(502), added: 997, mode: `MaxEncodedLen`)
	fn set_sylo_resolver_method() -> Weight {
		Weight::from_all(9_000_000)
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `Sylo::Resolvers` (r:1 w:1)
	// Proof: `Sylo::Resolvers` (`max_values`: None, `max_size`: Some(5551), added: 8026, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 250]`.
	/// The range of component `q` is `[1, 10]`.
	fn register_resolver(p: u32, q: u32, ) -> Weight {
		Weight::from_all(10_245_832)
			// Standard Error: 280
			.saturating_add(Weight::from_all(12_290_u64).saturating_mul(p as u64))
			// Standard Error: 7_321
			.saturating_add(Weight::from_all(682_226_u64).saturating_mul(q as u64))
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `Sylo::Resolvers` (r:1 w:1)
	// Proof: `Sylo::Resolvers` (`max_values`: None, `max_size`: Some(5551), added: 8026, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 250]`.
	/// The range of component `q` is `[1, 10]`.
	fn update_resolver(p: u32, q: u32, ) -> Weight {
		Weight::from_all(11_883_855)
			// Standard Error: 308
			.saturating_add(Weight::from_all(12_707_u64).saturating_mul(p as u64))
			// Standard Error: 8_037
			.saturating_add(Weight::from_all(670_881_u64).saturating_mul(q as u64))
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `Sylo::Resolvers` (r:1 w:1)
	// Proof: `Sylo::Resolvers` (`max_values`: None, `max_size`: Some(5551), added: 8026, mode: `MaxEncodedLen`)
	fn deregister_resolver() -> Weight {
		Weight::from_all(16_000_000)
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `Sylo::ValidationRecords` (r:1 w:1)
	// Proof: `Sylo::ValidationRecords` (`max_values`: None, `max_size`: Some(19724), added: 22199, mode: `MaxEncodedLen`)
	// Storage: `Sylo::SyloResolverMethod` (r:1 w:0)
	// Proof: `Sylo::SyloResolverMethod` (`max_values`: Some(1), `max_size`: Some(502), added: 997, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 250]`.
	/// The range of component `q` is `[1, 10]`.
	/// The range of component `r` is `[1, 10]`.
	fn create_validation_record(p: u32, q: u32, r: u32, ) -> Weight {
		Weight::from_all(12_583_999)
			// Standard Error: 325
			.saturating_add(Weight::from_all(18_414_u64).saturating_mul(p as u64))
			// Standard Error: 8_482
			.saturating_add(Weight::from_all(839_885_u64).saturating_mul(q as u64))
			// Standard Error: 8_482
			.saturating_add(Weight::from_all(376_512_u64).saturating_mul(r as u64))
			.saturating_add(RocksDbWeight::get().reads(2))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `Sylo::ValidationRecords` (r:1 w:1)
	// Proof: `Sylo::ValidationRecords` (`max_values`: None, `max_size`: Some(19724), added: 22199, mode: `MaxEncodedLen`)
	fn add_validation_record_entry() -> Weight {
		Weight::from_all(17_000_000)
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `Sylo::ValidationRecords` (r:1 w:1)
	// Proof: `Sylo::ValidationRecords` (`max_values`: None, `max_size`: Some(19724), added: 22199, mode: `MaxEncodedLen`)
	// Storage: `Sylo::SyloResolverMethod` (r:1 w:0)
	// Proof: `Sylo::SyloResolverMethod` (`max_values`: Some(1), `max_size`: Some(502), added: 997, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 250]`.
	/// The range of component `q` is `[1, 10]`.
	/// The range of component `r` is `[1, 10]`.
	fn update_validation_record(p: u32, q: u32, r: u32, ) -> Weight {
		Weight::from_all(2_813_514)
			// Standard Error: 544
			.saturating_add(Weight::from_all(57_067_u64).saturating_mul(p as u64))
			// Standard Error: 14_176
			.saturating_add(Weight::from_all(2_829_596_u64).saturating_mul(q as u64))
			// Standard Error: 14_176
			.saturating_add(Weight::from_all(670_440_u64).saturating_mul(r as u64))
			.saturating_add(RocksDbWeight::get().reads(2))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `Sylo::ValidationRecords` (r:1 w:1)
	// Proof: `Sylo::ValidationRecords` (`max_values`: None, `max_size`: Some(19724), added: 22199, mode: `MaxEncodedLen`)
	fn delete_validation_record() -> Weight {
		Weight::from_all(16_000_000)
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
}

