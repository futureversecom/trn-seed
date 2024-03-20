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

//! Autogenerated weights for pallet_xls20
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-09-19, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-102-147`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_xls20
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./pallet/xls20/src/weights.rs
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_xls20.
pub trait WeightInfo {
	fn set_relayer() -> Weight;
	fn set_xls20_fee() -> Weight;
	fn enable_xls20_compatibility() -> Weight;
	fn re_request_xls20_mint() -> Weight;
	fn fulfill_xls20_mint() -> Weight;
}

/// Weights for pallet_xls20 using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: Xls20 Relayer (r:0 w:1)
	fn set_relayer() -> Weight {
		Weight::from_all(36_424_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Xls20 Xls20MintFee (r:0 w:1)
	fn set_xls20_fee() -> Weight {
		Weight::from_all(36_004_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn enable_xls20_compatibility() -> Weight {
		Weight::from_all(54_993_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: Xls20 Xls20TokenMap (r:1 w:0)
	// Storage: Xls20 Xls20MintFee (r:1 w:0)
	fn re_request_xls20_mint() -> Weight {
		Weight::from_all(70_711_000 as u64)
			.saturating_add(T::DbWeight::get().reads(3 as u64))
	}
	// Storage: Xls20 Relayer (r:1 w:0)
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: Xls20 Xls20TokenMap (r:1 w:1)
	fn fulfill_xls20_mint() -> Weight {
		Weight::from_all(73_952_000 as u64)
			.saturating_add(T::DbWeight::get().reads(3 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: Xls20 Relayer (r:0 w:1)
	fn set_relayer() -> Weight {
		Weight::from_all(36_424_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Xls20 Xls20MintFee (r:0 w:1)
	fn set_xls20_fee() -> Weight {
		Weight::from_all(36_004_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn enable_xls20_compatibility() -> Weight {
		Weight::from_all(54_993_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: Xls20 Xls20TokenMap (r:1 w:0)
	// Storage: Xls20 Xls20MintFee (r:1 w:0)
	fn re_request_xls20_mint() -> Weight {
		Weight::from_all(70_711_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(3 as u64))
	}
	// Storage: Xls20 Relayer (r:1 w:0)
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: Xls20 Xls20TokenMap (r:1 w:1)
	fn fulfill_xls20_mint() -> Weight {
		Weight::from_all(73_952_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(3 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
}

