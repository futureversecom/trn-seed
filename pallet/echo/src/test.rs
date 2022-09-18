#![cfg(test)]
use super::*;
use crate::mock::{
	has_event, test_storage::NextEventProofId, Echo, ExtBuilder, MockEthereumEventRouter,
	TestRuntime,
};
use frame_support::{assert_ok, storage::StorageValue};
use seed_pallet_common::EthereumEventRouter;
use seed_primitives::AccountId;
use sp_core::H160;
use sp_runtime::traits::AccountIdConversion;

#[test]
fn ping_works_from_runtime() {
	ExtBuilder::default().build().execute_with(|| {
		let caller = H160::from_low_u64_be(123);
		let destination = <TestRuntime as Config>::PalletId::get().into_account_truncating();
		// let destination = H160::from_low_u64_be(124);
		let next_session_id = Echo::next_session_id();
		let next_event_proof_id = NextEventProofId::get();

		assert_ok!(Echo::ping(Some(AccountId::from(caller)).into(), destination));

		// Check storage updated
		assert_eq!(Echo::next_session_id(), next_session_id + 1);

		// Check PingSent event thrown
		assert!(has_event(Event::PingSent {
			session_id: next_session_id,
			source: caller,
			destination,
			event_proof_id: next_event_proof_id,
		}));

		// Check PongReceived event thrown with expected encoded data
		// In our tests, the MockBridge calls the eventRouter immediately
		assert!(has_event(Event::PongReceived {
			session_id: next_session_id,
			source: caller,
			data: ethabi::encode(&[
				Token::Uint(PING.into()),
				Token::Uint(next_session_id.into()),
				Token::Address(destination),
			]),
		}));
	});
}

#[test]
fn ping_works_from_ethereum() {
	ExtBuilder::default().build().execute_with(|| {
		let caller = H160::from_low_u64_be(123);
		let destination = <TestRuntime as Config>::PalletId::get().into_account_truncating();
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
		assert!(has_event(Event::PingReceived {
			session_id: next_session_id,
			source: caller,
			data
		}));

		// Check pong event thrown with expected encoded data
		// In our tests, the MockBridge calls the eventRouter immediately
		assert!(has_event(Event::PongSent {
			session_id: next_session_id,
			source: caller,
			destination,
			event_proof_id: next_event_proof_id
		}));
	});
}
