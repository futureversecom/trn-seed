
//! Autogenerated weights for `pallet_nft`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-08-16, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
// --pallet=pallet-nft
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_nft.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_nft`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_nft::WeightInfo for WeightInfo<T> {
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	fn claim_unowned_collection() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `298`
		//  Estimated: `3464`
		// Minimum execution time: 42_163_000 picoseconds.
		Weight::from_parts(42_916_000, 0)
			.saturating_add(Weight::from_parts(0, 3464))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	fn set_owner() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `298`
		//  Estimated: `3464`
		// Minimum execution time: 44_850_000 picoseconds.
		Weight::from_parts(45_554_000, 0)
			.saturating_add(Weight::from_parts(0, 3464))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	fn set_max_issuance() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `298`
		//  Estimated: `3464`
		// Minimum execution time: 44_795_000 picoseconds.
		Weight::from_parts(45_516_000, 0)
			.saturating_add(Weight::from_parts(0, 3464))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	fn set_base_uri() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `298`
		//  Estimated: `3464`
		// Minimum execution time: 46_574_000 picoseconds.
		Weight::from_parts(47_128_000, 0)
			.saturating_add(Weight::from_parts(0, 3464))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	fn set_name() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `298`
		//  Estimated: `3464`
		// Minimum execution time: 45_726_000 picoseconds.
		Weight::from_parts(46_460_000, 0)
			.saturating_add(Weight::from_parts(0, 3464))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	fn set_royalties_schedule() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `298`
		//  Estimated: `3464`
		// Minimum execution time: 46_109_000 picoseconds.
		Weight::from_parts(47_122_000, 0)
			.saturating_add(Weight::from_parts(0, 3464))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::NextCollectionId` (r:1 w:1)
	/// Proof: `Nft::NextCollectionId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `EVM::AccountCodes` (r:1 w:1)
	/// Proof: `EVM::AccountCodes` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Futurepass::DefaultProxy` (r:1 w:0)
	/// Proof: `Futurepass::DefaultProxy` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	/// Storage: `EVM::AccountCodesMetadata` (r:0 w:1)
	/// Proof: `EVM::AccountCodesMetadata` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Nft::CollectionInfo` (r:0 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	fn create_collection() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `249`
		//  Estimated: `3714`
		// Minimum execution time: 92_505_000 picoseconds.
		Weight::from_parts(93_852_000, 0)
			.saturating_add(Weight::from_parts(0, 3714))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:0)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Nft::PublicMintInfo` (r:1 w:1)
	/// Proof: `Nft::PublicMintInfo` (`max_values`: None, `max_size`: Some(34), added: 2509, mode: `MaxEncodedLen`)
	fn toggle_public_mint() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `298`
		//  Estimated: `3499`
		// Minimum execution time: 45_676_000 picoseconds.
		Weight::from_parts(48_866_000, 0)
			.saturating_add(Weight::from_parts(0, 3499))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:0)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Nft::PublicMintInfo` (r:1 w:1)
	/// Proof: `Nft::PublicMintInfo` (`max_values`: None, `max_size`: Some(34), added: 2509, mode: `MaxEncodedLen`)
	fn set_mint_fee() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `298`
		//  Estimated: `3499`
		// Minimum execution time: 48_525_000 picoseconds.
		Weight::from_parts(49_549_000, 0)
			.saturating_add(Weight::from_parts(0, 3499))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::UtilityFlags` (r:1 w:0)
	/// Proof: `Nft::UtilityFlags` (`max_values`: None, `max_size`: Some(15), added: 2490, mode: `MaxEncodedLen`)
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Nft::PublicMintInfo` (r:1 w:0)
	/// Proof: `Nft::PublicMintInfo` (`max_values`: None, `max_size`: Some(34), added: 2509, mode: `MaxEncodedLen`)
	fn mint() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `298`
		//  Estimated: `3499`
		// Minimum execution time: 58_872_000 picoseconds.
		Weight::from_parts(60_242_000, 0)
			.saturating_add(Weight::from_parts(0, 3499))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::UtilityFlags` (r:1 w:0)
	/// Proof: `Nft::UtilityFlags` (`max_values`: None, `max_size`: Some(15), added: 2490, mode: `MaxEncodedLen`)
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Nft::TokenLocks` (r:1 w:0)
	/// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	/// Storage: `TokenApprovals::ERC721Approvals` (r:0 w:1)
	/// Proof: `TokenApprovals::ERC721Approvals` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	fn transfer() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `298`
		//  Estimated: `3498`
		// Minimum execution time: 62_510_000 picoseconds.
		Weight::from_parts(63_030_000, 0)
			.saturating_add(Weight::from_parts(0, 3498))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Nft::TokenLocks` (r:1 w:0)
	/// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	/// Storage: `Nft::UtilityFlags` (r:1 w:0)
	/// Proof: `Nft::UtilityFlags` (`max_values`: None, `max_size`: Some(15), added: 2490, mode: `MaxEncodedLen`)
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `TokenApprovals::ERC721Approvals` (r:0 w:1)
	/// Proof: `TokenApprovals::ERC721Approvals` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	fn burn() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `298`
		//  Estimated: `3498`
		// Minimum execution time: 59_565_000 picoseconds.
		Weight::from_parts(60_558_000, 0)
			.saturating_add(Weight::from_parts(0, 3498))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:0)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Nft::UtilityFlags` (r:0 w:1)
	/// Proof: `Nft::UtilityFlags` (`max_values`: None, `max_size`: Some(15), added: 2490, mode: `MaxEncodedLen`)
	fn set_utility_flags() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `298`
		//  Estimated: `3464`
		// Minimum execution time: 49_052_000 picoseconds.
		Weight::from_parts(49_689_000, 0)
			.saturating_add(Weight::from_parts(0, 3464))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
