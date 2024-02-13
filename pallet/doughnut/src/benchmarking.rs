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
use crate::Pallet as DoughnutPallet;
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;
use hex_literal::hex;
use sp_io::hashing::keccak_256;
use sp_std::{vec, vec::Vec};

pub fn account<T: Config>(name: &'static str) -> T::AccountId
where
	T::AccountId: From<sp_core::H160>,
{
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId>
where
	T::AccountId: From<sp_core::H160>,
{
	RawOrigin::Signed(acc.clone())
}

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent)
where
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
{
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

benchmarks! {
	where_clause { where <T as frame_system::Config>::AccountId: From<sp_core::H160> + From<AccountId20> }

	transact {
		let bob = H160::from(hex!("25451a4de12dccc2d166922fa938e900fcc4ed24"));
		let bob: T::AccountId = bob.into();

		// Doughnut from Alice to Bob
		let doughnut_encoded = hex!("011800020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a10390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27000000000074726e0000000000000000000000000045000000000053797374656d00000000000000000000000000000000000000000000000000000072656d61726b000000000000000000000000000000000000000000000000000061cb11f74c449c9371e92b7e9d01c99488e106a152c5a4ff772d80d33b08418d23b000675bfad1641b58a6924382269329d115dc14c0ae5c03efd3eb36953cef00");
		let doughnut_encoded: Vec<u8> = doughnut_encoded.to_vec();
		// add bob to whitelisted holders
		WhitelistedHolders::<T>::insert(bob, true);
		// Signature not required for transact part
		let signature = vec![];
		let call: <T as Config>::RuntimeCall = frame_system::Call::<T>::remark { remark: b"Mischief Managed".to_vec() }.into();
		let nonce: u32 = 0;
	}: _(RawOrigin::None, Box::new(call), doughnut_encoded, nonce, signature)
	verify {
		// Verify success event was thrown
		assert_last_event::<T>(Event::DoughnutCallExecuted { result: DispatchResult::Ok(()) }.into());
	}

	revoke_doughnut {
		let alice = H160::from(hex!("e04cc55ebee1cbce552f250e85c57b70b2e2625b"));
		let alice: T::AccountId = alice.into();
		let doughnut_encoded = hex!("011800020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a10390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27000000000074726e0000000000000000000000000045000000000053797374656d00000000000000000000000000000000000000000000000000000072656d61726b000000000000000000000000000000000000000000000000000061cb11f74c449c9371e92b7e9d01c99488e106a152c5a4ff772d80d33b08418d23b000675bfad1641b58a6924382269329d115dc14c0ae5c03efd3eb36953cef00");
		let doughnut_encoded: Vec<u8> = doughnut_encoded.to_vec();
	}: _(origin::<T>(&alice), doughnut_encoded.clone(), true)
	verify {
		let doughnut_hash = keccak_256(&doughnut_encoded);
		assert!(BlockedDoughnuts::<T>::get(doughnut_hash));
	}

	revoke_holder {
		let alice = account::<T>("//Alice");
		let bob = account::<T>("//Bob");
	}: _(origin::<T>(&alice), bob.clone(), true)
	verify {
		assert!(BlockedHolders::<T>::get(alice, bob));
	}

	update_whitelisted_holders {
		let bob = account::<T>("//Bob");
	}: _(RawOrigin::Root, bob.clone(), true)
	verify {
		assert!(WhitelistedHolders::<T>::get(bob));
	}
}

impl_benchmark_test_suite!(
	DoughnutPallet,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
