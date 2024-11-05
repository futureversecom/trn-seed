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

use crate::{
	mock::*,
	types::{
		BridgePauseStatus, CheckedEthCallRequest, CheckedEthCallResult, EthAddress, EthBlock,
		EthHash, EthereumEventInfo, EthySigningRequest, EventClaim, EventClaimResult, EventProofId,
		TransactionReceipt,
	},
	AuthoritiesChangedThisEra, BridgePaused, ChallengePeriod, ChallengerAccount, Config,
	ContractAddress, DelayedEventProofsPerBlock, Error, EthCallNotarizationsAggregated,
	EthCallRequestInfo, Event, EventClaimStatus, MessagesValidAt, MissedMessageIds,
	NextAuthorityChange, NextEventProofId, NextNotaryKeys, NotaryKeys, NotarySetId,
	NotarySetProofId, NotaryXrplKeys, Pallet, PendingClaimChallenges, PendingClaimStatus,
	PendingEventClaims, PendingEventProofs, ProcessedMessageIds, Relayer, RelayerPaidBond,
	XrplDoorSigners, XrplNotarySetProofId, ETHY_ENGINE_ID, SUBMIT_BRIDGE_EVENT_SELECTOR,
};
use codec::Encode;
use ethabi::Token;
use frame_support::traits::{fungibles::Inspect, Hooks, OneSessionHandler, UnixTime};
use hex_literal::hex;
use seed_pallet_common::test_prelude::*;
use seed_primitives::{
	ethy::{
		crypto::AuthorityId, ConsensusLog, EthyChainId, EthyEcdsaToEthereum, EventClaimId,
		ValidatorSet,
	},
	xrpl::XrplAccountId,
};
use sp_core::{bounded::WeakBoundedVec, ByteArray};
use sp_keystore::{testing::MemoryKeystore, Keystore};
use sp_runtime::{
	generic::DigestItem,
	traits::{AccountIdConversion, Convert},
	Percent, RuntimeAppPublic, SaturatedConversion,
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

/// Ethereum ABI encode validator set message
fn encode_validator_set_message(validator_set: &[AuthorityId], token_id: u64) -> Vec<u8> {
	ethabi::encode(&[
		Token::Array(
			validator_set
				.iter()
				.map(|k| {
					let address: [u8; 20] = EthyEcdsaToEthereum::convert(k.as_slice());
					Token::Address(address.into())
				})
				.collect(),
		),
		Token::Uint(token_id.into()),
	])
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
			RuntimeOrigin::signed(relayer.into()),
			tx_hash,
			event_data.clone(),
		));

		let process_at = System::block_number() + ChallengePeriod::<Test>::get();
		assert_eq!(
			PendingEventClaims::<Test>::get(event_id),
			Some(EventClaim {
				tx_hash,
				source,
				destination,
				data: BoundedVec::truncate_from(message.to_vec())
			})
		);
		assert_eq!(MessagesValidAt::<Test>::get(process_at).into_inner(), [event_id],);
	});
}

#[test]
fn submit_event_relayer_only() {
	ExtBuilder::default().build().execute_with(|| {
		let not_relayer = H160::from_low_u64_be(11);
		assert_noop!(
			EthBridge::submit_event(
				RuntimeOrigin::signed(not_relayer.into()),
				H256::default(),
				vec![]
			),
			Error::<Test>::NoPermission
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
			EthBridge::submit_event(RuntimeOrigin::signed(relayer.into()), tx_hash, event_data),
			Error::<Test>::InvalidClaim
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
			RuntimeOrigin::signed(relayer.into()),
			tx_hash,
			event_data.clone(),
		));

		assert_noop!(
			EthBridge::submit_event(RuntimeOrigin::signed(relayer.into()), tx_hash, event_data),
			Error::<Test>::EventReplayPending
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
			RuntimeOrigin::signed(relayer.into()),
			tx_hash,
			event_data.clone(),
		));

		// Process the message
		let process_at = System::block_number() + ChallengePeriod::<Test>::get();
		EthBridge::on_initialize(process_at);

		assert_noop!(
			EthBridge::submit_event(RuntimeOrigin::signed(relayer.into()), tx_hash, event_data),
			Error::<Test>::EventReplayProcessed
		);
	});
}

#[test]
fn submit_missing_event_id_works() {
	let relayer = H160::from_low_u64_be(123);
	ExtBuilder::default().relayer(relayer).build().execute_with(|| {
		let missed_event_id: u64 = 3;
		// Setup storage to simulate missed_event_id being invalid but stored in Missing
		ProcessedMessageIds::<Test>::put(BoundedVec::truncate_from(vec![missed_event_id + 1]));
		MissedMessageIds::<Test>::put(vec![missed_event_id]);
		let tx_hash = EthHash::from_low_u64_be(33);
		let (source, destination, message) =
			(H160::from_low_u64_be(555), H160::from_low_u64_be(555), &[1_u8, 2, 3, 4, 5]);
		let event_item_data = encode_event_message(missed_event_id, source, destination, message);

		// Submit event should fail due to replay protection
		assert_noop!(
			EthBridge::submit_event(
				RuntimeOrigin::signed(relayer.into()),
				tx_hash,
				event_item_data.clone(),
			),
			Error::<Test>::EventReplayProcessed
		);

		// Submit missing event should work as event id is stored within MissedMessageIds
		assert_ok!(EthBridge::submit_missing_event(
			RuntimeOrigin::signed(relayer.into()),
			tx_hash,
			event_item_data.clone(),
		));

		assert_eq!(
			PendingEventClaims::<Test>::get(missed_event_id),
			Some(EventClaim {
				tx_hash,
				source,
				destination,
				data: BoundedVec::truncate_from(message.to_vec())
			})
		);

		let process_at = System::block_number() + ChallengePeriod::<Test>::get();
		EthBridge::on_initialize(process_at);
		// ProcessedMessageIds should be unchanged (As it was before the lowest
		assert_eq!(ProcessedMessageIds::<Test>::get().into_inner(), vec![missed_event_id + 1]);
		assert!(MissedMessageIds::<Test>::get().is_empty());

		// Submit missing event should now not work as the event is processed
		assert_noop!(
			EthBridge::submit_missing_event(
				RuntimeOrigin::signed(relayer.into()),
				tx_hash,
				event_item_data.clone(),
			),
			Error::<Test>::EventReplayProcessed
		);
	});
}

#[test]
fn submit_missing_event_id_prevents_replay() {
	let relayer = H160::from_low_u64_be(123);
	ExtBuilder::default().relayer(relayer).build().execute_with(|| {
		let missed_event_id: u64 = 3;
		// Setup storage to simulate missed_event_id being invalid but stored in Missing
		ProcessedMessageIds::<Test>::put(BoundedVec::truncate_from(vec![missed_event_id + 1]));
		MissedMessageIds::<Test>::put(vec![missed_event_id]);
		let tx_hash = EthHash::from_low_u64_be(33);
		let (source, destination, message) =
			(H160::from_low_u64_be(555), H160::from_low_u64_be(555), &[1_u8, 2, 3, 4, 5]);
		let event_item_data = encode_event_message(missed_event_id, source, destination, message);

		// Submit missing event should work as event id is stored within MissedMessageIds
		assert_ok!(EthBridge::submit_missing_event(
			RuntimeOrigin::signed(relayer.into()),
			tx_hash,
			event_item_data.clone(),
		));

		// Submit missing event should now not work as the event is pending
		assert_noop!(
			EthBridge::submit_missing_event(
				RuntimeOrigin::signed(relayer.into()),
				tx_hash,
				event_item_data.clone(),
			),
			Error::<Test>::EventReplayPending
		);
	});
}

#[test]
fn submit_missing_event_id_fails_if_not_in_missing() {
	let relayer = H160::from_low_u64_be(123);
	ExtBuilder::default().relayer(relayer).build().execute_with(|| {
		let missed_event_id: u64 = 3;
		// Setup storage to simulate missed_event_id not in missing (i.e. processed
		ProcessedMessageIds::<Test>::put(BoundedVec::truncate_from(vec![5]));
		MissedMessageIds::<Test>::put(vec![2, 4]);
		let tx_hash = EthHash::from_low_u64_be(33);
		let (source, destination, message) =
			(H160::from_low_u64_be(555), H160::from_low_u64_be(555), &[1_u8, 2, 3, 4, 5]);
		let event_item_data = encode_event_message(missed_event_id, source, destination, message);

		// Submit missing event should not work as it is not in MissedMessageIds
		assert_noop!(
			EthBridge::submit_missing_event(
				RuntimeOrigin::signed(relayer.into()),
				tx_hash,
				event_item_data.clone(),
			),
			Error::<Test>::EventReplayProcessed
		);

		assert_noop!(
			EthBridge::submit_event(
				RuntimeOrigin::signed(relayer.into()),
				tx_hash,
				event_item_data.clone(),
			),
			Error::<Test>::EventReplayProcessed
		);
	});
}

#[test]
fn remove_missing_event_id_works() {
	ExtBuilder::default().build().execute_with(|| {
		let event_ids = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
		MissedMessageIds::<Test>::put(event_ids);

		let range = (3, 6);
		assert_ok!(EthBridge::remove_missing_event_id(RuntimeOrigin::root(), range));
		let event_ids = vec![1, 2, 7, 8, 9];
		assert_eq!(MissedMessageIds::<Test>::get(), event_ids);
	});
}

#[test]
fn remove_missing_event_id_single_id() {
	ExtBuilder::default().build().execute_with(|| {
		let event_ids = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
		MissedMessageIds::<Test>::put(event_ids);

		let range = (3, 3); // Remove one ID
		assert_ok!(EthBridge::remove_missing_event_id(RuntimeOrigin::root(), range));
		let event_ids = vec![1, 2, 4, 5, 6, 7, 8, 9];
		assert_eq!(MissedMessageIds::<Test>::get(), event_ids);
	});
}

#[test]
fn remove_missing_event_id_outside_range() {
	ExtBuilder::default().build().execute_with(|| {
		let event_ids = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
		MissedMessageIds::<Test>::put(&event_ids);

		let range = (10, 1000000000); // Outside range is noop
		assert_ok!(EthBridge::remove_missing_event_id(RuntimeOrigin::root(), range));
		assert_eq!(MissedMessageIds::<Test>::get(), event_ids);
	});
}

#[test]
fn remove_missing_event_id_full_range() {
	ExtBuilder::default().build().execute_with(|| {
		let event_ids = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
		MissedMessageIds::<Test>::put(&event_ids);

		let range = (1, 9); // Full range of ids
		assert_ok!(EthBridge::remove_missing_event_id(RuntimeOrigin::root(), range));
		assert!(MissedMessageIds::<Test>::get().is_empty());
	});
}

#[test]
fn set_relayer_no_bond_should_fail() {
	let relayer = H160::from_low_u64_be(123);
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			EthBridge::set_relayer(frame_system::RawOrigin::Root.into(), relayer.into()),
			Error::<Test>::NoBondPaid
		);
	});
}

#[test]
fn deposit_relayer_bond_works() {
	let relayer = H160::from_low_u64_be(123);
	ExtBuilder::default()
		.with_endowed_account(relayer, RelayerBond::get())
		.build()
		.execute_with(|| {
			assert_ok!(EthBridge::deposit_relayer_bond(RuntimeOrigin::signed(relayer.into())));
			assert_eq!(
				AssetsExt::hold_balance(&BridgePalletId::get(), &relayer.into(), &ROOT_ASSET_ID),
				RelayerBond::get()
			);

			// Subsequent deposits should fail
			assert_noop!(
				EthBridge::deposit_relayer_bond(RuntimeOrigin::signed(relayer.into())),
				Error::<Test>::CantBondRelayer
			);

			// Setting relayer should work
			assert_ok!(EthBridge::set_relayer(
				frame_system::RawOrigin::Root.into(),
				relayer.into()
			));
			assert_eq!(
				AssetsExt::hold_balance(&BridgePalletId::get(), &relayer.into(), &ROOT_ASSET_ID),
				RelayerBond::get()
			);

			// Check storage
			assert_eq!(RelayerPaidBond::<Test>::get(AccountId::from(relayer)), RelayerBond::get());
			assert_eq!(Relayer::<Test>::get(), Some(relayer.into()));
		});
}

#[test]
fn deposit_relayer_bond_no_balance_should_fail() {
	let relayer = H160::from_low_u64_be(123);
	ExtBuilder::default().build().execute_with(|| {
		// Subsequent deposits should fail
		assert_noop!(
			EthBridge::deposit_relayer_bond(RuntimeOrigin::signed(relayer.into())),
			ArithmeticError::Underflow
		);
	});
}

