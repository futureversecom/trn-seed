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
	AssetsExt, DelayedPaymentBlockLimit, MaxPrunedTransactionsPerBlock, RuntimeOrigin, System,
	Test, XRPLBridge, XrpAssetId, XrpTxChallengePeriod,
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
fn set_xrp_source_tag_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let new_source_tag = 723456_u32;
		assert_ok!(XRPLBridge::set_xrp_source_tag(
			frame_system::RawOrigin::Root.into(),
			new_source_tag
		));
		assert_eq!(mock::SourceTag::get(), new_source_tag);

		// Only root can sign this tx, this should fail
		let account = AccountId::from(H160::from_slice(b"6490B68F1116BFE87DDC"));
		assert_noop!(XRPLBridge::set_xrp_source_tag(RuntimeOrigin::signed(account), 0), BadOrigin);
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
			TokenError::FundsUnavailable
		);

		// Withdraw second half
		assert_ok!(XRPLBridge::withdraw_xrp(RuntimeOrigin::signed(account), 1000, destination));
		let xrp_balance = xrp_balance_of(account);
		assert_eq!(xrp_balance, 0);

		// No xrp left to withdraw, should fail as account is reaped
		assert_noop!(
			XRPLBridge::withdraw_xrp(RuntimeOrigin::signed(account), 1, destination),
			TokenError::FundsUnavailable
		);
	})
}

#[test]
fn withdraw_request_with_destination_tag_works() {
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

		let destination_tag = 14321_u32;

		// door address unset
		assert_noop!(
			XRPLBridge::withdraw_xrp_with_destination_tag(
				RuntimeOrigin::signed(account),
				1000,
				destination,
				destination_tag
			),
			Error::<Test>::DoorAddressNotSet
		);
		assert_ok!(XRPLBridge::set_door_address(RuntimeOrigin::root(), door));

		// Withdraw half of available xrp
		assert_ok!(XRPLBridge::withdraw_xrp_with_destination_tag(
			RuntimeOrigin::signed(account),
			1000,
			destination,
			destination_tag
		));
		let xrp_balance = xrp_balance_of(account);
		assert_eq!(xrp_balance, 1000);

		// Withdraw more than available XRP should throw BalanceLow error
		assert_noop!(
			XRPLBridge::withdraw_xrp_with_destination_tag(
				RuntimeOrigin::signed(account),
				1001,
				destination,
				destination_tag
			),
			TokenError::FundsUnavailable
		);

		// Withdraw second half
		assert_ok!(XRPLBridge::withdraw_xrp_with_destination_tag(
			RuntimeOrigin::signed(account),
			1000,
			destination,
			destination_tag
		));
		let xrp_balance = xrp_balance_of(account);
		assert_eq!(xrp_balance, 0);

		// No xrp left to withdraw, should fail as account is reaped
		assert_noop!(
			XRPLBridge::withdraw_xrp_with_destination_tag(
				RuntimeOrigin::signed(account),
				1,
				destination,
				destination_tag
			),
			TokenError::FundsUnavailable
		);
	})
}

#[test]
fn withdraw_request_with_destination_tag_works_with_door_fee() {
	TestExt::<Test>::default().with_asset(2, "XRP", &[]).build().execute_with(|| {
		// For this test we will set the door_tx_fee to 100
		let door_tx_fee = 100_u64;
		assert_ok!(XRPLBridge::set_door_tx_fee(frame_system::RawOrigin::Root.into(), door_tx_fee));
		let account = create_account(1);
		process_transaction(account); // 2000 XRP deposited
		let destination = XrplAccountId::from_slice(b"6490B68F1116BFE87DDD");
		let initial_xrp_balance = xrp_balance_of(account);
		let withdraw_amount: u64 = 1_000;
		let destination_tag = 39321_u32;

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

		assert_ok!(XRPLBridge::withdraw_xrp_with_destination_tag(
			RuntimeOrigin::signed(account),
			withdraw_amount.into(),
			destination,
			destination_tag
		));

		// Balance should be less withdraw amount and door fee
		let xrp_balance = xrp_balance_of(account);
		assert_eq!(xrp_balance, initial_xrp_balance - withdraw_amount - door_tx_fee);

		// Try again for remainding
		let initial_xrp_balance = xrp_balance_of(account);
		let withdraw_amount: u64 = 800;
		assert_ok!(XRPLBridge::withdraw_xrp_with_destination_tag(
			RuntimeOrigin::signed(account),
			withdraw_amount.into(),
			destination,
			destination_tag
		));

		// Balance should be less withdraw amount and door fee
		let xrp_balance = xrp_balance_of(account);
		assert_eq!(xrp_balance, initial_xrp_balance - withdraw_amount - door_tx_fee);

		// No funds left to withdraw
		assert_eq!(xrp_balance, 0);
		assert_noop!(
			XRPLBridge::withdraw_xrp_with_destination_tag(
				RuntimeOrigin::signed(account),
				1,
				destination,
				destination_tag
			),
			TokenError::FundsUnavailable
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
			TokenError::FundsUnavailable
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
			TokenError::FundsUnavailable
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
		HighestPrunedLedgerIndex::<Test>::put(0);
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
		XRPLBridge::on_idle(block_number, Weight::from_all(1_000_000_000u64));

		// data outside the previous submission window end should be cleaned now
		assert!(<SettledXRPTransactionDetails<Test>>::get(2).is_none());
		assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash_1).is_none());

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
		HighestPrunedLedgerIndex::<Test>::put(0);
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
		// Call on idle to prune the settled data
		XRPLBridge::on_idle(block_number, Weight::from_all(1_000_000_000u64));

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
			None,
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
fn reset_settled_xrpl_tx_data_invalid_highest_pruned_ledger_index() {
	TestExt::<Test>::default().build().execute_with(|| {
		let highest_settled_ledger_index = 9;
		let submission_window_width = 6;
		let highest_pruned_ledger_index =
			highest_settled_ledger_index - submission_window_width + 1;

		// Should fail as highest pruned is within the submission window
		assert_noop!(
			XRPLBridge::reset_settled_xrpl_tx_data(
				RuntimeOrigin::root(),
				highest_settled_ledger_index,
				submission_window_width,
				Some(highest_pruned_ledger_index),
				None
			),
			Error::<Test>::InvalidHighestPrunedIndex
		);

		// This should pass
		let highest_pruned_ledger_index = highest_settled_ledger_index - submission_window_width;
		assert_ok!(XRPLBridge::reset_settled_xrpl_tx_data(
			RuntimeOrigin::root(),
			highest_settled_ledger_index,
			submission_window_width,
			Some(highest_pruned_ledger_index),
			None
		));

		assert_eq!(HighestPrunedLedgerIndex::<Test>::get(), highest_pruned_ledger_index);
	})
}

#[test]
fn clear_storages_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let relayer = create_account(2);
		assert_ok!(XRPLBridge::add_relayer(RuntimeOrigin::root(), relayer));

		let tx_hash_1 = XrplTxHash::from_low_u64_be(123);
		let tx_hash_2 = XrplTxHash::from_low_u64_be(124);

		// Set replay protection data
		let current_highest_settled_ledger_index = 8;
		HighestSettledLedgerIndex::<Test>::put(current_highest_settled_ledger_index);
		HighestPrunedLedgerIndex::<Test>::put(2);
		SubmissionWindowWidth::<Test>::put(5);

		let ledger_index = 2;

		// Add settled tx data outside the window
		<SettledXRPTransactionDetails<Test>>::try_append(ledger_index, tx_hash_1).unwrap();
		<SettledXRPTransactionDetails<Test>>::try_append(ledger_index, tx_hash_2).unwrap();
		let account: AccountId = [1_u8; 20].into();
		<ProcessXRPTransactionDetails<Test>>::insert(
			tx_hash_1,
			(ledger_index as LedgerIndex, XrpTransaction::default(), account),
		);
		<ProcessXRPTransactionDetails<Test>>::insert(
			tx_hash_2,
			(ledger_index as LedgerIndex, XrpTransaction::default(), account),
		);

		XRPLBridge::on_initialize(XrpTxChallengePeriod::get() as u64);
		System::set_block_number(XrpTxChallengePeriod::get() as u64);
		// Call on idle to prune the settled data with enough weight to settle both
		let idle_weight = XRPLBridge::clear_storages(Weight::from_all(10_000_000_000u64));
		let expected_weight = DbWeight::get().reads_writes(4, 4);
		assert_eq!(idle_weight, expected_weight);

		// all previous settled data should be pruned by now
		assert!(<SettledXRPTransactionDetails<Test>>::get(ledger_index).is_none());
		assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash_1).is_none());
		assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash_2).is_none());
		assert_eq!(<HighestPrunedLedgerIndex<Test>>::get(), 3);
	});
}

