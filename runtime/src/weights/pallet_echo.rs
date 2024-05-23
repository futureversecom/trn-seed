
//! Autogenerated weights for `pallet_echo`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-05-23, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ip-172-31-102-147`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: ``, WASM-EXECUTION: `Compiled`, CHAIN: `Some("dev")`, DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-echo
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_echo.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_echo`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_echo::WeightInfo for WeightInfo<T> {
	/// Storage: `Echo::NextSessionId` (r:1 w:1)
	/// Proof: `Echo::NextSessionId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `EthBridge::NextEventProofId` (r:1 w:1)
	/// Proof: `EthBridge::NextEventProofId` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `EthBridge::NotaryKeys` (r:1 w:0)
	/// Proof: `EthBridge::NotaryKeys` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `EthBridge::NotarySetId` (r:1 w:0)
	/// Proof: `EthBridge::NotarySetId` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `EthBridge::BridgePaused` (r:1 w:0)
	/// Proof: `EthBridge::BridgePaused` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `System::Digest` (r:1 w:1)
	/// Proof: `System::Digest` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn ping() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `334`
		//  Estimated: `1819`
		// Minimum execution time: 83_079_000 picoseconds.
		Weight::from_parts(84_954_000, 0)
			.saturating_add(Weight::from_parts(0, 1819))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(3))
	}
}