#[test]
fn withdraw_relayer_bond_works() {
	let relayer = H160::from_low_u64_be(123);
	ExtBuilder::default()
		.with_endowed_account(relayer, RelayerBond::get())
		.build()
		.execute_with(|| {
			// Withdraw with no bond set should fail
			assert_noop!(
				EthBridge::withdraw_relayer_bond(RuntimeOrigin::signed(relayer.into())),
				Error::<Test>::CantUnbondRelayer
			);

			// Submit bond
			assert_ok!(EthBridge::deposit_relayer_bond(RuntimeOrigin::signed(relayer.into())));
			assert_eq!(RelayerPaidBond::<Test>::get(AccountId::from(relayer)), RelayerBond::get());

			// Withdraw bond
			assert_ok!(EthBridge::withdraw_relayer_bond(RuntimeOrigin::signed(relayer.into())));

			// Check storage
			assert_eq!(RelayerPaidBond::<Test>::get(AccountId::from(relayer)), 0);
		});
}

#[test]
fn withdraw_active_relayer_bond_should_fail() {
	let relayer = H160::from_low_u64_be(123);
	ExtBuilder::default()
		.with_endowed_account(relayer, RelayerBond::get())
		.build()
		.execute_with(|| {
			// Submit bond
			assert_ok!(EthBridge::deposit_relayer_bond(RuntimeOrigin::signed(relayer.into())));

			// Setting relayer should work
			assert_ok!(EthBridge::set_relayer(
				frame_system::RawOrigin::Root.into(),
				relayer.into()
			));

			// Withdraw bond
			assert_noop!(
				EthBridge::withdraw_relayer_bond(RuntimeOrigin::signed(relayer.into())),
				Error::<Test>::CantUnbondRelayer
			);
		});
}

#[test]
fn submit_challenge() {
	let relayer = H160::from_low_u64_be(123);
	let challenger = H160::from_low_u64_be(1234);
	let tx_hash = EthHash::from_low_u64_be(33);
	let (event_id, source, destination, message) =
		(1_u64, H160::from_low_u64_be(555), H160::from_low_u64_be(555), &[1_u8, 2, 3, 4, 5]);
	let event_data = encode_event_message(event_id, source, destination, message);

	ExtBuilder::default()
		.relayer(relayer)
		.with_endowed_account(challenger, ChallengerBond::get())
		.build()
		.execute_with(|| {
			// No event claim should fail
			assert_noop!(
				EthBridge::submit_challenge(RuntimeOrigin::signed(challenger.into()), event_id),
				Error::<Test>::NoClaim
			);

			// Submit event
			assert_ok!(EthBridge::submit_event(
				RuntimeOrigin::signed(relayer.into()),
				tx_hash,
				event_data.clone(),
			));

			// Submit challenge
			assert_ok!(EthBridge::submit_challenge(
				RuntimeOrigin::signed(challenger.into()),
				event_id
			));
			assert_eq!(PendingClaimChallenges::<Test>::get(), vec![event_id]);
			assert_eq!(
				ChallengerAccount::<Test>::get(event_id),
				Some((AccountId::from(challenger), ChallengerBond::get()))
			);
			assert_eq!(
				PendingClaimStatus::<Test>::get(event_id),
				Some(EventClaimStatus::Challenged)
			);

			// Subsequent challenges on the same event_id should fail
			assert_noop!(
				EthBridge::submit_challenge(RuntimeOrigin::signed(challenger.into()), event_id),
				Error::<Test>::ClaimAlreadyChallenged
			);
		});
}

#[test]
fn submit_challenge_no_balance_should_fail() {
	let relayer = H160::from_low_u64_be(123);
	let challenger = H160::from_low_u64_be(1234);
	let tx_hash = EthHash::from_low_u64_be(33);
	let (event_id, source, destination, message) =
		(1_u64, H160::from_low_u64_be(555), H160::from_low_u64_be(555), &[1_u8, 2, 3, 4, 5]);
	let event_data = encode_event_message(event_id, source, destination, message);

	ExtBuilder::default().relayer(relayer).build().execute_with(|| {
		// Submit event
		assert_ok!(EthBridge::submit_event(
			RuntimeOrigin::signed(relayer.into()),
			tx_hash,
			event_data.clone(),
		));

		// Submit challenge with no balance should fail
		assert_noop!(
			EthBridge::submit_challenge(RuntimeOrigin::signed(challenger.into()), event_id),
			TokenError::FundsUnavailable
		);
	});
}

#[test]
fn handle_event_notarization_valid_claims() {
	let relayer = H160::from_low_u64_be(123);
	let challenger = H160::from_low_u64_be(1234);
	// First event data
	let tx_hash_1 = EthHash::from_low_u64_be(33);
	let (event_id_1, source_1, destination_1, message_1) =
		(1_u64, H160::from_low_u64_be(555), H160::from_low_u64_be(555), &[1_u8, 2, 3, 4, 5]);
	let event_data_1 = encode_event_message(event_id_1, source_1, destination_1, message_1);
	// Second event data
	let tx_hash_2 = EthHash::from_low_u64_be(33);
	let (event_id_2, source_2, destination_2, message_2) =
		(2_u64, H160::from_low_u64_be(666), H160::from_low_u64_be(666), &[1_u8, 2, 3, 4, 6]);
	let event_data_2 = encode_event_message(event_id_2, source_2, destination_2, message_2);

	// fake ecdsa public keys to represent the mocked validators
	let mock_notary_keys: Vec<<Test as Config>::EthyId> = (1_u8..=9_u8)
		.map(|k| <Test as Config>::EthyId::from_slice(&[k; 33]).unwrap())
		.collect();

	ExtBuilder::default()
		.relayer(relayer)
		.with_endowed_account(challenger, ChallengerBond::get() * 2)
		.build()
		.execute_with(|| {
			MockValidatorSet::mock_n_validators(mock_notary_keys.len() as u8);
			let process_at = System::block_number() + ChallengePeriod::<Test>::get();

			// Submit Event 1
			assert_ok!(EthBridge::submit_event(
				RuntimeOrigin::signed(relayer.into()),
				tx_hash_1,
				event_data_1.clone(),
			));
			assert_eq!(
				PendingClaimStatus::<Test>::get(event_id_1),
				Some(EventClaimStatus::Pending)
			);
			// Submit Event 2
			assert_ok!(EthBridge::submit_event(
				RuntimeOrigin::signed(relayer.into()),
				tx_hash_2,
				event_data_2.clone(),
			));
			assert_eq!(
				PendingClaimStatus::<Test>::get(event_id_2),
				Some(EventClaimStatus::Pending)
			);

			// Submit challenge 1
			assert_ok!(EthBridge::submit_challenge(
				RuntimeOrigin::signed(challenger.into()),
				event_id_1
			));
			// Submit challenge 2
			assert_ok!(EthBridge::submit_challenge(
				RuntimeOrigin::signed(challenger.into()),
				event_id_2
			));
			// Check storage
			assert_eq!(PendingClaimChallenges::<Test>::get(), vec![event_id_1, event_id_2]);
			assert_eq!(
				PendingClaimStatus::<Test>::get(event_id_1),
				Some(EventClaimStatus::Challenged)
			);
			assert_eq!(
				PendingClaimStatus::<Test>::get(event_id_2),
				Some(EventClaimStatus::Challenged)
			);

			let mut yay_count: usize = 0;
			let notary_count: usize = mock_notary_keys.len();
			// Submit valid notarization for all 9 validators
			// When the yay_count reaches over the NotarizationThreshold of 66% the storage should
			// be updated
			for i in 0..9 {
				if Percent::from_rational(yay_count as u64, notary_count as u64)
					>= <Test as Config>::NotarizationThreshold::get()
				{
					// Any further notarizations should return InvalidClaim error
					assert_noop!(
						EthBridge::handle_event_notarization(
							event_id_2,
							EventClaimResult::Valid,
							&mock_notary_keys[i]
						),
						Error::<Test>::InvalidClaim
					);
				} else {
					assert_ok!(EthBridge::handle_event_notarization(
						event_id_2,
						EventClaimResult::Valid,
						&mock_notary_keys[i]
					));
				}
				yay_count += 1;

				if Percent::from_rational(yay_count as u64, notary_count as u64)
					>= <Test as Config>::NotarizationThreshold::get()
				{
					// Over threshold, storage should be updated
					assert_eq!(PendingClaimChallenges::<Test>::get(), vec![event_id_1]);
					assert_eq!(
						PendingClaimStatus::<Test>::get(event_id_2),
						Some(EventClaimStatus::ProvenValid)
					);
				} else {
					// Under threshold, storage not updated
					assert_eq!(PendingClaimChallenges::<Test>::get(), vec![event_id_1, event_id_2]);
					assert_eq!(
						PendingClaimStatus::<Test>::get(event_id_2),
						Some(EventClaimStatus::Challenged)
					);
				}
			}

			// Check claim remains in storage so it can still be processed
			assert_eq!(
				PendingEventClaims::<Test>::get(event_id_2),
				Some(EventClaim {
					source: source_2,
					destination: destination_2,
					tx_hash: tx_hash_2,
					data: BoundedVec::truncate_from(message_2.to_vec())
				})
			);
			assert_eq!(MessagesValidAt::<Test>::get(process_at), vec![event_id_1, event_id_2]);
		});
}

#[test]
/// Check whether an event that was challenged and proven to be valid is still processed
fn process_valid_challenged_event() {
	let relayer = H160::from_low_u64_be(123);
	let challenger = H160::from_low_u64_be(1234);
	// First event data
	let tx_hash_1 = EthHash::from_low_u64_be(33);
	let (event_id_1, source_1, destination_1, message_1) =
		(1_u64, H160::from_low_u64_be(555), H160::from_low_u64_be(555), &[1_u8, 2, 3, 4, 5]);
	let event_data_1 = encode_event_message(event_id_1, source_1, destination_1, message_1);
	// fake ecdsa public keys to represent the mocked validators
	let mock_notary_keys: Vec<<Test as Config>::EthyId> = (1_u8..=9_u8)
		.map(|k| <Test as Config>::EthyId::from_slice(&[k; 33]).unwrap())
		.collect();

	ExtBuilder::default()
		.relayer(relayer)
		.with_endowed_account(challenger, ChallengerBond::get())
		.build()
		.execute_with(|| {
			MockValidatorSet::mock_n_validators(mock_notary_keys.len() as u8);
			assert_eq!(AssetsExt::balance(ROOT_ASSET_ID, &relayer.into()), 0);
			assert_eq!(RelayerPaidBond::<Test>::get(AccountId::from(relayer)), RelayerBond::get());

			let process_at = System::block_number() + ChallengePeriod::<Test>::get();

			// Submit Event 1
			assert_ok!(EthBridge::submit_event(
				RuntimeOrigin::signed(relayer.into()),
				tx_hash_1,
				event_data_1.clone(),
			));

			// Submit challenge 1
			assert_ok!(EthBridge::submit_challenge(
				RuntimeOrigin::signed(challenger.into()),
				event_id_1
			));
			assert_eq!(
				ChallengerAccount::<Test>::get(event_id_1),
				Some((AccountId::from(challenger), ChallengerBond::get()))
			);

			// Submit valid notarization for all 9 validators
			for i in 0..mock_notary_keys.len() {
				// We test the returned value in the previous test
				let _ = EthBridge::handle_event_notarization(
					event_id_1,
					EventClaimResult::Valid,
					&mock_notary_keys[i],
				);
			}

			// Check balances of relayer and challenger
			// Challenger should have no bond and no balance
			assert_eq!(
				AssetsExt::hold_balance(&BridgePalletId::get(), &challenger.into(), &ROOT_ASSET_ID),
				0
			);
			assert_eq!(AssetsExt::balance(ROOT_ASSET_ID, &challenger.into()), 0);
			assert!(ChallengerAccount::<Test>::get(event_id_1).is_none());

			// Relayer should still have bond and challenger bond as balance
			assert_eq!(
				AssetsExt::hold_balance(&BridgePalletId::get(), &relayer.into(), &ROOT_ASSET_ID),
				RelayerBond::get()
			);
			assert_eq!(RelayerPaidBond::<Test>::get(AccountId::from(relayer)), RelayerBond::get());
			assert_eq!(AssetsExt::balance(ROOT_ASSET_ID, &relayer.into()), ChallengerBond::get());

			// Check claim remains in storage so it can still be processed
			assert_eq!(
				PendingClaimStatus::<Test>::get(event_id_1),
				Some(EventClaimStatus::ProvenValid)
			);
			assert_eq!(
				PendingEventClaims::<Test>::get(event_id_1),
				Some(EventClaim {
					source: source_1,
					destination: destination_1,
					tx_hash: tx_hash_1,
					data: BoundedVec::truncate_from(message_1.to_vec())
				})
			);
			assert_eq!(MessagesValidAt::<Test>::get(process_at), vec![event_id_1]);

			// Weight returned should include the 1000 that we specified in MockEventRouter
			let expected_weight =
				DbWeight::get().reads_writes(5u64, 3u64) + Weight::from_all(1000u64);
			assert_eq!(EthBridge::on_initialize(process_at), expected_weight);

			// Storage should now be fully cleared
			assert!(PendingClaimChallenges::<Test>::get().is_empty());
			assert!(ChallengerAccount::<Test>::get(event_id_1).is_none());
			assert!(PendingEventClaims::<Test>::get(event_id_1).is_none());
			assert!(PendingClaimStatus::<Test>::get(event_id_1).is_none());
			assert!(MessagesValidAt::<Test>::get(process_at).is_empty());
			// The event is processed!
			assert_eq!(ProcessedMessageIds::<Test>::get().into_inner(), vec![event_id_1]);
		});
}

