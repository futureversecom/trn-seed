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

use super::*;

use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::traits::OnInitialize;
use frame_system::RawOrigin;

#[allow(unused_imports)]
use crate::Pallet as StakingPayouts;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

// benchmarks! {
// 	// 	let alice = account::<T>("Alice");
// 	// 	let destination = account::<T>("Bob").into();

// 	// 	let expected_session_id = NextSessionId::<T>::get() + 1;
// 	// }: on_initialize(1)
// 	on_initialize_external {
// 		// let block_number: T::BlockNumber = T::BlockNumber(1);
// 		start_active_era(2);
// 	}: { StakingPayouts::<T>::on_initialize(1u32.into()) }
// }

impl_benchmark_test_suite!(StakingPayouts, crate::mock::new_test_ext(), crate::mock::TestRuntime,);
