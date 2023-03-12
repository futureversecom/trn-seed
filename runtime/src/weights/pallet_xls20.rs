
//! Autogenerated weights for `pallet_xls20`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-03-09, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `fedora`, CPU: `13th Gen Intel(R) Core(TM) i7-13700K`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-xls20
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./output

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_xls20`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_xls20::WeightInfo for WeightInfo<T> {
	// Storage: Xls20 Relayer (r:0 w:1)
	fn set_relayer() -> Weight {
		(8_084_000 as Weight)
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Xls20 Xls20MintFee (r:0 w:1)
	fn set_xls20_fee() -> Weight {
		(8_003_000 as Weight)
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn enable_xls20_compatibility() -> Weight {
		(11_151_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: Xls20 Xls20TokenMap (r:1 w:0)
	// Storage: Xls20 Xls20MintFee (r:1 w:0)
	fn re_request_xls20_mint() -> Weight {
		(15_277_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
	}
	// Storage: Xls20 Relayer (r:1 w:0)
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: Xls20 Xls20TokenMap (r:1 w:1)
	fn fulfill_xls20_mint() -> Weight {
		(15_848_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
}
