
//! Autogenerated weights for `pallet_echo`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-06-02, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `Xiankuns-MBP-2`, CPU: `<UNKNOWN>`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_echo
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./output/pallet_echo.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_echo`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_echo::WeightInfo for WeightInfo<T> {
	/// Storage: Echo NextSessionId (r:1 w:1)
	/// Proof: Echo NextSessionId (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	/// Storage: EthBridge NextEventProofId (r:1 w:1)
	/// Proof Skipped: EthBridge NextEventProofId (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: EthBridge NotaryKeys (r:1 w:0)
	/// Proof Skipped: EthBridge NotaryKeys (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: EthBridge NotarySetId (r:1 w:0)
	/// Proof Skipped: EthBridge NotarySetId (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: EthBridge BridgePaused (r:1 w:0)
	/// Proof Skipped: EthBridge BridgePaused (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: System Digest (r:1 w:1)
	/// Proof Skipped: System Digest (max_values: Some(1), max_size: None, mode: Measured)
	fn ping() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `234`
		//  Estimated: `10088`
		// Minimum execution time: 28_000_000 picoseconds.
		Weight::from_parts(29_000_000, 0)
			.saturating_add(Weight::from_parts(0, 10088))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(3))
	}
}
