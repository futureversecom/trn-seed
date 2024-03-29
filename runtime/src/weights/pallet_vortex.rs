
//! Autogenerated weights for `pallet_vortex`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-11-14, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-102-147`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-vortex
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_vortex.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_vortex`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_vortex::WeightInfo for WeightInfo<T> {
	// Storage: VortexDistribution NextVortexId (r:1 w:1)
	// Storage: VortexDistribution VtxDistStatuses (r:0 w:1)
	fn create_vtx_dist() -> Weight {
		Weight::from_ref_time(53_641_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: VortexDistribution VtxDistStatuses (r:1 w:1)
	fn disable_vtx_dist() -> Weight {
		Weight::from_ref_time(55_311_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: VortexDistribution VtxDistStatuses (r:1 w:1)
	// Storage: VortexDistribution TotalVortex (r:1 w:1)
	fn start_vtx_dist() -> Weight {
		Weight::from_ref_time(77_840_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: VortexDistribution VtxDistEras (r:0 w:1)
	fn set_vtx_dist_eras() -> Weight {
		Weight::from_ref_time(44_888_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: VortexDistribution AssetPrices (r:0 w:1)
	/// The range of component `b` is `[1, 500]`.
	fn set_asset_prices(b: u32, ) -> Weight {
		Weight::from_ref_time(53_257_000 as u64)
			// Standard Error: 4_448
			.saturating_add(Weight::from_ref_time(3_512_910 as u64).saturating_mul(b as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
			.saturating_add(T::DbWeight::get().writes((1 as u64).saturating_mul(b as u64)))
	}
	// Storage: VortexDistribution VtxDistStatuses (r:1 w:0)
	// Storage: VortexDistribution VtxDistOrderbook (r:1 w:1)
	// Storage: VortexDistribution TotalVortex (r:1 w:1)
	fn register_rewards() -> Weight {
		Weight::from_ref_time(60_788_000 as u64)
			.saturating_add(T::DbWeight::get().reads(3 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: VortexDistribution VtxDistStatuses (r:1 w:1)
	// Storage: VortexDistribution AssetPrices (r:2 w:0)
	// Storage: Assets Account (r:2 w:2)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: System Account (r:3 w:3)
	fn trigger_vtx_distribution() -> Weight {
		Weight::from_ref_time(236_875_000 as u64)
			.saturating_add(T::DbWeight::get().reads(9 as u64))
			.saturating_add(T::DbWeight::get().writes(7 as u64))
	}
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:1)
	// Storage: VortexDistribution AssetPrices (r:2 w:0)
	// Storage: System Account (r:1 w:0)
	fn redeem_tokens_from_vault() -> Weight {
		Weight::from_ref_time(153_985_000 as u64)
			.saturating_add(T::DbWeight::get().reads(6 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: VortexDistribution VtxDistStatuses (r:1 w:0)
	// Storage: VortexDistribution VtxDistPayoutPivot (r:1 w:1)
	// Storage: VortexDistribution VtxDistOrderbook (r:2 w:1)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:2)
	// Storage: System Account (r:2 w:2)
	// Storage: VortexDistribution NextUnsignedAt (r:0 w:1)
	fn pay_unsigned() -> Weight {
		Weight::from_ref_time(206_154_000 as u64)
			.saturating_add(T::DbWeight::get().reads(9 as u64))
			.saturating_add(T::DbWeight::get().writes(8 as u64))
	}
}
