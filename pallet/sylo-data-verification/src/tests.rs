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

fn create_and_register_resolver(
	identifier: BoundedVec<u8, <Test as Config>::StringLimit>,
	service_endpoints: Vec<BoundedVec<u8, <Test as Config>::StringLimit>>,
) -> (
	AccountId,
	BoundedVec<u8, <Test as Config>::StringLimit>,
	BoundedVec<
		BoundedVec<u8, <Test as Config>::StringLimit>,
		<Test as Config>::MaxServiceEndpoints,
	>,
) {
	let controller: AccountId = create_account(1);

	let service_endpoints =
		BoundedVec::<_, <Test as Config>::MaxServiceEndpoints>::truncate_from(service_endpoints);

	assert_ok!(SyloDataVerification::register_resolver(
		RawOrigin::Signed(controller.clone()).into(),
		identifier.clone(),
		service_endpoints.clone(),
	));

	(controller, identifier, service_endpoints)
}

fn create_initial_validation_record(
	author: <Test as frame_system::Config>::AccountId,
	data_id: &str,
	resolvers: Vec<(&str, &str)>,
	data_type: &str,
	tags: Vec<&str>,
) -> (
	BoundedVec<u8, mock::StringLimit>,
	BoundedVec<ResolverId<mock::StringLimit>, mock::MaxResolvers>,
	BoundedVec<u8, mock::StringLimit>,
	BoundedVec<BoundedVec<u8, mock::StringLimit>, mock::MaxTags>,
	H256,
	ValidationRecord<
		<Test as frame_system::Config>::AccountId,
		BlockNumberFor<Test>,
		mock::MaxResolvers,
		mock::MaxTags,
		mock::MaxEntries,
		mock::StringLimit,
	>,
) {
	let data_id = bounded_string(data_id);
	let resolvers = BoundedVec::truncate_from(
		resolvers
			.iter()
			.map(|(method, identifier)| create_resolver_id(method, identifier))
			.collect(),
	);
	let data_type = bounded_string(data_type);
	let tags = BoundedVec::truncate_from(tags.iter().map(|tag| bounded_string(tag)).collect());
	let checksum = H256::from_low_u64_be(123);

	let record = ValidationRecord {
		author,
		resolvers: resolvers.clone(),
		data_type: data_type.clone(),
		tags: tags.clone(),
		entries: BoundedVec::truncate_from(vec![ValidationEntry {
			checksum,
			block: System::block_number(),
		}]),
	};

	return (data_id, resolvers, data_type, tags, checksum, record);
}

fn create_resolver_id(method: &str, identifier: &str) -> ResolverId<<Test as Config>::StringLimit> {
	ResolverId {
		method: BoundedVec::truncate_from(method.as_bytes().to_vec()),
		identifier: BoundedVec::truncate_from(identifier.as_bytes().to_vec()),
	}
}

fn bounded_string(str: &str) -> BoundedVec<u8, <Test as Config>::StringLimit> {
	BoundedVec::truncate_from(str.as_bytes().to_vec())
}

mod set_payment_asset {
	use super::*;

	#[test]
	fn set_payment_asset_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			assert_ok!(SyloDataVerification::set_payment_asset(RawOrigin::Root.into(), 50));
			assert_eq!(SyloAssetId::<Test>::get(), Some(50));

			// Check event
			System::assert_last_event(MockEvent::SyloDataVerification(
				crate::Event::PaymentAssetSet { asset_id: 50 },
			));
		});
	}

	#[test]
	fn set_payment_asset_not_root_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let new_account: AccountId = create_account(1);

			assert_noop!(
				SyloDataVerification::set_payment_asset(RawOrigin::Signed(new_account).into(), 50),
				BadOrigin
			);
		});
	}
}

mod set_sylo_resolver_method {
	use super::*;

	#[test]
	fn set_sylo_resolver_method_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let method = bounded_string("sylo");

			assert_ok!(SyloDataVerification::set_sylo_resolver_method(
				RawOrigin::Root.into(),
				method.clone()
			));
			assert_eq!(SyloResolverMethod::<Test>::get(), method.clone());

