/* Copyright 2019-2022 Centrality Investments Limited
 *
 * Licensed under the LGPL, Version 3.0 (the "License");
 * you may not use this file except in compliance with the License.
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * You may obtain a copy of the License at the root of this project source code,
 * or at:
 *     https://centrality.ai/licenses/gplv3.txt
 *     https://centrality.ai/licenses/lgplv3.txt
 */
#![cfg(test)]
use codec::Encode;
use ethabi::Token;
use frame_support::{
	assert_noop, assert_ok,
	dispatch::DispatchError,
	storage::{StorageMap, StorageValue},
	traits::{OnInitialize, OneSessionHandler, UnixTime},
	weights::{constants::RocksDbWeight as DbWeight, Weight},
};
use sp_core::{ByteArray, H160, H256, U256};
use sp_runtime::{generic::DigestItem, traits::AccountIdConversion, SaturatedConversion};

use seed_pallet_common::{EthCallFailure, EthereumBridge};
use seed_primitives::ethy::{
	crypto::AuthorityId, ConsensusLog, EventClaimId, PendingAuthorityChange, ValidatorSet,
};

use crate::{
	impls::{encode_event_for_proving, prune_event_ids},
	mock::*,
	types::{
		CheckedEthCallRequest, CheckedEthCallResult, EthAddress, EthBlock, EthHash, EventClaim,
		EventClaimResult, EventProofId, TransactionReceipt,
	},
	BridgePaused, Config, Error, EthCallRequestInfo, Module, ETHY_ENGINE_ID,
	SUBMIT_BRIDGE_EVENT_SELECTOR,
};

/// Mocks an Eth block for when get_block_by_number is called
/// Adds this to the mock storage
/// The latest block will be the highest block stored
fn mock_block_response(block_number: u64, timestamp: U256) -> EthBlock {
	let mock_block = MockBlockBuilder::new()
		.block_number(block_number)
		.block_hash(H256::from_low_u64_be(block_number))
		.timestamp(timestamp)
		.build();

	MockEthereumRpcClient::mock_block_response_at(
		block_number.saturated_into(),
		mock_block.clone(),
	);
	mock_block
}

/// Mocks a TransactionReceipt for when get_transaction_receipt is called
/// Adds this to the mock storage
fn create_transaction_receipt_mock(
	block_number: u64,
	tx_hash: EthHash,
	to: EthAddress,
	logs: Vec<MockLog>,
) -> TransactionReceipt {
	let mock_tx_receipt = MockReceiptBuilder::new()
		.block_number(block_number)
		.transaction_hash(tx_hash)
		.to(to)
		.logs(logs)
		.build();

	MockEthereumRpcClient::mock_transaction_receipt_for(tx_hash, mock_tx_receipt.clone());
	mock_tx_receipt
}

/// Ethereum ABI encode an event message according to the 1.5 standard
fn encode_event_message(
	event_id: EventClaimId,
	source: H160,
	destination: H160,
	message: &[u8],
) -> Vec<u8> {
	ethabi::encode(&[
		Token::Uint(event_id.into()),
		Token::Address(source),
		Token::Address(destination),
		Token::Bytes(message.to_vec()),
	])
}

/// Helper to get the bridge contract address that the `EthBridge` pallet is connected with
/// i.e the trusted origin for all submitted event logs
fn bridge_contract_address() -> EthAddress {
	<TestRuntime as Config>::BridgeContractAddress::get().into()
}

#[test]
fn submit_event() {
	let relayer = H160::from_low_u64_be(123);
	let tx_hash = EthHash::from_low_u64_be(33);
	let (event_id, source, destination, message) =
		(1_u64, H160::from_low_u64_be(555), H160::from_low_u64_be(555), &[1_u8, 2, 3, 4, 5]);
	let event_data = encode_event_message(event_id, source, destination, message);

	ExtBuilder::default().relayer(relayer).build().execute_with(|| {
		assert_ok!(EthBridge::submit_event(
			Origin::signed(relayer.into()),
			tx_hash.clone(),
			event_data.clone(),
		));

		let process_at = System::block_number() + <TestRuntime as Config>::ChallengePeriod::get();
		assert_eq!(
			EthBridge::pending_event_claims(event_id),
			Some(EventClaim { tx_hash, source, destination, data: message.to_vec() })
		);
		assert_eq!(EthBridge::messages_valid_at(process_at), [event_id],);
	});
}

#[test]
fn submit_event_relayer_only() {
	ExtBuilder::default().build().execute_with(|| {
		let not_relayer = H160::from_low_u64_be(11);
		assert_noop!(
			EthBridge::submit_event(Origin::signed(not_relayer.into()), H256::default(), vec![]),
			Error::<TestRuntime>::NoPermission
		);
	});
}