#[test]
fn clear_storages_doesnt_exceed_max() {
	TestExt::<Test>::default().build().execute_with(|| {
		let relayer = create_account(2);
		assert_ok!(XRPLBridge::add_relayer(RuntimeOrigin::root(), relayer));

		// Set replay protection data where highest settled is far higher than highest pruned
		HighestSettledLedgerIndex::<Test>::put(10_000_000);
		HighestPrunedLedgerIndex::<Test>::put(0);
		SubmissionWindowWidth::<Test>::put(288_000);

		// Call clear storages to prune the settled data with plenty of weight
		let idle_weight = XRPLBridge::clear_storages(Weight::from_all(10_000_000_000_000u64));
		// Expected weight should be 5000 reads + 3 and 1 write for the highest pruned ledger index
		let expected_weight =
			DbWeight::get().reads_writes(3 + MaxPrunedTransactionsPerBlock::get() as u64, 1);
		assert_eq!(idle_weight, expected_weight);

		// HighestPrunedLedgerIndex should be set to the max
		assert_eq!(<HighestPrunedLedgerIndex<Test>>::get(), MaxPrunedTransactionsPerBlock::get());
	});
}

#[test]
fn clear_storages_returns_zero_if_not_enough_weight() {
	TestExt::<Test>::default().build().execute_with(|| {
		let relayer = create_account(2);
		assert_ok!(XRPLBridge::add_relayer(RuntimeOrigin::root(), relayer));

		// Set replay protection data
		let current_highest_settled_ledger_index = 8;
		HighestSettledLedgerIndex::<Test>::put(current_highest_settled_ledger_index);
		HighestPrunedLedgerIndex::<Test>::put(2);
		SubmissionWindowWidth::<Test>::put(5);

		// Add settled tx data within the window
		let ledger_index = 2;
		let tx_hash = XrplTxHash::from_low_u64_be(123);
		<SettledXRPTransactionDetails<Test>>::try_append(ledger_index, tx_hash).unwrap();
		let account: AccountId = [1_u8; 20].into();
		<ProcessXRPTransactionDetails<Test>>::insert(
			tx_hash,
			(ledger_index as LedgerIndex, XrpTransaction::default(), account),
		);

		XRPLBridge::on_initialize(XrpTxChallengePeriod::get() as u64);

		// Call on idle to prune the settled data with not enough weight to settle one tx
		let remaining_weight = DbWeight::get().reads_writes(4, 3);
		let idle_weight = XRPLBridge::clear_storages(remaining_weight);
		assert_eq!(idle_weight, Weight::from_all(0));
		// Data remains in place
		assert!(<SettledXRPTransactionDetails<Test>>::get(ledger_index).is_some());
		assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash).is_some());
		assert_eq!(<HighestPrunedLedgerIndex<Test>>::get(), 2);

		// Call on idle to prune the settled data with JUST enough weight to settle one tx
		let remaining_weight = DbWeight::get().reads_writes(4, 3);
		let idle_weight = XRPLBridge::clear_storages(remaining_weight + Weight::from_all(1u64));
		assert_eq!(idle_weight, remaining_weight);
		// Data updated
		assert!(<SettledXRPTransactionDetails<Test>>::get(ledger_index).is_none());
		assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash).is_none());
		assert_eq!(<HighestPrunedLedgerIndex<Test>>::get(), 3);
	});
}

