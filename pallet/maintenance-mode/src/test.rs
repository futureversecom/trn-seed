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

#![cfg(test)]

use crate::{
	mock::{create_account, MaintenanceMode, System, SystemError, Test, TestExt},
	BlockedPallets, MaintenanceModeActive,
};
use frame_support::{assert_noop, assert_ok, dispatch::Dispatchable};
use frame_system::RawOrigin;
use seed_primitives::AccountId;
use sp_core::H160;

#[test]
fn enable_maintenance_mode_works() {
	TestExt::default().build().execute_with(|| {
		let signer = create_account(1);

		// Check that system.remark works
		// assert_ok!(System::remark(Some(signer).into(), vec![0, 1, 2, 3]));
		let call = frame_system::Call::<Test>::remark { remark: vec![0, 1, 2, 3] };
		let call = <Test as crate::Config>::Call::from(call);
		assert_ok!(call.dispatch(Some(signer).into()));

		// Enable maintenance mode
		assert_eq!(MaintenanceModeActive::<Test>::get(), false);
		assert_ok!(MaintenanceMode::enable_maintenance_mode(RawOrigin::Root.into(), true));
		assert_eq!(MaintenanceModeActive::<Test>::get(), true);

		// Test remark call
		let call = frame_system::Call::<Test>::remark { remark: vec![0, 1, 2, 3] };
		let call = <Test as crate::Config>::Call::from(call);
		assert_ok!(call.dispatch(Some(signer).into()));

		// Block System pallet
		assert_eq!(BlockedPallets::<Test>::get(b"system".to_vec()), None);
		assert_ok!(MaintenanceMode::block_pallet(RawOrigin::Root.into(), b"System".to_vec(), true));
		assert_eq!(BlockedPallets::<Test>::get(b"system".to_vec()).unwrap(), true);

		let call = frame_system::Call::<Test>::remark { remark: vec![0, 1, 2, 3] };
		let call = <Test as crate::Config>::Call::from(call);
		assert_noop!(call.dispatch(Some(signer).into()), SystemError::CallFiltered);
	});
}
