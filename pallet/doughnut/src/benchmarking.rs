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
use seed_primitives::AccountId;

use alloc::string::String;
use doughnut_rs::{
	doughnut::{Doughnut, DoughnutV0, DoughnutV1},
	signature::{sign_ecdsa, verify_signature, SignatureVersion},
	traits::{DoughnutApi, DoughnutVerify, FeeMode, PayloadVersion, Signing},
};
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use seed_primitives::Balance;
use sp_core::{
	bytes::to_hex, crypto::ByteArray, ecdsa, ecdsa::Public, keccak_256, Pair, H512, U256,
};
use sp_runtime::traits::One;
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
// Helper struct for a test account where a seed is supplied and provides common methods to
// receive parts of that account
pub struct TestAccount<T: Config>
where
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
{
	pub seed: &'static str,
	_phantom: PhantomData<T>,
}

impl<T: Config> TestAccount<T>
where
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
{
	pub fn new(seed: &'static str) -> Self {
		Self { seed, _phantom: PhantomData }
	}
	// Return the ECDSA pair for this account
	pub fn pair(&self) -> ecdsa::Pair {
		Pair::from_string(self.seed, None).unwrap()
	}

	// Return the public key for this account
	pub fn public(&self) -> Public {
		self.pair().public()
	}

	// Return the private key for this account
	pub fn private(&self) -> [u8; 32] {
		self.pair().seed().into()
	}

	// Return the AccountId type for this account
	pub fn address(&self) -> T::AccountId {
		DoughnutPallet::<T>::get_address(self.public().0.into()).unwrap()
	}
}

pub fn make_doughnut<T>(
	holder: &TestAccount<T>,
	issuer: &TestAccount<T>,
	fee_mode: FeeMode,
	domain: &str,
	domain_payload: Vec<u8>,
) -> Doughnut
where
	T: Config,
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
{
	let mut doughnut_v1 = DoughnutV1 {
		holder: holder.public().as_slice().try_into().expect("should not fail"),
		issuer: issuer.public().as_slice().try_into().expect("should not fail"),
		fee_mode: fee_mode as u8,
		domains: vec![(String::from(domain), domain_payload)],
		expiry: 0,
		not_before: 0,
		payload_version: PayloadVersion::V1 as u16,
		signature_version: SignatureVersion::ECDSA as u8,
		signature: [0_u8; 64],
	};
	// Sign and verify doughnut
	assert_ok!(doughnut_v1.sign_ecdsa(&issuer.private()));
	assert_ok!(doughnut_v1.verify());
	Doughnut::V1(doughnut_v1)
}

benchmarks! {
	where_clause { where <T as frame_system::Config>::AccountId: From<sp_core::H160> + From<AccountId20> }

	transact {
		let alice: TestAccount<T> = TestAccount::new("//Alice"); // holder
		let bob: TestAccount<T> = TestAccount::new("//Bob"); // issuer
		let doughnut = make_doughnut(
			&alice,
			&bob,
			FeeMode::ISSUER,
			"",
			vec![],
		);

		let doughnut_encoded = doughnut.encode();
		let signature: Vec<u8> = sign_ecdsa(&alice.private(), &doughnut_encoded.as_slice()).unwrap().to_vec();
		let call: <T as Config>::RuntimeCall = frame_system::Call::<T>::remark { remark: b"Mischief Managed".to_vec() }.into();
		let nonce: u32 = 0;
	}: _(origin::<T>(&alice.address()), Box::new(call), doughnut_encoded, nonce, signature)
	verify {
		// Verify success event was thrown
		assert_last_event::<T>(Event::DoughnutCallExecuted { result: DispatchResult::Ok(()) }.into());
	}

	revoke_doughnut {
		let alice: TestAccount<T> = TestAccount::new("//Alice");
		let bob: TestAccount<T> = TestAccount::new("//Bob");
		let doughnut = make_doughnut(
			&alice,
			&bob,
			FeeMode::ISSUER,
			"",
			vec![],
		);
		let doughnut_encoded = doughnut.encode();
	}: _(origin::<T>(&bob.address()), doughnut_encoded.clone(), true)
	verify {
		let doughnut_hash = keccak_256(&doughnut_encoded);
		assert!(BlockedDoughnuts::<T>::get(doughnut_hash));
	}

	revoke_holder {
		let alice: TestAccount<T> = TestAccount::new("//Alice");
		let bob: TestAccount<T> = TestAccount::new("//Bob");
	}: _(origin::<T>(&alice.address()), bob.address(), true)
	verify {
		assert!(BlockedHolders::<T>::get(alice.address(), bob.address()));
	}
}

impl_benchmark_test_suite!(
	DoughnutPallet,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
