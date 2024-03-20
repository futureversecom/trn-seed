
//! Autogenerated weights for `pallet_maintenance_mode`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-09-19, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-102-147`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_maintenance_mode
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_maintenance_mode.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_maintenance_mode`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_maintenance_mode::WeightInfo for WeightInfo<T> {
	// Storage: MaintenanceMode MaintenanceModeActive (r:0 w:1)
	fn enable_maintenance_mode() -> Weight {
		Weight::from_all(37_105_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: MaintenanceMode BlockedAccounts (r:0 w:1)
	fn block_account() -> Weight {
		Weight::from_all(39_698_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: MaintenanceMode BlockedEVMAddresses (r:0 w:1)
	fn block_evm_target() -> Weight {
		Weight::from_all(40_250_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: MaintenanceMode BlockedCalls (r:0 w:1)
	fn block_call() -> Weight {
		Weight::from_all(42_341_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: MaintenanceMode BlockedPallets (r:0 w:1)
	fn block_pallet() -> Weight {
		Weight::from_all(42_924_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
}
