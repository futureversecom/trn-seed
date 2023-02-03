use super::*;
use crate::mock::{
	Call as RuntimeCall, new_test_ext, AssetsExt, Origin, System, Test, XRPLBridge, XrpAssetId, XrpTxChallengePeriod, build_offchainify
};
use frame_support::{assert_err, assert_noop, assert_ok};
use frame_system::RawOrigin;
use seed_primitives::{AccountId, Balance};
use sp_core::{H160, H256, H512, offchain::testing::PendingRequest};
use sp_runtime::{traits::BadOrigin, testing::TestXt};

type Extrinsic = TestXt<RuntimeCall, ()>;

/// Helper function to create an AccountId from  a slice
fn create_account(address: &[u8]) -> AccountId {
	AccountId::from(H160::from_slice(address))
}

/// Helper function to get the xrp balance of an address slice
fn xrp_balance_of(address: &[u8]) -> u64 {
	AssetsExt::balance(XrpAssetId::get(), &H160::from_slice(address).into()) as u64
}

fn process_transaction(account_address: &[u8; 20]) {
	let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
	let transaction_hash_1 = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317C";
	let relayer = create_account(b"6490B68F1116BFE87DDD");
	XRPLBridge::initialize_relayer(&vec![relayer]);
	submit_transaction(relayer, 1_000_000, transaction_hash, account_address, 1);
	submit_transaction(relayer, 1_000_000, transaction_hash_1, account_address, 1);

	XRPLBridge::on_initialize(XrpTxChallengePeriod::get() as u64);
	System::set_block_number(XrpTxChallengePeriod::get() as u64);

	let xrp_balance = xrp_balance_of(account_address);
	assert_eq!(xrp_balance, 2000);
}

fn submit_transaction(
	relayer: AccountId,
	ledger_index: u64,
	transaction_hash: &[u8; 64],
	account_address: &[u8; 20],
	i: u64,
) {
	let transaction = XrplTxData::Payment {
		amount: (i * 1000u64) as Balance,
		address: H160::from_slice(account_address),
	};
	assert_ok!(XRPLBridge::submit_transaction(
		Origin::signed(relayer),
		ledger_index,
		XrplTxHash::from_slice(transaction_hash),
		transaction,
		1234
	));
}

struct PendingRequestBuilder(PendingRequest);

impl PendingRequestBuilder {
	fn new(uri: &str) -> Self {
		Self {
			0: PendingRequest {
				uri: uri.to_string(),
				sent: true,
				..Default::default()
			},
		}
	}
	fn request(mut self, request: &[u8]) -> Self {
		self.0.body = request.to_vec();
		self.0.headers = vec![
			("Content-Type".to_string(), "application/json".to_string()),
		];
		self
	}
	fn method(mut self, method: &str) -> Self {
		self.0.method = method.into();
		self
	}
	fn response(mut self, response: &[u8]) -> Self {
		self.0.response = Some(response.to_vec());
		self
	}
	fn build(self) -> PendingRequest {
		self.0
	}
}

#[test]
fn submit_transaction_replay() {
	new_test_ext().execute_with(|| {
		let relayer = create_account(b"6490B68F1116BFE87DDD");
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let transaction =
			XrplTxData::Payment { amount: 1000 as Balance, address: H160::from_low_u64_be(555) };
		assert_ok!(XRPLBridge::add_relayer(Origin::root(), relayer));
		assert_ok!(XRPLBridge::submit_transaction(
			Origin::signed(relayer),
			1,
			XrplTxHash::from_slice(transaction_hash),
			transaction.clone(),
			1234
		));
		assert_noop!(
			XRPLBridge::submit_transaction(
				Origin::signed(relayer),
				1,
				XrplTxHash::from_slice(transaction_hash),
				transaction,
				1234
			),
			Error::<Test>::TxReplay
		);
	});
}

