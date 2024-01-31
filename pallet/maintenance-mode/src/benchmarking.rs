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

//! Maintenance Mode benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
#[allow(unused_imports)]
use crate::Pallet as MaintenanceMode;
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;
use sp_runtime::BoundedVec;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

/// Helper function for creating the pallet/ call name type
pub fn bounded_string<T: Config>(name: &str) -> BoundedVec<u8, <T as Config>::StringLimit> {
	BoundedVec::truncate_from(name.as_bytes().to_vec())
}

benchmarks! {
	enable_maintenance_mode {
		let enabled = true;
	}: _(RawOrigin::Root, enabled)
	verify {
		assert!(MaintenanceModeActive::<T>::get());
	}

	block_account {
		let account = account::<T>("Bob");
		let enabled = true;
	}: _(RawOrigin::Root, account.clone(), enabled)
	verify {
		assert!(BlockedAccounts::<T>::get(account));
	}

	block_evm_target {
		let target_address = H160::default();
		let enabled = true;
	}: _(RawOrigin::Root, target_address.clone(), enabled)
	verify {
		assert!(BlockedEVMAddresses::<T>::get(target_address));
	}

	block_call {
		let pallet = bounded_string::<T>("system");
		let call = bounded_string::<T>("remark");
		let enabled = true;
	}: _(RawOrigin::Root, pallet.clone(), call.clone(), enabled)
	verify {
		assert!(BlockedCalls::<T>::get((pallet, call)));
	}

	block_pallet {
		let pallet = bounded_string::<T>("system");
		let enabled = true;
	}: _(RawOrigin::Root, pallet.clone(), enabled)
	verify {
		BlockedPallets::<T>::get(pallet);
	}
}

impl_benchmark_test_suite!(
	MaintenanceMode,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
