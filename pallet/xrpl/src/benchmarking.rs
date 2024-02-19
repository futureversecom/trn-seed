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
		// encoded call for: genesis_hash = 0x0, nonce = 0, max_block_number = 5, tip = 0, extrinsic = System::remark
		let call: <T as Config>::RuntimeCall = frame_system::Call::<T>::remark { remark: b"Mischief Managed".to_vec() }.into();
		let boxed_call = Box::new(call.clone());
		let tx_bytes = hex::decode("5916969036626990000000000000000000F236FD752B5E4C84810AB3D41A3C2580732102509540919FAACF9AB52146C9AA40DB68172D83777250B28E4679176E49CCDD9F81148E6106F6E98E7B21BFDFBFC3DEBA0EDED28A047AF9EA7C0965787472696E7369637D87303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303A353A353A303A33623832663037383031653632636437383966316233636333353936383236313436613163353136666165613766633633333263643362323563646666316331E1F1").unwrap();
		let signature_bytes = hex::decode("3045022100BD4846922C600DEDE58A0B1E15D728671538209DBE6A5A03ECE3C9731D7F5068022027FC270D8DE1FCBE3D7B76931BF1980F0E4BCDA6A764FC76E659620AE91C7841").unwrap();
		let encoded_msg = BoundedVec::truncate_from(tx_bytes.clone());
		let signature = BoundedVec::truncate_from(signature_bytes);
  }: _(RawOrigin::None, encoded_msg, signature, boxed_call)
  verify {
		let tx = XRPLTransaction::try_from(tx_bytes.as_bytes_ref()).unwrap();
		let public_key = tx.get_public_key().unwrap();
		let caller: T::AccountId = tx.get_account().unwrap().into();
		assert_last_event::<T>(Event::XRPLExtrinsicExecuted { public_key, caller, r_address: "rDyqBotBNJeXv8PBHY18ABjyw6FQuWXQnu".into(), call }.into())
  }
}

impl_benchmark_test_suite!(
	Xrpl,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