#[test]
fn add_transaction_works() {
	new_test_ext().execute_with(|| {
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let tx_address = b"6490B68F1116BFE87DDD";
		let relayer = create_account(b"6490B68F1116BFE87DDD");
		XRPLBridge::initialize_relayer(&vec![relayer]);
		for i in 0..9u64 {
			let mut transaction_hash = transaction_hash.clone();
			transaction_hash[0] = i as u8;
			submit_transaction(relayer, i * 1_000_000, &transaction_hash, tx_address, i);
		}
	})
}

#[test]
fn process_transaction_works() {
	new_test_ext().execute_with(|| {
		let account_address = b"6490B68F1116BFE87DDC";
		process_transaction(account_address);
	})
}

#[test]
fn process_transaction_challenge_works() {
	new_test_ext().execute_with(|| {
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let tx_address = b"6490B68F1116BFE87DDC";
		let relayer = create_account(b"6490B68F1116BFE87DDD");
		let challenger = create_account(b"6490B68F1116BFE87DDE");
		let ledger_index = 1_000_000;

		XRPLBridge::initialize_relayer(&vec![relayer]);
		submit_transaction(relayer, ledger_index, transaction_hash, tx_address, 1);

		assert_ok!(XRPLBridge::submit_challenge(
			Origin::signed(challenger),
			XrplTxHash::from_slice(transaction_hash),
			ledger_index
		));

		XRPLBridge::on_initialize(XrpTxChallengePeriod::get() as u64);
		System::set_block_number(XrpTxChallengePeriod::get() as u64);

		let xrp_balance = xrp_balance_of(tx_address);
		assert_eq!(xrp_balance, 0);
	})
}

