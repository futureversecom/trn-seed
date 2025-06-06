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

mod grant_dispatch_permission {
	use super::*;

	#[test]
	fn test_grant_dispatch_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);
			let spender = Spender::Grantor;
			let spending_balance = Some(100);
			let allowed_calls = all_allowed_calls();
			let expiry = Some(frame_system::Pallet::<Test>::block_number() + 10);

			// Grant permission
			assert_ok!(SyloActionPermissions::grant_dispatch_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				spender.clone(),
				spending_balance,
				allowed_calls.clone(),
				expiry,
			));

			// Verify permission exists
			let permission = DispatchPermissions::<Test>::get(&grantor, &grantee);
			assert!(permission.is_some());

			// Verify event was emitted
			System::assert_last_event(
				Event::DispatchPermissionGranted {
					grantor,
					grantee,
					spender,
					spending_balance,
					allowed_calls: allowed_calls.into_iter().collect(),
					expiry,
				}
				.into(),
			);
		});
	}

	#[test]
	fn test_grant_dispatch_permission_with_invalid_expiry() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Attempt to grant permission with an expiry in the past
			let expired_block = frame_system::Pallet::<Test>::block_number() - 1;
			assert_noop!(
				SyloActionPermissions::grant_dispatch_permission(
					RawOrigin::Signed(grantor.clone()).into(),
					grantee,
					Spender::Grantee,
					None,
					all_allowed_calls(),
					Some(expired_block),
				),
				Error::<Test>::InvalidExpiry
			);
		});
	}

	#[test]
	fn test_grant_dispatch_permission_already_exists() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);
			let spender = Spender::Grantor;
			let spending_balance = Some(100);
			let allowed_calls = all_allowed_calls();
			let expiry = Some(frame_system::Pallet::<Test>::block_number() + 10);

			// Grant permission
			assert_ok!(SyloActionPermissions::grant_dispatch_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				spender.clone(),
				spending_balance,
				allowed_calls.clone(),
				expiry,
			));

			// Attempt to grant permission again before expiry
			assert_noop!(
				SyloActionPermissions::grant_dispatch_permission(
					RawOrigin::Signed(grantor.clone()).into(),
					grantee.clone(),
					spender.clone(),
					spending_balance,
					allowed_calls.clone(),
					expiry,
				),
				Error::<Test>::PermissionAlreadyExists
			);
		});
	}

	#[test]
	fn test_grant_dispatch_permission_invalid_spending_balance() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Attempt to grant permission with spending_balance when spender is Grantee
			assert_noop!(
				SyloActionPermissions::grant_dispatch_permission(
					RawOrigin::Signed(grantor.clone()).into(),
					grantee.clone(),
					Spender::Grantee,
					Some(100),
					all_allowed_calls(),
					None,
				),
				Error::<Test>::InvalidSpendingBalance
			);
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

			assert_ok!(SyloActionPermissions::grant_dispatch_permission(
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
				grantor.clone(), // grantor
				Box::new(call),
			));

			// Verify event was emitted
			System::assert_last_event(
				Event::PermissionTransactExecuted { grantor, grantee }.into(),
			);
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

			assert_ok!(SyloActionPermissions::grant_dispatch_permission(
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

			assert_ok!(SyloActionPermissions::grant_dispatch_permission(
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

			assert_ok!(SyloActionPermissions::grant_dispatch_permission(
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

			assert_ok!(SyloActionPermissions::grant_dispatch_permission(
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

	#[test]
	fn test_transact_with_expired_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Grant permission with a valid expiry in the future
			let expiry_block = frame_system::Pallet::<Test>::block_number() + 5;
			assert_ok!(SyloActionPermissions::grant_dispatch_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::Grantee,
				None,
				all_allowed_calls(),
				Some(expiry_block),
			));

			// Simulate advancing the block number past the expiry
			frame_system::Pallet::<Test>::set_block_number(expiry_block + 1);

			// Attempt to execute an action
			let call: <Test as Config>::RuntimeCall =
				frame_system::Call::remark { remark: vec![] }.into();
			assert_noop!(
				SyloActionPermissions::transact(
					RawOrigin::Signed(grantee.clone()).into(),
					grantor,
					Box::new(call),
				),
				Error::<Test>::PermissionExpired
			);
		});
	}

	#[test]
	fn test_transact_with_not_expired_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Grant permission with a valid expiry in the future
			let future_block = frame_system::Pallet::<Test>::block_number() + 10;
			assert_ok!(SyloActionPermissions::grant_dispatch_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::Grantee,
				None,
				all_allowed_calls(),
				Some(future_block),
			));

			// Execute an action
			let call: <Test as Config>::RuntimeCall =
				frame_system::Call::remark { remark: vec![] }.into();
			assert_ok!(SyloActionPermissions::transact(
				RawOrigin::Signed(grantee.clone()).into(),
				grantor,
				Box::new(call),
			));
		});
	}

	#[test]
	fn test_transact_with_revoked_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Grant permission
			assert_ok!(SyloActionPermissions::grant_dispatch_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::Grantee,
				None,
				all_allowed_calls(),
				Some(frame_system::Pallet::<Test>::block_number() + 10),
			));

			// Revoke the permission
			assert_ok!(SyloActionPermissions::revoke_dispatch_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
			));

			// Attempt to execute an action
			let call: <Test as Config>::RuntimeCall =
				frame_system::Call::remark { remark: vec![] }.into();
			assert_noop!(
				SyloActionPermissions::transact(
					RawOrigin::Signed(grantee.clone()).into(),
					grantor,
					Box::new(call),
				),
				Error::<Test>::PermissionNotGranted
			);
		});
	}
}

