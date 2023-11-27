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
use crate::mock::{
	AssetsExt, RuntimeOrigin, System, Test, XRPLBridge, XrpAssetId, XrpTxChallengePeriod,
};
use seed_pallet_common::test_prelude::*;

/// Helper function to get the xrp balance of an address
fn xrp_balance_of(who: AccountId) -> u64 {
	AssetsExt::balance(XrpAssetId::get(), &who) as u64
}

fn process_transaction(relayer: AccountId) {
	let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
	let transaction_hash_1 = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317C";
	XRPLBridge::initialize_relayer(&vec![relayer]);
	submit_transaction(relayer, 1_000_000, transaction_hash, relayer.into(), 1);
	submit_transaction(relayer, 1_000_001, transaction_hash_1, relayer.into(), 1);

	XRPLBridge::on_initialize(XrpTxChallengePeriod::get() as u64 + 1);
	System::set_block_number(XrpTxChallengePeriod::get() as u64 + 1);

	let xrp_balance = xrp_balance_of(relayer);
	assert_eq!(xrp_balance, 2000);
}

fn submit_transaction(
	relayer: AccountId,
	ledger_index: u64,
	transaction_hash: &[u8; 64],
	account_address: H160,
	i: u64,
) {
	let transaction =
		XrplTxData::Payment { amount: (i * 1000u64) as Balance, address: account_address };
	assert_ok!(XRPLBridge::submit_transaction(
		RuntimeOrigin::signed(relayer),
		ledger_index,
		XrplTxHash::from_slice(transaction_hash),
		transaction,
		1234
	));
}

#[test]
fn submit_transaction_replay_within_submission_window() {
	TestExt::<Test>::default().build().execute_with(|| {
		let relayer = create_account(1);
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let transaction =
			XrplTxData::Payment { amount: 1000 as Balance, address: H160::from_low_u64_be(555) };
		assert_ok!(XRPLBridge::add_relayer(RuntimeOrigin::root(), relayer));

		// Set replay protection data
		HighestSettledLedgerIndex::<Test>::put(10);
		SubmissionWindowWidth::<Test>::put(8);

		// test the submission window end
		assert_ok!(XRPLBridge::submit_transaction(
			RuntimeOrigin::signed(relayer),
			2,
			XrplTxHash::from_slice(transaction_hash),
			transaction.clone(),
			1234
		));
		assert_noop!(
			XRPLBridge::submit_transaction(
				RuntimeOrigin::signed(relayer),
				2,
				XrplTxHash::from_slice(transaction_hash),
				transaction.clone(),
				1234
			),
			Error::<Test>::TxReplay
		);
	});
}

#[test]
fn submit_transaction_outside_submission_window() {
	TestExt::<Test>::default().build().execute_with(|| {
		let relayer = create_account(1);
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let transaction =
			XrplTxData::Payment { amount: 1000 as Balance, address: H160::from_low_u64_be(555) };
		assert_ok!(XRPLBridge::add_relayer(RuntimeOrigin::root(), relayer));

		// Set replay protection data
		HighestSettledLedgerIndex::<Test>::put(10);
		SubmissionWindowWidth::<Test>::put(5);

		let submission_window_end = 10 - 5;
		assert_noop!(
			XRPLBridge::submit_transaction(
				RuntimeOrigin::signed(relayer),
				submission_window_end - 1,
				XrplTxHash::from_slice(transaction_hash),
				transaction,
				1234
			),
			Error::<Test>::OutSideSubmissionWindow
		);
	});
}

#[test]
fn submit_transaction_with_default_replay_protection_values_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let relayer = create_account(1);
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let ledger_index = 1;
		let transaction =
			XrplTxData::Payment { amount: 1000 as Balance, address: H160::from_low_u64_be(555) };
		assert_ok!(XRPLBridge::add_relayer(RuntimeOrigin::root(), relayer));

		// We don't set  replay protection data so that it would use default values.

		assert_ok!(XRPLBridge::submit_transaction(
			RuntimeOrigin::signed(relayer),
			ledger_index,
			XrplTxHash::from_slice(transaction_hash),
			transaction,
			1234
		));
	});
}

#[test]
fn add_transaction_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let tx_address = create_account(12);
		let relayer = create_account(1);
		XRPLBridge::initialize_relayer(&vec![relayer]);
		for i in 0..9u64 {
			let mut transaction_hash = transaction_hash.clone();
			transaction_hash[0] = i as u8;
			submit_transaction(relayer, i * 1_000_000, &transaction_hash, tx_address.into(), i);
		}
	})
}

