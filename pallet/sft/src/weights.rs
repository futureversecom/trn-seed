
//! Autogenerated weights for `pallet_sft`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-04-13, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `Jasons-PC`, CPU: `AMD Ryzen 7 3800X 8-Core Processor`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain
// dev
// --steps=50
// --repeat=20
// --pallet=pallet_sft
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./output

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};

/// Weight functions needed for pallet_sft.
pub trait WeightInfo {
    fn create_collection() -> Weight;
    fn create_token() -> Weight;
    fn mint() -> Weight;
    fn transfer() -> Weight;
    fn burn() -> Weight;
    fn set_owner() -> Weight;
    fn set_max_issuance() -> Weight;
    fn set_base_uri() -> Weight;
}

// For backwards compatibility and tests
impl WeightInfo for () {
    // Storage: Nft NextCollectionId (r:1 w:1)
    // Storage: EVM AccountCodes (r:1 w:1)
    // Storage: System Account (r:1 w:1)
    // Storage: Sft SftCollectionInfo (r:0 w:1)
    fn create_collection() -> Weight {
        (44_824_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(3 as Weight))
            .saturating_add(RocksDbWeight::get().writes(4 as Weight))
    }
    // Storage: Sft SftCollectionInfo (r:1 w:1)
    // Storage: Sft TokenInfo (r:0 w:1)
    fn create_token() -> Weight {
        (29_756_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(1 as Weight))
            .saturating_add(RocksDbWeight::get().writes(2 as Weight))
    }
    // Storage: Sft SftCollectionInfo (r:1 w:0)
    // Storage: Sft TokenInfo (r:1 w:1)
    fn mint() -> Weight {
        (30_698_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(2 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    // Storage: Sft TokenInfo (r:1 w:1)
    fn transfer() -> Weight {
        (27_583_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(1 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    // Storage: Sft TokenInfo (r:1 w:1)
    fn burn() -> Weight {
        (29_236_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(1 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    // Storage: Sft SftCollectionInfo (r:1 w:1)
    fn set_owner() -> Weight {
        (25_348_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(1 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    // Storage: Sft SftCollectionInfo (r:1 w:0)
    // Storage: Sft TokenInfo (r:1 w:1)
    fn set_max_issuance() -> Weight {
        (27_462_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(2 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    // Storage: Sft SftCollectionInfo (r:1 w:1)
    fn set_base_uri() -> Weight {
        (23_354_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(1 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
}
