// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use crate::Pallet as Futurepass;

use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::{assert_ok, traits::fungibles::Mutate};
use frame_system::RawOrigin;
use sp_std::vec;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

// fund account with ROOT & XRP
pub fn fund<T: Config>(account: &T::AccountId) {
	let root_asset_id: u32 = 1;
	assert_ok!(T::MultiCurrency::mint_into(root_asset_id.into(), &account, 1_000_000u32.into()));
}

benchmarks! {

	create {
		let caller: T::AccountId = whitelisted_caller();
		let owner: T::AccountId = account("owner", 0, 0);

		fund::<T>(&caller);
	}: _(RawOrigin::Signed(caller.clone()), owner.clone())
	verify {
		assert_eq!(Holders::<T>::get(owner).is_some(), true);
	}

	register_delegate {
		let owner: T::AccountId = account("account", 0, 0);

		fund::<T>(&owner);
		assert_ok!(Futurepass::<T>::create(RawOrigin::Signed(owner.clone()).into(), owner.clone()));
		let futurepass: T::AccountId = Holders::<T>::get(&owner).unwrap();

		let delegate: T::AccountId = account("delegate", 0, 0);
		let proxy_type = T::ProxyType::default();
	}: _(RawOrigin::Signed(owner.clone()), futurepass.clone(), delegate.clone(), proxy_type.clone())
	verify {
		assert!(T::Proxy::exists(&futurepass, &delegate, Some(proxy_type)));
	}

	unregister_delegate {
		let owner: T::AccountId = account("owner", 0, 0);

		fund::<T>(&owner);
		assert_ok!(Futurepass::<T>::create(RawOrigin::Signed(owner.clone()).into(), owner.clone()));
		let futurepass: T::AccountId = Holders::<T>::get(&owner).unwrap();

		let delegate: T::AccountId = account("delegate", 0, 0);
		let proxy_type = T::ProxyType::default();

		assert_ok!(Futurepass::<T>::register_delegate(RawOrigin::Signed(owner.clone()).into(), futurepass.clone(), delegate.clone(), proxy_type.clone()));
	}: _(RawOrigin::Signed(owner.clone()), futurepass.clone(), delegate.clone())
	verify {
		assert!(!T::Proxy::exists(&futurepass, &delegate, Some(proxy_type)));
	}

	transfer_futurepass {
		let owner: T::AccountId = account("owner", 0, 0);
		let new_owner: T::AccountId = account("new-owner", 0, 0);

		fund::<T>(&owner);
		fund::<T>(&new_owner);
		assert_ok!(Futurepass::<T>::create(RawOrigin::Signed(owner.clone()).into(), owner.clone()));
		let futurepass: T::AccountId = Holders::<T>::get(&owner).unwrap();

	}: _(RawOrigin::Signed(owner), new_owner.clone())
	verify {
		assert_eq!(Holders::<T>::get(new_owner), Some(futurepass));
	}

	proxy_extrinsic {
		let owner: T::AccountId = account("owner", 0, 0);

		fund::<T>(&owner);
		assert_ok!(Futurepass::<T>::create(RawOrigin::Signed(owner.clone()).into(), owner.clone()));
		let futurepass: T::AccountId = Holders::<T>::get(&owner).unwrap();

		let call: <T as Config>::Call = frame_system::Call::<T>::remark { remark: vec![] }.into();

	}: _(RawOrigin::Signed(owner.clone()), futurepass.clone(), Box::new(call))
	verify {
		assert_last_event::<T>(Event::ProxyExecuted {  delegate: owner, result: Ok(()) }.into())
	}
}

impl_benchmark_test_suite!(Futurepass, crate::mock::TestExt::default().build(), crate::mock::Test);