#[test]
fn adding_more_transactions_than_the_limit_returns_error() {
	TestExt::<Test>::default().build().execute_with(|| {
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let tx_address = create_account(12);
		let relayer = create_account(1);
		XRPLBridge::initialize_relayer(&vec![relayer]);
		for i in 0..<Test as crate::Config>::XRPTransactionLimit::get() {
			let i = i as u64;
			let mut transaction_hash = transaction_hash.clone();
			transaction_hash[0] = i as u8;
			submit_transaction(relayer, i * 1_000_000, &transaction_hash, tx_address.into(), i);
		}

		let transaction =
			XrplTxData::Payment { amount: (1 * 1000u64) as Balance, address: tx_address.into() };

		let err = XRPLBridge::submit_transaction(
			RuntimeOrigin::signed(relayer),
			100,
			XrplTxHash::from_slice(transaction_hash),
			transaction,
			1234,
		);
		assert_noop!(err, Error::<Test>::CannotProcessMoreTransactionsAtThatBlock);
	})
}

#[test]
fn process_transaction_works() {
	TestExt::<Test>::default().with_asset(2, "XRP", &[]).build().execute_with(|| {
		process_transaction(create_account(1));
	})
}

#[test]
fn process_transaction_challenge_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let tx_address = create_account(12);
		let relayer = create_account(1);
		let challenger = create_account(2);
		XRPLBridge::initialize_relayer(&vec![relayer]);
		submit_transaction(relayer, 1_000_000, transaction_hash, tx_address.into(), 1);
		assert_ok!(XRPLBridge::submit_challenge(
			RuntimeOrigin::signed(challenger),
			XrplTxHash::from_slice(transaction_hash),
		));
		XRPLBridge::on_initialize(XrpTxChallengePeriod::get() as u64);
		System::set_block_number(XrpTxChallengePeriod::get() as u64);

		let xrp_balance = xrp_balance_of(tx_address);
		assert_eq!(xrp_balance, 0);
	})
}

#[test]
fn set_door_tx_fee_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let new_fee = 123456_u64;
		assert_ok!(XRPLBridge::set_door_tx_fee(frame_system::RawOrigin::Root.into(), new_fee));
		assert_eq!(XRPLBridge::door_tx_fee(), new_fee);

		// Only root can sign this tx, this should fail
		let account = AccountId::from(H160::from_slice(b"6490B68F1116BFE87DDC"));
		assert_noop!(XRPLBridge::set_door_tx_fee(RuntimeOrigin::signed(account), 0), BadOrigin);
	});
}

#[test]
fn withdraw_request_works() {
	TestExt::<Test>::default().with_asset(2, "XRP", &[]).build().execute_with(|| {
		// For this test we will set the door_tx_fee to 0
		assert_ok!(XRPLBridge::set_door_tx_fee(frame_system::RawOrigin::Root.into(), 0_u64));

		let door = XrplAccountId::from_slice(b"5490B68F2d16B3E87cba");
		let destination = XrplAccountId::from_slice(b"6490B68F1116BFE87DDD");
		let account = create_account(1);
		process_transaction(account); // 2000 XRP deposited

		// set initial ticket sequence params
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			RuntimeOrigin::root(),
			1_u32,
			1_u32,
			200_u32
		));

		// door address unset
		assert_noop!(
			XRPLBridge::withdraw_xrp(RuntimeOrigin::signed(account), 1000, destination),
			Error::<Test>::DoorAddressNotSet
		);
		assert_ok!(XRPLBridge::set_door_address(RuntimeOrigin::root(), door));

		// Withdraw half of available xrp
		assert_ok!(XRPLBridge::withdraw_xrp(RuntimeOrigin::signed(account), 1000, destination));
		let xrp_balance = xrp_balance_of(account);
		assert_eq!(xrp_balance, 1000);

		// Withdraw more than available XRP should throw BalanceLow error
		assert_noop!(
			XRPLBridge::withdraw_xrp(RuntimeOrigin::signed(account), 1001, destination),
			pallet_assets::Error::<Test>::BalanceLow
		);

		// Withdraw second half
		assert_ok!(XRPLBridge::withdraw_xrp(RuntimeOrigin::signed(account), 1000, destination));
		let xrp_balance = xrp_balance_of(account);
		assert_eq!(xrp_balance, 0);

		// No xrp left to withdraw, should fail as account is reaped
		assert_noop!(
			XRPLBridge::withdraw_xrp(RuntimeOrigin::signed(account), 1, destination),
			pallet_assets::Error::<Test>::NoAccount
		);
	})
}