			// Check event
			System::assert_last_event(MockEvent::SyloDataVerification(
				crate::Event::SyloResolverMethodSet { method: method.to_vec() },
			));
		});
	}

	#[test]
	fn set_sylo_resolver_method_not_root_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let new_account: AccountId = create_account(1);

			assert_noop!(
				SyloDataVerification::set_sylo_resolver_method(
					RawOrigin::Signed(new_account).into(),
					bounded_string("sylo")
				),
				BadOrigin
			);
		});
	}
}

mod resolver_registration {
	use super::*;

	#[test]
	fn resolver_registration_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (controller, identifier, service_endpoints) = create_and_register_resolver(
				bounded_string("test-resolver"),
				vec![
					bounded_string("https://endpoint.one"),
					bounded_string("https://endpoint.two"),
				],
			);

			System::assert_last_event(MockEvent::SyloDataVerification(
				Event::<Test>::ResolverRegistered {
					id: identifier.to_vec(),
					controller: controller.clone(),
					service_endpoints: service_endpoints.clone(),
				},
			));

			assert_eq!(
				Resolvers::<Test>::get(identifier).unwrap(),
				Resolver { controller, service_endpoints }
			)
		});
	}

	#[test]
	fn resolver_register_existing_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (controller, identifier, service_endpoints) = create_and_register_resolver(
				bounded_string("test-resolver"),
				vec![
					bounded_string("https://endpoint.one"),
					bounded_string("https://endpoint.two"),
				],
			);

			assert_noop!(
				SyloDataVerification::register_resolver(
					RawOrigin::Signed(controller).into(),
					identifier,
					service_endpoints,
				),
				Error::<Test>::ResolverAlreadyRegistered,
			);
		});
	}
}

mod resolver_update {
	use super::*;

	#[test]
	fn resolver_update_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (controller, identifier, mut service_endpoints) = create_and_register_resolver(
				bounded_string("test-resolver"),
				vec![
					bounded_string("https://endpoint.one"),
					bounded_string("https://endpoint.two"),
				],
			);

			service_endpoints.force_push(bounded_string("https://endpoint.three"));

			assert_ok!(SyloDataVerification::update_resolver(
				RawOrigin::Signed(controller.clone()).into(),
				identifier.clone(),
				service_endpoints.clone(),
			));

			System::assert_last_event(MockEvent::SyloDataVerification(
				Event::<Test>::ResolverUpdated {
					id: identifier.to_vec(),
					controller: controller.clone(),
					service_endpoints: service_endpoints.clone(),
				},
			));

			assert_eq!(
				Resolvers::<Test>::get(identifier).unwrap(),
				Resolver { controller, service_endpoints }
			)
		});
	}

	#[test]
	fn resolver_update_not_existing_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let controller: AccountId = create_account(1);

			let identifier = bounded_string("test-resolver");

			let service_endpoints =
				BoundedVec::<_, <Test as Config>::MaxServiceEndpoints>::truncate_from(vec![]);

			assert_noop!(
				SyloDataVerification::update_resolver(
					RawOrigin::Signed(controller).into(),
					identifier,
					service_endpoints,
				),
				Error::<Test>::ResolverNotRegistered,
			);
		});
	}

	#[test]
	fn resolver_update_not_controller_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (_, identifier, service_endpoints) = create_and_register_resolver(
				bounded_string("test-resolver"),
				vec![
					bounded_string("https://endpoint.one"),
					bounded_string("https://endpoint.two"),
				],
			);

			let not_controller: AccountId = create_account(2);

			assert_noop!(
				SyloDataVerification::update_resolver(
					RawOrigin::Signed(not_controller).into(),
					identifier,
					service_endpoints,
				),
				Error::<Test>::NotController,
			);
		});
	}
}

mod resolver_unregistration {
	use super::*;

	#[test]
	fn resolver_unregistration_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (controller, identifier, _) = create_and_register_resolver(
				bounded_string("test-resolver"),
				vec![
					bounded_string("https://endpoint.one"),
					bounded_string("https://endpoint.two"),
				],
			);

			assert_ok!(SyloDataVerification::deregister_resolver(
				RawOrigin::Signed(controller.clone()).into(),
				identifier.clone(),
			));

			System::assert_last_event(MockEvent::SyloDataVerification(
				Event::<Test>::ResolverDeregistered { id: identifier.to_vec() },
			));