#[test]
/// Check whether an event that was challenged and proven to be valid but reaches the process block
/// before a consensus is reached is processed after extended by the challenge period
fn process_valid_challenged_event_delayed() {
	let relayer = H160::from_low_u64_be(123);
	let challenger = H160::from_low_u64_be(1234);
	// First event data
	let tx_hash_1 = EthHash::from_low_u64_be(33);
	let (event_id_1, source_1, destination_1, message_1) =
		(1_u64, H160::from_low_u64_be(555), H160::from_low_u64_be(555), &[1_u8, 2, 3, 4, 5]);
	let event_data_1 = encode_event_message(event_id_1, source_1, destination_1, message_1);
	// fake ecdsa public keys to represent the mocked validators
	let mock_notary_keys: Vec<<Test as Config>::EthyId> = (1_u8..=9_u8)
		.map(|k| <Test as Config>::EthyId::from_slice(&[k; 33]).unwrap())
		.collect();

	ExtBuilder::default()
		.relayer(relayer)
		.with_endowed_account(challenger, ChallengerBond::get() * 2)
		.build()
		.execute_with(|| {
			MockValidatorSet::mock_n_validators(mock_notary_keys.len() as u8);
			// The block it should be processed at
			let process_at = System::block_number() + ChallengePeriod::<Test>::get();
			// The actual block it will be processed at
			let process_at_extended = process_at + ChallengePeriod::<Test>::get();
			// Submit Event 1
			assert_ok!(EthBridge::submit_event(
				RuntimeOrigin::signed(relayer.into()),
				tx_hash_1,
				event_data_1.clone(),
			));

			// Submit challenge 1
			assert_ok!(EthBridge::submit_challenge(
				RuntimeOrigin::signed(challenger.into()),
				event_id_1
			));

			assert_eq!(
				PendingClaimStatus::<Test>::get(event_id_1),
				Some(EventClaimStatus::Challenged)
			);

			assert_eq!(MessagesValidAt::<Test>::get(process_at), vec![event_id_1]);

			// Weight returned should not include the 1000 that we specified in MockEventRouter as a
			// consensus has not been reached
			let expected_weight = DbWeight::get().reads_writes(6u64, 3u64);
			assert_eq!(EthBridge::on_initialize(process_at), expected_weight);

			assert_eq!(MessagesValidAt::<Test>::get(process_at_extended), vec![event_id_1]);
			assert!(MessagesValidAt::<Test>::get(process_at).is_empty());

			// Submit valid notarization for all 9 validators
			for i in 0..mock_notary_keys.len() {
				let _ = EthBridge::handle_event_notarization(
					event_id_1,
					EventClaimResult::Valid,
					&mock_notary_keys[i],
				);
			}

			// Check claim remains in storage
			assert_eq!(
				PendingClaimStatus::<Test>::get(event_id_1),
				Some(EventClaimStatus::ProvenValid)
			);
			assert_eq!(
				PendingEventClaims::<Test>::get(event_id_1),
				Some(EventClaim {
					source: source_1,
					destination: destination_1,
					tx_hash: tx_hash_1,
					data: BoundedVec::truncate_from(message_1.to_vec())
				})
			);
			assert_eq!(MessagesValidAt::<Test>::get(process_at_extended), vec![event_id_1]);

			// Weight returned should include the 1000 that we specified in MockEventRouter
			let expected_weight =
				DbWeight::get().reads_writes(5u64, 3u64) + Weight::from_all(1000u64);
			assert_eq!(EthBridge::on_initialize(process_at_extended), expected_weight);

			// Storage should now be fully cleared
			assert!(PendingClaimChallenges::<Test>::get().is_empty());
			assert!(ChallengerAccount::<Test>::get(event_id_1).is_none());
			assert!(PendingEventClaims::<Test>::get(event_id_1).is_none());
			assert!(PendingClaimStatus::<Test>::get(event_id_1).is_none());
			assert!(MessagesValidAt::<Test>::get(process_at_extended).is_empty());
			// The event is processed!
			assert_eq!(ProcessedMessageIds::<Test>::get().into_inner(), vec![event_id_1]);
		});
}

#[test]
fn handle_event_notarization_invalid_claims() {
	let relayer = H160::from_low_u64_be(123);
	let challenger = H160::from_low_u64_be(1234);
	// Event data
	let tx_hash_1 = EthHash::from_low_u64_be(33);
	let (event_id_1, source_1, destination_1, message_1) =
		(1_u64, H160::from_low_u64_be(555), H160::from_low_u64_be(555), &[1_u8, 2, 3, 4, 5]);
	let event_data_1 = encode_event_message(event_id_1, source_1, destination_1, message_1);
	// fake ecdsa public keys to represent the mocked validators
	let mock_notary_keys: Vec<<Test as Config>::EthyId> = (1_u8..=9_u8)
		.map(|k| <Test as Config>::EthyId::from_slice(&[k; 33]).unwrap())
		.collect();

	ExtBuilder::default()
		.relayer(relayer)
		.with_endowed_account(challenger, ChallengerBond::get())
		.build()
		.execute_with(|| {
			MockValidatorSet::mock_n_validators(mock_notary_keys.len() as u8);
			let process_at = System::block_number() + ChallengePeriod::<Test>::get();

			// Submit Event 1
			assert_ok!(EthBridge::submit_event(
				RuntimeOrigin::signed(relayer.into()),
				tx_hash_1,
				event_data_1.clone(),
			));
			assert_eq!(
				PendingClaimStatus::<Test>::get(event_id_1),
				Some(EventClaimStatus::Pending)
			);

			// Submit challenge 1
			assert_ok!(EthBridge::submit_challenge(
				RuntimeOrigin::signed(challenger.into()),
				event_id_1
			));

			// Check storage
			assert_eq!(PendingClaimChallenges::<Test>::get(), vec![event_id_1]);
			assert_eq!(
				PendingClaimStatus::<Test>::get(event_id_1),
				Some(EventClaimStatus::Challenged)
			);

			let mut nay_count: usize = 0;
			let notary_count: usize = mock_notary_keys.len();
			// Submit invalid notarization for all 9 validators
			// When the nay_count reaches over 100 - NotarizationThreshold (33%) the storage should
			// be updated
			for i in 0..9 {
				if Percent::from_rational(nay_count as u64, notary_count as u64)
					> (Percent::from_parts(
						100_u8 - <Test as Config>::NotarizationThreshold::get().deconstruct(),
					)) {
					// further notarizations should return InvalidClaim error
					assert_noop!(
						EthBridge::handle_event_notarization(
							event_id_1,
							EventClaimResult::TxStatusFailed,
							&mock_notary_keys[i]
						),
						Error::<Test>::InvalidClaim
					);
				} else {
					assert_ok!(EthBridge::handle_event_notarization(
						event_id_1,
						EventClaimResult::TxStatusFailed,
						&mock_notary_keys[i]
					));
				}
				nay_count += 1;

				if Percent::from_rational(nay_count as u64, notary_count as u64)
					> (Percent::from_parts(
						100_u8 - <Test as Config>::NotarizationThreshold::get().deconstruct(),
					)) {
					// Over threshold, storage should be removed
					assert!(PendingClaimChallenges::<Test>::get().is_empty());
					assert_eq!(PendingClaimStatus::<Test>::get(event_id_1), None);
				} else {
					// Under threshold, storage not updated
					assert_eq!(PendingClaimChallenges::<Test>::get(), vec![event_id_1]);
					assert_eq!(
						PendingClaimStatus::<Test>::get(event_id_1),
						Some(EventClaimStatus::Challenged)
					);
				}
			}

			// Check claim removed from storage
			assert!(PendingEventClaims::<Test>::get(event_id_1).is_none());
			assert_eq!(MessagesValidAt::<Test>::get(process_at), vec![event_id_1]);

			// Check balances of relayer and challenger
			// Relayer should have no funds and no bond
			assert_eq!(AssetsExt::balance(ROOT_ASSET_ID, &relayer.into()), 0);
			assert_eq!(RelayerPaidBond::<Test>::get(AccountId::from(relayer)), 0);

			// Challenger should have balance of relayer bond + challenger bond
			assert_eq!(
				AssetsExt::balance(ROOT_ASSET_ID, &challenger.into()),
				RelayerBond::get() + ChallengerBond::get()
			);
			assert!(ChallengerAccount::<Test>::get(event_id_1).is_none());
		});
}

#[test]
fn do_event_notarization_ocw_doesnt_change_storage() {
	let relayer = H160::from_low_u64_be(123);
	let challenger = H160::from_low_u64_be(1234);
	// Event data
	let tx_hash_1 = EthHash::from_low_u64_be(33);
	let (event_id_1, source_1, destination_1, message_1) =
		(1_u64, H160::from_low_u64_be(555), H160::from_low_u64_be(555), &[1_u8, 2, 3, 4, 5]);
	let event_data_1 = encode_event_message(event_id_1, source_1, destination_1, message_1);

	ExtBuilder::default()
		.relayer(relayer)
		.with_endowed_account(challenger, ChallengerBond::get())
		.with_keystore()
		.build()
		.execute_with(|| {
			// Submit Event 1
			assert_ok!(EthBridge::submit_event(
				RuntimeOrigin::signed(relayer.into()),
				tx_hash_1,
				event_data_1.clone(),
			));

			// Submit challenge 1
			assert_ok!(EthBridge::submit_challenge(
				RuntimeOrigin::signed(challenger.into()),
				event_id_1
			));
			// Check storage
			assert_eq!(PendingClaimChallenges::<Test>::get(), vec![event_id_1]);

			// Generate public key using same authority id and seed as the mock
			let keystore = MemoryKeystore::new();
			Keystore::ecdsa_generate_new(&keystore, AuthorityId::ID, None).unwrap();
			let public_key =
				*Keystore::ecdsa_public_keys(&keystore, AuthorityId::ID).get(0).unwrap();
			let current_set_id = NotarySetId::<Test>::get();

			// Check no storage is changed
			assert_storage_noop!(EthBridge::do_event_notarization_ocw(
				&public_key.into(),
				current_set_id.saturated_into()
			));
		});
}

#[test]
fn pre_last_session_change() {
	ExtBuilder::default().next_session_final().build().execute_with(|| {
		let next_keys = vec![
			AuthorityId::from_slice(
				hex!("03e2161ca58ac2f2fa7dfd9f6980fdda1059b467e375ee78cdd5749dc058c0b2c9")
					.as_slice(),
			)
			.unwrap(),
			AuthorityId::from_slice(
				hex!("02276503736589d21316da95a46d82b2d5c7aa10b946abbdeb01728d7cb935235e")
					.as_slice(),
			)
			.unwrap(),
		];
		let event_proof_id = NextEventProofId::<Test>::get();
		_ = NotarySetId::<Test>::get() + 1;
		// Manually insert next keys
		NextNotaryKeys::<Test>::put(WeakBoundedVec::try_from(next_keys.clone()).unwrap());

		// Manually call handle_authorities_change to simulate 5 minutes before the next epoch
		EthBridge::handle_authorities_change();

		// signing request to prove validator change on other chain
		let new_validator_set_message = encode_validator_set_message(&next_keys, 1_u64);

		let signing_request = EthySigningRequest::Ethereum(EthereumEventInfo {
			event_proof_id,
			validator_set_id: 0,
			source: BridgePalletId::get().into_account_truncating(),
			destination: ContractAddress::<Test>::get(),
			message: BoundedVec::truncate_from(new_validator_set_message.to_vec()),
		});

		System::assert_has_event(
			Event::<Test>::EventSend { event_proof_id, signing_request: signing_request.clone() }
				.into(),
		);
		assert_eq!(
			System::digest().logs[0],
			DigestItem::Consensus(
				ETHY_ENGINE_ID,
				ConsensusLog::OpaqueSigningRequest::<AuthorityId> {
					chain_id: EthyChainId::Ethereum,
					event_proof_id,
					data: signing_request.data(),
				}
				.encode(),
			),
		);

		assert_eq!(NextNotaryKeys::<Test>::get(), next_keys);
		assert_eq!(NotarySetProofId::<Test>::get(), event_proof_id);
		assert_eq!(NextEventProofId::<Test>::get(), event_proof_id + 1);
	});
}

