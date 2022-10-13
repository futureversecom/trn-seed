use super::*;
use crate::xrpl_types::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;
use seed_primitives::{validator::crypto::AuthorityId, xrpl::XrplTxData, Balance};
use sp_core::{
	offchain::{
		testing::{TestOffchainExt, TestTransactionPoolExt},
		OffchainDbExt, OffchainWorkerExt, TransactionPoolExt,
	},
	ByteArray,
};
use sp_runtime::{testing::UintAuthorityId, traits::BadOrigin};

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
	let tx_hash = XrplTxHash::from_slice(transaction_hash);
	let tx_hash_1 = XrplTxHash::from_slice(transaction_hash_1);
	let relayer = create_account(b"6490B68F1116BFE87DDD");
	XRPLBridge::initialize_relayer(&vec![relayer]);
	submit_transaction(relayer, 1_000_000, tx_hash, account_address, 1);
	submit_transaction(relayer, 1_000_000, tx_hash_1, account_address, 1);

	XRPLBridge::on_initialize(XrpTxChallengePeriod::get() as u64);
	System::set_block_number(XrpTxChallengePeriod::get() as u64);

	let xrp_balance = xrp_balance_of(account_address);
	assert_eq!(xrp_balance, 2000);
}

fn submit_transaction(
	relayer: AccountId,
	ledger_index: u64,
	tx_hash: XrplTxHash,
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
		tx_hash,
		transaction,
		1234
	));
}

fn valid_transaction_entry_request_notarization(
	tx_hash: XrplTxHash,
	mock_notary_keys: Vec<<Test as Config>::ValidatorId>,
	call_id: ChainCallId,
) {
	// `notarizations[i]` is submitted by the i-th validator (`mock_notary_keys`)
	let notarizations = vec![
		CheckedChainCallResult::Ok(tx_hash),
		CheckedChainCallResult::Ok(tx_hash),
		CheckedChainCallResult::Ok(tx_hash),
		CheckedChainCallResult::Ok(tx_hash),
		CheckedChainCallResult::Ok(tx_hash),
		CheckedChainCallResult::Ok(tx_hash),
		CheckedChainCallResult::Ok(tx_hash),
		CheckedChainCallResult::Ok(tx_hash),
		CheckedChainCallResult::Ok(tx_hash),
	];
	// expected aggregated count after the i-th notarization
	let expected_aggregations = vec![
		Some(1_u32),
		Some(2),
		Some(3), // block # differs, count separately
		Some(4),
		Some(5), // timestamp differs, count separately
		None,
		None,
		None, // return_data differs, count separately
		None, // success callback & storage is reset after 6th notarization (2/3 * 9 = 6)
	];

	// aggregate the notarizations
	let mut i = 1;
	for ((notary_result, notary_pk), aggregation) in
		notarizations.iter().zip(mock_notary_keys).zip(expected_aggregations)
	{
		if i >= 7 && i <= 9 {
			assert_noop!(
				DefaultValidatorSet::handle_call_notarization(call_id, *notary_result, &notary_pk),
				Error::<Test>::InvalidClaim
			);
		} else {
			assert_ok!(DefaultValidatorSet::handle_call_notarization(
				call_id,
				*notary_result,
				&notary_pk
			));
		}
		i += 1;

		// assert notarization progress
		let aggregated_notarizations =
			DefaultValidatorSet::chain_call_notarizations_aggregated(call_id).unwrap_or_default();
		println!("{:?}", aggregated_notarizations);
		assert_eq!(aggregated_notarizations.get(&notary_result).map(|x| *x), aggregation);
	}
}

fn invalid_transaction_entry_request_notarization(
	tx_hash: XrplTxHash,
	mock_notary_keys: Vec<<Test as Config>::ValidatorId>,
	call_id: ChainCallId,
) {
	// `notarizations[i]` is submitted by the i-th validator (`mock_notary_keys`)
	let notarizations = vec![
		CheckedChainCallResult::Ok(tx_hash),
		CheckedChainCallResult::NotOk(tx_hash),
		CheckedChainCallResult::NotOk(tx_hash),
		CheckedChainCallResult::NotOk(tx_hash),
		CheckedChainCallResult::NotOk(tx_hash),
		CheckedChainCallResult::Ok(tx_hash),
		CheckedChainCallResult::NotOk(tx_hash),
		CheckedChainCallResult::Ok(tx_hash),
		CheckedChainCallResult::NotOk(tx_hash),
	];
	// expected aggregated count after the i-th notarization
	let expected_aggregations = vec![
		Some(1_u32),
		Some(1_u32),
		Some(2_u32), // block # differs, count separately
		Some(3_u32),
		Some(4_u32), // timestamp differs, count separately
		Some(2_u32),
		Some(5_u32),
		Some(3_u32), // return_data differs, count separately
		None,        // success callback & storage is reset after 6th notarization (2/3 * 9 = 6)
	];

	// aggregate the notarizations
	for ((notary_result, notary_pk), aggregation) in
		notarizations.iter().zip(mock_notary_keys).zip(expected_aggregations)
	{
		assert_ok!(DefaultValidatorSet::handle_call_notarization(
			call_id,
			*notary_result,
			&notary_pk
		));

		// assert notarization progress
		let aggregated_notarizations =
			DefaultValidatorSet::chain_call_notarizations_aggregated(call_id).unwrap_or_default();
		println!("{:?}", aggregated_notarizations);
		assert_eq!(aggregated_notarizations.get(&notary_result).map(|x| *x), aggregation);
	}
}

