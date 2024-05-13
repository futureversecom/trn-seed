
//! Autogenerated weights for `pallet_xrpl`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-05-13, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `Surangas-MacBook-Pro.local`, CPU: `<UNKNOWN>`
//! EXECUTION: ``, WASM-EXECUTION: `Compiled`, CHAIN: `Some("dev")`, DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-xrpl
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_xrpl.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_xrpl`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_xrpl::WeightInfo for WeightInfo<T> {
	/// Storage: `MaintenanceMode::BlockedCalls` (r:1 w:0)
	/// Proof: `MaintenanceMode::BlockedCalls` (`max_values`: None, `max_size`: Some(111), added: 2586, mode: `MaxEncodedLen`)
	/// Storage: `MaintenanceMode::BlockedPallets` (r:1 w:0)
	/// Proof: `MaintenanceMode::BlockedPallets` (`max_values`: None, `max_size`: Some(60), added: 2535, mode: `MaxEncodedLen`)
	fn transact() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `76`
		//  Estimated: `3576`
		// Minimum execution time: 83_000_000 picoseconds.
		Weight::from_parts(85_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3576))
			.saturating_add(T::DbWeight::get().reads(2))
	}
}
