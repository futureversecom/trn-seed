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

use super::*;
use mock::{RuntimeEvent as MockEvent, SyloDataPermissions, SyloDataVerification, System, Test};
use seed_pallet_common::test_prelude::*;

fn bounded_string(str: &str) -> BoundedVec<u8, <Test as Config>::StringLimit> {
	BoundedVec::truncate_from(str.as_bytes().to_vec())
}

fn create_validation_record(
	author: <Test as frame_system::Config>::AccountId,
	data_id: &str,
) -> BoundedVec<u8, mock::StringLimit> {
	let data_id = bounded_string(data_id);

	assert_ok!(SyloDataVerification::create_validation_record(
		RawOrigin::Signed(author.clone()).into(),
		data_id.clone(),
		BoundedVec::new(),
		BoundedVec::new(),
		BoundedVec::new(),
		H256::from_low_u64_be(123),
	));

	data_id
}
mod grant_data_permissions {
	use super::*;

	#[test]
	fn grant_data_permissions_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			let data_id = create_validation_record(grantor.clone(), "data-id");

			let permission = DataPermission::VIEW;
			let expiry = None;
			let irrevocable = false;

			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantor.clone(),
				grantee.clone(),
				BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
				permission,
				expiry,
				irrevocable
			));

			System::assert_last_event(MockEvent::SyloDataPermissions(
				crate::Event::DataPermissionGranted {
					data_author: grantor.clone(),
					grantor: grantor.clone(),
					grantee: grantee.clone(),
					data_id: data_id.clone().to_vec(),
					permission,
					expiry,
					irrevocable,
				},
			));

			let permission_record = PermissionRecords::<Test>::get((&grantor, &grantee, &data_id))
				.into_iter()
				.next()
				.unwrap()
				.1;

			let expected_permission_record = PermissionRecord {
				grantor,
				permission,
				block: System::block_number(),
				expiry,
				irrevocable,
			};

			assert_eq!(permission_record, expected_permission_record);
		});
	}

	#[test]
	fn grant_multiple_permissions_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			let data_ids: Vec<_> = (0..3)
				.map(|i| {
					return create_validation_record(
						grantor.clone(),
						format!("data-id-{i}").as_str(),
					);
				})
				.collect();

			let permission = DataPermission::MODIFY;
			let expiry = Some(2000);
			let irrevocable = false;

			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantor.clone(),
				grantee.clone(),
				BoundedVec::try_from(data_ids.clone()).unwrap(),
				permission,
				expiry,
				irrevocable
			));

			for data_id in data_ids.iter() {
				let permission_record =
					PermissionRecords::<Test>::get((&grantor, &grantee, data_id))
						.into_iter()
						.next()
						.unwrap()
						.1;

				let expected_permission_record = PermissionRecord {
					grantor,
					permission,
					block: System::block_number(),
					expiry,
					irrevocable,
				};

				assert_eq!(permission_record, expected_permission_record);
			}
		});
	}

	#[test]
	fn grant_same_permission_adds_another_record() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			let data_id = create_validation_record(grantor.clone(), "data-id");

			let irrevocable = false;

			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantor.clone(),
				grantee.clone(),
				BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
				DataPermission::VIEW,
				None,
				irrevocable
			));

			let new_permission = DataPermission::MODIFY;
			let new_expiry = Some(2000);

			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantor.clone(),
				grantee.clone(),
				BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
				new_permission,
				new_expiry,
				irrevocable
			));

			let permission_record = PermissionRecords::<Test>::get((&grantor, &grantee, &data_id))
				.get(1)
				.unwrap()
				.clone()
				.1;

			let expected_permission_record = PermissionRecord {
				grantor,
				permission: new_permission,
				block: System::block_number(),
				expiry: new_expiry,
				irrevocable,
			};

			assert_eq!(permission_record, expected_permission_record);
		});
	}

	#[test]
	fn cannot_grant_expirable_and_irrevocable_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			let data_id = create_validation_record(grantor.clone(), "data-id");

			assert_noop!(
				SyloDataPermissions::grant_data_permissions(
					RawOrigin::Signed(grantor.clone()).into(),
					grantor.clone(),
					grantee.clone(),
					BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
					DataPermission::VIEW,
					Some(2000),
					true
				),
				Error::<Test>::IrrevocableCannotBeExpirable
			);
		});
	}

	#[test]
	fn cannot_grant_permission_without_validation_record() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			let data_id = bounded_string("data-id");

			assert_noop!(
				SyloDataPermissions::grant_data_permissions(
					RawOrigin::Signed(grantor.clone()).into(),
					grantor.clone(),
					grantee.clone(),
					BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
					DataPermission::VIEW,
					None,
					false
				),
				Error::<Test>::DataRecordDoesNotExist
			);
		});
	}

	#[test]
	fn cannot_grant_permission_with_invalid_expiry() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			let data_id = create_validation_record(grantor.clone(), "data-id");

			assert_noop!(
				SyloDataPermissions::grant_data_permissions(
					RawOrigin::Signed(grantor.clone()).into(),
					grantor.clone(),
					grantee.clone(),
					BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
					DataPermission::VIEW,
					Some(0),
					false
				),
				Error::<Test>::InvalidExpiry
			);

			System::set_block_number(100);

			assert_noop!(
				SyloDataPermissions::grant_data_permissions(
					RawOrigin::Signed(grantor.clone()).into(),
					grantor.clone(),
					grantee.clone(),
					BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
					DataPermission::VIEW,
					Some(50),
					false
				),
				Error::<Test>::InvalidExpiry
			);
		});
	}

	#[test]
	fn can_grant_permission_as_distributor() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);
			let distributor: AccountId = create_account(3);

			let data_id = create_validation_record(grantor.clone(), "data-id");

			// grant the distributor permission
			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantor.clone(),
				distributor.clone(),
				BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
				DataPermission::DISTRIBUTE,
				None,
				false
			));

			// grant permission as distributor
			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(distributor.clone()).into(),
				grantor.clone(),
				grantee.clone(),
				BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
				DataPermission::VIEW,
				None,
				false
			));
		});
	}

	#[test]
	fn cannot_be_distributor_without_distribute_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);
			let distributor: AccountId = create_account(3);

			let data_id = create_validation_record(grantor.clone(), "data-id");

			// grant permission as distributor
			assert_noop!(
				SyloDataPermissions::grant_data_permissions(
					RawOrigin::Signed(distributor.clone()).into(),
					grantor.clone(),
					grantee.clone(),
					BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
					DataPermission::VIEW,
					None,
					false
				),
				Error::<Test>::MissingDistributePermission
			);
		});
	}

	#[test]
	fn cannot_grant_distribute_permission_as_distributor() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);
			let distributor: AccountId = create_account(3);

			let data_id = create_validation_record(grantor.clone(), "data-id");

			// grant the distributor permission
			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantor.clone(),
				distributor.clone(),
				BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
				DataPermission::DISTRIBUTE,
				None,
				false
			));

			// grant permission as distributor
			assert_noop!(
				SyloDataPermissions::grant_data_permissions(
					RawOrigin::Signed(distributor.clone()).into(),
					grantor.clone(),
					grantee.clone(),
					BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
					DataPermission::DISTRIBUTE,
					None,
					false
				),
				Error::<Test>::CannotGrantDistributePermission
			);
		});
	}

	#[test]
	fn cannot_grant_irrevocable_permission_as_distributor() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);
			let distributor: AccountId = create_account(3);

			let data_id = create_validation_record(grantor.clone(), "data-id");

			// grant the distributor permission
			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantor.clone(),
				distributor.clone(),
				BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
				DataPermission::DISTRIBUTE,
				None,
				false
			));

			// grant permission as distributor
			assert_noop!(
				SyloDataPermissions::grant_data_permissions(
					RawOrigin::Signed(distributor.clone()).into(),
					grantor.clone(),
					grantee.clone(),
					BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
					DataPermission::VIEW,
					None,
					true
				),
				Error::<Test>::CannotGrantIrrevocablePermission
			);
		});
	}
}

