
//! Autogenerated weights for `pallet_xrpl_bridge`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-05-14, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `Surangas-MacBook-Pro.local`, CPU: `<UNKNOWN>`
//! EXECUTION: ``, WASM-EXECUTION: `Compiled`, CHAIN: `Some("dev")`, DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-xrpl-bridge
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_xrpl_bridge.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_xrpl_bridge`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_xrpl_bridge::WeightInfo for WeightInfo<T> {
	/// Storage: `XRPLBridge::Relayer` (r:1 w:0)
	/// Proof: `XRPLBridge::Relayer` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::HighestSettledLedgerIndex` (r:1 w:0)
	/// Proof: `XRPLBridge::HighestSettledLedgerIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::SubmissionWindowWidth` (r:1 w:0)
	/// Proof: `XRPLBridge::SubmissionWindowWidth` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::ProcessXRPTransactionDetails` (r:1 w:1)
	/// Proof: `XRPLBridge::ProcessXRPTransactionDetails` (`max_values`: None, `max_size`: Some(233), added: 2708, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::ProcessXRPTransaction` (r:1 w:1)
	/// Proof: `XRPLBridge::ProcessXRPTransaction` (`max_values`: None, `max_size`: Some(64000016), added: 64002491, mode: `MaxEncodedLen`)
	fn submit_transaction() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `242`
		//  Estimated: `64003481`
		// Minimum execution time: 23_000_000 picoseconds.
		Weight::from_parts(24_000_000, 0)
			.saturating_add(Weight::from_parts(0, 64003481))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `XRPLBridge::ChallengeXRPTransactionList` (r:0 w:1)
	/// Proof: `XRPLBridge::ChallengeXRPTransactionList` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	fn submit_challenge() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 4_000_000 picoseconds.
		Weight::from_parts(5_000_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `XRPLBridge::PaymentDelay` (r:0 w:1)
	/// Proof: `XRPLBridge::PaymentDelay` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_payment_delay() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 6_000_000 picoseconds.
		Weight::from_parts(7_000_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `XRPLBridge::DoorTxFee` (r:1 w:0)
	/// Proof: `XRPLBridge::DoorTxFee` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::DoorAddress` (r:1 w:0)
	/// Proof: `XRPLBridge::DoorAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::DoorTicketSequence` (r:1 w:1)
	/// Proof: `XRPLBridge::DoorTicketSequence` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::DoorTicketSequenceParams` (r:1 w:1)
	/// Proof: `XRPLBridge::DoorTicketSequenceParams` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::DoorTicketSequenceParamsNext` (r:1 w:1)
	/// Proof: `XRPLBridge::DoorTicketSequenceParamsNext` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::PaymentDelay` (r:1 w:0)
	/// Proof: `XRPLBridge::PaymentDelay` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::SourceTag` (r:1 w:0)
	/// Proof: `XRPLBridge::SourceTag` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `EthBridge::NextEventProofId` (r:1 w:1)
	/// Proof: `EthBridge::NextEventProofId` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `EthBridge::BridgePaused` (r:1 w:0)
	/// Proof: `EthBridge::BridgePaused` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `System::Digest` (r:1 w:1)
	/// Proof: `System::Digest` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `XRPLBridge::TicketSequenceThresholdReachedEmitted` (r:0 w:1)
	/// Proof: `XRPLBridge::TicketSequenceThresholdReachedEmitted` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn withdraw_xrp() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1253`
		//  Estimated: `3627`
		// Minimum execution time: 70_000_000 picoseconds.
		Weight::from_parts(71_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3627))
			.saturating_add(T::DbWeight::get().reads(12))
			.saturating_add(T::DbWeight::get().writes(8))
	}
	/// Storage: `XRPLBridge::Relayer` (r:0 w:1)
	/// Proof: `XRPLBridge::Relayer` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	fn add_relayer() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 9_000_000 picoseconds.
		Weight::from_parts(9_000_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `XRPLBridge::Relayer` (r:1 w:1)
	/// Proof: `XRPLBridge::Relayer` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	fn remove_relayer() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `242`
		//  Estimated: `3502`
		// Minimum execution time: 14_000_000 picoseconds.
		Weight::from_parts(15_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3502))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `XRPLBridge::DoorTxFee` (r:0 w:1)
	/// Proof: `XRPLBridge::DoorTxFee` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	fn set_door_tx_fee() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 2_000_000 picoseconds.
		Weight::from_parts(3_000_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `XRPLBridge::SourceTag` (r:0 w:1)
	/// Proof: `XRPLBridge::SourceTag` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn set_xrp_source_tag() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 2_000_000 picoseconds.
		Weight::from_parts(3_000_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `XRPLBridge::DoorAddress` (r:0 w:1)
	/// Proof: `XRPLBridge::DoorAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_door_address() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 9_000_000 picoseconds.
		Weight::from_parts(9_000_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `XRPLBridge::Relayer` (r:1 w:0)
	/// Proof: `XRPLBridge::Relayer` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::DoorTicketSequence` (r:1 w:0)
	/// Proof: `XRPLBridge::DoorTicketSequence` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::DoorTicketSequenceParams` (r:1 w:0)
	/// Proof: `XRPLBridge::DoorTicketSequenceParams` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::DoorTicketSequenceParamsNext` (r:0 w:1)
	/// Proof: `XRPLBridge::DoorTicketSequenceParamsNext` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	fn set_ticket_sequence_next_allocation() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `242`
		//  Estimated: `3502`
		// Minimum execution time: 14_000_000 picoseconds.
		Weight::from_parts(15_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3502))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `XRPLBridge::DoorTicketSequence` (r:1 w:1)
	/// Proof: `XRPLBridge::DoorTicketSequence` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::DoorTicketSequenceParams` (r:1 w:1)
	/// Proof: `XRPLBridge::DoorTicketSequenceParams` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::TicketSequenceThresholdReachedEmitted` (r:0 w:1)
	/// Proof: `XRPLBridge::TicketSequenceThresholdReachedEmitted` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn set_ticket_sequence_current_allocation() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `207`
		//  Estimated: `1493`
		// Minimum execution time: 12_000_000 picoseconds.
		Weight::from_parts(12_000_000, 0)
			.saturating_add(Weight::from_parts(0, 1493))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `XRPLBridge::SettledXRPTransactionDetails` (r:256 w:256)
	/// Proof: `XRPLBridge::SettledXRPTransactionDetails` (`max_values`: None, `max_size`: Some(64000016), added: 64002491, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::SubmissionWindowWidth` (r:0 w:1)
	/// Proof: `XRPLBridge::SubmissionWindowWidth` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::HighestPrunedLedgerIndex` (r:0 w:1)
	/// Proof: `XRPLBridge::HighestPrunedLedgerIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::HighestSettledLedgerIndex` (r:0 w:1)
	/// Proof: `XRPLBridge::HighestSettledLedgerIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::ProcessXRPTransactionDetails` (r:0 w:256)
	/// Proof: `XRPLBridge::ProcessXRPTransactionDetails` (`max_values`: None, `max_size`: Some(233), added: 2708, mode: `MaxEncodedLen`)
	/// The range of component `i` is `[0, 256]`.
	fn reset_settled_xrpl_tx_data(i: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `180`
		//  Estimated: `1887623333 + i * (1669340 ±732_183)`
		// Minimum execution time: 5_000_000 picoseconds.
		Weight::from_parts(6_000_000, 0)
			.saturating_add(Weight::from_parts(0, 1887623333))
			// Standard Error: 3_081
			.saturating_add(Weight::from_parts(5_579_359, 0).saturating_mul(i.into()))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(i.into())))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(i.into())))
			.saturating_add(Weight::from_parts(0, 1669340).saturating_mul(i.into()))
	}
	/// Storage: `XRPLBridge::HighestSettledLedgerIndex` (r:1 w:0)
	/// Proof: `XRPLBridge::HighestSettledLedgerIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::SubmissionWindowWidth` (r:1 w:0)
	/// Proof: `XRPLBridge::SubmissionWindowWidth` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::SettledXRPTransactionDetails` (r:1 w:1)
	/// Proof: `XRPLBridge::SettledXRPTransactionDetails` (`max_values`: None, `max_size`: Some(64000016), added: 64002491, mode: `MaxEncodedLen`)
	/// Storage: `XRPLBridge::ProcessXRPTransactionDetails` (r:0 w:10)
	/// Proof: `XRPLBridge::ProcessXRPTransactionDetails` (`max_values`: None, `max_size`: Some(233), added: 2708, mode: `MaxEncodedLen`)
	/// The range of component `i` is `[0, 10]`.
	fn prune_settled_ledger_index(i: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `299 + i * (64 ±0)`
		//  Estimated: `64003481`
		// Minimum execution time: 14_000_000 picoseconds.
		Weight::from_parts(15_453_493, 0)
			.saturating_add(Weight::from_parts(0, 64003481))
			// Standard Error: 7_061
			.saturating_add(Weight::from_parts(1_238_165, 0).saturating_mul(i.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(i.into())))
	}
}
