
//! Autogenerated weights for `pallet_xls20`
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
// --pallet=pallet_xls20
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_xls20.rs

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
		Weight::from_ref_time(38_720_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Xls20 Xls20MintFee (r:0 w:1)
	fn set_xls20_fee() -> Weight {
		Weight::from_ref_time(37_570_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn enable_xls20_compatibility() -> Weight {
		Weight::from_ref_time(55_656_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: Xls20 Xls20TokenMap (r:1 w:0)
	// Storage: Xls20 Xls20MintFee (r:1 w:0)
	fn re_request_xls20_mint() -> Weight {
		Weight::from_ref_time(70_778_000 as u64)
			.saturating_add(T::DbWeight::get().reads(3 as u64))
	}
	// Storage: Xls20 Relayer (r:1 w:0)
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: Xls20 Xls20TokenMap (r:1 w:1)
	fn fulfill_xls20_mint() -> Weight {
		Weight::from_ref_time(73_595_000 as u64)
			.saturating_add(T::DbWeight::get().reads(3 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
}
