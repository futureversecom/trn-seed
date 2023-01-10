// /* Copyright 2019-2021 Centrality Investments Limited
// *
// * Licensed under the LGPL, Version 3.0 (the "License");
// * you may not use this file except in compliance with the License.
// * Unless required by applicable law or agreed to in writing, software
// * distributed under the License is distributed on an "AS IS" BASIS,
// * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// * See the License for the specific language governing permissions and
// * limitations under the License.
// * You may obtain a copy of the License at the root of this project source code,
// * or at:
// * https://centrality.ai/licenses/gplv3.txt
// * https://centrality.ai/licenses/lgplv3.txt
// */
use super::*;

use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;

#[allow(unused_imports)]
use crate::Pallet as Echo;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

benchmarks! {
	ping {
		let alice = account::<T>("Alice");
		let destination = account::<T>("Bob").into();

		let expected_session_id = NextSessionId::<T>::get() + 1;
	}: _(origin::<T>(&alice), destination)
	verify {
		let actual_session_id = NextSessionId::<T>::get();
		assert_eq!(actual_session_id, expected_session_id);
	}
}

impl_benchmark_test_suite!(Echo, crate::mock::new_test_ext(), crate::mock::TestRuntime,);
