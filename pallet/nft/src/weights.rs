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

//! Autogenerated weights for pallet_nft
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-08-18, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-101-56`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_nft
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./output/pallet_nft_weights.rs
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_nft.
pub trait WeightInfo {
	fn claim_unowned_collection() -> Weight;
	fn set_owner() -> Weight;
	fn set_max_issuance() -> Weight;
   fn set_base_uri() -> Weight;
   fn set_name() -> Weight;
	fn create_collection() -> Weight;
	fn mint() -> Weight;
	fn transfer() -> Weight;
	fn burn() -> Weight;
   fn toggle_eth_compatibility() -> Weight;
}

/// Weights for pallet_nft using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn claim_unowned_collection() -> Weight {
		Weight::from_ref_time(65_272_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn set_owner() -> Weight {
		Weight::from_ref_time(66_270_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn set_max_issuance() -> Weight {
		Weight::from_ref_time(67_101_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn set_base_uri() -> Weight {
		Weight::from_ref_time(68_393_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn set_name() -> Weight {
		Weight::from_ref_time(68_177_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Nft NextCollectionId (r:1 w:1)
	// Storage: EVM AccountCodes (r:1 w:1)
	// Storage: Futurepass DefaultProxy (r:1 w:0)
	// Storage: System Account (r:1 w:1)
	// Storage: Nft CollectionInfo (r:0 w:1)
	fn create_collection() -> Weight {
		Weight::from_ref_time(103_138_000 as u64)
			.saturating_add(T::DbWeight::get().reads(4 as u64))
			.saturating_add(T::DbWeight::get().writes(4 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn mint() -> Weight {
		Weight::from_ref_time(75_380_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: Nft TokenLocks (r:1 w:0)
	// Storage: TokenApprovals ERC721Approvals (r:0 w:1)
	fn transfer() -> Weight {
		Weight::from_ref_time(79_983_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: Nft TokenLocks (r:1 w:0)
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: TokenApprovals ERC721Approvals (r:0 w:1)
	fn burn() -> Weight {
		Weight::from_ref_time(77_279_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn toggle_eth_compatibility() -> Weight {
		Weight::from_ref_time(59_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn claim_unowned_collection() -> Weight {
		Weight::from_ref_time(65_272_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn set_owner() -> Weight {
		Weight::from_ref_time(66_270_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn set_max_issuance() -> Weight {
		Weight::from_ref_time(67_101_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn set_base_uri() -> Weight {
		Weight::from_ref_time(68_393_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn set_name() -> Weight {
		Weight::from_ref_time(68_177_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Nft NextCollectionId (r:1 w:1)
	// Storage: EVM AccountCodes (r:1 w:1)
	// Storage: Futurepass DefaultProxy (r:1 w:0)
	// Storage: System Account (r:1 w:1)
	// Storage: Nft CollectionInfo (r:0 w:1)
	fn create_collection() -> Weight {
		Weight::from_ref_time(103_138_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(4 as u64))
			.saturating_add(RocksDbWeight::get().writes(4 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn mint() -> Weight {
		Weight::from_ref_time(75_380_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: Nft TokenLocks (r:1 w:0)
	// Storage: TokenApprovals ERC721Approvals (r:0 w:1)
	fn transfer() -> Weight {
		Weight::from_ref_time(79_983_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(2 as u64))
			.saturating_add(RocksDbWeight::get().writes(2 as u64))
	}
	// Storage: Nft TokenLocks (r:1 w:0)
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: TokenApprovals ERC721Approvals (r:0 w:1)
	fn burn() -> Weight {
		Weight::from_ref_time(77_279_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(2 as u64))
			.saturating_add(RocksDbWeight::get().writes(2 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn toggle_eth_compatibility() -> Weight {
		Weight::from_ref_time(59_000_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
}

