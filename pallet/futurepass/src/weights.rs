//! Autogenerated weights for pallet_futurepass
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
// --template
// ./scripts/pallet_template.hbs
// --output
// ./output

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_futurepass.
pub trait WeightInfo {
	fn create() -> Weight;
	fn register_delegate() -> Weight;
	fn unregister_delegate() -> Weight;
	fn transfer_futurepass() -> Weight;
	fn proxy_extrinsic() -> Weight;
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: Futurepass Holders (r:1 w:1)
	// Storage: Futurepass NextFuturepassId (r:1 w:1)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	fn create() -> Weight {
		(78_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(4 as Weight))
			.saturating_add(RocksDbWeight::get().writes(4 as Weight))
	}
	// Storage: Futurepass Holders (r:1 w:0)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	fn register_delegate() -> Weight {
		(70_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(4 as Weight))
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
	}
	// Storage: Futurepass Holders (r:1 w:0)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	fn unregister_delegate() -> Weight {
		(69_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(4 as Weight))
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
	}
	// Storage: Futurepass Holders (r:2 w:2)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	fn transfer_futurepass() -> Weight {
		(109_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(5 as Weight))
			.saturating_add(RocksDbWeight::get().writes(5 as Weight))
	}
	// Storage: Proxy Proxies (r:1 w:0)
	fn proxy_extrinsic() -> Weight {
		(29_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
	}
}
