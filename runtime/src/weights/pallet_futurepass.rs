
//! Autogenerated weights for `pallet_futurepass`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-06-05, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `fedora`, CPU: `13th Gen Intel(R) Core(TM) i7-13700K`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_futurepass
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./output/pallet_futurepass.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_futurepass`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_futurepass::WeightInfo for WeightInfo<T> {
	// Storage: Futurepass Holders (r:1 w:1)
	// Storage: Futurepass NextFuturepassId (r:1 w:1)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	fn create() -> Weight {
		Weight::from_ref_time(45_974_000 as u64)
			.saturating_add(T::DbWeight::get().reads(4 as u64))
			.saturating_add(T::DbWeight::get().writes(4 as u64))
	}
	// Storage: Futurepass Holders (r:1 w:0)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	fn register_delegate() -> Weight {
		Weight::from_ref_time(42_738_000 as u64)
			.saturating_add(T::DbWeight::get().reads(4 as u64))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
	}
	// Storage: Futurepass Holders (r:2 w:0)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	fn unregister_delegate() -> Weight {
		Weight::from_ref_time(46_064_000 as u64)
			.saturating_add(T::DbWeight::get().reads(5 as u64))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
	}
	// Storage: Futurepass Holders (r:2 w:2)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	fn transfer_futurepass() -> Weight {
		Weight::from_ref_time(65_993_000 as u64)
			.saturating_add(T::DbWeight::get().reads(5 as u64))
			.saturating_add(T::DbWeight::get().writes(5 as u64))
	}
	// Storage: Proxy Proxies (r:1 w:0)
	fn proxy_extrinsic() -> Weight {
		Weight::from_ref_time(19_180_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
	}
}
