
//! Autogenerated weights for `pallet_futurepass`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-04-24, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `zeeshans-mbp.lan`, CPU: `<UNKNOWN>`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-futurepass
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./output/pallet-futurepass.rs

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
		(78_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	// Storage: Futurepass Holders (r:1 w:0)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	fn register_delegate_with_signature(p: u32, ) -> Weight {
		(72_000_000 as Weight)
			.saturating_add((76_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: Futurepass Holders (r:1 w:0)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	fn unregister_delegate(p: u32, ) -> Weight {
		(71_000_000 as Weight)
			.saturating_add((76_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: Futurepass Holders (r:2 w:2)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	fn transfer_futurepass() -> Weight {
		(107_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	// Storage: Proxy Proxies (r:1 w:0)
	fn proxy_extrinsic(p: u32, ) -> Weight {
		(29_000_000 as Weight)
			// Standard Error: 2_000
			.saturating_add((76_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
	}
}
