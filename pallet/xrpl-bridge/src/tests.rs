use crate::mock::*;
use frame_support::assert_ok;

#[test]
fn test_add_transaction_works() {
	new_test_ext().execute_with(|| {
		for i in 1..100u64 {
			add_transaction(i % 10, i * 1_000_000);
		}
	})
}

fn add_transaction(relayer: u64, block_number: u64) {
	let hash = b"6490B68F1116BFE87DDDAD4C5482D1514F9CA8B9B5B5BFD3CF81D8E68745317B";
	let transaction = br#"{
		  "TransactionType" : "Payment",
		  "Account" : "rf1BiGeXwwQoi8Z2ueFYTEXSwuJYfV2Jpn",
		  "Destination" : "ra5nK24KXen9AHvsdFTKHSANinZseWnPcX",
		  "Amount" : "1000000"
		}"#;
	assert_ok!(XRPLBridge::submit_transaction(
		Origin::signed(relayer),
		block_number,
		hash.to_vec().try_into().unwrap(),
		transaction.to_vec().try_into().unwrap(),
		1234
	));
}
