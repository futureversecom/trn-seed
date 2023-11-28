
//! Autogenerated weights for `pallet_liquidity_pools`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-11-28, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-102-147`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-liquidity-pools
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_liquidity_pools.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_liquidity_pools`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_liquidity_pools::WeightInfo for WeightInfo<T> {
	// Storage: LiquidityPools NextPoolId (r:1 w:1)
	// Storage: Assets Metadata (r:2 w:0)
	// Storage: LiquidityPools Pools (r:0 w:1)
	fn create_pool() -> Weight {
		Weight::from_ref_time(78_338_000 as u64)
			.saturating_add(T::DbWeight::get().reads(3 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: LiquidityPools Pools (r:1 w:1)
	// Storage: LiquidityPools RolloverPivot (r:0 w:1)
	// Storage: LiquidityPools PoolRelationships (r:0 w:1)
	fn close_pool() -> Weight {
		Weight::from_ref_time(59_407_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
	}
	// Storage: LiquidityPools Pools (r:2 w:0)
	// Storage: LiquidityPools PoolRelationships (r:0 w:1)
	fn set_pool_succession() -> Weight {
		Weight::from_ref_time(59_064_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: LiquidityPools Pools (r:1 w:0)
	// Storage: LiquidityPools PoolUsers (r:1 w:1)
	fn set_pool_rollover() -> Weight {
		Weight::from_ref_time(65_572_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: LiquidityPools Pools (r:1 w:1)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:2)
	// Storage: System Account (r:2 w:2)
	// Storage: LiquidityPools PoolUsers (r:1 w:1)
	fn join_pool() -> Weight {
		Weight::from_ref_time(174_471_000 as u64)
			.saturating_add(T::DbWeight::get().reads(7 as u64))
			.saturating_add(T::DbWeight::get().writes(7 as u64))
	}
	// Storage: LiquidityPools Pools (r:1 w:1)
	// Storage: LiquidityPools PoolUsers (r:1 w:1)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:2)
	// Storage: System Account (r:2 w:2)
	fn exit_pool() -> Weight {
		Weight::from_ref_time(176_918_000 as u64)
			.saturating_add(T::DbWeight::get().reads(7 as u64))
			.saturating_add(T::DbWeight::get().writes(7 as u64))
	}
	// Storage: LiquidityPools PoolUsers (r:1 w:1)
	// Storage: LiquidityPools Pools (r:1 w:1)
	// Storage: Assets Metadata (r:2 w:0)
	// Storage: System Account (r:2 w:2)
	fn claim_reward() -> Weight {
		Weight::from_ref_time(158_893_000 as u64)
			.saturating_add(T::DbWeight::get().reads(6 as u64))
			.saturating_add(T::DbWeight::get().writes(4 as u64))
	}
	// Storage: LiquidityPools Pools (r:1 w:0)
	// Storage: LiquidityPools NextRolloverUnsignedAt (r:0 w:1)
	fn rollover_unsigned() -> Weight {
		Weight::from_ref_time(70_881_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
}