#[test]
fn withdraw_request_works_with_door_fee() {
	TestExt::<Test>::default().with_asset(2, "XRP", &[]).build().execute_with(|| {
		// For this test we will set the door_tx_fee to 100
		let door_tx_fee = 100_u64;
		assert_ok!(XRPLBridge::set_door_tx_fee(frame_system::RawOrigin::Root.into(), door_tx_fee));
		let account = create_account(1);
		process_transaction(account); // 2000 XRP deposited
		let destination = XrplAccountId::from_slice(b"6490B68F1116BFE87DDD");
		let initial_xrp_balance = xrp_balance_of(account);
		let withdraw_amount: u64 = 1_000;

		// set initial ticket sequence params
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			RuntimeOrigin::root(),
			1_u32,
			1_u32,
			200_u32
		));
		// set door address
		assert_ok!(XRPLBridge::set_door_address(
			RuntimeOrigin::root(),
			b"6490B68F1116BFE87DDC".into()
		));

		assert_ok!(XRPLBridge::withdraw_xrp(
			RuntimeOrigin::signed(account),
			withdraw_amount.into(),
			destination
		));

		// Balance should be less withdraw amount and door fee
		let xrp_balance = xrp_balance_of(account);
		assert_eq!(xrp_balance, initial_xrp_balance - withdraw_amount - door_tx_fee);

		// Try again for remainding
		let initial_xrp_balance = xrp_balance_of(account);
		let withdraw_amount: u64 = 800;
		assert_ok!(XRPLBridge::withdraw_xrp(
			RuntimeOrigin::signed(account),
			withdraw_amount.into(),
			destination
		));

		// Balance should be less withdraw amount and door fee
		let xrp_balance = xrp_balance_of(account);
		assert_eq!(xrp_balance, initial_xrp_balance - withdraw_amount - door_tx_fee);

		// No funds left to withdraw
		assert_eq!(xrp_balance, 0);
		assert_noop!(
			XRPLBridge::withdraw_xrp(RuntimeOrigin::signed(account), 1, destination),
			pallet_assets::Error::<Test>::NoAccount
		);
	})
}

#[test]
fn withdraw_request_burn_fails() {
	TestExt::<Test>::default().with_asset(2, "XRP", &[]).build().execute_with(|| {
		// For this test we will set the door_tx_fee to 0 so we can ensure the Underflow is due to
		// the withdraw logic, not the door_tx_fee
		assert_ok!(XRPLBridge::set_door_tx_fee(frame_system::RawOrigin::Root.into(), 0_u64));
		assert_ok!(XRPLBridge::set_door_address(
			RuntimeOrigin::root(),
			b"6490B68F1116BFE87DDC".into()
		));

		let account = create_account(2);
		let destination = XrplAccountId::from_slice(b"6490B68F1116BFE87DDD");
		assert_noop!(
			XRPLBridge::withdraw_xrp(RuntimeOrigin::signed(account), 1000, destination),
			pallet_assets::Error::<Test>::NoAccount
		);
	})
}

#[test]
fn set_door_address_success() {
	TestExt::<Test>::default().build().execute_with(|| {
		let xprl_door_address = b"6490B68F1116BFE87DDD";
		assert_ok!(XRPLBridge::set_door_address(
			RuntimeOrigin::root(),
			H160::from(xprl_door_address)
		));
		assert_eq!(XRPLBridge::door_address(), Some(H160::from_slice(xprl_door_address)));
	})
}

#[test]
fn set_door_address_fail() {
	TestExt::<Test>::default().build().execute_with(|| {
		let xprl_door_address = b"6490B68F1116BFE87DDD";
		let caller = XrplAccountId::from_low_u64_be(1);
		assert_noop!(
			XRPLBridge::set_door_address(
				RuntimeOrigin::signed(AccountId::from(caller)),
				H160::from(xprl_door_address)
			),
			BadOrigin
		);
		assert_eq!(XRPLBridge::door_address(), None);
	})
}

#[test]
fn settle_new_higher_ledger_index_brings_submission_window_forward() {
	TestExt::<Test>::default().build().execute_with(|| {
		let relayer = create_account(1);
		assert_ok!(XRPLBridge::add_relayer(RuntimeOrigin::root(), relayer));

		let tx_hash_1 = XrplTxHash::from_low_u64_be(123);
		let tx_hash_2 = XrplTxHash::from_low_u64_be(124);
		let tx_hash_3 = XrplTxHash::from_low_u64_be(125);

		// Set replay protection data
		let current_highest_settled_ledger_index = 8;
		HighestSettledLedgerIndex::<Test>::put(current_highest_settled_ledger_index);
		SubmissionWindowWidth::<Test>::put(5);

		let current_submission_window_end = 8 - 5;

		// Add settled tx data within the window
		<SettledXRPTransactionDetails<Test>>::try_append(2, tx_hash_1).unwrap();
		<SettledXRPTransactionDetails<Test>>::try_append(current_submission_window_end, tx_hash_2)
			.unwrap();
		let account: AccountId = [1_u8; 20].into();
		<ProcessXRPTransactionDetails<Test>>::insert(
			tx_hash_1,
			(2 as LedgerIndex, XrpTransaction::default(), account),
		);
		<ProcessXRPTransactionDetails<Test>>::insert(
			tx_hash_2,
			(current_submission_window_end as LedgerIndex, XrpTransaction::default(), account),
		);

		//Submit higher leder index
		let new_transaction =
			XrplTxData::Payment { amount: 1000 as Balance, address: H160::from_low_u64_be(555) };
		let new_highest = current_highest_settled_ledger_index + 1;
		assert_ok!(XRPLBridge::submit_transaction(
			RuntimeOrigin::signed(relayer),
			new_highest as u64,
			tx_hash_3,
			new_transaction.clone(),
			1234
		));

		let block_number = System::block_number() + XrpTxChallengePeriod::get() as u64;
		XRPLBridge::on_initialize(block_number);
		System::set_block_number(block_number);

		// data outside the previous submission window end will not be cleaned in this iteration.
		// Ideally it should have been cleaned by now.
		assert!(<SettledXRPTransactionDetails<Test>>::get(2).is_some());
		assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash_1).is_some());

		// data from current_submission_window_end to new submission window end shuld be cleaned
		assert!(<SettledXRPTransactionDetails<Test>>::get(current_submission_window_end).is_none());
		assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash_2).is_none());

		// new data should be added
		assert!(<SettledXRPTransactionDetails<Test>>::get(new_highest).is_some());
		assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash_3).is_some());

		// Try to replay data outside submission window now
		assert_noop!(
			XRPLBridge::submit_transaction(
				RuntimeOrigin::signed(relayer),
				2,
				tx_hash_1,
				XrplTxData::default(),
				1234
			),
			Error::<Test>::OutSideSubmissionWindow
		);
		assert_noop!(
			XRPLBridge::submit_transaction(
				RuntimeOrigin::signed(relayer),
				current_submission_window_end as LedgerIndex,
				tx_hash_2,
				XrplTxData::default(),
				1234
			),
			Error::<Test>::OutSideSubmissionWindow
		);

		// Try to replay data inside submission window now
		assert_noop!(
			XRPLBridge::submit_transaction(
				RuntimeOrigin::signed(relayer),
				new_highest as LedgerIndex,
				tx_hash_3,
				new_transaction,
				1234
			),
			Error::<Test>::TxReplay
		);
	});
}

