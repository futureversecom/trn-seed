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

//! Autogenerated weights for pallet_nft_peg
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
// --pallet=pallet_nft_peg
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./output/pallet_nft_peg_weights.rs
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_nft_peg.
pub trait WeightInfo {
	fn set_contract_address() -> Weight;
	fn withdraw() -> Weight;
    fn reclaim_blocked_nfts() -> Weight;
}

/// Weights for pallet_nft_peg using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: NftPeg ContractAddress (r:0 w:1)
	fn set_contract_address() -> Weight {
		Weight::from_ref_time(12_000_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: Nft TokenLocks (r:3 w:0)
	// Storage: NftPeg RootNftToErc721 (r:1 w:0)
	// Storage: NftPeg ContractAddress (r:1 w:0)
	// Storage: EthBridge NextEventProofId (r:1 w:1)
	// Storage: EthBridge NotaryKeys (r:1 w:0)
	// Storage: EthBridge NotarySetId (r:1 w:0)
	// Storage: EthBridge BridgePaused (r:1 w:0)
	// Storage: System Digest (r:1 w:1)
	// Storage: TokenApprovals ERC721Approvals (r:0 w:3)
	fn withdraw() -> Weight {
		Weight::from_ref_time(54_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(11 as u64))
			.saturating_add(T::DbWeight::get().writes(6 as u64))
	}
	// Storage: NftPeg BlockedTokens (r:1 w:1)
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: NftPeg RootNftToErc721 (r:1 w:0)
	// Storage: NftPeg ContractAddress (r:1 w:0)
	// Storage: EthBridge NextEventProofId (r:1 w:1)
	// Storage: EthBridge NotaryKeys (r:1 w:0)
	// Storage: EthBridge NotarySetId (r:1 w:0)
	// Storage: EthBridge BridgePaused (r:1 w:0)
	// Storage: System Digest (r:1 w:1)
	fn reclaim_blocked_nfts() -> Weight {
		(40_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(9 as Weight))
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: NftPeg ContractAddress (r:0 w:1)
	fn set_contract_address() -> Weight {
		Weight::from_ref_time(12_000_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: Nft TokenLocks (r:3 w:0)
	// Storage: NftPeg RootNftToErc721 (r:1 w:0)
	// Storage: NftPeg ContractAddress (r:1 w:0)
	// Storage: EthBridge NextEventProofId (r:1 w:1)
	// Storage: EthBridge NotaryKeys (r:1 w:0)
	// Storage: EthBridge NotarySetId (r:1 w:0)
	// Storage: EthBridge BridgePaused (r:1 w:0)
	// Storage: System Digest (r:1 w:1)
	// Storage: TokenApprovals ERC721Approvals (r:0 w:3)
	fn withdraw() -> Weight {
		Weight::from_ref_time(54_000_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(11 as u64))
			.saturating_add(RocksDbWeight::get().writes(6 as u64))
	}
}

