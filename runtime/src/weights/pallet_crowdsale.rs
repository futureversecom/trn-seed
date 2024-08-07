
//! Autogenerated weights for `pallet_crowdsale`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-05-23, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ip-172-31-102-147`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: ``, WASM-EXECUTION: `Compiled`, CHAIN: `Some("dev")`, DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-crowdsale
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_crowdsale.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_crowdsale`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_crowdsale::WeightInfo for WeightInfo<T> {
	/// Storage: `Crowdsale::NextSaleId` (r:1 w:1)
	/// Proof: `Crowdsale::NextSaleId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:2 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Nft::PublicMintInfo` (r:1 w:0)
	/// Proof: `Nft::PublicMintInfo` (`max_values`: None, `max_size`: Some(34), added: 2509, mode: `MaxEncodedLen`)
	/// Storage: `AssetsExt::NextAssetId` (r:1 w:1)
	/// Proof: `AssetsExt::NextAssetId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `EVM::AccountCodes` (r:1 w:1)
	/// Proof: `EVM::AccountCodes` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Futurepass::DefaultProxy` (r:1 w:0)
	/// Proof: `Futurepass::DefaultProxy` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Metadata` (r:1 w:1)
	/// Proof: `Assets::Metadata` (`max_values`: None, `max_size`: Some(140), added: 2615, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	/// Storage: `EVM::AccountCodesMetadata` (r:0 w:1)
	/// Proof: `EVM::AccountCodesMetadata` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Crowdsale::SaleInfo` (r:0 w:1)
	/// Proof: `Crowdsale::SaleInfo` (`max_values`: None, `max_size`: Some(133), added: 2608, mode: `MaxEncodedLen`)
	fn initialize() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1420`
		//  Estimated: `6264`
		// Minimum execution time: 286_688_000 picoseconds.
		Weight::from_parts(288_630_000, 0)
			.saturating_add(Weight::from_parts(0, 6264))
			.saturating_add(T::DbWeight::get().reads(12))
			.saturating_add(T::DbWeight::get().writes(11))
	}
	/// Storage: `Crowdsale::SaleInfo` (r:1 w:1)
	/// Proof: `Crowdsale::SaleInfo` (`max_values`: None, `max_size`: Some(133), added: 2608, mode: `MaxEncodedLen`)
	/// Storage: `Crowdsale::SaleEndBlocks` (r:1 w:1)
	/// Proof: `Crowdsale::SaleEndBlocks` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	fn enable() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `215`
		//  Estimated: `3598`
		// Minimum execution time: 58_817_000 picoseconds.
		Weight::from_parts(60_031_000, 0)
			.saturating_add(Weight::from_parts(0, 3598))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Crowdsale::SaleInfo` (r:1 w:1)
	/// Proof: `Crowdsale::SaleInfo` (`max_values`: None, `max_size`: Some(133), added: 2608, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	/// Storage: `Crowdsale::SaleParticipation` (r:1 w:1)
	/// Proof: `Crowdsale::SaleParticipation` (`max_values`: None, `max_size`: Some(60), added: 2535, mode: `MaxEncodedLen`)
	fn participate() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1042`
		//  Estimated: `6172`
		// Minimum execution time: 176_128_000 picoseconds.
		Weight::from_parts(177_706_000, 0)
			.saturating_add(Weight::from_parts(0, 6172))
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(7))
	}
	/// Storage: `Crowdsale::SaleDistribution` (r:1 w:1)
	/// Proof: `Crowdsale::SaleDistribution` (`max_values`: Some(1), `max_size`: Some(16002), added: 16497, mode: `MaxEncodedLen`)
	/// Storage: `Crowdsale::SaleInfo` (r:1 w:1)
	/// Proof: `Crowdsale::SaleInfo` (`max_values`: None, `max_size`: Some(133), added: 2608, mode: `MaxEncodedLen`)
	/// Storage: `Nft::CollectionInfo` (r:1 w:0)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Crowdsale::SaleParticipation` (r:2 w:1)
	/// Proof: `Crowdsale::SaleParticipation` (`max_values`: None, `max_size`: Some(60), added: 2535, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	/// Storage: `Crowdsale::NextUnsignedAt` (r:0 w:1)
	/// Proof: `Crowdsale::NextUnsignedAt` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn distribute_crowdsale_rewards() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1387`
		//  Estimated: `17487`
		// Minimum execution time: 259_430_000 picoseconds.
		Weight::from_parts(261_562_000, 0)
			.saturating_add(Weight::from_parts(0, 17487))
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(9))
	}
	/// Storage: `Crowdsale::SaleInfo` (r:1 w:1)
	/// Proof: `Crowdsale::SaleInfo` (`max_values`: None, `max_size`: Some(133), added: 2608, mode: `MaxEncodedLen`)
	/// Storage: `Crowdsale::SaleParticipation` (r:2 w:1)
	/// Proof: `Crowdsale::SaleParticipation` (`max_values`: None, `max_size`: Some(60), added: 2535, mode: `MaxEncodedLen`)
	/// Storage: `Nft::CollectionInfo` (r:1 w:0)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	/// Storage: `Crowdsale::SaleDistribution` (r:1 w:1)
	/// Proof: `Crowdsale::SaleDistribution` (`max_values`: Some(1), `max_size`: Some(16002), added: 16497, mode: `MaxEncodedLen`)
	fn claim_voucher() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1387`
		//  Estimated: `17487`
		// Minimum execution time: 253_749_000 picoseconds.
		Weight::from_parts(256_196_000, 0)
			.saturating_add(Weight::from_parts(0, 17487))
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(8))
	}
	/// Storage: `Crowdsale::SaleInfo` (r:1 w:0)
	/// Proof: `Crowdsale::SaleInfo` (`max_values`: None, `max_size`: Some(133), added: 2608, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Nft::PublicMintInfo` (r:1 w:0)
	/// Proof: `Nft::PublicMintInfo` (`max_values`: None, `max_size`: Some(34), added: 2509, mode: `MaxEncodedLen`)
	fn redeem_voucher() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1172`
		//  Estimated: `3627`
		// Minimum execution time: 187_578_000 picoseconds.
		Weight::from_parts(189_375_000, 0)
			.saturating_add(Weight::from_parts(0, 3627))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Crowdsale::SaleInfo` (r:1 w:0)
	/// Proof: `Crowdsale::SaleInfo` (`max_values`: None, `max_size`: Some(133), added: 2608, mode: `MaxEncodedLen`)
	/// Storage: `MaintenanceMode::BlockedCalls` (r:1 w:0)
	/// Proof: `MaintenanceMode::BlockedCalls` (`max_values`: None, `max_size`: Some(111), added: 2586, mode: `MaxEncodedLen`)
	/// Storage: `MaintenanceMode::BlockedPallets` (r:1 w:0)
	/// Proof: `MaintenanceMode::BlockedPallets` (`max_values`: None, `max_size`: Some(60), added: 2535, mode: `MaxEncodedLen`)
	fn proxy_vault_call() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `324`
		//  Estimated: `3598`
		// Minimum execution time: 78_314_000 picoseconds.
		Weight::from_parts(79_096_000, 0)
			.saturating_add(Weight::from_parts(0, 3598))
			.saturating_add(T::DbWeight::get().reads(3))
	}
	/// Storage: `Crowdsale::SaleInfo` (r:1 w:1)
	/// Proof: `Crowdsale::SaleInfo` (`max_values`: None, `max_size`: Some(133), added: 2608, mode: `MaxEncodedLen`)
	/// Storage: `Crowdsale::SaleDistribution` (r:1 w:1)
	/// Proof: `Crowdsale::SaleDistribution` (`max_values`: Some(1), `max_size`: Some(16002), added: 16497, mode: `MaxEncodedLen`)
	fn try_force_distribution() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `248`
		//  Estimated: `17487`
		// Minimum execution time: 52_935_000 picoseconds.
		Weight::from_parts(53_443_000, 0)
			.saturating_add(Weight::from_parts(0, 17487))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Crowdsale::SaleEndBlocks` (r:1 w:1)
	/// Proof: `Crowdsale::SaleEndBlocks` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Crowdsale::SaleInfo` (r:5 w:5)
	/// Proof: `Crowdsale::SaleInfo` (`max_values`: None, `max_size`: Some(133), added: 2608, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:10 w:10)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:20 w:20)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:6 w:6)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	/// Storage: `Nft::CollectionInfo` (r:5 w:0)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Crowdsale::SaleDistribution` (r:1 w:1)
	/// Proof: `Crowdsale::SaleDistribution` (`max_values`: Some(1), `max_size`: Some(16002), added: 16497, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 5]`.
	fn on_initialize(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `499 + p * (884 ±0)`
		//  Estimated: `17487 + p * (10340 ±0)`
		// Minimum execution time: 259_583_000 picoseconds.
		Weight::from_parts(60_135_422, 0)
			.saturating_add(Weight::from_parts(0, 17487))
			// Standard Error: 262_230
			.saturating_add(Weight::from_parts(203_977_429, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().reads((9_u64).saturating_mul(p.into())))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(T::DbWeight::get().writes((8_u64).saturating_mul(p.into())))
			.saturating_add(Weight::from_parts(0, 10340).saturating_mul(p.into()))
	}
	/// Storage: `Crowdsale::SaleEndBlocks` (r:1 w:0)
	/// Proof: `Crowdsale::SaleEndBlocks` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	fn on_initialize_empty() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `42`
		//  Estimated: `3518`
		// Minimum execution time: 9_101_000 picoseconds.
		Weight::from_parts(9_267_000, 0)
			.saturating_add(Weight::from_parts(0, 3518))
			.saturating_add(T::DbWeight::get().reads(1))
	}
}