#[test]
fn reset_settled_xrpl_tx_data_success() {
	TestExt::<Test>::default().build().execute_with(|| {
		let relayer = create_account(1);
		assert_ok!(XRPLBridge::add_relayer(RuntimeOrigin::root(), relayer));

		let tx_hash_1 = XrplTxHash::from_low_u64_be(123);
		let tx_hash_2 = XrplTxHash::from_low_u64_be(124);
		let tx_hash_3 = XrplTxHash::from_low_u64_be(125);

		// Set replay protection data
		let current_highest_settled_ledger_index = 8;
		HighestSettledLedgerIndex::<Test>::put(current_highest_settled_ledger_index);
		SubmissionWindowWidth::<Test>::put(5);

		let current_submission_window_end = 8 - 5;

		// Add settled tx data within the window
		<SettledXRPTransactionDetails<Test>>::try_append(current_submission_window_end, tx_hash_1)
			.unwrap();
		<SettledXRPTransactionDetails<Test>>::try_append(
			current_submission_window_end + 1,
			tx_hash_2,
		)
		.unwrap();
		let account: AccountId = [1_u8; 20].into();
		<ProcessXRPTransactionDetails<Test>>::insert(
			tx_hash_1,
			(current_submission_window_end as LedgerIndex, XrpTransaction::default(), account),
		);
		<ProcessXRPTransactionDetails<Test>>::insert(
			tx_hash_2,
			(
				(current_submission_window_end + 1) as LedgerIndex,
				XrpTransaction::default(),
				account,
			),
		);

		//Submit very high leder index to move the submission window to future
		let new_transaction =
			XrplTxData::Payment { amount: 1000 as Balance, address: H160::from_low_u64_be(555) };
		let new_highest = current_highest_settled_ledger_index + 100;
		assert_ok!(XRPLBridge::submit_transaction(
			RuntimeOrigin::signed(relayer),
			new_highest as u64,
			tx_hash_3,
			new_transaction.clone(),
			1234
		));

		let block_number = System::block_number() + XrpTxChallengePeriod::get() as u64;
		XRPLBridge::on_initialize(block_number);
		System::set_block_number(block_number);

		// all previous settled data should be pruned by now
		assert!(<SettledXRPTransactionDetails<Test>>::get(current_submission_window_end).is_none());
		assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash_1).is_none());

		assert!(
			<SettledXRPTransactionDetails<Test>>::get(current_submission_window_end + 1).is_none()
		);
		assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash_2).is_none());

		// new data should be added
		assert!(<SettledXRPTransactionDetails<Test>>::get(new_highest).is_some());
		assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash_3).is_some());

		// Correct the submission window to the following
		// highest submitted ledger index = 9, submission window width = 6
		// we need to make sure to reinstate already processed data within submission window (9-6,
		// 9)
		let settled_xrpl_tx_data = vec![
			(tx_hash_1, current_submission_window_end, XrpTransaction::default(), account),
			(tx_hash_2, current_submission_window_end + 1, XrpTransaction::default(), account),
		];
		assert_ok!(XRPLBridge::reset_settled_xrpl_tx_data(
			RuntimeOrigin::root(),
			9,
			6,
			Some(settled_xrpl_tx_data)
		));

		// Now Try to replay old data
		assert_noop!(
			XRPLBridge::submit_transaction(
				RuntimeOrigin::signed(relayer),
				current_submission_window_end as LedgerIndex,
				tx_hash_1,
				XrplTxData::default(),
				1234
			),
			Error::<Test>::TxReplay
		);
		assert_noop!(
			XRPLBridge::submit_transaction(
				RuntimeOrigin::signed(relayer),
				(current_submission_window_end + 1) as LedgerIndex,
				tx_hash_2,
				XrplTxData::default(),
				1234
			),
			Error::<Test>::TxReplay
		);
		assert_noop!(
			XRPLBridge::submit_transaction(
				RuntimeOrigin::signed(relayer),
				new_highest as LedgerIndex,
				tx_hash_3,
				new_transaction,
				1234
			),
			Error::<Test>::TxReplay
		);

		// Try to replay data outside submission window
		assert_noop!(
			XRPLBridge::submit_transaction(
				RuntimeOrigin::signed(relayer),
				0,
				tx_hash_3,
				XrplTxData::default(),
				1234
			),
			Error::<Test>::OutSideSubmissionWindow
		);
	});
}

