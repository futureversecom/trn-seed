
//! Autogenerated weights for `pallet_election_provider_multi_phase`
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
// --pallet=pallet_election_provider_multi_phase
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./output/pallet_election_provider_multi_phase.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_election_provider_multi_phase`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_election_provider_multi_phase::WeightInfo for WeightInfo<T> {
	// Storage: Staking CurrentEra (r:1 w:0)
	// Storage: Staking CurrentPlannedSession (r:1 w:0)
	// Storage: Staking ErasStartSessionIndex (r:1 w:0)
	// Storage: Babe EpochIndex (r:1 w:0)
	// Storage: Babe GenesisSlot (r:1 w:0)
	// Storage: Babe CurrentSlot (r:1 w:0)
	// Storage: Staking ForceEra (r:1 w:0)
	// Storage: ElectionProviderMultiPhase CurrentPhase (r:1 w:0)
	fn on_initialize_nothing() -> Weight {
		Weight::from_all(42_951_000 as u64)
			.saturating_add(T::DbWeight::get().reads(8 as u64))
	}
	// Storage: ElectionProviderMultiPhase Round (r:1 w:0)
	// Storage: ElectionProviderMultiPhase CurrentPhase (r:0 w:1)
	fn on_initialize_open_signed() -> Weight {
		Weight::from_all(45_085_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: ElectionProviderMultiPhase Round (r:1 w:0)
	// Storage: ElectionProviderMultiPhase CurrentPhase (r:0 w:1)
	fn on_initialize_open_unsigned() -> Weight {
		Weight::from_all(44_953_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: System Account (r:1 w:1)
	// Storage: ElectionProviderMultiPhase QueuedSolution (r:0 w:1)
	fn finalize_signed_phase_accept_solution() -> Weight {
		Weight::from_all(86_504_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: System Account (r:2 w:2)
	fn finalize_signed_phase_reject_solution() -> Weight {
		Weight::from_all(102_875_000 as u64)
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: ElectionProviderMultiPhase SnapshotMetadata (r:0 w:1)
	// Storage: ElectionProviderMultiPhase DesiredTargets (r:0 w:1)
	// Storage: ElectionProviderMultiPhase Snapshot (r:0 w:1)
	/// The range of component `v` is `[1000, 2000]`.
	/// The range of component `t` is `[500, 1000]`.
	fn create_snapshot_internal(v: u32, _t: u32, ) -> Weight {
		Weight::from_all(940_302_000 as u64)
			// Standard Error: 4_670
			.saturating_add(Weight::from_all(324_279 as u64).saturating_mul(v as u64))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
	}
	// Storage: ElectionProviderMultiPhase SignedSubmissionIndices (r:1 w:1)
	// Storage: ElectionProviderMultiPhase SignedSubmissionNextIndex (r:1 w:1)
	// Storage: ElectionProviderMultiPhase SnapshotMetadata (r:1 w:1)
	// Storage: ElectionProviderMultiPhase SignedSubmissionsMap (r:1 w:0)
	// Storage: System BlockWeight (r:1 w:1)
	// Storage: ElectionProviderMultiPhase QueuedSolution (r:1 w:1)
	// Storage: ElectionProviderMultiPhase Round (r:1 w:1)
	// Storage: ElectionProviderMultiPhase DesiredTargets (r:0 w:1)
	// Storage: ElectionProviderMultiPhase Snapshot (r:0 w:1)
	// Storage: ElectionProviderMultiPhase CurrentPhase (r:0 w:1)
	/// The range of component `a` is `[500, 800]`.
	/// The range of component `d` is `[200, 400]`.
	fn elect_queued(a: u32, _d: u32, ) -> Weight {
		Weight::from_all(1_323_337_000 as u64)
			// Standard Error: 8_871
			.saturating_add(Weight::from_all(624_058 as u64).saturating_mul(a as u64))
			.saturating_add(T::DbWeight::get().reads(7 as u64))
			.saturating_add(T::DbWeight::get().writes(9 as u64))
	}
	// Storage: ElectionProviderMultiPhase CurrentPhase (r:1 w:0)
	// Storage: ElectionProviderMultiPhase SnapshotMetadata (r:1 w:0)
	// Storage: FeeControl Data (r:1 w:0)
	// Storage: TransactionPayment NextFeeMultiplier (r:1 w:0)
	// Storage: ElectionProviderMultiPhase SignedSubmissionIndices (r:1 w:1)
	// Storage: ElectionProviderMultiPhase SignedSubmissionNextIndex (r:1 w:1)
	// Storage: ElectionProviderMultiPhase SignedSubmissionsMap (r:0 w:1)
	fn submit() -> Weight {
		Weight::from_all(151_199_000 as u64)
			.saturating_add(T::DbWeight::get().reads(6 as u64))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
	}
	// Storage: ElectionProviderMultiPhase CurrentPhase (r:1 w:0)
	// Storage: ElectionProviderMultiPhase Round (r:1 w:0)
	// Storage: ElectionProviderMultiPhase DesiredTargets (r:1 w:0)
	// Storage: ElectionProviderMultiPhase QueuedSolution (r:1 w:1)
	// Storage: ElectionProviderMultiPhase SnapshotMetadata (r:1 w:0)
	// Storage: ElectionProviderMultiPhase MinimumUntrustedScore (r:1 w:0)
	// Storage: ElectionProviderMultiPhase Snapshot (r:1 w:0)
	/// The range of component `v` is `[1000, 2000]`.
	/// The range of component `t` is `[500, 1000]`.
	/// The range of component `a` is `[500, 800]`.
	/// The range of component `d` is `[200, 400]`.
	fn submit_unsigned(v: u32, _t: u32, a: u32, _d: u32, ) -> Weight {
		Weight::from_all(11_261_392_000 as u64)
			// Standard Error: 33_342
			.saturating_add(Weight::from_all(1_146_340 as u64).saturating_mul(v as u64))
			// Standard Error: 98_805
			.saturating_add(Weight::from_all(5_995_651 as u64).saturating_mul(a as u64))
			.saturating_add(T::DbWeight::get().reads(7 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: ElectionProviderMultiPhase Round (r:1 w:0)
	// Storage: ElectionProviderMultiPhase DesiredTargets (r:1 w:0)
	// Storage: ElectionProviderMultiPhase MinimumUntrustedScore (r:1 w:0)
	// Storage: ElectionProviderMultiPhase Snapshot (r:1 w:0)
	/// The range of component `v` is `[1000, 2000]`.
	/// The range of component `t` is `[500, 1000]`.
	/// The range of component `a` is `[500, 800]`.
	/// The range of component `d` is `[200, 400]`.
	fn feasibility_check(v: u32, _t: u32, a: u32, _d: u32, ) -> Weight {
		Weight::from_all(9_990_998_000 as u64)
			// Standard Error: 26_953
			.saturating_add(Weight::from_all(911_703 as u64).saturating_mul(v as u64))
			// Standard Error: 79_872
			.saturating_add(Weight::from_all(5_176_479 as u64).saturating_mul(a as u64))
			.saturating_add(T::DbWeight::get().reads(4 as u64))
	}
}
