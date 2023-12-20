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
#![cfg(feature = "std")]

use super::*;

#[allow(unused_imports)]
use crate::Pallet as DoughnutPallet;
use seed_primitives::AccountId;

use doughnut_rs::{
	doughnut::{Doughnut, DoughnutV0, DoughnutV1},
	signature::{sign_ecdsa, verify_signature, SignatureVersion},
	traits::{DoughnutApi, DoughnutVerify, Signing},
};
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;
use seed_primitives::Balance;
use sp_core::{
	bytes::to_hex, crypto::ByteArray, ecdsa, ecdsa::Public, keccak_256, Pair, H512, U256,
};
use sp_runtime::traits::One;

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

fn make_doughnut(
	holder: Public,
	issuer: Public,
	issuer_secret_key: &[u8; 32],
	domain: &str,
	domain_payload: Vec<u8>,
) -> Doughnut {
	let mut doughnut_v1 = DoughnutV1 {
		holder: holder.as_slice().try_into().expect("should not fail"),
		issuer: issuer.as_slice().try_into().expect("should not fail"),
		domains: vec![(domain.to_string(), domain_payload)],
		expiry: 0,
		not_before: 0,
		payload_version: 0,
		signature_version: SignatureVersion::ECDSA as u8,
		signature: [0_u8; 64],
	};
	let signature = doughnut_v1.sign_ecdsa(issuer_secret_key).unwrap();
	println!("Verified?: {:?}", doughnut_v1.verify());
	Doughnut::V1(doughnut_v1)
}

benchmarks! {
	where_clause { where <T as frame_system::Config>::AccountId: From<sp_core::H160> + From<AccountId20> }

	transact {
		let issuer: ecdsa::Pair = Pair::from_string("//Bob", None).unwrap();
		let holder: ecdsa::Pair = Pair::from_string("//Alice", None).unwrap();
		let mut doughnut = make_doughnut(
			holder.public(),
			issuer.public(),
			&hex!("79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf"),
			"",
			vec![],
		);

		let doughnut_encoded = doughnut.encode();
		let alice_private = hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854");
		let signature: Vec<u8> = sign_ecdsa(&alice_private, &doughnut_encoded.as_slice()).unwrap().to_vec();

		let call: <T as Config>::RuntimeCall = frame_system::Call::<T>::remark { remark: b"Mischief Managed".to_vec() }.into();
		let nonce: u32 = 0;

		let holder_address: T::AccountId =
		DoughnutPallet::<T>::get_address(holder.public().0.try_into().unwrap())
			.unwrap().into();
	}: _(origin::<T>(&holder_address), Box::new(call), doughnut_encoded, nonce, signature)
	verify {
		// Verify success event was thrown
		assert_last_event::<T>(Event::DoughnutCallExecuted { result: DispatchResult::Ok(()) }.into())
	}
}

impl_benchmark_test_suite!(
	DoughnutPallet,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
