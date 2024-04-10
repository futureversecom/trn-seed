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

//! Autogenerated weights for pallet_erc20_peg
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-04-10, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-102-147`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-erc20-peg
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./pallet/erc20-peg/src/weights.rs
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_erc20_peg.
pub trait WeightInfo {
	fn activate_deposits() -> Weight;
	fn activate_withdrawals() -> Weight;
	fn activate_deposits_delay() -> Weight;
	fn activate_withdrawals_delay() -> Weight;
	fn withdraw() -> Weight;
	fn set_erc20_peg_address() -> Weight;
	fn set_root_peg_address() -> Weight;
	fn set_erc20_asset_map() -> Weight;
	fn set_erc20_meta() -> Weight;
	fn set_payment_delay() -> Weight;
}

/// Weights for pallet_erc20_peg using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: Erc20Peg DepositsActive (r:0 w:1)
	fn activate_deposits() -> Weight {
		Weight::from_ref_time(15_386_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Erc20Peg WithdrawalsActive (r:0 w:1)
	fn activate_withdrawals() -> Weight {
		Weight::from_ref_time(15_112_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Erc20Peg DepositsDelayActive (r:0 w:1)
	fn activate_deposits_delay() -> Weight {
		Weight::from_ref_time(37_675_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Erc20Peg WithdrawalsDelayActive (r:0 w:1)
	fn activate_withdrawals_delay() -> Weight {
		Weight::from_ref_time(37_428_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Erc20Peg WithdrawalsActive (r:1 w:0)
	// Storage: Erc20Peg AssetIdToErc20 (r:1 w:0)
	// Storage: Erc20Peg PaymentDelay (r:1 w:0)
	// Storage: Erc20Peg WithdrawalsDelayActive (r:1 w:0)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:1 w:1)
	// Storage: Erc20Peg ContractAddress (r:1 w:0)
	// Storage: EthBridge NextEventProofId (r:1 w:1)
	// Storage: EthBridge NotaryKeys (r:1 w:0)
	// Storage: EthBridge NotarySetId (r:1 w:0)
	// Storage: EthBridge BridgePaused (r:1 w:0)
	// Storage: System Digest (r:1 w:1)
	fn withdraw() -> Weight {
		Weight::from_ref_time(162_424_000 as u64)
			.saturating_add(T::DbWeight::get().reads(12 as u64))
			.saturating_add(T::DbWeight::get().writes(4 as u64))
	}
	// Storage: Erc20Peg ContractAddress (r:0 w:1)
	fn set_erc20_peg_address() -> Weight {
		Weight::from_ref_time(39_285_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Erc20Peg RootPegContractAddress (r:0 w:1)
	fn set_root_peg_address() -> Weight {
		Weight::from_ref_time(38_419_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Erc20Peg Erc20ToAssetId (r:0 w:1)
	// Storage: Erc20Peg AssetIdToErc20 (r:0 w:1)
	fn set_erc20_asset_map() -> Weight {
		Weight::from_ref_time(19_335_000 as u64)
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: Erc20Peg Erc20Meta (r:0 w:1)
	fn set_erc20_meta() -> Weight {
		Weight::from_ref_time(18_553_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Erc20Peg PaymentDelay (r:0 w:1)
	fn set_payment_delay() -> Weight {
		Weight::from_ref_time(39_254_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: Erc20Peg DepositsActive (r:0 w:1)
	fn activate_deposits() -> Weight {
		Weight::from_ref_time(15_386_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Erc20Peg WithdrawalsActive (r:0 w:1)
	fn activate_withdrawals() -> Weight {
		Weight::from_ref_time(15_112_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Erc20Peg DepositsDelayActive (r:0 w:1)
	fn activate_deposits_delay() -> Weight {
		Weight::from_ref_time(37_675_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Erc20Peg WithdrawalsDelayActive (r:0 w:1)
	fn activate_withdrawals_delay() -> Weight {
		Weight::from_ref_time(37_428_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Erc20Peg WithdrawalsActive (r:1 w:0)
	// Storage: Erc20Peg AssetIdToErc20 (r:1 w:0)
	// Storage: Erc20Peg PaymentDelay (r:1 w:0)
	// Storage: Erc20Peg WithdrawalsDelayActive (r:1 w:0)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:1 w:1)
	// Storage: Erc20Peg ContractAddress (r:1 w:0)
	// Storage: EthBridge NextEventProofId (r:1 w:1)
	// Storage: EthBridge NotaryKeys (r:1 w:0)
	// Storage: EthBridge NotarySetId (r:1 w:0)
	// Storage: EthBridge BridgePaused (r:1 w:0)
	// Storage: System Digest (r:1 w:1)
	fn withdraw() -> Weight {
		Weight::from_ref_time(162_424_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(12 as u64))
			.saturating_add(RocksDbWeight::get().writes(4 as u64))
	}
	// Storage: Erc20Peg ContractAddress (r:0 w:1)
	fn set_erc20_peg_address() -> Weight {
		Weight::from_ref_time(39_285_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Erc20Peg RootPegContractAddress (r:0 w:1)
	fn set_root_peg_address() -> Weight {
		Weight::from_ref_time(38_419_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Erc20Peg Erc20ToAssetId (r:0 w:1)
	// Storage: Erc20Peg AssetIdToErc20 (r:0 w:1)
	fn set_erc20_asset_map() -> Weight {
		Weight::from_ref_time(19_335_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(2 as u64))
	}
	// Storage: Erc20Peg Erc20Meta (r:0 w:1)
	fn set_erc20_meta() -> Weight {
		Weight::from_ref_time(18_553_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	// Storage: Erc20Peg PaymentDelay (r:0 w:1)
	fn set_payment_delay() -> Weight {
		Weight::from_ref_time(39_254_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
}

