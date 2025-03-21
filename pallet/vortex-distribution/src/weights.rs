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

//! Autogenerated weights for pallet_vortex_distribution
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2025-03-21, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `Surangas-MacBook-Pro.local`, CPU: `<UNKNOWN>`
//! EXECUTION: , WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-vortex-distribution
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./pallet/vortex-distribution/src/weights.rs
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_vortex_distribution.
pub trait WeightInfo {
	fn set_admin() -> Weight;
	fn create_vtx_dist() -> Weight;
	fn disable_vtx_dist() -> Weight;
	fn set_vtx_total_supply() -> Weight;
	fn set_consider_current_balance() -> Weight;
	fn set_disable_redeem() -> Weight;
	fn start_vtx_dist() -> Weight;
	fn set_fee_pot_asset_balances(b: u32, ) -> Weight;
	fn set_vtx_vault_asset_balances(b: u32, ) -> Weight;
	fn set_asset_prices(b: u32, ) -> Weight;
	fn register_reward_points(b: u32, ) -> Weight;
	fn register_work_points(b: u32, ) -> Weight;
	fn trigger_vtx_distribution() -> Weight;
	fn redeem_tokens_from_vault() -> Weight;
	fn pay_unsigned() -> Weight;
}

/// Weights for pallet_vortex_distribution using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: `VortexDistribution::AdminAccount` (r:0 w:1)
	// Proof: `VortexDistribution::AdminAccount` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_admin() -> Weight {
		Weight::from_all(7_000_000)
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::NextVortexId` (r:1 w:1)
	// Proof: `VortexDistribution::NextVortexId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxDistStatuses` (r:0 w:1)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	fn create_vtx_dist() -> Weight {
		Weight::from_all(12_000_000)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:1)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	fn disable_vtx_dist() -> Weight {
		Weight::from_all(13_000_000)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::VtxTotalSupply` (r:0 w:1)
	// Proof: `VortexDistribution::VtxTotalSupply` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	fn set_vtx_total_supply() -> Weight {
		Weight::from_all(7_000_000)
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::ConsiderCurrentBalance` (r:0 w:1)
	// Proof: `VortexDistribution::ConsiderCurrentBalance` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn set_consider_current_balance() -> Weight {
		Weight::from_all(7_000_000)
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::DisableRedeem` (r:0 w:1)
	// Proof: `VortexDistribution::DisableRedeem` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn set_disable_redeem() -> Weight {
		Weight::from_all(7_000_000)
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:1)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalVortex` (r:1 w:0)
	// Proof: `VortexDistribution::TotalVortex` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:1 w:1)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:1 w:1)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	fn start_vtx_dist() -> Weight {
		Weight::from_all(47_000_000)
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	// Storage: `VortexDistribution::FeePotAssetsList` (r:0 w:1)
	// Proof: `VortexDistribution::FeePotAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn set_fee_pot_asset_balances(b: u32, ) -> Weight {
		Weight::from_all(9_958_493)
			// Standard Error: 1_167
			.saturating_add(Weight::from_all(58_105_u64).saturating_mul(b as u64))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::VtxVaultAssetsList` (r:0 w:1)
	// Proof: `VortexDistribution::VtxVaultAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn set_vtx_vault_asset_balances(b: u32, ) -> Weight {
		Weight::from_all(12_482_493)
			// Standard Error: 1_021
			.saturating_add(Weight::from_all(51_265_u64).saturating_mul(b as u64))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::FeePotAssetsList` (r:1 w:0)
	// Proof: `VortexDistribution::FeePotAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::AssetPrices` (r:0 w:500)
	// Proof: `VortexDistribution::AssetPrices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn set_asset_prices(b: u32, ) -> Weight {
		Weight::from_all(16_000_000)
			// Standard Error: 25_774
			.saturating_add(Weight::from_all(5_289_322_u64).saturating_mul(b as u64))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(b as u64)))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:0)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalRewardPoints` (r:1 w:1)
	// Proof: `VortexDistribution::TotalRewardPoints` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::RewardPoints` (r:500 w:500)
	// Proof: `VortexDistribution::RewardPoints` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn register_reward_points(b: u32, ) -> Weight {
		Weight::from_all(17_000_000)
			// Standard Error: 4_537
			.saturating_add(Weight::from_all(4_032_757_u64).saturating_mul(b as u64))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(b as u64)))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(b as u64)))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:0)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalWorkPoints` (r:1 w:1)
	// Proof: `VortexDistribution::TotalWorkPoints` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::WorkPoints` (r:500 w:500)
	// Proof: `VortexDistribution::WorkPoints` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn register_work_points(b: u32, ) -> Weight {
		Weight::from_all(17_000_000)
			// Standard Error: 5_743
			.saturating_add(Weight::from_all(4_044_534_u64).saturating_mul(b as u64))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(b as u64)))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(b as u64)))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:1)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxVaultAssetsList` (r:1 w:0)
	// Proof: `VortexDistribution::VtxVaultAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::AssetPrices` (r:2 w:0)
	// Proof: `VortexDistribution::AssetPrices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::ConsiderCurrentBalance` (r:1 w:0)
	// Proof: `VortexDistribution::ConsiderCurrentBalance` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxTotalSupply` (r:1 w:0)
	// Proof: `VortexDistribution::VtxTotalSupply` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::FeePotAssetsList` (r:1 w:0)
	// Proof: `VortexDistribution::FeePotAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:2 w:2)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:3 w:3)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalRewardPoints` (r:1 w:0)
	// Proof: `VortexDistribution::TotalRewardPoints` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalWorkPoints` (r:1 w:0)
	// Proof: `VortexDistribution::TotalWorkPoints` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::RewardPoints` (r:2 w:0)
	// Proof: `VortexDistribution::RewardPoints` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::WorkPoints` (r:1 w:0)
	// Proof: `VortexDistribution::WorkPoints` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxDistOrderbook` (r:1 w:1)
	// Proof: `VortexDistribution::VtxDistOrderbook` (`max_values`: Some(4294967295), `max_size`: Some(73), added: 4033, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalVortex` (r:0 w:1)
	// Proof: `VortexDistribution::TotalVortex` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalNetworkReward` (r:0 w:1)
	// Proof: `VortexDistribution::TotalNetworkReward` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalBootstrapReward` (r:0 w:1)
	// Proof: `VortexDistribution::TotalBootstrapReward` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxPrice` (r:0 w:1)
	// Proof: `VortexDistribution::VtxPrice` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	fn trigger_vtx_distribution() -> Weight {
		Weight::from_all(202_000_000)
			.saturating_add(T::DbWeight::get().reads(19))
			.saturating_add(T::DbWeight::get().writes(12))
	}
	// Storage: `VortexDistribution::DisableRedeem` (r:1 w:0)
	// Proof: `VortexDistribution::DisableRedeem` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:1 w:1)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:1 w:1)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	fn redeem_tokens_from_vault() -> Weight {
		Weight::from_all(52_000_000)
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:0)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxDistPayoutPivot` (r:1 w:1)
	// Proof: `VortexDistribution::VtxDistPayoutPivot` (`max_values`: None, `max_size`: Some(1014), added: 3489, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxDistOrderbook` (r:2 w:1)
	// Proof: `VortexDistribution::VtxDistOrderbook` (`max_values`: Some(4294967295), `max_size`: Some(73), added: 4033, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:2 w:2)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:2 w:2)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::NextUnsignedAt` (r:0 w:1)
	// Proof: `VortexDistribution::NextUnsignedAt` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn pay_unsigned() -> Weight {
		Weight::from_all(85_000_000)
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(8))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: `VortexDistribution::AdminAccount` (r:0 w:1)
	// Proof: `VortexDistribution::AdminAccount` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_admin() -> Weight {
		Weight::from_all(7_000_000)
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::NextVortexId` (r:1 w:1)
	// Proof: `VortexDistribution::NextVortexId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxDistStatuses` (r:0 w:1)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	fn create_vtx_dist() -> Weight {
		Weight::from_all(12_000_000)
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(2))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:1)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	fn disable_vtx_dist() -> Weight {
		Weight::from_all(13_000_000)
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::VtxTotalSupply` (r:0 w:1)
	// Proof: `VortexDistribution::VtxTotalSupply` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	fn set_vtx_total_supply() -> Weight {
		Weight::from_all(7_000_000)
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::ConsiderCurrentBalance` (r:0 w:1)
	// Proof: `VortexDistribution::ConsiderCurrentBalance` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn set_consider_current_balance() -> Weight {
		Weight::from_all(7_000_000)
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::DisableRedeem` (r:0 w:1)
	// Proof: `VortexDistribution::DisableRedeem` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn set_disable_redeem() -> Weight {
		Weight::from_all(7_000_000)
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:1)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalVortex` (r:1 w:0)
	// Proof: `VortexDistribution::TotalVortex` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:1 w:1)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:1 w:1)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	fn start_vtx_dist() -> Weight {
		Weight::from_all(47_000_000)
			.saturating_add(RocksDbWeight::get().reads(5))
			.saturating_add(RocksDbWeight::get().writes(4))
	}
	// Storage: `VortexDistribution::FeePotAssetsList` (r:0 w:1)
	// Proof: `VortexDistribution::FeePotAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn set_fee_pot_asset_balances(b: u32, ) -> Weight {
		Weight::from_all(9_958_493)
			// Standard Error: 1_167
			.saturating_add(Weight::from_all(58_105_u64).saturating_mul(b as u64))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::VtxVaultAssetsList` (r:0 w:1)
	// Proof: `VortexDistribution::VtxVaultAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn set_vtx_vault_asset_balances(b: u32, ) -> Weight {
		Weight::from_all(12_482_493)
			// Standard Error: 1_021
			.saturating_add(Weight::from_all(51_265_u64).saturating_mul(b as u64))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::FeePotAssetsList` (r:1 w:0)
	// Proof: `VortexDistribution::FeePotAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::AssetPrices` (r:0 w:500)
	// Proof: `VortexDistribution::AssetPrices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn set_asset_prices(b: u32, ) -> Weight {
		Weight::from_all(16_000_000)
			// Standard Error: 25_774
			.saturating_add(Weight::from_all(5_289_322_u64).saturating_mul(b as u64))
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes((1_u64).saturating_mul(b as u64)))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:0)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalRewardPoints` (r:1 w:1)
	// Proof: `VortexDistribution::TotalRewardPoints` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::RewardPoints` (r:500 w:500)
	// Proof: `VortexDistribution::RewardPoints` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn register_reward_points(b: u32, ) -> Weight {
		Weight::from_all(17_000_000)
			// Standard Error: 4_537
			.saturating_add(Weight::from_all(4_032_757_u64).saturating_mul(b as u64))
			.saturating_add(RocksDbWeight::get().reads(2))
			.saturating_add(RocksDbWeight::get().reads((1_u64).saturating_mul(b as u64)))
			.saturating_add(RocksDbWeight::get().writes(1))
			.saturating_add(RocksDbWeight::get().writes((1_u64).saturating_mul(b as u64)))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:0)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalWorkPoints` (r:1 w:1)
	// Proof: `VortexDistribution::TotalWorkPoints` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::WorkPoints` (r:500 w:500)
	// Proof: `VortexDistribution::WorkPoints` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn register_work_points(b: u32, ) -> Weight {
		Weight::from_all(17_000_000)
			// Standard Error: 5_743
			.saturating_add(Weight::from_all(4_044_534_u64).saturating_mul(b as u64))
			.saturating_add(RocksDbWeight::get().reads(2))
			.saturating_add(RocksDbWeight::get().reads((1_u64).saturating_mul(b as u64)))
			.saturating_add(RocksDbWeight::get().writes(1))
			.saturating_add(RocksDbWeight::get().writes((1_u64).saturating_mul(b as u64)))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:1)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxVaultAssetsList` (r:1 w:0)
	// Proof: `VortexDistribution::VtxVaultAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::AssetPrices` (r:2 w:0)
	// Proof: `VortexDistribution::AssetPrices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::ConsiderCurrentBalance` (r:1 w:0)
	// Proof: `VortexDistribution::ConsiderCurrentBalance` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxTotalSupply` (r:1 w:0)
	// Proof: `VortexDistribution::VtxTotalSupply` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::FeePotAssetsList` (r:1 w:0)
	// Proof: `VortexDistribution::FeePotAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:2 w:2)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:3 w:3)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalRewardPoints` (r:1 w:0)
	// Proof: `VortexDistribution::TotalRewardPoints` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalWorkPoints` (r:1 w:0)
	// Proof: `VortexDistribution::TotalWorkPoints` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::RewardPoints` (r:2 w:0)
	// Proof: `VortexDistribution::RewardPoints` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::WorkPoints` (r:1 w:0)
	// Proof: `VortexDistribution::WorkPoints` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxDistOrderbook` (r:1 w:1)
	// Proof: `VortexDistribution::VtxDistOrderbook` (`max_values`: Some(4294967295), `max_size`: Some(73), added: 4033, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalVortex` (r:0 w:1)
	// Proof: `VortexDistribution::TotalVortex` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalNetworkReward` (r:0 w:1)
	// Proof: `VortexDistribution::TotalNetworkReward` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalBootstrapReward` (r:0 w:1)
	// Proof: `VortexDistribution::TotalBootstrapReward` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxPrice` (r:0 w:1)
	// Proof: `VortexDistribution::VtxPrice` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	fn trigger_vtx_distribution() -> Weight {
		Weight::from_all(202_000_000)
			.saturating_add(RocksDbWeight::get().reads(19))
			.saturating_add(RocksDbWeight::get().writes(12))
	}
	// Storage: `VortexDistribution::DisableRedeem` (r:1 w:0)
	// Proof: `VortexDistribution::DisableRedeem` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:1 w:1)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:1 w:1)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	fn redeem_tokens_from_vault() -> Weight {
		Weight::from_all(52_000_000)
			.saturating_add(RocksDbWeight::get().reads(4))
			.saturating_add(RocksDbWeight::get().writes(3))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:0)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxDistPayoutPivot` (r:1 w:1)
	// Proof: `VortexDistribution::VtxDistPayoutPivot` (`max_values`: None, `max_size`: Some(1014), added: 3489, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxDistOrderbook` (r:2 w:1)
	// Proof: `VortexDistribution::VtxDistOrderbook` (`max_values`: Some(4294967295), `max_size`: Some(73), added: 4033, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:2 w:2)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:2 w:2)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::NextUnsignedAt` (r:0 w:1)
	// Proof: `VortexDistribution::NextUnsignedAt` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn pay_unsigned() -> Weight {
		Weight::from_all(85_000_000)
			.saturating_add(RocksDbWeight::get().reads(9))
			.saturating_add(RocksDbWeight::get().writes(8))
	}
}

