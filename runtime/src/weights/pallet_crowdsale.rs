
//! Autogenerated weights for `pallet_crowdsale`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-03-07, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
// ./runtime/src/weights/pallet_crowdsale.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_crowdsale`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_crowdsale::WeightInfo for WeightInfo<T> {
	// Storage: Crowdsale NextSaleId (r:1 w:1)
	// Storage: Assets Asset (r:2 w:1)
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: AssetsExt NextAssetId (r:1 w:1)
	// Storage: EVM AccountCodes (r:1 w:1)
	// Storage: Futurepass DefaultProxy (r:1 w:0)
	// Storage: System Account (r:2 w:2)
	// Storage: Assets Metadata (r:1 w:1)
	// Storage: Assets Account (r:1 w:1)
	// Storage: Crowdsale SaleInfo (r:0 w:1)
	fn initialize() -> Weight {
		Weight::from_ref_time(242_201_000 as u64)
			.saturating_add(T::DbWeight::get().reads(11 as u64))
			.saturating_add(T::DbWeight::get().writes(10 as u64))
	}
	// Storage: Crowdsale SaleInfo (r:1 w:1)
	// Storage: Crowdsale SaleEndBlocks (r:1 w:1)
	fn enable() -> Weight {
		Weight::from_ref_time(69_876_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: Crowdsale SaleInfo (r:1 w:1)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:2)
	// Storage: System Account (r:2 w:2)
	// Storage: Crowdsale SaleParticipation (r:1 w:1)
	fn participate() -> Weight {
		Weight::from_ref_time(157_983_000 as u64)
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
		Weight::from_ref_time(244_465_000 as u64)
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
		Weight::from_ref_time(238_375_000 as u64)
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
		Weight::from_ref_time(176_790_000 as u64)
			.saturating_add(T::DbWeight::get().reads(6 as u64))
			.saturating_add(T::DbWeight::get().writes(4 as u64))
	}
	// Storage: Crowdsale SaleInfo (r:1 w:1)
	// Storage: Crowdsale SaleDistribution (r:1 w:1)
	fn try_force_distribution() -> Weight {
		Weight::from_ref_time(66_053_000 as u64)
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
		Weight::from_ref_time(212_761_000 as u64)
			// Standard Error: 953_527
			.saturating_add(Weight::from_ref_time(97_043_987 as u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads(12 as u64))
			.saturating_add(T::DbWeight::get().reads((6 as u64).saturating_mul(p as u64)))
			.saturating_add(T::DbWeight::get().writes(11 as u64))
			.saturating_add(T::DbWeight::get().writes((5 as u64).saturating_mul(p as u64)))
	}
	// Storage: Crowdsale SaleEndBlocks (r:1 w:0)
	fn on_initialize_empty() -> Weight {
		Weight::from_ref_time(9_826_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
	}
}