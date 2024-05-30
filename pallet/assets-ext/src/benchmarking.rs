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
use frame_support::assert_ok;
use frame_system::RawOrigin;

use crate::Pallet as AssetsExt;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

benchmarks! {

	set_asset_deposit {
	}: _(RawOrigin::Root, One::one())
	verify {
		assert_eq!(AssetDeposit::<T>::get(), One::one());
	}

	create_asset {
		let alice = account::<T>("Alice");
		assert_ok!(AssetsExt::<T>::mint_into(T::NativeAssetId::get(), &alice, 1000000u32.into()));
		let name = "Marko".into();
		let symbol = "MRK".into();
		let decimals = 8;
		let min_balance = None;
		let owner = Some(alice.clone());
	}: _(origin::<T>(&alice), name, symbol, decimals, min_balance, owner)

	mint {
		let alice = account::<T>("Alice");
		let account_lookup = T::Lookup::unlookup(alice.clone());
		let usdc = AssetsExt::<T>::create(&alice, None).unwrap();
	}: _(origin::<T>(&alice), usdc, account_lookup, 1_500_000)
	verify {
		assert_eq!(AssetsExt::<T>::balance(usdc, &alice), 1_500_000);
	}

	transfer {
		let alice = account::<T>("Alice");
		let usdc = AssetsExt::<T>::create(&alice, None).unwrap();
		let bob = account::<T>("Bob");
		let account_lookup = T::Lookup::unlookup(bob.clone());
		assert_ok!(AssetsExt::<T>::mint_into(usdc, &alice, 1_500_000u32.into()));
	}: _(origin::<T>(&alice), usdc, bob.clone(), 1_500_000, false)
	verify {
		assert_eq!(AssetsExt::<T>::balance(usdc, &alice), 0);
		assert_eq!(AssetsExt::<T>::balance(usdc, &bob), 1_500_000);
	}

	burn_from {
		let alice = account::<T>("Alice");
		let usdc = AssetsExt::<T>::create(&alice, None).unwrap();
		assert_ok!(AssetsExt::<T>::mint_into(usdc, &alice, 1_500_000u32.into()));
	}: _(origin::<T>(&alice), usdc, T::Lookup::unlookup(alice.clone()), 1_000_000)
	verify {
		assert_eq!(AssetsExt::<T>::balance(usdc, &alice), 500_000);
	}
}

impl_benchmark_test_suite!(
	AssetsExt,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