			assert!(Resolvers::<Test>::get(identifier).is_none());
		});
	}

	#[test]
	fn resolver_deregister_not_existing_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let controller: AccountId = create_account(1);

			let identifier = bounded_string("test-resolver");

			assert_noop!(
				SyloDataVerification::deregister_resolver(
					RawOrigin::Signed(controller).into(),
					identifier,
				),
				Error::<Test>::ResolverNotRegistered,
			);
		});
	}

	#[test]
	fn resolver_deregister_not_controller_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (_, identifier, _) = create_and_register_resolver(
				bounded_string("test-resolver"),
				vec![
					bounded_string("https://endpoint.one"),
					bounded_string("https://endpoint.two"),
				],
			);

			let not_controller: AccountId = create_account(2);

			assert_noop!(
				SyloDataVerification::deregister_resolver(
					RawOrigin::Signed(not_controller).into(),
					identifier,
				),
				Error::<Test>::NotController,
			);
		});
	}
}

mod create_validation_record {
	use core::str;

	use sp_core::hexdisplay::AsBytesRef;

	use super::*;

	#[test]
	fn create_validation_records_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let alice: AccountId = create_account(2);

			let (data_id, resolvers, data_type, tags, checksum, record) =
				create_initial_validation_record(
					alice,
					"data_id",
					vec![("method-1", "resolver-1")],
					"data_type",
					vec!["tag-1", "tag-2"],
				);

			assert_ok!(SyloDataVerification::create_validation_record(
				RawOrigin::Signed(alice.clone()).into(),
				data_id.clone(),
				resolvers.clone(),
				data_type.clone(),
				tags.clone(),
				checksum.clone()
			));

			System::assert_last_event(MockEvent::SyloDataVerification(
				Event::<Test>::ValidationRecordCreated {
					author: alice.clone(),
					id: data_id.clone().to_vec(),
				},
			));

