
//! Autogenerated weights for `pallet_dex`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-09-10, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-102-147`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-dex
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_dex.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_dex`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_dex::WeightInfo for WeightInfo<T> {
	// Storage: Dex TradingPairStatuses (r:1 w:0)
	// Storage: Dex LiquidityPool (r:1 w:1)
	// Storage: Assets Asset (r:2 w:2)
	// Storage: Assets Account (r:4 w:4)
	// Storage: Dex FeeTo (r:1 w:0)
	// Storage: Dex TradingPairLPToken (r:1 w:0)
	// Storage: Dex LiquidityPoolLastK (r:1 w:1)
	fn swap_with_exact_supply() -> Weight {
		Weight::from_ref_time(241_774_000 as u64)
			.saturating_add(T::DbWeight::get().reads(11 as u64))
			.saturating_add(T::DbWeight::get().writes(8 as u64))
	}
	// Storage: Dex TradingPairStatuses (r:1 w:0)
	// Storage: Dex LiquidityPool (r:1 w:1)
	// Storage: Assets Asset (r:3 w:2)
	// Storage: Assets Account (r:4 w:4)
	// Storage: Dex FeeTo (r:1 w:0)
	// Storage: Dex TradingPairLPToken (r:1 w:0)
	// Storage: Dex LiquidityPoolLastK (r:1 w:1)
	fn swap_with_exact_target() -> Weight {
		Weight::from_ref_time(249_416_000 as u64)
			.saturating_add(T::DbWeight::get().reads(12 as u64))
			.saturating_add(T::DbWeight::get().writes(8 as u64))
	}
	// Storage: Dex TradingPairLPToken (r:1 w:1)
	// Storage: Assets Metadata (r:3 w:1)
	// Storage: AssetsExt NextAssetId (r:1 w:1)
	// Storage: Assets Asset (r:3 w:3)
	// Storage: EVM AccountCodes (r:1 w:1)
	// Storage: Futurepass DefaultProxy (r:1 w:0)
	// Storage: System Account (r:4 w:4)
	// Storage: Dex LiquidityPool (r:1 w:1)
	// Storage: Assets Account (r:6 w:6)
	// Storage: Dex FeeTo (r:1 w:0)
	// Storage: Dex LiquidityPoolLastK (r:1 w:1)
	// Storage: Dex TradingPairStatuses (r:0 w:1)
	fn add_liquidity() -> Weight {
		Weight::from_ref_time(409_994_000 as u64)
			.saturating_add(T::DbWeight::get().reads(23 as u64))
			.saturating_add(T::DbWeight::get().writes(20 as u64))
	}
	// Storage: Dex TradingPairLPToken (r:1 w:0)
	// Storage: Dex LiquidityPool (r:1 w:1)
	// Storage: Assets Asset (r:3 w:3)
	// Storage: Assets Account (r:6 w:6)
	// Storage: System Account (r:1 w:1)
	// Storage: Dex FeeTo (r:1 w:0)
	// Storage: Dex LiquidityPoolLastK (r:1 w:1)
	fn remove_liquidity() -> Weight {
		Weight::from_ref_time(333_350_000 as u64)
			.saturating_add(T::DbWeight::get().reads(14 as u64))
			.saturating_add(T::DbWeight::get().writes(12 as u64))
	}
	// Storage: Dex TradingPairLPToken (r:1 w:0)
	// Storage: Dex TradingPairStatuses (r:1 w:1)
	fn reenable_trading_pair() -> Weight {
		Weight::from_ref_time(70_400_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Dex TradingPairLPToken (r:1 w:0)
	// Storage: Dex TradingPairStatuses (r:1 w:1)
	fn disable_trading_pair() -> Weight {
		Weight::from_ref_time(69_817_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Dex FeeTo (r:0 w:1)
	fn set_fee_to() -> Weight {
		Weight::from_ref_time(42_312_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
}