#[test]
fn submit_event_bad_encoding() {
	let relayer = H160::from_low_u64_be(123);
	let tx_hash = EthHash::from_low_u64_be(33);
	let event_data = vec![1u8, 2, 3, 4, 5];

	ExtBuilder::default().relayer(relayer).build().execute_with(|| {
		assert_noop!(
			EthBridge::submit_event(Origin::signed(relayer.into()), tx_hash, event_data),
			Error::<TestRuntime>::InvalidClaim
		);
	});
}

#[test]
fn submit_event_tracks_pending() {
	let relayer = H160::from_low_u64_be(123);
	let tx_hash = EthHash::from_low_u64_be(33);
	let event_data = encode_event_message(
		1_u64,
		H160::from_low_u64_be(555),
		H160::from_low_u64_be(555),
		&[1_u8, 2, 3, 4, 5],
	);

	ExtBuilder::default().relayer(relayer).build().execute_with(|| {
		assert_ok!(EthBridge::submit_event(
			Origin::signed(relayer.into()),
			tx_hash.clone(),
			event_data.clone(),
		));

		assert_noop!(
			EthBridge::submit_event(Origin::signed(relayer.into()), tx_hash, event_data),
			Error::<TestRuntime>::EventReplayPending
		);
	});
}

#[test]
fn submit_event_tracks_completed() {
	let relayer = H160::from_low_u64_be(123);
	let tx_hash = EthHash::from_low_u64_be(33);
	let event_data = encode_event_message(
		1_u64,
		H160::from_low_u64_be(555),
		H160::from_low_u64_be(555),
		&[1_u8, 2, 3, 4, 5],
	);

	ExtBuilder::default().relayer(relayer).build().execute_with(|| {
		assert_ok!(EthBridge::submit_event(
			Origin::signed(relayer.into()),
			tx_hash.clone(),
			event_data.clone(),
		));

		// Process the message
		let process_at = System::block_number() + <TestRuntime as Config>::ChallengePeriod::get();
		EthBridge::on_initialize(process_at);

		assert_noop!(
			EthBridge::submit_event(Origin::signed(relayer.into()), tx_hash, event_data),
			Error::<TestRuntime>::EventReplayProcessed
		);
	});
}

#[test]
fn pre_last_session_change() {
	ExtBuilder::default().next_session_final().build().execute_with(|| {
		let current_keys = vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
		];
		let next_keys = vec![
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
		];
		let event_proof_id = EthBridge::next_event_proof_id();

		EthBridge::handle_authorities_change(current_keys, next_keys.clone());

		assert_eq!(
			System::digest().logs[0],
			DigestItem::Consensus(
				ETHY_ENGINE_ID,
				ConsensusLog::PendingAuthoritiesChange(PendingAuthorityChange {
					source: BridgePalletId::get().into_account_truncating(),
					destination: EthereumBridgeContractAddress::get().into(),
					next_validator_set: ValidatorSet {
						validators: next_keys.to_vec(),
						id: 1,
						proof_threshold: 2,
					},
					event_proof_id,
				})
				.encode(),
			),
		);

		assert_eq!(EthBridge::next_notary_keys(), next_keys);
		assert_eq!(EthBridge::notary_set_proof_id(), event_proof_id);
		assert_eq!(EthBridge::next_event_proof_id(), event_proof_id + 1);
	});
}

#[test]
fn last_session_change() {
	ExtBuilder::default().active_session_final().build().execute_with(|| {
		let current_set_id = EthBridge::notary_set_id();

		// setup storage
		let current_keys = vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
		];
		crate::NotaryKeys::<TestRuntime>::put(&current_keys);
		assert_eq!(
			EthBridge::validator_set(),
			ValidatorSet {
				validators: current_keys.clone(),
				id: current_set_id,
				proof_threshold: 2 // ceil(2 * 0.66)
			}
		);

		let next_keys = vec![
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[5_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[6_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[7_u8; 33]).unwrap(),
		];
		crate::NextNotaryKeys::<TestRuntime>::put(&next_keys);

		// current session is last in era: starting
		EthBridge::handle_authorities_change(current_keys, next_keys.clone());
		assert!(EthBridge::bridge_paused());
		// current session is last in era: finishing
		<Module<TestRuntime> as OneSessionHandler<AccountId>>::on_before_session_ending();
		assert_eq!(EthBridge::notary_keys(), next_keys);
		assert_eq!(EthBridge::notary_set_id(), current_set_id + 1);
		assert_eq!(
			EthBridge::validator_set(),
			ValidatorSet {
				validators: next_keys,
				id: current_set_id + 1,
				proof_threshold: 4 // ceil(5 * 0.66)
			}
		);
		assert!(!EthBridge::bridge_paused());
	});
}