#[test]
fn clear_storages_doesnt_exceed_on_idle_weight() {
	TestExt::<Test>::default().build().execute_with(|| {
		let relayer = create_account(2);
		assert_ok!(XRPLBridge::add_relayer(RuntimeOrigin::root(), relayer));

		// Set replay protection data
		let current_highest_settled_ledger_index = 8;
		HighestSettledLedgerIndex::<Test>::put(current_highest_settled_ledger_index);
		SubmissionWindowWidth::<Test>::put(5);
		HighestPrunedLedgerIndex::<Test>::put(2); // 8 - 5 - 1

		// Create data
		let tx_count = 10;
		let ledger_index = 2;
		let account: AccountId = [1_u8; 20].into();
		for i in 0..tx_count {
			let tx_hash = XrplTxHash::from_low_u64_be(i);
			<SettledXRPTransactionDetails<Test>>::try_append(ledger_index, tx_hash).unwrap();
			<ProcessXRPTransactionDetails<Test>>::insert(
				tx_hash,
				(ledger_index as LedgerIndex, XrpTransaction::default(), account),
			);
		}

		XRPLBridge::on_initialize(XrpTxChallengePeriod::get() as u64);
		System::set_block_number(XrpTxChallengePeriod::get() as u64);

		// Call on idle with enough weight to clear only 1 tx
		let remaining_weight = DbWeight::get().reads_writes(4, 2 + 1);
		let idle_weight = XRPLBridge::clear_storages(remaining_weight + Weight::from_all(1u64));
		// We subtract 1 from as we did not end up updating HighestPrunedLedgerIndex
		assert_eq!(idle_weight, remaining_weight - DbWeight::get().writes(1));
		// One settledXRPTransaction should have been removed
		assert_eq!(<SettledXRPTransactionDetails<Test>>::get(ledger_index).unwrap().len(), 9);
		// Highest remains at 2 because we have not cleared all the settled TX details for that
		// index
		assert_eq!(<HighestPrunedLedgerIndex<Test>>::get(), 2);
		// One ProcessXRPTransactionDetails should have been removed
		for i in 0..tx_count {
			let tx_hash = XrplTxHash::from_low_u64_be(i);
			if i < 1 {
				assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash).is_none());
			} else {
				assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash).is_some());
			}
		}

		// Call on idle with enough weight to clear 4 more txs
		let remaining_weight = DbWeight::get().reads_writes(4, 2 + 4);
		let idle_weight = XRPLBridge::clear_storages(remaining_weight + Weight::from_all(1u64));
		// We subtract 1 from as we did not end up updating HighestPrunedLedgerIndex
		assert_eq!(idle_weight, remaining_weight - DbWeight::get().writes(1));
		// 5 settledXRPTransaction should have been removed total
		assert_eq!(<SettledXRPTransactionDetails<Test>>::get(ledger_index).unwrap().len(), 5);
		// Highest remains at 2 because we have not cleared all the settled TX details for that
		// index
		assert_eq!(<HighestPrunedLedgerIndex<Test>>::get(), 2);
		// 5 ProcessXRPTransactionDetails should have been removed
		for i in 0..tx_count {
			let tx_hash = XrplTxHash::from_low_u64_be(i);
			if i < 5 {
				assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash).is_none());
			} else {
				assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash).is_some());
			}
		}

		// Call on idle with enough weight to clear the last 5 txs
		let remaining_weight = DbWeight::get().reads_writes(4, 2 + 5);
		let idle_weight = XRPLBridge::clear_storages(remaining_weight + Weight::from_all(1u64));
		assert_eq!(idle_weight, remaining_weight);
		// SettledXRPTransactionDetails should now be cleared
		assert!(<SettledXRPTransactionDetails<Test>>::get(ledger_index).is_none());
		// Highest is now 3 because we have cleared all the settled TX details for that index
		assert_eq!(<HighestPrunedLedgerIndex<Test>>::get(), 3);
		// All ProcessXRPTransactionDetails should have been removed
		for i in 0..tx_count {
			let tx_hash = XrplTxHash::from_low_u64_be(i);
			assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash).is_none());
		}
	});
}

#[test]
fn clear_storages_across_multiple_ledger_indices() {
	TestExt::<Test>::default().build().execute_with(|| {
		let relayer = create_account(2);
		assert_ok!(XRPLBridge::add_relayer(RuntimeOrigin::root(), relayer));

		// Set replay protection data
		let current_highest_settled_ledger_index = 9;
		HighestSettledLedgerIndex::<Test>::put(current_highest_settled_ledger_index);
		SubmissionWindowWidth::<Test>::put(5);
		HighestPrunedLedgerIndex::<Test>::put(2); // 8 - 5 - 2

		// Create data across 2 ledger indices
		let tx_count = 5;
		let ledger_index_1 = 2;
		let ledger_index_2 = 3;
		let account: AccountId = [1_u8; 20].into();
		for i in 0..tx_count {
			let tx_hash_1 = XrplTxHash::from_low_u64_be(i);
			let tx_hash_2 = XrplTxHash::from_low_u64_be(i + 10);
			<SettledXRPTransactionDetails<Test>>::try_append(ledger_index_1, tx_hash_1).unwrap();
			<SettledXRPTransactionDetails<Test>>::try_append(ledger_index_2, tx_hash_2).unwrap();
			<ProcessXRPTransactionDetails<Test>>::insert(
				tx_hash_1,
				(ledger_index_1 as LedgerIndex, XrpTransaction::default(), account),
			);
			<ProcessXRPTransactionDetails<Test>>::insert(
				tx_hash_2,
				(ledger_index_2 as LedgerIndex, XrpTransaction::default(), account),
			);
		}

		XRPLBridge::on_initialize(XrpTxChallengePeriod::get() as u64);
		System::set_block_number(XrpTxChallengePeriod::get() as u64);

		// Call on idle with enough weight to clear both ledger indices
		let base_weight = DbWeight::get().reads_writes(3, 1);
		let weight_per_index = DbWeight::get().reads_writes(1, 1);
		let weight_per_hash = DbWeight::get().writes(1);
		let remaining_weight = base_weight + (weight_per_index * 2) + (weight_per_hash * 10);
		let idle_weight = XRPLBridge::clear_storages(remaining_weight + Weight::from_all(1u64));
		assert_eq!(idle_weight, remaining_weight);

		// SettledXRPTransactionDetails should now be cleared
		assert!(<SettledXRPTransactionDetails<Test>>::get(ledger_index_1).is_none());
		assert!(<SettledXRPTransactionDetails<Test>>::get(ledger_index_2).is_none());
		// Highest is now 4 because we have cleared all the settled TX details for both indices
		assert_eq!(<HighestPrunedLedgerIndex<Test>>::get(), 4);
		// All ProcessXRPTransactionDetails should have been removed
		for i in 0..tx_count {
			let tx_hash_1 = XrplTxHash::from_low_u64_be(i);
			let tx_hash_2 = XrplTxHash::from_low_u64_be(i + 10);
			assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash_1).is_none());
			assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash_2).is_none());
		}
	});
}

