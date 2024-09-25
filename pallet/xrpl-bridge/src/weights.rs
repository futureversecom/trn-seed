// This file is part of Substrate.

// Copyright (C) 2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Autogenerated weights for pallet_xrpl_bridge
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-09-09, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-102-147`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: , WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

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
// ./pallet/xrpl-bridge/src/weights.rs
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_xrpl_bridge.
pub trait WeightInfo {
	fn submit_transaction() -> Weight;
	fn submit_challenge() -> Weight;
	fn set_payment_delay() -> Weight;
	fn withdraw_xrp() -> Weight;
	fn withdraw_asset() -> Weight;
	fn add_relayer() -> Weight;
	fn remove_relayer() -> Weight;
	fn set_door_tx_fee() -> Weight;
	fn set_xrp_source_tag() -> Weight;
	fn set_door_address() -> Weight;
	fn set_ticket_sequence_next_allocation() -> Weight;
	fn set_ticket_sequence_current_allocation() -> Weight;
	fn reset_settled_xrpl_tx_data(i: u32, ) -> Weight;
	fn prune_settled_ledger_index(i: u32, ) -> Weight;
	fn set_xrpl_asset_map() -> Weight;
	fn remove_xrpl_asset_map() -> Weight;
}

/// Weights for pallet_xrpl_bridge using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: `XRPLBridge::Relayer` (r:1 w:0)
	// Proof: `XRPLBridge::Relayer` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::HighestSettledLedgerIndex` (r:1 w:0)
	// Proof: `XRPLBridge::HighestSettledLedgerIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::SubmissionWindowWidth` (r:1 w:0)
	// Proof: `XRPLBridge::SubmissionWindowWidth` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::ProcessXRPTransactionDetails` (r:1 w:1)
	// Proof: `XRPLBridge::ProcessXRPTransactionDetails` (`max_values`: None, `max_size`: Some(242), added: 2717, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::ProcessXRPTransaction` (r:1 w:1)
	// Proof: `XRPLBridge::ProcessXRPTransaction` (`max_values`: None, `max_size`: Some(64000016), added: 64002491, mode: `MaxEncodedLen`)
	fn submit_transaction() -> Weight {
		Weight::from_all(64_972_000 as u64)
			.saturating_add(T::DbWeight::get().reads(5 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: `XRPLBridge::ChallengeXRPTransactionList` (r:0 w:1)
	// Proof: `XRPLBridge::ChallengeXRPTransactionList` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	fn submit_challenge() -> Weight {
		Weight::from_all(15_065_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: `XRPLBridge::PaymentDelay` (r:0 w:1)
	// Proof: `XRPLBridge::PaymentDelay` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	fn set_payment_delay() -> Weight {
		Weight::from_all(25_892_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: `XRPLBridge::DoorTxFee` (r:1 w:0)
	// Proof: `XRPLBridge::DoorTxFee` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorAddress` (r:1 w:0)
	// Proof: `XRPLBridge::DoorAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:1 w:1)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequence` (r:1 w:1)
	// Proof: `XRPLBridge::DoorTicketSequence` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequenceParams` (r:1 w:1)
	// Proof: `XRPLBridge::DoorTicketSequenceParams` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequenceParamsNext` (r:1 w:1)
	// Proof: `XRPLBridge::DoorTicketSequenceParamsNext` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::PaymentDelay` (r:1 w:0)
	// Proof: `XRPLBridge::PaymentDelay` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::SourceTag` (r:1 w:0)
	// Proof: `XRPLBridge::SourceTag` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `EthBridge::NextEventProofId` (r:1 w:1)
	// Proof: `EthBridge::NextEventProofId` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `EthBridge::BridgePaused` (r:1 w:0)
	// Proof: `EthBridge::BridgePaused` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `System::Digest` (r:1 w:1)
	// Proof: `System::Digest` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `XRPLBridge::TicketSequenceThresholdReachedEmitted` (r:0 w:1)
	// Proof: `XRPLBridge::TicketSequenceThresholdReachedEmitted` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn withdraw_xrp() -> Weight {
		Weight::from_all(178_438_000 as u64)
			.saturating_add(T::DbWeight::get().reads(12 as u64))
			.saturating_add(T::DbWeight::get().writes(8 as u64))
	}
	// Storage: `Assets::Metadata` (r:1 w:0)
	// Proof: `Assets::Metadata` (`max_values`: None, `max_size`: Some(140), added: 2615, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTxFee` (r:1 w:0)
	// Proof: `XRPLBridge::DoorTxFee` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorAddress` (r:1 w:0)
	// Proof: `XRPLBridge::DoorAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::AssetIdToXRPL` (r:1 w:0)
	// Proof: `XRPLBridge::AssetIdToXRPL` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:1 w:1)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:2 w:2)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequence` (r:1 w:1)
	// Proof: `XRPLBridge::DoorTicketSequence` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequenceParams` (r:1 w:1)
	// Proof: `XRPLBridge::DoorTicketSequenceParams` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequenceParamsNext` (r:1 w:1)
	// Proof: `XRPLBridge::DoorTicketSequenceParamsNext` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::PaymentDelay` (r:1 w:0)
	// Proof: `XRPLBridge::PaymentDelay` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::SourceTag` (r:1 w:0)
	// Proof: `XRPLBridge::SourceTag` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `EthBridge::NextEventProofId` (r:1 w:1)
	// Proof: `EthBridge::NextEventProofId` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `EthBridge::BridgePaused` (r:1 w:0)
	// Proof: `EthBridge::BridgePaused` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `System::Digest` (r:1 w:1)
	// Proof: `System::Digest` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `XRPLBridge::TicketSequenceThresholdReachedEmitted` (r:0 w:1)
	// Proof: `XRPLBridge::TicketSequenceThresholdReachedEmitted` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn withdraw_asset() -> Weight {
		Weight::from_all(310_289_000 as u64)
			.saturating_add(T::DbWeight::get().reads(16 as u64))
			.saturating_add(T::DbWeight::get().writes(10 as u64))
	}
	// Storage: `XRPLBridge::Relayer` (r:0 w:1)
	// Proof: `XRPLBridge::Relayer` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	fn add_relayer() -> Weight {
		Weight::from_all(29_363_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: `XRPLBridge::Relayer` (r:1 w:1)
	// Proof: `XRPLBridge::Relayer` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	fn remove_relayer() -> Weight {
		Weight::from_all(42_895_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: `XRPLBridge::DoorTxFee` (r:0 w:1)
	// Proof: `XRPLBridge::DoorTxFee` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	fn set_door_tx_fee() -> Weight {
		Weight::from_all(10_176_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: `XRPLBridge::SourceTag` (r:0 w:1)
	// Proof: `XRPLBridge::SourceTag` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn set_xrp_source_tag() -> Weight {
		Weight::from_all(9_973_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: `XRPLBridge::DoorAddress` (r:0 w:1)
	// Proof: `XRPLBridge::DoorAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_door_address() -> Weight {
		Weight::from_all(27_777_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: `XRPLBridge::Relayer` (r:1 w:0)
	// Proof: `XRPLBridge::Relayer` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequence` (r:1 w:0)
	// Proof: `XRPLBridge::DoorTicketSequence` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequenceParams` (r:1 w:0)
	// Proof: `XRPLBridge::DoorTicketSequenceParams` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequenceParamsNext` (r:0 w:1)
	// Proof: `XRPLBridge::DoorTicketSequenceParamsNext` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	fn set_ticket_sequence_next_allocation() -> Weight {
		Weight::from_all(41_951_000 as u64)
			.saturating_add(T::DbWeight::get().reads(3 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: `XRPLBridge::DoorTicketSequence` (r:1 w:1)
	// Proof: `XRPLBridge::DoorTicketSequence` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequenceParams` (r:1 w:1)
	// Proof: `XRPLBridge::DoorTicketSequenceParams` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::TicketSequenceThresholdReachedEmitted` (r:0 w:1)
	// Proof: `XRPLBridge::TicketSequenceThresholdReachedEmitted` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn set_ticket_sequence_current_allocation() -> Weight {
		Weight::from_all(36_436_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
	}
	// Storage: `XRPLBridge::SettledXRPTransactionDetails` (r:256 w:256)
	// Proof: `XRPLBridge::SettledXRPTransactionDetails` (`max_values`: None, `max_size`: Some(64000016), added: 64002491, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::SubmissionWindowWidth` (r:0 w:1)
	// Proof: `XRPLBridge::SubmissionWindowWidth` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::HighestPrunedLedgerIndex` (r:0 w:1)
	// Proof: `XRPLBridge::HighestPrunedLedgerIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::HighestSettledLedgerIndex` (r:0 w:1)
	// Proof: `XRPLBridge::HighestSettledLedgerIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::ProcessXRPTransactionDetails` (r:0 w:256)
	// Proof: `XRPLBridge::ProcessXRPTransactionDetails` (`max_values`: None, `max_size`: Some(242), added: 2717, mode: `MaxEncodedLen`)
	/// The range of component `i` is `[0, 256]`.
	fn reset_settled_xrpl_tx_data(i: u32, ) -> Weight {
		Weight::from_all(16_313_850 as u64)
			// Standard Error: 22_394
			.saturating_add(Weight::from_all(11_235_575 as u64).saturating_mul(i as u64))
			.saturating_add(T::DbWeight::get().reads((1 as u64).saturating_mul(i as u64)))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
			.saturating_add(T::DbWeight::get().writes((2 as u64).saturating_mul(i as u64)))
	}
	// Storage: `XRPLBridge::HighestSettledLedgerIndex` (r:1 w:0)
	// Proof: `XRPLBridge::HighestSettledLedgerIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::SubmissionWindowWidth` (r:1 w:0)
	// Proof: `XRPLBridge::SubmissionWindowWidth` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::SettledXRPTransactionDetails` (r:1 w:1)
	// Proof: `XRPLBridge::SettledXRPTransactionDetails` (`max_values`: None, `max_size`: Some(64000016), added: 64002491, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::ProcessXRPTransactionDetails` (r:0 w:10)
	// Proof: `XRPLBridge::ProcessXRPTransactionDetails` (`max_values`: None, `max_size`: Some(242), added: 2717, mode: `MaxEncodedLen`)
	/// The range of component `i` is `[0, 10]`.
	fn prune_settled_ledger_index(i: u32, ) -> Weight {
		Weight::from_all(44_939_320 as u64)
			// Standard Error: 14_299
			.saturating_add(Weight::from_all(2_531_049 as u64).saturating_mul(i as u64))
			.saturating_add(T::DbWeight::get().reads(3 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
			.saturating_add(T::DbWeight::get().writes((1 as u64).saturating_mul(i as u64)))
	}
	// Storage: `XRPLBridge::XRPLToAssetId` (r:0 w:1)
	// Proof: `XRPLBridge::XRPLToAssetId` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::AssetIdToXRPL` (r:0 w:1)
	// Proof: `XRPLBridge::AssetIdToXRPL` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	fn set_xrpl_asset_map() -> Weight {
		Weight::from_all(31_578_000 as u64)
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: `XRPLBridge::XRPLToAssetId` (r:0 w:1)
	// Storage: `XRPLBridge::AssetIdToXRPL` (r:0 w:1)
	fn remove_xrpl_asset_map() -> Weight {
		Weight::from_all(31_578_000 as u64)
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: `XRPLBridge::Relayer` (r:1 w:0)
	// Proof: `XRPLBridge::Relayer` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::HighestSettledLedgerIndex` (r:1 w:0)
	// Proof: `XRPLBridge::HighestSettledLedgerIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::SubmissionWindowWidth` (r:1 w:0)
	// Proof: `XRPLBridge::SubmissionWindowWidth` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::ProcessXRPTransactionDetails` (r:1 w:1)
	// Proof: `XRPLBridge::ProcessXRPTransactionDetails` (`max_values`: None, `max_size`: Some(242), added: 2717, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::ProcessXRPTransaction` (r:1 w:1)
	// Proof: `XRPLBridge::ProcessXRPTransaction` (`max_values`: None, `max_size`: Some(64000016), added: 64002491, mode: `MaxEncodedLen`)
	fn submit_transaction() -> Weight {
		Weight::from_all(64_972_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(5 as u64))
			.saturating_add(RocksDbWeight::get().writes(2 as u64))
	}
	// Storage: `XRPLBridge::ChallengeXRPTransactionList` (r:0 w:1)
	// Proof: `XRPLBridge::ChallengeXRPTransactionList` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	fn submit_challenge() -> Weight {
		Weight::from_all(15_065_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: `XRPLBridge::PaymentDelay` (r:0 w:1)
	// Proof: `XRPLBridge::PaymentDelay` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	fn set_payment_delay() -> Weight {
		Weight::from_all(25_892_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: `XRPLBridge::DoorTxFee` (r:1 w:0)
	// Proof: `XRPLBridge::DoorTxFee` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorAddress` (r:1 w:0)
	// Proof: `XRPLBridge::DoorAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:1 w:1)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequence` (r:1 w:1)
	// Proof: `XRPLBridge::DoorTicketSequence` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequenceParams` (r:1 w:1)
	// Proof: `XRPLBridge::DoorTicketSequenceParams` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequenceParamsNext` (r:1 w:1)
	// Proof: `XRPLBridge::DoorTicketSequenceParamsNext` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::PaymentDelay` (r:1 w:0)
	// Proof: `XRPLBridge::PaymentDelay` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::SourceTag` (r:1 w:0)
	// Proof: `XRPLBridge::SourceTag` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `EthBridge::NextEventProofId` (r:1 w:1)
	// Proof: `EthBridge::NextEventProofId` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `EthBridge::BridgePaused` (r:1 w:0)
	// Proof: `EthBridge::BridgePaused` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `System::Digest` (r:1 w:1)
	// Proof: `System::Digest` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `XRPLBridge::TicketSequenceThresholdReachedEmitted` (r:0 w:1)
	// Proof: `XRPLBridge::TicketSequenceThresholdReachedEmitted` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn withdraw_xrp() -> Weight {
		Weight::from_all(178_438_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(12 as u64))
			.saturating_add(RocksDbWeight::get().writes(8 as u64))
	}
	// Storage: `Assets::Metadata` (r:1 w:0)
	// Proof: `Assets::Metadata` (`max_values`: None, `max_size`: Some(140), added: 2615, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTxFee` (r:1 w:0)
	// Proof: `XRPLBridge::DoorTxFee` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorAddress` (r:1 w:0)
	// Proof: `XRPLBridge::DoorAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::AssetIdToXRPL` (r:1 w:0)
	// Proof: `XRPLBridge::AssetIdToXRPL` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:1 w:1)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:2 w:2)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequence` (r:1 w:1)
	// Proof: `XRPLBridge::DoorTicketSequence` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequenceParams` (r:1 w:1)
	// Proof: `XRPLBridge::DoorTicketSequenceParams` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequenceParamsNext` (r:1 w:1)
	// Proof: `XRPLBridge::DoorTicketSequenceParamsNext` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::PaymentDelay` (r:1 w:0)
	// Proof: `XRPLBridge::PaymentDelay` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::SourceTag` (r:1 w:0)
	// Proof: `XRPLBridge::SourceTag` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `EthBridge::NextEventProofId` (r:1 w:1)
	// Proof: `EthBridge::NextEventProofId` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `EthBridge::BridgePaused` (r:1 w:0)
	// Proof: `EthBridge::BridgePaused` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `System::Digest` (r:1 w:1)
	// Proof: `System::Digest` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `XRPLBridge::TicketSequenceThresholdReachedEmitted` (r:0 w:1)
	// Proof: `XRPLBridge::TicketSequenceThresholdReachedEmitted` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn withdraw_asset() -> Weight {
		Weight::from_all(310_289_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(16 as u64))
			.saturating_add(RocksDbWeight::get().writes(10 as u64))
	}
	// Storage: `XRPLBridge::Relayer` (r:0 w:1)
	// Proof: `XRPLBridge::Relayer` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	fn add_relayer() -> Weight {
		Weight::from_all(29_363_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: `XRPLBridge::Relayer` (r:1 w:1)
	// Proof: `XRPLBridge::Relayer` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	fn remove_relayer() -> Weight {
		Weight::from_all(42_895_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: `XRPLBridge::DoorTxFee` (r:0 w:1)
	// Proof: `XRPLBridge::DoorTxFee` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	fn set_door_tx_fee() -> Weight {
		Weight::from_all(10_176_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: `XRPLBridge::SourceTag` (r:0 w:1)
	// Proof: `XRPLBridge::SourceTag` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn set_xrp_source_tag() -> Weight {
		Weight::from_all(9_973_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: `XRPLBridge::DoorAddress` (r:0 w:1)
	// Proof: `XRPLBridge::DoorAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_door_address() -> Weight {
		Weight::from_all(27_777_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: `XRPLBridge::Relayer` (r:1 w:0)
	// Proof: `XRPLBridge::Relayer` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequence` (r:1 w:0)
	// Proof: `XRPLBridge::DoorTicketSequence` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequenceParams` (r:1 w:0)
	// Proof: `XRPLBridge::DoorTicketSequenceParams` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequenceParamsNext` (r:0 w:1)
	// Proof: `XRPLBridge::DoorTicketSequenceParamsNext` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	fn set_ticket_sequence_next_allocation() -> Weight {
		Weight::from_all(41_951_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(3 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: `XRPLBridge::DoorTicketSequence` (r:1 w:1)
	// Proof: `XRPLBridge::DoorTicketSequence` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::DoorTicketSequenceParams` (r:1 w:1)
	// Proof: `XRPLBridge::DoorTicketSequenceParams` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::TicketSequenceThresholdReachedEmitted` (r:0 w:1)
	// Proof: `XRPLBridge::TicketSequenceThresholdReachedEmitted` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn set_ticket_sequence_current_allocation() -> Weight {
		Weight::from_all(36_436_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(2 as u64))
			.saturating_add(RocksDbWeight::get().writes(3 as u64))
	}
	// Storage: `XRPLBridge::SettledXRPTransactionDetails` (r:256 w:256)
	// Proof: `XRPLBridge::SettledXRPTransactionDetails` (`max_values`: None, `max_size`: Some(64000016), added: 64002491, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::SubmissionWindowWidth` (r:0 w:1)
	// Proof: `XRPLBridge::SubmissionWindowWidth` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::HighestPrunedLedgerIndex` (r:0 w:1)
	// Proof: `XRPLBridge::HighestPrunedLedgerIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::HighestSettledLedgerIndex` (r:0 w:1)
	// Proof: `XRPLBridge::HighestSettledLedgerIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::ProcessXRPTransactionDetails` (r:0 w:256)
	// Proof: `XRPLBridge::ProcessXRPTransactionDetails` (`max_values`: None, `max_size`: Some(242), added: 2717, mode: `MaxEncodedLen`)
	/// The range of component `i` is `[0, 256]`.
	fn reset_settled_xrpl_tx_data(i: u32, ) -> Weight {
		Weight::from_all(16_313_850 as u64)
			// Standard Error: 22_394
			.saturating_add(Weight::from_all(11_235_575 as u64).saturating_mul(i as u64))
			.saturating_add(RocksDbWeight::get().reads((1 as u64).saturating_mul(i as u64)))
			.saturating_add(RocksDbWeight::get().writes(3 as u64))
			.saturating_add(RocksDbWeight::get().writes((2 as u64).saturating_mul(i as u64)))
	}
	// Storage: `XRPLBridge::HighestSettledLedgerIndex` (r:1 w:0)
	// Proof: `XRPLBridge::HighestSettledLedgerIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::SubmissionWindowWidth` (r:1 w:0)
	// Proof: `XRPLBridge::SubmissionWindowWidth` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::SettledXRPTransactionDetails` (r:1 w:1)
	// Proof: `XRPLBridge::SettledXRPTransactionDetails` (`max_values`: None, `max_size`: Some(64000016), added: 64002491, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::ProcessXRPTransactionDetails` (r:0 w:10)
	// Proof: `XRPLBridge::ProcessXRPTransactionDetails` (`max_values`: None, `max_size`: Some(242), added: 2717, mode: `MaxEncodedLen`)
	/// The range of component `i` is `[0, 10]`.
	fn prune_settled_ledger_index(i: u32, ) -> Weight {
		Weight::from_all(44_939_320 as u64)
			// Standard Error: 14_299
			.saturating_add(Weight::from_all(2_531_049 as u64).saturating_mul(i as u64))
			.saturating_add(RocksDbWeight::get().reads(3 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
			.saturating_add(RocksDbWeight::get().writes((1 as u64).saturating_mul(i as u64)))
	}
	// Storage: `XRPLBridge::XRPLToAssetId` (r:0 w:1)
	// Proof: `XRPLBridge::XRPLToAssetId` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	// Storage: `XRPLBridge::AssetIdToXRPL` (r:0 w:1)
	// Proof: `XRPLBridge::AssetIdToXRPL` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	fn set_xrpl_asset_map() -> Weight {
		Weight::from_all(31_578_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(2 as u64))
	}
	// Storage: `XRPLBridge::XRPLToAssetId` (r:0 w:1)
	// Storage: `XRPLBridge::AssetIdToXRPL` (r:0 w:1)
	fn remove_xrpl_asset_map() -> Weight {
		Weight::from_all(31_578_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(2 as u64))
	}
}