#[test]
fn send_event() {
	ExtBuilder::default().build().execute_with(|| {
		// Test generating event proof without delay
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		let message = &b"hello world"[..];
		let event_proof_id = EthBridge::next_event_proof_id();

		// Generate event proof
		assert_ok!(EthBridge::send_event(&source, &destination, &message));
		// Ensure event has not been added to delayed claims
		assert_eq!(EthBridge::pending_event_proofs(event_proof_id), None);
		assert_eq!(EthBridge::next_event_proof_id(), event_proof_id + 1);
		// On initialize does up to 2 reads to check for delayed proofs
		assert_eq!(
			EthBridge::on_initialize(frame_system::Pallet::<TestRuntime>::block_number() + 1),
			DbWeight::get().reads(2 as Weight)
		);
	});
}

#[test]
fn request_multiple_event_proofs() {
	ExtBuilder::default().build().execute_with(|| {
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		let message = &b"hello world"[..];

		assert_ok!(EthBridge::send_event(&source, &destination, &message));
		assert_ok!(EthBridge::send_event(&source, &destination, &message));
		let block_digest = <frame_system::Pallet<TestRuntime>>::digest();
		assert_eq!(block_digest.logs.len(), 2_usize);
	});
}

#[test]
fn delayed_event_proof() {
	ExtBuilder::default().build().execute_with(|| {
		let message = &b"hello world"[..];
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		BridgePaused::put(true);
		assert_eq!(EthBridge::bridge_paused(), true);

		let event_proof_id = EthBridge::next_event_proof_id();
		let encoded_event = encode_event_for_proving(
			source,
			destination.clone(),
			message,
			EthBridge::validator_set().id,
			event_proof_id,
		);

		// Generate event proof
		assert_ok!(EthBridge::send_event(&source, &destination, &message));
		// Ensure event has been added to delayed claims
		assert_eq!(EthBridge::pending_event_proofs(event_proof_id), Some(encoded_event));
		assert_eq!(EthBridge::next_event_proof_id(), event_proof_id + 1);

		// Re-enable bridge
		BridgePaused::kill();
		// initialize pallet and initiate event proof
		let max_delayed_events = EthBridge::delayed_event_proofs_per_block() as u64;
		let expected_weight: Weight = DbWeight::get().reads(3 as Weight) +
			DbWeight::get().writes(2 as Weight) * max_delayed_events;
		assert_eq!(
			EthBridge::on_initialize(frame_system::Pallet::<TestRuntime>::block_number() + 1),
			expected_weight
		);
		// Ensure event has been removed from delayed claims
		assert!(EthBridge::pending_event_proofs(event_proof_id).is_none());
	});
}

#[test]
fn multiple_delayed_event_proof() {
	ExtBuilder::default().build().execute_with(|| {
		let message = &b"hello world"[..];
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		BridgePaused::put(true);
		assert_eq!(EthBridge::bridge_paused(), true);

		let max_delayed_events = EthBridge::delayed_event_proofs_per_block();
		let event_count: u8 = max_delayed_events * 2;
		let mut event_ids: Vec<EventProofId> = vec![];
		let mut events_for_proving = vec![];
		for _ in 0..event_count {
			let event_proof_id = EthBridge::next_event_proof_id();
			event_ids.push(event_proof_id);
			let expected_encoded_event = encode_event_for_proving(
				source,
				destination.clone(),
				message,
				EthBridge::validator_set().id,
				event_proof_id,
			);
			events_for_proving.push(expected_encoded_event.clone());
			// Generate event proof
			assert_ok!(EthBridge::send_event(&source, &destination, &message));
			// Ensure event has been added to delayed claims
			assert_eq!(
				EthBridge::pending_event_proofs(event_proof_id),
				Some(expected_encoded_event)
			);
			assert_eq!(EthBridge::next_event_proof_id(), event_proof_id + 1);
		}

		// Re-enable bridge
		BridgePaused::kill();
		// initialize pallet and initiate event proof
		assert_eq!(
			EthBridge::on_initialize(frame_system::Pallet::<TestRuntime>::block_number() + 1),
			DbWeight::get().reads(3 as Weight) +
				DbWeight::get().writes(2 as Weight) * max_delayed_events as u64
		);

		let mut removed_count = 0;
		for i in 0..event_count {
			// Ensure event has been removed from delayed claims
			if EthBridge::pending_event_proofs(event_ids[i as usize]).is_none() {
				removed_count += 1;
			} else {
				assert_eq!(
					EthBridge::pending_event_proofs(event_ids[i as usize]),
					Some(events_for_proving[i as usize].clone())
				)
			}
		}
		// Should have only processed max amount
		assert_eq!(removed_count, max_delayed_events);

		// Now initialize next block and process the rest
		assert_eq!(
			EthBridge::on_initialize(frame_system::Pallet::<TestRuntime>::block_number() + 2),
			DbWeight::get().reads(3 as Weight) +
				DbWeight::get().writes(2 as Weight) * max_delayed_events as u64
		);

		let mut removed_count = 0;
		for i in 0..event_count {
			// Ensure event has been removed from delayed claims
			if EthBridge::pending_event_proofs(event_ids[i as usize]).is_none() {
				removed_count += 1;
			}
		}
		// All events should have now been processed
		assert_eq!(removed_count, event_count);
	});
}