#[test]
fn on_new_session_updates_keys() {
	ExtBuilder::default().next_session_final().build().execute_with(|| {
		let default_account = AccountId::default();
		let next_keys = vec![
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
		];
		let next_keys_iter = vec![
			(&default_account, AuthorityId::from_slice(&[3_u8; 33]).unwrap()),
			(&default_account, AuthorityId::from_slice(&[4_u8; 33]).unwrap()),
		]
		.into_iter();

		// Call on_new_session but is_active_session_final is false
		<EthBridge as OneSessionHandler<AccountId>>::on_new_session(
			true,
			next_keys_iter.clone(),
			next_keys_iter.clone(),
		);
		// Storage remains unchanged
		assert_eq!(
			NextNotaryKeys::<Test>::get(),
			next_keys_iter.clone().map(|(&_acc, pk)| pk).collect::<Vec<AuthorityId>>()
		);
		assert!(NextAuthorityChange::<Test>::get().is_none());

		let block_number: BlockNumber = 100;
		System::set_block_number(block_number);
		// Call on_new_session where is_active_session_final is true, should change storage
		<EthBridge as OneSessionHandler<AccountId>>::on_new_session(
			true,
			next_keys_iter.clone(),
			next_keys_iter.clone(),
		);
		let epoch_duration: BlockNumber = EpochDuration::get().saturated_into();
		let expected_block: BlockNumber = block_number + epoch_duration - 75_u64;
		assert_eq!(NextAuthorityChange::<Test>::get(), Some(expected_block as u64));
		assert_eq!(NextNotaryKeys::<Test>::get(), next_keys.clone());

		let event_proof_id = NextEventProofId::<Test>::get();
		let next_validator_set_id = NotarySetId::<Test>::get() + 1;
		// Now call on_initialize with the expected block to check it gets processed correctly
		EthBridge::on_initialize(expected_block);

		// Storage updated
		assert_eq!(NotarySetProofId::<Test>::get(), event_proof_id);
		assert_eq!(NextEventProofId::<Test>::get(), event_proof_id + 1);
		assert!(NextAuthorityChange::<Test>::get().is_none());
		assert!(AuthoritiesChangedThisEra::<Test>::get());
		// only one log is deposited when the next_authority_change is executed - this is the
		// Ethereum proof request. Xrpl proof request is not made since no change in xrpl notary
		// keys. Also no ConsensusLog::AuthoritiesChange yet
		assert_eq!(System::digest().logs.len(), 1);
		// signing request to prove validator change on Ethereum chain
		let new_validator_set_message = encode_validator_set_message(&next_keys, 1_u64);
		let new_validator_set_message: BoundedVec<u8, MaxEthData> =
			BoundedVec::truncate_from(new_validator_set_message.to_vec());
		let signing_request = EthySigningRequest::Ethereum(EthereumEventInfo {
			event_proof_id,
			validator_set_id: 0,
			source: BridgePalletId::get().into_account_truncating(),
			destination: ContractAddress::<Test>::get(),
			message: new_validator_set_message,
		});
		assert_eq!(
			System::digest().logs[0],
			DigestItem::Consensus(
				ETHY_ENGINE_ID,
				ConsensusLog::OpaqueSigningRequest::<AuthorityId> {
					chain_id: EthyChainId::Ethereum,
					event_proof_id,
					data: signing_request.data(),
				}
				.encode(),
			),
		);

		// Calling on_before_session_ending should NOT call handle_authorities_change again,
		// but do_finalise_authorities_change() will add ConsensusLog::AuthoritiesChange
		// notification log to the header
		<Pallet<Test> as OneSessionHandler<AccountId>>::on_before_session_ending();
		assert_eq!(System::digest().logs.len(), 2); // previous one + new
		assert_eq!(
			System::digest().logs[1],
			DigestItem::Consensus(
				ETHY_ENGINE_ID,
				ConsensusLog::AuthoritiesChange(ValidatorSet {
					validators: next_keys.to_vec(),
					id: next_validator_set_id,
					proof_threshold: 2,
				})
				.encode(),
			),
		);
		assert!(!EthBridge::bridge_paused());
		// Next_notary_keys hasn't been cleared
		assert_eq!(NextNotaryKeys::<Test>::get(), next_keys);
		assert_eq!(NotaryKeys::<Test>::get(), next_keys);
		assert!(!AuthoritiesChangedThisEra::<Test>::get());
	});
}

#[test]
/// This test ensures that authorities are changed in the event that the 5 minute window was missed
/// This will quickly change the authorities right before the session ending.
/// This can happen in the case of a forced era, if it does happen, the bridge will be scheduled to
/// unpause after 5 minutes.
fn on_before_session_ending_handles_authorities() {
	ExtBuilder::default().next_session_final().build().execute_with(|| {
		let default_account = AccountId::default();
		let next_keys = vec![
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
		];
		let next_keys_iter = vec![
			(&default_account, AuthorityId::from_slice(&[3_u8; 33]).unwrap()),
			(&default_account, AuthorityId::from_slice(&[4_u8; 33]).unwrap()),
		]
		.into_iter();

		// Call on_new_session but is_active_session_final is false
		<EthBridge as OneSessionHandler<AccountId>>::on_new_session(
			true,
			next_keys_iter.clone(),
			next_keys_iter.clone(),
		);
		// next notary keys queued up
		assert_eq!(
			NextNotaryKeys::<Test>::get(),
			next_keys_iter.clone().map(|(&_acc, pk)| pk).collect::<Vec<AuthorityId>>()
		);
		// Next authority change not scheduled, not final session
		assert!(NextAuthorityChange::<Test>::get().is_none());

		let block_number: BlockNumber = 100;
		System::set_block_number(block_number);
		// Call on_new_session where is_active_session_final is true, should change storage
		<EthBridge as OneSessionHandler<AccountId>>::on_new_session(
			true,
			next_keys_iter.clone(),
			next_keys_iter.clone(),
		);
		let epoch_duration: BlockNumber = EpochDuration::get().saturated_into();
		let expected_block: BlockNumber = block_number + epoch_duration - 75_u64;
		assert_eq!(NextAuthorityChange::<Test>::get(), Some(expected_block as u64));
		assert_eq!(NextNotaryKeys::<Test>::get(), next_keys.clone());

		let event_proof_id = NextEventProofId::<Test>::get();
		let next_validator_set_id = NotarySetId::<Test>::get() + 1;

		// Calling on_before_session_ending should call handle_authorities_change as it wasn't
		// changed in on_initialize
		<Pallet<Test> as OneSessionHandler<AccountId>>::on_before_session_ending();

		// Storage should represent the storage before the authorities are finalized
		assert_eq!(NotarySetProofId::<Test>::get(), event_proof_id);
		assert_eq!(NextEventProofId::<Test>::get(), event_proof_id + 1);
		assert!(NextAuthorityChange::<Test>::get().is_none());
		assert_eq!(NextNotaryKeys::<Test>::get(), next_keys);
		assert!(NotaryKeys::<Test>::get().is_empty());
		assert!(EthBridge::bridge_paused());

		// Item should be scheduled
		let scheduled_block: BlockNumber = block_number + 75_u64;
		Scheduler::on_initialize(scheduled_block);

		// scheduled do_finalise_authorities_change() will add notification log to the header
		assert_eq!(System::digest().logs.len(), 2_usize);
		assert_eq!(
			System::digest().logs[1],
			DigestItem::Consensus(
				ETHY_ENGINE_ID,
				ConsensusLog::AuthoritiesChange(ValidatorSet {
					validators: next_keys.to_vec(),
					id: next_validator_set_id,
					proof_threshold: 2,
				})
				.encode(),
			),
		);

		// This should update all the storage items
		assert!(!EthBridge::bridge_paused());
		assert_eq!(NotarySetProofId::<Test>::get(), event_proof_id);
		assert_eq!(NextEventProofId::<Test>::get(), event_proof_id + 1);
		assert!(NextAuthorityChange::<Test>::get().is_none());
		// Next_notary_keys hasn't been cleared
		assert_eq!(NextNotaryKeys::<Test>::get(), next_keys);
		assert_eq!(NotaryKeys::<Test>::get(), next_keys);
	});
}

#[test]
/// This test is similar to the one above except NextAuthorityChange is never set so simulates
/// a new era being forced before the final session
fn on_before_session_ending_handles_authorities_without_on_new_session() {
	ExtBuilder::default().next_session_final().build().execute_with(|| {
		let default_account = AccountId::default();
		let next_keys_iter = vec![
			(&default_account, AuthorityId::from_slice(&[3_u8; 33]).unwrap()),
			(&default_account, AuthorityId::from_slice(&[4_u8; 33]).unwrap()),
		]
		.into_iter();

		// Call on_new_session but is_active_session_final is false
		<EthBridge as OneSessionHandler<AccountId>>::on_new_session(
			true,
			next_keys_iter.clone(),
			next_keys_iter.clone(),
		);
		// next notary keys queued up
		assert_eq!(
			NextNotaryKeys::<Test>::get(),
			next_keys_iter.clone().map(|(&_acc, pk)| pk).collect::<Vec<AuthorityId>>()
		);
		// Next authority change not scheduled, not final session
		assert!(NextAuthorityChange::<Test>::get().is_none());

		// Block number as 2 triggers is_active_session_final = true
		let block_number: BlockNumber = 100;
		System::set_block_number(block_number);

		// Calling on_before_session_ending should call handle_authorities_change as it wasn't
		// changed in on_initialize
		<Pallet<Test> as OneSessionHandler<AccountId>>::on_before_session_ending();

		// Item should be scheduled and bridge still paused
		assert!(EthBridge::bridge_paused());
		let scheduled_block: BlockNumber = block_number + 75_u64;

		// Block before scheduled should not unpause bridge
		Scheduler::on_initialize(scheduled_block - 1_u64);
		assert!(EthBridge::bridge_paused());

		// Scheduler unpauses bridge
		Scheduler::on_initialize(scheduled_block);
		assert!(!EthBridge::bridge_paused());
	});
}

#[test]
fn scheduled_authorities_change_keeps_bridge_paused_if_manually_paused() {
	ExtBuilder::default().next_session_final().build().execute_with(|| {
		let default_account = AccountId::default();
		let next_keys_iter = vec![
			(&default_account, AuthorityId::from_slice(&[3_u8; 33]).unwrap()),
			(&default_account, AuthorityId::from_slice(&[4_u8; 33]).unwrap()),
		]
		.into_iter();

		// Manual pause of bridge
		assert_ok!(EthBridge::set_bridge_paused(frame_system::RawOrigin::Root.into(), true));
		assert!(EthBridge::bridge_paused());
		assert_eq!(
			BridgePaused::<Test>::get(),
			BridgePauseStatus { manual_pause: true, authorities_change: false }
		);

		// Call on_new_session but is_active_session_final is false
		<EthBridge as OneSessionHandler<AccountId>>::on_new_session(
			true,
			next_keys_iter.clone(),
			next_keys_iter.clone(),
		);
		// Next authority change not scheduled, not final session
		assert!(NextAuthorityChange::<Test>::get().is_none());

		// Block number as 2 triggers is_active_session_final = true
		let block_number: BlockNumber = 100;
		System::set_block_number(block_number);

		// Calling on_before_session_ending should call handle_authorities_change as it wasn't
		// changed in on_initialize
		<Pallet<Test> as OneSessionHandler<AccountId>>::on_before_session_ending();

		// Item should be scheduled and bridge still paused with both manual and authorities change
		assert!(EthBridge::bridge_paused());
		assert_eq!(
			BridgePaused::<Test>::get(),
			BridgePauseStatus { manual_pause: true, authorities_change: true }
		);
		let scheduled_block: BlockNumber = block_number + 75_u64;

		// Block before scheduled should not unpause bridge
		Scheduler::on_initialize(scheduled_block - 1_u64);
		assert!(EthBridge::bridge_paused());

		// Scheduler unpauses bridge
		Scheduler::on_initialize(scheduled_block);
		// Bridge still paused due to manual pause
		assert!(EthBridge::bridge_paused());
		assert_eq!(
			BridgePaused::<Test>::get(),
			BridgePauseStatus { manual_pause: true, authorities_change: false }
		);
	});
}

