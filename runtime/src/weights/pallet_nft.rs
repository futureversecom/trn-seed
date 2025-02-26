
//! Autogenerated weights for `pallet_nft`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2025-02-25, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `Johns-Macbook-Pro.local`, CPU: `<UNKNOWN>`
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
		//  Measured:  `331`
		//  Estimated: `3464`
		// Minimum execution time: 15_000_000 picoseconds.
		Weight::from_parts(16_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3464))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	fn set_owner() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `331`
		//  Estimated: `3464`
		// Minimum execution time: 17_000_000 picoseconds.
		Weight::from_parts(17_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3464))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	fn set_max_issuance() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `331`
		//  Estimated: `3464`
		// Minimum execution time: 16_000_000 picoseconds.
		Weight::from_parts(17_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3464))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	fn set_base_uri() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `331`
		//  Estimated: `3464`
		// Minimum execution time: 17_000_000 picoseconds.
		Weight::from_parts(18_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3464))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	fn set_name() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `331`
		//  Estimated: `3464`
		// Minimum execution time: 17_000_000 picoseconds.
		Weight::from_parts(17_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3464))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	fn set_royalties_schedule() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `331`
		//  Estimated: `3464`
		// Minimum execution time: 17_000_000 picoseconds.
		Weight::from_parts(18_000_000, 0)
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
		//  Measured:  `282`
		//  Estimated: `3747`
		// Minimum execution time: 37_000_000 picoseconds.
		Weight::from_parts(38_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3747))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:0)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Nft::PublicMintInfo` (r:1 w:1)
	/// Proof: `Nft::PublicMintInfo` (`max_values`: None, `max_size`: Some(34), added: 2509, mode: `MaxEncodedLen`)
	fn toggle_public_mint() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `331`
		//  Estimated: `3499`
		// Minimum execution time: 17_000_000 picoseconds.
		Weight::from_parts(18_000_000, 0)
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
		//  Measured:  `331`
		//  Estimated: `3499`
		// Minimum execution time: 18_000_000 picoseconds.
		Weight::from_parts(19_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3499))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Nft::PublicMintInfo` (r:1 w:0)
	/// Proof: `Nft::PublicMintInfo` (`max_values`: None, `max_size`: Some(34), added: 2509, mode: `MaxEncodedLen`)
	/// Storage: `Nft::UtilityFlags` (r:1 w:0)
	/// Proof: `Nft::UtilityFlags` (`max_values`: None, `max_size`: Some(15), added: 2490, mode: `MaxEncodedLen`)
	/// Storage: `EVMChainId::ChainId` (r:1 w:0)
	/// Proof: `EVMChainId::ChainId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Nfi::NfiEnabled` (r:1 w:0)
	/// Proof: `Nfi::NfiEnabled` (`max_values`: None, `max_size`: Some(529), added: 3004, mode: `MaxEncodedLen`)
	fn mint() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `377`
		//  Estimated: `3994`
		// Minimum execution time: 28_000_000 picoseconds.
		Weight::from_parts(29_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3994))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::UtilityFlags` (r:1 w:0)
	/// Proof: `Nft::UtilityFlags` (`max_values`: None, `max_size`: Some(15), added: 2490, mode: `MaxEncodedLen`)
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Nft::TokenLocks` (r:500 w:0)
	/// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	/// Storage: `Nft::TokenUtilityFlags` (r:500 w:0)
	/// Proof: `Nft::TokenUtilityFlags` (`max_values`: None, `max_size`: Some(19), added: 2494, mode: `MaxEncodedLen`)
	/// Storage: `TokenApprovals::ERC721Approvals` (r:0 w:500)
	/// Proof: `TokenApprovals::ERC721Approvals` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 500]`.
	fn transfer(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `332 + p * (4 ±0)`
		//  Estimated: `3480 + p * (2508 ±0)`
		// Minimum execution time: 25_000_000 picoseconds.
		Weight::from_parts(26_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3480))
			// Standard Error: 4_985
			.saturating_add(Weight::from_parts(4_350_701, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(p.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(p.into())))
			.saturating_add(Weight::from_parts(0, 2508).saturating_mul(p.into()))
	}
	/// Storage: `Nft::TokenLocks` (r:1 w:0)
	/// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	/// Storage: `Nft::UtilityFlags` (r:1 w:0)
	/// Proof: `Nft::UtilityFlags` (`max_values`: None, `max_size`: Some(15), added: 2490, mode: `MaxEncodedLen`)
	/// Storage: `EVMChainId::ChainId` (r:1 w:0)
	/// Proof: `EVMChainId::ChainId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Nfi::NfiData` (r:1 w:0)
	/// Proof: `Nfi::NfiData` (`max_values`: None, `max_size`: Some(1166), added: 3641, mode: `MaxEncodedLen`)
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Nft::TokenUtilityFlags` (r:1 w:0)
	/// Proof: `Nft::TokenUtilityFlags` (`max_values`: None, `max_size`: Some(19), added: 2494, mode: `MaxEncodedLen`)
	/// Storage: `TokenApprovals::ERC721Approvals` (r:0 w:1)
	/// Proof: `TokenApprovals::ERC721Approvals` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	fn burn() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `377`
		//  Estimated: `4631`
		// Minimum execution time: 29_000_000 picoseconds.
		Weight::from_parts(30_000_000, 0)
			.saturating_add(Weight::from_parts(0, 4631))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:0)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Nft::UtilityFlags` (r:0 w:1)
	/// Proof: `Nft::UtilityFlags` (`max_values`: None, `max_size`: Some(15), added: 2490, mode: `MaxEncodedLen`)
	fn set_utility_flags() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `331`
		//  Estimated: `3464`
		// Minimum execution time: 18_000_000 picoseconds.
		Weight::from_parts(18_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3464))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:0)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Nft::TokenUtilityFlags` (r:1 w:1)
	/// Proof: `Nft::TokenUtilityFlags` (`max_values`: None, `max_size`: Some(19), added: 2494, mode: `MaxEncodedLen`)
	fn set_token_transferable_flag() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `331`
		//  Estimated: `3484`
		// Minimum execution time: 21_000_000 picoseconds.
		Weight::from_parts(22_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3484))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:0)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Nft::PendingIssuances` (r:1 w:1)
	/// Proof: `Nft::PendingIssuances` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	fn issue_soulbound() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `331`
		//  Estimated: `3464`
		// Minimum execution time: 19_000_000 picoseconds.
		Weight::from_parts(20_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3464))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::PendingIssuances` (r:1 w:1)
	/// Proof: `Nft::PendingIssuances` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Nft::UtilityFlags` (r:1 w:0)
	/// Proof: `Nft::UtilityFlags` (`max_values`: None, `max_size`: Some(15), added: 2490, mode: `MaxEncodedLen`)
	/// Storage: `EVMChainId::ChainId` (r:1 w:0)
	/// Proof: `EVMChainId::ChainId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Nfi::NfiEnabled` (r:1 w:0)
	/// Proof: `Nfi::NfiEnabled` (`max_values`: None, `max_size`: Some(529), added: 3004, mode: `MaxEncodedLen`)
	/// Storage: `Nft::TokenUtilityFlags` (r:1 w:1)
	/// Proof: `Nft::TokenUtilityFlags` (`max_values`: None, `max_size`: Some(19), added: 2494, mode: `MaxEncodedLen`)
	fn accept_soulbound_issuance() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `447`
		//  Estimated: `3994`
		// Minimum execution time: 33_000_000 picoseconds.
		Weight::from_parts(35_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3994))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(3))
	}
}