#[test]
fn process_transaction_challenge_offchain_worker() {
	let ext = new_test_ext();
	let (mut ext, pool_state, offchain_state) = build_offchainify(ext);
	{
		let mut offchain_state = offchain_state.write();
		let expected_request = br#"{"method":"transaction_entry","params":[{"ledger_index":72014720,"tx_hash":"CAECA8C9DE80AE296D260FD86A4233D38E9DE9E749AFE4967BCE41533443B114"}]}"#;
		let expected_response = br#"{"result":{"ledger_hash":"47268554D5076134ABB28F0C0917543850B42E65EE0798B0D492789BC9BD26E7","ledger_index":72014720,"metadata":{"AffectedNodes":[{"ModifiedNode":{"FinalFields":{"Balance":{"currency":"OCW","issuer":"rrrrrrrrrrrrrrrrrrrrBZbvji","value":"0.03276467"},"Flags":1114112,"HighLimit":{"currency":"OCW","issuer":"rK9DrarGKnVEo2nYp5MfVRXRYf5yRX3mwD","value":"0"},"HighNode":"26b","LowLimit":{"currency":"OCW","issuer":"rscF9kdWeQEgsZo4fbcX397gtWgrJvPtgy","value":"36000"},"LowNode":"0"},"LedgerEntryType":"RippleState","LedgerIndex":"7141E08A5D459A2CA5D13B35E32C1DE978B7C21D08AF694E51EFC73E83D38568","PreviousFields":{"Balance":{"currency":"OCW","issuer":"rrrrrrrrrrrrrrrrrrrrBZbvji","value":"0.03174188"}},"PreviousTxnID":"CA21645DF08EE730FC567C483DCADBD768E95F5F0656DDBA3E88D22325A6D63D","PreviousTxnLgrSeq":71993242}},{"ModifiedNode":{"FinalFields":{"Balance":{"currency":"OCW","issuer":"rrrrrrrrrrrrrrrrrrrrBZbvji","value":"4.59112055"},"Flags":1114112,"HighLimit":{"currency":"OCW","issuer":"rK9DrarGKnVEo2nYp5MfVRXRYf5yRX3mwD","value":"0"},"HighNode":"a49","LowLimit":{"currency":"OCW","issuer":"rsTAYkk7VQfBdD5btt2WzXYphER6F2BTuN","value":"1000000000000000e-3"},"LowNode":"0"},"LedgerEntryType":"RippleState","LedgerIndex":"B928AE6CB874AB69F6546A45C123071F5F8CFC4B1E8E1FE9EFD8F2CE3E14854B","PreviousFields":{"Balance":{"currency":"OCW","issuer":"rrrrrrrrrrrrrrrrrrrrBZbvji","value":"4.59214334"}},"PreviousTxnID":"C82D9B05ADED5D3A776CE576F495B254351A36BFD69D51EEDAFD4A826BFB6EF9","PreviousTxnLgrSeq":72014719}},{"ModifiedNode":{"FinalFields":{"Account":"rsTAYkk7VQfBdD5btt2WzXYphER6F2BTuN","Balance":"98324250","Flags":0,"OwnerCount":12,"Sequence":71033319},"LedgerEntryType":"AccountRoot","LedgerIndex":"DFD9FDC5BB6CD27225B22BC3EAEDF070946074F20B421E49118133A5BB9D0644","PreviousFields":{"Balance":"98324270","Sequence":71033318},"PreviousTxnID":"48A7606F614DF47D66CBC5EACC24E5738786290B21D440ED3781EB4D06B18EFF","PreviousTxnLgrSeq":72014719}}],"TransactionIndex":0,"TransactionResult":"tesSUCCESS"},"status":"success","tx_json":{"Account":"rsTAYkk7VQfBdD5btt2WzXYphER6F2BTuN","Amount":{"currency":"OCW","issuer":"rK9DrarGKnVEo2nYp5MfVRXRYf5yRX3mwD","value":"0.00102279"},"Destination":"rscF9kdWeQEgsZo4fbcX397gtWgrJvPtgy","DestinationTag":1,"Fee":"20","Flags":131072,"LastLedgerSequence":72015917,"Memos":[{"Memo":{"MemoData":"537461796B696E6720526577617264733A200A302E3030312066726F6D204461696C7920535458202D3E204F43572C"}}],"Sequence":71033318,"SigningPubKey":"ED72F2BC78ECC506BFE6211AD67CE836D0CA0A850F80769C1D3BBBD3659F38577E","TransactionType":"Payment","TxnSignature":"B300A6E4AE62C2EA8487F211228C5B03117E42E647B03C06340CD01786124B3DE0EBD49D3ADA06FA53EAEC8885EFFA17A5733BB99DE9CED342E803F71821EA04","hash":"CAECA8C9DE80AE296D260FD86A4233D38E9DE9E749AFE4967BCE41533443B114"},"validated":true,"warnings":[{"id":1004,"message":"This is a reporting server.  The default behavior of a reporting server is to only return validated data. If you are looking for not yet validated data, include \"ledger_index : current\" in your request, which will cause this server to forward the request to a p2p node. If the forward is successful the response will include \"forwarded\" : \"true\""}]}}"#;

		let expected_request_response = PendingRequestBuilder::new("https://s1.ripple.com:51234/")
			.method("POST")
			.request(expected_request)
			.response(expected_response)
			.build();
			offchain_state.expect_request(expected_request_response);
	}

	ext.execute_with(|| {
		let transaction_hash = H512::from_slice(b"CAECA8C9DE80AE296D260FD86A4233D38E9DE9E749AFE4967BCE41533443B114");
		let ledger_index = 72014720;

		let relayer = create_account(b"6490B68F1116BFE87DDD");
		let challenger = create_account(b"6490B68F1116BFE87DDE");

		XRPLBridge::initialize_relayer(&vec![relayer]);

		ChallengeXRPTransactionList::<Test>::insert((&transaction_hash, ledger_index), challenger);

		<Pallet<Test> as Hooks<<Test as frame_system::Config>::BlockNumber>>::offchain_worker(
			XrpTxChallengePeriod::get() as u64
		);

		let tx = pool_state.write().transactions.pop().unwrap();
		let ext = Extrinsic::decode(&mut &*tx).unwrap();

		// Offchain worker has submitted the challenge verification ext to the pool
		assert!(matches!(ext.call, RuntimeCall::XRPLBridge(Call::receive_offchain_challenge_verification { .. } )));
	})
}

