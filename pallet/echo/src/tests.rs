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

#![cfg(test)]
use super::*;
use crate::mock::{test_storage::NextEventProofId, Echo, MockEthereumEventRouter, System, Test};
use frame_support::storage::StorageValue;
use seed_pallet_common::test_prelude::*;
use sp_runtime::traits::AccountIdConversion;

#[test]
fn ping_works_from_runtime() {
	TestExt::<Test>::default().build().execute_with(|| {
		let caller = H160::from_low_u64_be(123);
		let destination = <Test as Config>::PalletId::get().into_account_truncating();
		// let destination = H160::from_low_u64_be(124);
		let next_session_id = Echo::next_session_id();
		let next_event_proof_id = NextEventProofId::get();

		assert_ok!(Echo::ping(Some(AccountId::from(caller)).into(), destination));

		// Check storage updated
		assert_eq!(Echo::next_session_id(), next_session_id + 1);

		// Check PingSent event thrown
		System::assert_has_event(
			Event::PingSent {
				session_id: next_session_id,
				source: caller,
				destination,
				event_proof_id: next_event_proof_id,
			}
			.into(),
		);

		// Check PongReceived event thrown with expected encoded data
		// In our tests, the MockBridge calls the eventRouter immediately
		System::assert_has_event(
			Event::PongReceived {
				session_id: next_session_id,
				source: caller,
				data: ethabi::encode(&[
					Token::Uint(PING.into()),
					Token::Uint(next_session_id.into()),
					Token::Address(destination),
				]),
			}
			.into(),
		);
	});
}

#[test]
fn ping_works_from_ethereum() {
	TestExt::<Test>::default().build().execute_with(|| {
		let caller = H160::from_low_u64_be(123);
		let destination = <Test as Config>::PalletId::get().into_account_truncating();
		// let destination = H160::from_low_u64_be(124);
		let next_session_id = Echo::next_session_id();
		let next_event_proof_id = NextEventProofId::get();

		let data = ethabi::encode(&[
			Token::Uint(PONG.into()),
			Token::Uint(next_session_id.into()),
			Token::Address(destination),
		]);
		assert_ok!(MockEthereumEventRouter::route(&caller, &destination, data.clone().as_slice()));

		// Check Ping event thrown
		System::assert_has_event(
			Event::PingReceived { session_id: next_session_id, source: caller, data }.into(),
		);

		// Check pong event thrown with expected encoded data
		// In our tests, the MockBridge calls the eventRouter immediately
		System::assert_has_event(
			Event::PongSent {
				session_id: next_session_id,
				source: caller,
				destination,
				event_proof_id: next_event_proof_id,
			}
			.into(),
		);
	});
}