#[test]
fn reset_settled_xrpl_tx_data_can_only_be_called_by_root() {
	TestExt::<Test>::default().build().execute_with(|| {
		let account: AccountId = [1_u8; 20].into();
		assert_noop!(
			XRPLBridge::reset_settled_xrpl_tx_data(RuntimeOrigin::signed(account), 9, 6, None),
			BadOrigin
		);

		assert_ok!(XRPLBridge::reset_settled_xrpl_tx_data(RuntimeOrigin::root(), 9, 6, None));
	})
}

#[test]
fn get_door_ticket_sequence_success_at_start() {
	TestExt::<Test>::default().build().execute_with(|| {
		// set initial ticket sequence params
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			RuntimeOrigin::root(),
			1_u32,
			1_u32,
			200_u32
		));

		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(1));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(2));
	})
}

#[test]
fn get_door_ticket_sequence_success_at_start_if_initial_params_not_set() {
	TestExt::<Test>::default().build().execute_with(|| {
		System::set_block_number(1);
		let relayer = create_account(1);
		XRPLBridge::initialize_relayer(&vec![relayer]);

		assert_noop!(
			XRPLBridge::get_door_ticket_sequence(),
			Error::<Test>::NextTicketSequenceParamsNotSet
		);

		// set the params for next round
		assert_ok!(XRPLBridge::set_ticket_sequence_next_allocation(
			RuntimeOrigin::signed(relayer),
			3_u32, // start ticket sequence next round
			2_u32, // ticket sequence bucket size next round
		));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(3));
		assert_eq!(XRPLBridge::ticket_sequence_threshold_reached_emitted(), false);
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(4));
		assert_eq!(XRPLBridge::ticket_sequence_threshold_reached_emitted(), true);
		System::assert_has_event(Event::<Test>::TicketSequenceThresholdReached(4).into());

		// try to fetch again - error
		assert_err!(
			XRPLBridge::get_door_ticket_sequence(),
			Error::<Test>::NextTicketSequenceParamsNotSet
		);
		// try to fetch again - error
		assert_noop!(
			XRPLBridge::get_door_ticket_sequence(),
			Error::<Test>::NextTicketSequenceParamsNotSet
		);
	})
}

#[test]
fn get_door_ticket_sequence_success_over_next_round() {
	TestExt::<Test>::default().build().execute_with(|| {
		let relayer = create_account(1);
		XRPLBridge::initialize_relayer(&vec![relayer]);

		// set initial ticket sequence params
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			RuntimeOrigin::root(),
			1_u32,
			1_u32,
			2_u32
		));

		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(1));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(2));
		// need to set the next ticket params on or before the last of current
		assert_ok!(XRPLBridge::set_ticket_sequence_next_allocation(
			RuntimeOrigin::signed(relayer),
			3_u32, // start ticket sequence next round
			2_u32, // ticket sequence bucket size next round
		));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(3));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(4));
		assert_ok!(XRPLBridge::set_ticket_sequence_next_allocation(
			RuntimeOrigin::signed(relayer),
			10_u32, // start ticket sequence next round
			10_u32, // ticket sequence bucket size next round
		));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(10));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(11));
	})
}

#[test]
fn get_door_ticket_sequence_success_force_set_current_round() {
	TestExt::<Test>::default().build().execute_with(|| {
		let relayer = create_account(1);
		XRPLBridge::initialize_relayer(&vec![relayer]);

		// set initial ticket sequence params
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			RuntimeOrigin::root(),
			1_u32,
			1_u32,
			10_u32
		));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(1));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(2));

		// force set current values to (current=5 start=5, bucket_size=2)
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			RuntimeOrigin::root(),
			5_u32,
			5_u32,
			3_u32
		));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(5));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(6));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(7));

		// need to set the next ticket params on or before the last of current
		assert_ok!(XRPLBridge::set_ticket_sequence_next_allocation(
			RuntimeOrigin::signed(relayer),
			11_u32, // start ticket sequence next round
			2_u32,  // ticket sequence bucket size next round
		));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(11));
	})
}