#[test]
/// This test tests the bug where the next_notary_keys were being set before the scheduled
/// finalise_authorities_change was called
fn force_new_era_with_scheduled_authority_change_works() {
	ExtBuilder::default().next_session_final().build().execute_with(|| {
		let default_account = AccountId::default();
		let next_keys_iter = vec![
			(&default_account, AuthorityId::from_slice(&[3_u8; 33]).unwrap()),
			(&default_account, AuthorityId::from_slice(&[4_u8; 33]).unwrap()),
		]
		.into_iter();

		// Call on_new_session but is_active_session_final is false
		<EthBridge as OneSessionHandler<AccountId>>::on_new_session(
			true,
			next_keys_iter.clone(),
			next_keys_iter.clone(),
		);
		// next notary keys queued up
		assert_eq!(
			NextNotaryKeys::<Test>::get(),
			next_keys_iter.clone().map(|(&_acc, pk)| pk).collect::<Vec<AuthorityId>>()
		);
		// Next authority change not scheduled, not final session
		assert!(NextAuthorityChange::<Test>::get().is_none());

		// Simulate force new era
		Forcing::<Test>::put(true);

		// Add a validator to the next_keys, simulating a change in validator during the last
		// session
		let next_keys_expanded = vec![
			(&default_account, AuthorityId::from_slice(&[3_u8; 33]).unwrap()),
			(&default_account, AuthorityId::from_slice(&[4_u8; 33]).unwrap()),
			(&default_account, AuthorityId::from_slice(&[5_u8; 33]).unwrap()),
		]
		.into_iter();

		// Block number 50 isn't final session, but we have forced
		let block_number: BlockNumber = 50;
		System::set_block_number(block_number);

		// Calling on_before_session_ending should call handle_authorities_change as it wasn't
		// changed in on_initialize
		<Pallet<Test> as OneSessionHandler<AccountId>>::on_before_session_ending();
		// Item should be scheduled and bridge still paused
		assert!(EthBridge::bridge_paused());

		// Call on_new_session while is_active_session_final is true due to force
		<EthBridge as OneSessionHandler<AccountId>>::on_new_session(
			true,
			next_keys_expanded.clone(),
			next_keys_expanded.clone(),
		);
		// next notary keys now the expanded keys
		assert_eq!(
			NextNotaryKeys::<Test>::get(),
			next_keys_expanded.clone().map(|(&_acc, pk)| pk).collect::<Vec<AuthorityId>>()
		);

		// Even though the next notary keys were updated in the new session,
		// When we schedule the finalise_authorities_change, we still have the previous keys
		let scheduled_block: BlockNumber = block_number + 75_u64;

		// Scheduler unpauses bridge
		Scheduler::on_initialize(scheduled_block);
		assert!(!EthBridge::bridge_paused());

		// Keys updated with the correct value
		assert_eq!(
			NotaryKeys::<Test>::get(),
			next_keys_iter.clone().map(|(&_acc, pk)| pk).collect::<Vec<AuthorityId>>()
		);
	});
}

#[test]
fn last_session_change() {
	ExtBuilder::default().active_session_final().build().execute_with(|| {
		let current_set_id = NotarySetId::<Test>::get();

		// setup storage
		let current_keys = vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
		];
		NotaryKeys::<Test>::put(WeakBoundedVec::try_from(current_keys.clone()).unwrap());
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
		NextNotaryKeys::<Test>::put(WeakBoundedVec::try_from(next_keys.clone()).unwrap());

		// current session is last in era: starting
		EthBridge::handle_authorities_change();
		assert!(EthBridge::bridge_paused());
		// current session is last in era: finishing
		<Pallet<Test> as OneSessionHandler<AccountId>>::on_before_session_ending();
		assert_eq!(NotaryKeys::<Test>::get(), next_keys);
		assert_eq!(NotarySetId::<Test>::get(), current_set_id + 1);
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
		let event_proof_id = NextEventProofId::<Test>::get();

		// Generate event proof
		assert_ok!(EthBridge::send_event(&source, &destination, message));
		// Ensure event has not been added to delayed queue
		assert_eq!(PendingEventProofs::<Test>::get(event_proof_id), None);
		assert_eq!(NextEventProofId::<Test>::get(), event_proof_id + 1);
		// On initialize does up to 2 reads to check for delayed proofs
		assert_eq!(
			EthBridge::on_initialize(frame_system::Pallet::<Test>::block_number() + 1),
			DbWeight::get().reads(3u64)
		);
	});
}

#[test]
fn xrpl_tx_signing_request() {
	ExtBuilder::default().build().execute_with(|| {
		let event_proof_id = NextEventProofId::<Test>::get();

		// Request tx signing
		assert_ok!(EthBridge::sign_xrpl_transaction("hello world".as_bytes()), event_proof_id);
		// Ensure request has not been added to queue
		assert_eq!(PendingEventProofs::<Test>::get(event_proof_id), None);
		assert_eq!(NextEventProofId::<Test>::get(), event_proof_id + 1);

		let signing_request = EthySigningRequest::XrplTx(BoundedVec::truncate_from(
			"hello world".as_bytes().to_vec(),
		));
		System::assert_has_event(
			Event::<Test>::EventSend { event_proof_id, signing_request: signing_request.clone() }
				.into(),
		);
		assert_eq!(
			System::digest().logs[0],
			DigestItem::Consensus(
				ETHY_ENGINE_ID,
				ConsensusLog::OpaqueSigningRequest::<AuthorityId> {
					chain_id: EthyChainId::Xrpl,
					event_proof_id,
					data: signing_request.data(),
				}
				.encode(),
			),
		);

		// Bridge is paused, request signing
		assert_ok!(EthBridge::set_bridge_paused(frame_system::RawOrigin::Root.into(), true));
		assert_ok!(EthBridge::sign_xrpl_transaction("hello world".as_bytes()), event_proof_id + 1);
		assert_eq!(
			PendingEventProofs::<Test>::get(event_proof_id + 1),
			Some(EthySigningRequest::XrplTx(BoundedVec::truncate_from(
				"hello world".as_bytes().to_vec()
			)))
		);

		System::assert_has_event(
			Event::<Test>::ProofDelayed { event_proof_id: event_proof_id + 1 }.into(),
		);
	});
}

#[test]
fn request_multiple_event_proofs() {
	ExtBuilder::default().build().execute_with(|| {
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		let message = &b"hello world"[..];

		assert_ok!(EthBridge::send_event(&source, &destination, message));
		assert_ok!(EthBridge::send_event(&source, &destination, message));
		let block_digest = <frame_system::Pallet<Test>>::digest();
		assert_eq!(block_digest.logs.len(), 2_usize);
	});
}

#[test]
fn delayed_event_proof() {
	ExtBuilder::default().build().execute_with(|| {
		let message = &b"hello world"[..];
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		assert_ok!(EthBridge::set_bridge_paused(frame_system::RawOrigin::Root.into(), true));
		assert_eq!(EthBridge::bridge_paused(), true);

		let event_proof_id = NextEventProofId::<Test>::get();
		let event_proof_info = EthySigningRequest::Ethereum(EthereumEventInfo {
			source,
			destination: destination,
			message: BoundedVec::truncate_from(message.to_vec()),
			validator_set_id: EthBridge::validator_set().id,
			event_proof_id,
		});

		// Generate event proof
		assert_ok!(EthBridge::send_event(&source, &destination, message));
		// Ensure event has been added to delayed claims
		assert_eq!(PendingEventProofs::<Test>::get(event_proof_id), Some(event_proof_info));
		assert_eq!(NextEventProofId::<Test>::get(), event_proof_id + 1);

		// Re-enable bridge
		assert_ok!(EthBridge::set_bridge_paused(frame_system::RawOrigin::Root.into(), false));
		// initialize pallet and initiate event proof
		let expected_weight: Weight =
			DbWeight::get().reads(3u64) + DbWeight::get().reads_writes(2, 2);
		assert_eq!(
			EthBridge::on_idle(
				frame_system::Pallet::<Test>::block_number() + 1,
				Weight::from_all(1_000_000_000_000)
			),
			expected_weight
		);
		// Ensure event has been removed from delayed claims
		assert!(PendingEventProofs::<Test>::get(event_proof_id).is_none());
	});
}

#[test]
fn delayed_event_proof_updates_validator_set_id_on_last_minute_authorities_change() {
	ExtBuilder::default().build().execute_with(|| {
		let default_account = AccountId::default();
		let message = &b"hello world"[..];
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		let next_keys_iter = vec![
			(&default_account, AuthorityId::from_slice(&[3_u8; 33]).unwrap()),
			(&default_account, AuthorityId::from_slice(&[4_u8; 33]).unwrap()),
		]
		.into_iter();

		let block_number: BlockNumber = 100;
		System::set_block_number(block_number);

		// Call on_new_session where is_active_session_final is true, should change storage
		<EthBridge as OneSessionHandler<AccountId>>::on_new_session(
			true,
			next_keys_iter.clone(),
			next_keys_iter.clone(),
		);
		let next_validator_set_id = NotarySetId::<Test>::get() + 1;

		// Calling on_before_session_ending should call handle_authorities_change as it wasn't
		// changed in on_initialize
		<Pallet<Test> as OneSessionHandler<AccountId>>::on_before_session_ending();

		// Bridge paused due to authorities change
		assert!(EthBridge::bridge_paused());
		assert_eq!(
			BridgePaused::<Test>::get(),
			BridgePauseStatus { manual_pause: false, authorities_change: true }
		);

		let event_proof_id = NextEventProofId::<Test>::get();
		let event_proof_info = EthySigningRequest::Ethereum(EthereumEventInfo {
			source,
			destination: destination,
			message: BoundedVec::truncate_from(message.to_vec()),
			validator_set_id: EthBridge::validator_set().id,
			event_proof_id,
		});

		// Generate event proof
		assert_ok!(EthBridge::send_event(&source, &destination, message));
		// Ensure event has been added to delayed claims
		assert_eq!(PendingEventProofs::<Test>::get(event_proof_id), Some(event_proof_info));
		assert_eq!(NextEventProofId::<Test>::get(), event_proof_id + 1);

		// Finalise authorities change by calling scheduled block
		let block_number = frame_system::Pallet::<Test>::block_number();
		let scheduled_block: BlockNumber = block_number + 75_u64;
		Scheduler::on_initialize(scheduled_block);

		// validator_set_id now updated
		assert_eq!(NotarySetId::<Test>::get(), next_validator_set_id);
		assert!(!EthBridge::bridge_paused());

		// Call on_idle and initiate event proof
		EthBridge::on_idle(
			frame_system::Pallet::<Test>::block_number() + 1,
			Weight::from_all(1_000_000_000_000),
		);
		// Ensure event has been removed from delayed claims
		assert!(PendingEventProofs::<Test>::get(event_proof_id).is_none());

		// check event is thrown with the correct validator_set_id
		let event_proof_info = EthySigningRequest::Ethereum(EthereumEventInfo {
			source,
			destination: destination,
			message: BoundedVec::truncate_from(message.to_vec()),
			validator_set_id: next_validator_set_id, // Updated validator set id
			event_proof_id,
		});
		System::assert_has_event(
			Event::<Test>::EventSend { event_proof_id, signing_request: event_proof_info.clone() }
				.into(),
		);
	});
}