mod revoke_data_permission {
	use super::*;

	#[test]
	fn revoke_data_permission_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			let data_id = create_validation_record(grantor.clone(), "data-id");

			let permission = DataPermission::VIEW;

			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantor.clone(),
				grantee.clone(),
				BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
				permission,
				None,
				false
			));

			let permission_id = 0;

			assert_ok!(SyloDataPermissions::revoke_data_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantor.clone(),
				permission_id,
				grantee.clone(),
				data_id.clone(),
			));

			System::assert_last_event(MockEvent::SyloDataPermissions(
				crate::Event::DataPermissionRevoked {
					revoker: grantor.clone(),
					grantee: grantee.clone(),
					permission,
					data_id: data_id.to_vec(),
				},
			));

			assert_eq!(PermissionRecords::<Test>::get((&grantor, &grantee, &data_id)).len(), 0);
		});
	}

	#[test]
	fn revoke_data_permission_as_distributor_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);
			let distributor: AccountId = create_account(3);

			let data_id = create_validation_record(grantor.clone(), "data-id");

			// grant the distributor permission
			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantor.clone(),
				distributor.clone(),
				BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
				DataPermission::DISTRIBUTE,
				None,
				false
			));

			// grant permission as distributor
			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(distributor.clone()).into(),
				grantor.clone(),
				grantee.clone(),
				BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
				DataPermission::VIEW,
				None,
				false
			));

			// revoke permission as distributor
			assert_ok!(SyloDataPermissions::revoke_data_permission(
				RawOrigin::Signed(distributor.clone()).into(),
				grantor.clone(),
				1,
				grantee.clone(),
				data_id.clone(),
			));
		})
	}

	#[test]
	fn cannot_revoke_if_irrevocable() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			let data_id = create_validation_record(grantor.clone(), "data-id");

			let permission = DataPermission::VIEW;

			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantor.clone(),
				grantee.clone(),
				BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
				permission,
				None,
				true
			));

			let permission_id = 0;

			assert_noop!(
				SyloDataPermissions::revoke_data_permission(
					RawOrigin::Signed(grantor.clone()).into(),
					grantor.clone(),
					permission_id,
					grantee.clone(),
					data_id.clone(),
				),
				Error::<Test>::PermissionIrrevocable
			);
		});
	}

	#[test]
	fn cannot_revoke_permission_if_not_exists() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			let data_id = create_validation_record(grantor.clone(), "data-id");

			let permission = DataPermission::VIEW;

			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantor.clone(),
				grantee.clone(),
				BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
				permission,
				None,
				false
			));

			assert_noop!(
				SyloDataPermissions::revoke_data_permission(
					RawOrigin::Signed(grantor.clone()).into(),
					grantor.clone(),
					1, // wrong permission id
					grantee.clone(),
					data_id.clone(),
				),
				Error::<Test>::PermissionNotFound
			);
		})
	}

	#[test]
	fn cannot_revoke_permission_if_not_grantor() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);
			let distributor: AccountId = create_account(3);

			let data_id = create_validation_record(grantor.clone(), "data-id");

			// grant the distributor permission
			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantor.clone(),
				distributor.clone(),
				BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
				DataPermission::DISTRIBUTE,
				None,
				false
			));

			// grant another permission
			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantor.clone(),
				grantee.clone(),
				BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
				DataPermission::VIEW,
				None,
				false
			));

			// attempt to revoke permission as distributor
			assert_noop!(
				SyloDataPermissions::revoke_data_permission(
					RawOrigin::Signed(distributor.clone()).into(),
					grantor.clone(),
					1,
					grantee.clone(),
					data_id.clone(),
				),
				Error::<Test>::NotPermissionGrantor
			);
		})
	}

	#[test]
	fn can_always_revoke_permission_as_data_author() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);
			let distributor: AccountId = create_account(3);

			let data_id = create_validation_record(grantor.clone(), "data-id");

			// grant the distributor permission
			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantor.clone(),
				distributor.clone(),
				BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
				DataPermission::DISTRIBUTE,
				None,
				false
			));

			// grant permission as distributor
			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(distributor.clone()).into(),
				grantor.clone(),
				grantee.clone(),
				BoundedVec::try_from(vec![data_id.clone()]).unwrap(),
				DataPermission::VIEW,
				None,
				false
			));

			// revoke as data author
			assert_ok!(SyloDataPermissions::revoke_data_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantor.clone(),
				1,
				grantee.clone(),
				data_id.clone(),
			));
		})
	}
}

