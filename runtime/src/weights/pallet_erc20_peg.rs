
//! Autogenerated weights for `pallet_erc20_peg`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-10-15, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
// --pallet=pallet-erc20-peg
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_erc20_peg.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_erc20_peg`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_erc20_peg::WeightInfo for WeightInfo<T> {
	/// Storage: `Erc20Peg::DepositsActive` (r:0 w:1)
	/// Proof: `Erc20Peg::DepositsActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn activate_deposits() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 22_484_000 picoseconds.
		Weight::from_parts(23_229_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Erc20Peg::WithdrawalsActive` (r:0 w:1)
	/// Proof: `Erc20Peg::WithdrawalsActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn activate_withdrawals() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 22_308_000 picoseconds.
		Weight::from_parts(23_039_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Erc20Peg::DepositsDelayActive` (r:0 w:1)
	/// Proof: `Erc20Peg::DepositsDelayActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn activate_deposits_delay() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 22_449_000 picoseconds.
		Weight::from_parts(23_028_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Erc20Peg::WithdrawalsDelayActive` (r:0 w:1)
	/// Proof: `Erc20Peg::WithdrawalsDelayActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn activate_withdrawals_delay() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 22_622_000 picoseconds.
		Weight::from_parts(22_952_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Erc20Peg::WithdrawalsActive` (r:1 w:0)
	/// Proof: `Erc20Peg::WithdrawalsActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `Erc20Peg::AssetIdToErc20` (r:1 w:0)
	/// Proof: `Erc20Peg::AssetIdToErc20` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	/// Storage: `Erc20Peg::PaymentDelay` (r:1 w:0)
	/// Proof: `Erc20Peg::PaymentDelay` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	/// Storage: `Erc20Peg::WithdrawalsDelayActive` (r:1 w:0)
	/// Proof: `Erc20Peg::WithdrawalsDelayActive` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	/// Storage: `Erc20Peg::ContractAddress` (r:1 w:0)
	/// Proof: `Erc20Peg::ContractAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
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
	fn withdraw() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `780`
		//  Estimated: `3627`
		// Minimum execution time: 172_314_000 picoseconds.
		Weight::from_parts(173_972_000, 0)
			.saturating_add(Weight::from_parts(0, 3627))
			.saturating_add(T::DbWeight::get().reads(12))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Erc20Peg::Erc20ToAssetId` (r:1 w:1)
	/// Proof: `Erc20Peg::Erc20ToAssetId` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	/// Storage: `Erc20Peg::Erc20Meta` (r:1 w:0)
	/// Proof: `Erc20Peg::Erc20Meta` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `AssetsExt::NextAssetId` (r:1 w:1)
	/// Proof: `AssetsExt::NextAssetId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	/// Storage: `EVM::AccountCodes` (r:1 w:1)
	/// Proof: `EVM::AccountCodes` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Futurepass::DefaultProxy` (r:1 w:0)
	/// Proof: `Futurepass::DefaultProxy` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Metadata` (r:1 w:1)
	/// Proof: `Assets::Metadata` (`max_values`: None, `max_size`: Some(140), added: 2615, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	/// Storage: `EVM::AccountCodesMetadata` (r:0 w:1)
	/// Proof: `EVM::AccountCodesMetadata` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Erc20Peg::AssetIdToErc20` (r:0 w:1)
	/// Proof: `Erc20Peg::AssetIdToErc20` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	fn process_deposit() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1033`
		//  Estimated: `6172`
		// Minimum execution time: 204_526_000 picoseconds.
		Weight::from_parts(206_503_000, 0)
			.saturating_add(Weight::from_parts(0, 6172))
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	/// Storage: `Erc20Peg::DelayedPaymentSchedule` (r:1 w:1)
	/// Proof: `Erc20Peg::DelayedPaymentSchedule` (`max_values`: None, `max_size`: Some(4814), added: 7289, mode: `MaxEncodedLen`)
	/// Storage: `Erc20Peg::DelayedPayments` (r:1 w:1)
	/// Proof: `Erc20Peg::DelayedPayments` (`max_values`: None, `max_size`: Some(109), added: 2584, mode: `MaxEncodedLen`)
	/// Storage: `Erc20Peg::Erc20ToAssetId` (r:1 w:0)
	/// Proof: `Erc20Peg::Erc20ToAssetId` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	/// Storage: `Erc20Peg::ContractAddress` (r:1 w:0)
	/// Proof: `Erc20Peg::ContractAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
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
	fn claim_delayed_payment() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `431`
		//  Estimated: `8279`
		// Minimum execution time: 103_493_000 picoseconds.
		Weight::from_parts(104_161_000, 0)
			.saturating_add(Weight::from_parts(0, 8279))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Erc20Peg::ContractAddress` (r:0 w:1)
	/// Proof: `Erc20Peg::ContractAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_erc20_peg_address() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 23_733_000 picoseconds.
		Weight::from_parts(24_430_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Erc20Peg::RootPegContractAddress` (r:0 w:1)
	/// Proof: `Erc20Peg::RootPegContractAddress` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_root_peg_address() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 23_760_000 picoseconds.
		Weight::from_parts(24_459_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Erc20Peg::Erc20ToAssetId` (r:0 w:1)
	/// Proof: `Erc20Peg::Erc20ToAssetId` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	/// Storage: `Erc20Peg::AssetIdToErc20` (r:0 w:1)
	/// Proof: `Erc20Peg::AssetIdToErc20` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	fn set_erc20_asset_map() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 15_064_000 picoseconds.
		Weight::from_parts(15_545_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Erc20Peg::Erc20Meta` (r:0 w:1)
	/// Proof: `Erc20Peg::Erc20Meta` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	fn set_erc20_meta() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 13_596_000 picoseconds.
		Weight::from_parts(13_993_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Erc20Peg::PaymentDelay` (r:0 w:1)
	/// Proof: `Erc20Peg::PaymentDelay` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	fn set_payment_delay() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 24_603_000 picoseconds.
		Weight::from_parts(25_375_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