#[test]
fn clear_storages_nothing_to_prune() {
	TestExt::<Test>::default().build().execute_with(|| {
		// Set replay protection data
		HighestSettledLedgerIndex::<Test>::put(8);
		SubmissionWindowWidth::<Test>::put(5);
		HighestPrunedLedgerIndex::<Test>::put(3); // 8 - 5

		// Call on idle and only use enough weight to read the 3 storage values
		let idle_weight = XRPLBridge::clear_storages(Weight::from_all(10_000_000_000u64));
		// 3 reads for the base storage values
		let expected_weight = DbWeight::get().reads(3);
		assert_eq!(idle_weight, expected_weight);
		assert_eq!(HighestPrunedLedgerIndex::<Test>::get(), 3);

		// Set replay protection data with one empty ledger index
		HighestSettledLedgerIndex::<Test>::put(9);
		SubmissionWindowWidth::<Test>::put(5);
		HighestPrunedLedgerIndex::<Test>::put(3);

		// Call on idle and only use enough weight to read the 3 storage values
		// We have one additional write to update the HighestPrunedLedgerIndex
		let idle_weight = XRPLBridge::clear_storages(Weight::from_all(10_000_000_000u64));
		// Extra read and write:
		// read: SettledXRPTransactionDetails
		// write: HighestPrunedLedgerIndex
		let expected_weight = DbWeight::get().reads_writes(4, 1);
		assert_eq!(idle_weight, expected_weight);
		assert_eq!(HighestPrunedLedgerIndex::<Test>::get(), 4);
	});
}

#[test]
fn clear_storages_nothing_to_prune_increases_ledger_index() {
	TestExt::<Test>::default().build().execute_with(|| {
		// Set replay protection data
		HighestSettledLedgerIndex::<Test>::put(10500);
		SubmissionWindowWidth::<Test>::put(500);
		HighestPrunedLedgerIndex::<Test>::put(0); // 10000 to clear

		// Call on idle and only use enough weight to read 5000 ledger indices
		// Note there are 3 reads instead of 1, this is because it will stop when it doesn't have
		// enough weight to write the data in the case that there is data to write
		// So we need to give it enough weight to theoretically write if it can
		let remaining_weight = DbWeight::get().reads_writes(3 + 5000, 3);
		let idle_weight = XRPLBridge::clear_storages(remaining_weight + Weight::from_all(1));
		// It uses only enough weight to read all 5000
		let expected_weight = DbWeight::get().reads_writes(3 + 5000, 1);
		assert_eq!(idle_weight, expected_weight);
		assert_eq!(HighestPrunedLedgerIndex::<Test>::get(), 5000);

		// Call on idle with plenty of weight to cover the last 5000 ledger indices
		// It should only use as much as it needs and no more
		let idle_weight = XRPLBridge::clear_storages(Weight::from_all(u64::MAX));
		// It uses only enough weight to read all 5000
		let expected_weight = DbWeight::get().reads_writes(3 + 5000, 1);
		assert_eq!(idle_weight, expected_weight);
		assert_eq!(HighestPrunedLedgerIndex::<Test>::get(), 10_000);
	});
}

#[test]
fn prune_settled_ledger_index_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		System::reset_events();
		HighestSettledLedgerIndex::<Test>::put(8);
		SubmissionWindowWidth::<Test>::put(5);
		HighestPrunedLedgerIndex::<Test>::put(3);

		// Create data
		let tx_count = 10;
		let ledger_index: u32 = 2;
		let account: AccountId = [1_u8; 20].into();
		for i in 0..tx_count {
			let tx_hash = XrplTxHash::from_low_u64_be(i);
			<SettledXRPTransactionDetails<Test>>::try_append(ledger_index, tx_hash).unwrap();
			<ProcessXRPTransactionDetails<Test>>::insert(
				tx_hash,
				(ledger_index as LedgerIndex, XrpTransaction::default(), account),
			);
		}

		assert_ok!(XRPLBridge::prune_settled_ledger_index(RuntimeOrigin::root(), ledger_index));

		// SettledXRPTransactionDetails should now be cleared
		assert!(<SettledXRPTransactionDetails<Test>>::get(ledger_index).is_none());
		// Doesn't affect HighestPrunedLedgerIndex
		assert_eq!(<HighestPrunedLedgerIndex<Test>>::get(), 3);
		// All ProcessXRPTransactionDetails should have been removed
		for i in 0..tx_count {
			let tx_hash = XrplTxHash::from_low_u64_be(i);
			assert!(<ProcessXRPTransactionDetails<Test>>::get(tx_hash).is_none());
		}
	})
}

#[test]
fn prune_settled_ledger_index_inside_submission_window_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		HighestSettledLedgerIndex::<Test>::put(8);
		SubmissionWindowWidth::<Test>::put(5);

		let ledger_index = 3; // Still within submission window
		assert_noop!(
			XRPLBridge::prune_settled_ledger_index(RuntimeOrigin::root(), ledger_index),
			Error::<Test>::CannotPruneActiveLedgerIndex
		);
	})
}

#[test]
fn prune_settled_ledger_index_no_transaction_details_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		HighestSettledLedgerIndex::<Test>::put(8);
		SubmissionWindowWidth::<Test>::put(5);

		let ledger_index = 2;
		assert_noop!(
			XRPLBridge::prune_settled_ledger_index(RuntimeOrigin::root(), ledger_index),
			Error::<Test>::NoTransactionDetails
		);
	})
}

#[test]
fn prune_settled_ledger_index_only_root() {
	TestExt::<Test>::default().build().execute_with(|| {
		let account: AccountId = [1_u8; 20].into();
		assert_noop!(
			XRPLBridge::prune_settled_ledger_index(RuntimeOrigin::signed(account), 9,),
			DispatchError::BadOrigin
		);
	})
}

