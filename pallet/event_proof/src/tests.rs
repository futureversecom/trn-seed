use super::*;
use crate::Event;
use frame_support::{assert_noop, assert_ok};
use mock::*;
use seed_pallet_common::{eth_types::EthereumEventInfo, EventProofAdapter};
use seed_primitives::{
	validator::{crypto::AuthorityId, ChainId, ConsensusLog},
	AccountId, Balance, DigestItem,
};
use sp_core::H160;
use sp_runtime::traits::BadOrigin;

#[test]
fn xrpl_tx_signing_request() {
	new_test_ext().execute_with(|| {
		let event_proof_id = EventProof::next_event_proof_id();

		// Request tx signing
		assert_ok!(EventProof::sign_xrpl_transaction("hello world".as_bytes()), event_proof_id);
		// Ensure request has not been added to queue
		assert_eq!(EventProof::pending_event_proofs(event_proof_id), None);
		assert_eq!(EventProof::next_event_proof_id(), event_proof_id + 1);

		let signing_request = SigningRequest::XrplTx("hello world".as_bytes().to_vec());
		System::assert_has_event(
			Event::<Test>::EventSend { event_proof_id, signing_request: signing_request.clone() }
				.into(),
		);
		assert_eq!(
			System::digest().logs[0],
			DigestItem::Consensus(
				ENGINE_ID,
				ConsensusLog::OpaqueSigningRequest::<AuthorityId> {
					chain_id: ChainId::Xrpl,
					event_proof_id,
					data: signing_request.data(),
				}
				.encode(),
			),
		);

		// Bridge is paused, request signing
		<BridgePaused<Test>>::put(true);
		MockValidatorAdapter::bridge_paused(true);
		assert_ok!(EventProof::sign_xrpl_transaction("hello world".as_bytes()), event_proof_id + 1);
		assert_eq!(
			EventProof::pending_event_proofs(event_proof_id + 1),
			Some(SigningRequest::XrplTx("hello world".as_bytes().to_vec()))
		);

		System::assert_has_event(Event::<Test>::ProofDelayed(event_proof_id + 1).into());
	});
}

#[ignore]
#[test]
fn delayed_event_proof() {
	new_test_ext().execute_with(|| {
		let message = &b"hello world"[..];
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		<BridgePaused<Test>>::put(true);
		MockValidatorAdapter::bridge_paused(true);
		assert_eq!(EventProof::bridge_paused(), true);

		let event_proof_id = EventProof::next_event_proof_id();
		let event_proof_info = SigningRequest::Ethereum(EthereumEventInfo {
			source,
			destination: destination.clone(),
			message: message.to_vec(),
			validator_set_id: MockValidatorAdapter::validator_set_id(),
			event_proof_id,
		});

		// Generate event proof
		assert_ok!(EventProof::sign_eth_transaction(
			&source,
			&destination,
			&message,
			MockValidatorAdapter::validator_set_id()
		));
		// Ensure event has been added to delayed claims
		assert_eq!(EventProof::pending_event_proofs(event_proof_id), Some(event_proof_info));
		assert_eq!(EventProof::next_event_proof_id(), event_proof_id + 1);

		// Re-enable bridge
		<BridgePaused<Test>>::kill();
		MockValidatorAdapter::bridge_kill();
		// initialize pallet and initiate event proof
		let max_delayed_events = EventProof::delayed_event_proofs_per_block() as u64;
		let expected_weight: Weight = DbWeight::get().reads(3 as Weight) +
			DbWeight::get().writes(2 as Weight) * max_delayed_events;
		assert_eq!(
			EventProof::on_initialize(frame_system::Pallet::<Test>::block_number() + 1),
			expected_weight
		);
		// Ensure event has been removed from delayed claims
		assert!(EventProof::pending_event_proofs(event_proof_id).is_none());
	});
}

#[ignore]
#[test]
fn multiple_delayed_event_proof() {
	new_test_ext().execute_with(|| {
		let message = &b"hello world"[..];
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		<BridgePaused<Test>>::put(true);
		MockValidatorAdapter::bridge_paused(true);
		assert_eq!(EventProof::bridge_paused(), true);

		let max_delayed_events = EventProof::delayed_event_proofs_per_block();
		let event_count: u8 = max_delayed_events * 2;
		let mut event_ids: Vec<EventProofId> = vec![];
		let mut events_for_proving = vec![];
		for _ in 0..event_count {
			let event_proof_id = EventProof::next_event_proof_id();
			event_ids.push(event_proof_id);
			let event_proof_info = SigningRequest::Ethereum(EthereumEventInfo {
				source,
				destination: destination.clone(),
				message: message.to_vec(),
				validator_set_id: MockValidatorAdapter::validator_set_id(),
				event_proof_id,
			});
			events_for_proving.push(event_proof_info.clone());
			// Generate event proof
			assert_ok!(EventProof::sign_eth_transaction(
				&source,
				&destination,
				&message,
				MockValidatorAdapter::validator_set_id()
			));
			// Ensure event has been added to delayed claims
			assert_eq!(EventProof::pending_event_proofs(event_proof_id), Some(event_proof_info));
			assert_eq!(EventProof::next_event_proof_id(), event_proof_id + 1);
		}

		// Re-enable bridge
		<BridgePaused<Test>>::kill();
		MockValidatorAdapter::bridge_kill();
		// initialize pallet and initiate event proof
		assert_eq!(
			EventProof::on_initialize(frame_system::Pallet::<Test>::block_number() + 1),
			DbWeight::get().reads(3 as Weight) +
				DbWeight::get().writes(2 as Weight) * max_delayed_events as u64
		);

		let mut removed_count = 0;
		for i in 0..event_count {
			// Ensure event has been removed from delayed claims
			if EventProof::pending_event_proofs(event_ids[i as usize]).is_none() {
				removed_count += 1;
			} else {
				assert_eq!(
					EventProof::pending_event_proofs(event_ids[i as usize]),
					Some(events_for_proving[i as usize].clone())
				)
			}
		}
		// Should have only processed max amount
		assert_eq!(removed_count, max_delayed_events);

		// Now initialize next block and process the rest
		assert_eq!(
			EventProof::on_initialize(frame_system::Pallet::<Test>::block_number() + 2),
			DbWeight::get().reads(3 as Weight) +
				DbWeight::get().writes(2 as Weight) * max_delayed_events as u64
		);

		let mut removed_count = 0;
		for i in 0..event_count {
			// Ensure event has been removed from delayed claims
			if EventProof::pending_event_proofs(event_ids[i as usize]).is_none() {
				removed_count += 1;
			}
		}
		// All events should have now been processed
		assert_eq!(removed_count, event_count);
	});
}

