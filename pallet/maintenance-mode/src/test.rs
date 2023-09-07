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

// Unit tests for maintenance mode pallet
// Note these tests do not test the call filtering, integration tests for this can be found in
// runtime/src/tests/maintenance_mode.rs

#![cfg(test)]

use crate::{
	mock::{create_account, MaintenanceMode, RuntimeEvent, System, Test, TestExt},
	BlockedAccounts, BlockedCalls, BlockedEVMAddresses, BlockedPallets, Config,
	MaintenanceModeActive,
};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use sp_core::H160;
use sp_runtime::{traits::BadOrigin, BoundedVec};

pub fn bounded_string(name: &str) -> BoundedVec<u8, <Test as Config>::StringLimit> {
	BoundedVec::truncate_from(name.as_bytes().to_vec())
}

mod enable_maintenance_mode {
	use super::*;

	#[test]
	fn enable_maintenance_mode_updates_storage() {
		TestExt::default().build().execute_with(|| {
			// Enable maintenance mode
			assert_eq!(MaintenanceModeActive::<Test>::get(), false);
			assert_ok!(MaintenanceMode::enable_maintenance_mode(RawOrigin::Root.into(), true));
			assert_eq!(MaintenanceModeActive::<Test>::get(), true);

			// Verify event
			System::assert_last_event(RuntimeEvent::MaintenanceMode(
				crate::Event::MaintenanceModeActivated { enabled: true },
			));

			// Disable maintenance mode
			assert_ok!(MaintenanceMode::enable_maintenance_mode(RawOrigin::Root.into(), false));
			assert_eq!(MaintenanceModeActive::<Test>::get(), false);
		});
	}

	#[test]
	fn enable_maintenance_mode_not_sudo_fails() {
		TestExt::default().build().execute_with(|| {
			let signer = create_account(1);

			// Enable maintenance mode should fail as not root
			assert_noop!(
				MaintenanceMode::enable_maintenance_mode(Some(signer).into(), true),
				BadOrigin
			);
		});
	}
}

mod block_account {
	use super::*;

	#[test]
	fn block_account_updates_storage() {
		TestExt::default().build().execute_with(|| {
			let blocked_account = create_account(2);
			// Enable maintenance mode
			assert_eq!(BlockedAccounts::<Test>::get(blocked_account), false);
			assert_ok!(MaintenanceMode::block_account(
				RawOrigin::Root.into(),
				blocked_account,
				true
			));
			assert_eq!(BlockedAccounts::<Test>::get(blocked_account), true);

			// Verify event
			System::assert_last_event(RuntimeEvent::MaintenanceMode(
				crate::Event::AccountBlocked { account: blocked_account, blocked: true },
			));

			// Disable maintenance mode
			assert_ok!(MaintenanceMode::block_account(
				RawOrigin::Root.into(),
				blocked_account,
				false
			));
			assert_eq!(BlockedAccounts::<Test>::get(blocked_account), false);
		});
	}

	#[test]
	fn block_account_updatesasfasf_storage() {
		TestExt::default().build().execute_with(|| {
			let blocked_account = create_account(2);
			// Enable maintenance mode
			assert_eq!(BlockedAccounts::<Test>::get(blocked_account), false);
			assert_ok!(MaintenanceMode::block_account(
				RawOrigin::Root.into(),
				blocked_account,
				true
			));
			assert_ok!(MaintenanceMode::block_account(
				RawOrigin::Root.into(),
				blocked_account,
				true
			));
			assert_eq!(BlockedAccounts::<Test>::get(blocked_account), true);

			// Verify event
			System::assert_last_event(RuntimeEvent::MaintenanceMode(
				crate::Event::AccountBlocked { account: blocked_account, blocked: true },
			));
		});
	}

	#[test]
	fn block_account_not_sudo_fails() {
		TestExt::default().build().execute_with(|| {
			let signer = create_account(1);
			let blocked_account = create_account(2);

			// Enable maintenance mode should fail as not root
			assert_noop!(
				MaintenanceMode::block_account(Some(signer).into(), blocked_account, true),
				BadOrigin
			);
		});
	}
}

mod block_evm_target {
	use super::*;