mod grant_tagged_permissions {
	use super::*;

	#[test]
	fn grant_tagged_permissions_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			let tags = BoundedVec::try_from(vec![bounded_string("tag-1"), bounded_string("tag-2")])
				.unwrap();

			let permission = DataPermission::MODIFY;
			let expiry = Some(2000);
			let irrevocable = false;

			assert_ok!(SyloDataPermissions::grant_tagged_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				permission,
				tags.clone(),
				expiry,
				irrevocable
			));

			System::assert_last_event(MockEvent::SyloDataPermissions(
				crate::Event::TaggedDataPermissionsGranted {
					grantor: grantor.clone(),
					grantee: grantee.clone(),
					permission,
					tags: tags.clone().iter().map(|v| v.to_vec()).collect(),
					expiry,
					irrevocable,
				},
			));

			let permission_record = TaggedPermissionRecords::<Test>::get(&grantor, &grantee)
				.get(0)
				.unwrap()
				.clone()
				.1;

			let expected = TaggedPermissionRecord {
				permission,
				tags,
				block: System::block_number(),
				expiry,
				irrevocable,
			};

			assert_eq!(permission_record, expected);
		});
	}

	#[test]
	fn grant_tagged_permissions_for_distributor_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);
			let distributor: AccountId = create_account(3);

			let tags = BoundedVec::try_from(vec![bounded_string("tag-1"), bounded_string("tag-2")])
				.unwrap();

			// grant distributor permission for those tags
			assert_ok!(SyloDataPermissions::grant_tagged_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				distributor.clone(),
				DataPermission::DISTRIBUTE,
				tags.clone(),
				None,
				false
			));

			let data_id = bounded_string("data-id");

			// create a verification record with the specified tags
			assert_ok!(SyloDataVerification::create_validation_record(
				RawOrigin::Signed(grantor.clone()).into(),
				data_id.clone(),
				BoundedVec::new(),
				BoundedVec::new(),
				tags.clone(),
				H256::from_low_u64_be(123),
			));

			// test granting permission as a distributor
			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(distributor.clone()).into(),
				grantor.clone(),
				grantee.clone(),
				BoundedVec::try_from(vec![data_id]).unwrap(),
				DataPermission::VIEW,
				None,
				false
			));
		});
	}

	#[test]
	fn grant_multiple_tagged_permissions_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			let permission = DataPermission::MODIFY;
			let expiry = Some(2000);
			let irrevocable = false;

			let tags_one =
				BoundedVec::try_from(vec![bounded_string("tag-1"), bounded_string("tag-2")])
					.unwrap();

			assert_ok!(SyloDataPermissions::grant_tagged_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				permission,
				tags_one.clone(),
				expiry,
				irrevocable
			));

			let tags_two =
				BoundedVec::try_from(vec![bounded_string("tag-2"), bounded_string("tag-2")])
					.unwrap();

			assert_ok!(SyloDataPermissions::grant_tagged_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				permission,
				tags_two.clone(),
				expiry,
				irrevocable
			));

			let tagged_permissions = TaggedPermissionRecords::<Test>::get(&grantor, &grantee);

			assert!(tagged_permissions
				.iter()
				.find(|(_, record)| record.tags == tags_one)
				.is_some());
			assert!(tagged_permissions
				.iter()
				.find(|(_, record)| record.tags == tags_two)
				.is_some());
		});
	}

	#[test]
	fn cannot_grant_permission_with_invalid_expiry() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			let tags = BoundedVec::try_from(vec![bounded_string("tag-1"), bounded_string("tag-2")])
				.unwrap();

			assert_noop!(
				SyloDataPermissions::grant_tagged_permissions(
					RawOrigin::Signed(grantor.clone()).into(),
					grantee.clone(),
					DataPermission::VIEW,
					tags.clone(),
					Some(0),
					false
				),
				Error::<Test>::InvalidExpiry
			);

			System::set_block_number(100);

			assert_noop!(
				SyloDataPermissions::grant_tagged_permissions(
					RawOrigin::Signed(grantor.clone()).into(),
					grantee.clone(),
					DataPermission::VIEW,
					tags.clone(),
					Some(50),
					false
				),
				Error::<Test>::InvalidExpiry
			);
		});
	}
}

