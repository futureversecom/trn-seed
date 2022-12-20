
//! Autogenerated weights for `pallet_fee_control`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-12-14, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `justin-System-Product-Name`, CPU: `12th Gen Intel(R) Core(TM) i9-12900K`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain
// dev
// --steps
// 50
// --repeat
// 20
// --pallet
// pallet-fee-control
// --extrinsic=*
// --execution
// wasm
// --wasm-execution
// compiled
// --heap-pages
// 4096
// --output
// ./output

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_fee_control`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_fee_control::WeightInfo for WeightInfo<T> {
	// Storage: FeeControl EvmBaseFeePerGas (r:0 w:1)
	fn set_evm_base_fee() -> Weight {
		(2_533_000 as Weight)
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: FeeControl ExtrinsicWeightToFee (r:0 w:1)
	fn set_extrinsic_weight_to_fee_factor() -> Weight {
		(2_490_000 as Weight)
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
}
