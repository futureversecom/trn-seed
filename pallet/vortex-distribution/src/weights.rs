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
//! DATE: 2025-07-01, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-102-147`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
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
	fn start_vtx_dist(p: u32, ) -> Weight;
	fn set_fee_pot_asset_balances(b: u32, ) -> Weight;
	fn set_vtx_vault_asset_balances(b: u32, ) -> Weight;
	fn set_asset_prices(b: u32, ) -> Weight;
	fn register_reward_points(b: u32, ) -> Weight;
	fn register_work_points(b: u32, ) -> Weight;
	fn trigger_vtx_distribution(b: u32, p: u32, ) -> Weight;
	fn redeem_tokens_from_vault() -> Weight;
	fn pay_unsigned() -> Weight;
	fn set_vtx_vault_redeem_asset_list(b: u32, ) -> Weight;
	fn register_rewards(b: u32, ) -> Weight;
	fn set_enable_manual_reward_input() -> Weight;
}

/// Weights for pallet_vortex_distribution using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: `VortexDistribution::AdminAccount` (r:1 w:1)
	// Proof: `VortexDistribution::AdminAccount` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_admin() -> Weight {
		Weight::from_all(28_948_000)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::NextVortexId` (r:1 w:1)
	// Proof: `VortexDistribution::NextVortexId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxDistStatuses` (r:0 w:1)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	fn create_vtx_dist() -> Weight {
		Weight::from_all(38_129_000)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:1)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	fn disable_vtx_dist() -> Weight {
		Weight::from_all(39_768_000)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::VtxTotalSupply` (r:0 w:1)
	// Proof: `VortexDistribution::VtxTotalSupply` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	fn set_vtx_total_supply() -> Weight {
		Weight::from_all(25_535_000)
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::ConsiderCurrentBalance` (r:0 w:1)
	// Proof: `VortexDistribution::ConsiderCurrentBalance` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn set_consider_current_balance() -> Weight {
		Weight::from_all(22_405_000)
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::DisableRedeem` (r:0 w:1)
	// Proof: `VortexDistribution::DisableRedeem` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn set_disable_redeem() -> Weight {
		Weight::from_all(22_244_000)
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
	// Storage: `VortexDistribution::PartnerAttributionRewards` (r:1 w:0)
	// Proof: `VortexDistribution::PartnerAttributionRewards` (`max_values`: None, `max_size`: Some(7214), added: 9689, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 199]`.
	fn start_vtx_dist(p: u32, ) -> Weight {
		Weight::from_all(139_457_725)
			// Standard Error: 6_130
			.saturating_add(Weight::from_all(7_694_961_u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	// Storage: `VortexDistribution::FeePotAssetsList` (r:0 w:1)
	// Proof: `VortexDistribution::FeePotAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn set_fee_pot_asset_balances(b: u32, ) -> Weight {
		Weight::from_all(34_888_388)
			// Standard Error: 1_206
			.saturating_add(Weight::from_all(67_113_u64).saturating_mul(b as u64))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::VtxVaultAssetsList` (r:0 w:1)
	// Proof: `VortexDistribution::VtxVaultAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn set_vtx_vault_asset_balances(b: u32, ) -> Weight {
		Weight::from_all(34_286_415)
			// Standard Error: 1_480
			.saturating_add(Weight::from_all(72_707_u64).saturating_mul(b as u64))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::AssetPrices` (r:0 w:500)
	// Proof: `VortexDistribution::AssetPrices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn set_asset_prices(b: u32, ) -> Weight {
		Weight::from_all(835_151)
			// Standard Error: 13_685
			.saturating_add(Weight::from_all(3_824_386_u64).saturating_mul(b as u64))
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
		Weight::from_all(27_156_501)
			// Standard Error: 7_668
			.saturating_add(Weight::from_all(8_185_317_u64).saturating_mul(b as u64))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(b as u64)))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(b as u64)))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:0)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalWorkPoints` (r:1 w:1)
	// Proof: `VortexDistribution::TotalWorkPoints` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::RewardPoints` (r:500 w:0)
	// Proof: `VortexDistribution::RewardPoints` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::WorkPoints` (r:500 w:500)
	// Proof: `VortexDistribution::WorkPoints` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn register_work_points(b: u32, ) -> Weight {
		Weight::from_all(63_335_000)
			// Standard Error: 11_739
			.saturating_add(Weight::from_all(15_698_452_u64).saturating_mul(b as u64))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(b as u64)))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(b as u64)))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:1)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxVaultAssetsList` (r:1 w:0)
	// Proof: `VortexDistribution::VtxVaultAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::AssetPrices` (r:501 w:0)
	// Proof: `VortexDistribution::AssetPrices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	// Storage: `Assets::Metadata` (r:501 w:0)
	// Proof: `Assets::Metadata` (`max_values`: None, `max_size`: Some(140), added: 2615, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::ConsiderCurrentBalance` (r:1 w:0)
	// Proof: `VortexDistribution::ConsiderCurrentBalance` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxTotalSupply` (r:1 w:0)
	// Proof: `VortexDistribution::VtxTotalSupply` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::FeePotAssetsList` (r:1 w:0)
	// Proof: `VortexDistribution::FeePotAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:3 w:3)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:499 w:499)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:998 w:998)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::EnableManualRewardInput` (r:1 w:0)
	// Proof: `VortexDistribution::EnableManualRewardInput` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	// Storage: `PartnerAttribution::Partners` (r:200 w:199)
	// Proof: `PartnerAttribution::Partners` (`max_values`: None, `max_size`: Some(85), added: 2560, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::PartnerAttributions` (r:0 w:1)
	// Proof: `VortexDistribution::PartnerAttributions` (`max_values`: None, `max_size`: Some(8214), added: 10689, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalAttributionRewards` (r:0 w:1)
	// Proof: `VortexDistribution::TotalAttributionRewards` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalVortex` (r:0 w:1)
	// Proof: `VortexDistribution::TotalVortex` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::PartnerAttributionRewards` (r:0 w:1)
	// Proof: `VortexDistribution::PartnerAttributionRewards` (`max_values`: None, `max_size`: Some(7214), added: 9689, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalNetworkReward` (r:0 w:1)
	// Proof: `VortexDistribution::TotalNetworkReward` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalBootstrapReward` (r:0 w:1)
	// Proof: `VortexDistribution::TotalBootstrapReward` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxPrice` (r:0 w:1)
	// Proof: `VortexDistribution::VtxPrice` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 499]`.
	/// The range of component `p` is `[1, 199]`.
	fn trigger_vtx_distribution(b: u32, p: u32, ) -> Weight {
		Weight::from_all(4_688_372_000)
			// Standard Error: 137_123
			.saturating_add(Weight::from_all(106_501_316_u64).saturating_mul(b as u64))
			// Standard Error: 343_950
			.saturating_add(Weight::from_all(1_884_787_u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads(14))
			.saturating_add(T::DbWeight::get().reads((5_u64).saturating_mul(b as u64)))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(p as u64)))
			.saturating_add(T::DbWeight::get().writes(11))
			.saturating_add(T::DbWeight::get().writes((3_u64).saturating_mul(b as u64)))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(p as u64)))
	}
	// Storage: `VortexDistribution::DisableRedeem` (r:1 w:0)
	// Proof: `VortexDistribution::DisableRedeem` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:2 w:2)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:3 w:3)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxVaultRedeemAssetList` (r:1 w:0)
	// Proof: `VortexDistribution::VtxVaultRedeemAssetList` (`max_values`: Some(1), `max_size`: Some(2002), added: 2497, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:2 w:2)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	fn redeem_tokens_from_vault() -> Weight {
		Weight::from_all(320_495_000)
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(7))
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
		Weight::from_all(229_878_000)
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(8))
	}
	// Storage: `VortexDistribution::VtxVaultRedeemAssetList` (r:0 w:1)
	// Proof: `VortexDistribution::VtxVaultRedeemAssetList` (`max_values`: Some(1), `max_size`: Some(2002), added: 2497, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn set_vtx_vault_redeem_asset_list(b: u32, ) -> Weight {
		Weight::from_all(26_084_360)
			// Standard Error: 131
			.saturating_add(Weight::from_all(10_351_u64).saturating_mul(b as u64))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::EnableManualRewardInput` (r:1 w:0)
	// Proof: `VortexDistribution::EnableManualRewardInput` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:0)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalVortex` (r:1 w:1)
	// Proof: `VortexDistribution::TotalVortex` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxDistOrderbook` (r:500 w:500)
	// Proof: `VortexDistribution::VtxDistOrderbook` (`max_values`: Some(4294967295), `max_size`: Some(73), added: 4033, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn register_rewards(b: u32, ) -> Weight {
		Weight::from_all(41_670_068)
			// Standard Error: 7_464
			.saturating_add(Weight::from_all(8_695_246_u64).saturating_mul(b as u64))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(b as u64)))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(b as u64)))
	}
	// Storage: `VortexDistribution::EnableManualRewardInput` (r:0 w:1)
	// Proof: `VortexDistribution::EnableManualRewardInput` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn set_enable_manual_reward_input() -> Weight {
		Weight::from_all(23_799_000)
			.saturating_add(T::DbWeight::get().writes(1))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: `VortexDistribution::AdminAccount` (r:1 w:1)
	// Proof: `VortexDistribution::AdminAccount` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_admin() -> Weight {
		Weight::from_all(28_948_000)
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::NextVortexId` (r:1 w:1)
	// Proof: `VortexDistribution::NextVortexId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxDistStatuses` (r:0 w:1)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	fn create_vtx_dist() -> Weight {
		Weight::from_all(38_129_000)
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(2))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:1)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	fn disable_vtx_dist() -> Weight {
		Weight::from_all(39_768_000)
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::VtxTotalSupply` (r:0 w:1)
	// Proof: `VortexDistribution::VtxTotalSupply` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	fn set_vtx_total_supply() -> Weight {
		Weight::from_all(25_535_000)
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::ConsiderCurrentBalance` (r:0 w:1)
	// Proof: `VortexDistribution::ConsiderCurrentBalance` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn set_consider_current_balance() -> Weight {
		Weight::from_all(22_405_000)
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::DisableRedeem` (r:0 w:1)
	// Proof: `VortexDistribution::DisableRedeem` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn set_disable_redeem() -> Weight {
		Weight::from_all(22_244_000)
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
	// Storage: `VortexDistribution::PartnerAttributionRewards` (r:1 w:0)
	// Proof: `VortexDistribution::PartnerAttributionRewards` (`max_values`: None, `max_size`: Some(7214), added: 9689, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 199]`.
	fn start_vtx_dist(p: u32, ) -> Weight {
		Weight::from_all(139_457_725)
			// Standard Error: 6_130
			.saturating_add(Weight::from_all(7_694_961_u64).saturating_mul(p as u64))
			.saturating_add(RocksDbWeight::get().reads(6))
			.saturating_add(RocksDbWeight::get().writes(4))
	}
	// Storage: `VortexDistribution::FeePotAssetsList` (r:0 w:1)
	// Proof: `VortexDistribution::FeePotAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn set_fee_pot_asset_balances(b: u32, ) -> Weight {
		Weight::from_all(34_888_388)
			// Standard Error: 1_206
			.saturating_add(Weight::from_all(67_113_u64).saturating_mul(b as u64))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::VtxVaultAssetsList` (r:0 w:1)
	// Proof: `VortexDistribution::VtxVaultAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn set_vtx_vault_asset_balances(b: u32, ) -> Weight {
		Weight::from_all(34_286_415)
			// Standard Error: 1_480
			.saturating_add(Weight::from_all(72_707_u64).saturating_mul(b as u64))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::AssetPrices` (r:0 w:500)
	// Proof: `VortexDistribution::AssetPrices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn set_asset_prices(b: u32, ) -> Weight {
		Weight::from_all(835_151)
			// Standard Error: 13_685
			.saturating_add(Weight::from_all(3_824_386_u64).saturating_mul(b as u64))
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
		Weight::from_all(27_156_501)
			// Standard Error: 7_668
			.saturating_add(Weight::from_all(8_185_317_u64).saturating_mul(b as u64))
			.saturating_add(RocksDbWeight::get().reads(2))
			.saturating_add(RocksDbWeight::get().reads((1_u64).saturating_mul(b as u64)))
			.saturating_add(RocksDbWeight::get().writes(1))
			.saturating_add(RocksDbWeight::get().writes((1_u64).saturating_mul(b as u64)))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:0)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalWorkPoints` (r:1 w:1)
	// Proof: `VortexDistribution::TotalWorkPoints` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::RewardPoints` (r:500 w:0)
	// Proof: `VortexDistribution::RewardPoints` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::WorkPoints` (r:500 w:500)
	// Proof: `VortexDistribution::WorkPoints` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn register_work_points(b: u32, ) -> Weight {
		Weight::from_all(63_335_000)
			// Standard Error: 11_739
			.saturating_add(Weight::from_all(15_698_452_u64).saturating_mul(b as u64))
			.saturating_add(RocksDbWeight::get().reads(2))
			.saturating_add(RocksDbWeight::get().reads((2_u64).saturating_mul(b as u64)))
			.saturating_add(RocksDbWeight::get().writes(1))
			.saturating_add(RocksDbWeight::get().writes((1_u64).saturating_mul(b as u64)))
	}
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:1)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxVaultAssetsList` (r:1 w:0)
	// Proof: `VortexDistribution::VtxVaultAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::AssetPrices` (r:501 w:0)
	// Proof: `VortexDistribution::AssetPrices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	// Storage: `Assets::Metadata` (r:501 w:0)
	// Proof: `Assets::Metadata` (`max_values`: None, `max_size`: Some(140), added: 2615, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::ConsiderCurrentBalance` (r:1 w:0)
	// Proof: `VortexDistribution::ConsiderCurrentBalance` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxTotalSupply` (r:1 w:0)
	// Proof: `VortexDistribution::VtxTotalSupply` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::FeePotAssetsList` (r:1 w:0)
	// Proof: `VortexDistribution::FeePotAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:3 w:3)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:499 w:499)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:998 w:998)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::EnableManualRewardInput` (r:1 w:0)
	// Proof: `VortexDistribution::EnableManualRewardInput` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	// Storage: `PartnerAttribution::Partners` (r:200 w:199)
	// Proof: `PartnerAttribution::Partners` (`max_values`: None, `max_size`: Some(85), added: 2560, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::PartnerAttributions` (r:0 w:1)
	// Proof: `VortexDistribution::PartnerAttributions` (`max_values`: None, `max_size`: Some(8214), added: 10689, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalAttributionRewards` (r:0 w:1)
	// Proof: `VortexDistribution::TotalAttributionRewards` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalVortex` (r:0 w:1)
	// Proof: `VortexDistribution::TotalVortex` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::PartnerAttributionRewards` (r:0 w:1)
	// Proof: `VortexDistribution::PartnerAttributionRewards` (`max_values`: None, `max_size`: Some(7214), added: 9689, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalNetworkReward` (r:0 w:1)
	// Proof: `VortexDistribution::TotalNetworkReward` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalBootstrapReward` (r:0 w:1)
	// Proof: `VortexDistribution::TotalBootstrapReward` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxPrice` (r:0 w:1)
	// Proof: `VortexDistribution::VtxPrice` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 499]`.
	/// The range of component `p` is `[1, 199]`.
	fn trigger_vtx_distribution(b: u32, p: u32, ) -> Weight {
		Weight::from_all(4_688_372_000)
			// Standard Error: 137_123
			.saturating_add(Weight::from_all(106_501_316_u64).saturating_mul(b as u64))
			// Standard Error: 343_950
			.saturating_add(Weight::from_all(1_884_787_u64).saturating_mul(p as u64))
			.saturating_add(RocksDbWeight::get().reads(14))
			.saturating_add(RocksDbWeight::get().reads((5_u64).saturating_mul(b as u64)))
			.saturating_add(RocksDbWeight::get().reads((1_u64).saturating_mul(p as u64)))
			.saturating_add(RocksDbWeight::get().writes(11))
			.saturating_add(RocksDbWeight::get().writes((3_u64).saturating_mul(b as u64)))
			.saturating_add(RocksDbWeight::get().writes((1_u64).saturating_mul(p as u64)))
	}
	// Storage: `VortexDistribution::DisableRedeem` (r:1 w:0)
	// Proof: `VortexDistribution::DisableRedeem` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:2 w:2)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:3 w:3)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxVaultRedeemAssetList` (r:1 w:0)
	// Proof: `VortexDistribution::VtxVaultRedeemAssetList` (`max_values`: Some(1), `max_size`: Some(2002), added: 2497, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:2 w:2)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	fn redeem_tokens_from_vault() -> Weight {
		Weight::from_all(320_495_000)
			.saturating_add(RocksDbWeight::get().reads(9))
			.saturating_add(RocksDbWeight::get().writes(7))
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
		Weight::from_all(229_878_000)
			.saturating_add(RocksDbWeight::get().reads(9))
			.saturating_add(RocksDbWeight::get().writes(8))
	}
	// Storage: `VortexDistribution::VtxVaultRedeemAssetList` (r:0 w:1)
	// Proof: `VortexDistribution::VtxVaultRedeemAssetList` (`max_values`: Some(1), `max_size`: Some(2002), added: 2497, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn set_vtx_vault_redeem_asset_list(b: u32, ) -> Weight {
		Weight::from_all(26_084_360)
			// Standard Error: 131
			.saturating_add(Weight::from_all(10_351_u64).saturating_mul(b as u64))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	// Storage: `VortexDistribution::EnableManualRewardInput` (r:1 w:0)
	// Proof: `VortexDistribution::EnableManualRewardInput` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:0)
	// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::TotalVortex` (r:1 w:1)
	// Proof: `VortexDistribution::TotalVortex` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	// Storage: `VortexDistribution::VtxDistOrderbook` (r:500 w:500)
	// Proof: `VortexDistribution::VtxDistOrderbook` (`max_values`: Some(4294967295), `max_size`: Some(73), added: 4033, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn register_rewards(b: u32, ) -> Weight {
		Weight::from_all(41_670_068)
			// Standard Error: 7_464
			.saturating_add(Weight::from_all(8_695_246_u64).saturating_mul(b as u64))
			.saturating_add(RocksDbWeight::get().reads(3))
			.saturating_add(RocksDbWeight::get().reads((1_u64).saturating_mul(b as u64)))
			.saturating_add(RocksDbWeight::get().writes(1))
			.saturating_add(RocksDbWeight::get().writes((1_u64).saturating_mul(b as u64)))
	}
	// Storage: `VortexDistribution::EnableManualRewardInput` (r:0 w:1)
	// Proof: `VortexDistribution::EnableManualRewardInput` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn set_enable_manual_reward_input() -> Weight {
		Weight::from_all(23_799_000)
			.saturating_add(RocksDbWeight::get().writes(1))
	}
}

