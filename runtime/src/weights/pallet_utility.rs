
//! Autogenerated weights for `pallet_utility`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-04-07, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `fedora`, CPU: `13th Gen Intel(R) Core(TM) i7-13700K`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-utility
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

/// Weight functions for `pallet_utility`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_utility::WeightInfo for WeightInfo<T> {
	/// The range of component `c` is `[0, 1000]`.
	fn batch(c: u32, ) -> Weight {
		Weight::from_ref_time(8_421_000 as u64)
			// Standard Error: 629
			.saturating_add(Weight::from_ref_time(2_945_722 as u64).saturating_mul(c as u64))
	}
	fn as_derivative() -> Weight {
		Weight::from_ref_time(4_344_000 as u64)
	}
	/// The range of component `c` is `[0, 1000]`.
	fn batch_all(c: u32, ) -> Weight {
		Weight::from_ref_time(8_454_000 as u64)
			// Standard Error: 896
			.saturating_add(Weight::from_ref_time(3_081_642 as u64).saturating_mul(c as u64))
	}
	fn dispatch_as() -> Weight {
		Weight::from_ref_time(9_806_000 as u64)
	}
	/// The range of component `c` is `[0, 1000]`.
	fn force_batch(c: u32, ) -> Weight {
		Weight::from_ref_time(8_427_000 as u64)
			// Standard Error: 763
			.saturating_add(Weight::from_ref_time(2_949_301 as u64).saturating_mul(c as u64))
	}
}
