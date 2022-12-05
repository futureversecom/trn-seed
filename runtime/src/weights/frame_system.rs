
//! Autogenerated weights for `frame_system`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-11-24, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-40-128`, CPU: `Intel(R) Xeon(R) CPU E5-2676 v3 @ 2.40GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=frame_system
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

/// Weight functions for `frame_system`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> frame_system::WeightInfo for WeightInfo<T> {
	/// The range of component `b` is `[0, 3932160]`.
	fn remark(b: u32, ) -> Weight {
		(10_159_000 as Weight)
			// Standard Error: 0
			.saturating_add((1_000 as Weight).saturating_mul(b as Weight))
	}
	/// The range of component `b` is `[0, 3932160]`.
	fn remark_with_event(b: u32, ) -> Weight {
		(43_110_000 as Weight)
			// Standard Error: 0
			.saturating_add((3_000 as Weight).saturating_mul(b as Weight))
	}
	// Storage: System Digest (r:1 w:1)
	// Storage: unknown [0x3a686561707061676573] (r:0 w:1)
	fn set_heap_pages() -> Weight {
		(16_774_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: Skipped Metadata (r:0 w:0)
	/// The range of component `i` is `[1, 1000]`.
	fn set_storage(i: u32, ) -> Weight {
		(0 as Weight)
			// Standard Error: 2_000
			.saturating_add((1_366_000 as Weight).saturating_mul(i as Weight))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(i as Weight)))
	}
	// Storage: Skipped Metadata (r:0 w:0)
	/// The range of component `i` is `[1, 1000]`.
	fn kill_storage(i: u32, ) -> Weight {
		(0 as Weight)
			// Standard Error: 2_000
			.saturating_add((1_093_000 as Weight).saturating_mul(i as Weight))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(i as Weight)))
	}
	// Storage: Skipped Metadata (r:0 w:0)
	/// The range of component `p` is `[1, 1000]`.
	fn kill_prefix(p: u32, ) -> Weight {
		(0 as Weight)
			// Standard Error: 3_000
			.saturating_add((2_043_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(p as Weight)))
	}
}
