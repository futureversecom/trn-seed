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
use seed_primitives::Balance;
use sp_core::U256;
use sp_runtime::traits::One;

benchmarks! {
	set_evm_base_fee {
	}: _(RawOrigin::Root, U256::one())

	set_weight_multiplier {
	}: _(RawOrigin::Root, Perbill::one())

	set_length_multiplier {
	}: _(RawOrigin::Root, Balance::one())
}

impl_benchmark_test_suite!(
	FeeControl,
	seed_primitives::test_utils::test_ext::<crate::mock::Test>().build(),
	crate::mock::Test
);