#[ignore]
#[test]
fn set_delayed_event_proofs_per_block() {
	new_test_ext().execute_with(|| {
		// Check that it starts as default value
		assert_eq!(EventProof::delayed_event_proofs_per_block(), 5);
		let new_max_delayed_events: u8 = 10;
		assert_ok!(EventProof::set_delayed_event_proofs_per_block(
			frame_system::RawOrigin::Root.into(),
			new_max_delayed_events
		));
		assert_eq!(EventProof::delayed_event_proofs_per_block(), new_max_delayed_events);

		let message = &b"hello world"[..];
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		let mut event_ids: Vec<EventProofId> = vec![];
		<BridgePaused<Test>>::put(true);
		MockValidatorAdapter::bridge_paused(true);

		for _ in 0..new_max_delayed_events {
			let event_proof_id = EventProof::next_event_proof_id();
			event_ids.push(event_proof_id);
			let event_proof_info = SigningRequest::Ethereum(EthereumEventInfo {
				source,
				destination: destination.clone(),
				message: message.to_vec(),
				validator_set_id: MockValidatorAdapter::validator_set_id(),
				event_proof_id,
			});
			// Generate event proof
			assert_ok!(EventProof::sign_eth_transaction(
				&source,
				&destination,
				&message,
				MockValidatorAdapter::validator_set_id()
			));
			// Ensure event has been added to delayed claims
			assert_eq!(EventProof::pending_event_proofs(event_proof_id), Some(event_proof_info));
			assert_eq!(EventProof::next_event_proof_id(), event_proof_id + 1);
		}

		// Re-enable bridge
		<BridgePaused<Test>>::kill();
		MockValidatorAdapter::bridge_kill();
		// initialize pallet and initiate event proof
		assert_eq!(
			EventProof::on_initialize(frame_system::Pallet::<Test>::block_number() + 1),
			DbWeight::get().reads(3 as Weight) +
				DbWeight::get().writes(2 as Weight) * new_max_delayed_events as u64
		);

		for i in 0..new_max_delayed_events {
			// Ensure event has been removed from delayed claims
			assert!(EventProof::pending_event_proofs(event_ids[i as usize]).is_none());
		}
	});
}

#[test]
fn set_delayed_event_proofs_per_block_not_root_should_fail() {
	new_test_ext().execute_with(|| {
		// Check that it starts as default value
		assert_eq!(EventProof::delayed_event_proofs_per_block(), 5);
		let new_value: u8 = 10;
		assert_noop!(
			EventProof::set_delayed_event_proofs_per_block(
				frame_system::RawOrigin::None.into(),
				new_value
			),
			DispatchError::BadOrigin
		);
		assert_eq!(EventProof::delayed_event_proofs_per_block(), 5);
	});
}

#[test]
fn sign_eth_transaction() {
	new_test_ext().execute_with(|| {
		// Test generating event proof without delay
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		let message = &b"hello world"[..];
		let event_proof_id = EventProof::next_event_proof_id();

		// Generate event proof
		assert_ok!(EventProof::sign_eth_transaction(
			&source,
			&destination,
			&message,
			MockValidatorAdapter::validator_set_id()
		));
		// Ensure event has not been added to delayed queue
		assert_eq!(EventProof::pending_event_proofs(event_proof_id), None);
		assert_eq!(EventProof::next_event_proof_id(), event_proof_id + 1);
		// On initialize does up to 2 reads to check for delayed proofs
		assert_eq!(
			EventProof::on_initialize(frame_system::Pallet::<Test>::block_number() + 1),
			DbWeight::get().reads(2 as Weight)
		);
	});
}

#[test]
fn request_multiple_event_proofs() {
	new_test_ext().execute_with(|| {
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		let message = &b"hello world"[..];

		assert_ok!(EventProof::sign_eth_transaction(
			&source,
			&destination,
			&message,
			MockValidatorAdapter::validator_set_id()
		));
		assert_ok!(EventProof::sign_eth_transaction(
			&source,
			&destination,
			&message,
			MockValidatorAdapter::validator_set_id()
		));
		let block_digest = <frame_system::Pallet<Test>>::digest();
		assert_eq!(block_digest.logs.len(), 2_usize);
	});
}
