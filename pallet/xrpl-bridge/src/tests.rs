use super::*;
use frame_support::assert_ok;
use mock::*;
use seed_primitives::{AccountId, Balance};
use sp_core::H160;

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
		let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
		let transaction_hash_1 =
			b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317C";
		let tx_address = b"6490B68F1116BFE87DDC";
		let relayer_address = b"6490B68F1116BFE87DDD";
		let relayer = AccountId::from(H160::from_slice(relayer_address));
		XRPLBridge::initialize_relayer(&vec![relayer]);
		submit_transaction(relayer, 1_000_000, transaction_hash, tx_address, 1);
		submit_transaction(relayer, 1_000_000, transaction_hash_1, tx_address, 1);
		XRPLBridge::on_initialize(3_000); // wait for 5 hours (3000 blocks) to process transaction
		System::set_block_number(3_000);
		let xrp_balance =
			AssetsExt::balance(XrpAssetId::get(), &H160::from_slice(tx_address).into());
		assert_eq!(xrp_balance, 200);
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
		XRPLBridge::on_initialize(3_000); // wait for 5 hours (3000 blocks) to process transaction
		System::set_block_number(3_000);
		let xrp_balance =
			AssetsExt::balance(XrpAssetId::get(), &H160::from_slice(tx_address).into());
		assert_eq!(xrp_balance, 0);
	})
}

fn submit_transaction(
	relayer: AccountId,
	ledger_index: u64,
	transaction_hash: &[u8; 64],
	tx_address: &[u8; 20],
	i: u64,
) {
	let transaction = XrplTxData::Payment {
		amount: (i * 100u64) as Balance,
		address: H160::from_slice(tx_address),
	};
	assert_ok!(XRPLBridge::submit_transaction(
		Origin::signed(relayer),
		ledger_index,
		XrplTxHash::from_slice(transaction_hash),
		transaction,
		1234
	));
}
