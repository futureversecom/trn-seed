
//! Autogenerated weights for `pallet_sylo_data_verification`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2025-04-08, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
// --pallet=pallet-sylo-data-verification
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_sylo_data_verification.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_sylo_data_verification`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_sylo_data_verification::WeightInfo for WeightInfo<T> {
	/// Storage: `SyloDataVerification::SyloAssetId` (r:0 w:1)
	/// Proof: `SyloDataVerification::SyloAssetId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn set_payment_asset() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 23_143_000 picoseconds.
		Weight::from_parts(23_749_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `SyloDataVerification::SyloResolverMethod` (r:0 w:1)
	/// Proof: `SyloDataVerification::SyloResolverMethod` (`max_values`: Some(1), `max_size`: Some(502), added: 997, mode: `MaxEncodedLen`)
	fn set_sylo_resolver_method() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 24_780_000 picoseconds.
		Weight::from_parts(25_308_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `SyloDataVerification::Resolvers` (r:1 w:1)
	/// Proof: `SyloDataVerification::Resolvers` (`max_values`: None, `max_size`: Some(5551), added: 8026, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 10]`.
	fn register_resolver(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `76`
		//  Estimated: `9016`
		// Minimum execution time: 37_742_000 picoseconds.
		Weight::from_parts(38_485_785, 0)
			.saturating_add(Weight::from_parts(0, 9016))
			// Standard Error: 20_011
			.saturating_add(Weight::from_parts(2_354_522, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `SyloDataVerification::Resolvers` (r:1 w:1)
	/// Proof: `SyloDataVerification::Resolvers` (`max_values`: None, `max_size`: Some(5551), added: 8026, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 10]`.
	fn update_resolver(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `190`
		//  Estimated: `9016`
		// Minimum execution time: 39_015_000 picoseconds.
		Weight::from_parts(39_804_674, 0)
			.saturating_add(Weight::from_parts(0, 9016))
			// Standard Error: 19_535
			.saturating_add(Weight::from_parts(2_376_451, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `SyloDataVerification::Resolvers` (r:1 w:1)
	/// Proof: `SyloDataVerification::Resolvers` (`max_values`: None, `max_size`: Some(5551), added: 8026, mode: `MaxEncodedLen`)
	fn deregister_resolver() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `190`
		//  Estimated: `9016`
		// Minimum execution time: 40_418_000 picoseconds.
		Weight::from_parts(41_322_000, 0)
			.saturating_add(Weight::from_parts(0, 9016))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `SyloDataVerification::ValidationRecords` (r:1 w:1)
	/// Proof: `SyloDataVerification::ValidationRecords` (`max_values`: None, `max_size`: Some(19724), added: 22199, mode: `MaxEncodedLen`)
	/// Storage: `SyloDataVerification::SyloResolverMethod` (r:1 w:0)
	/// Proof: `SyloDataVerification::SyloResolverMethod` (`max_values`: Some(1), `max_size`: Some(502), added: 997, mode: `MaxEncodedLen`)
	/// The range of component `q` is `[1, 10]`.
	/// The range of component `r` is `[1, 10]`.
	fn create_validation_record(q: u32, r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `102 + q * (22 ±0)`
		//  Estimated: `23189`
		// Minimum execution time: 55_824_000 picoseconds.
		Weight::from_parts(53_527_362, 0)
			.saturating_add(Weight::from_parts(0, 23189))
			// Standard Error: 134_683
			.saturating_add(Weight::from_parts(1_999_735, 0).saturating_mul(q.into()))
			// Standard Error: 134_683
			.saturating_add(Weight::from_parts(1_264_339, 0).saturating_mul(r.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `SyloDataVerification::ValidationRecords` (r:1 w:1)
	/// Proof: `SyloDataVerification::ValidationRecords` (`max_values`: None, `max_size`: Some(19724), added: 22199, mode: `MaxEncodedLen`)
	fn add_validation_record_entry() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `218`
		//  Estimated: `23189`
		// Minimum execution time: 44_595_000 picoseconds.
		Weight::from_parts(45_902_000, 0)
			.saturating_add(Weight::from_parts(0, 23189))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `SyloDataVerification::ValidationRecords` (r:1 w:1)
	/// Proof: `SyloDataVerification::ValidationRecords` (`max_values`: None, `max_size`: Some(19724), added: 22199, mode: `MaxEncodedLen`)
	/// Storage: `SyloDataVerification::SyloResolverMethod` (r:1 w:0)
	/// Proof: `SyloDataVerification::SyloResolverMethod` (`max_values`: Some(1), `max_size`: Some(502), added: 997, mode: `MaxEncodedLen`)
	/// The range of component `q` is `[1, 10]`.
	/// The range of component `r` is `[1, 10]`.
	fn update_validation_record(q: u32, r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `240 + q * (22 ±0)`
		//  Estimated: `23189`
		// Minimum execution time: 71_920_000 picoseconds.
		Weight::from_parts(44_928_122, 0)
			.saturating_add(Weight::from_parts(0, 23189))
			// Standard Error: 37_335
			.saturating_add(Weight::from_parts(7_646_201, 0).saturating_mul(q.into()))
			// Standard Error: 37_335
			.saturating_add(Weight::from_parts(2_588_016, 0).saturating_mul(r.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `SyloDataVerification::ValidationRecords` (r:1 w:1)
	/// Proof: `SyloDataVerification::ValidationRecords` (`max_values`: None, `max_size`: Some(19724), added: 22199, mode: `MaxEncodedLen`)
	fn delete_validation_record() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `177`
		//  Estimated: `23189`
		// Minimum execution time: 40_103_000 picoseconds.
		Weight::from_parts(41_004_000, 0)
			.saturating_add(Weight::from_parts(0, 23189))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