#[test]
#[allow(non_snake_case)]
fn get_door_ticket_sequence_check_events_emitted() {
	TestExt::<Test>::default().build().execute_with(|| {
		System::set_block_number(1);
		let relayer = create_account(1);
		XRPLBridge::initialize_relayer(&vec![relayer]);

		assert_noop!(
			XRPLBridge::get_door_ticket_sequence(),
			Error::<Test>::NextTicketSequenceParamsNotSet
		);
		assert_noop!(
			XRPLBridge::get_door_ticket_sequence(),
			Error::<Test>::NextTicketSequenceParamsNotSet
		);

		// set the params for next round
		assert_ok!(XRPLBridge::set_ticket_sequence_next_allocation(
			RuntimeOrigin::signed(relayer),
			3_u32, // start ticket sequence next round
			3_u32, // ticket sequence bucket size next round
		));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(3));
		assert_eq!(XRPLBridge::ticket_sequence_threshold_reached_emitted(), false);
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(4));
		// event should be emitted here since ((4 - 3) + 1)/3 = 0.66 == TicketSequenceThreshold
		assert_eq!(XRPLBridge::ticket_sequence_threshold_reached_emitted(), true);
		System::assert_has_event(Event::<Test>::TicketSequenceThresholdReached(4).into());

		// try to fetch again - error - but no TicketSequenceThresholdReached
		System::reset_events();
		assert_eq!(System::events(), []);
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(5));
		assert_eq!(System::events(), []);

		// try to fetch again - error - but no TicketSequenceThresholdReached
		System::reset_events();
		assert_eq!(System::events(), []);
		assert_noop!(
			XRPLBridge::get_door_ticket_sequence(),
			Error::<Test>::NextTicketSequenceParamsNotSet
		);
		assert_eq!(System::events(), []);

		// set the params for next round
		assert_ok!(XRPLBridge::set_ticket_sequence_next_allocation(
			RuntimeOrigin::signed(relayer),
			10_u32, // start ticket sequence next round
			5_u32,  // ticket sequence bucket size next round
		));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(10));
	})
}

#[test]
fn set_ticket_sequence_current_allocation_success() {
	TestExt::<Test>::default().build().execute_with(|| {
		System::set_block_number(1);
		let relayer = create_account(1);
		XRPLBridge::initialize_relayer(&vec![relayer]);

		// set initial ticket sequence params
		System::reset_events();
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			RuntimeOrigin::root(),
			1_u32,
			1_u32,
			200_u32
		));
		System::assert_has_event(
			Event::<Test>::DoorTicketSequenceParamSet {
				ticket_sequence: 1_u32,
				ticket_sequence_start: 1_u32,
				ticket_bucket_size: 200_u32,
			}
			.into(),
		);

		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(1));

		// Force set the current param set
		System::reset_events();
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			RuntimeOrigin::root(),
			10_u32,
			1_u32,
			200_u32
		));
		System::assert_has_event(
			Event::<Test>::DoorTicketSequenceParamSet {
				ticket_sequence: 10_u32,
				ticket_sequence_start: 1_u32,
				ticket_bucket_size: 200_u32,
			}
			.into(),
		);
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(10));
	})
}

#[test]
fn set_ticket_sequence_current_allocation_failure() {
	TestExt::<Test>::default().build().execute_with(|| {
		System::set_block_number(1);
		let relayer = create_account(1);
		XRPLBridge::initialize_relayer(&vec![relayer]);

		// set initial ticket sequence params - success
		System::reset_events();
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			RuntimeOrigin::root(),
			1_u32,
			1_u32,
			200_u32
		));
		System::assert_has_event(
			Event::<Test>::DoorTicketSequenceParamSet {
				ticket_sequence: 1_u32,
				ticket_sequence_start: 1_u32,
				ticket_bucket_size: 200_u32,
			}
			.into(),
		);

		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(1));

		// Force set the current param set with ticket_bucket_size = 0
		System::reset_events();
		assert_noop!(
			XRPLBridge::set_ticket_sequence_current_allocation(
				RuntimeOrigin::root(),
				10_u32,
				10_u32,
				0_u32
			),
			Error::<Test>::TicketSequenceParamsInvalid
		);

		// try to fetch it, should give the next ticket sequence in current allocation
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(2));

		// Force set the current param set with ticket_sequence < current ticket_sequence
		System::reset_events();
		assert_noop!(
			XRPLBridge::set_ticket_sequence_current_allocation(
				RuntimeOrigin::root(),
				2_u32,
				1_u32,
				200_u32
			),
			Error::<Test>::TicketSequenceParamsInvalid
		);

		// Force set the current param set with start_ticket_sequence < current
		// start_ticket_sequence
		System::reset_events();
		assert_noop!(
			XRPLBridge::set_ticket_sequence_current_allocation(
				RuntimeOrigin::root(),
				10_u32,
				0_u32,
				200_u32
			),
			Error::<Test>::TicketSequenceParamsInvalid
		);

		// try to fetch it, should give the next ticket sequence in current allocation
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(3));

		// Force set the current param set with valid params, but with relayer
		System::reset_events();
		assert_noop!(
			XRPLBridge::set_ticket_sequence_current_allocation(
				RuntimeOrigin::signed(relayer),
				10_u32,
				1_u32,
				200_u32
			),
			BadOrigin
		);

		// Force set the same valid params set, with root
		System::reset_events();
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			RuntimeOrigin::root(),
			10_u32,
			1_u32,
			200_u32
		));

		// try to fetch it, should give the start of the new allocation
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(10));
	})
}

