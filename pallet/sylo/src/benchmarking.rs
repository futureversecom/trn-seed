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

use crate::Pallet as Sylo;

use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, BoundedVec};
use frame_system::RawOrigin;

const MAX_SERVICE_ENDPOINTS: u32 = 20;
const STRING_LIMIT: u32 = 1000;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId
where
	T::AccountId: From<H160>,
{
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

benchmarks! {
	where_clause { where <T as frame_system::Config>::AccountId: From<sp_core::H160> + Into<sp_core::H160> }

	register_resolver {
		let p in 1 .. STRING_LIMIT;
		let q in 1 .. MAX_SERVICE_ENDPOINTS;

		let alice = account::<T>("Alice");

		let mut identifier = BoundedVec::new();
		for _ in 1..p {
			identifier.force_push(b'a');
		}

		let mut service_endpoints = BoundedVec::new();
		for _ in 1..q {
			let endpoint = BoundedVec::truncate_from("https://service-endpoint.one.two.three".encode());
			service_endpoints.force_push(endpoint);
		}

	}: _(origin::<T>(&account::<T>("Alice")), identifier, service_endpoints)
}

impl_benchmark_test_suite!(
	Sylo,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
