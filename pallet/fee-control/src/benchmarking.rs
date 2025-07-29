// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

#![cfg(feature = "runtime-benchmarks")]

use super::*;

#[allow(unused_imports)]
use crate::Pallet as FeeControl;

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;
use pallet_transaction_payment::Multiplier;
use seed_primitives::Balance;
use sp_core::U256;
use sp_runtime::{traits::One, FixedPointNumber, Perbill};

benchmarks! {
	set_evm_base_fee {
		let value = U256::from(12345u64);
	}: _(RawOrigin::Root, value)
	verify {
		assert_eq!(Data::<T>::get().evm_base_fee_per_gas, value);
	}

	set_weight_multiplier {
		let value = 500000u32;
	}: _(RawOrigin::Root, value)
	verify {
		assert_eq!(Data::<T>::get().weight_multiplier, value);
	}

	set_length_multiplier {
		let value = Balance::from(123u32);
	}: _(RawOrigin::Root, value)
	verify {
		assert_eq!(Data::<T>::get().length_multiplier, value);
	}

	set_minimum_multiplier {
		let numerator = 250_000_000u128; // 25%
		let expected_multiplier = Multiplier::saturating_from_rational(numerator, 1_000_000_000u128);
	}: _(RawOrigin::Root, numerator)
	verify {
		assert_eq!(Data::<T>::get().minimum_multiplier, expected_multiplier);
	}
}

impl_benchmark_test_suite!(
	FeeControl,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
