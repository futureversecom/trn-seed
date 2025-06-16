// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

use std::collections::BTreeSet;

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;
use seed_pallet_common::test_prelude::*;
use sp_core::U256;

fn to_call_id(pallet: &str, extrinsic: &str) -> CallId<<Test as Config>::StringLimit> {
	(
		BoundedVec::truncate_from(pallet.as_bytes().to_vec()),
		BoundedVec::truncate_from(extrinsic.as_bytes().to_vec()),
	)
}

fn all_allowed_calls(
) -> BoundedBTreeSet<CallId<<Test as Config>::StringLimit>, <Test as Config>::MaxCallIds> {
	BoundedBTreeSet::try_from(BTreeSet::from([to_call_id("*", "*")])).unwrap()
}

mod grant_transact_permission {
	use super::*;

	#[test]
	fn test_grant_transact_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);
			let spender = Spender::GRANTOR;
			let spending_balance = Some(100);
			let allowed_calls = all_allowed_calls();
			let expiry = Some(frame_system::Pallet::<Test>::block_number() + 10);

			// Grant permission
			assert_ok!(SyloActionPermissions::grant_transact_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				spender.clone(),
				spending_balance,
				allowed_calls.clone(),
				expiry,
			));

			// Verify permission exists
			let permission = TransactPermissions::<Test>::get(&grantor, &grantee);
			assert!(permission.is_some());

			// Verify event was emitted
			System::assert_last_event(
				Event::TransactPermissionGranted {
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
	fn test_grant_transact_permission_with_invalid_expiry() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Attempt to grant permission with an expiry in the past
			let expired_block = frame_system::Pallet::<Test>::block_number() - 1;
			assert_noop!(
				SyloActionPermissions::grant_transact_permission(
					RawOrigin::Signed(grantor.clone()).into(),
					grantee,
					Spender::GRANTEE,
					None,
					all_allowed_calls(),
					Some(expired_block),
				),
				Error::<Test>::InvalidExpiry
			);
		});
	}

	#[test]
	fn test_grant_transact_permission_already_exists() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);
			let spender = Spender::GRANTOR;
			let spending_balance = Some(100);
			let allowed_calls = all_allowed_calls();
			let expiry = Some(frame_system::Pallet::<Test>::block_number() + 10);

			// Grant permission
			assert_ok!(SyloActionPermissions::grant_transact_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				spender.clone(),
				spending_balance,
				allowed_calls.clone(),
				expiry,
			));

			// Attempt to grant permission again before expiry
			assert_noop!(
				SyloActionPermissions::grant_transact_permission(
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
	fn test_grant_transact_permission_invalid_spending_balance() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Attempt to grant permission with spending_balance when spender is Grantee
			assert_noop!(
				SyloActionPermissions::grant_transact_permission(
					RawOrigin::Signed(grantor.clone()).into(),
					grantee.clone(),
					Spender::GRANTEE,
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

			assert_ok!(SyloActionPermissions::grant_transact_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::GRANTEE,
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

			assert_ok!(SyloActionPermissions::grant_transact_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::GRANTEE,
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

			assert_ok!(SyloActionPermissions::grant_transact_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::GRANTEE,
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

			assert_ok!(SyloActionPermissions::grant_transact_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::GRANTEE,
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

			assert_ok!(SyloActionPermissions::grant_transact_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::GRANTEE,
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
			assert_ok!(SyloActionPermissions::grant_transact_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::GRANTEE,
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
			assert_ok!(SyloActionPermissions::grant_transact_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::GRANTEE,
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
			assert_ok!(SyloActionPermissions::grant_transact_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::GRANTEE,
				None,
				all_allowed_calls(),
				Some(frame_system::Pallet::<Test>::block_number() + 10),
			));

			// Revoke the permission
			assert_ok!(SyloActionPermissions::revoke_transact_permission(
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

	#[test]
	fn test_transact_blacklisted_calls_are_rejected() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Grant permission with all_allowed_calls
			assert_ok!(SyloActionPermissions::grant_transact_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::GRANTEE,
				None,
				all_allowed_calls(),
				None,
			));

			let blacklisted_calls: Vec<<Test as Config>::RuntimeCall> = vec![
				pallet_sudo::Call::sudo {
					call: Box::new(frame_system::Call::remark { remark: vec![] }.into()),
				}
				.into(),
				pallet_futurepass::Call::proxy_extrinsic {
					futurepass: create_account(3),
					call: Box::new(frame_system::Call::remark { remark: vec![] }.into()),
				}
				.into(),
				pallet_proxy::Call::proxy {
					real: create_account(3),
					force_proxy_type: None,
					call: Box::new(frame_system::Call::remark { remark: vec![] }.into()),
				}
				.into(),
			];

			// Ensure each blacklisted call is rejected
			for call in blacklisted_calls.iter() {
				assert_noop!(
					SyloActionPermissions::transact(
						RawOrigin::Signed(grantee.clone()).into(),
						grantor.clone(),
						Box::new(call.clone()),
					),
					Error::<Test>::NotAuthorizedCall
				);
			}
		});
	}
}

mod update_transact_permission {
	use super::*;

	#[test]
	fn test_update_transact_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Grant initial permission
			assert_ok!(SyloActionPermissions::grant_transact_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::GRANTOR,
				Some(100),
				all_allowed_calls(),
				Some(frame_system::Pallet::<Test>::block_number() + 10),
			));

			// Update the permission
			let new_allowed_calls =
				BoundedBTreeSet::try_from(BTreeSet::from([to_call_id("system", "remark")]))
					.unwrap();
			let new_expiry = Some(frame_system::Pallet::<Test>::block_number() + 20);
			assert_ok!(SyloActionPermissions::update_transact_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Some(Spender::GRANTOR),
				Some(Some(200)),
				Some(new_allowed_calls.clone()),
				Some(new_expiry),
			));

			// Verify the updated permission
			let updated_permission = TransactPermissions::<Test>::get(&grantor, &grantee).unwrap();
			assert_eq!(updated_permission.spender, Spender::GRANTOR);
			assert_eq!(updated_permission.spending_balance, Some(200));
			assert_eq!(updated_permission.allowed_calls, new_allowed_calls);
			assert_eq!(updated_permission.expiry, new_expiry);

			// Verify event was emitted
			System::assert_last_event(
				Event::TransactPermissionUpdated {
					grantor,
					grantee,
					spender: Spender::GRANTOR,
					spending_balance: Some(200),
					allowed_calls: new_allowed_calls.into_iter().collect(),
					expiry: new_expiry,
				}
				.into(),
			);
		});
	}

	#[test]
	fn test_update_transact_permission_not_granted() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Attempt to update a non-existent permission
			assert_noop!(
				SyloActionPermissions::update_transact_permission(
					RawOrigin::Signed(grantor.clone()).into(),
					grantee.clone(),
					Some(Spender::GRANTOR),
					Some(Some(200)),
					None,
					None,
				),
				Error::<Test>::PermissionNotGranted
			);
		});
	}

	#[test]
	fn test_update_transact_permission_invalid_expiry() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Grant initial permission
			assert_ok!(SyloActionPermissions::grant_transact_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::GRANTOR,
				Some(100),
				all_allowed_calls(),
				Some(frame_system::Pallet::<Test>::block_number() + 10),
			));

			// Attempt to update with an invalid expiry
			assert_noop!(
				SyloActionPermissions::update_transact_permission(
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
	fn test_update_transact_permission_invalid_spending_balance() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Grant initial permission with spender as Grantee
			assert_ok!(SyloActionPermissions::grant_transact_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::GRANTEE,
				None,
				all_allowed_calls(),
				None,
			));

			// Attempt to update spending_balance when spender is Grantee
			assert_noop!(
				SyloActionPermissions::update_transact_permission(
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

mod revoke_transact_permission {
	use super::*;

	#[test]
	fn test_revoke_transact_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Grant initial permission
			assert_ok!(SyloActionPermissions::grant_transact_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				Spender::GRANTOR,
				Some(100),
				all_allowed_calls(),
				Some(frame_system::Pallet::<Test>::block_number() + 10),
			));

			// Revoke the permission
			assert_ok!(SyloActionPermissions::revoke_transact_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
			));

			// Verify the permission no longer exists
			let permission = TransactPermissions::<Test>::get(&grantor, &grantee);
			assert!(permission.is_none());

			System::assert_last_event(Event::TransactPermissionRevoked { grantor, grantee }.into());
		});
	}

	#[test]
	fn test_revoke_transact_permission_not_granted() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// Attempt to revoke a non-existent permission
			assert_noop!(
				SyloActionPermissions::revoke_transact_permission(
					RawOrigin::Signed(grantor.clone()).into(),
					grantee.clone(),
				),
				Error::<Test>::PermissionNotGranted
			);
		});
	}
}

