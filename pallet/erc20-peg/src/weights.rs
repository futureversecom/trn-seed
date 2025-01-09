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
//! DATE: 2024-10-15, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-102-147`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: , WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-erc20-peg
// --extrinsic=*
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
	fn process_deposit() -> Weight;
	fn claim_delayed_payment() -> Weight;
	fn set_erc20_peg_address() -> Weight;
	fn set_root_peg_address() -> Weight;
	fn set_erc20_asset_map() -> Weight;
	fn set_erc20_meta() -> Weight;
	fn set_payment_delay() -> Weight;
}

/// Weights for pallet_erc20_peg using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: `Erc20Peg::DepositsActive` (r:0 w:1)
	// Proof: `Erc20Peg::DepositsActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn activate_deposits() -> Weight {
		Weight::from_all(23_931_000_u64)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	// Storage: `Erc20Peg::WithdrawalsActive` (r:0 w:1)
	// Proof: `Erc20Peg::WithdrawalsActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn activate_withdrawals() -> Weight {
		Weight::from_all(23_712_000_u64)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	// Storage: `Erc20Peg::DepositsDelayActive` (r:0 w:1)
	// Proof: `Erc20Peg::DepositsDelayActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn activate_deposits_delay() -> Weight {
		Weight::from_all(23_408_000_u64)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	// Storage: `Erc20Peg::WithdrawalsDelayActive` (r:0 w:1)
	// Proof: `Erc20Peg::WithdrawalsDelayActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn activate_withdrawals_delay() -> Weight {
		Weight::from_all(23_447_000_u64)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	// Storage: `Erc20Peg::WithdrawalsActive` (r:1 w:0)
	// Proof: `Erc20Peg::WithdrawalsActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	// Storage: `Erc20Peg::AssetIdToErc20` (r:1 w:0)
	// Proof: `Erc20Peg::AssetIdToErc20` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	// Storage: `Erc20Peg::PaymentDelay` (r:1 w:0)
	// Proof: `Erc20Peg::PaymentDelay` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	// Storage: `Erc20Peg::WithdrawalsDelayActive` (r:1 w:0)
	// Proof: `Erc20Peg::WithdrawalsDelayActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:1 w:1)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `Erc20Peg::ContractAddress` (r:1 w:0)
	// Proof: `Erc20Peg::ContractAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	// Storage: `EthBridge::NextEventProofId` (r:1 w:1)
	// Proof: `EthBridge::NextEventProofId` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `EthBridge::NotaryKeys` (r:1 w:0)
	// Proof: `EthBridge::NotaryKeys` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `EthBridge::NotarySetId` (r:1 w:0)
	// Proof: `EthBridge::NotarySetId` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `EthBridge::BridgePaused` (r:1 w:0)
	// Proof: `EthBridge::BridgePaused` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `System::Digest` (r:1 w:1)
	// Proof: `System::Digest` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn withdraw() -> Weight {
		Weight::from_all(180_161_000_u64)
			.saturating_add(T::DbWeight::get().reads(12_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}
    // Storage: `Erc20Peg::Erc20ToAssetId` (r:1 w:1)
    // Proof: `Erc20Peg::Erc20ToAssetId` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
    // Storage: `Erc20Peg::Erc20Meta` (r:1 w:0)
    // Proof: `Erc20Peg::Erc20Meta` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
    // Storage: `AssetsExt::NextAssetId` (r:1 w:1)
    // Proof: `AssetsExt::NextAssetId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
    // Storage: `Assets::Asset` (r:1 w:1)
    // Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
    // Storage: `EVM::AccountCodes` (r:1 w:1)
    // Proof: `EVM::AccountCodes` (`max_values`: None, `max_size`: None, mode: `Measured`)
    // Storage: `Futurepass::DefaultProxy` (r:1 w:0)
    // Proof: `Futurepass::DefaultProxy` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
    // Storage: `System::Account` (r:2 w:2)
    // Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
    // Storage: `Assets::Metadata` (r:1 w:1)
    // Proof: `Assets::Metadata` (`max_values`: None, `max_size`: Some(140), added: 2615, mode: `MaxEncodedLen`)
    // Storage: `Assets::Account` (r:1 w:1)
    // Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
    // Storage: `EVM::AccountCodesMetadata` (r:0 w:1)
    // Proof: `EVM::AccountCodesMetadata` (`max_values`: None, `max_size`: None, mode: `Measured`)
    // Storage: `Erc20Peg::AssetIdToErc20` (r:0 w:1)
    // Proof: `Erc20Peg::AssetIdToErc20` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
    fn process_deposit() -> Weight {
        Weight::from_all(200_713_000_u64)
            .saturating_add(T::DbWeight::get().reads(10_u64))
            .saturating_add(T::DbWeight::get().writes(10_u64))
    }
    // Storage: `Erc20Peg::DelayedPaymentSchedule` (r:1 w:1)
    // Proof: `Erc20Peg::DelayedPaymentSchedule` (`max_values`: None, `max_size`: Some(4814), added: 7289, mode: `MaxEncodedLen`)
    // Storage: `Erc20Peg::DelayedPayments` (r:1 w:1)
    // Proof: `Erc20Peg::DelayedPayments` (`max_values`: None, `max_size`: Some(109), added: 2584, mode: `MaxEncodedLen`)
    // Storage: `Erc20Peg::Erc20ToAssetId` (r:1 w:0)
    // Proof: `Erc20Peg::Erc20ToAssetId` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
    // Storage: `Erc20Peg::ContractAddress` (r:1 w:0)
    // Proof: `Erc20Peg::ContractAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
    // Storage: `EthBridge::NextEventProofId` (r:1 w:1)
    // Proof: `EthBridge::NextEventProofId` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
    // Storage: `EthBridge::NotaryKeys` (r:1 w:0)
    // Proof: `EthBridge::NotaryKeys` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
    // Storage: `EthBridge::NotarySetId` (r:1 w:0)
    // Proof: `EthBridge::NotarySetId` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
    // Storage: `EthBridge::BridgePaused` (r:1 w:0)
    // Proof: `EthBridge::BridgePaused` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
    // Storage: `System::Digest` (r:1 w:1)
    // Proof: `System::Digest` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
    fn claim_delayed_payment() -> Weight {
        Weight::from_all(101_333_000_u64)
            .saturating_add(T::DbWeight::get().reads(9_u64))
            .saturating_add(T::DbWeight::get().writes(4_u64))
    }
	// Storage: `Erc20Peg::ContractAddress` (r:0 w:1)
	// Proof: `Erc20Peg::ContractAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_erc20_peg_address() -> Weight {
		Weight::from_all(24_962_000_u64)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	// Storage: `Erc20Peg::RootPegContractAddress` (r:0 w:1)
	// Proof: `Erc20Peg::RootPegContractAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_root_peg_address() -> Weight {
		Weight::from_all(24_926_000_u64)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	// Storage: `Erc20Peg::Erc20ToAssetId` (r:0 w:1)
	// Proof: `Erc20Peg::Erc20ToAssetId` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	// Storage: `Erc20Peg::AssetIdToErc20` (r:0 w:1)
	// Proof: `Erc20Peg::AssetIdToErc20` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	fn set_erc20_asset_map() -> Weight {
		Weight::from_all(15_637_000_u64)
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	// Storage: `Erc20Peg::Erc20Meta` (r:0 w:1)
	// Proof: `Erc20Peg::Erc20Meta` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	fn set_erc20_meta() -> Weight {
		Weight::from_all(14_207_000_u64)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	// Storage: `Erc20Peg::PaymentDelay` (r:0 w:1)
	// Proof: `Erc20Peg::PaymentDelay` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	fn set_payment_delay() -> Weight {
		Weight::from_all(26_115_000_u64)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: `Erc20Peg::DepositsActive` (r:0 w:1)
	// Proof: `Erc20Peg::DepositsActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn activate_deposits() -> Weight {
		Weight::from_all(23_931_000_u64)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	// Storage: `Erc20Peg::WithdrawalsActive` (r:0 w:1)
	// Proof: `Erc20Peg::WithdrawalsActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn activate_withdrawals() -> Weight {
		Weight::from_all(23_712_000_u64)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	// Storage: `Erc20Peg::DepositsDelayActive` (r:0 w:1)
	// Proof: `Erc20Peg::DepositsDelayActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn activate_deposits_delay() -> Weight {
		Weight::from_all(23_408_000_u64)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	// Storage: `Erc20Peg::WithdrawalsDelayActive` (r:0 w:1)
	// Proof: `Erc20Peg::WithdrawalsDelayActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn activate_withdrawals_delay() -> Weight {
		Weight::from_all(23_447_000_u64)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	// Storage: `Erc20Peg::WithdrawalsActive` (r:1 w:0)
	// Proof: `Erc20Peg::WithdrawalsActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	// Storage: `Erc20Peg::AssetIdToErc20` (r:1 w:0)
	// Proof: `Erc20Peg::AssetIdToErc20` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	// Storage: `Erc20Peg::PaymentDelay` (r:1 w:0)
	// Proof: `Erc20Peg::PaymentDelay` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	// Storage: `Erc20Peg::WithdrawalsDelayActive` (r:1 w:0)
	// Proof: `Erc20Peg::WithdrawalsDelayActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:1 w:1)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `Erc20Peg::ContractAddress` (r:1 w:0)
	// Proof: `Erc20Peg::ContractAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	// Storage: `EthBridge::NextEventProofId` (r:1 w:1)
	// Proof: `EthBridge::NextEventProofId` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `EthBridge::NotaryKeys` (r:1 w:0)
	// Proof: `EthBridge::NotaryKeys` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `EthBridge::NotarySetId` (r:1 w:0)
	// Proof: `EthBridge::NotarySetId` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `EthBridge::BridgePaused` (r:1 w:0)
	// Proof: `EthBridge::BridgePaused` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	// Storage: `System::Digest` (r:1 w:1)
	// Proof: `System::Digest` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn withdraw() -> Weight {
		Weight::from_all(180_161_000_u64)
			.saturating_add(RocksDbWeight::get().reads(12_u64))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
	}
    // Storage: `Erc20Peg::Erc20ToAssetId` (r:1 w:1)
    // Proof: `Erc20Peg::Erc20ToAssetId` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
    // Storage: `Erc20Peg::Erc20Meta` (r:1 w:0)
    // Proof: `Erc20Peg::Erc20Meta` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
    // Storage: `AssetsExt::NextAssetId` (r:1 w:1)
    // Proof: `AssetsExt::NextAssetId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
    // Storage: `Assets::Asset` (r:1 w:1)
    // Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
    // Storage: `EVM::AccountCodes` (r:1 w:1)
    // Proof: `EVM::AccountCodes` (`max_values`: None, `max_size`: None, mode: `Measured`)
    // Storage: `Futurepass::DefaultProxy` (r:1 w:0)
    // Proof: `Futurepass::DefaultProxy` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
    // Storage: `System::Account` (r:2 w:2)
    // Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
    // Storage: `Assets::Metadata` (r:1 w:1)
    // Proof: `Assets::Metadata` (`max_values`: None, `max_size`: Some(140), added: 2615, mode: `MaxEncodedLen`)
    // Storage: `Assets::Account` (r:1 w:1)
    // Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
    // Storage: `EVM::AccountCodesMetadata` (r:0 w:1)
    // Proof: `EVM::AccountCodesMetadata` (`max_values`: None, `max_size`: None, mode: `Measured`)
    // Storage: `Erc20Peg::AssetIdToErc20` (r:0 w:1)
    // Proof: `Erc20Peg::AssetIdToErc20` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
    fn process_deposit() -> Weight {
        Weight::from_all(200_713_000_u64)
            .saturating_add(RocksDbWeight::get().reads(10_u64))
            .saturating_add(RocksDbWeight::get().writes(10_u64))
    }
    // Storage: `Erc20Peg::DelayedPaymentSchedule` (r:1 w:1)
    // Proof: `Erc20Peg::DelayedPaymentSchedule` (`max_values`: None, `max_size`: Some(4814), added: 7289, mode: `MaxEncodedLen`)
    // Storage: `Erc20Peg::DelayedPayments` (r:1 w:1)
    // Proof: `Erc20Peg::DelayedPayments` (`max_values`: None, `max_size`: Some(109), added: 2584, mode: `MaxEncodedLen`)
    // Storage: `Erc20Peg::Erc20ToAssetId` (r:1 w:0)
    // Proof: `Erc20Peg::Erc20ToAssetId` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
    // Storage: `Erc20Peg::ContractAddress` (r:1 w:0)
    // Proof: `Erc20Peg::ContractAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
    // Storage: `EthBridge::NextEventProofId` (r:1 w:1)
    // Proof: `EthBridge::NextEventProofId` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
    // Storage: `EthBridge::NotaryKeys` (r:1 w:0)
    // Proof: `EthBridge::NotaryKeys` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
    // Storage: `EthBridge::NotarySetId` (r:1 w:0)
    // Proof: `EthBridge::NotarySetId` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
    // Storage: `EthBridge::BridgePaused` (r:1 w:0)
    // Proof: `EthBridge::BridgePaused` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
    // Storage: `System::Digest` (r:1 w:1)
    // Proof: `System::Digest` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
    fn claim_delayed_payment() -> Weight {
        Weight::from_all(101_333_000_u64)
            .saturating_add(RocksDbWeight::get().reads(9_u64))
            .saturating_add(RocksDbWeight::get().writes(4_u64))
    }
	// Storage: `Erc20Peg::ContractAddress` (r:0 w:1)
	// Proof: `Erc20Peg::ContractAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_erc20_peg_address() -> Weight {
		Weight::from_all(24_962_000_u64)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	// Storage: `Erc20Peg::RootPegContractAddress` (r:0 w:1)
	// Proof: `Erc20Peg::RootPegContractAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_root_peg_address() -> Weight {
		Weight::from_all(24_926_000_u64)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	// Storage: `Erc20Peg::Erc20ToAssetId` (r:0 w:1)
	// Proof: `Erc20Peg::Erc20ToAssetId` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	// Storage: `Erc20Peg::AssetIdToErc20` (r:0 w:1)
	// Proof: `Erc20Peg::AssetIdToErc20` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	fn set_erc20_asset_map() -> Weight {
		Weight::from_all(15_637_000_u64)
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	// Storage: `Erc20Peg::Erc20Meta` (r:0 w:1)
	// Proof: `Erc20Peg::Erc20Meta` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	fn set_erc20_meta() -> Weight {
		Weight::from_all(14_207_000_u64)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	// Storage: `Erc20Peg::PaymentDelay` (r:0 w:1)
	// Proof: `Erc20Peg::PaymentDelay` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	fn set_payment_delay() -> Weight {
		Weight::from_all(26_115_000_u64)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
}

