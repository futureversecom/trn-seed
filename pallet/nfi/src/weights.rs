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

//! Autogenerated weights for pallet_nfi
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-12-05, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-102-147`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: , WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-nfi
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./pallet/nfi/src/weights.rs
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_nfi.
pub trait WeightInfo {
	fn set_relayer() -> Weight;
	fn set_fee_to() -> Weight;
	fn set_fee_details() -> Weight;
	fn enable_nfi_for_trn_collection() -> Weight;
	fn manual_data_request() -> Weight;
	fn submit_nfi_data() -> Weight;
}

/// Weights for pallet_nfi using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: `Nfi::Relayer` (r:0 w:1)
	// Proof: `Nfi::Relayer` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_relayer() -> Weight {
		Weight::from_all(24_660_000)
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `Nfi::FeeTo` (r:0 w:1)
	// Proof: `Nfi::FeeTo` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_fee_to() -> Weight {
		Weight::from_all(24_267_000)
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `Nfi::MintFee` (r:0 w:1)
	// Proof: `Nfi::MintFee` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	fn set_fee_details() -> Weight {
		Weight::from_all(26_903_000)
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `Nft::CollectionInfo` (r:1 w:0)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `EVMChainId::ChainId` (r:1 w:0)
	// Proof: `EVMChainId::ChainId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `Nfi::NfiEnabled` (r:0 w:1)
	// Proof: `Nfi::NfiEnabled` (`max_values`: None, `max_size`: Some(529), added: 3004, mode: `MaxEncodedLen`)
	fn enable_nfi_for_trn_collection() -> Weight {
		Weight::from_all(49_913_000)
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `EVMChainId::ChainId` (r:1 w:0)
	// Proof: `EVMChainId::ChainId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `Nfi::NfiEnabled` (r:1 w:0)
	// Proof: `Nfi::NfiEnabled` (`max_values`: None, `max_size`: Some(529), added: 3004, mode: `MaxEncodedLen`)
	// Storage: `Nft::CollectionInfo` (r:1 w:0)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `Nfi::MintFee` (r:1 w:0)
	// Proof: `Nfi::MintFee` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	// Storage: `Nfi::NfiData` (r:1 w:0)
	// Proof: `Nfi::NfiData` (`max_values`: None, `max_size`: Some(1166), added: 3641, mode: `MaxEncodedLen`)
	fn manual_data_request() -> Weight {
		Weight::from_all(67_722_000)
			.saturating_add(T::DbWeight::get().reads(5))
	}
	// Storage: `Nfi::Relayer` (r:1 w:0)
	// Proof: `Nfi::Relayer` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	// Storage: `EVMChainId::ChainId` (r:1 w:0)
	// Proof: `EVMChainId::ChainId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `Nfi::NfiEnabled` (r:1 w:0)
	// Proof: `Nfi::NfiEnabled` (`max_values`: None, `max_size`: Some(529), added: 3004, mode: `MaxEncodedLen`)
	// Storage: `Nft::CollectionInfo` (r:1 w:0)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `Nfi::NfiData` (r:0 w:1)
	// Proof: `Nfi::NfiData` (`max_values`: None, `max_size`: Some(1166), added: 3641, mode: `MaxEncodedLen`)
	fn submit_nfi_data() -> Weight {
		Weight::from_all(70_048_000)
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: `Nfi::Relayer` (r:0 w:1)
	// Proof: `Nfi::Relayer` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_relayer() -> Weight {
		Weight::from_all(24_660_000)
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `Nfi::FeeTo` (r:0 w:1)
	// Proof: `Nfi::FeeTo` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_fee_to() -> Weight {
		Weight::from_all(24_267_000)
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `Nfi::MintFee` (r:0 w:1)
	// Proof: `Nfi::MintFee` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	fn set_fee_details() -> Weight {
		Weight::from_all(26_903_000)
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `Nft::CollectionInfo` (r:1 w:0)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `EVMChainId::ChainId` (r:1 w:0)
	// Proof: `EVMChainId::ChainId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `Nfi::NfiEnabled` (r:0 w:1)
	// Proof: `Nfi::NfiEnabled` (`max_values`: None, `max_size`: Some(529), added: 3004, mode: `MaxEncodedLen`)
	fn enable_nfi_for_trn_collection() -> Weight {
		Weight::from_all(49_913_000)
			.saturating_add(RocksDbWeight::get().reads(2))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `EVMChainId::ChainId` (r:1 w:0)
	// Proof: `EVMChainId::ChainId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `Nfi::NfiEnabled` (r:1 w:0)
	// Proof: `Nfi::NfiEnabled` (`max_values`: None, `max_size`: Some(529), added: 3004, mode: `MaxEncodedLen`)
	// Storage: `Nft::CollectionInfo` (r:1 w:0)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `Nfi::MintFee` (r:1 w:0)
	// Proof: `Nfi::MintFee` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	// Storage: `Nfi::NfiData` (r:1 w:0)
	// Proof: `Nfi::NfiData` (`max_values`: None, `max_size`: Some(1166), added: 3641, mode: `MaxEncodedLen`)
	fn manual_data_request() -> Weight {
		Weight::from_all(67_722_000)
			.saturating_add(RocksDbWeight::get().reads(5))
	}
	// Storage: `Nfi::Relayer` (r:1 w:0)
	// Proof: `Nfi::Relayer` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	// Storage: `EVMChainId::ChainId` (r:1 w:0)
	// Proof: `EVMChainId::ChainId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `Nfi::NfiEnabled` (r:1 w:0)
	// Proof: `Nfi::NfiEnabled` (`max_values`: None, `max_size`: Some(529), added: 3004, mode: `MaxEncodedLen`)
	// Storage: `Nft::CollectionInfo` (r:1 w:0)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `Nfi::NfiData` (r:0 w:1)
	// Proof: `Nfi::NfiData` (`max_values`: None, `max_size`: Some(1166), added: 3641, mode: `MaxEncodedLen`)
	fn submit_nfi_data() -> Weight {
		Weight::from_all(70_048_000)
			.saturating_add(RocksDbWeight::get().reads(4))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
}

