
//! Autogenerated weights for `pallet_xls20`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-10-28, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `Jasons-Ubuntu`, CPU: `AMD Ryzen 9 7950X 16-Core Processor`
//! EXECUTION: ``, WASM-EXECUTION: `Compiled`, CHAIN: `Some("dev")`, DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-xls20
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_xls20.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_xls20`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_xls20::WeightInfo for WeightInfo<T> {
	/// Storage: `Xls20::Relayer` (r:0 w:1)
	/// Proof: `Xls20::Relayer` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_relayer() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 5_711_000 picoseconds.
		Weight::from_parts(6_002_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Xls20::Xls20MintFee` (r:0 w:1)
	/// Proof: `Xls20::Xls20MintFee` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	fn set_xls20_fee() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 5_761_000 picoseconds.
		Weight::from_parts(6_222_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	fn enable_xls20_compatibility() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `306`
		//  Estimated: `3464`
		// Minimum execution time: 11_452_000 picoseconds.
		Weight::from_parts(11_752_000, 0)
			.saturating_add(Weight::from_parts(0, 3464))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Nft::CollectionInfo` (r:1 w:0)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Xls20::Xls20TokenMap` (r:1 w:0)
	/// Proof: `Xls20::Xls20TokenMap` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// Storage: `Xls20::Xls20MintFee` (r:1 w:0)
	/// Proof: `Xls20::Xls20MintFee` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	fn re_request_xls20_mint() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `373`
		//  Estimated: `3521`
		// Minimum execution time: 16_221_000 picoseconds.
		Weight::from_parts(16_481_000, 0)
			.saturating_add(Weight::from_parts(0, 3521))
			.saturating_add(T::DbWeight::get().reads(3))
	}
	/// Storage: `Xls20::Relayer` (r:1 w:0)
	/// Proof: `Xls20::Relayer` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	/// Storage: `Nft::CollectionInfo` (r:1 w:0)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	/// Storage: `Xls20::Xls20TokenMap` (r:1 w:1)
	/// Proof: `Xls20::Xls20TokenMap` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	fn fulfill_xls20_mint() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `379`
		//  Estimated: `3521`
		// Minimum execution time: 17_513_000 picoseconds.
		Weight::from_parts(18_015_000, 0)
			.saturating_add(Weight::from_parts(0, 3521))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Xls20::CollectionMapping` (r:0 w:1)
	/// Proof: `Xls20::CollectionMapping` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	/// The range of component `i` is `[0, 256]`.
	fn set_collection_mappings(i: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 6_422_000 picoseconds.
		Weight::from_parts(7_132_321, 0)
			.saturating_add(Weight::from_parts(0, 0))
			// Standard Error: 1_009
			.saturating_add(Weight::from_parts(1_442_241, 0).saturating_mul(i.into()))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Xls20::CollectionMapping` (r:1 w:0)
	/// Proof: `Xls20::CollectionMapping` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	/// Storage: `Xls20::Xls20TokenMap` (r:1 w:1)
	/// Proof: `Xls20::Xls20TokenMap` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// Storage: `Nft::CollectionInfo` (r:1 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	fn deposit_token_mint() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `371`
		//  Estimated: `3521`
		// Minimum execution time: 16_100_000 picoseconds.
		Weight::from_parts(16_792_000, 0)
			.saturating_add(Weight::from_parts(0, 3521))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Xls20::CollectionMapping` (r:1 w:1)
	/// Proof: `Xls20::CollectionMapping` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
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
	/// Storage: `Xls20::Xls20TokenMap` (r:0 w:1)
	/// Proof: `Xls20::Xls20TokenMap` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// Storage: `Nft::CollectionInfo` (r:0 w:1)
	/// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	fn deposit_token_create_collection() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `286`
		//  Estimated: `3751`
		// Minimum execution time: 33_053_000 picoseconds.
		Weight::from_parts(33_544_000, 0)
			.saturating_add(Weight::from_parts(0, 3751))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(7))
	}
}
