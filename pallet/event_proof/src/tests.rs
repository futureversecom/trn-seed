/*
#[test]
fn xrpl_tx_signing_request() {
	ExtBuilder::default().build().execute_with(|| {
		let event_proof_id = EventProof::next_event_proof_id();

		// Request tx signing
		assert_ok!(EthBridge::sign_xrpl_transaction("hello world".as_bytes()), event_proof_id);
		// Ensure request has not been added to queue
		assert_eq!(EventProof::pending_event_proofs(event_proof_id), None);
		assert_eq!(EventProof::next_event_proof_id(), event_proof_id + 1);

		let signing_request = EthySigningRequest::XrplTx("hello world".as_bytes().to_vec());
		System::assert_has_event(
			Event::<TestRuntime>::EventSend {
				event_proof_id,
				signing_request: signing_request.clone(),
			}
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
		BridgePaused::put(true);
		assert_ok!(EthBridge::sign_xrpl_transaction("hello world".as_bytes()), event_proof_id + 1);
		assert_eq!(
			EventProof::pending_event_proofs(event_proof_id + 1),
			Some(EthySigningRequest::XrplTx("hello world".as_bytes().to_vec()))
		);

		System::assert_has_event(Event::<TestRuntime>::ProofDelayed(event_proof_id + 1).into());
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

		let event_proof_id = EventProof::next_event_proof_id();
		let event_proof_info = EthySigningRequest::Ethereum(EthereumEventInfo {
			source,
			destination: destination.clone(),
			message: message.to_vec(),
			validator_set_id: EthBridge::validator_set().id,
			event_proof_id,
		});

		// Generate event proof
		assert_ok!(EthBridge::send_event(&source, &destination, &message));
		// Ensure event has been added to delayed claims
		assert_eq!(EventProof::pending_event_proofs(event_proof_id), Some(event_proof_info));
		assert_eq!(EventProof::next_event_proof_id(), event_proof_id + 1);

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
		assert!(EventProof::pending_event_proofs(event_proof_id).is_none());
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
			let event_proof_id = EventProof::next_event_proof_id();
			event_ids.push(event_proof_id);
			let event_proof_info = EthySigningRequest::Ethereum(EthereumEventInfo {
				source,
				destination: destination.clone(),
				message: message.to_vec(),
				validator_set_id: EthBridge::validator_set().id,
				event_proof_id,
			});
			events_for_proving.push(event_proof_info.clone());
			// Generate event proof
			assert_ok!(EthBridge::send_event(&source, &destination, &message));
			// Ensure event has been added to delayed claims
			assert_eq!(EventProof::pending_event_proofs(event_proof_id), Some(event_proof_info));
			assert_eq!(EventProof::next_event_proof_id(), event_proof_id + 1);
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
			EthBridge::on_initialize(frame_system::Pallet::<TestRuntime>::block_number() + 2),
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
			let event_proof_id = EventProof::next_event_proof_id();
			event_ids.push(event_proof_id);
			let event_proof_info = EthySigningRequest::Ethereum(EthereumEventInfo {
				source,
				destination: destination.clone(),
				message: message.to_vec(),
				validator_set_id: EthBridge::validator_set().id,
				event_proof_id,
			});
			// Generate event proof
			assert_ok!(EthBridge::send_event(&source, &destination, &message));
			// Ensure event has been added to delayed claims
			assert_eq!(EventProof::pending_event_proofs(event_proof_id), Some(event_proof_info));
			assert_eq!(EventProof::next_event_proof_id(), event_proof_id + 1);
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
			assert!(EventProof::pending_event_proofs(event_ids[i as usize]).is_none());
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
fn send_event() {
	ExtBuilder::default().build().execute_with(|| {
		// Test generating event proof without delay
		let source = H160::from_low_u64_be(444);
		let destination = H160::from_low_u64_be(555);
		let message = &b"hello world"[..];
		let event_proof_id = EventProof::next_event_proof_id();

		// Generate event proof
		assert_ok!(EthBridge::send_event(&source, &destination, &message));
		// Ensure event has not been added to delayed queue
		assert_eq!(EventProof::pending_event_proofs(event_proof_id), None);
		assert_eq!(EventProof::next_event_proof_id(), event_proof_id + 1);
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
 */
