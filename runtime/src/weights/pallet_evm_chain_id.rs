
//! Autogenerated weights for `pallet_evm_chain_id`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-07-25, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `Surangas-MacBook-Pro.local`, CPU: `<UNKNOWN>`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_evm_chain_id
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./output/pallet_evm_chain_id.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_evm_chain_id`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_evm_chain_id::WeightInfo for WeightInfo<T> {
	// Storage: EVMChainId ChainId (r:0 w:1)
	fn set_chain_id() -> Weight {
		Weight::from_ref_time(11_000_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
}
