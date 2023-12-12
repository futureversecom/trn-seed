
//! Autogenerated weights for `pallet_token_approvals`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-09-19, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-102-147`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_token_approvals
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_token_approvals.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_token_approvals`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_token_approvals::WeightInfo for WeightInfo<T> {
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: TokenApprovals ERC721ApprovalsForAll (r:1 w:0)
	// Storage: TokenApprovals ERC721Approvals (r:0 w:1)
	fn erc721_approval() -> Weight {
		Weight::from_ref_time(41_824_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: TokenApprovals ERC721Approvals (r:1 w:1)
	// Storage: Nft CollectionInfo (r:1 w:0)
	fn erc721_remove_approval() -> Weight {
		Weight::from_ref_time(42_988_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: TokenApprovals ERC20Approvals (r:0 w:1)
	fn erc20_approval() -> Weight {
		Weight::from_ref_time(21_470_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: TokenApprovals ERC20Approvals (r:1 w:1)
	fn erc20_update_approval() -> Weight {
		Weight::from_ref_time(32_760_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: TokenApprovals ERC721ApprovalsForAll (r:0 w:1)
	fn erc721_approval_for_all() -> Weight {
		Weight::from_ref_time(21_398_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: TokenApprovals ERC1155ApprovalsForAll (r:0 w:1)
	fn erc1155_approval_for_all() -> Weight {
		Weight::from_ref_time(21_109_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
}
