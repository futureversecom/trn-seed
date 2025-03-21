
//! Autogenerated weights for `pallet_vortex_distribution`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2025-03-21, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `Surangas-MacBook-Pro.local`, CPU: `<UNKNOWN>`
//! EXECUTION: ``, WASM-EXECUTION: `Compiled`, CHAIN: `Some("dev")`, DB CACHE: 1024

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
// ./runtime/src/weights/pallet_vortex_distribution.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_vortex_distribution`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_vortex_distribution::WeightInfo for WeightInfo<T> {
	/// Storage: `VortexDistribution::AdminAccount` (r:0 w:1)
	/// Proof: `VortexDistribution::AdminAccount` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_admin() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 7_000_000 picoseconds.
		Weight::from_parts(8_000_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `VortexDistribution::NextVortexId` (r:1 w:1)
	/// Proof: `VortexDistribution::NextVortexId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::VtxDistStatuses` (r:0 w:1)
	/// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	fn create_vtx_dist() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `142`
		//  Estimated: `1489`
		// Minimum execution time: 12_000_000 picoseconds.
		Weight::from_parts(12_000_000, 0)
			.saturating_add(Weight::from_parts(0, 1489))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:1)
	/// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	fn disable_vtx_dist() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `202`
		//  Estimated: `3478`
		// Minimum execution time: 13_000_000 picoseconds.
		Weight::from_parts(13_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3478))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `VortexDistribution::VtxTotalSupply` (r:0 w:1)
	/// Proof: `VortexDistribution::VtxTotalSupply` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	fn set_vtx_total_supply() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 7_000_000 picoseconds.
		Weight::from_parts(8_000_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `VortexDistribution::ConsiderCurrentBalance` (r:0 w:1)
	/// Proof: `VortexDistribution::ConsiderCurrentBalance` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn set_consider_current_balance() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 6_000_000 picoseconds.
		Weight::from_parts(7_000_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `VortexDistribution::DisableRedeem` (r:0 w:1)
	/// Proof: `VortexDistribution::DisableRedeem` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn set_disable_redeem() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 6_000_000 picoseconds.
		Weight::from_parts(7_000_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:1)
	/// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::TotalVortex` (r:1 w:0)
	/// Proof: `VortexDistribution::TotalVortex` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	fn start_vtx_dist() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1122`
		//  Estimated: `3627`
		// Minimum execution time: 45_000_000 picoseconds.
		Weight::from_parts(47_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3627))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `VortexDistribution::FeePotAssetsList` (r:0 w:1)
	/// Proof: `VortexDistribution::FeePotAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn set_fee_pot_asset_balances(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 8_000_000 picoseconds.
		Weight::from_parts(15_648_088, 0)
			.saturating_add(Weight::from_parts(0, 0))
			// Standard Error: 1_441
			.saturating_add(Weight::from_parts(43_420, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `VortexDistribution::VtxVaultAssetsList` (r:0 w:1)
	/// Proof: `VortexDistribution::VtxVaultAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn set_vtx_vault_asset_balances(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 8_000_000 picoseconds.
		Weight::from_parts(10_524_644, 0)
			.saturating_add(Weight::from_parts(0, 0))
			// Standard Error: 1_059
			.saturating_add(Weight::from_parts(56_016, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `VortexDistribution::FeePotAssetsList` (r:1 w:0)
	/// Proof: `VortexDistribution::FeePotAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::AssetPrices` (r:0 w:500)
	/// Proof: `VortexDistribution::AssetPrices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn set_asset_prices(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `242 + b * (20 ±0)`
		//  Estimated: `13479`
		// Minimum execution time: 16_000_000 picoseconds.
		Weight::from_parts(16_000_000, 0)
			.saturating_add(Weight::from_parts(0, 13479))
			// Standard Error: 26_830
			.saturating_add(Weight::from_parts(5_318_765, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(b.into())))
	}
	/// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:0)
	/// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::TotalRewardPoints` (r:1 w:1)
	/// Proof: `VortexDistribution::TotalRewardPoints` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::RewardPoints` (r:500 w:500)
	/// Proof: `VortexDistribution::RewardPoints` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn register_reward_points(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `202`
		//  Estimated: `3493 + b * (2547 ±0)`
		// Minimum execution time: 16_000_000 picoseconds.
		Weight::from_parts(2_126_198, 0)
			.saturating_add(Weight::from_parts(0, 3493))
			// Standard Error: 7_923
			.saturating_add(Weight::from_parts(3_901_575, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(b.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(b.into())))
			.saturating_add(Weight::from_parts(0, 2547).saturating_mul(b.into()))
	}
	/// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:0)
	/// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::TotalWorkPoints` (r:1 w:1)
	/// Proof: `VortexDistribution::TotalWorkPoints` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::WorkPoints` (r:500 w:500)
	/// Proof: `VortexDistribution::WorkPoints` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 500]`.
	fn register_work_points(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `202`
		//  Estimated: `3493 + b * (2547 ±0)`
		// Minimum execution time: 16_000_000 picoseconds.
		Weight::from_parts(16_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3493))
			// Standard Error: 4_045
			.saturating_add(Weight::from_parts(3_839_868, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(b.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(b.into())))
			.saturating_add(Weight::from_parts(0, 2547).saturating_mul(b.into()))
	}
	/// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:1)
	/// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::VtxVaultAssetsList` (r:1 w:0)
	/// Proof: `VortexDistribution::VtxVaultAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::AssetPrices` (r:2 w:0)
	/// Proof: `VortexDistribution::AssetPrices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::ConsiderCurrentBalance` (r:1 w:0)
	/// Proof: `VortexDistribution::ConsiderCurrentBalance` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::VtxTotalSupply` (r:1 w:0)
	/// Proof: `VortexDistribution::VtxTotalSupply` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::FeePotAssetsList` (r:1 w:0)
	/// Proof: `VortexDistribution::FeePotAssetsList` (`max_values`: None, `max_size`: Some(10014), added: 12489, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::TotalRewardPoints` (r:1 w:0)
	/// Proof: `VortexDistribution::TotalRewardPoints` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::TotalWorkPoints` (r:1 w:0)
	/// Proof: `VortexDistribution::TotalWorkPoints` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::RewardPoints` (r:2 w:0)
	/// Proof: `VortexDistribution::RewardPoints` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::WorkPoints` (r:1 w:0)
	/// Proof: `VortexDistribution::WorkPoints` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::VtxDistOrderbook` (r:1 w:1)
	/// Proof: `VortexDistribution::VtxDistOrderbook` (`max_values`: Some(4294967295), `max_size`: Some(73), added: 4033, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::TotalVortex` (r:0 w:1)
	/// Proof: `VortexDistribution::TotalVortex` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::TotalNetworkReward` (r:0 w:1)
	/// Proof: `VortexDistribution::TotalNetworkReward` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::TotalBootstrapReward` (r:0 w:1)
	/// Proof: `VortexDistribution::TotalBootstrapReward` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::VtxPrice` (r:0 w:1)
	/// Proof: `VortexDistribution::VtxPrice` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	fn trigger_vtx_distribution() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1744`
		//  Estimated: `13479`
		// Minimum execution time: 200_000_000 picoseconds.
		Weight::from_parts(202_000_000, 0)
			.saturating_add(Weight::from_parts(0, 13479))
			.saturating_add(T::DbWeight::get().reads(19))
			.saturating_add(T::DbWeight::get().writes(12))
	}
	/// Storage: `VortexDistribution::DisableRedeem` (r:1 w:0)
	/// Proof: `VortexDistribution::DisableRedeem` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	fn redeem_tokens_from_vault() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1214`
		//  Estimated: `3627`
		// Minimum execution time: 51_000_000 picoseconds.
		Weight::from_parts(52_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3627))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `VortexDistribution::VtxDistStatuses` (r:1 w:0)
	/// Proof: `VortexDistribution::VtxDistStatuses` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::VtxDistPayoutPivot` (r:1 w:1)
	/// Proof: `VortexDistribution::VtxDistPayoutPivot` (`max_values`: None, `max_size`: Some(1014), added: 3489, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::VtxDistOrderbook` (r:2 w:1)
	/// Proof: `VortexDistribution::VtxDistOrderbook` (`max_values`: Some(4294967295), `max_size`: Some(73), added: 4033, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	/// Storage: `VortexDistribution::NextUnsignedAt` (r:0 w:1)
	/// Proof: `VortexDistribution::NextUnsignedAt` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn pay_unsigned() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1504`
		//  Estimated: `9056`
		// Minimum execution time: 81_000_000 picoseconds.
		Weight::from_parts(85_000_000, 0)
			.saturating_add(Weight::from_parts(0, 9056))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(8))
	}
}