#[test]
fn set_door_tx_fee_works() {
	new_test_ext().execute_with(|| {
		let new_fee = 123456_u64;
		assert_ok!(XRPLBridge::set_door_tx_fee(frame_system::RawOrigin::Root.into(), new_fee));
		assert_eq!(XRPLBridge::door_tx_fee(), new_fee);

		// Only root can sign this tx, this should fail
		let account = AccountId::from(H160::from_slice(b"6490B68F1116BFE87DDC"));
		assert_noop!(
			XRPLBridge::set_door_tx_fee(Origin::signed(account), 0),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn withdraw_request_works() {
	new_test_ext().execute_with(|| {
		// For this test we will set the door_tx_fee to 0
		assert_ok!(XRPLBridge::set_door_tx_fee(frame_system::RawOrigin::Root.into(), 0_u64));

		let door = XrplAccountId::from_slice(b"5490B68F2d16B3E87cba");
		let destination = XrplAccountId::from_slice(b"6490B68F1116BFE87DDD");
		let account_address = b"6490B68F1116BFE87DDC";
		let account = create_account(account_address);
		process_transaction(account_address); // 2000 XRP deposited

		// set initial ticket sequence params
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			Origin::root(),
			1_u32,
			1_u32,
			200_u32
		));

		// door address unset
		assert_noop!(
			XRPLBridge::withdraw_xrp(Origin::signed(account), 1000, destination),
			Error::<Test>::DoorAddressNotSet
		);
		assert_ok!(XRPLBridge::set_door_address(Origin::root(), door));

		// Withdraw half of available xrp
		assert_ok!(XRPLBridge::withdraw_xrp(Origin::signed(account), 1000, destination));
		let xrp_balance = xrp_balance_of(account_address);
		assert_eq!(xrp_balance, 1000);

		// Withdraw second half
		assert_ok!(XRPLBridge::withdraw_xrp(Origin::signed(account), 1000, destination));
		let xrp_balance = xrp_balance_of(account_address);
		assert_eq!(xrp_balance, 0);

		// No xrp left to withdraw, should fail
		assert_noop!(
			XRPLBridge::withdraw_xrp(Origin::signed(account), 1, destination),
			ArithmeticError::Underflow
		);
	})
}

#[test]
fn withdraw_request_works_with_door_fee() {
	new_test_ext().execute_with(|| {
		// For this test we will set the door_tx_fee to 100
		let door_tx_fee = 100_u64;
		assert_ok!(XRPLBridge::set_door_tx_fee(frame_system::RawOrigin::Root.into(), door_tx_fee));
		let account_address = b"6490B68F1116BFE87DDC";
		let account = create_account(account_address);
		process_transaction(account_address); // 2000 XRP deposited
		let destination = XrplAccountId::from_slice(b"6490B68F1116BFE87DDD");
		let initial_xrp_balance = xrp_balance_of(account_address);
		let withdraw_amount: u64 = 1_000;

		// set initial ticket sequence params
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			Origin::root(),
			1_u32,
			1_u32,
			200_u32
		));
		// set door address
		assert_ok!(XRPLBridge::set_door_address(Origin::root(), b"6490B68F1116BFE87DDC".into()));

		assert_ok!(XRPLBridge::withdraw_xrp(
			Origin::signed(account),
			withdraw_amount.into(),
			destination
		));

		// Balance should be less withdraw amount and door fee
		let xrp_balance = xrp_balance_of(account_address);
		assert_eq!(xrp_balance, initial_xrp_balance - withdraw_amount - door_tx_fee);

		// Try again for remainding
		let initial_xrp_balance = xrp_balance_of(account_address);
		let withdraw_amount: u64 = 800;
		assert_ok!(XRPLBridge::withdraw_xrp(
			Origin::signed(account),
			withdraw_amount.into(),
			destination
		));

		// Balance should be less withdraw amount and door fee
		let xrp_balance = xrp_balance_of(account_address);
		assert_eq!(xrp_balance, initial_xrp_balance - withdraw_amount - door_tx_fee);

		// No funds left to withdraw
		assert_eq!(xrp_balance, 0);
		assert_noop!(
			XRPLBridge::withdraw_xrp(Origin::signed(account), 1, destination),
			ArithmeticError::Underflow
		);
	})
}