#[test]
fn set_delayed_event_proofs_per_block() {
	ExtBuilder::default().build().execute_with(|| {
		// Check that it starts as default value
		assert_eq!(EthBridge::delayed_event_proofs_per_block(), 5);
		let new_max_delayed_events: u8 = 10;
		assert_ok!(EthBridge::set_delayed_event_proofs_per_block(
			frame_system::RawOrigin::Root.into(),
			new_max_delayed_events
		));
		assert_eq!(EthBridge::delayed_event_proofs_per_block(), new_max_delayed_events);

		let message = &b"hello world"[..];
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		let mut event_ids: Vec<EventProofId> = vec![];
		BridgePaused::put(true);

		for _ in 0..new_max_delayed_events {
			let event_proof_id = EthBridge::next_event_proof_id();
			event_ids.push(event_proof_id);
			let expected_encoded_event = encode_event_for_proving(
				source,
				destination.clone(),
				message,
				EthBridge::validator_set().id,
				event_proof_id,
			);
			// Generate event proof
			assert_ok!(EthBridge::send_event(&source, &destination, &message));
			// Ensure event has been added to delayed claims
			assert_eq!(
				EthBridge::pending_event_proofs(event_proof_id),
				Some(expected_encoded_event)
			);
			assert_eq!(EthBridge::next_event_proof_id(), event_proof_id + 1);
		}

		// Re-enable bridge
		BridgePaused::kill();
		// initialize pallet and initiate event proof
		assert_eq!(
			EthBridge::on_initialize(frame_system::Pallet::<TestRuntime>::block_number() + 1),
			DbWeight::get().reads(3 as Weight) +
				DbWeight::get().writes(2 as Weight) * new_max_delayed_events as u64
		);

		for i in 0..new_max_delayed_events {
			// Ensure event has been removed from delayed claims
			assert!(EthBridge::pending_event_proofs(event_ids[i as usize]).is_none());
		}
	});
}

#[test]
fn set_delayed_event_proofs_per_block_not_root_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		// Check that it starts as default value
		assert_eq!(EthBridge::delayed_event_proofs_per_block(), 5);
		let new_value: u8 = 10;
		assert_noop!(
			EthBridge::set_delayed_event_proofs_per_block(
				frame_system::RawOrigin::None.into(),
				new_value
			),
			DispatchError::BadOrigin
		);
		assert_eq!(EthBridge::delayed_event_proofs_per_block(), 5);
	});
}

#[test]
fn offchain_try_notarize_event() {
	ExtBuilder::default().build().execute_with(|| {
		// Mock block response and transaction receipt
		let block_number = 10;
		let timestamp =
			U256::from(<MockUnixTime as UnixTime>::now().as_secs().saturated_into::<u64>());
		let tx_hash = EthHash::from_low_u64_be(222);
		let source = EthAddress::from_low_u64_be(333);
		let destination = EthAddress::from_low_u64_be(444);
		let message = vec![1_u8, 2, 3, 4, 5];
		let event_id = 1;
		let event_data = encode_event_message(event_id, source, destination, message.as_slice());

		// Create block info for both the transaction block and a later block
		let _mock_block_1 = mock_block_response(block_number, timestamp);
		let _mock_block_2 = mock_block_response(block_number + 5, timestamp);
		let mock_log = MockLogBuilder::new()
			.address(bridge_contract_address())
			.data(event_data.as_slice())
			.topics(vec![SUBMIT_BRIDGE_EVENT_SELECTOR.into()])
			.transaction_hash(tx_hash)
			.build();
		let _mock_tx_receipt =
			create_transaction_receipt_mock(block_number, tx_hash, source, vec![mock_log]);

		let event_claim = EventClaim { tx_hash, source, destination, data: message };
		assert_eq!(
			EthBridge::offchain_try_notarize_event(event_id, event_claim),
			EventClaimResult::Valid
		);
	});
}

#[test]
fn offchain_try_notarize_event_no_tx_receipt_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let event_claim = EventClaim {
			tx_hash: H256::from_low_u64_be(222),
			source: H160::from_low_u64_be(333),
			..Default::default()
		};
		let event_id = 1;
		assert_eq!(
			EthBridge::offchain_try_notarize_event(event_id, event_claim),
			EventClaimResult::NoTxReceipt
		);
	});
}

