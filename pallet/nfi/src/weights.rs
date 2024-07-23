
//! Autogenerated weights for `pallet_nfi`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-07-17, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `Jasons-Ubuntu`, CPU: `AMD Ryzen 9 7950X 16-Core Processor`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain
// dev
// --steps=50
// --repeat=20
// --pallet=pallet-nfi
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./output

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use frame_support::weights::constants::RocksDbWeight;
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_nft.
pub trait WeightInfo {
    fn set_relayer() -> Weight;
    fn set_fee_to() -> Weight;
    fn set_fee_details() -> Weight;
    fn enable_nfi() -> Weight;
    fn manual_data_request() -> Weight;
    fn submit_nfi_data() -> Weight;
}

/// Weight functions for `pallet_nfi`.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    // Storage: Nfi Relayer (r:0 w:1)
    fn set_relayer() -> Weight {
        Weight::from_all(10_189_000 as u64)
            .saturating_add(T::DbWeight::get().writes(1 as u64))
    }
    // Storage: Nfi FeeTo (r:0 w:1)
    fn set_fee_to() -> Weight {
        Weight::from_all(10_329_000 as u64)
            .saturating_add(T::DbWeight::get().writes(1 as u64))
    }
    // Storage: Nfi MintFee (r:0 w:1)
    fn set_fee_details() -> Weight {
        Weight::from_all(10_610_000 as u64)
            .saturating_add(T::DbWeight::get().writes(1 as u64))
    }
    // Storage: Nft CollectionInfo (r:1 w:0)
    // Storage: Nfi NfiEnabled (r:0 w:1)
    fn enable_nfi() -> Weight {
        Weight::from_all(15_359_000 as u64)
            .saturating_add(T::DbWeight::get().reads(1 as u64))
            .saturating_add(T::DbWeight::get().writes(1 as u64))
    }
    // Storage: Nfi NfiEnabled (r:1 w:0)
    // Storage: Nft CollectionInfo (r:1 w:0)
    // Storage: Nfi MintFee (r:1 w:0)
    fn manual_data_request() -> Weight {
        Weight::from_all(18_365_000 as u64)
            .saturating_add(T::DbWeight::get().reads(3 as u64))
    }
    // Storage: Nfi Relayer (r:1 w:0)
    // Storage: Nfi NfiEnabled (r:1 w:0)
    // Storage: Nft CollectionInfo (r:1 w:0)
    // Storage: Nfi NfiData (r:0 w:1)
    fn submit_nfi_data() -> Weight {
        Weight::from_all(20_548_000 as u64)
            .saturating_add(T::DbWeight::get().reads(3 as u64))
            .saturating_add(T::DbWeight::get().writes(1 as u64))
    }
}

impl WeightInfo for () {
    // Storage: Nfi Relayer (r:0 w:1)
    fn set_relayer() -> Weight {
        Weight::from_all(10_189_000 as u64)
            .saturating_add(RocksDbWeight::get().writes(1 as u64))
    }
    // Storage: Nfi FeeTo (r:0 w:1)
    fn set_fee_to() -> Weight {
        Weight::from_all(10_329_000 as u64)
            .saturating_add(RocksDbWeight::get().writes(1 as u64))
    }
    // Storage: Nfi MintFee (r:0 w:1)
    fn set_fee_details() -> Weight {
        Weight::from_all(10_610_000 as u64)
            .saturating_add(RocksDbWeight::get().writes(1 as u64))
    }
    // Storage: Nft CollectionInfo (r:1 w:0)
    // Storage: Nfi NfiEnabled (r:0 w:1)
    fn enable_nfi() -> Weight {
        Weight::from_all(15_359_000 as u64)
            .saturating_add(RocksDbWeight::get().reads(1 as u64))
            .saturating_add(RocksDbWeight::get().writes(1 as u64))
    }
    // Storage: Nfi NfiEnabled (r:1 w:0)
    // Storage: Nft CollectionInfo (r:1 w:0)
    // Storage: Nfi MintFee (r:1 w:0)
    fn manual_data_request() -> Weight {
        Weight::from_all(18_365_000 as u64)
            .saturating_add(RocksDbWeight::get().reads(3 as u64))
    }
    // Storage: Nfi Relayer (r:1 w:0)
    // Storage: Nfi NfiEnabled (r:1 w:0)
    // Storage: Nft CollectionInfo (r:1 w:0)
    // Storage: Nfi NfiData (r:0 w:1)
    fn submit_nfi_data() -> Weight {
        Weight::from_all(20_548_000 as u64)
            .saturating_add(RocksDbWeight::get().reads(3 as u64))
            .saturating_add(RocksDbWeight::get().writes(1 as u64))
    }
}
