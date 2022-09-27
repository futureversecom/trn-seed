use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;
use seed_primitives::{AccountId, Balance};
use sp_core::H160;
use sp_runtime::{traits::BadOrigin, SaturatedConversion};

#[test]
fn test_add_transaction_works() {
	new_test_ext().execute_with(|| {
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let tx_address = b"6490B68F1116BFE87DDD";
		let relayer_address = b"6490B68F1116BFE87DDD";
		let relayer = AccountId::from(H160::from_slice(relayer_address));
		XRPLBridge::initialize_relayer(&vec![relayer]);
		for i in 1..100u64 {
			submit_transaction(relayer, i * 1_000_000, transaction_hash, tx_address, i);
		}
	})
}

#[test]
fn test_process_transaction_works() {
	new_test_ext().execute_with(|| {
		let account_address = b"6490B68F1116BFE87DDC";
		process_transaction(account_address);
	})
}

#[test]
fn test_process_transaction_challenge_works() {
	new_test_ext().execute_with(|| {
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let tx_address = b"6490B68F1116BFE87DDC";
		let relayer_address = b"6490B68F1116BFE87DDD";
		let challenger_address = b"6490B68F1116BFE87DDE";
		let relayer = AccountId::from(H160::from_slice(relayer_address));
		let challenger = AccountId::from(H160::from_slice(challenger_address));
		XRPLBridge::initialize_relayer(&vec![relayer]);
		submit_transaction(relayer, 1_000_000, transaction_hash, tx_address, 1);
		assert_ok!(XRPLBridge::submit_challenge(
			Origin::signed(challenger),
			XrplTxHash::from_slice(transaction_hash),
		));
		XRPLBridge::on_initialize((10 * MINUTES).into()); // wait for 5 hours (3000 blocks) to process transaction
		System::set_block_number((10 * MINUTES).into());
		let xrp_balance =
			AssetsExt::balance(XrpAssetId::get(), &H160::from_slice(tx_address).into());
		assert_eq!(xrp_balance, 0);
	})
}

#[test]
fn test_withdraw_tx_id_inc_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(XRPLBridge::withdraw_tx_nonce_inc());
		let id = CurrentWithdrawTxNonce::<Test>::get().unwrap();
		assert_ok!(XRPLBridge::withdraw_tx_nonce_inc());
		assert_eq!(CurrentWithdrawTxNonce::<Test>::get().unwrap(), id + 1);
	});
}

#[test]
fn test_withdraw_request_works() {
	new_test_ext().execute_with(|| {
		let account_address = b"6490B68F1116BFE87DDC";
		let account = AccountId::from(H160::from_slice(account_address));
		process_transaction(account_address); // 2000 XRP deposited
		let destination_address = b"6490B68F1116BFE87DDD";
		let destination = XrplAddress::from_slice(destination_address);
		assert_ok!(XRPLBridge::withdraw_xrp(Origin::signed(account), 1000, destination));
		let xrp_balance =
			AssetsExt::balance(XrpAssetId::get(), &H160::from_slice(account_address).into());
		assert_eq!(xrp_balance, 1000);
		assert_ok!(XRPLBridge::withdraw_xrp(Origin::signed(account), 1000, destination));
		let xrp_balance =
			AssetsExt::balance(XrpAssetId::get(), &H160::from_slice(account_address).into());
		assert_eq!(xrp_balance, 0);
		assert_noop!(
			XRPLBridge::withdraw_xrp(Origin::signed(account), 1, destination),
			ArithmeticError::Underflow
		);
	})
}

#[test]
fn test_withdraw_request_burn_fails() {
	new_test_ext().execute_with(|| {
		let account_address = b"6490B68F1116BFE87DDC";
		let account = AccountId::from(H160::from_slice(account_address));
		let destination_address = b"6490B68F1116BFE87DDD";
		let destination = XrplAddress::from_slice(destination_address);
		assert_noop!(
			XRPLBridge::withdraw_xrp(Origin::signed(account), 1000, destination),
			ArithmeticError::Underflow
		);
	})
}

fn process_transaction(account_address: &[u8; 20]) {
	let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
	let transaction_hash_1 = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317C";
	let relayer_address = b"6490B68F1116BFE87DDD";
	let relayer = AccountId::from(H160::from_slice(relayer_address));
	XRPLBridge::initialize_relayer(&vec![relayer]);
	submit_transaction(relayer, 1_000_000, transaction_hash, account_address, 1);
	submit_transaction(relayer, 1_000_000, transaction_hash_1, account_address, 1);
	XRPLBridge::on_initialize((10 * MINUTES).into()); // wait for 5 hours (3000 blocks) to process transaction
	System::set_block_number((10 * MINUTES).into());
	let xrp_balance =
		AssetsExt::balance(XrpAssetId::get(), &H160::from_slice(account_address).into());
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

#[test]
fn test_set_xrpl_door_address_success() {
	new_test_ext().execute_with(|| {
		let xprl_door_address = b"6490B68F1116BFE87DDD";
		assert_ok!(XRPLBridge::set_xrpl_door_address(
			Origin::root(),
			H160::from(xprl_door_address)
		));
		assert_eq!(XRPLBridge::get_xrpl_door_address(), Some(H160::from_slice(xprl_door_address)));
	})
}

#[test]
fn test_set_xrpl_door_address_fail() {
	new_test_ext().execute_with(|| {
		let xprl_door_address = b"6490B68F1116BFE87DDD";
		let caller = XrplAddress::from_low_u64_be(1);
		assert_noop!(
			XRPLBridge::set_xrpl_door_address(
				Origin::signed(AccountId::from(caller)),
				H160::from(xprl_door_address)
			),
			BadOrigin
		);
		assert_eq!(XRPLBridge::get_xrpl_door_address(), None);
	})
}