mod revoke_tagged_permission {
	use super::*;

	#[test]
	fn revoke_tagged_permission_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			let tags = BoundedVec::try_from(vec![bounded_string("tag-1"), bounded_string("tag-2")])
				.unwrap();

			let permission = DataPermission::MODIFY;
			let expiry = Some(2000);
			let irrevocable = false;

			assert_ok!(SyloDataPermissions::grant_tagged_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				permission,
				tags.clone(),
				expiry,
				irrevocable
			));

			assert_ok!(SyloDataPermissions::revoke_tagged_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				0,
			));

			System::assert_last_event(MockEvent::SyloDataPermissions(
				crate::Event::TaggedDataPermissionsRevoked {
					revoker: grantor.clone(),
					grantee: grantee.clone(),
					permission,
					tags: tags.clone().iter().map(|v| v.to_vec()).collect(),
				},
			));

			assert_eq!(TaggedPermissionRecords::<Test>::get(&grantor, &grantee).len(), 0);
		});
	}

	#[test]
	fn revoke_from_multiple_tagged_permissions_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			let tags = BoundedVec::try_from(vec![bounded_string("tag-1"), bounded_string("tag-2")])
				.unwrap();

			let permission = DataPermission::MODIFY;
			let expiry = Some(2000);
			let irrevocable = false;

			for _ in 0..3 {
				assert_ok!(SyloDataPermissions::grant_tagged_permissions(
					RawOrigin::Signed(grantor.clone()).into(),
					grantee.clone(),
					permission,
					tags.clone(),
					expiry,
					irrevocable
				));
			}

			assert_ok!(SyloDataPermissions::revoke_tagged_permission(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				1,
			));

			// check records 0 and 2 are still present
			let permission_records = TaggedPermissionRecords::<Test>::get(&grantor, &grantee);

			assert!(permission_records.iter().find(|(i, _)| *i == 0).is_some());
			assert!(permission_records.iter().find(|(i, _)| *i == 2).is_some());
		});
	}

	#[test]
	fn cannot_revoke_missing_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			assert_noop!(
				SyloDataPermissions::revoke_tagged_permission(
					RawOrigin::Signed(grantor.clone()).into(),
					grantee.clone(),
					1,
				),
				Error::<Test>::PermissionNotFound
			);
		});
	}

	#[test]
	fn cannot_revoke_irrevocable_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			let tags = BoundedVec::try_from(vec![bounded_string("tag-1"), bounded_string("tag-2")])
				.unwrap();

			assert_ok!(SyloDataPermissions::grant_tagged_permissions(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				DataPermission::VIEW,
				tags.clone(),
				None,
				true
			));

			assert_noop!(
				SyloDataPermissions::revoke_tagged_permission(
					RawOrigin::Signed(grantor.clone()).into(),
					grantee.clone(),
					0,
				),
				Error::<Test>::PermissionIrrevocable
			);
		});
	}
}

