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

			let permission_record = PermissionRecords::<Test>::get((&grantor, &data_id, &grantee))
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
					PermissionRecords::<Test>::get((&grantor, data_id, &grantee))
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

			let permission_record = PermissionRecords::<Test>::get((&grantor, &data_id, &grantee))
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
		TestExt::<Test>::default().build().execute_with(|| {});
	}

	#[test]
	fn cannot_grant_permission_without_validation_record() {
		TestExt::<Test>::default().build().execute_with(|| {});
	}

	#[test]
	fn can_upgrade_irrevocable_permission() {
		// test existing record remains irrevocable
		TestExt::<Test>::default().build().execute_with(|| {});
	}

	#[test]
	fn cannot_downgrade_irrevocable_permission() {
		TestExt::<Test>::default().build().execute_with(|| {});
	}

	#[test]
	fn can_grant_permission_with_distribute_permission() {
		TestExt::<Test>::default().build().execute_with(|| {});
	}

	#[test]
	fn cannot_be_distributor_without_distribute_permission() {
		TestExt::<Test>::default().build().execute_with(|| {});
	}

	#[test]
	fn cannot_grant_distribute_permission_as_distributor() {
		TestExt::<Test>::default().build().execute_with(|| {});
	}
}