#[test]
fn delayed_event_proof_updates_validator_set_id_on_normal_authorities_change() {
	ExtBuilder::default().build().execute_with(|| {
		let default_account = AccountId::default();
		let message = &b"hello world"[..];
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		let next_keys = vec![
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
		];
		let next_keys_iter = vec![
			(&default_account, AuthorityId::from_slice(&[3_u8; 33]).unwrap()),
			(&default_account, AuthorityId::from_slice(&[4_u8; 33]).unwrap()),
		]
		.into_iter();
		let next_validator_set_id = NotarySetId::<Test>::get() + 1;

		let block_number: BlockNumber = 100;
		System::set_block_number(block_number);
		// Call on_new_session where is_active_session_final is true, should change storage
		<EthBridge as OneSessionHandler<AccountId>>::on_new_session(
			true,
			next_keys_iter.clone(),
			next_keys_iter.clone(),
		);
		assert_eq!(NextNotaryKeys::<Test>::get(), next_keys.clone());

		// Now call on_initialise with the expected block to check it gets processed correctly
		let expected_block: BlockNumber = NextAuthorityChange::<Test>::get().unwrap();
		EthBridge::on_initialize(expected_block);

		// Bridge paused due to authorities change
		assert!(EthBridge::bridge_paused());
		assert_eq!(
			BridgePaused::<Test>::get(),
			BridgePauseStatus { manual_pause: false, authorities_change: true }
		);

		let event_proof_id = NextEventProofId::<Test>::get();
		let event_proof_info = EthySigningRequest::Ethereum(EthereumEventInfo {
			source,
			destination: destination,
			message: BoundedVec::truncate_from(message.to_vec()),
			validator_set_id: EthBridge::validator_set().id,
			event_proof_id,
		});

		// Generate event proof
		assert_ok!(EthBridge::send_event(&source, &destination, message));
		// Ensure event has been added to delayed claims
		assert_eq!(PendingEventProofs::<Test>::get(event_proof_id), Some(event_proof_info));
		assert_eq!(NextEventProofId::<Test>::get(), event_proof_id + 1);

		// Calling on_before_session_ending should now finalise authorities change
		<Pallet<Test> as OneSessionHandler<AccountId>>::on_before_session_ending();

		// validator_set_id now updated
		assert!(!EthBridge::bridge_paused());
		assert_eq!(NotarySetId::<Test>::get(), next_validator_set_id);

		// call on_idle and initiate event proof
		EthBridge::on_idle(
			frame_system::Pallet::<Test>::block_number() + 1,
			Weight::from_all(1_000_000_000_000),
		);
		// Ensure event has been removed from delayed claims
		assert!(PendingEventProofs::<Test>::get(event_proof_id).is_none());

		// check event is thrown with the correct validator_set_id
		let event_proof_info = EthySigningRequest::Ethereum(EthereumEventInfo {
			source,
			destination: destination,
			message: BoundedVec::truncate_from(message.to_vec()),
			validator_set_id: next_validator_set_id, // Updated validator set id
			event_proof_id,
		});
		System::assert_has_event(
			Event::<Test>::EventSend { event_proof_id, signing_request: event_proof_info.clone() }
				.into(),
		);
	});
}

#[test]
fn multiple_delayed_event_proof() {
	ExtBuilder::default().build().execute_with(|| {
		let message = &b"hello world"[..];
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		assert_ok!(EthBridge::set_bridge_paused(frame_system::RawOrigin::Root.into(), true));
		assert_eq!(EthBridge::bridge_paused(), true);

		let max_delayed_events = DelayedEventProofsPerBlock::<Test>::get();
		let event_count: u8 = max_delayed_events * 2;
		let mut event_ids: Vec<EventProofId> = vec![];
		let mut events_for_proving = vec![];
		for _ in 0..event_count {
			let event_proof_id = NextEventProofId::<Test>::get();
			event_ids.push(event_proof_id);
			let event_proof_info = EthySigningRequest::Ethereum(EthereumEventInfo {
				source,
				destination: destination,
				message: BoundedVec::truncate_from(message.to_vec()),
				validator_set_id: EthBridge::validator_set().id,
				event_proof_id,
			});
			events_for_proving.push(event_proof_info.clone());
			// Generate event proof
			assert_ok!(EthBridge::send_event(&source, &destination, message));
			// Ensure event has been added to delayed claims
			assert_eq!(PendingEventProofs::<Test>::get(event_proof_id), Some(event_proof_info));
			assert_eq!(NextEventProofId::<Test>::get(), event_proof_id + 1);
		}

		// Re-enable bridge
		assert_ok!(EthBridge::set_bridge_paused(frame_system::RawOrigin::Root.into(), false));
		// Call on_idle to process half of the pending event proofs
		let expected_weight: Weight = DbWeight::get().reads(3u64)
			+ DbWeight::get().reads_writes(2, 2) * max_delayed_events as u64;
		assert_eq!(
			EthBridge::on_idle(
				frame_system::Pallet::<Test>::block_number() + 1,
				Weight::from_all(1_000_000_000_000)
			),
			expected_weight
		);

		let mut removed_count = 0;
		for i in 0..event_count {
			// Ensure event has been removed from delayed claims
			if PendingEventProofs::<Test>::get(event_ids[i as usize]).is_none() {
				removed_count += 1;
			} else {
				assert_eq!(
					PendingEventProofs::<Test>::get(event_ids[i as usize]),
					Some(events_for_proving[i as usize].clone())
				)
			}
		}
		// Should have only processed max amount
		assert_eq!(removed_count, max_delayed_events);

		// Calling on_idle should process the rest
		assert_eq!(
			EthBridge::on_idle(
				frame_system::Pallet::<Test>::block_number() + 1,
				Weight::from_all(1_000_000_000_000)
			),
			expected_weight
		);

		// All events should have now been processed
		for i in 0..event_count {
			assert!(PendingEventProofs::<Test>::get(event_ids[i as usize]).is_none());
		}
	});
}

#[test]
fn on_idle_limits_processing() {
	ExtBuilder::default().build().execute_with(|| {
		let message = &b"hello world"[..];
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		assert_ok!(EthBridge::set_bridge_paused(frame_system::RawOrigin::Root.into(), true));

		let max_delayed_events = DelayedEventProofsPerBlock::<Test>::get();
		let event_count: u8 = max_delayed_events;
		let mut event_ids: Vec<EventProofId> = vec![];
		let mut events_for_proving = vec![];
		for _ in 0..event_count {
			let event_proof_id = NextEventProofId::<Test>::get();
			event_ids.push(event_proof_id);
			let event_proof_info = EthySigningRequest::Ethereum(EthereumEventInfo {
				source,
				destination: destination,
				message: BoundedVec::truncate_from(message.to_vec()),
				validator_set_id: EthBridge::validator_set().id,
				event_proof_id,
			});
			events_for_proving.push(event_proof_info.clone());
			// Generate event proof
			assert_ok!(EthBridge::send_event(&source, &destination, message));
			// Ensure event has been added to delayed claims
			assert_eq!(PendingEventProofs::<Test>::get(event_proof_id), Some(event_proof_info));
			assert_eq!(NextEventProofId::<Test>::get(), event_proof_id + 1);
		}

		// Re-enable bridge
		assert_ok!(EthBridge::set_bridge_paused(frame_system::RawOrigin::Root.into(), false));

		// Call on_idle with only enough weight to process 2 claims
		let claims_to_process = 2;
		let expected_weight: Weight = DbWeight::get().reads(3u64)
			+ DbWeight::get().reads_writes(2, 2) * claims_to_process as u64;
		assert_eq!(
			EthBridge::on_idle(
				frame_system::Pallet::<Test>::block_number() + 1,
				expected_weight + Weight::from_parts(1, 1)
			),
			expected_weight
		);

		let mut removed_count = 0;
		for i in 0..event_count {
			// Ensure event has been removed from delayed claims
			if PendingEventProofs::<Test>::get(event_ids[i as usize]).is_none() {
				removed_count += 1;
			} else {
				assert_eq!(
					PendingEventProofs::<Test>::get(event_ids[i as usize]),
					Some(events_for_proving[i as usize].clone())
				)
			}
		}
		// Should have only processed max amount
		assert_eq!(removed_count, claims_to_process);

		// Call on_idle with enough weight to process the rest
		let claims_to_process = event_count - claims_to_process;
		let expected_weight: Weight = DbWeight::get().reads(3u64)
			+ DbWeight::get().reads_writes(2, 2) * claims_to_process as u64;
		assert_eq!(
			EthBridge::on_idle(
				frame_system::Pallet::<Test>::block_number() + 1,
				Weight::from_all(1_000_000_000_000)
			),
			expected_weight
		);

		// All events should have now been processed
		for i in 0..event_count {
			assert!(PendingEventProofs::<Test>::get(event_ids[i as usize]).is_none());
		}
	});
}

#[test]
fn on_idle_no_remaining_weight_is_noop() {
	ExtBuilder::default().build().execute_with(|| {
		let message = &b"hello world"[..];
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		assert_ok!(EthBridge::set_bridge_paused(frame_system::RawOrigin::Root.into(), true));

		let max_delayed_events = DelayedEventProofsPerBlock::<Test>::get();
		let event_count: u8 = max_delayed_events * 2;
		let mut event_ids: Vec<EventProofId> = vec![];
		let mut events_for_proving = vec![];
		for _ in 0..event_count {
			let event_proof_id = NextEventProofId::<Test>::get();
			event_ids.push(event_proof_id);
			let event_proof_info = EthySigningRequest::Ethereum(EthereumEventInfo {
				source,
				destination: destination,
				message: BoundedVec::truncate_from(message.to_vec()),
				validator_set_id: EthBridge::validator_set().id,
				event_proof_id,
			});
			events_for_proving.push(event_proof_info.clone());
			// Generate event proof
			assert_ok!(EthBridge::send_event(&source, &destination, message));
			// Ensure event has been added to delayed claims
			assert_eq!(PendingEventProofs::<Test>::get(event_proof_id), Some(event_proof_info));
			assert_eq!(NextEventProofId::<Test>::get(), event_proof_id + 1);
		}

		// Re-enable bridge
		assert_ok!(EthBridge::set_bridge_paused(frame_system::RawOrigin::Root.into(), false));
		// Calling on_idle with not enough weight to process one claim should do nothing
		let minimum_weight: Weight =
			DbWeight::get().reads(3u64) + DbWeight::get().reads_writes(2, 2);
		assert_eq!(
			EthBridge::on_idle(frame_system::Pallet::<Test>::block_number() + 1, minimum_weight),
			Weight::from_all(0)
		);

		// All PendingEventProofs should still be present
		for i in 0..event_count {
			assert!(PendingEventProofs::<Test>::get(event_ids[i as usize]).is_some());
		}
	});
}

#[test]
fn set_delayed_event_proofs_per_block() {
	ExtBuilder::default().build().execute_with(|| {
		// Check that it starts as default value
		assert_eq!(DelayedEventProofsPerBlock::<Test>::get(), 5);
		let new_max_delayed_events: u8 = 10;
		assert_ok!(EthBridge::set_delayed_event_proofs_per_block(
			frame_system::RawOrigin::Root.into(),
			new_max_delayed_events
		));
		assert_eq!(DelayedEventProofsPerBlock::<Test>::get(), new_max_delayed_events);

		let message = &b"hello world"[..];
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		let mut event_ids: Vec<EventProofId> = vec![];
		assert_ok!(EthBridge::set_bridge_paused(frame_system::RawOrigin::Root.into(), true));

		for _ in 0..new_max_delayed_events {
			let event_proof_id = NextEventProofId::<Test>::get();
			event_ids.push(event_proof_id);
			let event_proof_info = EthySigningRequest::Ethereum(EthereumEventInfo {
				source,
				destination: destination,
				message: BoundedVec::truncate_from(message.to_vec()),
				validator_set_id: EthBridge::validator_set().id,
				event_proof_id,
			});
			// Generate event proof
			assert_ok!(EthBridge::send_event(&source, &destination, message));
			// Ensure event has been added to delayed claims
			assert_eq!(PendingEventProofs::<Test>::get(event_proof_id), Some(event_proof_info));
			assert_eq!(NextEventProofId::<Test>::get(), event_proof_id + 1);
		}

		// Re-enable bridge
		assert_ok!(EthBridge::set_bridge_paused(frame_system::RawOrigin::Root.into(), false));
		// call on_idle for pallet and initiate event proof
		let expected_weight: Weight = DbWeight::get().reads(3u64)
			+ DbWeight::get().reads_writes(2, 2) * new_max_delayed_events as u64;
		assert_eq!(
			EthBridge::on_idle(
				frame_system::Pallet::<Test>::block_number() + 1,
				Weight::from_all(1_000_000_000_000)
			),
			expected_weight
		);

		for i in 0..new_max_delayed_events {
			// Ensure event has been removed from delayed claims
			assert!(PendingEventProofs::<Test>::get(event_ids[i as usize]).is_none());
		}
	});
}