#[test]
fn set_payment_delay_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let payment_delay = Some((100, 1000));
		assert_ok!(XRPLBridge::set_payment_delay(RuntimeOrigin::root(), payment_delay));
		assert_eq!(PaymentDelay::<Test>::get(), payment_delay);
		System::assert_has_event(
			Event::<Test>::PaymentDelaySet { payment_threshold: 100, delay: 1000 }.into(),
		);

		let payment_delay_2 = None;
		assert_ok!(XRPLBridge::set_payment_delay(RuntimeOrigin::root(), payment_delay_2));
		assert_eq!(PaymentDelay::<Test>::get(), payment_delay_2);
		System::assert_has_event(Event::<Test>::PaymentDelayRemoved.into());

		let payment_delay_3 = Some((1234, 123456789));
		assert_ok!(XRPLBridge::set_payment_delay(RuntimeOrigin::root(), payment_delay_3));
		assert_eq!(PaymentDelay::<Test>::get(), payment_delay_3);
		System::assert_has_event(
			Event::<Test>::PaymentDelaySet { payment_threshold: 1234, delay: 123456789 }.into(),
		);
	})
}

#[test]
fn set_payment_delay_not_sudo_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let payment_delay = Some((100, 1000));
		let account: AccountId = [1_u8; 20].into();
		assert_noop!(
			XRPLBridge::set_payment_delay(RuntimeOrigin::signed(account), payment_delay),
			BadOrigin
		);
	})
}

#[test]
fn withdraw_with_payment_delay_works() {
	let account = create_account(1);
	let initial_balance = 10000;
	TestExt::<Test>::default()
		.with_asset(XRP_ASSET_ID, "XRP", &[(account, initial_balance)])
		.build()
		.execute_with(|| {
			let amount = 100;
			let door = XrplAccountId::from_slice(b"5490B68F2d16B3E87cba");
			let destination = XrplAccountId::from_slice(b"6490B68F1116BFE87DDD");
			let delay_blocks = 1000;
			let payment_delay = Some((100, 1000)); // (min_balance, delay)
			let block_number = System::block_number();

			// Set initial parameters
			assert_ok!(XRPLBridge::set_door_tx_fee(frame_system::RawOrigin::Root.into(), 0_u64));
			assert_ok!(XRPLBridge::set_payment_delay(RuntimeOrigin::root(), payment_delay));
			assert_ok!(XRPLBridge::set_door_address(RuntimeOrigin::root(), door));
			assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
				RuntimeOrigin::root(),
				1_u32,
				1_u32,
				200_u32
			));

			// Get door ticket sequence before
			let next_ticket_sequence = XRPLBridge::get_door_ticket_sequence().unwrap() + 1;
			// Check NextPaymentId before
			let delayed_payment_id = NextDelayedPaymentId::<Test>::get();
			let payment_block = block_number + delay_blocks;

			// Withdraw amount which should add to pending withdrawals
			assert_ok!(XRPLBridge::withdraw_xrp(
				RuntimeOrigin::signed(account),
				amount,
				destination
			));

			// Ensure event is thrown
			System::assert_has_event(
				Event::<Test>::WithdrawDelayed {
					sender: account,
					amount,
					destination: destination.clone(),
					delayed_payment_id,
					payment_block,
				}
				.into(),
			);

			// Expected tx data
			let tx_data = XrpWithdrawTransaction {
				tx_nonce: 0_u32,
				tx_fee: 0,
				amount,
				destination,
				tx_ticket_sequence: next_ticket_sequence,
			};
			let delayed_withdrawal = DelayedWithdrawal {
				sender: account,
				destination_tag: None,
				withdraw_tx: tx_data.clone(),
			};
			// Check balance is reduced
			let xrp_balance = AssetsExt::balance(XrpAssetId::get(), &account);
			assert_eq!(xrp_balance, initial_balance - amount);

			// Check storage is correctly mutated
			assert_eq!(NextDelayedPaymentId::<Test>::get(), delayed_payment_id + 1);
			assert_eq!(DelayedPayments::<Test>::get(delayed_payment_id), Some(delayed_withdrawal));
			assert_eq!(
				DelayedPaymentSchedule::<Test>::get(block_number + delay_blocks)
					.unwrap()
					.into_inner(),
				vec![delayed_payment_id]
			);
		})
}

#[test]
fn withdraw_with_destination_tag_payment_delay_works() {
	let account = create_account(1);
	let initial_balance = 10000;
	TestExt::<Test>::default()
		.with_asset(XRP_ASSET_ID, "XRP", &[(account, initial_balance)])
		.build()
		.execute_with(|| {
			let amount = 100;
			let door = XrplAccountId::from_slice(b"5490B68F2d16B3E87cba");
			let destination = XrplAccountId::from_slice(b"6490B68F1116BFE87DDD");
			let payment_delay = Some((100, 1000)); // (min_balance, delay)
			let destination_tag = 12;
			let delay_blocks = 1000;
			let block_number = System::block_number();
			let payment_block = block_number + delay_blocks;

			// Set initial parameters
			assert_ok!(XRPLBridge::set_door_tx_fee(frame_system::RawOrigin::Root.into(), 0_u64));
			assert_ok!(XRPLBridge::set_payment_delay(RuntimeOrigin::root(), payment_delay));
			assert_ok!(XRPLBridge::set_door_address(RuntimeOrigin::root(), door));
			assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
				RuntimeOrigin::root(),
				1_u32,
				1_u32,
				200_u32
			));

			// Get door ticket sequence before
			let next_ticket_sequence = XRPLBridge::get_door_ticket_sequence().unwrap() + 1;
			// Check NextPaymentId before
			let delayed_payment_id = NextDelayedPaymentId::<Test>::get();

			// Withdraw amount which should add to pending withdrawals
			assert_ok!(XRPLBridge::withdraw_xrp_with_destination_tag(
				RuntimeOrigin::signed(account),
				amount,
				destination,
				destination_tag
			));

			// Ensure event is thrown
			System::assert_has_event(
				Event::<Test>::WithdrawDelayed {
					sender: account,
					amount,
					destination: destination.clone(),
					delayed_payment_id,
					payment_block,
				}
				.into(),
			);

			// Expected tx data, including destination tag
			let tx_data = XrpWithdrawTransaction {
				tx_nonce: 0_u32,
				tx_fee: 0,
				amount,
				destination,
				tx_ticket_sequence: next_ticket_sequence,
			};
			let delayed_withdrawal = DelayedWithdrawal {
				sender: account,
				destination_tag: Some(destination_tag),
				withdraw_tx: tx_data.clone(),
			};
			// Check balance is reduced
			let xrp_balance = AssetsExt::balance(XrpAssetId::get(), &account);
			assert_eq!(xrp_balance, initial_balance - amount);

			// Check storage is correctly mutated
			assert_eq!(NextDelayedPaymentId::<Test>::get(), delayed_payment_id + 1);
			assert_eq!(DelayedPayments::<Test>::get(delayed_payment_id), Some(delayed_withdrawal));
			assert_eq!(
				DelayedPaymentSchedule::<Test>::get(1001).unwrap().into_inner(),
				vec![delayed_payment_id]
			);
		})
}

