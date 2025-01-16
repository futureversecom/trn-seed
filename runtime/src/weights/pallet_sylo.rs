
//! Autogenerated weights for `pallet_sylo`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2025-01-16, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
// --pallet=pallet-sylo
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_sylo.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_sylo`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_sylo::WeightInfo for WeightInfo<T> {
	/// Storage: `Sylo::SyloAssetId` (r:0 w:1)
	/// Proof: `Sylo::SyloAssetId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn set_payment_asset() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 24_224_000 picoseconds.
		Weight::from_parts(25_097_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sylo::SyloResolverMethod` (r:0 w:1)
	/// Proof: `Sylo::SyloResolverMethod` (`max_values`: Some(1), `max_size`: Some(502), added: 997, mode: `MaxEncodedLen`)
	fn set_sylo_resolver_method() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 25_480_000 picoseconds.
		Weight::from_parts(26_580_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sylo::Resolvers` (r:1 w:1)
	/// Proof: `Sylo::Resolvers` (`max_values`: None, `max_size`: Some(5551), added: 8026, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 10]`.
	fn register_resolver(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `175`
		//  Estimated: `9016`
		// Minimum execution time: 40_189_000 picoseconds.
		Weight::from_parts(40_826_395, 0)
			.saturating_add(Weight::from_parts(0, 9016))
			// Standard Error: 16_717
			.saturating_add(Weight::from_parts(2_427_734, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sylo::Resolvers` (r:1 w:1)
	/// Proof: `Sylo::Resolvers` (`max_values`: None, `max_size`: Some(5551), added: 8026, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 10]`.
	fn update_resolver(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `289`
		//  Estimated: `9016`
		// Minimum execution time: 41_635_000 picoseconds.
		Weight::from_parts(42_559_132, 0)
			.saturating_add(Weight::from_parts(0, 9016))
			// Standard Error: 19_527
			.saturating_add(Weight::from_parts(2_447_564, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sylo::Resolvers` (r:1 w:1)
	/// Proof: `Sylo::Resolvers` (`max_values`: None, `max_size`: Some(5551), added: 8026, mode: `MaxEncodedLen`)
	fn deregister_resolver() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `289`
		//  Estimated: `9016`
		// Minimum execution time: 43_117_000 picoseconds.
		Weight::from_parts(43_972_000, 0)
			.saturating_add(Weight::from_parts(0, 9016))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sylo::ValidationRecords` (r:1 w:1)
	/// Proof: `Sylo::ValidationRecords` (`max_values`: None, `max_size`: Some(19724), added: 22199, mode: `MaxEncodedLen`)
	/// Storage: `Sylo::SyloResolverMethod` (r:1 w:0)
	/// Proof: `Sylo::SyloResolverMethod` (`max_values`: Some(1), `max_size`: Some(502), added: 997, mode: `MaxEncodedLen`)
	/// The range of component `q` is `[1, 10]`.
	/// The range of component `r` is `[1, 10]`.
	fn create_validation_record(q: u32, r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `201 + q * (22 ±0)`
		//  Estimated: `23189`
		// Minimum execution time: 59_181_000 picoseconds.
		Weight::from_parts(52_554_909, 0)
			.saturating_add(Weight::from_parts(0, 23189))
			// Standard Error: 27_986
			.saturating_add(Weight::from_parts(2_621_417, 0).saturating_mul(q.into()))
			// Standard Error: 27_986
			.saturating_add(Weight::from_parts(1_213_630, 0).saturating_mul(r.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sylo::ValidationRecords` (r:1 w:1)
	/// Proof: `Sylo::ValidationRecords` (`max_values`: None, `max_size`: Some(19724), added: 22199, mode: `MaxEncodedLen`)
	fn add_validation_record_entry() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `317`
		//  Estimated: `23189`
		// Minimum execution time: 48_869_000 picoseconds.
		Weight::from_parts(49_821_000, 0)
			.saturating_add(Weight::from_parts(0, 23189))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sylo::ValidationRecords` (r:1 w:1)
	/// Proof: `Sylo::ValidationRecords` (`max_values`: None, `max_size`: Some(19724), added: 22199, mode: `MaxEncodedLen`)
	/// Storage: `Sylo::SyloResolverMethod` (r:1 w:0)
	/// Proof: `Sylo::SyloResolverMethod` (`max_values`: Some(1), `max_size`: Some(502), added: 997, mode: `MaxEncodedLen`)
	/// The range of component `q` is `[1, 10]`.
	/// The range of component `r` is `[1, 10]`.
	fn update_validation_record(q: u32, r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `339 + q * (22 ±0)`
		//  Estimated: `23189`
		// Minimum execution time: 75_749_000 picoseconds.
		Weight::from_parts(47_859_866, 0)
			.saturating_add(Weight::from_parts(0, 23189))
			// Standard Error: 34_288
			.saturating_add(Weight::from_parts(7_561_509, 0).saturating_mul(q.into()))
			// Standard Error: 34_288
			.saturating_add(Weight::from_parts(2_809_128, 0).saturating_mul(r.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Sylo::ValidationRecords` (r:1 w:1)
	/// Proof: `Sylo::ValidationRecords` (`max_values`: None, `max_size`: Some(19724), added: 22199, mode: `MaxEncodedLen`)
	fn delete_validation_record() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `276`
		//  Estimated: `23189`
		// Minimum execution time: 43_446_000 picoseconds.
		Weight::from_parts(44_252_000, 0)
			.saturating_add(Weight::from_parts(0, 23189))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
