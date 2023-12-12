
//! Autogenerated weights for `pallet_xrpl_transaction`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-12-12, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `zeeshans-mbp.lan`, CPU: `<UNKNOWN>`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-xrpl-transaction
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_xrpl_transaction.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_xrpl_transaction`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_xrpl_transaction::WeightInfo for WeightInfo<T> {
	// Storage: MaintenanceMode BlockedCalls (r:1 w:0)
	// Storage: MaintenanceMode BlockedPallets (r:1 w:0)
	fn submit_encoded_xumm_transaction() -> Weight {
		Weight::from_ref_time(90_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
	}
}