#[test]
fn offchain_try_notarize_event_no_status_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		// Mock transaction receipt
		let tx_hash = EthHash::from_low_u64_be(222);
		let source = EthAddress::from_low_u64_be(333);
		let mock_tx_receipt = MockReceiptBuilder::new()
			.block_number(10)
			.transaction_hash(tx_hash)
			.status(0)
			.build();

		// Create mock info for transaction receipt
		MockEthereumRpcClient::mock_transaction_receipt_for(tx_hash, mock_tx_receipt.clone());

		let event_claim = EventClaim { tx_hash, source, ..Default::default() };
		let event_id = 1;
		assert_eq!(
			EthBridge::offchain_try_notarize_event(event_id, event_claim),
			EventClaimResult::TxStatusFailed
		);
	});
}

#[test]
fn offchain_try_notarize_event_unexpected_source_address_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		// Mock transaction receipt
		let block_number = 10;
		let tx_hash = EthHash::from_low_u64_be(222);
		let source = EthAddress::from_low_u64_be(333);

		// Create mock info for transaction receipt
		let mock_log = MockLogBuilder::new().address(source).build(); // `source` is not the `bridge_contract_address`
		let _mock_tx_receipt =
			create_transaction_receipt_mock(block_number, tx_hash, source, vec![mock_log]);

		// Create event claim where event is emitted by a different address to the tx_receipt 'to'
		let event_claim =
			EventClaim { tx_hash, source: H160::from_low_u64_be(444), ..Default::default() };
		let event_id = 1;

		assert_eq!(
			EthBridge::offchain_try_notarize_event(event_id, event_claim),
			EventClaimResult::UnexpectedSource
		);
	});
}

#[test]
fn offchain_try_notarize_event_no_block_number_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		// Mock transaction receipt
		let block_number = 10;
		let tx_hash = EthHash::from_low_u64_be(222);
		let source = EthAddress::from_low_u64_be(333);
		let destination = EthAddress::from_low_u64_be(444);
		let event_id = 1;

		// Create mock info for transaction receipt
		let event_data = encode_event_message(event_id, source, destination, Default::default());
		let mock_log = MockLogBuilder::new()
			.address(bridge_contract_address())
			.topics(vec![SUBMIT_BRIDGE_EVENT_SELECTOR.into()])
			.data(event_data.as_slice())
			.transaction_hash(tx_hash)
			.build();
		let _mock_tx_receipt =
			create_transaction_receipt_mock(block_number, tx_hash, source, vec![mock_log]);

		let event_claim = EventClaim { tx_hash, source, destination, ..Default::default() };

		assert_eq!(
			EthBridge::offchain_try_notarize_event(event_id, event_claim),
			EventClaimResult::DataProviderErr
		);
	});
}

#[test]
fn offchain_try_notarize_event_no_confirmations_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		// Mock block response and transaction receipt
		let block_number = 10;
		let timestamp =
			U256::from(<MockUnixTime as UnixTime>::now().as_secs().saturated_into::<u64>());
		let tx_hash = EthHash::from_low_u64_be(222);
		let source = EthAddress::from_low_u64_be(333);
		let destination = EthAddress::from_low_u64_be(444);
		let event_id = 1;

		// Create block info for both the transaction block and a later block
		let _mock_block_1 = mock_block_response(block_number, timestamp);
		let _mock_block_2 = mock_block_response(block_number, timestamp);
		let event_data = encode_event_message(event_id, source, destination, Default::default());
		let mock_log = MockLogBuilder::new()
			.address(bridge_contract_address())
			.topics(vec![SUBMIT_BRIDGE_EVENT_SELECTOR.into()])
			.data(event_data.as_slice())
			.transaction_hash(tx_hash)
			.build();
		let _mock_tx_receipt =
			create_transaction_receipt_mock(block_number, tx_hash, source, vec![mock_log]);

		let event_claim = EventClaim { tx_hash, source, destination, ..Default::default() };

		assert_eq!(
			EthBridge::offchain_try_notarize_event(event_id, event_claim),
			EventClaimResult::NotEnoughConfirmations
		);
	});
}

#[test]
fn offchain_try_notarize_event_expired_confirmation_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1_000_000);
		// Mock block response and transaction receipt
		let block_number = 10;
		let timestamp = U256::zero();
		let event_id = 2;
		let tx_hash = EthHash::from_low_u64_be(222);
		let source = EthAddress::from_low_u64_be(333);
		let destination = EthAddress::from_low_u64_be(444);

		// Create block info for both the transaction block and a later block
		let _mock_block_1 = mock_block_response(block_number, timestamp);
		let _mock_block_2 = mock_block_response(block_number + 5, timestamp);
		let event_data = encode_event_message(event_id, source, destination, Default::default());
		let mock_log = MockLogBuilder::new()
			.address(bridge_contract_address())
			.topics(vec![SUBMIT_BRIDGE_EVENT_SELECTOR.into()])
			.data(event_data.as_slice())
			.transaction_hash(tx_hash)
			.build();
		let _mock_tx_receipt =
			create_transaction_receipt_mock(block_number, tx_hash, source, vec![mock_log]);

		let event_claim = EventClaim { tx_hash, source, destination, ..Default::default() };
		assert_eq!(
			EthBridge::offchain_try_notarize_event(event_id, event_claim),
			EventClaimResult::Expired
		);
	});
}

