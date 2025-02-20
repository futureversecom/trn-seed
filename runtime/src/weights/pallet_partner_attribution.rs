
//! Autogenerated weights for `pallet_partner_attribution`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2025-02-17, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
// --pallet=pallet-partner-attribution
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./runtime/src/weights/pallet_partner_attribution.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_partner_attribution`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_partner_attribution::WeightInfo for WeightInfo<T> {
	/// Storage: `PartnerAttribution::NextPartnerId` (r:1 w:1)
	/// Proof: `PartnerAttribution::NextPartnerId` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `PartnerAttribution::Partners` (r:0 w:1)
	/// Proof: `PartnerAttribution::Partners` (`max_values`: None, `max_size`: Some(85), added: 2560, mode: `MaxEncodedLen`)
	fn register_partner_account() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `76`
		//  Estimated: `1501`
		// Minimum execution time: 34_068_000 picoseconds.
		Weight::from_parts(34_624_000, 0)
			.saturating_add(Weight::from_parts(0, 1501))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `PartnerAttribution::Partners` (r:1 w:1)
	/// Proof: `PartnerAttribution::Partners` (`max_values`: None, `max_size`: Some(85), added: 2560, mode: `MaxEncodedLen`)
	fn update_partner_account() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `218`
		//  Estimated: `3550`
		// Minimum execution time: 39_572_000 picoseconds.
		Weight::from_parts(40_566_000, 0)
			.saturating_add(Weight::from_parts(0, 3550))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `PartnerAttribution::Partners` (r:1 w:0)
	/// Proof: `PartnerAttribution::Partners` (`max_values`: None, `max_size`: Some(85), added: 2560, mode: `MaxEncodedLen`)
	/// Storage: `PartnerAttribution::Attributions` (r:1 w:1)
	/// Proof: `PartnerAttribution::Attributions` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	fn attribute_account() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `218`
		//  Estimated: `3550`
		// Minimum execution time: 42_003_000 picoseconds.
		Weight::from_parts(42_469_000, 0)
			.saturating_add(Weight::from_parts(0, 3550))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `PartnerAttribution::Partners` (r:1 w:1)
	/// Proof: `PartnerAttribution::Partners` (`max_values`: None, `max_size`: Some(85), added: 2560, mode: `MaxEncodedLen`)
	fn upgrade_partner() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `218`
		//  Estimated: `3550`
		// Minimum execution time: 36_887_000 picoseconds.
		Weight::from_parts(37_756_000, 0)
			.saturating_add(Weight::from_parts(0, 3550))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `PartnerAttribution::Partners` (r:1 w:0)
	/// Proof: `PartnerAttribution::Partners` (`max_values`: None, `max_size`: Some(85), added: 2560, mode: `MaxEncodedLen`)
	/// Storage: `Futurepass::Holders` (r:1 w:1)
	/// Proof: `Futurepass::Holders` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `Futurepass::NextFuturepassId` (r:1 w:1)
	/// Proof: `Futurepass::NextFuturepassId` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `Proxy::Proxies` (r:1 w:1)
	/// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(845), added: 3320, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	/// Storage: `PartnerAttribution::Attributions` (r:1 w:1)
	/// Proof: `PartnerAttribution::Attributions` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	fn create_futurepass_with_partner() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `570`
		//  Estimated: `6172`
		// Minimum execution time: 216_068_000 picoseconds.
		Weight::from_parts(218_308_000, 0)
			.saturating_add(Weight::from_parts(0, 6172))
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(6))
	}
}