mod accept_transact_permission {
	use sp_core::{hexdisplay::AsBytesRef, keccak_256};

	use super::*;

	#[test]
	fn test_accept_transact_permission_with_eip191_signature() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (signer, grantor) = create_random_pair();
			let grantee: AccountId = create_account(2);

			let nonce = U256::from(1);
			let mut allowed_calls = BoundedBTreeSet::new();
			allowed_calls.try_insert(to_call_id("system", "*")).unwrap();
			let permission_token = TransactPermissionToken {
				grantee: grantee.clone(),
				futurepass: None,
				spender: Spender::GRANTOR,
				spending_balance: Some(100),
				allowed_calls: allowed_calls.clone(),
				expiry: None,
				nonce,
			};

			let token_signature = TransactPermissionTokenSignature::EIP191(
				signer
					.sign_prehashed(&keccak_256(
						seed_primitives::ethereum_signed_message(
							Encode::encode(&permission_token).as_bytes_ref(),
						)
						.as_ref(),
					))
					.into(),
			);

			// Accept the permission
			assert_ok!(SyloActionPermissions::accept_transact_permission(
				RawOrigin::Signed(grantee.clone()).into(),
				grantor.clone(),
				permission_token.clone(),
				token_signature.clone(),
			));

			// Verify event was emitted
			System::assert_last_event(
				Event::TransactPermissionAccepted { grantor, grantee }.into(),
			);

