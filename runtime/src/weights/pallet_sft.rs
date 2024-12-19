
//! Autogenerated weights for `pallet_sft`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-12-19, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
// --pallet=pallet-sft
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_sft.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_sft`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_sft::WeightInfo for WeightInfo<T> {
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
	/// Storage: `Sft::SftCollectionInfo` (r:0 w:1)
	/// Proof: `Sft::SftCollectionInfo` (`max_values`: None, `max_size`: Some(484), added: 2959, mode: `MaxEncodedLen`)
	fn create_collection() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `322`
		//  Estimated: `3787`
		// Minimum execution time: 92_919_000 picoseconds.
		Weight::from_parts(94_567_000, 0)
			.saturating_add(Weight::from_parts(0, 3787))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `Sft::SftCollectionInfo` (r:1 w:1)
	/// Proof: `Sft::SftCollectionInfo` (`max_values`: None, `max_size`: Some(484), added: 2959, mode: `MaxEncodedLen`)
	/// Storage: `EVMChainId::ChainId` (r:1 w:0)
	/// Proof: `EVMChainId::ChainId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Nfi::NfiEnabled` (r:1 w:0)
	/// Proof: `Nfi::NfiEnabled` (`max_values`: None, `max_size`: Some(529), added: 3004, mode: `MaxEncodedLen`)
	/// Storage: `Sft::TokenInfo` (r:0 w:1)
	/// Proof: `Sft::TokenInfo` (`max_values`: None, `max_size`: Some(52000104), added: 52002579, mode: `MaxEncodedLen`)
	fn create_token() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `390`
		//  Estimated: `3994`
		// Minimum execution time: 65_563_000 picoseconds.
		Weight::from_parts(67_555_000, 0)
			.saturating_add(Weight::from_parts(0, 3994))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Sft::SftCollectionInfo` (r:1 w:0)
	/// Proof: `Sft::SftCollectionInfo` (`max_values`: None, `max_size`: Some(484), added: 2959, mode: `MaxEncodedLen`)
	/// Storage: `Sft::PublicMintInfo` (r:1 w:1)
	/// Proof: `Sft::PublicMintInfo` (`max_values`: None, `max_size`: Some(38), added: 2513, mode: `MaxEncodedLen`)
	fn toggle_public_mint() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `207`
		//  Estimated: `3949`
		// Minimum execution time: 45_539_000 picoseconds.
		Weight::from_parts(46_615_000, 0)
			.saturating_add(Weight::from_parts(0, 3949))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sft::SftCollectionInfo` (r:1 w:0)
	/// Proof: `Sft::SftCollectionInfo` (`max_values`: None, `max_size`: Some(484), added: 2959, mode: `MaxEncodedLen`)
	/// Storage: `Sft::PublicMintInfo` (r:1 w:1)
	/// Proof: `Sft::PublicMintInfo` (`max_values`: None, `max_size`: Some(38), added: 2513, mode: `MaxEncodedLen`)
	fn set_mint_fee() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `207`
		//  Estimated: `3949`
		// Minimum execution time: 45_861_000 picoseconds.
		Weight::from_parts(46_925_000, 0)
			.saturating_add(Weight::from_parts(0, 3949))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sft::SftCollectionInfo` (r:1 w:0)
	/// Proof: `Sft::SftCollectionInfo` (`max_values`: None, `max_size`: Some(484), added: 2959, mode: `MaxEncodedLen`)
	/// Storage: `Sft::UtilityFlags` (r:1 w:0)
	/// Proof: `Sft::UtilityFlags` (`max_values`: None, `max_size`: Some(15), added: 2490, mode: `MaxEncodedLen`)
	/// Storage: `Sft::PublicMintInfo` (r:1 w:0)
	/// Proof: `Sft::PublicMintInfo` (`max_values`: None, `max_size`: Some(38), added: 2513, mode: `MaxEncodedLen`)
	/// Storage: `Sft::TokenInfo` (r:1 w:1)
	/// Proof: `Sft::TokenInfo` (`max_values`: None, `max_size`: Some(52000104), added: 52002579, mode: `MaxEncodedLen`)
	fn mint() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `240`
		//  Estimated: `52003569`
		// Minimum execution time: 61_451_000 picoseconds.
		Weight::from_parts(62_188_000, 0)
			.saturating_add(Weight::from_parts(0, 52003569))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sft::UtilityFlags` (r:1 w:0)
	/// Proof: `Sft::UtilityFlags` (`max_values`: None, `max_size`: Some(15), added: 2490, mode: `MaxEncodedLen`)
	/// Storage: `Sft::TokenInfo` (r:50 w:50)
	/// Proof: `Sft::TokenInfo` (`max_values`: None, `max_size`: Some(52000104), added: 52002579, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 50]`.
	fn transfer(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `132 + p * (104 ±0)`
		//  Estimated: `3480 + p * (52002579 ±0)`
		// Minimum execution time: 51_510_000 picoseconds.
		Weight::from_parts(43_512_508, 0)
			.saturating_add(Weight::from_parts(0, 3480))
			// Standard Error: 20_449
			.saturating_add(Weight::from_parts(10_204_969, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(p.into())))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(p.into())))
			.saturating_add(Weight::from_parts(0, 52002579).saturating_mul(p.into()))
	}
	/// Storage: `Sft::UtilityFlags` (r:1 w:0)
	/// Proof: `Sft::UtilityFlags` (`max_values`: None, `max_size`: Some(15), added: 2490, mode: `MaxEncodedLen`)
	/// Storage: `Sft::TokenInfo` (r:1 w:1)
	/// Proof: `Sft::TokenInfo` (`max_values`: None, `max_size`: Some(52000104), added: 52002579, mode: `MaxEncodedLen`)
	fn burn() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `235`
		//  Estimated: `52003569`
		// Minimum execution time: 54_573_000 picoseconds.
		Weight::from_parts(55_597_000, 0)
			.saturating_add(Weight::from_parts(0, 52003569))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sft::SftCollectionInfo` (r:1 w:1)
	/// Proof: `Sft::SftCollectionInfo` (`max_values`: None, `max_size`: Some(484), added: 2959, mode: `MaxEncodedLen`)
	fn set_owner() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `173`
		//  Estimated: `3949`
		// Minimum execution time: 44_064_000 picoseconds.
		Weight::from_parts(45_016_000, 0)
			.saturating_add(Weight::from_parts(0, 3949))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sft::SftCollectionInfo` (r:1 w:0)
	/// Proof: `Sft::SftCollectionInfo` (`max_values`: None, `max_size`: Some(484), added: 2959, mode: `MaxEncodedLen`)
	/// Storage: `Sft::TokenInfo` (r:1 w:1)
	/// Proof: `Sft::TokenInfo` (`max_values`: None, `max_size`: Some(52000104), added: 52002579, mode: `MaxEncodedLen`)
	fn set_max_issuance() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `240`
		//  Estimated: `52003569`
		// Minimum execution time: 47_422_000 picoseconds.
		Weight::from_parts(48_442_000, 0)
			.saturating_add(Weight::from_parts(0, 52003569))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sft::SftCollectionInfo` (r:1 w:1)
	/// Proof: `Sft::SftCollectionInfo` (`max_values`: None, `max_size`: Some(484), added: 2959, mode: `MaxEncodedLen`)
	fn set_base_uri() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `173`
		//  Estimated: `3949`
		// Minimum execution time: 41_217_000 picoseconds.
		Weight::from_parts(41_977_000, 0)
			.saturating_add(Weight::from_parts(0, 3949))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sft::SftCollectionInfo` (r:1 w:1)
	/// Proof: `Sft::SftCollectionInfo` (`max_values`: None, `max_size`: Some(484), added: 2959, mode: `MaxEncodedLen`)
	fn set_name() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `173`
		//  Estimated: `3949`
		// Minimum execution time: 41_881_000 picoseconds.
		Weight::from_parts(42_264_000, 0)
			.saturating_add(Weight::from_parts(0, 3949))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sft::SftCollectionInfo` (r:1 w:1)
	/// Proof: `Sft::SftCollectionInfo` (`max_values`: None, `max_size`: Some(484), added: 2959, mode: `MaxEncodedLen`)
	fn set_royalties_schedule() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `173`
		//  Estimated: `3949`
		// Minimum execution time: 41_580_000 picoseconds.
		Weight::from_parts(42_583_000, 0)
			.saturating_add(Weight::from_parts(0, 3949))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sft::SftCollectionInfo` (r:1 w:0)
	/// Proof: `Sft::SftCollectionInfo` (`max_values`: None, `max_size`: Some(484), added: 2959, mode: `MaxEncodedLen`)
	/// Storage: `Sft::UtilityFlags` (r:0 w:1)
	/// Proof: `Sft::UtilityFlags` (`max_values`: None, `max_size`: Some(15), added: 2490, mode: `MaxEncodedLen`)
	fn set_utility_flags() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `173`
		//  Estimated: `3949`
		// Minimum execution time: 46_138_000 picoseconds.
		Weight::from_parts(47_054_000, 0)
			.saturating_add(Weight::from_parts(0, 3949))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sft::SftCollectionInfo` (r:1 w:0)
	/// Proof: `Sft::SftCollectionInfo` (`max_values`: None, `max_size`: Some(484), added: 2959, mode: `MaxEncodedLen`)
	/// Storage: `Sft::TokenInfo` (r:1 w:1)
	/// Proof: `Sft::TokenInfo` (`max_values`: None, `max_size`: Some(52000104), added: 52002579, mode: `MaxEncodedLen`)
	fn set_token_name() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `294`
		//  Estimated: `52003569`
		// Minimum execution time: 50_168_000 picoseconds.
		Weight::from_parts(51_088_000, 0)
			.saturating_add(Weight::from_parts(0, 52003569))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
