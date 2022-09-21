// This file is part of Substrate.

// Copyright (C) 2021 Parity Technologies (UK) Ltd.
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

//! Autogenerated weights for pallet_liquid_staking
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-03-09, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("vanilla-dev"), DB CACHE: 1024

// Executed Command:
// target/release/seed
// benchmark
// --chain=dev
// --execution=wasm
// --wasm-execution=compiled
// --pallet=pallet_xrpl_bridge
// --extrinsic=*
// --steps=50
// --repeat=20
// --heap-pages=4096
// --template=./.maintain/frame-weight-template.hbs
// --output=./pallets/xrpl_bridge/src/weights.rs

#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::all)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_liquid_staking.
pub trait WeightInfo {
	fn submit_transaction() -> Weight;
	fn submit_challenge() -> Weight;
	fn withdraw_xrp() -> Weight;
	fn add_relayer() -> Weight;
	fn remove_relayer() -> Weight;
	fn set_xrpl_door_address() -> Weight;
}

/// Weights for pallet_liquid_staking using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn submit_transaction() -> Weight {
		(190_935_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(15 as Weight))
			.saturating_add(T::DbWeight::get().writes(10 as Weight))
	}

	fn submit_challenge() -> Weight {
		(190_935_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(15 as Weight))
			.saturating_add(T::DbWeight::get().writes(10 as Weight))
	}

	fn withdraw_xrp() -> Weight {
		(190_935_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(15 as Weight))
			.saturating_add(T::DbWeight::get().writes(10 as Weight))
	}

	fn add_relayer() -> Weight {
		(190_935_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(15 as Weight))
			.saturating_add(T::DbWeight::get().writes(10 as Weight))
	}

	fn remove_relayer() -> Weight {
		(190_935_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(15 as Weight))
			.saturating_add(T::DbWeight::get().writes(10 as Weight))
	}

	fn set_xrpl_door_address() -> Weight { // spc
		(190_935_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(15 as Weight))
			.saturating_add(T::DbWeight::get().writes(10 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn submit_transaction() -> Weight {
		(190_935_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(15 as Weight))
			.saturating_add(RocksDbWeight::get().writes(10 as Weight))
	}

	fn submit_challenge() -> Weight {
		(190_935_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(15 as Weight))
			.saturating_add(RocksDbWeight::get().writes(10 as Weight))
	}

	fn withdraw_xrp() -> Weight {
		(190_935_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(15 as Weight))
			.saturating_add(RocksDbWeight::get().writes(10 as Weight))
	}

	fn add_relayer() -> Weight {
		(190_935_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(15 as Weight))
			.saturating_add(RocksDbWeight::get().writes(10 as Weight))
	}

	fn remove_relayer() -> Weight {
		(190_935_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(15 as Weight))
			.saturating_add(RocksDbWeight::get().writes(10 as Weight))
	}

	fn set_xrpl_door_address() -> Weight { // spc
		(190_935_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(15 as Weight))
			.saturating_add(RocksDbWeight::get().writes(10 as Weight))
	}
}
