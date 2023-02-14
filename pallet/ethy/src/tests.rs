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

use super::*;
use crate::mock::{
	EpochDuration, Ethy, ExtBuilder, MockEthBridgeAdapter, MockValidatorSetAdapter,
	MockXrplBridgeAdapter, Origin, System, TestRuntime,
};
use frame_support::{
	assert_noop, assert_ok,
	traits::{Hooks, OneSessionHandler},
};
use seed_pallet_common::ethy::EthereumBridgeAdapter;
use seed_primitives::{ethy::crypto::AuthorityId, xrpl::XrplAccountId, AccountId, BlockNumber};
use sp_core::ByteArray;
use sp_runtime::DispatchError::BadOrigin;
use std::default::Default;

#[test]
fn set_ethy_state() {
	ExtBuilder::default().build().execute_with(|| {
		// Default ethy state is Active
		assert_eq!(EthyState::<TestRuntime>::get(), State::Active);
		// try requesting for proof - should succeed
		let ethy_xrpl_request = EthySigningRequest::XrplTx(Vec::<u8>::default());
		System::reset_events();
		Ethy::request_for_event_proof(1, ethy_xrpl_request.clone());
		System::assert_has_event(
			Event::<TestRuntime>::EventSend {
				event_proof_id: 1,
				signing_request: ethy_xrpl_request.clone(),
			}
			.into(),
		);

		// set state to paused
		assert_ok!(Ethy::set_ethy_state(Origin::root(), State::Paused));
		// try requesting for proof - should buffer
		System::reset_events();
		Ethy::request_for_event_proof(1, ethy_xrpl_request.clone());
		System::assert_has_event(Event::<TestRuntime>::ProofDelayed { event_proof_id: 1 }.into());
		assert_eq!(PendingProofRequests::<TestRuntime>::get(1).unwrap(), ethy_xrpl_request);
	});
}
#[test]
fn get_next_event_proof_id() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(NextEventProofId::<TestRuntime>::get(), 0);
		let next_event_proof_id = Ethy::get_next_event_proof_id();
		assert_eq!(next_event_proof_id, 0);
		assert_eq!(NextEventProofId::<TestRuntime>::get(), next_event_proof_id.wrapping_add(1));
	});
}
#[test]
fn request_for_proof() {
	ExtBuilder::default().build().execute_with(|| {
		{
			// request for proof ethereum
			let next_proof_id = Ethy::get_next_event_proof_id();
			let eth_event_info = EthereumEventInfo {
				source: Default::default(),
				destination: Default::default(),
				message: vec![],
				validator_set_id: 0,
				event_proof_id: next_proof_id,
			};
			let proof_id = Ethy::request_for_proof(
				EthySigningRequest::Ethereum(eth_event_info),
				Some(next_proof_id),
			);
			assert_eq!(proof_id.unwrap(), next_proof_id);
		}
		{
			// request for proof xrpl
			let next_proof_id = NextEventProofId::<TestRuntime>::get();
			let xrpl_payload = Vec::<u8>::default();
			let proof_id = Ethy::request_for_proof(EthySigningRequest::XrplTx(xrpl_payload), None);
			assert_eq!(proof_id.unwrap(), next_proof_id);
		}
	});
}
fn get_validator_set_change_payload_ethereum(
	info: &ValidatorSetChangeInfo<AuthorityId>,
) -> ethabi::Bytes {
	let next_validator_addresses: Vec<Token> = info
		.next_validator_set
		.to_vec()
		.into_iter()
		.map(|k| EthyEcdsaToEthereum::convert(k.as_ref()))
		.map(|k| Token::Address(k.into()))
		.collect();
	ethabi::encode(&[
		Token::Array(next_validator_addresses),
		Token::Uint(info.next_validator_set_id.into()),
	])
}
fn get_validator_set_change_payload_xrpl(info: &ValidatorSetChangeInfo<AuthorityId>) -> Vec<u8> {
	// we don't need the actual implementation here. should be as same as
	MockXrplBridgeAdapter::get_signer_list_set_payload(Vec::<(XrplAccountId, u16)>::default())
		.unwrap()
}
#[test]
fn validator_set_change_in_progress() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		let current_validator_set = vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
		];
		let current_validator_set_id = 0;
		let next_validator_set = vec![
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
		];
		let next_validator_set_id = 1;
		let change_info = ValidatorSetChangeInfo {
			current_validator_set_id,
			current_validator_set,
			next_validator_set_id,
			next_validator_set,
		};
		let proof_id = NextEventProofId::<TestRuntime>::get();
		System::reset_events();
		// trigger incoming from ValidatorSet pallet
		Ethy::validator_set_change_in_progress(change_info.clone());

		//check - eth proof and xrpl proof should be requested
		let eth_event_info = EthereumEventInfo {
			source: MockEthBridgeAdapter::get_pallet_id().into_account_truncating(),
			destination: MockEthBridgeAdapter::get_contract_address(),
			message: get_validator_set_change_payload_ethereum(&change_info),
			validator_set_id: change_info.current_validator_set_id,
			event_proof_id: proof_id,
		};
		// ethereum
		System::assert_has_event(
			Event::<TestRuntime>::EventSend {
				event_proof_id: proof_id,
				signing_request: EthySigningRequest::Ethereum(eth_event_info.clone()),
			}
			.into(),
		);
		assert_eq!(System::digest().logs.len(), 2_usize);
		assert_eq!(
			System::digest().logs[0],
			DigestItem::Consensus(
				ETHY_ENGINE_ID,
				ConsensusLog::OpaqueSigningRequest::<AuthorityId> {
					chain_id: EthyChainId::Ethereum,
					event_proof_id: proof_id,
					data: EthySigningRequest::Ethereum(eth_event_info).data(),
				}
				.encode(),
			)
		);
		System::assert_has_event(
			Event::<TestRuntime>::AuthoritySetChangeInProgress {
				event_proof_id: proof_id,
				new_validator_set_id: change_info.next_validator_set_id,
			}
			.into(),
		);
		assert_eq!(NotarySetProofId::<TestRuntime>::get(), proof_id);
		// XRPL
		let xrpl_paylod = get_validator_set_change_payload_xrpl(&change_info);
		System::assert_has_event(
			Event::<TestRuntime>::EventSend {
				event_proof_id: proof_id + 1,
				signing_request: EthySigningRequest::XrplTx(xrpl_paylod.clone()),
			}
			.into(),
		);
		assert_eq!(System::digest().logs.len(), 2_usize);
		assert_eq!(
			System::digest().logs[1],
			DigestItem::Consensus(
				ETHY_ENGINE_ID,
				ConsensusLog::OpaqueSigningRequest::<AuthorityId> {
					chain_id: EthyChainId::Xrpl,
					event_proof_id: proof_id + 1,
					data: EthySigningRequest::XrplTx(xrpl_paylod).data(),
				}
				.encode(),
			)
		);
		System::assert_has_event(
			Event::<TestRuntime>::XrplAuthoritySetChangeInProgress {
				event_proof_id: proof_id + 1,
				new_validator_set_id: change_info.next_validator_set_id,
			}
			.into(),
		);
		assert_eq!(XrplNotarySetProofId::<TestRuntime>::get(), proof_id + 1); // XRPL proof id  = Eth proof Id + 1

		// Ethy state should be updated to Paused state
		assert_eq!(EthyState::<TestRuntime>::get(), State::Paused);
	});
}
#[test]
fn validator_set_change_finalized() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		let new_validator_set = vec![
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
		];
		let new_validator_set_id = 1;
		let change_info = ValidatorSetChangeInfo {
			current_validator_set_id: new_validator_set_id,
			current_validator_set: new_validator_set,
			..Default::default()
		};
		System::reset_events();
		// trigger incoming from ValidatorSet pallet
		Ethy::validator_set_change_finalized(change_info.clone());
		//check
		assert_eq!(System::digest().logs.len(), 1_usize);
		assert_eq!(
			System::digest().logs[0],
			DigestItem::Consensus(
				ETHY_ENGINE_ID,
				ConsensusLog::AuthoritiesChange(ValidatorSet {
					validators: change_info.current_validator_set.clone(),
					id: change_info.current_validator_set_id,
					proof_threshold: MockEthBridgeAdapter::get_notarization_threshold()
						.mul_ceil(change_info.current_validator_set.len() as u32),
				})
				.encode()
			)
		);
		System::assert_has_event(
			Event::<TestRuntime>::AuthoritySetChangeFinalized {
				new_validator_set_id: change_info.current_validator_set_id,
			}
			.into(),
		);
		// Ethy should be in Active state now
		assert_eq!(EthyState::<TestRuntime>::get(), State::Active);
	});
}
