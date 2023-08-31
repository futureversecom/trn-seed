#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::traits::{One, StaticLookup, TrailingZeroInput};
use sp_std::{prelude::*, vec};

use crate::StakingPayouts;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_support::{
	codec::Decode,
	pallet_prelude::*,
	traits::{Get, KeyOwnerProofSystem, OnInitialize},
};
use frame_system::{pallet_prelude::*, RawOrigin};

use crate::tests;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	// pub trait Config: pallet_staking_payouts::Config + pallet_staking::Config
	pub trait Config: frame_system::Config {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn benchmarking_support_staking_payouts_on_initialize(
			origin: OriginFor<T>,
		) -> DispatchResult {
			Ok(())
		}
	}

	benchmarks! {
		on_initialize {
			let caller: T::AccountId = whitelisted_caller();
			start_active_era(1);
		}: { <StakingPayouts as OnInitialize<u32>>::on_initialize(1_u32.into()) }
		// }: { on_initialize(1_u32.into()) }
	}
}

// impl<T: Config> OnInitialize<T::BlockNumber> for Pallet<T> {
// 	fn on_initialize(n: T::BlockNumber) -> frame_support::weights::Weight {
// 		pallet_staking_payouts::Pallet::<T>::on_initialize(n)
// 	}
// }

// Benchmark requires `Call`, `Config`, and `Pallet`, all defined in the local crate
// use crate::Call;

// use pallet::{Config, Pallet};
