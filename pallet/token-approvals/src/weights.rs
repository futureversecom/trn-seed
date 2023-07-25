
//! Autogenerated weights for pallet_token_approvals
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-06-02, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `Xiankuns-MBP-2`, CPU: `<UNKNOWN>`
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
// ./output/pallet_token_approvals_weights.rs
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_token_approvals.
pub trait WeightInfo {
	fn erc721_approval() -> Weight;
	fn erc721_remove_approval() -> Weight;
	fn erc20_approval() -> Weight;
	fn erc20_update_approval() -> Weight;
	fn erc721_approval_for_all() -> Weight;
}

/// Weights for pallet_token_approvals using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	/// Storage: Nft CollectionInfo (r:1 w:0)
	/// Proof Skipped: Nft CollectionInfo (max_values: None, max_size: None, mode: Measured)
	/// Storage: TokenApprovals ERC721ApprovalsForAll (r:1 w:0)
	/// Proof: TokenApprovals ERC721ApprovalsForAll (max_values: None, max_size: Some(61), added: 2536, mode: MaxEncodedLen)
	/// Storage: TokenApprovals ERC721Approvals (r:0 w:1)
	/// Proof: TokenApprovals ERC721Approvals (max_values: None, max_size: Some(36), added: 2511, mode: MaxEncodedLen)
	fn erc721_approval() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `506`
		//  Estimated: `7497`
		// Minimum execution time: 15_000_000 picoseconds.
		Weight::from_parts(15_000_000, 7497)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: TokenApprovals ERC721Approvals (r:1 w:1)
	/// Proof: TokenApprovals ERC721Approvals (max_values: None, max_size: Some(36), added: 2511, mode: MaxEncodedLen)
	/// Storage: Nft CollectionInfo (r:1 w:0)
	/// Proof Skipped: Nft CollectionInfo (max_values: None, max_size: None, mode: Measured)
	fn erc721_remove_approval() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `567`
		//  Estimated: `7533`
		// Minimum execution time: 14_000_000 picoseconds.
		Weight::from_parts(15_000_000, 7533)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: TokenApprovals ERC20Approvals (r:0 w:1)
	/// Proof: TokenApprovals ERC20Approvals (max_values: None, max_size: Some(76), added: 2551, mode: MaxEncodedLen)
	fn erc20_approval() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 4_000_000 picoseconds.
		Weight::from_parts(5_000_000, 0)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: TokenApprovals ERC20Approvals (r:1 w:1)
	/// Proof: TokenApprovals ERC20Approvals (max_values: None, max_size: Some(76), added: 2551, mode: MaxEncodedLen)
	fn erc20_update_approval() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `144`
		//  Estimated: `3541`
		// Minimum execution time: 11_000_000 picoseconds.
		Weight::from_parts(11_000_000, 3541)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: TokenApprovals ERC721ApprovalsForAll (r:0 w:1)
	/// Proof: TokenApprovals ERC721ApprovalsForAll (max_values: None, max_size: Some(61), added: 2536, mode: MaxEncodedLen)
	fn erc721_approval_for_all() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 4_000_000 picoseconds.
		Weight::from_parts(4_000_000, 0)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	/// Storage: Nft CollectionInfo (r:1 w:0)
	/// Proof Skipped: Nft CollectionInfo (max_values: None, max_size: None, mode: Measured)
	/// Storage: TokenApprovals ERC721ApprovalsForAll (r:1 w:0)
	/// Proof: TokenApprovals ERC721ApprovalsForAll (max_values: None, max_size: Some(61), added: 2536, mode: MaxEncodedLen)
	/// Storage: TokenApprovals ERC721Approvals (r:0 w:1)
	/// Proof: TokenApprovals ERC721Approvals (max_values: None, max_size: Some(36), added: 2511, mode: MaxEncodedLen)
	fn erc721_approval() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `506`
		//  Estimated: `7497`
		// Minimum execution time: 15_000_000 picoseconds.
		Weight::from_parts(15_000_000, 7497)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: TokenApprovals ERC721Approvals (r:1 w:1)
	/// Proof: TokenApprovals ERC721Approvals (max_values: None, max_size: Some(36), added: 2511, mode: MaxEncodedLen)
	/// Storage: Nft CollectionInfo (r:1 w:0)
	/// Proof Skipped: Nft CollectionInfo (max_values: None, max_size: None, mode: Measured)
	fn erc721_remove_approval() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `567`
		//  Estimated: `7533`
		// Minimum execution time: 14_000_000 picoseconds.
		Weight::from_parts(15_000_000, 7533)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: TokenApprovals ERC20Approvals (r:0 w:1)
	/// Proof: TokenApprovals ERC20Approvals (max_values: None, max_size: Some(76), added: 2551, mode: MaxEncodedLen)
	fn erc20_approval() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 4_000_000 picoseconds.
		Weight::from_parts(5_000_000, 0)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: TokenApprovals ERC20Approvals (r:1 w:1)
	/// Proof: TokenApprovals ERC20Approvals (max_values: None, max_size: Some(76), added: 2551, mode: MaxEncodedLen)
	fn erc20_update_approval() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `144`
		//  Estimated: `3541`
		// Minimum execution time: 11_000_000 picoseconds.
		Weight::from_parts(11_000_000, 3541)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: TokenApprovals ERC721ApprovalsForAll (r:0 w:1)
	/// Proof: TokenApprovals ERC721ApprovalsForAll (max_values: None, max_size: Some(61), added: 2536, mode: MaxEncodedLen)
	fn erc721_approval_for_all() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 4_000_000 picoseconds.
		Weight::from_parts(4_000_000, 0)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
}