#[test]
fn withdraw_request_burn_fails() {
	new_test_ext().execute_with(|| {
		// For this test we will set the door_tx_fee to 0 so we can ensure the Underflow is due to
		// the withdraw logic, not the door_tx_fee
		assert_ok!(XRPLBridge::set_door_tx_fee(frame_system::RawOrigin::Root.into(), 0_u64));
		assert_ok!(XRPLBridge::set_door_address(Origin::root(), b"6490B68F1116BFE87DDC".into()));

		let account = create_account(b"6490B68F1116BFE87DDC");
		let destination = XrplAccountId::from_slice(b"6490B68F1116BFE87DDD");
		assert_noop!(
			XRPLBridge::withdraw_xrp(Origin::signed(account), 1000, destination),
			ArithmeticError::Underflow
		);
	})
}

#[test]
fn set_door_address_success() {
	new_test_ext().execute_with(|| {
		let xprl_door_address = b"6490B68F1116BFE87DDD";
		assert_ok!(XRPLBridge::set_door_address(Origin::root(), H160::from(xprl_door_address)));
		assert_eq!(XRPLBridge::door_address(), Some(H160::from_slice(xprl_door_address)));
	})
}

#[test]
fn set_door_address_fail() {
	new_test_ext().execute_with(|| {
		let xprl_door_address = b"6490B68F1116BFE87DDD";
		let caller = XrplAccountId::from_low_u64_be(1);
		assert_noop!(
			XRPLBridge::set_door_address(
				Origin::signed(AccountId::from(caller)),
				H160::from(xprl_door_address)
			),
			BadOrigin
		);
		assert_eq!(XRPLBridge::door_address(), None);
	})
}

#[test]
fn clear_storages() {
	new_test_ext().execute_with(|| {
		let process_block = 5;
		let tx_hash_1 = XrplTxHash::from_low_u64_be(123);
		let tx_hash_2 = XrplTxHash::from_low_u64_be(123);
		let ledger_index:LedgerIndex = 2;

		<SettledXRPTransactionDetails<Test>>::append(process_block, (tx_hash_1, ledger_index));
		<SettledXRPTransactionDetails<Test>>::append(process_block, (tx_hash_2, ledger_index));

		let account: AccountId = [1_u8; 20].into();
		<ProcessXRPTransactionDetails<Test>>::insert(
			(tx_hash_1, ledger_index),
			(XrpTransaction::default(), account),
		);
		<ProcessXRPTransactionDetails<Test>>::insert(
			(tx_hash_2, ledger_index),
			(XrpTransaction::default(), account),
		);

		XRPLBridge::on_initialize(process_block);

		assert!(<SettledXRPTransactionDetails<Test>>::get(process_block).is_none());
		assert!(<ProcessXRPTransactionDetails<Test>>::get((tx_hash_1, ledger_index)).is_none());
		assert!(<ProcessXRPTransactionDetails<Test>>::get((tx_hash_2, ledger_index)).is_none());
	});
}

