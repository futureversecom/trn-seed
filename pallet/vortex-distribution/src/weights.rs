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

//! Autogenerated weights for pallet_vortex
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-10-19, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `Dev-MBP-2`, CPU: `<UNKNOWN>`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps
// 50
// --repeat
// 20
// --pallet
// pallet_vortex
// --extrinsic
// *
// --wasm-execution=compiled
// --output
// ./pallet/vortex-distribution/src/weights.rs
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_vortex.
pub trait WeightInfo {
	fn create_vtx_dist() -> Weight;
	fn disable_vtx_dist() -> Weight;
	fn start_vtx_dist() -> Weight;
	fn set_vtx_dist_eras() -> Weight;
	fn set_asset_prices(b: u32, ) -> Weight;
	fn register_rewards() -> Weight;
	fn trigger_vtx_distribution() -> Weight;
	fn redeem_tokens_from_vault() -> Weight;
	fn pay_unsigned() -> Weight;
}

/// Weights for pallet_vortex using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: VortexDistribution NextVortexId (r:1 w:1)
	// Storage: VortexDistribution VtxDistStatuses (r:0 w:1)
	// Storage: VortexDistribution TotalVortex (r:0 w:1)
	fn create_vtx_dist() -> Weight {
		Weight::from_ref_time(10_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
	}
	// Storage: VortexDistribution VtxDistStatuses (r:1 w:1)
	fn disable_vtx_dist() -> Weight {
		Weight::from_ref_time(11_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: VortexDistribution VtxDistStatuses (r:1 w:1)
	fn start_vtx_dist() -> Weight {
		Weight::from_ref_time(13_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: VortexDistribution VtxDistEras (r:0 w:1)
	fn set_vtx_dist_eras() -> Weight {
		Weight::from_ref_time(8_000_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: VortexDistribution AssetPrices (r:0 w:1)
	/// The range of component `b` is `[1, 1000]`.
	fn set_asset_prices(b: u32, ) -> Weight {
		Weight::from_ref_time(9_000_000 as u64)
			// Standard Error: 1_608
			.saturating_add(Weight::from_ref_time(743_068 as u64).saturating_mul(b as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
			.saturating_add(T::DbWeight::get().writes((1 as u64).saturating_mul(b as u64)))
	}
	// Storage: VortexDistribution VtxDistStatuses (r:1 w:0)
	// Storage: VortexDistribution TotalVortex (r:1 w:0)
	// Storage: VortexDistribution VtxDistOrderbook (r:1 w:1)
	fn register_rewards() -> Weight {
		Weight::from_ref_time(14_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(3 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: VortexDistribution VtxDistStatuses (r:1 w:1)
	// Storage: VortexDistribution TotalVortex (r:1 w:0)
	// Storage: VortexDistribution AssetPrices (r:2 w:0)
	// Storage: Assets Account (r:2 w:2)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: System Account (r:3 w:3)
	fn trigger_vtx_distribution() -> Weight {
		Weight::from_ref_time(47_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(10 as u64))
			.saturating_add(T::DbWeight::get().writes(7 as u64))
	}
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:1)
	// Storage: VortexDistribution AssetPrices (r:2 w:0)
	// Storage: System Account (r:1 w:0)
	fn redeem_tokens_from_vault() -> Weight {
		Weight::from_ref_time(33_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(6 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: VortexDistribution VtxDistStatuses (r:1 w:0)
	// Storage: VortexDistribution VtxDistPayoutPivot (r:1 w:1)
	// Storage: VortexDistribution VtxDistOrderbook (r:2 w:1)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: VortexDistribution NextUnsignedAt (r:0 w:1)
	fn pay_unsigned() -> Weight {
		Weight::from_ref_time(35_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(7 as u64))
			.saturating_add(T::DbWeight::get().writes(6 as u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: VortexDistribution NextVortexId (r:1 w:1)
	// Storage: VortexDistribution VtxDistStatuses (r:0 w:1)
	// Storage: VortexDistribution TotalVortex (r:0 w:1)
	fn create_vtx_dist() -> Weight {
		Weight::from_ref_time(10_000_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().writes(3 as u64))
	}
	// Storage: VortexDistribution VtxDistStatuses (r:1 w:1)
	fn disable_vtx_dist() -> Weight {
		Weight::from_ref_time(11_000_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: VortexDistribution VtxDistStatuses (r:1 w:1)
	fn start_vtx_dist() -> Weight {
		Weight::from_ref_time(13_000_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: VortexDistribution VtxDistEras (r:0 w:1)
	fn set_vtx_dist_eras() -> Weight {
		Weight::from_ref_time(8_000_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: VortexDistribution AssetPrices (r:0 w:1)
	/// The range of component `b` is `[1, 1000]`.
	fn set_asset_prices(b: u32, ) -> Weight {
		Weight::from_ref_time(9_000_000 as u64)
			// Standard Error: 1_608
			.saturating_add(Weight::from_ref_time(743_068 as u64).saturating_mul(b as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
			.saturating_add(RocksDbWeight::get().writes((1 as u64).saturating_mul(b as u64)))
	}
	// Storage: VortexDistribution VtxDistStatuses (r:1 w:0)
	// Storage: VortexDistribution TotalVortex (r:1 w:0)
	// Storage: VortexDistribution VtxDistOrderbook (r:1 w:1)
	fn register_rewards() -> Weight {
		Weight::from_ref_time(14_000_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(3 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: VortexDistribution VtxDistStatuses (r:1 w:1)
	// Storage: VortexDistribution TotalVortex (r:1 w:0)
	// Storage: VortexDistribution AssetPrices (r:2 w:0)
	// Storage: Assets Account (r:2 w:2)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: System Account (r:3 w:3)
	fn trigger_vtx_distribution() -> Weight {
		Weight::from_ref_time(47_000_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(10 as u64))
			.saturating_add(RocksDbWeight::get().writes(7 as u64))
	}
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:1)
	// Storage: VortexDistribution AssetPrices (r:2 w:0)
	// Storage: System Account (r:1 w:0)
	fn redeem_tokens_from_vault() -> Weight {
		Weight::from_ref_time(33_000_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(6 as u64))
			.saturating_add(RocksDbWeight::get().writes(2 as u64))
	}
	// Storage: VortexDistribution VtxDistStatuses (r:1 w:0)
	// Storage: VortexDistribution VtxDistPayoutPivot (r:1 w:1)
	// Storage: VortexDistribution VtxDistOrderbook (r:2 w:1)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: VortexDistribution NextUnsignedAt (r:0 w:1)
	fn pay_unsigned() -> Weight {
		Weight::from_ref_time(35_000_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(7 as u64))
			.saturating_add(RocksDbWeight::get().writes(6 as u64))
	}
}
