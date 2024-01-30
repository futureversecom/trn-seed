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

  transact {
		// encoded call for: chainIid = 0, nonce = 0, max_block_number = 5, tip = 0, extrinsic = System::remark
		let call: <T as Config>::RuntimeCall = frame_system::Call::<T>::remark { remark: Default::default() }.into();
		let boxed_call = Box::new(call.clone());
		let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C25807321021A765BED04797D2DD723C9FDC1ED9D20FEC478F7E8E7D16236F8504C5740C10781145FF8490F22ABFA576788227DB2E80D3F5F104654F9EA7C0965787472696E7369637D48303A303A353A303A35633933633236383339613137636235616366323765383961616330306639646433663531643161316161346234383266363930663634333633396665383732E1F1").unwrap();
		let signature_bytes = hex::decode("3045022100A6E6546A845ED811FF833789ABE96A5D196737D6FAE0612F40639344DB3ABC2202205D4E3A3753EBC50CB5EBC1A0E861BE0DABA1EE062C08BB40DA9F65F20DEF0CF8").unwrap();
		let encoded_msg = BoundedVec::truncate_from(tx_bytes.clone());
		let signature = BoundedVec::truncate_from(signature_bytes);
  }: _(RawOrigin::None, encoded_msg, signature, boxed_call)
  verify {
		let tx = XRPLTransaction::try_from(tx_bytes.as_bytes_ref()).unwrap();
		let public_key = tx.get_public_key().unwrap();
		let caller: T::AccountId = tx.get_account().unwrap().into();
		assert_last_event::<T>(Event::XRPLExtrinsicExecuted { public_key, caller, r_address: "r9kSdPu1GRr75qfy636iraAm7CbMRmDC3o".into(), call }.into())
  }
}

impl_benchmark_test_suite!(
	Xrpl,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