			// Verify permission exists
			let permission = TransactPermissions::<Test>::get(&grantor, &grantee);
			assert!(permission.is_some());

			// Verify the permission can be used for a transaction
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
	fn test_accept_transact_permission_grantee_must_match_origin() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (signer, grantor) = create_random_pair();
			let grantee: AccountId = create_account(2);
			let invalid_grantee: AccountId = create_account(3);

			let nonce = U256::from(1);
			let expiry = Some(frame_system::Pallet::<Test>::block_number() + 10);
			let allowed_calls = all_allowed_calls();
			let permission_token = TransactPermissionToken {
				grantee: grantee.clone(),
				futurepass: None,
				spender: Spender::GRANTOR,
				spending_balance: Some(100),
				allowed_calls: allowed_calls.clone(),
				expiry,
				nonce,
			};

			let token_signature = TransactPermissionTokenSignature::EIP191(
				signer
					.sign_prehashed(&keccak_256(
						seed_primitives::ethereum_signed_message(
							Encode::encode(&permission_token).as_bytes_ref(),
						)
						.as_ref(),
					))
					.into(),
			);

			// Attempt to accept the permission with an invalid grantee
			assert_noop!(
				SyloActionPermissions::accept_transact_permission(
					RawOrigin::Signed(invalid_grantee.clone()).into(),
					grantor.clone(),
					permission_token.clone(),
					token_signature.clone(),
				),
				Error::<Test>::GranteeDoesNotMatch
			);
		});
	}

	#[test]
	fn test_accept_transact_permission_nonce_cannot_be_reused() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (signer, grantor) = create_random_pair();
			let grantee: AccountId = create_account(2);

			let nonce = U256::from(1);
			let expiry = Some(frame_system::Pallet::<Test>::block_number() + 10);
			let allowed_calls = all_allowed_calls();
			let permission_token = TransactPermissionToken {
				grantee: grantee.clone(),
				futurepass: None,
				spender: Spender::GRANTOR,
				spending_balance: Some(100),
				allowed_calls: allowed_calls.clone(),
				expiry,
				nonce,
			};

			let token_signature = TransactPermissionTokenSignature::EIP191(
				signer
					.sign_prehashed(&keccak_256(
						seed_primitives::ethereum_signed_message(
							Encode::encode(&permission_token).as_bytes_ref(),
						)
						.as_ref(),
					))
					.into(),
			);

			// Accept the permission
			assert_ok!(SyloActionPermissions::accept_transact_permission(
				RawOrigin::Signed(grantee.clone()).into(),
				grantor.clone(),
				permission_token.clone(),
				token_signature.clone(),
			));

			// Attempt to reuse the same nonce
			assert_noop!(
				SyloActionPermissions::accept_transact_permission(
					RawOrigin::Signed(grantee.clone()).into(),
					grantor.clone(),
					permission_token.clone(),
					token_signature.clone(),
				),
				Error::<Test>::NonceAlreadyUsed
			);
		});
	}

	#[test]
	fn test_accept_transact_permission_signature_must_match_token() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (signer, grantor) = create_random_pair();
			let grantee: AccountId = create_account(2);

			let nonce = U256::from(1);
			let expiry = Some(frame_system::Pallet::<Test>::block_number() + 10);
			let allowed_calls = all_allowed_calls();
			let permission_token = TransactPermissionToken {
				grantee: grantee.clone(),
				futurepass: None,
				spender: Spender::GRANTOR,
				spending_balance: Some(100),
				allowed_calls: allowed_calls.clone(),
				expiry,
				nonce,
			};

			// Create a signature for a different token
			let invalid_permission_token = TransactPermissionToken {
				grantee: grantee.clone(),
				futurepass: None,
				spender: Spender::GRANTEE, // Different spender
				spending_balance: None,
				allowed_calls: allowed_calls.clone(),
				expiry,
				nonce,
			};

			let token_signature = TransactPermissionTokenSignature::EIP191(
				signer
					.sign_prehashed(&keccak_256(
						seed_primitives::ethereum_signed_message(
							Encode::encode(&permission_token).as_bytes_ref(),
						)
						.as_ref(),
					))
					.into(),
			);

			// Attempt to accept the permission with a mismatched signature
			assert_noop!(
				SyloActionPermissions::accept_transact_permission(
					RawOrigin::Signed(grantee.clone()).into(),
					grantor.clone(),
					invalid_permission_token.clone(),
					token_signature.clone(),
				),
				Error::<Test>::GrantorDoesNotMatch
			);
		});
	}
}