			assert_eq!(
				ValidationRecords::<Test>::get(alice.clone(), data_id.clone()).unwrap(),
				record
			);
		});
	}

	#[test]
	fn create_existing_validation_record_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let alice: AccountId = create_account(2);

			let (data_id, resolvers, data_type, tags, checksum, _) =
				create_initial_validation_record(
					alice,
					"data_id",
					vec![("method-1", "resolver-1")],
					"data_type",
					vec!["tag-1", "tag-2"],
				);

			assert_ok!(SyloDataVerification::create_validation_record(
				RawOrigin::Signed(alice.clone()).into(),
				data_id.clone(),
				resolvers.clone(),
				data_type.clone(),
				tags.clone(),
				checksum.clone()
			));

			assert_noop!(
				SyloDataVerification::create_validation_record(
					RawOrigin::Signed(alice.clone()).into(),
					data_id.clone(),
					resolvers.clone(),
					data_type.clone(),
					tags.clone(),
					checksum.clone()
				),
				Error::<Test>::RecordAlreadyCreated
			);
		});
	}

	#[test]
	fn create_validation_records_with_sylo_resolvers_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Ensure sylo resolver is registered
			let (_, identifier, _) = create_and_register_resolver(
				bounded_string("test-resolver"),
				vec![bounded_string("https://endpoint.one")],
			);

			let alice: AccountId = create_account(2);

			let (data_id, resolvers, data_type, tags, checksum, record) =
				create_initial_validation_record(
					alice,
					"data_id",
					vec![(
						str::from_utf8(SyloResolverMethod::<Test>::get().as_bytes_ref()).unwrap(),
						str::from_utf8(identifier.to_vec().as_bytes_ref()).unwrap(),
					)],
					"data_type",
					vec!["tag-1", "tag-2"],
				);

			assert_ok!(SyloDataVerification::create_validation_record(
				RawOrigin::Signed(alice.clone()).into(),
				data_id.clone(),
				resolvers.clone(),
				data_type.clone(),
				tags.clone(),
				checksum.clone()
			));

			System::assert_last_event(MockEvent::SyloDataVerification(
				Event::<Test>::ValidationRecordCreated {
					author: alice.clone(),
					id: data_id.clone().to_vec(),
				},
			));

			assert_eq!(
				ValidationRecords::<Test>::get(alice.clone(), data_id.clone()).unwrap(),
				record
			);
		});
	}

	#[test]
	fn create_validation_record_with_deregistered_sylo_resolver_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let alice: AccountId = create_account(2);

			let (data_id, resolvers, data_type, tags, checksum, _) =
				create_initial_validation_record(
					alice,
					"data_id",
					vec![(
						str::from_utf8(SyloResolverMethod::<Test>::get().as_bytes_ref()).unwrap(),
						// identifier references a non-existent resolver
						"deregistered-resolver",
					)],
					"data_type",
					vec!["tag-1", "tag-2"],
				);

			assert_noop!(
				SyloDataVerification::create_validation_record(
					RawOrigin::Signed(alice.clone()).into(),
					data_id.clone(),
					resolvers.clone(),
					data_type.clone(),
					tags.clone(),
					checksum.clone()
				),
				Error::<Test>::ResolverNotRegistered
			);
		});
	}

	#[test]
	fn create_multiple_validation_records_with_same_author_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let alice: AccountId = create_account(2);

			for i in 1..5 {
				let (data_id, resolvers, data_type, tags, checksum, record) =
					create_initial_validation_record(
						alice,
						format!("data_id_{i}").as_str(),
						vec![("method-1", "resolver-1")],
						"data_type",
						vec!["tag-1", "tag-2"],
					);

				assert_ok!(SyloDataVerification::create_validation_record(
					RawOrigin::Signed(alice.clone()).into(),
					data_id.clone(),
					resolvers.clone(),
					data_type.clone(),
					tags.clone(),
					checksum.clone()
				));

				System::assert_last_event(MockEvent::SyloDataVerification(
					Event::<Test>::ValidationRecordCreated {
						author: alice.clone(),
						id: data_id.clone().to_vec(),
					},
				));

				assert_eq!(
					ValidationRecords::<Test>::get(alice.clone(), data_id.clone()).unwrap(),
					record
				);
			}
		});
	}

	#[test]
	fn create_validation_records_with_different_author_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			for i in 2..5 {
				let author: AccountId = create_account(i);

				let (data_id, resolvers, data_type, tags, checksum, record) =
					create_initial_validation_record(
						author,
						// use the same data id for each author's validation record
						format!("data_id").as_str(),
						vec![("method-1", "resolver-1")],
						"data_type",
						vec!["tag-1", "tag-2"],
					);

				assert_ok!(SyloDataVerification::create_validation_record(
					RawOrigin::Signed(author.clone()).into(),
					data_id.clone(),
					resolvers.clone(),
					data_type.clone(),
					tags.clone(),
					checksum.clone()
				));

				System::assert_last_event(MockEvent::SyloDataVerification(
					Event::<Test>::ValidationRecordCreated {
						author: author.clone(),
						id: data_id.clone().to_vec(),
					},
				));

				assert_eq!(
					ValidationRecords::<Test>::get(author.clone(), data_id.clone()).unwrap(),
					record
				);
			}
		});
	}
}

mod add_validation_record_entry {
	use super::*;

	#[test]
	fn add_validation_entry_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let alice: AccountId = create_account(2);

			let (data_id, resolvers, data_type, tags, checksum, _) =
				create_initial_validation_record(
					alice,
					"data_id",
					vec![("method-1", "resolver-1")],
					"data_type",
					vec!["tag-1", "tag-2"],
				);

			assert_ok!(SyloDataVerification::create_validation_record(
				RawOrigin::Signed(alice.clone()).into(),
				data_id.clone(),
				resolvers.clone(),
				data_type.clone(),
				tags.clone(),
				checksum.clone()
			));