	#[test]
	fn block_evm_target_updates_storage() {
		TestExt::default().build().execute_with(|| {
			let blocked_target = H160::from_low_u64_be(2);

			// Enable maintenance mode
			assert_eq!(BlockedEVMAddresses::<Test>::get(blocked_target), false);
			assert_ok!(MaintenanceMode::block_evm_target(
				RawOrigin::Root.into(),
				blocked_target,
				true
			));
			assert_eq!(BlockedEVMAddresses::<Test>::get(blocked_target), true);

			// Verify event
			System::assert_last_event(RuntimeEvent::MaintenanceMode(
				crate::Event::EVMTargetBlocked { target_address: blocked_target, blocked: true },
			));

			// Disable maintenance mode
			assert_ok!(MaintenanceMode::block_evm_target(
				RawOrigin::Root.into(),
				blocked_target,
				false
			));
			assert_eq!(BlockedEVMAddresses::<Test>::get(blocked_target), false);
		});
	}

	#[test]
	fn block_evm_target_not_sudo_fails() {
		TestExt::default().build().execute_with(|| {
			let signer = create_account(1);
			let blocked_target = H160::from_low_u64_be(2);

			// Enable maintenance mode should fail as not root
			assert_noop!(
				MaintenanceMode::block_evm_target(Some(signer).into(), blocked_target, true),
				BadOrigin
			);
		});
	}
}

mod block_call {
	use super::*;
	use crate::Error;
	use sp_runtime::BoundedVec;

	#[test]
	fn block_call_updates_storage() {
		TestExt::default().build().execute_with(|| {
			let blocked_pallet = bounded_string("assets");
			let blocked_call = bounded_string("transfer");

			// Enable maintenance mode
			assert_eq!(BlockedCalls::<Test>::get((&blocked_pallet, &blocked_call)), false);
			assert_ok!(MaintenanceMode::block_call(
				RawOrigin::Root.into(),
				blocked_pallet.clone(),
				blocked_call.clone(),
				true
			));
			assert_eq!(BlockedCalls::<Test>::get((&blocked_pallet, &blocked_call)), true);

			// Verify event
			System::assert_last_event(RuntimeEvent::MaintenanceMode(crate::Event::CallBlocked {
				pallet_name: blocked_pallet.clone(),
				call_name: blocked_call.clone(),
				blocked: true,
			}));

			// Disable maintenance mode
			assert_ok!(MaintenanceMode::block_call(
				RawOrigin::Root.into(),
				blocked_pallet.clone(),
				blocked_call.clone(),
				false
			));
			assert_eq!(BlockedCalls::<Test>::get((&blocked_pallet, &blocked_call)), false);
		});
	}

	#[test]
	fn block_call_not_sudo_fails() {
		TestExt::default().build().execute_with(|| {
			let signer = create_account(1);
			let blocked_pallet = bounded_string("assets");
			let blocked_call = bounded_string("transfer");

			// Block call should fail as not root
			assert_noop!(
				MaintenanceMode::block_call(
					Some(signer).into(),
					blocked_pallet,
					blocked_call,
					true
				),
				BadOrigin
			);
		});
	}

	#[test]
	fn block_maintenance_mode_pallet_call_fails() {
		TestExt::default().build().execute_with(|| {
			let blocked_pallet = bounded_string("MaintenanceMode");
			let blocked_call = bounded_string("block_call");

			// Block call should fail as pallet is maintenance mode
			assert_noop!(
				MaintenanceMode::block_call(
					RawOrigin::Root.into(),
					blocked_pallet.clone(),
					blocked_call.clone(),
					true
				),
				Error::<Test>::CannotBlock
			);

			// Check it fails, even if passing in lowercase pallet name
			let blocked_pallet = bounded_string("maintenancemode");
			let blocked_call = bounded_string("block_pallet");

			assert_noop!(
				MaintenanceMode::block_call(
					RawOrigin::Root.into(),
					blocked_pallet.clone(),
					blocked_call.clone(),
					true
				),
				Error::<Test>::CannotBlock
			);
		});
	}