#[test]
fn set_delayed_event_proofs_per_block_not_root_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		// Check that it starts as default value
		assert_eq!(DelayedEventProofsPerBlock::<Test>::get(), 5);
		let new_value: u8 = 10;
		assert_noop!(
			EthBridge::set_delayed_event_proofs_per_block(
				frame_system::RawOrigin::None.into(),
				new_value
			),
			DispatchError::BadOrigin
		);
		assert_eq!(DelayedEventProofsPerBlock::<Test>::get(), 5);
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
			.address(ContractAddress::<Test>::get())
			.data(event_data.as_slice())
			.topics(vec![SUBMIT_BRIDGE_EVENT_SELECTOR.into()])
			.transaction_hash(tx_hash)
			.build();
		let _mock_tx_receipt =
			create_transaction_receipt_mock(block_number, tx_hash, source, vec![mock_log]);

		let event_claim =
			EventClaim { tx_hash, source, destination, data: BoundedVec::truncate_from(message) };
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
			destination: EthAddress::default(),
			data: BoundedVec::truncate_from(vec![]),
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

		let event_claim = EventClaim {
			tx_hash,
			source,
			destination: EthAddress::default(),
			data: BoundedVec::truncate_from(vec![]),
		};
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
		let event_claim = EventClaim {
			tx_hash,
			source: H160::from_low_u64_be(444),
			destination: EthAddress::default(),
			data: BoundedVec::truncate_from(vec![]),
		};
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
			.address(ContractAddress::<Test>::get())
			.topics(vec![SUBMIT_BRIDGE_EVENT_SELECTOR.into()])
			.data(event_data.as_slice())
			.transaction_hash(tx_hash)
			.build();
		let _mock_tx_receipt =
			create_transaction_receipt_mock(block_number, tx_hash, source, vec![mock_log]);

		let event_claim =
			EventClaim { tx_hash, source, destination, data: BoundedVec::truncate_from(vec![]) };

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
			.address(ContractAddress::<Test>::get())
			.topics(vec![SUBMIT_BRIDGE_EVENT_SELECTOR.into()])
			.data(event_data.as_slice())
			.transaction_hash(tx_hash)
			.build();
		let _mock_tx_receipt =
			create_transaction_receipt_mock(block_number, tx_hash, source, vec![mock_log]);

		let event_claim =
			EventClaim { tx_hash, source, destination, data: BoundedVec::truncate_from(vec![]) };

		assert_eq!(
			EthBridge::offchain_try_notarize_event(event_id, event_claim),
			EventClaimResult::NotEnoughConfirmations
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
			.address(ContractAddress::<Test>::get())
			.data(event_data.as_slice())
			.transaction_hash(tx_hash)
			.build();
		let _mock_tx_receipt =
			create_transaction_receipt_mock(block_number + 1, tx_hash, source, vec![mock_log]);
		let event_claim =
			EventClaim { tx_hash, source, destination, data: BoundedVec::truncate_from(vec![]) };

		// Set event confirmations to 0 so it doesn't fail early
		let _ = EthBridge::set_event_block_confirmations(frame_system::RawOrigin::Root.into(), 0);
		assert_eq!(
			EthBridge::offchain_try_notarize_event(event_id, event_claim),
			EventClaimResult::NoTxLogs
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
	let mock_notary_keys: Vec<<Test as Config>::EthyId> = (1_u8..=9_u8)
		.map(|k| <Test as Config>::EthyId::from_slice(&[k; 33]).unwrap())
		.collect();
	ExtBuilder::default().build().execute_with(|| {
		let call_id = 1_u64;
		let eth_call_request = CheckedEthCallRequest {
			input: BoundedVec::truncate_from(vec![]),
			target: EthAddress::default(),
			timestamp: 0,
			max_block_look_behind: 0,
			try_block_number: 0,
			check_timestamp: 0,
		};
		EthCallRequestInfo::<Test>::insert(call_id, eth_call_request);
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
				EthCallNotarizationsAggregated::<Test>::get(call_id).unwrap_or_default();
			println!("{:?}", aggregated_notarizations);
			assert_eq!(aggregated_notarizations.get(&notary_result).copied(), aggregation);
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
	let mock_notary_keys: Vec<<Test as Config>::EthyId> = (1_u8..=6_u8)
		.map(|k| <Test as Config>::EthyId::from_slice(&[k; 33]).unwrap())
		.collect();
	ExtBuilder::default().build().execute_with(|| {
		let call_id = 1_u64;
		let eth_call_request = CheckedEthCallRequest {
			input: BoundedVec::truncate_from(vec![]),
			target: EthAddress::default(),
			timestamp: 0,
			max_block_look_behind: 0,
			try_block_number: 0,
			check_timestamp: 0,
		};
		EthCallRequestInfo::<Test>::insert(call_id, eth_call_request);
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
					Error::<Test>::InvalidClaim
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
				EthCallNotarizationsAggregated::<Test>::get(call_id).unwrap_or_default();
			println!("{:?}", aggregated_notarizations);
			assert_eq!(aggregated_notarizations.get(&notary_result).copied(), aggregation);
		}

		// failure callback triggered with correct value
		assert_eq!(
			MockEthCallSubscriber::failed_result(),
			Some((call_id, EthCallFailure::Internal)),
		);
	});
}

#[test]
fn test_prune_claim_ids() {
	{
		let mut test_vec = vec![1, 2, 3, 4, 6, 7];
		Pallet::<Test>::prune_claim_ids(&mut test_vec);
		assert_eq!(test_vec, vec![4, 6, 7]);
	}
	{
		let mut test_vec = vec![4, 5, 6, 7];
		Pallet::<Test>::prune_claim_ids(&mut test_vec);
		assert_eq!(test_vec, vec![7]);
	}
	{
		let mut test_vec: Vec<EventClaimId> = vec![];
		Pallet::<Test>::prune_claim_ids(&mut test_vec);
		assert_eq!(test_vec, vec![] as Vec<EventClaimId>);
	}
	{
		let mut test_vec = vec![5];
		Pallet::<Test>::prune_claim_ids(&mut test_vec);
		assert_eq!(test_vec, vec![5]);
	}
	{
		let mut test_vec = vec![0, 0, 0]; // event_id will be unique. Hence not applicable
		Pallet::<Test>::prune_claim_ids(&mut test_vec);
		assert_eq!(test_vec, vec![0, 0, 0]);
	}
	{
		let mut test_vec = vec![0, 1, 1, 2, 5]; // event_id will be unique. Hence not applicable
		Pallet::<Test>::prune_claim_ids(&mut test_vec);
		assert_eq!(test_vec, vec![1, 1, 2, 5]);
	}
}

#[test]
fn prune_claim_ids_keeps_missed() {
	ExtBuilder::default().build().execute_with(|| {
		let mut test_vec = vec![1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
		Pallet::<Test>::prune_claim_ids(&mut test_vec);
		assert_eq!(test_vec, vec![12]);
		assert_eq!(MissedMessageIds::<Test>::get(), vec![2]);
	});

	ExtBuilder::default().build().execute_with(|| {
		let mut test_vec = vec![2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26];
		Pallet::<Test>::prune_claim_ids(&mut test_vec);
		assert_eq!(test_vec, vec![8, 10, 12, 14, 16, 18, 20, 22, 24, 26]);
		assert_eq!(MissedMessageIds::<Test>::get(), vec![3, 5, 7]);
	});

	ExtBuilder::default().build().execute_with(|| {
		let mut test_vec = vec![1, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14];
		Pallet::<Test>::prune_claim_ids(&mut test_vec);
		assert_eq!(test_vec, vec![6, 8, 9, 10, 11, 12, 13, 14]);
		assert_eq!(MissedMessageIds::<Test>::get(), vec![2, 3]);
	});
}

#[test]
fn on_initialize_doesnt_prune_if_not_needed() {
	ExtBuilder::default().build().execute_with(|| {
		// These ids will be pruned if the pruning function gets called in on_initialize
		// However there is nothing to process so it doesn't prune
		// This prevents pruning happening when no changes are made
		let event_ids = vec![1, 2, 3, 4, 5];
		ProcessedMessageIds::<Test>::put(BoundedVec::truncate_from(event_ids.clone()));

		// Call on_initialize with nothing to prune
		let consumed_weight = EthBridge::on_initialize(25);
		assert_eq!(consumed_weight, DbWeight::get().reads(3u64));

		// ProcessedMessageIds unchanged
		assert_eq!(ProcessedMessageIds::<Test>::get().into_inner(), event_ids)
	});
}

#[test]
fn on_initialize_moves_missed_to_vec() {
	let relayer = H160::from_low_u64_be(123);
	ExtBuilder::default().relayer(relayer).build().execute_with(|| {
		// Setup data for even ids up to 26
		let event_ids = vec![1, 10, 12, 14, 16, 18, 20, 22, 24, 26];
		ProcessedMessageIds::<Test>::put(BoundedVec::truncate_from(event_ids.clone()));

		// submit event 28 and 30
		let event_data = encode_event_message(
			28_u64,
			H160::from_low_u64_be(555),
			H160::from_low_u64_be(555),
			&[2, 3, 4, 5],
		);
		let tx_hash = EthHash::from_low_u64_be(33);
		assert_ok!(EthBridge::submit_event(
			RuntimeOrigin::signed(relayer.into()),
			tx_hash,
			event_data.clone(),
		));
		let event_data = encode_event_message(
			30_u64,
			H160::from_low_u64_be(555),
			H160::from_low_u64_be(555),
			&[2, 3, 4, 5],
		);
		let tx_hash = EthHash::from_low_u64_be(33);
		assert_ok!(EthBridge::submit_event(
			RuntimeOrigin::signed(relayer.into()),
			tx_hash,
			event_data.clone(),
		));

		// Process the messages
		let process_at = System::block_number() + ChallengePeriod::<Test>::get();
		EthBridge::on_initialize(process_at);

		// ProcessedMessageIds updated with new id and pruned old ids
		assert_eq!(
			ProcessedMessageIds::<Test>::get().into_inner(),
			vec![12, 14, 16, 18, 20, 22, 24, 26, 28, 30]
		);

		// MissedMessageIds contains pruned ids
		assert_eq!(MissedMessageIds::<Test>::get(), vec![2, 3, 4, 5, 6, 7, 8, 9, 11]);
	});
}

#[test]
fn test_submit_event_replay_check() {
	let relayer = H160::from_low_u64_be(123);
	let tx_hash = EthHash::from_low_u64_be(33);
	let mut event_data: Vec<Vec<u8>> = Default::default();
	// prepare 4 events
	for i in 0..4 {
		let event_item_data = encode_event_message(
			i as u64,
			H160::from_low_u64_be(555),
			H160::from_low_u64_be(555),
			&[i as u8, 2, 3, 4, 5],
		);
		event_data.push(event_item_data);
	}

	ExtBuilder::default().relayer(relayer).build().execute_with(|| {
		// submit event 0, 1, 3 only
		for i in 0..4 {
			if i != 2 {
				assert_ok!(EthBridge::submit_event(
					RuntimeOrigin::signed(relayer.into()),
					tx_hash,
					event_data[i].clone(),
				));
			}
		}
		// Process the messages
		let process_at = System::block_number() + ChallengePeriod::<Test>::get();
		EthBridge::on_initialize(process_at);
		// check the processed_message_ids has [1, 3]
		assert_eq!(ProcessedMessageIds::<Test>::get().into_inner(), vec![1, 3]);
		// try to resubmit claim 0 again.
		assert_noop!(
			EthBridge::submit_event(
				RuntimeOrigin::signed(relayer.into()),
				tx_hash,
				event_data[0].clone()
			),
			Error::<Test>::EventReplayProcessed
		);

		// submit claim 2 now
		assert_ok!(EthBridge::submit_event(
			RuntimeOrigin::signed(relayer.into()),
			tx_hash,
			event_data[2].clone(),
		));
		// Process the messages
		let process_at2 = System::block_number() + ChallengePeriod::<Test>::get();
		EthBridge::on_initialize(process_at2);

		// check the processed_message_ids has [3]
		assert_eq!(ProcessedMessageIds::<Test>::get().into_inner(), vec![3]);
	});
}

#[test]
fn pause_bridge_works() {
	ExtBuilder::default().build().execute_with(|| {
		// Check initial state
		assert!(!EthBridge::bridge_paused());

		assert_ok!(EthBridge::set_bridge_paused(frame_system::RawOrigin::Root.into(), true));
		assert!(EthBridge::bridge_paused());

		// And unpause again
		assert_ok!(EthBridge::set_bridge_paused(frame_system::RawOrigin::Root.into(), false));
		assert!(!EthBridge::bridge_paused());
	});
}

#[test]
fn set_bridge_paused_not_root_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let account = H160::from_low_u64_be(123);

		assert_noop!(
			EthBridge::set_bridge_paused(RuntimeOrigin::signed(account.into()), true),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn set_challenge_period_works() {
	ExtBuilder::default().build().execute_with(|| {
		let new_challenge_period: BlockNumber = 12345;

		assert_ok!(EthBridge::set_challenge_period(
			frame_system::RawOrigin::Root.into(),
			new_challenge_period
		));
		// Check storage updated
		assert_eq!(ChallengePeriod::<Test>::get(), new_challenge_period);
	});
}

#[test]
fn set_contract_address_works() {
	ExtBuilder::default().build().execute_with(|| {
		let new_bridge_address: EthAddress =
			EthAddress::from(hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));

		assert_ok!(EthBridge::set_contract_address(
			frame_system::RawOrigin::Root.into(),
			new_bridge_address
		));
		// Check storage updated
		assert_eq!(ContractAddress::<Test>::get(), new_bridge_address);
	});
}

#[test]
fn set_contract_address_not_root_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let new_bridge_address: EthAddress =
			EthAddress::from(hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));
		let ken = H160::from_low_u64_be(123);

		assert_noop!(
			EthBridge::set_contract_address(RuntimeOrigin::signed(ken.into()), new_bridge_address),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn set_door_signers_fails() {
	ExtBuilder::default().build().execute_with(|| {
		let caller = XrplAccountId::from_low_u64_be(1);
		assert_noop!(
			EthBridge::set_xrpl_door_signers(
				RuntimeOrigin::signed(AccountId::from(caller)),
				vec![(AuthorityId::from_slice(&[1_u8; 33]).unwrap(), true)]
			),
			BadOrigin
		);
	});
}

#[test]
fn set_door_signers() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(EthBridge::set_xrpl_door_signers(
			RuntimeOrigin::root(),
			vec![
				(AuthorityId::from_slice(&[1_u8; 33]).unwrap(), true),
				(AuthorityId::from_slice(&[2_u8; 33]).unwrap(), false)
			],
		));
	});
}

