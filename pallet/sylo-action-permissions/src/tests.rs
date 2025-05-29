use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;
use seed_pallet_common::test_prelude::*;

mod grant_action_permission {
	use super::*;

	#[test]
	fn test_grant_action_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Grant permission
			assert_ok!(SyloActionPermissions::grant_action_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee, // grantee
			));

			// Verify permission exists
			let permission = Permissions::<Test>::get(&grantor, &grantee);
			assert!(permission.is_some());
		});
	}
}

mod execute_action {
	use super::*;

	#[test]
	fn test_execute_action_with_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			assert_ok!(SyloActionPermissions::grant_action_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
			));

			// Execute action
			let call: <Test as Config>::RuntimeCall =
				frame_system::Call::remark { remark: vec![] }.into();

			assert_ok!(SyloActionPermissions::execute_action(
				RawOrigin::Signed(grantee.clone()).into(),
				grantor, // grantor
				Box::new(call),
			));
		});
	}

	#[test]
	fn test_execute_action_without_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Attempt to execute action without permission
			let call: <Test as Config>::RuntimeCall =
				frame_system::Call::remark { remark: vec![] }.into();
			assert_noop!(
				SyloActionPermissions::execute_action(
					RawOrigin::Signed(grantee.clone()).into(),
					grantor, // grantor
					Box::new(call),
				),
				Error::<Test>::PermissionNotGranted
			);
		});
	}
}
