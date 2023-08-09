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

use crate::Pallet as Futurepass;

use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::{assert_ok, traits::fungibles::Mutate};
use frame_system::RawOrigin;
use hex_literal::hex;
use sp_std::vec;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event)
where
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
{
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

// fund account with ROOT & XRP
pub fn fund<T: Config>(account: &T::AccountId)
where
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
{
	let root_asset_id: u32 = 1;
	assert_ok!(T::MultiCurrency::mint_into(root_asset_id.into(), &account, 1_000_000u32.into()));
}

pub fn add_delegates<T: Config>(
	n: u32,
	futurepass: T::AccountId,
	maybe_who: Option<T::AccountId>,
) -> Result<(), &'static str>
where
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
{
	let caller = maybe_who.unwrap_or_else(whitelisted_caller);
	for i in 0..n {
		T::Proxy::add_delegate(
			&caller,
			&futurepass,
			&account::<T::AccountId>("trarget", i, 0),
			&T::ProxyType::default().into(),
		)?;
	}
	Ok(())
}

benchmarks! {
	where_clause { where <T as frame_system::Config>::AccountId: From<sp_core::H160> }
	create {
		let caller: T::AccountId = whitelisted_caller();
		let owner: T::AccountId = account("owner", 0, 0);

		fund::<T>(&caller);
	}: _(RawOrigin::Signed(caller.clone()), owner.clone())
	verify {
		assert_eq!(Holders::<T>::get(owner).is_some(), true);
	}

	register_delegate_with_signature {
		let p in 1 .. (32 - 1);

		let owner: T::AccountId = account("account", 0, 0);
		fund::<T>(&owner);
		assert_ok!(Futurepass::<T>::create(RawOrigin::Signed(owner.clone()).into(), owner.clone()));
		let futurepass: T::AccountId = Holders::<T>::get(&owner).unwrap();
		add_delegates::<T>(p-1, futurepass.clone(), Some(owner.clone()))?;
		let delegate: T::AccountId = H160::from_slice(&hex!("420aC537F1a4f78d4Dfb3A71e902be0E3d480AFB")).into();
		let proxy_type = T::ProxyType::default();
		let deadline: u32 = 200;

		// keccak256(abi.encodePacked(0xFfFFFFff00000000000000000000000000000001, 0x420aC537F1a4f78d4Dfb3A71e902be0E3d480AFB, 1, 200)
		// cast wallet sign --private-key 0x7e9c7ad85df5cdc88659f53e06fb2eb9bab3ebc59083a3190eaf2c730332529c "a2c9ac848a21f14e5b065959d946c4eb82f384948eaa2799d3a6f162b5a0ac0a"
		let signature: [u8; 65] = hex!("94d1780e44c250d6c87b062e4c2e329deeec176513361fcf006869429f4bdfda549256c203096e9c580b89abbc5c61829cb5eb29270e342a82e21456712d7d411b");
	}: _(RawOrigin::Signed(owner.clone()), futurepass.clone(), delegate.clone(), proxy_type.clone(), deadline, signature)
	verify {
		assert!(T::Proxy::exists(&futurepass, &delegate, Some(proxy_type)));
	}

	unregister_delegate {
		let p in 1 .. (32 - 1);

		let owner: T::AccountId = account("owner", 0, 0);
		fund::<T>(&owner);
		assert_ok!(Futurepass::<T>::create(RawOrigin::Signed(owner.clone()).into(), owner.clone()));
		let futurepass: T::AccountId = Holders::<T>::get(&owner).unwrap();
		add_delegates::<T>(p-1, futurepass.clone(), Some(owner.clone()))?;
		let delegate: T::AccountId = H160::from_slice(&hex!("420aC537F1a4f78d4Dfb3A71e902be0E3d480AFB")).into();
		let proxy_type = T::ProxyType::default();
		let deadline: u32= 200;

		// keccak256(abi.encodePacked(0xFfFFFFff00000000000000000000000000000001, 0x420aC537F1a4f78d4Dfb3A71e902be0E3d480AFB, 1, 200)
		// cast wallet sign --private-key 0x7e9c7ad85df5cdc88659f53e06fb2eb9bab3ebc59083a3190eaf2c730332529c "a2c9ac848a21f14e5b065959d946c4eb82f384948eaa2799d3a6f162b5a0ac0a"
		let signature: [u8; 65] = hex!("94d1780e44c250d6c87b062e4c2e329deeec176513361fcf006869429f4bdfda549256c203096e9c580b89abbc5c61829cb5eb29270e342a82e21456712d7d411b");

		assert_ok!(Futurepass::<T>::register_delegate_with_signature(RawOrigin::Signed(owner.clone()).into(), futurepass.clone(), delegate.clone(), proxy_type.clone(), deadline, signature));
	}: _(RawOrigin::Signed(owner.clone()), futurepass.clone(), delegate.clone())
	verify {
		assert!(!T::Proxy::exists(&futurepass, &delegate, Some(proxy_type)));
	}

	transfer_futurepass {
		let p in 1 .. (32 - 1);

		let owner: T::AccountId = account("owner", 0, 0);
		let new_owner: T::AccountId = account("new-owner", 0, 0);
		fund::<T>(&owner);
		fund::<T>(&new_owner);
		assert_ok!(Futurepass::<T>::create(RawOrigin::Signed(owner.clone()).into(), owner.clone()));
		let futurepass: T::AccountId = Holders::<T>::get(&owner).unwrap();
		add_delegates::<T>(p-1, futurepass.clone(), Some(owner.clone()))?;

	}: _(RawOrigin::Signed(owner.clone()), owner.clone(), Some(new_owner.clone()))
	verify {
		assert_eq!(Holders::<T>::get(new_owner), Some(futurepass));
	}

	proxy_extrinsic {
		let p in 1 .. (32 - 1);

		let owner: T::AccountId = account("owner", 0, 0);
		fund::<T>(&owner);
		assert_ok!(Futurepass::<T>::create(RawOrigin::Signed(owner.clone()).into(), owner.clone()));
		let futurepass: T::AccountId = Holders::<T>::get(&owner).unwrap();
		add_delegates::<T>(p-1, futurepass.clone(), Some(owner.clone()))?;

		let call: <T as Config>::Call = frame_system::Call::<T>::remark { remark: vec![] }.into();

	}: _(RawOrigin::Signed(owner.clone()), futurepass.clone(), Box::new(call))
	verify {
		assert_last_event::<T>(Event::ProxyExecuted {  delegate: owner, result: Ok(()) }.into())
	}
}

impl_benchmark_test_suite!(Futurepass, crate::mock::TestExt::default().build(), crate::mock::Test);