	#[test]
	fn block_sudo_pallet_call_fails() {
		TestExt::default().build().execute_with(|| {
			let blocked_pallet = bounded_string("Sudo");
			let blocked_call = bounded_string("sudo");

			// Block call should fail as pallet is sudo
			assert_noop!(
				MaintenanceMode::block_call(
					RawOrigin::Root.into(),
					blocked_pallet.clone(),
					blocked_call.clone(),
					true
				),
				Error::<Test>::CannotBlock
			);

			// Check it fails, even if passing in lowercase pallet name
			let blocked_pallet = bounded_string("sudo");
			let blocked_call = bounded_string("sudo_as");

			assert_noop!(
				MaintenanceMode::block_call(
					RawOrigin::Root.into(),
					blocked_pallet.clone(),
					blocked_call.clone(),
					true
				),
				Error::<Test>::CannotBlock
			);
		});
	}

	#[test]
	fn block_call_invalid_pallet_name_fails() {
		TestExt::default().build().execute_with(|| {
			// Invalid pallet name
			let blocked_pallet = BoundedVec::truncate_from(vec![0xfe, 0xff]);
			let blocked_call = bounded_string("block_call");

			// Block call should fail with invalid pallet name
			assert_noop!(
				MaintenanceMode::block_call(
					RawOrigin::Root.into(),
					blocked_pallet.clone(),
					blocked_call.clone(),
					true
				),
				Error::<Test>::InvalidPalletName
			);

			// Empty pallet name
			let blocked_pallet = BoundedVec::truncate_from(vec![]);

			// Block call should fail with empty pallet name
			assert_noop!(
				MaintenanceMode::block_call(
					RawOrigin::Root.into(),
					blocked_pallet.clone(),
					blocked_call.clone(),
					true
				),
				Error::<Test>::InvalidPalletName
			);
		});
	}

	#[test]
	fn block_call_invalid_call_name_fails() {
		TestExt::default().build().execute_with(|| {
			let blocked_pallet = bounded_string("Assets");
			let blocked_call = BoundedVec::truncate_from(vec![0xfe, 0xff]);

			// block_call should fail with invalid call name
			assert_noop!(
				MaintenanceMode::block_call(
					RawOrigin::Root.into(),
					blocked_pallet.clone(),
					blocked_call.clone(),
					true
				),
				Error::<Test>::InvalidCallName
			);

			// Empty call name
			let blocked_call = BoundedVec::truncate_from(vec![]);

			// Block call should fail with empty call name
			assert_noop!(
				MaintenanceMode::block_call(
					RawOrigin::Root.into(),
					blocked_pallet.clone(),
					blocked_call.clone(),
					true
				),
				Error::<Test>::InvalidCallName
			);
		});
	}

	#[test]
	fn block_call_stores_lowercase_names() {
		TestExt::default().build().execute_with(|| {
			let blocked_pallet = bounded_string("ASSETS");
			let blocked_call = bounded_string("TRANSFER");

			// Enable maintenance mode
			assert_ok!(MaintenanceMode::block_call(
				RawOrigin::Root.into(),
				blocked_pallet.clone(),
				blocked_call.clone(),
				true
			),);

			let expected_pallet = bounded_string("assets");
			let expected_call = bounded_string("transfer");
			assert_eq!(BlockedCalls::<Test>::get((&expected_pallet, &expected_call)), true);

			// Try with balances pallet
			let blocked_pallet = bounded_string("Balances");
			let blocked_call = bounded_string("Transfer");

			// Enable maintenance mode
			assert_ok!(MaintenanceMode::block_call(
				RawOrigin::Root.into(),
				blocked_pallet.clone(),
				blocked_call.clone(),
				true
			),);

			let expected_pallet = bounded_string("balances");
			let expected_call = bounded_string("transfer");
			assert_eq!(BlockedCalls::<Test>::get((&expected_pallet, &expected_call)), true);
		});
	}
}

mod block_pallet {
	use super::*;
	use crate::Error;

