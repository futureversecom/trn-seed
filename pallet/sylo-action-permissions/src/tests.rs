use std::collections::BTreeSet;

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;
use seed_pallet_common::test_prelude::*;

fn to_call_id(method: &str, extrinsic: &str) -> CallId<<Test as Config>::StringLimit> {
	(
		BoundedVec::truncate_from(method.as_bytes().to_vec()),
		BoundedVec::truncate_from(extrinsic.as_bytes().to_vec()),
	)
}

fn all_allowed_calls(
) -> BoundedBTreeSet<CallId<<Test as Config>::StringLimit>, <Test as Config>::MaxCallIds> {
	BoundedBTreeSet::try_from(BTreeSet::from([to_call_id("*", "*")])).unwrap()
}

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
				Spender::Grantee,
				None,
				all_allowed_calls(),
				None,
			));

			// Verify permission exists
			let permission = DispatchPermissions::<Test>::get(&grantor, &grantee);
			assert!(permission.is_some());
		});
	}
}

mod transact {
	use super::*;

	#[test]
	fn test_transact_with_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			assert_ok!(SyloActionPermissions::grant_action_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::Grantee,
				None,
				all_allowed_calls(),
				None,
			));

			// Execute action
			let call: <Test as Config>::RuntimeCall =
				frame_system::Call::remark { remark: vec![] }.into();

			assert_ok!(SyloActionPermissions::transact(
				RawOrigin::Signed(grantee.clone()).into(),
				grantor, // grantor
				Box::new(call),
			));
		});
	}

	#[test]
	fn test_transact_without_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Attempt to execute action without permission
			let call: <Test as Config>::RuntimeCall =
				frame_system::Call::remark { remark: vec![] }.into();
			assert_noop!(
				SyloActionPermissions::transact(
					RawOrigin::Signed(grantee.clone()).into(),
					grantor, // grantor
					Box::new(call),
				),
				Error::<Test>::PermissionNotGranted
			);
		});
	}

	#[test]
	fn test_transact_with_specific_allowed_calls() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Allow only the `remark` function from the `system` pallet
			let allowed_calls =
				BoundedBTreeSet::try_from(BTreeSet::from([to_call_id("system", "remark")]))
					.unwrap();

			assert_ok!(SyloActionPermissions::grant_action_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::Grantee,
				None,
				allowed_calls,
				None,
			));

			// Execute allowed action
			let call: <Test as Config>::RuntimeCall =
				frame_system::Call::remark { remark: vec![] }.into();
			assert_ok!(SyloActionPermissions::transact(
				RawOrigin::Signed(grantee.clone()).into(),
				grantor.clone(),
				Box::new(call),
			));

			// Attempt to execute a disallowed action
			let call: <Test as Config>::RuntimeCall =
				frame_system::Call::remark_with_event { remark: vec![] }.into();
			assert_noop!(
				SyloActionPermissions::transact(
					RawOrigin::Signed(grantee.clone()).into(),
					grantor,
					Box::new(call),
				),
				Error::<Test>::NotAuthorizedCall
			);
		});
	}

	#[test]
	fn test_transact_with_wildcard_allowed_calls() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Allow all calls using wildcard
			let allowed_calls = all_allowed_calls();

			assert_ok!(SyloActionPermissions::grant_action_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::Grantee,
				None,
				allowed_calls,
				None,
			));

			// Execute any action
			let call: <Test as Config>::RuntimeCall =
				frame_system::Call::remark { remark: vec![] }.into();
			assert_ok!(SyloActionPermissions::transact(
				RawOrigin::Signed(grantee.clone()).into(),
				grantor.clone(),
				Box::new(call),
			));
		});
	}

	#[test]
	fn test_transact_with_empty_allowed_calls() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// No calls are allowed
			let allowed_calls = BoundedBTreeSet::new();

			assert_ok!(SyloActionPermissions::grant_action_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::Grantee,
				None,
				allowed_calls,
				None,
			));

			// Attempt to execute any action
			let call: <Test as Config>::RuntimeCall =
				frame_system::Call::remark { remark: vec![] }.into();
			assert_noop!(
				SyloActionPermissions::transact(
					RawOrigin::Signed(grantee.clone()).into(),
					grantor,
					Box::new(call),
				),
				Error::<Test>::NotAuthorizedCall
			);
		});
	}

	#[test]
	fn test_transact_with_function_wildcard_allowed_calls() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Allow all calls from the `system` pallet
			let mut allowed_calls = BoundedBTreeSet::new();
			allowed_calls.try_insert(to_call_id("system", "*")).unwrap();

			assert_ok!(SyloActionPermissions::grant_action_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::Grantee,
				None,
				allowed_calls,
				None,
			));

			// Execute allowed actions
			let calls = vec![
				frame_system::Call::remark { remark: vec![] },
				frame_system::Call::remark_with_event { remark: vec![] },
			];

			for call in calls {
				let call: <Test as Config>::RuntimeCall = call.into();
				assert_ok!(SyloActionPermissions::transact(
					RawOrigin::Signed(grantee.clone()).into(),
					grantor.clone(),
					Box::new(call),
				));
			}

			// Attempt to execute a disallowed action from another pallet
			let call: <Test as Config>::RuntimeCall = pallet_assets_ext::Call::transfer {
				asset_id: 1,
				destination: create_account(3),
				amount: 100,
				keep_alive: true,
			}
			.into();
			assert_noop!(
				SyloActionPermissions::transact(
					RawOrigin::Signed(grantee.clone()).into(),
					grantor,
					Box::new(call),
				),
				Error::<Test>::NotAuthorizedCall
			);
		});
	}
}