mod update_dispatch_permission {
	use super::*;

	#[test]
	fn test_update_dispatch_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Grant initial permission
			assert_ok!(SyloActionPermissions::grant_dispatch_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::Grantor,
				Some(100),
				all_allowed_calls(),
				Some(frame_system::Pallet::<Test>::block_number() + 10),
			));

			// Update the permission
			let new_allowed_calls =
				BoundedBTreeSet::try_from(BTreeSet::from([to_call_id("system", "remark")]))
					.unwrap();
			let new_expiry = Some(frame_system::Pallet::<Test>::block_number() + 20);
			assert_ok!(SyloActionPermissions::update_dispatch_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Some(Spender::Grantor),
				Some(Some(200)),
				Some(new_allowed_calls.clone()),
				Some(new_expiry),
			));

			// Verify the updated permission
			let updated_permission = DispatchPermissions::<Test>::get(&grantor, &grantee).unwrap();
			assert_eq!(updated_permission.spender, Spender::Grantor);
			assert_eq!(updated_permission.spending_balance, Some(200));
			assert_eq!(updated_permission.allowed_calls, new_allowed_calls);
			assert_eq!(updated_permission.expiry, new_expiry);

			// Verify event was emitted
			System::assert_last_event(
				Event::DispatchPermissionUpdated {
					grantor,
					grantee,
					spender: Spender::Grantor,
					spending_balance: Some(200),
					allowed_calls: new_allowed_calls.into_iter().collect(),
					expiry: new_expiry,
				}
				.into(),
			);
		});
	}

	#[test]
	fn test_update_dispatch_permission_not_granted() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Attempt to update a non-existent permission
			assert_noop!(
				SyloActionPermissions::update_dispatch_permission(
					RawOrigin::Signed(grantor.clone()).into(),
					grantee.clone(),
					Some(Spender::Grantor),
					Some(Some(200)),
					None,
					None,
				),
				Error::<Test>::PermissionNotGranted
			);
		});
	}

	#[test]
	fn test_update_dispatch_permission_invalid_expiry() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Grant initial permission
			assert_ok!(SyloActionPermissions::grant_dispatch_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::Grantor,
				Some(100),
				all_allowed_calls(),
				Some(frame_system::Pallet::<Test>::block_number() + 10),
			));

			// Attempt to update with an invalid expiry
			assert_noop!(
				SyloActionPermissions::update_dispatch_permission(
					RawOrigin::Signed(grantor.clone()).into(),
					grantee.clone(),
					None,
					None,
					None,
					Some(Some(frame_system::Pallet::<Test>::block_number() - 1)),
				),
				Error::<Test>::InvalidExpiry
			);
		});
	}

	#[test]
	fn test_update_dispatch_permission_invalid_spending_balance() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Grant initial permission with spender as Grantee
			assert_ok!(SyloActionPermissions::grant_dispatch_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::Grantee,
				None,
				all_allowed_calls(),
				None,
			));

			// Attempt to update spending_balance when spender is Grantee
			assert_noop!(
				SyloActionPermissions::update_dispatch_permission(
					RawOrigin::Signed(grantor.clone()).into(),
					grantee.clone(),
					None,
					Some(Some(100)),
					None,
					None,
				),
				Error::<Test>::InvalidSpendingBalance
			);
		});
	}
}

mod revoke_dispatch_permission {
	use super::*;

	#[test]
	fn test_revoke_dispatch_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Grant initial permission
			assert_ok!(SyloActionPermissions::grant_dispatch_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::Grantor,
				Some(100),
				all_allowed_calls(),
				Some(frame_system::Pallet::<Test>::block_number() + 10),
			));

			// Revoke the permission
			assert_ok!(SyloActionPermissions::revoke_dispatch_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
			));

			// Verify the permission no longer exists
			let permission = DispatchPermissions::<Test>::get(&grantor, &grantee);
			assert!(permission.is_none());

			System::assert_last_event(Event::DispatchPermissionRevoked { grantor, grantee }.into());
		});
	}

	#[test]
	fn test_revoke_dispatch_permission_not_granted() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Attempt to revoke a non-existent permission
			assert_noop!(
				SyloActionPermissions::revoke_dispatch_permission(
					RawOrigin::Signed(grantor.clone()).into(),
					grantee.clone(),
				),
				Error::<Test>::PermissionNotGranted
			);
		});
	}
}