#[test]
fn set_ticket_sequence_next_allocation_success() {
	TestExt::<Test>::default().build().execute_with(|| {
		System::set_block_number(1);
		let relayer = create_account(1);
		XRPLBridge::initialize_relayer(&vec![relayer]);

		// no initial ticket sequence params, start setting the params for next allocation
		System::reset_events();
		assert_ok!(XRPLBridge::set_ticket_sequence_next_allocation(
			RuntimeOrigin::signed(relayer),
			1_u32,
			200_u32
		));
		System::assert_has_event(
			Event::<Test>::DoorNextTicketSequenceParamSet {
				ticket_sequence_start_next: 1_u32,
				ticket_bucket_size_next: 200_u32,
			}
			.into(),
		);

		// We did not set the initial door ticket sequence,
		// In a correct setup, we should set the initial param set using
		// set_ticket_sequence_current_allocation(). This demonstrates that even without it, setting
		// the next params would be enough. It will switch over and continue switch over happened,
		// should give start of the next allocation(1,200)
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(1));

		// Force update the current param set
		System::reset_events();
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			RuntimeOrigin::root(),
			10_u32,
			1_u32,
			12_u32
		));
		System::assert_has_event(
			Event::<Test>::DoorTicketSequenceParamSet {
				ticket_sequence: 10_u32,
				ticket_sequence_start: 1_u32,
				ticket_bucket_size: 12_u32,
			}
			.into(),
		);
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(10));

		// set the next params
		System::reset_events();
		assert_ok!(XRPLBridge::set_ticket_sequence_next_allocation(
			RuntimeOrigin::signed(relayer),
			15_u32,
			10_u32
		));
		System::assert_has_event(
			Event::<Test>::DoorNextTicketSequenceParamSet {
				ticket_sequence_start_next: 15_u32,
				ticket_bucket_size_next: 10_u32,
			}
			.into(),
		);

		// try to fetch, should still give the next in current allocation(11) since current
		// allocation is not consumed yet
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(11));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(12));

		// current allocation exhausted, switch over and give start of next allocation
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(15));
	})
}

#[test]
fn set_ticket_sequence_next_allocation_failure() {
	TestExt::<Test>::default().build().execute_with(|| {
		System::set_block_number(1);
		let relayer = create_account(1);
		XRPLBridge::initialize_relayer(&vec![relayer]);

		// set initial ticket sequence params - success
		System::reset_events();
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			RuntimeOrigin::root(),
			1_u32,
			1_u32,
			5_u32
		));
		System::assert_has_event(
			Event::<Test>::DoorTicketSequenceParamSet {
				ticket_sequence: 1_u32,
				ticket_sequence_start: 1_u32,
				ticket_bucket_size: 5_u32,
			}
			.into(),
		);

		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(1));

		// set the next param set with ticket_bucket_size = 0
		System::reset_events();
		assert_noop!(
			XRPLBridge::set_ticket_sequence_next_allocation(
				RuntimeOrigin::signed(relayer),
				10_u32,
				0_u32
			),
			Error::<Test>::NextTicketSequenceParamsInvalid
		);

		// try to fetch it, should give the next ticket sequence in current allocation
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(2));

		// set the next param set with start_ticket_sequence < current ticket_sequence
		System::reset_events();
		assert_noop!(
			XRPLBridge::set_ticket_sequence_next_allocation(
				RuntimeOrigin::signed(relayer),
				1_u32,
				200_u32
			),
			Error::<Test>::NextTicketSequenceParamsInvalid
		);

		// set the next param set with start_ticket_sequence < current start_ticket_sequence
		System::reset_events();
		assert_noop!(
			XRPLBridge::set_ticket_sequence_next_allocation(
				RuntimeOrigin::signed(relayer),
				0_u32,
				200_u32
			),
			Error::<Test>::NextTicketSequenceParamsInvalid
		);

		// try to fetch it, should give the next ticket sequence in current allocation
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(3));

		// set the next param set with valid params, but with !relayer
		System::reset_events();
		assert_noop!(
			XRPLBridge::set_ticket_sequence_next_allocation(
				RuntimeOrigin::signed(create_account(2)),
				10_u32,
				200_u32
			),
			Error::<Test>::NotPermitted
		);

		// same valid params set, with relayer
		System::reset_events();
		assert_ok!(XRPLBridge::set_ticket_sequence_next_allocation(
			RuntimeOrigin::signed(relayer),
			10_u32,
			200_u32
		));
		System::assert_has_event(
			Event::<Test>::DoorNextTicketSequenceParamSet {
				ticket_sequence_start_next: 10_u32,
				ticket_bucket_size_next: 200_u32,
			}
			.into(),
		);

		// try to fetch it, should give from the current allocation
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(4));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(5));

		// switch over
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(10));
	})
}