mod grant_permission_reference {
	use super::*;

	#[test]
	fn grant_permission_reference_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// this simulates the accompanying validation record for
			// the offchain permission record
			let permission_record_id =
				create_validation_record(grantor.clone(), "permission-record-id");

			assert_ok!(SyloDataPermissions::grant_permission_reference(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				permission_record_id.clone()
			));

			System::assert_last_event(MockEvent::SyloDataPermissions(
				crate::Event::PermissionReferenceGranted {
					grantor: grantor.clone(),
					grantee: grantee.clone(),
					permission_record_id: permission_record_id.to_vec(),
				},
			));

			let permission_reference =
				PermissionReferences::<Test>::get(&grantor, &grantee).unwrap();

			assert_eq!(permission_reference, PermissionReference { permission_record_id });
		});
	}

	#[test]
	fn cannot_grant_permission_reference_without_validation_record() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			let permission_record_id = bounded_string("permission-record-id");

			assert_noop!(
				SyloDataPermissions::grant_permission_reference(
					RawOrigin::Signed(grantor).into(),
					grantee,
					permission_record_id
				),
				Error::<Test>::MissingValidationRecord
			);
		});
	}
}

mod revoke_permission_reference {
	use super::*;

	#[test]
	fn revoke_permission_reference_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			// this simulates the accompanying validation record for
			// the offchain permission record
			let permission_record_id =
				create_validation_record(grantor.clone(), "permission-record-id");

			assert_ok!(SyloDataPermissions::grant_permission_reference(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
				permission_record_id.clone()
			));

			assert_ok!(SyloDataPermissions::revoke_permission_reference(
				RawOrigin::Signed(grantor.clone()).into(),
				grantee.clone(),
			));

			System::assert_last_event(MockEvent::SyloDataPermissions(
				crate::Event::PermissionReferenceRevoked {
					grantor: grantor.clone(),
					grantee: grantee.clone(),
					permission_record_id: permission_record_id.to_vec(),
				},
			));

			assert!(PermissionReferences::<Test>::get(&grantor, &grantee).is_none());
		});
	}

	#[test]
	fn cannot_revoke_missing_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let grantor: AccountId = create_account(1);
			let grantee: AccountId = create_account(2);

			assert_noop!(
				SyloDataPermissions::revoke_permission_reference(
					RawOrigin::Signed(grantor.clone()).into(),
					grantee.clone(),
				),
				Error::<Test>::PermissionNotFound
			);
		})
	}
}
