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
use crate::Pallet as XrplTransaction;
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
	let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732102C3E733C74A768A566F6B317B0C3D8778CD85244A2916D759BBB870BDDACDA82B8114CA8E9A489A5D6DD56BA053494D851D3B29899DFCF9EA7C0965787472696E7369637D2E303A353A353030343030303134303464363937333633363836393635363632303464363136653631363736353634E1F1").unwrap();
	let encoded_msg = BoundedVec::truncate_from(tx_bytes.clone());
	let signature = BoundedVec::truncate_from(hex::decode("3045022100BD734A38F9C5C210CC7E1D57AEA6DA45039D0068E3ABBA348189A5EBC6A0757D022077B4212F023C66B6C99FB68DC7AEF7921A1BAFF2A85AC6C5E70000C50009231C").unwrap());
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
	XrplTransaction,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
