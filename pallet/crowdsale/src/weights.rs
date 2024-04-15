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

//! Autogenerated weights for pallet_crowdsale
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-03-15, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-102-147`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-crowdsale
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./pallet/crowdsale/src/weights.rs
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_crowdsale.
pub trait WeightInfo {
	fn initialize() -> Weight;
	fn enable() -> Weight;
	fn participate() -> Weight;
	fn distribute_crowdsale_rewards() -> Weight;
	fn claim_voucher() -> Weight;
	fn redeem_voucher() -> Weight;
	fn proxy_vault_call() -> Weight;
	fn try_force_distribution() -> Weight;
	fn on_initialize(p: u32, ) -> Weight;
	fn on_initialize_empty() -> Weight;
}

/// Weights for pallet_crowdsale using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: Crowdsale NextSaleId (r:1 w:1)
	// Storage: Assets Asset (r:2 w:1)
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: Nft PublicMintInfo (r:1 w:0)
	// Storage: AssetsExt NextAssetId (r:1 w:1)
	// Storage: EVM AccountCodes (r:1 w:1)
	// Storage: Futurepass DefaultProxy (r:1 w:0)
	// Storage: System Account (r:2 w:2)
	// Storage: Assets Metadata (r:1 w:1)
	// Storage: Assets Account (r:1 w:1)
	// Storage: Crowdsale SaleInfo (r:0 w:1)
	fn initialize() -> Weight {
		Weight::from_all(252_840_000 as u64)
			.saturating_add(T::DbWeight::get().reads(12 as u64))
			.saturating_add(T::DbWeight::get().writes(10 as u64))
	}
	// Storage: Crowdsale SaleInfo (r:1 w:1)
	// Storage: Crowdsale SaleEndBlocks (r:1 w:1)
	fn enable() -> Weight {
		Weight::from_all(71_553_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: Crowdsale SaleInfo (r:1 w:1)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:2)
	// Storage: System Account (r:2 w:2)
	// Storage: Crowdsale SaleParticipation (r:1 w:1)
	fn participate() -> Weight {
		Weight::from_all(164_780_000 as u64)
			.saturating_add(T::DbWeight::get().reads(7 as u64))
			.saturating_add(T::DbWeight::get().writes(7 as u64))
	}
	// Storage: Crowdsale SaleDistribution (r:1 w:1)
	// Storage: Crowdsale SaleInfo (r:1 w:1)
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: Crowdsale SaleParticipation (r:2 w:1)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:2)
	// Storage: System Account (r:2 w:2)
	// Storage: Crowdsale NextUnsignedAt (r:0 w:1)
	fn distribute_crowdsale_rewards() -> Weight {
		Weight::from_all(254_171_000 as u64)
			.saturating_add(T::DbWeight::get().reads(10 as u64))
			.saturating_add(T::DbWeight::get().writes(9 as u64))
	}
	// Storage: Crowdsale SaleInfo (r:1 w:1)
	// Storage: Crowdsale SaleParticipation (r:2 w:1)
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:2)
	// Storage: System Account (r:2 w:2)
	// Storage: Crowdsale SaleDistribution (r:1 w:1)
	fn claim_voucher() -> Weight {
		Weight::from_all(245_727_000 as u64)
			.saturating_add(T::DbWeight::get().reads(10 as u64))
			.saturating_add(T::DbWeight::get().writes(8 as u64))
	}
	// Storage: Crowdsale SaleInfo (r:1 w:0)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: Nft PublicMintInfo (r:1 w:0)
	fn redeem_voucher() -> Weight {
		Weight::from_all(181_395_000 as u64)
			.saturating_add(T::DbWeight::get().reads(6 as u64))
			.saturating_add(T::DbWeight::get().writes(4 as u64))
	}
	// Storage: Crowdsale SaleInfo (r:1 w:0)
	// Storage: MaintenanceMode BlockedCalls (r:1 w:0)
	// Storage: MaintenanceMode BlockedPallets (r:1 w:0)
	fn proxy_vault_call() -> Weight {
		Weight::from_ref_time(89_316_000 as u64)
			.saturating_add(T::DbWeight::get().reads(3 as u64))
	}
	// Storage: Crowdsale SaleInfo (r:1 w:1)
	// Storage: Crowdsale SaleDistribution (r:1 w:1)
	fn try_force_distribution() -> Weight {
		Weight::from_all(66_821_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: Crowdsale SaleEndBlocks (r:1 w:1)
	// Storage: Crowdsale SaleInfo (r:1 w:1)
	// Storage: Assets Asset (r:2 w:2)
	// Storage: Assets Account (r:4 w:4)
	// Storage: System Account (r:2 w:2)
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: Crowdsale SaleDistribution (r:1 w:1)
	/// The range of component `p` is `[1, 5]`.
	fn on_initialize(p: u32, ) -> Weight {
		Weight::from_all(219_624_000 as u64)
			// Standard Error: 1_029_078
			.saturating_add(Weight::from_all(105_022_670 as u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads(12 as u64))
			.saturating_add(T::DbWeight::get().reads((6 as u64).saturating_mul(p as u64)))
			.saturating_add(T::DbWeight::get().writes(11 as u64))
			.saturating_add(T::DbWeight::get().writes((5 as u64).saturating_mul(p as u64)))
	}
	// Storage: Crowdsale SaleEndBlocks (r:1 w:0)
	fn on_initialize_empty() -> Weight {
		Weight::from_all(10_420_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: Crowdsale NextSaleId (r:1 w:1)
	// Storage: Assets Asset (r:2 w:1)
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: Nft PublicMintInfo (r:1 w:0)
	// Storage: AssetsExt NextAssetId (r:1 w:1)
	// Storage: EVM AccountCodes (r:1 w:1)
	// Storage: Futurepass DefaultProxy (r:1 w:0)
	// Storage: System Account (r:2 w:2)
	// Storage: Assets Metadata (r:1 w:1)
	// Storage: Assets Account (r:1 w:1)
	// Storage: Crowdsale SaleInfo (r:0 w:1)
	fn initialize() -> Weight {
		Weight::from_all(252_840_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(12 as u64))
			.saturating_add(RocksDbWeight::get().writes(10 as u64))
	}
	// Storage: Crowdsale SaleInfo (r:1 w:1)
	// Storage: Crowdsale SaleEndBlocks (r:1 w:1)
	fn enable() -> Weight {
		Weight::from_all(71_553_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(2 as u64))
			.saturating_add(RocksDbWeight::get().writes(2 as u64))
	}
	// Storage: Crowdsale SaleInfo (r:1 w:1)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:2)
	// Storage: System Account (r:2 w:2)
	// Storage: Crowdsale SaleParticipation (r:1 w:1)
	fn participate() -> Weight {
		Weight::from_all(164_780_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(7 as u64))
			.saturating_add(RocksDbWeight::get().writes(7 as u64))
	}
	// Storage: Crowdsale SaleDistribution (r:1 w:1)
	// Storage: Crowdsale SaleInfo (r:1 w:1)
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: Crowdsale SaleParticipation (r:2 w:1)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:2)
	// Storage: System Account (r:2 w:2)
	// Storage: Crowdsale NextUnsignedAt (r:0 w:1)
	fn distribute_crowdsale_rewards() -> Weight {
		Weight::from_all(254_171_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(10 as u64))
			.saturating_add(RocksDbWeight::get().writes(9 as u64))
	}
	// Storage: Crowdsale SaleInfo (r:1 w:1)
	// Storage: Crowdsale SaleParticipation (r:2 w:1)
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:2)
	// Storage: System Account (r:2 w:2)
	// Storage: Crowdsale SaleDistribution (r:1 w:1)
	fn claim_voucher() -> Weight {
		Weight::from_all(245_727_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(10 as u64))
			.saturating_add(RocksDbWeight::get().writes(8 as u64))
	}
	// Storage: Crowdsale SaleInfo (r:1 w:0)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: Nft PublicMintInfo (r:1 w:0)
	fn redeem_voucher() -> Weight {
		Weight::from_all(181_395_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(6 as u64))
			.saturating_add(RocksDbWeight::get().writes(4 as u64))
	}
	// Storage: Crowdsale SaleInfo (r:1 w:0)
	// Storage: MaintenanceMode BlockedCalls (r:1 w:0)
	// Storage: MaintenanceMode BlockedPallets (r:1 w:0)
	fn proxy_vault_call() -> Weight {
		Weight::from_ref_time(89_316_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(3 as u64))
	}
	// Storage: Crowdsale SaleInfo (r:1 w:1)
	// Storage: Crowdsale SaleDistribution (r:1 w:1)
	fn try_force_distribution() -> Weight {
		Weight::from_all(66_821_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(2 as u64))
			.saturating_add(RocksDbWeight::get().writes(2 as u64))
	}
	// Storage: Crowdsale SaleEndBlocks (r:1 w:1)
	// Storage: Crowdsale SaleInfo (r:1 w:1)
	// Storage: Assets Asset (r:2 w:2)
	// Storage: Assets Account (r:4 w:4)
	// Storage: System Account (r:2 w:2)
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: Crowdsale SaleDistribution (r:1 w:1)
	/// The range of component `p` is `[1, 5]`.
	fn on_initialize(p: u32, ) -> Weight {
		Weight::from_all(219_624_000 as u64)
			// Standard Error: 1_029_078
			.saturating_add(Weight::from_all(105_022_670 as u64).saturating_mul(p as u64))
			.saturating_add(RocksDbWeight::get().reads(12 as u64))
			.saturating_add(RocksDbWeight::get().reads((6 as u64).saturating_mul(p as u64)))
			.saturating_add(RocksDbWeight::get().writes(11 as u64))
			.saturating_add(RocksDbWeight::get().writes((5 as u64).saturating_mul(p as u64)))
	}
	// Storage: Crowdsale SaleEndBlocks (r:1 w:0)
	fn on_initialize_empty() -> Weight {
		Weight::from_all(10_420_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
	}
}

