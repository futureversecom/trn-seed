use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;
use seed_primitives::validator::crypto::AuthorityId;
use sp_core::ByteArray;
use sp_runtime::traits::BadOrigin;

#[test]
fn test_approved_origin_enforced() {
	new_test_ext().execute_with(|| {
		let account1: AuthorityId = AuthorityId::from_slice(&[1_u8; 33]).unwrap();
		let account_address = b"6490B68F1116BFE87DDD";
		let account = AccountId::from(H160::from_slice(account_address));
		// Should throw error on un_approved origin
		assert_noop!(
			DefaultValidatorWhiteList::add_validator(Origin::signed(account), account1.clone()),
			BadOrigin
		);
		// Should work with approved origin
		assert_ok!(DefaultValidatorWhiteList::add_validator(Origin::root(), account1));
	})
}

#[test]
fn test_add_validator_works() {
	new_test_ext().execute_with(|| {
		let account1: AuthorityId = AuthorityId::from_slice(&[1_u8; 33]).unwrap();
		let account2: AuthorityId = AuthorityId::from_slice(&[2_u8; 33]).unwrap();
		let _ = DefaultValidatorWhiteList::add_validator(Origin::root(), account1.clone());
		assert_eq!(<WhiteListValidators<Test>>::iter_values().collect::<Vec<_>>(), vec![true]);

		// Test trying to add a validator twice.
		assert_noop!(
			DefaultValidatorWhiteList::add_validator(Origin::root(), account1),
			Error::<Test>::DuplicateValidator
		);

		// Test trying to add a new validator.
		assert_ok!(DefaultValidatorWhiteList::add_validator(Origin::root(), account2));
	})
}

#[test]
fn test_remove_validator_works() {
	new_test_ext().execute_with(|| {
		let account1: AuthorityId = AuthorityId::from_slice(&[1_u8; 33]).unwrap();
		let account2: AuthorityId = AuthorityId::from_slice(&[2_u8; 33]).unwrap();
		let _ = DefaultValidatorWhiteList::add_validator(Origin::root(), account1);
		let _ = DefaultValidatorWhiteList::add_validator(Origin::root(), account2.clone());

		// Test removing an existing validator.
		assert_ok!(DefaultValidatorWhiteList::remove_validator(Origin::root(), account2.clone()));
		assert_eq!(<WhiteListValidators<Test>>::iter_values().collect::<Vec<_>>(), vec![true]);

		// Should throw error if non-existing validator is tried to removed.
		assert_noop!(
			DefaultValidatorWhiteList::remove_validator(Origin::root(), account2),
			Error::<Test>::ValidatorNotFound
		);
	})
}

#[test]
fn test_is_validator_works() {
	new_test_ext().execute_with(|| {
		let account1: AuthorityId = AuthorityId::from_slice(&[1_u8; 33]).unwrap();
		let account2: AuthorityId = AuthorityId::from_slice(&[2_u8; 33]).unwrap();
		let _ = DefaultValidatorWhiteList::add_validator(Origin::root(), account1.clone());
		// Positive test
		assert!(DefaultValidatorWhiteList::is_validator(&account1));
		// Negative test
		assert!(!DefaultValidatorWhiteList::is_validator(&account2));
	})
}
