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
	EpochDuration, ExtBuilder, MockEthBridgeAdapter, Origin, Scheduler, System, TestRuntime,
	ValidatorSet,
};
use frame_support::{
	assert_noop, assert_ok,
	traits::{Hooks, OneSessionHandler},
};
use seed_pallet_common::ethy::EthereumBridgeAdapter;
use seed_primitives::{ethy::crypto::AuthorityId, AccountId, BlockNumber};
use sp_core::ByteArray;
use sp_runtime::DispatchError::BadOrigin;

#[test]
fn xrpl_door_signers_set_at_genesis() {
	let xrpl_door_signers_genesis = [0_u8; 33];

	ExtBuilder::default()
		.xrp_door_signers(xrpl_door_signers_genesis)
		.build()
		.execute_with(|| {
			assert_eq!(
				ValidatorSet::xrpl_door_signers(
					AuthorityId::from_slice(&xrpl_door_signers_genesis).unwrap()
				),
				true
			);
			assert_eq!(
				ValidatorSet::xrpl_door_signers(AuthorityId::from_slice(&[1_u8; 33]).unwrap()),
				false
			);
		});
}
#[test]
fn set_xrpl_door_signers_success() {
	let xrpl_door_signers = vec![
		AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
	];

	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(ValidatorSet::set_xrpl_door_signers(Origin::root(), xrpl_door_signers));
		assert_eq!(
			ValidatorSet::xrpl_door_signers(AuthorityId::from_slice(&[1_u8; 33]).unwrap()),
			true
		);
		assert_eq!(
			ValidatorSet::xrpl_door_signers(AuthorityId::from_slice(&[2_u8; 33]).unwrap()),
			true
		);
		assert_eq!(
			ValidatorSet::xrpl_door_signers(AuthorityId::from_slice(&[0_u8; 33]).unwrap()),
			false
		);
	});
}
#[test]
fn set_xrpl_door_signers_failed_non_root() {
	let xrpl_door_signers = vec![
		AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
	];

	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			ValidatorSet::set_xrpl_door_signers(
				Origin::signed(AccountId::from([1_u8; 20])),
				xrpl_door_signers
			),
			BadOrigin
		);
		assert_eq!(
			ValidatorSet::xrpl_door_signers(AuthorityId::from_slice(&[1_u8; 33]).unwrap()),
			false
		);
		assert_eq!(
			ValidatorSet::xrpl_door_signers(AuthorityId::from_slice(&[2_u8; 33]).unwrap()),
			false
		);
	});
}
#[test]
fn update_xrpl_notary_keys() {
	let xrpl_door_signers = vec![
		AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
	];
	ExtBuilder::default().build().execute_with(|| {
		//set door signers
		assert_ok!(ValidatorSet::set_xrpl_door_signers(Origin::root(), xrpl_door_signers));
		// new validators/notary keys
		let validators = vec![
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(), // overlap with xrpl door signers
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
		];
		// call update
		ValidatorSet::update_xrpl_notary_keys(&validators);
		// check NotaryXrplKeys == [2_u8; 33]
		assert_eq!(
			ValidatorSet::notary_xrpl_keys(),
			vec![AuthorityId::from_slice(&[2_u8; 33]).unwrap()]
		);
	});
}
#[test]
fn get_xrpl_notary_keys() {
	let xrpl_door_signers = vec![
		AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
	];
	ExtBuilder::default().build().execute_with(|| {
		//set door signers
		assert_ok!(ValidatorSet::set_xrpl_door_signers(Origin::root(), xrpl_door_signers));
		{
			// no overlap between active notary keys and xrpl door signers
			// new validators/notary keys
			let validators = vec![
				AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
				AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
			];
			assert_eq!(ValidatorSet::get_xrpl_notary_keys(&validators), vec![]);
		}
		{
			// overlap between active notary keys and xrpl door signers < T::MaxXrplKeys
			// new validators/notary keys
			let validators = vec![
				AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
				AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
			];
			assert_eq!(
				ValidatorSet::get_xrpl_notary_keys(&validators),
				vec![AuthorityId::from_slice(&[1_u8; 33]).unwrap()]
			);
		}
		{
			// overlap between active notary keys and xrpl door signers > T::MaxXrplKeys(8)
			//set door signers more than T::MaxXrplKeys(8) entries
			assert_ok!(ValidatorSet::set_xrpl_door_signers(
				Origin::root(),
				vec![
					AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
					AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
					AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
					AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
					AuthorityId::from_slice(&[5_u8; 33]).unwrap(),
					AuthorityId::from_slice(&[6_u8; 33]).unwrap(),
					AuthorityId::from_slice(&[8_u8; 33]).unwrap(),
					AuthorityId::from_slice(&[8_u8; 33]).unwrap(),
					AuthorityId::from_slice(&[9_u8; 33]).unwrap(),
					AuthorityId::from_slice(&[10_u8; 33]).unwrap(),
				]
			));
			// new validators/notary keys with 9 overlapping entries
			let validators = vec![
				AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
				AuthorityId::from_slice(&[5_u8; 33]).unwrap(),
				AuthorityId::from_slice(&[6_u8; 33]).unwrap(),
				AuthorityId::from_slice(&[8_u8; 33]).unwrap(),
				AuthorityId::from_slice(&[8_u8; 33]).unwrap(),
				AuthorityId::from_slice(&[9_u8; 33]).unwrap(),
				AuthorityId::from_slice(&[10_u8; 33]).unwrap(),
				AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
				AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
				AuthorityId::from_slice(&[11_u8; 33]).unwrap(),
			];
			//only the first 8 that has an overlap is taken
			assert_eq!(
				ValidatorSet::get_xrpl_notary_keys(&validators),
				vec![
					AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
					AuthorityId::from_slice(&[5_u8; 33]).unwrap(),
					AuthorityId::from_slice(&[6_u8; 33]).unwrap(),
					AuthorityId::from_slice(&[8_u8; 33]).unwrap(),
					AuthorityId::from_slice(&[8_u8; 33]).unwrap(),
					AuthorityId::from_slice(&[9_u8; 33]).unwrap(),
					AuthorityId::from_slice(&[10_u8; 33]).unwrap(),
					AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
				]
			);
		}
	});
}
#[test]
fn get_eth_validator_set() {
	let validator_set = vec![
		AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[5_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[6_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[8_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[8_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[9_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[10_u8; 33]).unwrap(),
	];

	ExtBuilder::default().build().execute_with(|| {
		//set the notary keys
		NotaryKeys::<TestRuntime>::put(&validator_set);
		assert_eq!(ValidatorSet::notary_keys(), validator_set.clone());

		//query - should include all the NotaryKeys in the set
		assert_eq!(
			ValidatorSet::get_eth_validator_set(),
			ValidatorSetS {
				validators: validator_set.clone(),
				id: NotarySetId::<TestRuntime>::get(),
				proof_threshold: MockEthBridgeAdapter::get_notarization_threshold()
					.mul_ceil(validator_set.len() as u32)
			}
		);
	});
}
#[test]
fn get_xrpl_validator_set() {
	let validator_set = vec![
		AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[5_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[6_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[8_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[8_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[9_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[10_u8; 33]).unwrap(),
	];

	ExtBuilder::default().build().execute_with(|| {
		//set the notary keys
		NotaryKeys::<TestRuntime>::put(&validator_set);
		assert_eq!(ValidatorSet::notary_keys(), validator_set.clone());
		// set the xrpl door signers as same as NotaryKeys
		assert_ok!(ValidatorSet::set_xrpl_door_signers(Origin::root(), validator_set.clone()));
		for item in validator_set {
			assert_eq!(ValidatorSet::xrpl_door_signers(item), true);
		}
		let notary_xrpl_keys = NotaryXrplKeys::<TestRuntime>::get();
		//query - should include only the NotaryXrplKeys in the set
		assert_eq!(
			ValidatorSet::get_xrpl_validator_set(),
			ValidatorSetS {
				validators: notary_xrpl_keys.clone(),
				id: NotarySetId::<TestRuntime>::get(),
				proof_threshold: notary_xrpl_keys.len().saturating_sub(1) as u32
			}
		);
	});
}
#[test]
fn natural_era_rotation_success() {
	let validator_keys = vec![
		AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
	];
	let default_account = AccountId::default();
	let next_validator_keys = vec![
		AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
	];
	let next_keys_iter = vec![
		(&default_account, AuthorityId::from_slice(&[3_u8; 33]).unwrap()),
		(&default_account, AuthorityId::from_slice(&[4_u8; 33]).unwrap()),
	]
	.into_iter();

	ExtBuilder::default().next_session_final().build().execute_with(|| {
		//set the notary keys
		NotaryKeys::<TestRuntime>::put(&validator_keys);
		assert_eq!(ValidatorSet::notary_keys(), validator_keys.clone());
		// set the xrpl door signers as same as NotaryKeys
		assert_ok!(ValidatorSet::set_xrpl_door_signers(Origin::root(), validator_keys.clone()));
		for item in validator_keys {
			assert_eq!(ValidatorSet::xrpl_door_signers(item), true);
		}
		let notary_xrpl_keys = NotaryXrplKeys::<TestRuntime>::get();
		let current_notary_set_id = NotarySetId::<TestRuntime>::get();

		// Call on_new_session but is_active_session_final is false
		<ValidatorSet as OneSessionHandler<AccountId>>::on_new_session(
			true,
			next_keys_iter.clone(),
			next_keys_iter.clone(),
		);
		// NextNotaryKeys has been updated. But NextValidatorSetChangeBlock should not be set
		// since not the last session
		assert_eq!(ValidatorSet::next_notary_keys(), next_validator_keys.clone());
		assert!(ValidatorSet::next_validator_set_change_block().is_none());

		let block_number: BlockNumber = 100;
		System::set_block_number(block_number.into());
		// Call on_new_session where is_active_session_final is true, should change storage
		<ValidatorSet as OneSessionHandler<AccountId>>::on_new_session(
			true,
			next_keys_iter.clone(),
			next_keys_iter.clone(),
		);
		let epoch_duration: BlockNumber = EpochDuration::get().saturated_into();
		let expected_block: BlockNumber = block_number + epoch_duration - 75_u32;
		assert_eq!(ValidatorSet::next_validator_set_change_block(), Some(expected_block as u64));
		assert_eq!(ValidatorSet::next_notary_keys(), next_validator_keys.clone());

		// Call on_initialise with the expected block
		System::reset_events();
		ValidatorSet::on_initialize(expected_block.into());
		// EthyAdapter should be notified and ValidatorsChangeInProgress is set to true
		assert_eq!(ValidatorsChangeInProgress::<TestRuntime>::get(), true);
		System::assert_has_event(
			Event::<TestRuntime>::ValidatorSetChangeInProgress {
				next_validator_set_id: current_notary_set_id.wrapping_add(1),
			}
			.into(),
		);

		// trigger end of the final session of the era
		System::reset_events();
		<ValidatorSet as OneSessionHandler<AccountId>>::on_before_session_ending();
		// check storage has been updated
		assert_eq!(ValidatorsChangeInProgress::<TestRuntime>::get(), false);
		assert_eq!(ValidatorSet::notary_keys(), next_validator_keys);
		assert_eq!(NotarySetId::<TestRuntime>::get(), current_notary_set_id.wrapping_add(1));
		let notary_xrpl_keys = ValidatorSet::get_xrpl_notary_keys(&next_validator_keys); // this will do the filtration with xrpl door signers
		assert_eq!(ValidatorSet::notary_xrpl_keys(), notary_xrpl_keys);
		System::assert_has_event(
			Event::<TestRuntime>::ValidatorSetChangeFinalizeSuccess {
				new_validator_set_id: current_notary_set_id.wrapping_add(1),
			}
			.into(),
		);
	});
}
#[test]
fn force_era_rotation_success() {
	let validator_keys = vec![
		AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
	];
	let default_account = AccountId::default();
	let next_validator_keys = vec![
		AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
		AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
	];
	let next_keys_iter = vec![
		(&default_account, AuthorityId::from_slice(&[3_u8; 33]).unwrap()),
		(&default_account, AuthorityId::from_slice(&[4_u8; 33]).unwrap()),
	]
	.into_iter();

	ExtBuilder::default().next_session_final().build().execute_with(|| {
		//set the notary keys
		NotaryKeys::<TestRuntime>::put(&validator_keys);
		assert_eq!(ValidatorSet::notary_keys(), validator_keys.clone());
		// set the xrpl door signers as same as NotaryKeys
		assert_ok!(ValidatorSet::set_xrpl_door_signers(Origin::root(), validator_keys.clone()));
		for item in validator_keys.clone() {
			assert_eq!(ValidatorSet::xrpl_door_signers(item), true);
		}
		let notary_xrpl_keys = NotaryXrplKeys::<TestRuntime>::get();
		let current_notary_set_id = NotarySetId::<TestRuntime>::get();

		// Call on_new_session but is_active_session_final is false
		<ValidatorSet as OneSessionHandler<AccountId>>::on_new_session(
			true,
			next_keys_iter.clone(),
			next_keys_iter.clone(),
		);
		// NextNotaryKeys has been updated. But NextValidatorSetChangeBlock should not be set
		// since not the last session
		assert_eq!(ValidatorSet::next_notary_keys(), next_validator_keys.clone());
		assert!(ValidatorSet::next_validator_set_change_block().is_none());

		let block_number: BlockNumber = 100;
		System::set_block_number(block_number.into());
		// Trigger the end of the final session of the era - This should be equivalent to forcing an
		// era since on_initialize() has not been triggered and no ValidatorSetChangeInProgress
		System::reset_events();
		<ValidatorSet as OneSessionHandler<AccountId>>::on_before_session_ending();
		// should trigger start_validator_set_change()
		// EthyAdapter should be notified and ValidatorsChangeInProgress is set to true
		assert_eq!(ValidatorsChangeInProgress::<TestRuntime>::get(), true);
		System::assert_has_event(
			Event::<TestRuntime>::ValidatorSetChangeInProgress {
				next_validator_set_id: current_notary_set_id.wrapping_add(1),
			}
			.into(),
		);
		// storage items should not be updated yet - same as old state
		assert_eq!(ValidatorSet::notary_keys(), validator_keys.clone());

		// finalise_validator_set_change() should be scheduled
		let scheduled_block: BlockNumber = block_number + 75_u32;
		System::reset_events();
		Scheduler::on_initialize(scheduled_block.into());
		// check storage has been updated
		assert_eq!(ValidatorsChangeInProgress::<TestRuntime>::get(), false);
		assert_eq!(ValidatorSet::notary_keys(), next_validator_keys);
		assert_eq!(NotarySetId::<TestRuntime>::get(), current_notary_set_id.wrapping_add(1));
		let notary_xrpl_keys = ValidatorSet::get_xrpl_notary_keys(&next_validator_keys); // this will do the filtration with xrpl door signers
		assert_eq!(ValidatorSet::notary_xrpl_keys(), notary_xrpl_keys);
		System::assert_has_event(
			Event::<TestRuntime>::ValidatorSetChangeFinalizeSuccess {
				new_validator_set_id: current_notary_set_id.wrapping_add(1),
			}
			.into(),
		);
	});
}
