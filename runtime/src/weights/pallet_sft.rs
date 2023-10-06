//! Autogenerated weights for `pallet_sft`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-08-18, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-101-56`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_sft
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./output/pallet_sft.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_sft`.
pub struct WeightInfo<T>(PhantomData<T>);

impl<T: frame_system::Config> pallet_sft::WeightInfo for WeightInfo<T> {
    // Storage: Nft NextCollectionId (r:1 w:1)
    // Storage: EVM AccountCodes (r:1 w:1)
    // Storage: Futurepass DefaultProxy (r:1 w:0)
    // Storage: System Account (r:1 w:1)
    // Storage: Sft SftCollectionInfo (r:0 w:1)
    fn create_collection() -> Weight {
        Weight::from_ref_time(50_786_000 as u64)
            .saturating_add(T::DbWeight::get().reads(4 as u64))
            .saturating_add(T::DbWeight::get().writes(4 as u64))
    }
    // Storage: Sft SftCollectionInfo (r:1 w:1)
    // Storage: Sft TokenInfo (r:0 w:1)
    fn create_token() -> Weight {
        Weight::from_ref_time(33_744_000 as u64)
            .saturating_add(T::DbWeight::get().reads(1 as u64))
            .saturating_add(T::DbWeight::get().writes(2 as u64))
    }
    // Storage: Sft SftCollectionInfo (r:1 w:0)
    // Storage: Sft TokenInfo (r:1 w:1)
    /// The range of component `p` is `[1, 500]`.
    fn mint(p: u32) -> Weight {
        Weight::from_ref_time(34_185_000 as u64)
            // Standard Error: 30_221
            .saturating_add(Weight::from_ref_time(1_709_631 as u64).saturating_mul(p as u64))
            .saturating_add(T::DbWeight::get().reads(2 as u64))
            .saturating_add(T::DbWeight::get().writes(1 as u64))
    }
    // Storage: Sft TokenInfo (r:1 w:1)
    /// The range of component `p` is `[1, 500]`.
    fn transfer(p: u32) -> Weight {
        Weight::from_ref_time(30_739_000 as u64)
            // Standard Error: 29_508
            .saturating_add(Weight::from_ref_time(1_686_018 as u64).saturating_mul(p as u64))
            .saturating_add(T::DbWeight::get().reads(1 as u64))
            .saturating_add(T::DbWeight::get().writes(1 as u64))
    }
    // Storage: Sft TokenInfo (r:1 w:1)
    /// The range of component `p` is `[1, 500]`.
    fn burn(p: u32) -> Weight {
        Weight::from_ref_time(30_908_000 as u64)
            // Standard Error: 29_520
            .saturating_add(Weight::from_ref_time(1_713_376 as u64).saturating_mul(p as u64))
            .saturating_add(T::DbWeight::get().reads(1 as u64))
            .saturating_add(T::DbWeight::get().writes(1 as u64))
    }
    // Storage: Sft SftCollectionInfo (r:1 w:1)
    fn set_owner() -> Weight {
        Weight::from_ref_time(28_013_000 as u64)
            .saturating_add(T::DbWeight::get().reads(1 as u64))
            .saturating_add(T::DbWeight::get().writes(1 as u64))
    }
    // Storage: Sft SftCollectionInfo (r:1 w:0)
    // Storage: Sft TokenInfo (r:1 w:1)
    fn set_max_issuance() -> Weight {
        Weight::from_ref_time(30_248_000 as u64)
            .saturating_add(T::DbWeight::get().reads(2 as u64))
            .saturating_add(T::DbWeight::get().writes(1 as u64))
    }
    // Storage: Sft SftCollectionInfo (r:1 w:1)
    fn set_base_uri() -> Weight {
        Weight::from_ref_time(26_530_000 as u64)
            .saturating_add(T::DbWeight::get().reads(1 as u64))
            .saturating_add(T::DbWeight::get().writes(1 as u64))
    }
    // Storage: Sft SftCollectionInfo (r:1 w:1)
    fn set_name() -> Weight {
        Weight::from_ref_time(26_831_000 as u64)
            .saturating_add(T::DbWeight::get().reads(1 as u64))
            .saturating_add(T::DbWeight::get().writes(1 as u64))
    }
}