#[test]
fn offchain_try_notarize_event_no_observed_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		// Mock block response and transaction receipt
		let block_number = 10;
		let timestamp =
			U256::from(<MockUnixTime as UnixTime>::now().as_secs().saturated_into::<u64>());
		let tx_hash = EthHash::from_low_u64_be(222);
		let source = EthAddress::from_low_u64_be(333);
		let destination = EthAddress::from_low_u64_be(444);
		let event_id = 1;

		// Create block info for both the transaction block and a later block
		let _mock_block_1 = mock_block_response(block_number, timestamp);
		let event_data = encode_event_message(event_id, source, destination, Default::default());
		let mock_log = MockLogBuilder::new()
			.address(bridge_contract_address())
			.topics(vec![SUBMIT_BRIDGE_EVENT_SELECTOR.into()])
			.data(event_data.as_slice())
			.transaction_hash(tx_hash)
			.build();
		let _mock_tx_receipt =
			create_transaction_receipt_mock(block_number + 1, tx_hash, source, vec![mock_log]);
		let event_claim = EventClaim { tx_hash, source, destination, ..Default::default() };

		// Set event confirmations to 0 so it doesn't fail early
		let _ = EthBridge::set_event_block_confirmations(frame_system::RawOrigin::Root.into(), 0);
		assert_eq!(
			EthBridge::offchain_try_notarize_event(event_id, event_claim),
			EventClaimResult::DataProviderErr
		);
	});
}

#[test]
fn offchain_try_eth_call_cant_fetch_latest_block() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			EthBridge::offchain_try_eth_call(&CheckedEthCallRequestBuilder::new().build()),
			CheckedEthCallResult::DataProviderErr
		);
	});
}

#[test]
fn offchain_try_eth_call_cant_check_call() {
	ExtBuilder::default().build().execute_with(|| {
		mock_block_response(123_u64, now().into());
		assert_eq!(
			EthBridge::offchain_try_eth_call(&CheckedEthCallRequestBuilder::new().build()),
			CheckedEthCallResult::DataProviderErr,
		);
	});
}

#[test]
fn offchain_try_eth_call_at_historic_block() {
	// given a request where `try_block_number` is within `max_look_behind_blocks` from the latest
	// ethereum block when the validator checks the request
	// then the `eth_call` should be executed at `try_block_number`
	ExtBuilder::default().build().execute_with(|| {
		let latest_block_number = 123_u64;
		let latest_block_timestamp = now();
		mock_timestamp(now());
		mock_block_response(latest_block_number, latest_block_timestamp.into());

		let try_block_number = 121_u64;
		let try_block_timestamp = latest_block_timestamp - 15 * 2; // ethereum block timestamp 3 blocks before latest
		mock_block_response(try_block_number, try_block_timestamp.into());

		let remote_contract = H160::from_low_u64_be(333);
		let expected_return_data = [0x01_u8; 32];
		MockEthereumRpcClient::mock_call_at(
			try_block_number,
			remote_contract,
			&expected_return_data,
		);

		let request = CheckedEthCallRequestBuilder::new()
			.try_block_number(try_block_number)
			.max_block_look_behind(latest_block_number - try_block_number)
			.target(remote_contract)
			.build();

		// When
		let result = EthBridge::offchain_try_eth_call(&request);

		// Then
		assert_eq!(
			result,
			CheckedEthCallResult::Ok(expected_return_data, try_block_number, try_block_timestamp)
		);
	});
}

#[test]
fn offchain_try_eth_call_at_latest_block() {
	// given a request where `try_block_number` is outside `max_look_behind_blocks` from the latest
	// ethereum block when the validator checks the request
	// then the `eth_call` should be executed at `latest_block_number`
	ExtBuilder::default().build().execute_with(|| {
		let latest_block_number = 123_u64;
		let latest_block_timestamp = now();
		mock_timestamp(now());
		mock_block_response(latest_block_number, latest_block_timestamp.into());

		let remote_contract = H160::from_low_u64_be(333);
		let expected_return_data = [0x01_u8; 32];
		MockEthereumRpcClient::mock_call_at(
			latest_block_number,
			remote_contract,
			&expected_return_data,
		);

		let request = CheckedEthCallRequestBuilder::new()
			.check_timestamp(latest_block_timestamp)
			.max_block_look_behind(2)
			.try_block_number(latest_block_number - 3) // lookbehind is 2 => try block falls out of range
			.target(remote_contract)
			.build();

		// When
		let result = EthBridge::offchain_try_eth_call(&request);

		// Then
		assert_eq!(
			result,
			CheckedEthCallResult::Ok(
				expected_return_data,
				latest_block_number,
				latest_block_timestamp
			)
		);
	});
}

