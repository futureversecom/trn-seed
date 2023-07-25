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

//! Autogenerated weights for pallet_dex
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-07-25, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `Surangas-MacBook-Pro.local`, CPU: `<UNKNOWN>`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_dex
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./output/pallet_dex_weights.rs
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_dex.
pub trait WeightInfo {
	fn swap_with_exact_supply() -> Weight;
	fn swap_with_exact_target() -> Weight;
	fn add_liquidity() -> Weight;
	fn remove_liquidity() -> Weight;
	fn reenable_trading_pair() -> Weight;
	fn disable_trading_pair() -> Weight;
	fn set_fee_to() -> Weight;
}

/// Weights for pallet_dex using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: Dex TradingPairStatuses (r:1 w:0)
	// Storage: Dex LiquidityPool (r:1 w:1)
	// Storage: Assets Asset (r:2 w:2)
	// Storage: Assets Account (r:4 w:4)
	fn swap_with_exact_supply() -> Weight {
		Weight::from_ref_time(77_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(8 as u64))
			.saturating_add(T::DbWeight::get().writes(7 as u64))
	}
	// Storage: Dex TradingPairStatuses (r:1 w:0)
	// Storage: Dex LiquidityPool (r:1 w:1)
	// Storage: Assets Asset (r:2 w:2)
	// Storage: Assets Account (r:4 w:4)
	fn swap_with_exact_target() -> Weight {
		Weight::from_ref_time(77_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(8 as u64))
			.saturating_add(T::DbWeight::get().writes(7 as u64))
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
	// Storage: Dex LiquidityPoolLastK (r:1 w:1)
	// Storage: Dex FeeTo (r:1 w:0)
	// Storage: Dex TradingPairStatuses (r:0 w:1)
	fn add_liquidity() -> Weight {
		Weight::from_ref_time(155_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(23 as u64))
			.saturating_add(T::DbWeight::get().writes(20 as u64))
	}
	// Storage: Dex TradingPairLPToken (r:1 w:0)
	// Storage: Dex LiquidityPool (r:1 w:1)
	// Storage: Assets Asset (r:3 w:3)
	// Storage: Assets Account (r:6 w:6)
	// Storage: System Account (r:1 w:1)
	// Storage: Dex LiquidityPoolLastK (r:1 w:1)
	// Storage: Dex FeeTo (r:1 w:0)
	fn remove_liquidity() -> Weight {
		Weight::from_ref_time(130_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(14 as u64))
			.saturating_add(T::DbWeight::get().writes(12 as u64))
	}
	// Storage: Dex TradingPairLPToken (r:1 w:0)
	// Storage: Dex TradingPairStatuses (r:1 w:1)
	fn reenable_trading_pair() -> Weight {
		Weight::from_ref_time(20_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Dex TradingPairLPToken (r:1 w:0)
	// Storage: Dex TradingPairStatuses (r:1 w:1)
	fn disable_trading_pair() -> Weight {
		Weight::from_ref_time(20_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Dex FeeTo (r:0 w:1)
	fn set_fee_to() -> Weight {
		Weight::from_ref_time(13_000_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: Dex TradingPairStatuses (r:1 w:0)
	// Storage: Dex LiquidityPool (r:1 w:1)
	// Storage: Assets Asset (r:2 w:2)
	// Storage: Assets Account (r:4 w:4)
	fn swap_with_exact_supply() -> Weight {
		Weight::from_ref_time(77_000_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(8 as u64))
			.saturating_add(RocksDbWeight::get().writes(7 as u64))
	}
	// Storage: Dex TradingPairStatuses (r:1 w:0)
	// Storage: Dex LiquidityPool (r:1 w:1)
	// Storage: Assets Asset (r:2 w:2)
	// Storage: Assets Account (r:4 w:4)
	fn swap_with_exact_target() -> Weight {
		Weight::from_ref_time(77_000_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(8 as u64))
			.saturating_add(RocksDbWeight::get().writes(7 as u64))
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
	// Storage: Dex LiquidityPoolLastK (r:1 w:1)
	// Storage: Dex FeeTo (r:1 w:0)
	// Storage: Dex TradingPairStatuses (r:0 w:1)
	fn add_liquidity() -> Weight {
		Weight::from_ref_time(155_000_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(23 as u64))
			.saturating_add(RocksDbWeight::get().writes(20 as u64))
	}
	// Storage: Dex TradingPairLPToken (r:1 w:0)
	// Storage: Dex LiquidityPool (r:1 w:1)
	// Storage: Assets Asset (r:3 w:3)
	// Storage: Assets Account (r:6 w:6)
	// Storage: System Account (r:1 w:1)
	// Storage: Dex LiquidityPoolLastK (r:1 w:1)
	// Storage: Dex FeeTo (r:1 w:0)
	fn remove_liquidity() -> Weight {
		Weight::from_ref_time(130_000_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(14 as u64))
			.saturating_add(RocksDbWeight::get().writes(12 as u64))
	}
	// Storage: Dex TradingPairLPToken (r:1 w:0)
	// Storage: Dex TradingPairStatuses (r:1 w:1)
	fn reenable_trading_pair() -> Weight {
		Weight::from_ref_time(20_000_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(2 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Dex TradingPairLPToken (r:1 w:0)
	// Storage: Dex TradingPairStatuses (r:1 w:1)
	fn disable_trading_pair() -> Weight {
		Weight::from_ref_time(20_000_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(2 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Dex FeeTo (r:0 w:1)
	fn set_fee_to() -> Weight {
		Weight::from_ref_time(13_000_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
}