			for i in 2..5 {
				let checksum = H256::from_low_u64_be(i);

				assert_ok!(SyloDataVerification::add_validation_record_entry(
					RawOrigin::Signed(alice.clone()).into(),
					alice.clone(),
					data_id.clone(),
					checksum.clone()
				));

				System::assert_last_event(MockEvent::SyloDataVerification(
					Event::<Test>::ValidationEntryAdded {
						author: alice.clone(),
						id: data_id.clone().to_vec(),
						checksum,
					},
				));

				let record =
					ValidationRecords::<Test>::get(alice.clone(), data_id.clone()).unwrap();

				assert!(record.entries.len() as u64 == i);
				assert!(record.entries.last().unwrap().checksum == checksum);
			}
		});
	}

	#[test]
	fn add_not_existing_validation_entry_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let alice: AccountId = create_account(2);

			let (data_id, _, _, _, checksum, _) =
				create_initial_validation_record(alice, "data_id", vec![], "data_type", vec![]);

			assert_noop!(
				SyloDataVerification::add_validation_record_entry(
					RawOrigin::Signed(alice.clone()).into(),
					alice.clone(),
					data_id.clone(),
					checksum.clone()
				),
				Error::<Test>::NoValidationRecord
			);
		});
	}

	#[test]
	fn only_author_or_modifier_can_add_validation_entry() {
		TestExt::<Test>::default().build().execute_with(|| {
			let alice: AccountId = create_account(2);

			let (data_id, resolvers, data_type, tags, checksum, _) =
				create_initial_validation_record(
					alice,
					"data_id",
					vec![("method-1", "resolver-1")],
					"data_type",
					vec!["tag-1", "tag-2"],
				);

			assert_ok!(SyloDataVerification::create_validation_record(
				RawOrigin::Signed(alice.clone()).into(),
				data_id.clone(),
				resolvers.clone(),
				data_type.clone(),
				tags.clone(),
				checksum.clone()
			));

			let bob: AccountId = create_account(3);

			assert_noop!(
				SyloDataVerification::add_validation_record_entry(
					RawOrigin::Signed(bob.clone()).into(),
					alice.clone(),
					data_id,
					checksum
				),
				Error::<Test>::MissingModifyPermission
			);
		});
	}

	#[test]
	fn add_validation_entry_as_modifier_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let alice: AccountId = create_account(2);

			let (data_id, resolvers, data_type, tags, checksum, _) =
				create_initial_validation_record(
					alice,
					"data_id",
					vec![("method-1", "resolver-1")],
					"data_type",
					vec!["tag-1", "tag-2"],
				);

			assert_ok!(SyloDataVerification::create_validation_record(
				RawOrigin::Signed(alice.clone()).into(),
				data_id.clone(),
				resolvers.clone(),
				data_type.clone(),
				tags.clone(),
				checksum.clone()
			));

			let bob: AccountId = create_account(3);

			// grant bob the MODIFY permission
			assert_ok!(SyloDataPermissions::grant_data_permissions(
				RawOrigin::Signed(alice.clone()).into(),
				alice.clone(),
				bob.clone(),
				BoundedVec::truncate_from(vec![data_id.clone()]),
				DataPermission::MODIFY,
				None,
				false
			));

			assert_ok!(SyloDataVerification::add_validation_record_entry(
				RawOrigin::Signed(bob.clone()).into(),
				alice.clone(),
				data_id.clone(),
				checksum.clone()
			));
		});
	}

	#[test]
	fn add_validation_entry_as_modifier_via_tagged_permission_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let alice: AccountId = create_account(2);

			let (data_id, resolvers, data_type, tags, checksum, _) =
				create_initial_validation_record(
					alice,
					"data_id",
					vec![("method-1", "resolver-1")],
					"data_type",
					vec!["tag-1", "tag-2"],
				);

			assert_ok!(SyloDataVerification::create_validation_record(
				RawOrigin::Signed(alice.clone()).into(),
				data_id.clone(),
				resolvers.clone(),
				data_type.clone(),
				tags.clone(),
				checksum.clone()
			));

			let bob: AccountId = create_account(3);

			// grant bob the MODIFY permission using tags
			assert_ok!(SyloDataPermissions::grant_tagged_permissions(
				RawOrigin::Signed(alice.clone()).into(),
				bob.clone(),
				DataPermission::MODIFY,
				tags.clone(),
				None,
				false
			));

			assert_ok!(SyloDataVerification::add_validation_record_entry(
				RawOrigin::Signed(bob.clone()).into(),
				alice.clone(),
				data_id.clone(),
				checksum.clone()
			));
		});
	}

	#[test]
	fn modifier_must_have_modify_permission() {
		TestExt::<Test>::default().build().execute_with(|| {
			let alice: AccountId = create_account(2);

			let (data_id, resolvers, data_type, tags, checksum, _) =
				create_initial_validation_record(
					alice,
					"data_id",
					vec![("method-1", "resolver-1")],
					"data_type",
					vec!["tag-1", "tag-2"],
				);

			assert_ok!(SyloDataVerification::create_validation_record(
				RawOrigin::Signed(alice.clone()).into(),
				data_id.clone(),
				resolvers.clone(),
				data_type.clone(),
				tags.clone(),
				checksum.clone()
			));

			let bob: AccountId = create_account(3);

			// grant bob the VIEW and DISTRIBUTE permissions
			assert_ok!(SyloDataPermissions::grant_tagged_permissions(
				RawOrigin::Signed(alice.clone()).into(),
				bob.clone(),
				DataPermission::VIEW,
				tags.clone(),
				None,
				false
			));
			assert_ok!(SyloDataPermissions::grant_tagged_permissions(
				RawOrigin::Signed(alice.clone()).into(),
				bob.clone(),
				DataPermission::DISTRIBUTE,
				tags.clone(),
				None,
				false
			));

			assert_noop!(
				SyloDataVerification::add_validation_record_entry(
					RawOrigin::Signed(bob.clone()).into(),
					alice.clone(),
					data_id,
					checksum
				),
				Error::<Test>::MissingModifyPermission
			);
		});
	}
}