#[test]
fn offchain_try_eth_call_reports_oversized_return_data() {
	// given a request where returndata is > 32 bytes
	// when the validator checks the request
	// then it should be reported as oversized
	ExtBuilder::default().build().execute_with(|| {
		let latest_block_number = 123_u64;
		mock_timestamp(now());
		mock_block_response(latest_block_number, now().into());
		let remote_contract = H160::from_low_u64_be(333);
		MockEthereumRpcClient::mock_call_at(latest_block_number, remote_contract, &[0x02, 33]); // longer than 32 bytes

		let request = CheckedEthCallRequestBuilder::new()
			.target(remote_contract)
			.try_block_number(5)
			.build();

		// When
		let result = EthBridge::offchain_try_eth_call(&request);

		// Then
		assert_eq!(result, CheckedEthCallResult::ReturnDataExceedsLimit);
	});
}

#[test]
fn offchain_try_eth_call_at_historic_block_after_delay() {
	// given a request where `try_block_number` is originally within `max_look_behind_blocks` but
	// moves outside of this range due to a delay in the challenge
	// when the validator checks the request
	// then the `eth_call` should be executed at `try_block_number`, factoring in the delay
	ExtBuilder::default().build().execute_with(|| {
		let latest_block_number = 130_u64;
		let latest_block_timestamp = now();
		mock_timestamp(now());
		mock_block_response(latest_block_number, latest_block_timestamp.into());

		let try_block_number = 123_u64;
		let try_block_timestamp = latest_block_timestamp - 15 * 7; // ethereum block timestamp 7 blocks before latest
		mock_block_response(try_block_number, try_block_timestamp.into());

		let remote_contract = H160::from_low_u64_be(333);
		let expected_return_data = [0x01_u8; 32];
		MockEthereumRpcClient::mock_call_at(
			try_block_number,
			remote_contract,
			&expected_return_data,
		);

		// The max look behind blocks is 3 which is correct at the time of request
		// (`check_timestamp`) however, a delay in challenge execution means another 4 blocks have
		// passed (target block is now 7 behind latest) the additional 4 blocks lenience should be
		// granted due to the `check_timestamp`
		let request_timestamp = now() - 2 * 60; // 2 mins ago
		let check_timestamp = request_timestamp + 60; // 1 min ago
		let request = CheckedEthCallRequestBuilder::new()
			.timestamp(request_timestamp)
			.check_timestamp(check_timestamp)
			.max_block_look_behind(3)
			.try_block_number(try_block_number)
			.target(remote_contract)
			.build();

		// When
		let result = EthBridge::offchain_try_eth_call(&request);

		// Then
		assert_eq!(
			result,
			CheckedEthCallResult::Ok(expected_return_data, try_block_number, try_block_timestamp)
		);

		// same request as before but the check time set is set to _now_
		// no delay is considered and so the checked call happens at the latest block (which is not
		// mocked)
		let request = CheckedEthCallRequestBuilder::new()
			.timestamp(request_timestamp)
			.check_timestamp(latest_block_timestamp)
			.max_block_look_behind(3)
			.try_block_number(try_block_number) // lookbehind is 2 => try block falls out of range
			.target(remote_contract)
			.build();
		let result = EthBridge::offchain_try_eth_call(&request);
		assert_eq!(result, CheckedEthCallResult::DataProviderErr);
	});
}

