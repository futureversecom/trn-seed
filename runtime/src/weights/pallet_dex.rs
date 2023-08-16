
//! Autogenerated weights for `pallet_dex`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-07-19, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `Hans-MacBook-Pro.local`, CPU: `<UNKNOWN>`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain
// dev
// --steps=50
// --repeat=20
// --pallet=pallet_dex
// --extrinsic
// *
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// runtime/src/weights/pallet_dex.rs

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
	fn swap_with_exact_supply() -> Weight {
		(72_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(8 as Weight))
			.saturating_add(T::DbWeight::get().writes(7 as Weight))
	}
	// Storage: Dex TradingPairStatuses (r:1 w:0)
	// Storage: Dex LiquidityPool (r:1 w:1)
	// Storage: Assets Asset (r:2 w:2)
	// Storage: Assets Account (r:4 w:4)
	fn swap_with_exact_target() -> Weight {
		(71_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(8 as Weight))
			.saturating_add(T::DbWeight::get().writes(7 as Weight))
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
	// Storage: Dex LiquidityPoolLastK (r:1 w:0)
	// Storage: Dex FeeTo (r:1 w:0)
	// Storage: Dex TradingPairStatuses (r:0 w:1)
	fn add_liquidity() -> Weight {
		(146_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(23 as Weight))
			.saturating_add(T::DbWeight::get().writes(19 as Weight))
	}
	// Storage: Dex TradingPairLPToken (r:1 w:0)
	// Storage: Dex LiquidityPool (r:1 w:1)
	// Storage: Assets Asset (r:3 w:3)
	// Storage: Assets Account (r:6 w:6)
	// Storage: System Account (r:1 w:1)
	// Storage: Dex LiquidityPoolLastK (r:1 w:0)
	// Storage: Dex FeeTo (r:1 w:0)
	fn remove_liquidity() -> Weight {
		(118_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(14 as Weight))
			.saturating_add(T::DbWeight::get().writes(11 as Weight))
	}
	// Storage: Dex TradingPairLPToken (r:1 w:0)
	// Storage: Dex TradingPairStatuses (r:1 w:1)
	fn reenable_trading_pair() -> Weight {
		(19_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Dex TradingPairLPToken (r:1 w:0)
	// Storage: Dex TradingPairStatuses (r:1 w:1)
	fn disable_trading_pair() -> Weight {
		(19_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Dex FeeTo (r:0 w:1)
	fn set_fee_to() -> Weight {
		(12_000_000 as Weight)
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
}