#[test]
fn withdraw_below_payment_delay_does_not_delay_payment() {
	let account = create_account(1);
	let initial_balance = 10000;
	TestExt::<Test>::default()
		.with_asset(XRP_ASSET_ID, "XRP", &[(account, initial_balance)])
		.build()
		.execute_with(|| {
			let amount = 99; // 1 below payment_delay amount
			let door = XrplAccountId::from_slice(b"5490B68F2d16B3E87cba");
			let destination = XrplAccountId::from_slice(b"6490B68F1116BFE87DDD");
			let payment_delay = Some((100, 1000));

			// Set initial parameters
			assert_ok!(XRPLBridge::set_door_tx_fee(frame_system::RawOrigin::Root.into(), 0_u64));
			assert_ok!(XRPLBridge::set_payment_delay(RuntimeOrigin::root(), payment_delay));
			assert_ok!(XRPLBridge::set_door_address(RuntimeOrigin::root(), door));
			assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
				RuntimeOrigin::root(),
				1_u32,
				1_u32,
				200_u32
			));

			// Check NextPaymentId before
			let delayed_payment_id = NextDelayedPaymentId::<Test>::get();

			// Withdraw amount which should add to pending withdrawals
			assert_ok!(XRPLBridge::withdraw_xrp(
				RuntimeOrigin::signed(account),
				amount,
				destination
			));

			// Ensure event is thrown
			System::assert_last_event(
				Event::<Test>::WithdrawRequest {
					proof_id: 1,
					sender: account,
					amount,
					destination: destination.clone(),
				}
				.into(),
			);

			// Check balance is reduced
			let xrp_balance = AssetsExt::balance(XrpAssetId::get(), &account);
			assert_eq!(xrp_balance, initial_balance - amount);

			// Check delay storage is unchanged
			assert_eq!(NextDelayedPaymentId::<Test>::get(), delayed_payment_id);
			assert_eq!(DelayedPayments::<Test>::get(delayed_payment_id), None);
			assert_eq!(DelayedPaymentSchedule::<Test>::get(1001), None);
		})
}

#[test]
fn process_delayed_payments_works() {
	let account = create_account(1);
	let initial_balance = 10000;
	TestExt::<Test>::default()
		.with_asset(XRP_ASSET_ID, "XRP", &[(account, initial_balance)])
		.build()
		.execute_with(|| {
			let amount = 100;
			let door = XrplAccountId::from_slice(b"5490B68F2d16B3E87cba");
			let destination = XrplAccountId::from_slice(b"6490B68F1116BFE87DDD");
			let payment_delay = Some((100, 1000)); // (min_balance, delay)

			// Set initial parameters
			assert_ok!(XRPLBridge::set_door_tx_fee(frame_system::RawOrigin::Root.into(), 0_u64));
			assert_ok!(XRPLBridge::set_payment_delay(RuntimeOrigin::root(), payment_delay));
			assert_ok!(XRPLBridge::set_door_address(RuntimeOrigin::root(), door));
			assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
				RuntimeOrigin::root(),
				1_u32,
				1_u32,
				200_u32
			));

			// Check NextPaymentId before
			let delayed_payment_id = NextDelayedPaymentId::<Test>::get();

			// Withdraw amount which should add to pending withdrawals
			assert_ok!(XRPLBridge::withdraw_xrp(
				RuntimeOrigin::signed(account),
				amount,
				destination
			));

			// Check balance is reduced
			let xrp_balance = AssetsExt::balance(XrpAssetId::get(), &account);
			assert_eq!(xrp_balance, initial_balance - amount);

			// Set next process block to this block
			NextDelayProcessBlock::<Test>::put(1001);
			// Call process delayed payments with enough weight to process the delayed payment
			let weight_used =
				XRPLBridge::process_delayed_payments(1001, Weight::from_all(1_000_000_000_000));
			// Assert weight used is as expected
			assert_eq!(weight_used, DbWeight::get().reads_writes(6, 4));

			// Ensure event is thrown
			System::assert_last_event(
				Event::<Test>::WithdrawRequest {
					proof_id: 1,
					sender: account,
					amount,
					destination: destination.clone(),
				}
				.into(),
			);

			// Storage should now be updated
			assert_eq!(NextDelayedPaymentId::<Test>::get(), delayed_payment_id + 1);
			assert_eq!(DelayedPayments::<Test>::get(delayed_payment_id), None);
			assert_eq!(DelayedPaymentSchedule::<Test>::get(1001), None);
			assert_eq!(NextDelayProcessBlock::<Test>::get(), 1002);
		})
}