#[test]
fn handle_call_notarization_success() {
	// given 9 validators and 6 agreeing notarizations (over required 2/3 threshold)
	// when the notarizations are aggregated
	// then it triggers the success callback

	// fake ecdsa public keys to represent the mocked validators
	let mock_notary_keys: Vec<<TestRuntime as Config>::EthyId> = (1_u8..=9_u8)
		.map(|k| <TestRuntime as Config>::EthyId::from_slice(&[k; 33]).unwrap())
		.collect();
	ExtBuilder::default().build().execute_with(|| {
		let call_id = 1_u64;
		EthCallRequestInfo::insert(call_id, CheckedEthCallRequest::default());
		MockValidatorSet::mock_n_validators(mock_notary_keys.len() as u8);

		let block = 555_u64;
		let timestamp = now();
		let return_data = [0x3f_u8; 32];

		// `notarizations[i]` is submitted by the i-th validator (`mock_notary_keys`)
		let notarizations = vec![
			CheckedEthCallResult::Ok(return_data, block, timestamp),
			CheckedEthCallResult::Ok(return_data, block, timestamp),
			CheckedEthCallResult::Ok(return_data, block - 1, timestamp),
			CheckedEthCallResult::Ok(return_data, block, timestamp),
			CheckedEthCallResult::Ok(return_data, block, timestamp + 5),
			CheckedEthCallResult::Ok(return_data, block, timestamp),
			CheckedEthCallResult::Ok(return_data, block, timestamp),
			CheckedEthCallResult::Ok([0x11_u8; 32], block, timestamp),
			CheckedEthCallResult::Ok(return_data, block, timestamp),
		];
		// expected aggregated count after the i-th notarization
		let expected_aggregations = vec![
			Some(1_u32),
			Some(2),
			Some(1), // block # differs, count separately
			Some(3),
			Some(1), // timestamp differs, count separately
			Some(4),
			Some(5),
			Some(1), // return_data differs, count separately
			None,    // success callback & storage is reset after 6th notarization (2/3 * 9 = 6)
		];

		// aggregate the notarizations
		for ((notary_result, notary_pk), aggregation) in
			notarizations.iter().zip(mock_notary_keys).zip(expected_aggregations)
		{
			assert_ok!(EthBridge::handle_call_notarization(call_id, *notary_result, &notary_pk));

			// assert notarization progress
			let aggregated_notarizations =
				EthBridge::eth_call_notarizations_aggregated(call_id).unwrap_or_default();
			println!("{:?}", aggregated_notarizations);
			assert_eq!(aggregated_notarizations.get(&notary_result).map(|x| *x), aggregation);
		}

		// callback triggered with correct value
		assert_eq!(MockEthCallSubscriber::success_result(), Some((call_id, notarizations[0])),);
	});
}

#[test]
fn handle_call_notarization_aborts_no_consensus() {
	// Given in-progress notarizations such that there cannot be consensus even from uncounted
	// notarizations When aggregating the notarizations
	// Then it triggers the failure callback

	// fake ecdsa public keys to represent the mocked validators
	let mock_notary_keys: Vec<<TestRuntime as Config>::EthyId> = (1_u8..=6_u8)
		.map(|k| <TestRuntime as Config>::EthyId::from_slice(&[k; 33]).unwrap())
		.collect();
	ExtBuilder::default().build().execute_with(|| {
		let call_id = 1_u64;
		EthCallRequestInfo::insert(call_id, CheckedEthCallRequest::default());
		MockValidatorSet::mock_n_validators(mock_notary_keys.len() as u8);
		let block = 555_u64;
		let timestamp = now();
		let return_data = [0x3f_u8; 32];

		// `notarizations[i]` is submitted by the i-th validator (`mock_notary_keys`)
		let notarizations = vec![
			CheckedEthCallResult::Ok(return_data, block, timestamp),
			CheckedEthCallResult::Ok(return_data, block, timestamp - 1),
			CheckedEthCallResult::Ok(return_data, block, timestamp - 2),
			CheckedEthCallResult::Ok(return_data, block, timestamp),
			CheckedEthCallResult::DataProviderErr,
			CheckedEthCallResult::DataProviderErr,
		];
		// expected aggregated count after the i-th notarization
		let expected_aggregations = vec![
			Some(1_u32),
			Some(1),
			Some(1),
			Some(2),
			None, /* after counting 4th notarization the system realizes consensus is impossible
			       * and triggers failure callback, clearing storage */
			None, /* this notarization is be (no longer tracked by the system after the previous
			       * notarization) */
		];

		// aggregate the notarizations
		for (idx, ((notary_result, notary_pk), aggregation)) in notarizations
			.iter()
			.zip(mock_notary_keys)
			.zip(expected_aggregations)
			.enumerate()
		{
			if idx == 5 {
				// handling the (5th) notarization triggers failure as reaching consensus is no
				// longer possible this (6th) notarization is effectively ignored
				assert_noop!(
					EthBridge::handle_call_notarization(call_id, *notary_result, &notary_pk),
					Error::<TestRuntime>::InvalidClaim
				);
			} else {
				// normal case the notarization is counted
				assert_ok!(EthBridge::handle_call_notarization(
					call_id,
					*notary_result,
					&notary_pk
				));
			}

			// assert notarization progress
			let aggregated_notarizations =
				EthBridge::eth_call_notarizations_aggregated(call_id).unwrap_or_default();
			println!("{:?}", aggregated_notarizations);
			assert_eq!(aggregated_notarizations.get(&notary_result).map(|x| *x), aggregation);
		}

		// failure callback triggered with correct value
		assert_eq!(
			MockEthCallSubscriber::failed_result(),
			Some((call_id, EthCallFailure::Internal)),
		);
	});
}

#[test]
fn test_prune_event_ids() {
	let test_vec = vec![1,2,3,4,6,7];
	assert_eq!(prune_event_ids(test_vec), vec![4, 6, 7])
}
