use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::BadOrigin;

#[test]
fn test_approved_origin_enforced() {
	new_test_ext().execute_with(|| {
		// Should throw error on un_approved origin
		assert_noop!(ValidatorSet::add_validator(Origin::signed(1), 2), BadOrigin);
		// Should work with approved origin
		assert_ok!(ValidatorSet::add_validator(Origin::root(), 2));
	})
}

#[test]
fn test_add_validator_works() {
	new_test_ext().execute_with(|| {
		let _ = ValidatorSet::add_validator(Origin::root(), 2);
		let validator_list = ValidatorSet::validator_list();
		assert_eq!(validator_list.len(), 1);

		// Test trying to add a validator twice.
		assert_noop!(
			ValidatorSet::add_validator(Origin::root(), 2),
			Error::<Test>::DuplicateValidator
		);

		// Test trying to add a new validator.
		assert_ok!(ValidatorSet::add_validator(Origin::root(), 3));
	})
}

#[test]
fn test_remove_validator_works() {
	new_test_ext().execute_with(|| {
		let _ = ValidatorSet::add_validator(Origin::root(), 1);
		let _ = ValidatorSet::add_validator(Origin::root(), 2);

		// Test removing an existing validator.
		assert_ok!(ValidatorSet::remove_validator(Origin::root(), 2, 2));
		let validator_list = ValidatorSet::validator_list();
		assert_eq!(validator_list.len(), 1);

		// Should throw error if non-existing validator is tried to removed.
		assert_noop!(
			ValidatorSet::remove_validator(Origin::root(), 2, 2),
			Error::<Test>::ValidatorNotFound
		);
	})
}

#[test]
fn test_is_validator_works() {
	new_test_ext().execute_with(|| {
		let _ = ValidatorSet::add_validator(Origin::root(), 1);
		// Positive test
		assert!(ValidatorSet::is_validator(1, &1));
		// Negative test
		assert!(!ValidatorSet::is_validator(1, &2));
	})
}