mod update_validation_record {
	use super::*;

	#[test]
	fn update_validation_record_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let alice: AccountId = create_account(2);

			let (data_id, resolvers, data_type, tags, checksum, record) =
				create_initial_validation_record(
					alice,
					"data_id",
					vec![("method-1", "resolver-1")],
					"data_type",
					vec!["tag-1", "tag-2"],
				);

			assert_ok!(SyloDataVerification::create_validation_record(
				RawOrigin::Signed(alice.clone()).into(),
				data_id.clone(),
				resolvers.clone(),
				data_type.clone(),
				tags.clone(),
				checksum.clone()
			));

			let (_, new_resolvers, new_data_type, new_tags, _, _) =
				create_initial_validation_record(
					alice,
					"data_id",
					// add anotehr resolver
					vec![("method-1", "resolver-1"), ("method-2", "resolver-2")],
					// modify data type
					"data_type_2",
					// add more tags
					vec!["tag-1", "tag-2", "tag-3"],
				);

			// Update the list of resolvers
			assert_ok!(SyloDataVerification::update_validation_record(
				RawOrigin::Signed(alice.clone()).into(),
				data_id.clone(),
				Some(new_resolvers.clone()),
				None,
				None
			));

			assert_eq!(
				ValidationRecords::<Test>::get(alice.clone(), data_id.clone()).unwrap(),
				ValidationRecord {
					author: alice.clone(),
					resolvers: new_resolvers.clone(),
					data_type: data_type.clone(),
					tags: tags.clone(),
					entries: record.entries.clone(),
				}
			);

			System::assert_last_event(MockEvent::SyloDataVerification(
				Event::<Test>::ValidationRecordUpdated {
					author: alice.clone(),
					id: data_id.clone().to_vec(),
					resolvers: Some(
						new_resolvers.clone().iter().map(|resolver| resolver.to_did()).collect(),
					),
					data_type: None,
					tags: None,
				},
			));

			// Update the data type
			assert_ok!(SyloDataVerification::update_validation_record(
				RawOrigin::Signed(alice.clone()).into(),
				data_id.clone(),
				None,
				Some(new_data_type.clone()),
				None
			));

			assert_eq!(
				ValidationRecords::<Test>::get(alice.clone(), data_id.clone()).unwrap(),
				ValidationRecord {
					author: alice.clone(),
					resolvers: new_resolvers.clone(),
					data_type: new_data_type.clone(),
					tags: tags.clone(),
					entries: record.entries.clone(),
				}
			);

			System::assert_last_event(MockEvent::SyloDataVerification(
				Event::<Test>::ValidationRecordUpdated {
					author: alice.clone(),
					id: data_id.clone().to_vec(),
					resolvers: None,
					data_type: Some(new_data_type.clone().to_vec()),
					tags: None,
				},
			));

			// Update the list of tags
			assert_ok!(SyloDataVerification::update_validation_record(
				RawOrigin::Signed(alice.clone()).into(),
				data_id.clone(),
				None,
				None,
				Some(new_tags.clone()),
			));

			assert_eq!(
				ValidationRecords::<Test>::get(alice.clone(), data_id.clone()).unwrap(),
				ValidationRecord {
					author: alice.clone(),
					resolvers: new_resolvers.clone(),
					data_type: new_data_type.clone(),
					tags: new_tags.clone(),
					entries: record.entries.clone(),
				}
			);