#[test]
fn get_door_ticket_sequence_success_at_start() {
	new_test_ext().execute_with(|| {
		// set initial ticket sequence params
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			Origin::root(),
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
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let relayer = create_account(b"6490B68F1116BFE87DDD");
		XRPLBridge::initialize_relayer(&vec![relayer]);

		assert_noop!(
			XRPLBridge::get_door_ticket_sequence(),
			Error::<Test>::NextTicketSequenceParamsNotSet
		);

		// set the params for next round
		assert_ok!(XRPLBridge::set_ticket_sequence_next_allocation(
			Origin::signed(relayer),
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
	new_test_ext().execute_with(|| {
		let relayer = create_account(b"6490B68F1116BFE87DDD");
		XRPLBridge::initialize_relayer(&vec![relayer]);

		// set initial ticket sequence params
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			Origin::root(),
			1_u32,
			1_u32,
			2_u32
		));

		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(1));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(2));
		// need to set the next ticket params on or before the last of current
		assert_ok!(XRPLBridge::set_ticket_sequence_next_allocation(
			Origin::signed(relayer),
			3_u32, // start ticket sequence next round
			2_u32, // ticket sequence bucket size next round
		));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(3));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(4));
		assert_ok!(XRPLBridge::set_ticket_sequence_next_allocation(
			Origin::signed(relayer),
			10_u32, // start ticket sequence next round
			10_u32, // ticket sequence bucket size next round
		));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(10));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(11));
	})
}

#[test]
fn get_door_ticket_sequence_success_force_set_current_round() {
	new_test_ext().execute_with(|| {
		let relayer = create_account(b"6490B68F1116BFE87DDD");
		XRPLBridge::initialize_relayer(&vec![relayer]);

		// set initial ticket sequence params
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			Origin::root(),
			1_u32,
			1_u32,
			10_u32
		));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(1));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(2));

		// force set current values to (current=5 start=5, bucket_size=2)
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			Origin::root(),
			5_u32,
			5_u32,
			3_u32
		));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(5));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(6));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(7));

		// need to set the next ticket params on or before the last of current
		assert_ok!(XRPLBridge::set_ticket_sequence_next_allocation(
			Origin::signed(relayer),
			11_u32, // start ticket sequence next round
			2_u32,  // ticket sequence bucket size next round
		));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(11));
	})
}

#[test]
#[allow(non_snake_case)]
fn get_door_ticket_sequence_check_events_emitted() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let relayer = create_account(b"6490B68F1116BFE87DDD");
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
			Origin::signed(relayer),
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
			Origin::signed(relayer),
			10_u32, // start ticket sequence next round
			5_u32,  // ticket sequence bucket size next round
		));
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(10));
	})
}

