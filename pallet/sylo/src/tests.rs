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
use hex::encode;
use mock::{RuntimeEvent as MockEvent, RuntimeOrigin, Sylo, System, Test, TestExt};
use seed_pallet_common::test_prelude::*;
use sp_arithmetic::helpers_128bit::sqrt;

mod resolvers {
	use super::*;

	#[test]
	fn resolver_registration_works() {
		TestExt.build().execute_with(|| {
			let (controller, identifier, service_endpoints) = create_and_register_resolver(
				bounded_string("test-resolver"),
				vec![
					bounded_string("https://endpoint.one"),
					bounded_string("https://endpoint.two"),
				],
			);

			System::assert_last_event(MockEvent::Sylo(Event::<Test>::ResolverRegistered {
				id: identifier.to_vec(),
				controller: controller.clone(),
				service_endpoints: service_endpoints.clone(),
			}));

			assert_eq!(
				Resolvers::<Test>::get(identifier).unwrap(),
				Resolver { controller, service_endpoints }
			)
		});
	}

	#[test]
	fn resolver_update_works() {
		TestExt.build().execute_with(|| {
			let (controller, identifier, mut service_endpoints) = create_and_register_resolver(
				bounded_string("test-resolver"),
				vec![
					bounded_string("https://endpoint.one"),
					bounded_string("https://endpoint.two"),
				],
			);

			service_endpoints.force_push(bounded_string("https://endpoint.three"));

			assert_ok!(Sylo::update_resolver(
				RawOrigin::Signed(controller.clone()).into(),
				identifier.clone(),
				service_endpoints.clone(),
			));

			System::assert_last_event(MockEvent::Sylo(Event::<Test>::ResolverUpdated {
				id: identifier.to_vec(),
				controller: controller.clone(),
				service_endpoints: service_endpoints.clone(),
			}));

			assert_eq!(
				Resolvers::<Test>::get(identifier).unwrap(),
				Resolver { controller, service_endpoints }
			)
		});
	}

	#[test]
	fn resolver_unregistration_works() {
		TestExt.build().execute_with(|| {
			let (controller, identifier, mut service_endpoints) = create_and_register_resolver(
				bounded_string("test-resolver"),
				vec![
					bounded_string("https://endpoint.one"),
					bounded_string("https://endpoint.two"),
				],
			);

			assert_ok!(Sylo::unregister_resolver(
				RawOrigin::Signed(controller.clone()).into(),
				identifier.clone(),
			));

			System::assert_last_event(MockEvent::Sylo(Event::<Test>::ResolverUnregistered {
				id: identifier.to_vec(),
			}));

			assert!(Resolvers::<Test>::get(identifier).is_none());
		});
	}

	#[test]
	fn resolver_register_existing_fails() {
		TestExt.build().execute_with(|| {
			let (controller, identifier, mut service_endpoints) = create_and_register_resolver(
				bounded_string("test-resolver"),
				vec![
					bounded_string("https://endpoint.one"),
					bounded_string("https://endpoint.two"),
				],
			);

			assert_noop!(
				Sylo::register_resolver(
					RawOrigin::Signed(controller).into(),
					identifier,
					service_endpoints,
				),
				Error::<Test>::ResolverAlreadyRegistered,
			);
		});
	}

	#[test]
	fn resolver_update_not_existing_fails() {
		TestExt.build().execute_with(|| {
			let controller: AccountId = create_account(1);

			let identifier = bounded_string("test-resolver");

			let service_endpoints =
				BoundedVec::<_, <Test as Config>::MaxServiceEndpoints>::try_from(vec![]).unwrap();

			assert_noop!(
				Sylo::update_resolver(
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
		TestExt.build().execute_with(|| {
			let (controller, identifier, service_endpoints) = create_and_register_resolver(
				bounded_string("test-resolver"),
				vec![
					bounded_string("https://endpoint.one"),
					bounded_string("https://endpoint.two"),
				],
			);

			let not_controller: AccountId = create_account(2);

			assert_noop!(
				Sylo::update_resolver(
					RawOrigin::Signed(not_controller).into(),
					identifier,
					service_endpoints,
				),
				Error::<Test>::NotController,
			);
		});
	}

	#[test]
	fn resolver_unregister_not_existing_fails() {
		TestExt.build().execute_with(|| {
			let controller: AccountId = create_account(1);

			let identifier = bounded_string("test-resolver");

			assert_noop!(
				Sylo::unregister_resolver(RawOrigin::Signed(controller).into(), identifier,),
				Error::<Test>::ResolverNotRegistered,
			);
		});
	}

	#[test]
	fn resolver_unregister_not_controller_fails() {
		TestExt.build().execute_with(|| {
			let (controller, identifier, service_endpoints) = create_and_register_resolver(
				bounded_string("test-resolver"),
				vec![
					bounded_string("https://endpoint.one"),
					bounded_string("https://endpoint.two"),
				],
			);

			let not_controller: AccountId = create_account(2);

			assert_noop!(
				Sylo::unregister_resolver(RawOrigin::Signed(not_controller).into(), identifier,),
				Error::<Test>::NotController,
			);
		});
	}
}

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
		BoundedVec::<_, <Test as Config>::MaxServiceEndpoints>::try_from(service_endpoints)
			.unwrap();

	assert_ok!(Sylo::register_resolver(
		RawOrigin::Signed(controller.clone()).into(),
		identifier.clone(),
		service_endpoints.clone(),
	));

	(controller, identifier, service_endpoints)
}

fn bounded_string(name: &str) -> BoundedVec<u8, <Test as Config>::StringLimit> {
	BoundedVec::truncate_from(name.as_bytes().to_vec())
}