			System::assert_last_event(MockEvent::SyloDataVerification(
				Event::<Test>::ValidationRecordUpdated {
					author: alice.clone(),
					id: data_id.clone().to_vec(),
					resolvers: None,
					data_type: None,
					tags: Some(new_tags.iter().map(|tag| tag.to_vec()).collect()),
				},
			));
		});
	}

	#[test]
	fn update_not_existing_validation_record_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let alice: AccountId = create_account(2);

			let (data_id, resolvers, data_type, tags, _, _) = create_initial_validation_record(
				alice,
				"data_id",
				vec![("method-1", "resolver-1")],
				"data_type",
				vec!["tag-1", "tag-2"],
			);

			assert_noop!(
				SyloDataVerification::update_validation_record(
					RawOrigin::Signed(alice.clone()).into(),
					data_id.clone(),
					Some(resolvers.clone()),
					Some(data_type.clone()),
					Some(tags.clone()),
				),
				Error::<Test>::NoValidationRecord
			);
		});
	}

	#[test]
	fn only_author_can_update_validation_record() {
		TestExt::<Test>::default().build().execute_with(|| {
			let alice: AccountId = create_account(2);

			let (data_id, resolvers, data_type, tags, checksum, _) =
				create_initial_validation_record(
					alice,
					"data_id",
					vec![("method-1", "resolver-1")],
					"data_type",
					vec!["tag-1", "tag-2"],
				);

			assert_ok!(SyloDataVerification::create_validation_record(
				RawOrigin::Signed(alice.clone()).into(),
				data_id.clone(),
				resolvers.clone(),
				data_type.clone(),
				tags.clone(),
				checksum.clone()
			));

			let bob: AccountId = create_account(3);

			assert_noop!(
				SyloDataVerification::update_validation_record(
					RawOrigin::Signed(bob.clone()).into(),
					data_id.clone(),
					Some(resolvers.clone()),
					Some(data_type.clone()),
					Some(tags.clone()),
				),
				Error::<Test>::NoValidationRecord
			);
		});
	}
}

mod delete_validation_record {
	use super::*;

	#[test]
	fn delete_validation_record_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let alice: AccountId = create_account(2);

			let (data_id, resolvers, data_type, tags, checksum, _) =
				create_initial_validation_record(
					alice,
					"data_id",
					vec![("method-1", "resolver-1")],
					"data_type",
					vec!["tag-1", "tag-2"],
				);

			assert_ok!(SyloDataVerification::create_validation_record(
				RawOrigin::Signed(alice.clone()).into(),
				data_id.clone(),
				resolvers.clone(),
				data_type.clone(),
				tags.clone(),
				checksum.clone()
			));

			assert_ok!(SyloDataVerification::delete_validation_record(
				RawOrigin::Signed(alice.clone()).into(),
				data_id.clone(),
			));
		});
	}

	#[test]
	fn delete_not_existing_validation_record_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let alice: AccountId = create_account(2);

			let (data_id, resolvers, data_type, tags, _, _) = create_initial_validation_record(
				alice,
				"data_id",
				vec![("method-1", "resolver-1")],
				"data_type",
				vec!["tag-1", "tag-2"],
			);

			assert_noop!(
				SyloDataVerification::update_validation_record(
					RawOrigin::Signed(alice.clone()).into(),
					data_id.clone(),
					Some(resolvers.clone()),
					Some(data_type.clone()),
					Some(tags.clone()),
				),
				Error::<Test>::NoValidationRecord
			);
		});
	}

	#[test]
	fn only_author_can_delete_validation_record() {
		TestExt::<Test>::default().build().execute_with(|| {
			let alice: AccountId = create_account(2);

			let (data_id, resolvers, data_type, tags, checksum, _) =
				create_initial_validation_record(
					alice,
					"data_id",
					vec![("method-1", "resolver-1")],
					"data_type",
					vec!["tag-1", "tag-2"],
				);

			assert_ok!(SyloDataVerification::create_validation_record(
				RawOrigin::Signed(alice.clone()).into(),
				data_id.clone(),
				resolvers.clone(),
				data_type.clone(),
				tags.clone(),
				checksum.clone()
			));

			let bob: AccountId = create_account(3);

			assert_noop!(
				SyloDataVerification::delete_validation_record(
					RawOrigin::Signed(bob.clone()).into(),
					data_id.clone(),
				),
				Error::<Test>::NoValidationRecord
			);
		});
	}
}
