#![cfg(test)]
use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok, error::BadOrigin};

#[test]
fn create_futurepass() {
	TestExt::default().build().execute_with(|| {
		let owner = create_account(10);
		let delegate = create_account(20);

		// assert the futurepass is not created yet for the delegate account
		assert_eq!(Holders::<Test>::contains_key(&delegate), false);

		// creation fails if not balance
		// assert_noop!(
		// 	Futurepass::create(Origin::signed(owner.clone()), delegate.clone()),
		// 	pallet_balances::Error::<Test>::InsufficientBalance
		// );

		// TODO fund account (origin)

		// create futurepass account
		// assert_ok!(Futurepass::create(Origin::signed(owner.clone()), delegate.clone()));

		// TODO assert last event (account creation)

		// // Check if the futurepass account is created and associated with the delegate account
		// let futurepass = Holders::<Test>::get(&delegate).unwrap();
		// assert!(pallet_futurepass::ProxyProvider::<Test>::exists(&futurepass, &delegate));

		// // Test for error scenario when the account is already registered
		// assert_noop!(
		// 		Futurepass::create(Origin::signed(owner), delegate.clone()),
		// 		Error::<Test>::AccountAlreadyRegistered
		// );
	});
}