	#[test]
	fn block_pallet_updates_storage() {
		TestExt::default().build().execute_with(|| {
			let blocked_pallet = bounded_string("assets");

			// Enable maintenance mode
			assert_eq!(BlockedPallets::<Test>::get(&blocked_pallet), false);
			assert_ok!(MaintenanceMode::block_pallet(
				RawOrigin::Root.into(),
				blocked_pallet.clone(),
				true
			));
			assert_eq!(BlockedPallets::<Test>::get(&blocked_pallet), true);

			// Verify event
			System::assert_last_event(RuntimeEvent::MaintenanceMode(crate::Event::PalletBlocked {
				pallet_name: blocked_pallet.clone(),
				blocked: true,
			}));

			// Disable maintenance mode
			assert_ok!(MaintenanceMode::block_pallet(
				RawOrigin::Root.into(),
				blocked_pallet.clone(),
				false
			));
			assert_eq!(BlockedPallets::<Test>::get(&blocked_pallet), false);
		});
	}

	#[test]
	fn block_pallet_not_sudo_fails() {
		TestExt::default().build().execute_with(|| {
			let signer = create_account(1);
			let blocked_pallet = bounded_string("assets");

			// Block call should fail as not root
			assert_noop!(
				MaintenanceMode::block_pallet(Some(signer).into(), blocked_pallet, true),
				BadOrigin
			);
		});
	}

	#[test]
	fn block_maintenance_mode_pallet_fails() {
		TestExt::default().build().execute_with(|| {
			let blocked_pallet = bounded_string("MaintenanceMode");

			// Block call should fail as pallet is maintenance mode
			assert_noop!(
				MaintenanceMode::block_pallet(RawOrigin::Root.into(), blocked_pallet.clone(), true),
				Error::<Test>::CannotBlock
			);

			// Check it fails, even if passing in lowercase pallet name
			let blocked_pallet = bounded_string("maintenancemode");

			assert_noop!(
				MaintenanceMode::block_pallet(RawOrigin::Root.into(), blocked_pallet.clone(), true),
				Error::<Test>::CannotBlock
			);
		});
	}

	#[test]
	fn block_sudo_pallet_fails() {
		TestExt::default().build().execute_with(|| {
			let blocked_pallet = bounded_string("Sudo");

			// Block call should fail as pallet is sudo
			assert_noop!(
				MaintenanceMode::block_pallet(RawOrigin::Root.into(), blocked_pallet.clone(), true),
				Error::<Test>::CannotBlock
			);

			// Check it fails, even if passing in lowercase pallet name
			let blocked_pallet = bounded_string("sudo");

			assert_noop!(
				MaintenanceMode::block_pallet(RawOrigin::Root.into(), blocked_pallet.clone(), true),
				Error::<Test>::CannotBlock
			);
		});
	}

	#[test]
	fn block_pallet_invalid_pallet_name_fails() {
		TestExt::default().build().execute_with(|| {
			// Invalid pallet name
			let blocked_pallet = BoundedVec::truncate_from(vec![0xfe, 0xff]);

			// Block call should fail with invalid pallet name
			assert_noop!(
				MaintenanceMode::block_pallet(RawOrigin::Root.into(), blocked_pallet.clone(), true),
				Error::<Test>::InvalidPalletName
			);

			// Empty pallet name
			let blocked_pallet = BoundedVec::truncate_from(vec![]);

			// Block call should fail with empty pallet name
			assert_noop!(
				MaintenanceMode::block_pallet(RawOrigin::Root.into(), blocked_pallet.clone(), true),
				Error::<Test>::InvalidPalletName
			);
		});
	}

	#[test]
	fn block_pallet_stores_lowercase_names() {
		TestExt::default().build().execute_with(|| {
			let blocked_pallet = bounded_string("ASSETS");

			// Enable maintenance mode
			assert_ok!(MaintenanceMode::block_pallet(
				RawOrigin::Root.into(),
				blocked_pallet.clone(),
				true
			),);

			let expected_pallet = bounded_string("assets");
			assert_eq!(BlockedPallets::<Test>::get(&expected_pallet), true);

			// Try with balances pallet
			let blocked_pallet = bounded_string("Balances");

			// Enable maintenance mode
			assert_ok!(MaintenanceMode::block_pallet(
				RawOrigin::Root.into(),
				blocked_pallet.clone(),
				true
			),);

			let expected_pallet = bounded_string("balances");
			assert_eq!(BlockedPallets::<Test>::get(&expected_pallet), true);
		});
	}
}