#[test]
fn genesis_xrp_door_signers_works() {
	ExtBuilder::default().xrp_door_signers([1_u8; 33]).build().execute_with(|| {
		assert_eq!(
			XrplDoorSigners::<Test>::get(AuthorityId::from_slice(&[1_u8; 33]).unwrap()),
			true
		);
		assert_eq!(
			XrplDoorSigners::<Test>::get(AuthorityId::from_slice(&[2_u8; 33]).unwrap()),
			false
		);
	});
}

#[test]
fn notary_xrpl_keys_unchanged_do_not_request_for_xrpl_proof() {
	ExtBuilder::default().active_session_final().build().execute_with(|| {
		let current_set_id = NotarySetId::<Test>::get();

		// setup storage
		let current_keys = vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
		];
		NotaryKeys::<Test>::put(WeakBoundedVec::try_from(current_keys.clone()).unwrap());
		NotaryXrplKeys::<Test>::put(WeakBoundedVec::try_from(current_keys.clone()).unwrap());
		for door_signer in current_keys.iter() {
			XrplDoorSigners::<Test>::insert(door_signer, true);
		}

		assert_eq!(
			EthBridge::validator_set(),
			ValidatorSet {
				validators: current_keys.clone(),
				id: current_set_id,
				proof_threshold: 2 // ceil(2 * 0.66)
			}
		);
		assert_eq!(NotaryXrplKeys::<Test>::get(), current_keys.clone());

		let next_keys = current_keys.clone();
		NextNotaryKeys::<Test>::put(WeakBoundedVec::try_from(next_keys.clone()).unwrap());

		assert_eq!(XrplNotarySetProofId::<Test>::get(), 0);
		let eth_proof_id = NextEventProofId::<Test>::get();
		// current session is last in era: starting
		EthBridge::handle_authorities_change();
		assert!(EthBridge::bridge_paused());
		assert_eq!(NotarySetProofId::<Test>::get(), eth_proof_id);
		assert_eq!(XrplNotarySetProofId::<Test>::get(), 0); // No change to XrplNotarySetProofId since no change to NotaryXrplKeys

		// current session is last in era: finishing
		<Pallet<Test> as OneSessionHandler<AccountId>>::on_before_session_ending();
		assert_eq!(NotaryKeys::<Test>::get(), next_keys);
		assert_eq!(NotarySetId::<Test>::get(), current_set_id + 1);
		assert_eq!(NotaryXrplKeys::<Test>::get(), next_keys);
		assert_eq!(XrplNotarySetProofId::<Test>::get(), 0);

		assert_eq!(
			EthBridge::validator_set(),
			ValidatorSet {
				validators: current_keys.clone(),
				id: current_set_id + 1,
				proof_threshold: 2 // ceil(2 * 0.66)
			}
		);
		assert_eq!(NotaryXrplKeys::<Test>::get(), current_keys.clone());
		assert_eq!(XrplNotarySetProofId::<Test>::get(), 0); // No change to XrplNotarySetProofId since no change to NotaryXrplKeys
		assert!(!EthBridge::bridge_paused());
	});
}

#[test]
fn notary_xrpl_keys_same_set_shuffled_do_not_request_for_xrpl_proof() {
	ExtBuilder::default().active_session_final().build().execute_with(|| {
		let current_set_id = NotarySetId::<Test>::get();

		// setup storage
		let current_keys = vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
		];
		NotaryKeys::<Test>::put(WeakBoundedVec::try_from(current_keys.clone()).unwrap());
		NotaryXrplKeys::<Test>::put(WeakBoundedVec::try_from(current_keys.clone()).unwrap());
		for door_signer in current_keys.iter() {
			XrplDoorSigners::<Test>::insert(door_signer, true);
		}

		assert_eq!(
			EthBridge::validator_set(),
			ValidatorSet {
				validators: current_keys.clone(),
				id: current_set_id,
				proof_threshold: 2 // ceil(2 * 0.66)
			}
		);
		assert_eq!(NotaryXrplKeys::<Test>::get(), current_keys.clone());

		let next_keys = vec![
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
		];
		NextNotaryKeys::<Test>::put(WeakBoundedVec::try_from(next_keys.clone()).unwrap());

		assert_eq!(XrplNotarySetProofId::<Test>::get(), 0);
		let eth_proof_id = NextEventProofId::<Test>::get();
		// current session is last in era: starting
		EthBridge::handle_authorities_change();
		assert!(EthBridge::bridge_paused());
		assert_eq!(NotarySetProofId::<Test>::get(), eth_proof_id);
		assert_eq!(XrplNotarySetProofId::<Test>::get(), 0); // No change to XrplNotarySetProofId since no change to NotaryXrplKeys

		// current session is last in era: finishing
		<Pallet<Test> as OneSessionHandler<AccountId>>::on_before_session_ending();
		assert_eq!(NotaryKeys::<Test>::get(), next_keys);
		assert_eq!(NotarySetId::<Test>::get(), current_set_id + 1);
		assert_eq!(NotaryXrplKeys::<Test>::get(), next_keys);
		assert_eq!(XrplNotarySetProofId::<Test>::get(), 0);

		assert_eq!(
			EthBridge::validator_set(),
			ValidatorSet {
				validators: next_keys.clone(),
				id: current_set_id + 1,
				proof_threshold: 2 // ceil(2 * 0.66)
			}
		);
		assert_eq!(NotaryXrplKeys::<Test>::get(), next_keys.clone());
		assert_eq!(XrplNotarySetProofId::<Test>::get(), 0); // No change to XrplNotarySetProofId since no change to NotaryXrplKeys
		assert!(!EthBridge::bridge_paused());
	});
}

#[test]
fn notary_xrpl_keys_changed_request_for_xrpl_proof() {
	ExtBuilder::default().active_session_final().build().execute_with(|| {
		let current_set_id = NotarySetId::<Test>::get();

		// setup storage
		let current_keys = vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
		];
		NotaryKeys::<Test>::put(WeakBoundedVec::try_from(current_keys.clone()).unwrap());
		NotaryXrplKeys::<Test>::put(WeakBoundedVec::try_from(current_keys.clone()).unwrap());
		for door_signer in current_keys.iter() {
			XrplDoorSigners::<Test>::insert(door_signer, true);
		}

		assert_eq!(
			EthBridge::validator_set(),
			ValidatorSet {
				validators: current_keys.clone(),
				id: current_set_id,
				proof_threshold: 2 // ceil(2 * 0.66)
			}
		);
		assert_eq!(NotaryXrplKeys::<Test>::get(), current_keys.clone());

		let next_keys = vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
		];
		NextNotaryKeys::<Test>::put(WeakBoundedVec::try_from(next_keys.clone()).unwrap());
		for door_signer in next_keys.iter() {
			XrplDoorSigners::<Test>::insert(door_signer, true);
		}

		assert_eq!(XrplNotarySetProofId::<Test>::get(), 0);
		let eth_proof_id = NextEventProofId::<Test>::get();
		// current session is last in era: starting
		EthBridge::handle_authorities_change();
		assert!(EthBridge::bridge_paused());
		assert_eq!(NotarySetProofId::<Test>::get(), eth_proof_id);
		// Requested for xrpl proof since NotaryXrplKeys changed, XrplNotarySetProofId is
		// (eth_proof_id + 1)
		assert_eq!(XrplNotarySetProofId::<Test>::get(), eth_proof_id + 1);

		// current session is last in era: finishing
		<Pallet<Test> as OneSessionHandler<AccountId>>::on_before_session_ending();
		assert_eq!(NotaryKeys::<Test>::get(), next_keys);
		assert_eq!(NotarySetId::<Test>::get(), current_set_id + 1);
		assert_eq!(NotaryXrplKeys::<Test>::get(), next_keys);
		assert_eq!(XrplNotarySetProofId::<Test>::get(), eth_proof_id + 1);

		assert_eq!(
			EthBridge::validator_set(),
			ValidatorSet {
				validators: next_keys.clone(),
				id: current_set_id + 1,
				proof_threshold: 2 // ceil(2 * 0.66)
			}
		);
		assert_eq!(NotaryXrplKeys::<Test>::get(), next_keys.clone());
		assert_eq!(XrplNotarySetProofId::<Test>::get(), eth_proof_id + 1);
		assert!(!EthBridge::bridge_paused());
	});
}

#[test]
fn notary_xrpl_keys_removed_request_for_xrpl_proof() {
	ExtBuilder::default().active_session_final().build().execute_with(|| {
		let current_set_id = NotarySetId::<Test>::get();

		// setup storage
		let current_keys = vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
		];
		NotaryKeys::<Test>::put(WeakBoundedVec::try_from(current_keys.clone()).unwrap());
		NotaryXrplKeys::<Test>::put(WeakBoundedVec::try_from(current_keys.clone()).unwrap());
		for door_signer in current_keys.iter() {
			XrplDoorSigners::<Test>::insert(door_signer, true);
		}

		assert_eq!(
			EthBridge::validator_set(),
			ValidatorSet {
				validators: current_keys.clone(),
				id: current_set_id,
				proof_threshold: 2 // ceil(2 * 0.66)
			}
		);
		assert_eq!(NotaryXrplKeys::<Test>::get(), current_keys.clone());

		let next_keys = vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
		];
		NextNotaryKeys::<Test>::put(WeakBoundedVec::try_from(next_keys.clone()).unwrap());

		assert_ok!(EthBridge::set_xrpl_door_signers(
			RuntimeOrigin::root(),
			vec![
				(AuthorityId::from_slice(&[1_u8; 33]).unwrap(), true),
				(AuthorityId::from_slice(&[2_u8; 33]).unwrap(), false),
				(AuthorityId::from_slice(&[3_u8; 33]).unwrap(), true)
			],
		));
		assert_eq!(
			NotaryXrplKeys::<Test>::get(),
			vec![AuthorityId::from_slice(&[1_u8; 33]).unwrap()]
		);

		assert_eq!(XrplNotarySetProofId::<Test>::get(), 0);
		let eth_proof_id = NextEventProofId::<Test>::get();
		// current session is last in era: starting
		EthBridge::handle_authorities_change();
		assert!(EthBridge::bridge_paused());
		assert_eq!(NotarySetProofId::<Test>::get(), eth_proof_id);
		// Requested for xrpl proof since NotaryXrplKeys changed, XrplNotarySetProofId is
		// (eth_proof_id + 1)
		assert_eq!(XrplNotarySetProofId::<Test>::get(), eth_proof_id + 1);

		// current session is last in era: finishing
		<Pallet<Test> as OneSessionHandler<AccountId>>::on_before_session_ending();
		assert_eq!(NotaryKeys::<Test>::get(), next_keys);
		assert_eq!(NotarySetId::<Test>::get(), current_set_id + 1);
		let keys_filtered = vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
		];
		assert_eq!(NotaryXrplKeys::<Test>::get(), keys_filtered);
		assert_eq!(XrplNotarySetProofId::<Test>::get(), eth_proof_id + 1);

		assert_eq!(
			EthBridge::validator_set(),
			ValidatorSet {
				validators: next_keys.clone(),
				id: current_set_id + 1,
				proof_threshold: 2 // ceil(2 * 0.66)
			}
		);
		assert_eq!(NotaryXrplKeys::<Test>::get(), keys_filtered.clone());
		assert_eq!(XrplNotarySetProofId::<Test>::get(), eth_proof_id + 1);
		assert!(!EthBridge::bridge_paused());
	});
}
