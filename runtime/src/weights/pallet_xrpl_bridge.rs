
//! Autogenerated weights for `pallet_xrpl_bridge`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-09-19, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `Surangas-MacBook-Pro.local`, CPU: `<UNKNOWN>`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-xrpl-bridge
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_xrpl_bridge.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_xrpl_bridge`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_xrpl_bridge::WeightInfo for WeightInfo<T> {
	// Storage: XRPLBridge Relayer (r:1 w:0)
	// Storage: XRPLBridge HighestSettledLedgerIndex (r:1 w:0)
	// Storage: XRPLBridge SubmissionWindowWidth (r:1 w:0)
	// Storage: XRPLBridge ProcessXRPTransactionDetails (r:1 w:1)
	// Storage: XRPLBridge ProcessXRPTransaction (r:1 w:1)
	fn submit_transaction() -> Weight {
		Weight::from_ref_time(24_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(5 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: XRPLBridge ChallengeXRPTransactionList (r:0 w:1)
	fn submit_challenge() -> Weight {
		Weight::from_ref_time(6_000_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: XRPLBridge DoorTxFee (r:1 w:0)
	// Storage: XRPLBridge DoorAddress (r:1 w:0)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:1 w:1)
	// Storage: XRPLBridge DoorTicketSequence (r:1 w:1)
	// Storage: XRPLBridge DoorTicketSequenceParams (r:1 w:1)
	// Storage: XRPLBridge DoorTicketSequenceParamsNext (r:1 w:1)
	// Storage: EthBridge NextEventProofId (r:1 w:1)
	// Storage: EthBridge BridgePaused (r:1 w:0)
	// Storage: System Digest (r:1 w:1)
	// Storage: XRPLBridge TicketSequenceThresholdReachedEmitted (r:0 w:1)
	fn withdraw_xrp() -> Weight {
		Weight::from_ref_time(51_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(10 as u64))
			.saturating_add(T::DbWeight::get().writes(8 as u64))
	}
	// Storage: XRPLBridge Relayer (r:0 w:1)
	fn add_relayer() -> Weight {
		Weight::from_ref_time(13_000_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: XRPLBridge Relayer (r:1 w:1)
	fn remove_relayer() -> Weight {
		Weight::from_ref_time(17_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: XRPLBridge DoorTxFee (r:0 w:1)
	fn set_door_tx_fee() -> Weight {
		Weight::from_ref_time(4_000_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: XRPLBridge DoorAddress (r:0 w:1)
	fn set_door_address() -> Weight {
		Weight::from_ref_time(13_000_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: XRPLBridge Relayer (r:1 w:0)
	// Storage: XRPLBridge DoorTicketSequence (r:1 w:0)
	// Storage: XRPLBridge DoorTicketSequenceParams (r:1 w:0)
	// Storage: XRPLBridge DoorTicketSequenceParamsNext (r:0 w:1)
	fn set_ticket_sequence_next_allocation() -> Weight {
		Weight::from_ref_time(17_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(3 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: XRPLBridge DoorTicketSequence (r:1 w:1)
	// Storage: XRPLBridge DoorTicketSequenceParams (r:1 w:1)
	// Storage: XRPLBridge TicketSequenceThresholdReachedEmitted (r:0 w:1)
	fn set_ticket_sequence_current_allocation() -> Weight {
		Weight::from_ref_time(16_000_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
	}
	// Storage: XRPLBridge SubmissionWindowWidth (r:0 w:1)
	// Storage: XRPLBridge HighestSettledLedgerIndex (r:0 w:1)
	// Storage: XRPLBridge SettledXRPTransactionDetails (r:5 w:5)
	// Storage: XRPLBridge ProcessXRPTransactionDetails (r:0 w:5)
	/// The range of component `i` is `[0, 256]`.
	fn reset_settled_xrpl_tx_data(i: u32, ) -> Weight {
		Weight::from_ref_time(5_000_000 as u64)
			// Standard Error: 5_325
			.saturating_add(Weight::from_ref_time(4_185_009 as u64).saturating_mul(i as u64))
			.saturating_add(T::DbWeight::get().reads((1 as u64).saturating_mul(i as u64)))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
			.saturating_add(T::DbWeight::get().writes((2 as u64).saturating_mul(i as u64)))
	}
}