#[test]
fn process_delayed_payments_works_in_on_idle() {
	let account = create_account(1);
	let initial_balance = 10000;
	TestExt::<Test>::default()
		.with_asset(XRP_ASSET_ID, "XRP", &[(account, initial_balance)])
		.build()
		.execute_with(|| {
			let amount = 100;
			let door = XrplAccountId::from_slice(b"5490B68F2d16B3E87cba");
			let destination = XrplAccountId::from_slice(b"6490B68F1116BFE87DDD");
			let payment_delay = Some((100, 1000)); // (min_balance, delay)

			// Set initial parameters
			assert_ok!(XRPLBridge::set_door_tx_fee(frame_system::RawOrigin::Root.into(), 0_u64));
			assert_ok!(XRPLBridge::set_payment_delay(RuntimeOrigin::root(), payment_delay));
			assert_ok!(XRPLBridge::set_door_address(RuntimeOrigin::root(), door));
			assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
				RuntimeOrigin::root(),
				1_u32,
				1_u32,
				200_u32
			));

			// Check NextPaymentId before
			let delayed_payment_id = NextDelayedPaymentId::<Test>::get();

			// Withdraw amount which should add to pending withdrawals
			assert_ok!(XRPLBridge::withdraw_xrp(
				RuntimeOrigin::signed(account),
				amount,
				destination
			));

			// Check balance is reduced
			let xrp_balance = AssetsExt::balance(XrpAssetId::get(), &account);
			assert_eq!(xrp_balance, initial_balance - amount);

			// Set next process block to this block
			NextDelayProcessBlock::<Test>::put(1001);
			// Call process delayed payments with enough weight to process the delayed payment
			XRPLBridge::on_idle(1001, Weight::from_all(1_000_000_000_000));

			// Ensure event is thrown
			System::assert_last_event(
				Event::<Test>::WithdrawRequest {
					proof_id: 1,
					sender: account,
					amount,
					destination: destination.clone(),
				}
				.into(),
			);

			// Storage should now be updated
			assert_eq!(NextDelayedPaymentId::<Test>::get(), delayed_payment_id + 1);
			assert_eq!(DelayedPayments::<Test>::get(delayed_payment_id), None);
			assert_eq!(DelayedPaymentSchedule::<Test>::get(1001), None);
			assert_eq!(NextDelayProcessBlock::<Test>::get(), 1002);
		})
}

#[test]
fn process_delayed_payments_multiple_withdrawals() {
	let account = create_account(1);
	let initial_balance: u128 = 10000;
	TestExt::<Test>::default()
		.with_asset(XRP_ASSET_ID, "XRP", &[(account, initial_balance)])
		.build()
		.execute_with(|| {
			let amount: u128 = 10;
			let door = XrplAccountId::from_slice(b"5490B68F2d16B3E87cba");
			let destination = XrplAccountId::from_slice(b"6490B68F1116BFE87DDD");
			let payment_delay = Some((10, 1000)); // (min_balance, delay)

			// Set initial parameters
			assert_ok!(XRPLBridge::set_door_tx_fee(frame_system::RawOrigin::Root.into(), 0_u64));
			assert_ok!(XRPLBridge::set_payment_delay(RuntimeOrigin::root(), payment_delay));
			assert_ok!(XRPLBridge::set_door_address(RuntimeOrigin::root(), door));
			assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
				RuntimeOrigin::root(),
				1_u32,
				1_u32,
				200_u32
			));

			// Check NextPaymentId before
			let delayed_payment_id = NextDelayedPaymentId::<Test>::get();
			let withdrawal_count: u128 = 100;

			for _ in 0..withdrawal_count {
				// Withdraw amount which should add to pending withdrawals
				assert_ok!(XRPLBridge::withdraw_xrp(
					RuntimeOrigin::signed(account),
					amount,
					destination
				));
			}
			// Check storage updated for all withdrawals
			assert_eq!(
				NextDelayedPaymentId::<Test>::get(),
				delayed_payment_id + withdrawal_count as u64
			);
			assert_eq!(
				DelayedPaymentSchedule::<Test>::get(1001).unwrap().len(),
				withdrawal_count as usize
			);

			// Check balance is reduced
			let xrp_balance = AssetsExt::balance(XrpAssetId::get(), &account);
			assert_eq!(xrp_balance, initial_balance - (amount * withdrawal_count));

			// Set next process block to this block
			NextDelayProcessBlock::<Test>::put(1001);
			// Call process delayed payments with enough weight to process all delayed payments
			let weight_used =
				XRPLBridge::process_delayed_payments(1001, Weight::from_all(1_000_000_000_000));
			// Assert weight used is as expected
			let weight_per_tx = DbWeight::get().reads_writes(3u64, 2u64);
			let base_weight = DbWeight::get().reads_writes(7u64, 1u64);
			let total_weight = base_weight
				+ Weight::from_parts(
					weight_per_tx.ref_time() * withdrawal_count as u64,
					weight_per_tx.proof_size() * withdrawal_count as u64,
				);
			assert_eq!(weight_used, total_weight);

			// Storage should now be updated
			assert_eq!(DelayedPayments::<Test>::get(delayed_payment_id), None);
			assert_eq!(DelayedPaymentSchedule::<Test>::get(1001), None);
			assert_eq!(NextDelayProcessBlock::<Test>::get(), 1002);
		})
}

