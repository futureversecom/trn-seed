use crate::{mock::*, vec, XrplTxData, H512};
use frame_support::assert_ok;
use seed_primitives::Balance;
use sp_core::H160;

#[test]
fn test_add_transaction_works() {
	new_test_ext().execute_with(|| {
		XRPLBridge::initialize_relayer(&vec![1]);
		for i in 1..100u64 {
			add_transaction(1, i * 1_000_000, i);
		}
	})
}

fn add_transaction(relayer: u64, ledger_index: u64, i: u64) {
	let transaction_hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
	let address = b"6490B68F1116BFE87DDD";
	let transaction =
		XrplTxData::Payment { amount: (i * 100u64) as Balance, address: H160::from_slice(address) };
	assert_ok!(XRPLBridge::submit_transaction(
		Origin::signed(relayer),
		ledger_index,
		H512::from_slice(transaction_hash),
		transaction,
		1234
	));
}