#[test]
fn process_xrp_tx_success() {
	TestExt::<Test>::default().with_asset(2, "XRP", &[]).build().execute_with(|| {
		System::set_block_number(1);
		let account = create_account(12);
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let relayer = create_account(1);
		XRPLBridge::initialize_relayer(&vec![relayer]);

		// submit payment tx
		let payment_tx =
			XrplTxData::Payment { amount: (1 * 1000u64) as Balance, address: account.into() };
		assert_ok!(XRPLBridge::submit_transaction(
			RuntimeOrigin::signed(relayer),
			1_000_000,
			XrplTxHash::from_slice(transaction_hash),
			payment_tx,
			1234
		));

		System::reset_events();
		XRPLBridge::process_xrp_tx(XrpTxChallengePeriod::get() as u64 + 1);
		System::set_block_number(XrpTxChallengePeriod::get() as u64 + 1);
		System::assert_has_event(
			Event::<Test>::ProcessingOk(1_000_000_u64, XrplTxHash::from_slice(transaction_hash))
				.into(),
		);

		let xrp_balance = xrp_balance_of(account);
		assert_eq!(xrp_balance, 1000);
	})
}

#[test]
fn process_xrp_tx_not_supported_transaction() {
	TestExt::<Test>::default().build().execute_with(|| {
		System::set_block_number(1);
		let account = create_account(2);
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let relayer = create_account(1);
		XRPLBridge::initialize_relayer(&vec![relayer]);

		// submit currency payment tx
		let currency_payment_tx = XrplTxData::CurrencyPayment {
			amount: (1 * 1000u64) as Balance,
			address: account.into(),
			currency_id: H256::random(),
		};
		assert_ok!(XRPLBridge::submit_transaction(
			RuntimeOrigin::signed(relayer),
			1_000_000,
			XrplTxHash::from_slice(transaction_hash),
			currency_payment_tx,
			1234
		));

		System::reset_events();
		XRPLBridge::process_xrp_tx(XrpTxChallengePeriod::get() as u64 + 1);
		System::set_block_number(XrpTxChallengePeriod::get() as u64 + 1);
		System::assert_has_event(Event::<Test>::NotSupportedTransaction.into());

		let xrp_balance = xrp_balance_of(account);
		assert_eq!(xrp_balance, 0);
	})
}

#[test]
fn process_xrp_tx_processing_failed() {
	TestExt::<Test>::default().with_asset(2, "XRP", &[]).build().execute_with(|| {
		System::set_block_number(1);
		let account_address = b"6490B68F1116BFE87DDC";
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317C";
		let transaction_hash2 = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317D";
		let relayer = create_account(1);
		XRPLBridge::initialize_relayer(&vec![relayer]);
		{
			// submit payment tx - this will mint the max Balance for the asset ID
			let payment_tx = XrplTxData::Payment {
				amount: u128::MAX as Balance,
				address: H160::from_slice(account_address),
			};
			assert_ok!(XRPLBridge::submit_transaction(
				RuntimeOrigin::signed(relayer),
				1_000_000,
				XrplTxHash::from_slice(transaction_hash),
				payment_tx,
				1234
			));

			System::reset_events();
			XRPLBridge::process_xrp_tx(XrpTxChallengePeriod::get() as u64 + 1);
			System::set_block_number(XrpTxChallengePeriod::get() as u64 + 1);
			System::assert_has_event(
				Event::<Test>::ProcessingOk(
					1_000_000_u64,
					XrplTxHash::from_slice(transaction_hash),
				)
				.into(),
			);

			let xrp_balance =
				AssetsExt::balance(XrpAssetId::get(), &H160::from_slice(account_address).into());
			assert_eq!(xrp_balance, u128::MAX);
		}
		{
			// submit payment tx to mint 1 more than max Balance. Should fail.
			let payment_tx = XrplTxData::Payment {
				amount: 1_u128 as Balance,
				address: H160::from_slice(account_address),
			};
			assert_ok!(XRPLBridge::submit_transaction(
				RuntimeOrigin::signed(relayer),
				1_000_001,
				XrplTxHash::from_slice(transaction_hash2),
				payment_tx,
				1235
			));

			System::reset_events();
			XRPLBridge::process_xrp_tx(2 * (XrpTxChallengePeriod::get() as u64) + 1);
			System::set_block_number(2 * (XrpTxChallengePeriod::get() as u64) + 1);
			System::assert_has_event(
				Event::<Test>::ProcessingFailed(
					1_000_001_u64,
					XrplTxHash::from_slice(transaction_hash2),
					ArithmeticError::Overflow.into(),
				)
				.into(),
			);

			// no changes to the account_address balance
			let xrp_balance =
				AssetsExt::balance(XrpAssetId::get(), &H160::from_slice(account_address).into());
			assert_eq!(xrp_balance, u128::MAX);
		}
	})
}
