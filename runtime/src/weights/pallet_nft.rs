
//! Autogenerated weights for `pallet_nft`
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
// ./output/pallet_nft.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_nft`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_nft::WeightInfo for WeightInfo<T> {
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn claim_unowned_collection() -> Weight {
		Weight::from_ref_time(61_290_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn set_owner() -> Weight {
		Weight::from_ref_time(63_384_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn set_max_issuance() -> Weight {
		Weight::from_ref_time(63_020_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn set_base_uri() -> Weight {
		Weight::from_ref_time(66_963_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn set_name() -> Weight {
		Weight::from_ref_time(67_610_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Nft NextCollectionId (r:1 w:1)
	// Storage: EVM AccountCodes (r:1 w:1)
	// Storage: Futurepass DefaultProxy (r:1 w:0)
	// Storage: System Account (r:1 w:1)
	// Storage: Nft CollectionInfo (r:0 w:1)
	fn create_collection() -> Weight {
		Weight::from_ref_time(103_001_000 as u64)
			.saturating_add(T::DbWeight::get().reads(4 as u64))
			.saturating_add(T::DbWeight::get().writes(4 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn mint() -> Weight {
		Weight::from_ref_time(73_200_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: Nft TokenLocks (r:1 w:0)
	// Storage: TokenApprovals ERC721Approvals (r:0 w:1)
	fn transfer() -> Weight {
		Weight::from_ref_time(79_873_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: Nft TokenLocks (r:1 w:0)
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: TokenApprovals ERC721Approvals (r:0 w:1)
	fn burn() -> Weight {
		Weight::from_ref_time(77_993_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
}