#[test]
fn challenge_fails_process_transaction() {
	let mut ext = new_test_ext();
	let (offchain, _state) = TestOffchainExt::new();
	let (pool, state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));
	ext.execute_with(|| {
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let tx_hash = XrplTxHash::from_slice(transaction_hash);
		let account_address = b"6490B68F1116BFE87DDC";
		let relayer = create_account(b"6490B68F1116BFE87DDD");
		let challenger = create_account(b"6490B68F1116BFE87DDE");
		XRPLBridge::initialize_relayer(&vec![relayer]);
		submit_transaction(relayer, 1_000_000, tx_hash, account_address, 1);
		assert_ok!(XRPLBridge::submit_challenge(Origin::signed(challenger), 1_000_000, tx_hash,));
		XRPLBridge::on_initialize(XrpTxChallengePeriod::get() as u64);
		System::set_block_number(XrpTxChallengePeriod::get() as u64);

		let xrp_balance = xrp_balance_of(account_address);
		assert_eq!(xrp_balance, 0);

		let block = 1;
		System::set_block_number(block);
		let keys = init_keys();
		Session::rotate_session();
		DefaultValidatorSet::on_initialize(block);
		DefaultValidatorSet::offchain_worker(block);
		let call_id = 0_u64;
		valid_transaction_entry_request_notarization(
			XrplTxHash::from_slice(transaction_hash),
			keys,
			call_id,
		);

		XRPLBridge::on_initialize(XrpTxChallengePeriod::get() as u64 + 1);
		System::set_block_number(XrpTxChallengePeriod::get() as u64 + 1);

		let xrp_balance = xrp_balance_of(account_address);
		assert_eq!(
			XRPLBridge::process_xrp_transaction(XrpTxChallengePeriod::get() as u64 + 1),
			None
		);
		assert_eq!(xrp_balance, 2000);
	})
}

#[test]
fn challenge_success_do_not_process_transaction() {
	let mut ext = new_test_ext();
	let (offchain, _state) = TestOffchainExt::new();
	let (pool, state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));
	ext.execute_with(|| {
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let tx_hash = XrplTxHash::from_slice(transaction_hash);
		let account_address = b"6490B68F1116BFE87DDC";
		let relayer = create_account(b"6490B68F1116BFE87DDD");
		let challenger = create_account(b"6490B68F1116BFE87DDE");
		XRPLBridge::initialize_relayer(&vec![relayer]);
		submit_transaction(relayer, 1_000_000, tx_hash, account_address, 1);
		assert_ok!(XRPLBridge::submit_challenge(Origin::signed(challenger), 1_000_000, tx_hash,));
		XRPLBridge::on_initialize(XrpTxChallengePeriod::get() as u64);
		System::set_block_number(XrpTxChallengePeriod::get() as u64);

		let xrp_balance = xrp_balance_of(account_address);
		assert_eq!(xrp_balance, 0);

		let block = 1;
		System::set_block_number(block);
		let keys = init_keys();
		Session::rotate_session();
		DefaultValidatorSet::on_initialize(block);
		DefaultValidatorSet::offchain_worker(block);
		let call_id = 0_u64;
		invalid_transaction_entry_request_notarization(
			XrplTxHash::from_slice(transaction_hash),
			keys,
			call_id,
		);

		XRPLBridge::on_initialize(XrpTxChallengePeriod::get() as u64 + 1);
		System::set_block_number(XrpTxChallengePeriod::get() as u64 + 1);

		let xrp_balance = xrp_balance_of(account_address);
		assert_eq!(
			XRPLBridge::process_xrp_transaction(XrpTxChallengePeriod::get() as u64 + 1),
			None
		);
		assert_eq!(xrp_balance, 0);
	})
}
