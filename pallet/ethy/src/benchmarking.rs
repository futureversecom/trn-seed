#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as Ethy;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;
use sp_std::prelude::*;

benchmarks! {
	set_ethy_state {
		assert_eq!(EthyState::<T>::get(), State::Active); // Active is the default for State

	}: _(RawOrigin::Root, State::Paused)
	verify {
		assert_eq!(EthyState::<T>::get(), State::Paused);
	}
}

impl_benchmark_test_suite!(
	Ethy,
	crate::mock::ExtBuilder::default().build(),
	crate::mock::TestRuntime
);
