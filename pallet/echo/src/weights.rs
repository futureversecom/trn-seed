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

//! Autogenerated weights for pallet_echo
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
// --pallet=pallet_echo
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./pallet/echo/src/weights.rs
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_echo.
pub trait WeightInfo {
	fn ping() -> Weight;
}

/// Weights for pallet_echo using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: Echo NextSessionId (r:1 w:1)
	// Storage: EthBridge NextEventProofId (r:1 w:1)
	// Storage: EthBridge NotaryKeys (r:1 w:0)
	// Storage: EthBridge NotarySetId (r:1 w:0)
	// Storage: EthBridge BridgePaused (r:1 w:0)
	// Storage: System Digest (r:1 w:1)
	fn ping() -> Weight {
		Weight::from_ref_time(85_832_000 as u64)
			.saturating_add(T::DbWeight::get().reads(6 as u64))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: Echo NextSessionId (r:1 w:1)
	// Storage: EthBridge NextEventProofId (r:1 w:1)
	// Storage: EthBridge NotaryKeys (r:1 w:0)
	// Storage: EthBridge NotarySetId (r:1 w:0)
	// Storage: EthBridge BridgePaused (r:1 w:0)
	// Storage: System Digest (r:1 w:1)
	fn ping() -> Weight {
		Weight::from_ref_time(85_832_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(6 as u64))
			.saturating_add(RocksDbWeight::get().writes(3 as u64))
	}
}

