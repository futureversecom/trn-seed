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
use crate::Pallet as EVMChainId;

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
use frame_support::traits::Get;
use frame_system::RawOrigin;

benchmarks! {
	set_chain_id {
		assert_eq!(ChainId::<T>::get(), T::DefaultChainId::get());
	}: _(RawOrigin::Root, 1234)
	verify {
		assert_eq!(ChainId::<T>::get(), 1234);
	}

}

impl_benchmark_test_suite!(EVMChainId, crate::mock::TestExt::default().build(), crate::mock::Test);
