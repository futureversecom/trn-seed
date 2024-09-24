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
use mock::*;
use seed_pallet_common::test_prelude::*;

#[test]
fn test_approved_origin_enforced() {
	TestExt::<Test>::default().build().execute_with(|| {
		let relayer_address = b"6490B68F1116BFE87DDD";
		let relayer = AccountId::from(H160::from_slice(relayer_address));
		let account_address = b"6490B68F1116BFE87DDD";
		let account = AccountId::from(H160::from_slice(account_address));
		// Should throw error on un_approved origin
		assert_noop!(XRPLBridge::add_relayer(RuntimeOrigin::signed(account), relayer), BadOrigin);
		// Should work with approved origin
		assert_ok!(XRPLBridge::add_relayer(RuntimeOrigin::root(), relayer));
	})
}

#[test]
fn test_add_relayer_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let relayer_address = b"6490B68F1116BFE87DDD";
		let relayer = AccountId::from(H160::from_slice(relayer_address));
		let _ = XRPLBridge::add_relayer(RuntimeOrigin::root(), relayer);
		assert_eq!(<Relayer<Test>>::iter_values().collect::<Vec<_>>(), vec![true]);

		let relayer_address2 = b"6490B68F1116BFE87DDE";
		let relayer2 = AccountId::from(H160::from_slice(relayer_address2));

		assert_ok!(XRPLBridge::add_relayer(RuntimeOrigin::root(), relayer2));
		assert_eq!(<Relayer<Test>>::iter_values().collect::<Vec<_>>(), vec![true, true]);
	})
}

#[test]
fn test_remove_relayer_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let relayer_address = b"6490B68F1116BFE87DDD";
		let relayer = AccountId::from(H160::from_slice(relayer_address));
		let relayer_address2 = b"6490B68F1116BFE87DDE";
		let relayer2 = AccountId::from(H160::from_slice(relayer_address2));

		let _ = XRPLBridge::add_relayer(RuntimeOrigin::root(), relayer);
		let _ = XRPLBridge::add_relayer(RuntimeOrigin::root(), relayer2);

		// Test removing an existing relayer.
		assert_ok!(XRPLBridge::remove_relayer(RuntimeOrigin::root(), relayer));
		assert_eq!(<Relayer<Test>>::iter_values().collect::<Vec<_>>(), vec![true]);

		// Should throw error if non-existing relayer is tried to removed.
		assert_noop!(
			XRPLBridge::remove_relayer(RuntimeOrigin::root(), relayer),
			Error::<Test>::RelayerDoesNotExists
		);
	})
}

#[test]
fn test_is_relayer_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let relayer_address = b"6490B68F1116BFE87DDD";
		let relayer = AccountId::from(H160::from_slice(relayer_address));
		let relayer_address2 = b"6490B68F1116BFE87DDE";
		let relayer2 = AccountId::from(H160::from_slice(relayer_address2));
		let _ = XRPLBridge::add_relayer(RuntimeOrigin::root(), relayer);
		// Positive test
		assert_eq!(Relayer::<Test>::get(relayer), Some(true));
		// Negative test
		assert_eq!(Relayer::<Test>::get(relayer2), None);
	})
}
