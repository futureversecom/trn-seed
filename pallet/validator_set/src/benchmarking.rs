//! Benchmarking setup for pallet-validator-set
#![allow(unused_imports)]
use super::*;

#[allow(unused_imports)]
use crate::Pallet as ValidatorSet;
#[allow(unused)]
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;

const SEED: u32 = 0;
const MAX_MEMBERS: u32 = 100;

benchmarks! {
	add_validator {
		let m in 1 .. MAX_MEMBERS;
		let validator: T::AccountId = account("soume_account", m, SEED);
	}: add_validator(RawOrigin::Root, validator.clone())
	verify {
		assert!(ValidatorList::<T>::get().contains(&validator));
		// assert_last_event::<T>(Event::ValidatorAdded { m }.into());
	}

	impl_benchmark_test_suite!(
		ValidatorSet,
		crate::mock::new_test_ext(),
		crate::mock::Test
	);
}