#[test]
fn set_ticket_sequence_current_allocation_success() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let relayer = create_account(b"6490B68F1116BFE87DDD");
		XRPLBridge::initialize_relayer(&vec![relayer]);

		// set initial ticket sequence params
		System::reset_events();
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			Origin::root(),
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
			Origin::root(),
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
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let relayer = create_account(b"6490B68F1116BFE87DDD");
		XRPLBridge::initialize_relayer(&vec![relayer]);

		// set initial ticket sequence params - success
		System::reset_events();
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			Origin::root(),
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
				Origin::root(),
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
				Origin::root(),
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
				Origin::root(),
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
				Origin::signed(relayer),
				10_u32,
				1_u32,
				200_u32
			),
			BadOrigin
		);

		// Force set the same valid params set, with root
		System::reset_events();
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			Origin::root(),
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
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let relayer = create_account(b"6490B68F1116BFE87DDD");
		XRPLBridge::initialize_relayer(&vec![relayer]);

		// no initial ticket sequence params, start setting the params for next allocation
		System::reset_events();
		assert_ok!(XRPLBridge::set_ticket_sequence_next_allocation(
			Origin::signed(relayer),
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
			Origin::root(),
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
			Origin::signed(relayer),
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
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let relayer = create_account(b"6490B68F1116BFE87DDD");
		XRPLBridge::initialize_relayer(&vec![relayer]);

		// set initial ticket sequence params - success
		System::reset_events();
		assert_ok!(XRPLBridge::set_ticket_sequence_current_allocation(
			Origin::root(),
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
			XRPLBridge::set_ticket_sequence_next_allocation(Origin::signed(relayer), 10_u32, 0_u32),
			Error::<Test>::NextTicketSequenceParamsInvalid
		);

		// try to fetch it, should give the next ticket sequence in current allocation
		assert_eq!(XRPLBridge::get_door_ticket_sequence(), Ok(2));

		// set the next param set with start_ticket_sequence < current ticket_sequence
		System::reset_events();
		assert_noop!(
			XRPLBridge::set_ticket_sequence_next_allocation(
				Origin::signed(relayer),
				1_u32,
				200_u32
			),
			Error::<Test>::NextTicketSequenceParamsInvalid
		);

		// set the next param set with start_ticket_sequence < current start_ticket_sequence
		System::reset_events();
		assert_noop!(
			XRPLBridge::set_ticket_sequence_next_allocation(
				Origin::signed(relayer),
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
				Origin::signed(create_account(b"6490B68F1116BFE87DDE")),
				10_u32,
				200_u32
			),
			Error::<Test>::NotPermitted
		);

		// same valid params set, with relayer
		System::reset_events();
		assert_ok!(XRPLBridge::set_ticket_sequence_next_allocation(
			Origin::signed(relayer),
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
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let account_address = b"6490B68F1116BFE87DDC";
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let relayer = create_account(b"6490B68F1116BFE87DDD");
		XRPLBridge::initialize_relayer(&vec![relayer]);

		// submit payment tx
		let payment_tx = XrplTxData::Payment {
			amount: (1 * 1000u64) as Balance,
			address: H160::from_slice(account_address),
		};
		assert_ok!(XRPLBridge::submit_transaction(
			Origin::signed(relayer),
			1_000_000,
			XrplTxHash::from_slice(transaction_hash),
			payment_tx,
			1234
		));

		System::reset_events();
		XRPLBridge::process_xrp_tx(XrpTxChallengePeriod::get() as u64 + 1);
		System::set_block_number(XrpTxChallengePeriod::get() as u64 + 1);

		// System::assert_has_event(
		// 	Event::<Test>::ProcessingOk(1_000_000_u64, XrplTxHash::from_slice(transaction_hash))
		// 		.into(),
		// );

		let xrp_balance = xrp_balance_of(account_address);
		assert_eq!(xrp_balance, 1000);
	})
}

#[test]
fn process_xrp_tx_not_supported_transaction() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let account_address = b"6490B68F1116BFE87DDC";
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let relayer = create_account(b"6490B68F1116BFE87DDD");
		XRPLBridge::initialize_relayer(&vec![relayer]);

		// submit currency payment tx
		let currency_payment_tx = XrplTxData::CurrencyPayment {
			amount: (1 * 1000u64) as Balance,
			address: H160::from_slice(account_address),
			currency_id: H256::random(),
		};
		assert_ok!(XRPLBridge::submit_transaction(
			Origin::signed(relayer),
			1_000_000,
			XrplTxHash::from_slice(transaction_hash),
			currency_payment_tx,
			1234
		));

		System::reset_events();
		XRPLBridge::process_xrp_tx(XrpTxChallengePeriod::get() as u64 + 1);
		System::set_block_number(XrpTxChallengePeriod::get() as u64 + 1);
		System::assert_has_event(Event::<Test>::NotSupportedTransaction.into());

		let xrp_balance = xrp_balance_of(account_address);
		assert_eq!(xrp_balance, 0);
	})
}

#[test]
fn process_xrp_tx_processing_failed() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let account_address = b"6490B68F1116BFE87DDC";
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317C";
		let transaction_hash2 = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317D";
		let relayer = create_account(b"6490B68F1116BFE87DDD");
		XRPLBridge::initialize_relayer(&vec![relayer]);
		{
			// submit payment tx - this will mint the max Balance for the asset ID
			let payment_tx = XrplTxData::Payment {
				amount: u128::MAX as Balance,
				address: H160::from_slice(account_address),
			};
			assert_ok!(XRPLBridge::submit_transaction(
				Origin::signed(relayer),
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
				Origin::signed(relayer),
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