#[test]
fn process_delayed_payments_multiple_withdrawals_across_multiple_blocks() {
	let account = create_account(1);
	let initial_balance: u128 = 10000;
	TestExt::<Test>::default()
		.with_asset(XRP_ASSET_ID, "XRP", &[(account, initial_balance)])
		.build()
		.execute_with(|| {
			let amount: u128 = 10;
			let door = XrplAccountId::from_slice(b"5490B68F2d16B3E87cba");
			let destination = XrplAccountId::from_slice(b"6490B68F1116BFE87DDD");
			let payment_delay = Some((10, 1000)); // (min_balance, delay)

			// Set initial parameters
			assert_ok!(XRPLBridge::set_door_tx_fee(frame_system::RawOrigin::Root.into(), 0_u64));
			assert_ok!(XRPLBridge::set_payment_delay(RuntimeOrigin::root(), payment_delay));
			assert_ok!(XRPLBridge::set_door_address(RuntimeOrigin::root(), door));
			assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
				RuntimeOrigin::root(),
				1_u32,
				1_u32,
				200_u32
			));

			// Check NextPaymentId before
			let delayed_payment_id = NextDelayedPaymentId::<Test>::get();
			let withdrawal_count: u128 = 100;

			for i in 0..withdrawal_count {
				System::set_block_number(i as u64 + 1u64);
				// Withdraw amount which should add to pending withdrawals
				assert_ok!(XRPLBridge::withdraw_xrp(
					RuntimeOrigin::signed(account),
					amount,
					destination
				));
				assert_eq!(
					DelayedPaymentSchedule::<Test>::get(1000 + 1).unwrap().into_inner(),
					vec![delayed_payment_id as u64]
				);
			}
			// Check storage updated for all withdrawals
			assert_eq!(
				NextDelayedPaymentId::<Test>::get(),
				delayed_payment_id + withdrawal_count as u64
			);

			// Check balance is reduced
			let xrp_balance = AssetsExt::balance(XrpAssetId::get(), &account);
			assert_eq!(xrp_balance, initial_balance - (amount * withdrawal_count));

			// Set next process block to this block to be the first block we need to process
			NextDelayProcessBlock::<Test>::put(1001);
			// Call process delayed payments with enough weight to process all delayed payments
			// Set block number to the last block we need to process
			let weight_used =
				XRPLBridge::process_delayed_payments(1101, Weight::from_all(1_000_000_000_000));
			// Assert weight used is as expected
			let weight_per_tx = DbWeight::get().reads_writes(4u64, 3u64);
			let base_weight = DbWeight::get().reads_writes(3u64, 1u64);
			let total_weight = base_weight
				+ Weight::from_parts(
					weight_per_tx.ref_time() * withdrawal_count as u64,
					weight_per_tx.proof_size() * withdrawal_count as u64,
				);
			assert_eq!(weight_used, total_weight);

			// Storage should now be updated
			assert_eq!(DelayedPayments::<Test>::get(delayed_payment_id), None);
			assert_eq!(DelayedPaymentSchedule::<Test>::get(1001), None);
			assert_eq!(NextDelayProcessBlock::<Test>::get(), 1102);
		})
}

#[test]
fn process_delayed_payments_nothing_to_process_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let door = XrplAccountId::from_slice(b"5490B68F2d16B3E87cba");
		assert_ok!(XRPLBridge::set_door_address(RuntimeOrigin::root(), door));
		let delayed_payment_block_limit = DelayedPaymentBlockLimit::get(); // 1000
																   // Set next process block to 0
		NextDelayProcessBlock::<Test>::put(0);
		// Call process delayed payments with enough weight to process 1000 blocks
		let weight_used = XRPLBridge::process_delayed_payments(
			delayed_payment_block_limit,
			Weight::from_all(1_000_000_000_000_000),
		);
		// Assert weight used is as expected
		assert_eq!(weight_used, DbWeight::get().reads_writes(3 + delayed_payment_block_limit, 1));

		// NextDelayProcessBlock should now be updated to 1001
		assert_eq!(NextDelayProcessBlock::<Test>::get(), delayed_payment_block_limit + 1);

		// Call process delayed payments for the next block, should only process one block
		let weight_used = XRPLBridge::process_delayed_payments(
			delayed_payment_block_limit + 1,
			Weight::from_all(1_000_000_000_000_000),
		);
		// Assert weight used is as expected
		assert_eq!(weight_used, DbWeight::get().reads_writes(3, 1));

		// NextDelayProcessBlock should now be updated to 1001
		assert_eq!(NextDelayProcessBlock::<Test>::get(), delayed_payment_block_limit + 2);
	})
}

#[test]
fn process_delayed_payments_does_not_exceed_max_delayed_payments() {
	TestExt::<Test>::default().build().execute_with(|| {
		let door = XrplAccountId::from_slice(b"5490B68F2d16B3E87cba");
		assert_ok!(XRPLBridge::set_door_address(RuntimeOrigin::root(), door));
		let delayed_payment_block_limit = DelayedPaymentBlockLimit::get(); // 1000
																   // Set next process block to 0
		NextDelayProcessBlock::<Test>::put(0);
		// Call process delayed payments with more than max_payments_per_block
		let weight_used = XRPLBridge::process_delayed_payments(
			delayed_payment_block_limit + 10000,
			Weight::from_all(1_000_000_000_000_000),
		);
		// Assert weight used is as expected
		assert_eq!(weight_used, DbWeight::get().reads_writes(3 + delayed_payment_block_limit, 1));

		// NextDelayProcessBlock should now be updated to 1001
		assert_eq!(NextDelayProcessBlock::<Test>::get(), delayed_payment_block_limit + 1);
	})
}

#[test]
fn process_delayed_payments_not_enough_weight_returns_zero() {
	TestExt::<Test>::default().build().execute_with(|| {
		NextDelayProcessBlock::<Test>::put(1);

		// Call process delayed payments with not enough weight to process one payment
		let weight = DbWeight::get().reads_writes(7u64, 5u64);
		let weight_used = XRPLBridge::process_delayed_payments(1000, weight);
		// Assert weight used is as expected
		assert_eq!(weight_used, Weight::zero());

		// NextDelayProcessBlock should not have changed
		assert_eq!(NextDelayProcessBlock::<Test>::get(), 1);
	})
}

#[test]
fn reset_settled_xrpl_tx_data_can_only_be_called_by_root() {
	TestExt::<Test>::default().build().execute_with(|| {
		let account: AccountId = [1_u8; 20].into();
		assert_noop!(
			XRPLBridge::reset_settled_xrpl_tx_data(
				RuntimeOrigin::signed(account),
				9,
				6,
				None,
				None
			),
			BadOrigin
		);

		assert_ok!(XRPLBridge::reset_settled_xrpl_tx_data(RuntimeOrigin::root(), 9, 6, None, None));
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
fn process_xrp_tx_for_root_bridging_transaction() {
	TestExt::<Test>::default().build().execute_with(|| {
		System::set_block_number(1);
		let account = create_account(2);
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let relayer = create_account(1);
		XRPLBridge::initialize_relayer(&vec![relayer]);
		let currency =
			BoundedVec::try_from("524F4F5400000000000000000000000000000000".to_vec()).unwrap();
		assert_ok!(XRPLBridge::set_xrpl_asset_map(RuntimeOrigin::root(), 1_u32, currency));

		// submit currency payment tx
		let currency_payment_tx = XrplTxData::CurrencyPayment {
			amount: (1 * 1000u64) as Balance,
			address: account.into(),
			currency: currency.clone(), //H256::random(),
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
		System::assert_has_event(Event::<Test>::ProcessingOk.into());

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
