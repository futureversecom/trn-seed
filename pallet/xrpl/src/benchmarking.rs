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
use crate::Pallet as Xrpl;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent)
where
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
{
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

benchmarks! {
	where_clause { where <T as frame_system::Config>::AccountId: From<sp_core::H160> }

  submit_encoded_xrpl_transaction {
	let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C25807321026577EEF1DDBC8B7B883BF19457A5FA4CCBD1EEAF29A51AD2D8370CB3E2DC9F2B81149308E2A8716F3F4BCBE49EFA6FA9DAF75AA31D0DF9EA7C0965787472696E7369637D30303A303A353A353030343030303134303464363937333633363836393635363632303464363136653631363736353634E1F1").unwrap();
	let encoded_msg = BoundedVec::truncate_from(tx_bytes.clone());
	let signature = BoundedVec::truncate_from(hex::decode("3045022100E3021242142C82E9B0E0EA46A4BDAFFA08E928F1E1DA74434908BA36512C0E9202206D3A4FFD571A09A8A3C8945DB372D496E8FD1B93E122F4FF41129D3CDF793D66").unwrap());
  }: _(RawOrigin::None, encoded_msg, signature)
  verify {
	let tx = XRPLTransaction::try_from(tx_bytes.as_bytes_ref()).unwrap();
	let public_key = tx.get_public_key().unwrap();
	let caller: T::AccountId = tx.get_account().unwrap().into();
	let call: <T as Config>::RuntimeCall = frame_system::Call::<T>::remark { remark: b"Mischief Managed".to_vec() }.into();
	assert_last_event::<T>(Event::XRPLExtrinsicExecuted { public_key, caller, call }.into())
  }
}

impl_benchmark_test_suite!(
	Xrpl,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
