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

use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;

use crate::Pallet as XrplBridge;

pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

benchmarks! {
	submit_transaction {
		let relayer = account::<T>("Relayer");
		let ledger_index = 0;
		let transaction_hash: XrplTxHash = [0u8; 64].into();
		let transaction = XrplTxData::Xls20;
		let timestamp = 100;

		assert_ok!(XrplBridge::<T>::add_relayer(RawOrigin::Root.into(), relayer.clone()));

	}: _(origin::<T>(&relayer), ledger_index, transaction_hash.clone(), transaction.clone(), timestamp)
	verify {
		let val = XrpTransaction { transaction_hash, transaction, timestamp };
		let details = ProcessXRPTransactionDetails::<T>::get(transaction_hash);
		assert_eq!(details, Some((ledger_index, val, relayer)))
	}

	submit_challenge {
		let challenger = account::<T>("Challenger");
		let transaction_hash: XrplTxHash = [0u8; 64].into();

		// Sanity check
		assert!(ChallengeXRPTransactionList::<T>::get(transaction_hash).is_none());

	}: _(origin::<T>(&challenger), transaction_hash.clone())
	verify {
		let transaction_list = ChallengeXRPTransactionList::<T>::get(transaction_hash);
		assert_eq!(transaction_list, Some(challenger))
	}

	withdraw_xrp {
		let alice = account::<T>("Alice");
		let amount: Balance = 100u32.into();
		let destination: XrplAccountId = [0u8; 20].into();
		let door_address: XrplAccountId  = [1u8; 20].into();
		let alice_balance = amount + 1000000000;
		let asset_id = T::XrpAssetId::get();

		assert_ok!(XrplBridge::<T>::set_door_address(RawOrigin::Root.into(), door_address));
		assert_ok!(T::MultiCurrency::mint_into(
			asset_id,
			&alice.clone().into(),
			alice_balance,
		));
		assert_ok!(XrplBridge::<T>::add_relayer(RawOrigin::Root.into(), alice.clone()));
		assert_ok!(XrplBridge::<T>::set_ticket_sequence_next_allocation(origin::<T>(&alice).into(), 1, 1));

	}: _(origin::<T>(&alice), amount, destination)
	verify {
		let new_alice_balance = T::MultiCurrency::balance(asset_id, &alice);
		assert_ne!(alice_balance, new_alice_balance);
	}

	add_relayer {
		let relayer = account::<T>("Alice");

		// Sanity check
		let is_relayer = Relayer::<T>::get(relayer.clone());
		assert_eq!(is_relayer, None);

	}: _(RawOrigin::Root, relayer.clone())
	verify {
		let is_relayer = Relayer::<T>::get(relayer);
		assert_eq!(is_relayer, Some(true));
	}

	remove_relayer {
		let relayer = account::<T>("Alice");

		assert_ok!(XrplBridge::<T>::add_relayer(RawOrigin::Root.into(), relayer.clone()));
	}: _(RawOrigin::Root, relayer.clone())
	verify {
		let is_relayer = Relayer::<T>::get(relayer);
		assert_eq!(is_relayer, None);
	}

	set_door_tx_fee {
		let tx_fee = 100;
		// Sanity check
		assert_ne!(DoorTxFee::<T>::get(), tx_fee);

	}: _(RawOrigin::Root, tx_fee)
	verify {
		assert_eq!(DoorTxFee::<T>::get(), tx_fee);
	}

	set_door_address {
		let door_address: XrplAccountId = [1u8; 20].into();
		// Sanity check
		assert_ne!(DoorAddress::<T>::get(), Some(door_address));

	}: _(RawOrigin::Root, door_address)
	verify {
		assert_eq!(DoorAddress::<T>::get(), Some(door_address));
	}

	set_ticket_sequence_next_allocation {
		let alice = account::<T>("Alice");
		let start_sequence = 1;
		let bucket_size = 1;
		let expected_param = XrplTicketSequenceParams { start_sequence, bucket_size };

		assert_ok!(XrplBridge::<T>::add_relayer(RawOrigin::Root.into(), alice.clone()));

	}: _(origin::<T>(&alice), start_sequence, bucket_size)
	verify {
		let actual_param = DoorTicketSequenceParamsNext::<T>::get();
		assert_eq!(actual_param, expected_param);
	}

	set_ticket_sequence_current_allocation {
		let alice = account::<T>("Alice");
		let ticket_sequence = 1;
		let start_sequence = 1;
		let bucket_size = 1;
		let expected_param = XrplTicketSequenceParams { start_sequence, bucket_size };

		assert_ok!(XrplBridge::<T>::add_relayer(RawOrigin::Root.into(), alice.clone()));
		assert_ok!(XrplBridge::<T>::set_ticket_sequence_next_allocation(origin::<T>(&alice).into(), start_sequence, bucket_size));

	}: _(RawOrigin::Root, ticket_sequence, start_sequence, bucket_size)
	verify {
		let actual_param = DoorTicketSequenceParams::<T>::get();
		assert_eq!(actual_param, expected_param);
	}

	reset_settled_xrpl_tx_data {
		let i in 0..256;
		let alice = account::<T>("Alice");
		let mut settled_data = Vec::default();

		for j in 0..i {
			let ledger_index = j;
			let transaction_hash: XrplTxHash = [j as u8; 64].into();
			let timestamp = 100;
			let transaction = XrpTransaction { transaction_hash, transaction: XrplTxData::Xls20, timestamp } ;
			settled_data.push((transaction_hash, ledger_index, transaction, alice.clone()));
		}

		// Submission window related data
		let highest_settled_ledger_index = i + 1;
		let submission_window_width = i;
		let highest_pruned_ledger_index = Some(0);

	}: _(RawOrigin::Root, highest_settled_ledger_index, submission_window_width, highest_pruned_ledger_index, Some(settled_data) )
	verify {
		assert_eq!(HighestSettledLedgerIndex::<T>::get(), highest_settled_ledger_index);
		assert_eq!(HighestPrunedLedgerIndex::<T>::get(), 0);
		assert_eq!(SubmissionWindowWidth::<T>::get(), submission_window_width);

		//check all the settled tx details are added to the storage
		for j in 0..i {
			assert_eq!(ProcessXRPTransactionDetails::<T>::get(XrplTxHash::from([j as u8; 64])).is_some(), true);
			assert_eq!(SettledXRPTransactionDetails::<T>::get(j).is_some(), true);
		}
	}

	prune_settled_ledger_index {
		// The amount of transactions stored under the ledger index
		let i in 0..10;
		let alice = account::<T>("Alice");

		HighestSettledLedgerIndex::<T>::put(100);
		SubmissionWindowWidth::<T>::put(10);
		let ledger_index = i;

		let mut settled_data = Vec::default();
		for tx in 0..i {
			let tx_hash = XrplTxHash::from_low_u64_be(tx as u64);
			settled_data.push(tx_hash);
			<ProcessXRPTransactionDetails<T>>::insert(
				tx_hash,
				(ledger_index as LedgerIndex, XrpTransaction::default(), alice.clone()),
			);
		}
		<SettledXRPTransactionDetails<T>>::insert(ledger_index, BoundedVec::truncate_from(settled_data));
	}: _(RawOrigin::Root, ledger_index )
	verify {
		assert!(<SettledXRPTransactionDetails<T>>::get(ledger_index).is_none());

		//check all the settled tx details are added to the storage
		for tx in 0..i {
			let tx_hash = XrplTxHash::from_low_u64_be(tx as u64);
			assert!(ProcessXRPTransactionDetails::<T>::get(tx_hash).is_none());
		}
	}
}

impl_benchmark_test_suite!(XrplBridge, crate::mock::new_test_ext_benchmark(), crate::mock::Test);
