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

//! Integration tests for maintenance mode pallet.
#![cfg(test)]

use crate::{
	tests::{alice, bob, charlie, sign_xt, signed_extra, ExtBuilder},
	CheckedExtrinsic, Executive, MaintenanceMode, Runtime, RuntimeCall,
};
use ethabi::Token;
use frame_support::{assert_err, assert_noop, assert_ok, dispatch::RawOrigin};
use pallet_maintenance_mode::MaintenanceModeActive;
use pallet_token_approvals::ERC20Approvals;
use precompile_utils::{constants::ERC20_PRECOMPILE_ADDRESS_PREFIX, ErcIdConversion};
use seed_primitives::{AssetId, Balance};
use sp_core::{H160, H256, U256};
use sp_runtime::{
	traits::Dispatchable,
	transaction_validity::{InvalidTransaction, TransactionValidityError},
	BoundedVec,
};

type SystemError = frame_system::Error<Runtime>;

pub fn bounded_string(
	name: &str,
) -> BoundedVec<u8, <Runtime as pallet_maintenance_mode::Config>::StringLimit> {
	BoundedVec::truncate_from(name.as_bytes().to_vec())
}

mod enable_maintenance_mode {
	use super::*;

	#[test]
	fn enable_maintenance_mode_works() {
		ExtBuilder::default().build().execute_with(|| {
			let signer = bob();

			// Enable maintenance mode
			assert_ok!(MaintenanceMode::enable_maintenance_mode(RawOrigin::Root.into(), true));

			// send signed transaction should fail as we are in maintenance mode
			let xt = sign_xt(CheckedExtrinsic {
				signed: fp_self_contained::CheckedSignature::Signed(signer, signed_extra(0, 0)),
				function: RuntimeCall::System(frame_system::Call::remark {
					remark: b"hello blocked chain".to_vec(),
				}),
			});
			assert_err!(
				Executive::apply_extrinsic(xt),
				TransactionValidityError::Invalid(InvalidTransaction::Custom(1))
			);

			// Disable maintenance mode
			assert_ok!(MaintenanceMode::enable_maintenance_mode(RawOrigin::Root.into(), false));

			// RuntimeCall should now succeed
			let xt2 = sign_xt(CheckedExtrinsic {
				signed: fp_self_contained::CheckedSignature::Signed(signer, signed_extra(1, 0)),
				function: RuntimeCall::System(frame_system::Call::remark {
					remark: b"hello unblocked chain".to_vec(),
				}),
			});
			assert_ok!(Executive::apply_extrinsic(xt2));
		});
	}

	#[test]
	fn maintenance_mode_can_be_disabled_by_sudo() {
		ExtBuilder::default().build().execute_with(|| {
			let signer = alice();

			// Enable maintenance mode by calling extrinsic directly
			assert_ok!(MaintenanceMode::enable_maintenance_mode(RawOrigin::Root.into(), true));
			assert!(MaintenanceModeActive::<Runtime>::get());

			// Send signed tx to disable maintenance mode
			let call = RuntimeCall::MaintenanceMode(
				pallet_maintenance_mode::Call::enable_maintenance_mode { enabled: false },
			);
			let xt2 = sign_xt(CheckedExtrinsic {
				signed: fp_self_contained::CheckedSignature::Signed(signer, signed_extra(0, 0)),
				function: RuntimeCall::Sudo(pallet_sudo::Call::sudo { call: Box::new(call) }),
			});
			assert_ok!(Executive::apply_extrinsic(xt2));

			// Maintenance mode disabled
			assert!(!MaintenanceModeActive::<Runtime>::get());
		});
	}

	#[test]
	fn maintenance_mode_works_with_evm() {
		let payment_asset: AssetId = 2;
		let target: H160 = <Runtime as ErcIdConversion<AssetId>>::runtime_id_to_evm_id(
			payment_asset,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.into();
		ExtBuilder::default()
			.accounts_to_fund(&[target.into()])
			.build()
			.execute_with(|| {
				let signer = alice();

				// Setup input for an erc20 approve
				let mut input: Vec<u8> = [0x09, 0x5e, 0xa7, 0xb3].to_vec();
				let approve_amount: Balance = 12345;
				input.append(&mut ethabi::encode(&[
					Token::Address(bob().into()),
					Token::Uint(approve_amount.into()),
				]));

				// Setup inner EVM.call call
				let access_list: Vec<(H160, Vec<H256>)> = vec![];
				let call = crate::RuntimeCall::EVM(pallet_evm::Call::call {
					source: signer.into(),
					target,
					input,
					value: U256::zero(),
					gas_limit: 50_000,
					max_fee_per_gas: U256::from(1_600_000_000_000_000_u64),
					max_priority_fee_per_gas: None,
					nonce: None,
					access_list,
				});

				// Enable maintenance mode
				assert_ok!(MaintenanceMode::enable_maintenance_mode(RawOrigin::Root.into(), true));

				// EVM call should fail
				assert_eq!(
					call.clone().dispatch(Some(signer).into()).unwrap_err().error,
					pallet_evm::Error::<Runtime>::WithdrawFailed.into()
				);
				// The storage should not have been updated in TokenApprovals pallet
				assert_eq!(ERC20Approvals::<Runtime>::get((&signer, payment_asset), bob()), None);

				// Disable maintenance mode
				assert_ok!(MaintenanceMode::enable_maintenance_mode(RawOrigin::Root.into(), false));

				// EVM call should now work
				assert_ok!(call.dispatch(Some(signer).into()));
				assert_eq!(
					ERC20Approvals::<Runtime>::get((&signer, payment_asset), bob()),
					Some(approve_amount)
				);
			});
	}
}

mod block_account {
	use super::*;

	#[test]
	fn block_account_works() {
		ExtBuilder::default().build().execute_with(|| {
			let signer = alice();

			// Block signer account
			assert_ok!(MaintenanceMode::block_account(RawOrigin::Root.into(), signer, true));

			// send signed transaction should fail as we have blocked the account
			let function =
				RuntimeCall::System(frame_system::Call::remark { remark: b"hello chain".to_vec() });
			let xt = sign_xt(CheckedExtrinsic {
				signed: fp_self_contained::CheckedSignature::Signed(signer, signed_extra(0, 0)),
				function: function.clone(),
			});
			assert_err!(
				Executive::apply_extrinsic(xt),
				TransactionValidityError::Invalid(InvalidTransaction::Custom(2))
			);

			// A non blocked account should still be able to make the call
			let xt = sign_xt(CheckedExtrinsic {
				signed: fp_self_contained::CheckedSignature::Signed(bob(), signed_extra(0, 0)),
				function: function.clone(),
			});
			assert_ok!(Executive::apply_extrinsic(xt),);

			// Unblock account
			assert_ok!(MaintenanceMode::block_account(RawOrigin::Root.into(), signer, false));

			// RuntimeCall should now succeed
			let xt = sign_xt(CheckedExtrinsic {
				signed: fp_self_contained::CheckedSignature::Signed(signer, signed_extra(1, 0)),
				function,
			});
			assert_ok!(Executive::apply_extrinsic(xt));
		});
	}

	#[test]
	fn block_account_works_with_evm_call() {
		ExtBuilder::default().build().execute_with(|| {
			let signer = alice();
			let payment_asset: AssetId = 2;
			let target: H160 = <Runtime as ErcIdConversion<AssetId>>::runtime_id_to_evm_id(
				payment_asset,
				ERC20_PRECOMPILE_ADDRESS_PREFIX,
			)
			.into();

			// Setup input for an erc20 approve
			let mut input: Vec<u8> = [0x09, 0x5e, 0xa7, 0xb3].to_vec();
			let approve_amount: Balance = 12345;
			input.append(&mut ethabi::encode(&[
				Token::Address(bob().into()),
				Token::Uint(approve_amount.into()),
			]));

			// Setup inner EVM.call call
			let access_list: Vec<(H160, Vec<H256>)> = vec![];
			let call = crate::RuntimeCall::EVM(pallet_evm::Call::call {
				source: signer.into(),
				target,
				input,
				value: U256::default(),
				gas_limit: 50_000,
				max_fee_per_gas: U256::from(1_600_000_000_000_000_u64),
				max_priority_fee_per_gas: None,
				nonce: None,
				access_list,
			});

			// Block signer account
			assert_ok!(MaintenanceMode::block_account(RawOrigin::Root.into(), signer, true));

			// EVM call should fail
			assert_eq!(
				call.clone().dispatch(Some(signer).into()).unwrap_err().error,
				pallet_evm::Error::<Runtime>::WithdrawFailed.into()
			);
			// The storage should not have been updated in TokenApprovals pallet
			assert_eq!(ERC20Approvals::<Runtime>::get((&signer, payment_asset), bob()), None);

			// Unblock signer account
			assert_ok!(MaintenanceMode::block_account(RawOrigin::Root.into(), signer, false));

			// EVM call should now work
			assert_ok!(call.dispatch(Some(signer).into()));
			assert_eq!(
				ERC20Approvals::<Runtime>::get((&signer, payment_asset), bob()),
				Some(approve_amount)
			);
		});
	}

	#[test]
	fn block_account_works_with_evm_create() {
		ExtBuilder::default().build().execute_with(|| {
			let signer = alice();

			// Setup empty init value
			let init: Vec<u8> = vec![];

			// Setup inner EVM.create call
			let access_list: Vec<(H160, Vec<H256>)> = vec![];
			let call = crate::RuntimeCall::EVM(pallet_evm::Call::create {
				source: signer.into(),
				init,
				value: U256::default(),
				gas_limit: 5_000_000,
				max_fee_per_gas: U256::from(1_600_000_000_000_000_u64),
				max_priority_fee_per_gas: None,
				nonce: None,
				access_list,
			});

			// Block signer account
			assert_ok!(MaintenanceMode::block_account(RawOrigin::Root.into(), signer, true));

			// EVM call should fail
			assert_eq!(
				call.clone().dispatch(Some(signer).into()).unwrap_err().error,
				pallet_evm::Error::<Runtime>::WithdrawFailed.into()
			);

			// Unblock signer account
			assert_ok!(MaintenanceMode::block_account(RawOrigin::Root.into(), signer, false));

			// EVM create should now work
			assert_ok!(call.dispatch(Some(signer).into()));
		});
	}

	#[test]
	fn block_account_works_with_evm_create2() {
		ExtBuilder::default().build().execute_with(|| {
			let signer = alice();

			// Setup empty init value
			let init: Vec<u8> = vec![];

			// Setup inner EVM.create call
			let access_list: Vec<(H160, Vec<H256>)> = vec![];
			let call = crate::RuntimeCall::EVM(pallet_evm::Call::create2 {
				source: signer.into(),
				init,
				salt: H256::default(),
				value: U256::default(),
				gas_limit: 5_000_000,
				max_fee_per_gas: U256::from(1_600_000_000_000_000_u64),
				max_priority_fee_per_gas: None,
				nonce: None,
				access_list,
			});

			// Block signer account
			assert_ok!(MaintenanceMode::block_account(RawOrigin::Root.into(), signer, true));

			// EVM call should fail
			assert_eq!(
				call.clone().dispatch(Some(signer).into()).unwrap_err().error,
				pallet_evm::Error::<Runtime>::WithdrawFailed.into()
			);

			// Unblock signer account
			assert_ok!(MaintenanceMode::block_account(RawOrigin::Root.into(), signer, false));

			// EVM create should now work
			assert_ok!(call.dispatch(Some(signer).into()));
		});
	}
}

mod block_evm_target {
	use super::*;

	#[test]
	fn block_evm_target_works() {
		ExtBuilder::default().build().execute_with(|| {
			let signer = alice();

			let payment_asset: AssetId = 2;
			let target: H160 = <Runtime as ErcIdConversion<AssetId>>::runtime_id_to_evm_id(
				payment_asset,
				ERC20_PRECOMPILE_ADDRESS_PREFIX,
			)
			.into();

			// Block the precompile address
			assert_ok!(MaintenanceMode::block_evm_target(RawOrigin::Root.into(), target, true));

			// Setup input for an erc20 transfer to Bob
			let mut input: Vec<u8> = [0xa9, 0x05, 0x9c, 0xbb].to_vec();
			let transfer_amount: Balance = 12345;
			input.append(&mut ethabi::encode(&[
				Token::Address(bob().into()),
				Token::Uint(transfer_amount.into()),
			]));
			// Setup inner EVM.call call
			let access_list: Vec<(H160, Vec<H256>)> = vec![];
			let call = crate::RuntimeCall::EVM(pallet_evm::Call::call {
				source: signer.into(),
				target,
				input,
				value: U256::default(),
				gas_limit: 50_000,
				max_fee_per_gas: U256::from(1_600_000_000_000_000_u64),
				max_priority_fee_per_gas: None,
				nonce: None,
				access_list,
			});

			// EVM call should fail
			assert_eq!(
				call.clone().dispatch(Some(signer).into()).unwrap_err().error,
				pallet_evm::Error::<Runtime>::WithdrawFailed.into()
			);

			// Unblock the precompile address
			assert_ok!(MaintenanceMode::block_evm_target(RawOrigin::Root.into(), target, false));

			// EVM call should now work
			assert_ok!(call.dispatch(Some(signer).into()));
		});
	}
}

mod block_pallet {
	use super::*;

	#[test]
	fn block_pallet_works() {
		ExtBuilder::default().build().execute_with(|| {
			let signer = bob();

			// Check that system.remark works
			let call = frame_system::Call::<Runtime>::remark { remark: vec![0, 1, 2, 3] };
			let call = <Runtime as frame_system::Config>::RuntimeCall::from(call);
			assert_ok!(call.dispatch(Some(signer).into()));

			// Block System pallet
			let blocked_pallet = bounded_string("system");
			assert_ok!(MaintenanceMode::block_pallet(
				RawOrigin::Root.into(),
				blocked_pallet.clone(),
				true
			));

			// System.remark should now fail
			let call = frame_system::Call::<Runtime>::remark { remark: vec![0, 1, 2, 3] };
			let call = <Runtime as frame_system::Config>::RuntimeCall::from(call);
			assert_noop!(call.dispatch(Some(signer).into()), SystemError::CallFiltered);

			// RuntimeCall to other pallet should still work
			let call = pallet_marketplace::Call::<Runtime>::register_marketplace {
				marketplace_account: None,
				entitlement: Default::default(),
			};
			let call = <Runtime as frame_system::Config>::RuntimeCall::from(call);
			assert_ok!(call.dispatch(Some(signer).into()));

			// Unblock System pallet
			assert_ok!(MaintenanceMode::block_pallet(
				RawOrigin::Root.into(),
				blocked_pallet.clone(),
				false
			));

			// Check that system.remark works again
			let call = frame_system::Call::<Runtime>::remark { remark: vec![0, 1, 2, 3] };
			let call = <Runtime as frame_system::Config>::RuntimeCall::from(call);
			assert_ok!(call.dispatch(Some(signer).into()));
		});
	}

	#[test]
	fn block_pallet_works_with_evm() {
		ExtBuilder::default().build().execute_with(|| {
			let signer = alice();
			let payment_asset: AssetId = 2;
			let target: H160 = <Runtime as ErcIdConversion<AssetId>>::runtime_id_to_evm_id(
				payment_asset,
				ERC20_PRECOMPILE_ADDRESS_PREFIX,
			)
			.into();

			// Setup input for an erc20 approve
			let mut input: Vec<u8> = [0x09, 0x5e, 0xa7, 0xb3].to_vec();
			let approve_amount: Balance = 12345;
			input.append(&mut ethabi::encode(&[
				Token::Address(bob().into()),
				Token::Uint(approve_amount.into()),
			]));

			// Setup inner EVM.call call
			let access_list: Vec<(H160, Vec<H256>)> = vec![];
			let call = crate::RuntimeCall::EVM(pallet_evm::Call::call {
				source: signer.into(),
				target,
				input,
				value: U256::default(),
				gas_limit: 50_000,
				max_fee_per_gas: U256::from(1_600_000_000_000_000_u64),
				max_priority_fee_per_gas: None,
				nonce: None,
				access_list,
			});

			// Block TokenApprovals pallet
			let blocked_pallet = bounded_string("tokenapprovals");
			assert_ok!(MaintenanceMode::block_pallet(
				RawOrigin::Root.into(),
				blocked_pallet.clone(),
				true
			));

			// EVM call should succeed, however the internal call fails
			assert_ok!(call.clone().dispatch(Some(signer).into()));
			// The storage should not have been updated in TokenApprovals pallet
			assert_eq!(ERC20Approvals::<Runtime>::get((&signer, payment_asset), bob()), None);

			// Unblock TokenApprovals pallet
			assert_ok!(MaintenanceMode::block_pallet(
				RawOrigin::Root.into(),
				blocked_pallet.clone(),
				false
			));

			// EVM call should now work
			assert_ok!(call.dispatch(Some(signer).into()));
			assert_eq!(
				ERC20Approvals::<Runtime>::get((&signer, payment_asset), bob()),
				Some(approve_amount)
			);
		});
	}

	#[test]
	fn block_sudo_pallet_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			// Block Sudo pallet should fail
			let blocked_pallet = bounded_string("sudo");
			assert_noop!(
				MaintenanceMode::block_pallet(RawOrigin::Root.into(), blocked_pallet.clone(), true),
				pallet_maintenance_mode::Error::<Runtime>::CannotBlock
			);
		});
	}

	#[test]
	fn block_timestamp_pallet_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			// Block Timestamp pallet should fail
			let blocked_pallet = bounded_string("timestamp");
			assert_noop!(
				MaintenanceMode::block_pallet(RawOrigin::Root.into(), blocked_pallet.clone(), true),
				pallet_maintenance_mode::Error::<Runtime>::CannotBlock
			);
		});
	}

	#[test]
	fn block_im_online_pallet_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			// Block ImOnline pallet should fail
			let blocked_pallet = bounded_string("imonline");
			assert_noop!(
				MaintenanceMode::block_pallet(RawOrigin::Root.into(), blocked_pallet.clone(), true),
				pallet_maintenance_mode::Error::<Runtime>::CannotBlock
			);
		});
	}

	#[test]
	fn block_ethy_pallet_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			// Block EthBridge pallet should fail
			let blocked_pallet = bounded_string("ethbridge");
			assert_noop!(
				MaintenanceMode::block_pallet(RawOrigin::Root.into(), blocked_pallet.clone(), true),
				pallet_maintenance_mode::Error::<Runtime>::CannotBlock
			);
		});
	}

	#[test]
	fn block_maintenance_mode_pallet_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			// Block MaintenanceMode pallet should fail
			let blocked_pallet = bounded_string("maintenancemode");
			assert_noop!(
				MaintenanceMode::block_pallet(RawOrigin::Root.into(), blocked_pallet.clone(), true),
				pallet_maintenance_mode::Error::<Runtime>::CannotBlock
			);
		});
	}
}

mod block_call {
	use super::*;

	#[test]
	fn block_call_works() {
		ExtBuilder::default().build().execute_with(|| {
			let signer = bob();

			// Check that system.remark works
			let call = frame_system::Call::<Runtime>::remark { remark: vec![0, 1, 2, 3] };
			let call = <Runtime as frame_system::Config>::RuntimeCall::from(call);
			assert_ok!(call.dispatch(Some(signer).into()));

			// Block System.remark
			let blocked_pallet = bounded_string("System");
			let blocked_call = bounded_string("Remark");
			assert_ok!(MaintenanceMode::block_call(
				RawOrigin::Root.into(),
				blocked_pallet.clone(),
				blocked_call.clone(),
				true
			));

			// System.remark should now fail
			let call = frame_system::Call::<Runtime>::remark { remark: vec![0, 1, 2, 3] };
			let call = <Runtime as frame_system::Config>::RuntimeCall::from(call);
			assert_noop!(call.dispatch(Some(signer).into()), SystemError::CallFiltered);

			// System.remark_with_event should still work
			let call =
				frame_system::Call::<Runtime>::remark_with_event { remark: vec![0, 1, 2, 3] };
			let call = <Runtime as frame_system::Config>::RuntimeCall::from(call);
			assert_ok!(call.dispatch(Some(signer).into()));

			// Unblock System.remark
			assert_ok!(MaintenanceMode::block_call(
				RawOrigin::Root.into(),
				blocked_pallet.clone(),
				blocked_call.clone(),
				false
			));

			// Check that system.remark works again
			let call = frame_system::Call::<Runtime>::remark { remark: vec![0, 1, 2, 3] };
			let call = <Runtime as frame_system::Config>::RuntimeCall::from(call);
			assert_ok!(call.dispatch(Some(signer).into()));
		});
	}

	#[test]
	fn block_call_works_with_evm() {
		ExtBuilder::default().build().execute_with(|| {
			let signer = alice();
			let payment_asset: AssetId = 2;
			let target: H160 = <Runtime as ErcIdConversion<AssetId>>::runtime_id_to_evm_id(
				payment_asset,
				ERC20_PRECOMPILE_ADDRESS_PREFIX,
			)
			.into();

			// Setup input for an erc20 approve
			let mut input: Vec<u8> = [0x09, 0x5e, 0xa7, 0xb3].to_vec();
			let approve_amount: Balance = 12345;
			input.append(&mut ethabi::encode(&[
				Token::Address(bob().into()),
				Token::Uint(approve_amount.into()),
			]));

			// Setup inner EVM.call call
			let access_list: Vec<(H160, Vec<H256>)> = vec![];
			let call = crate::RuntimeCall::EVM(pallet_evm::Call::call {
				source: signer.into(),
				target,
				input,
				value: U256::default(),
				gas_limit: 50_000,
				max_fee_per_gas: U256::from(1_600_000_000_000_000_u64),
				max_priority_fee_per_gas: None,
				nonce: None,
				access_list,
			});

			// Block erc20 approve call
			let blocked_pallet = bounded_string("TokenApprovals");
			let blocked_call = bounded_string("erc20_approval");
			assert_ok!(MaintenanceMode::block_call(
				RawOrigin::Root.into(),
				blocked_pallet.clone(),
				blocked_call.clone(),
				true
			));

			// EVM call should succeed, however the internal call fails
			assert_ok!(call.clone().dispatch(Some(signer).into()));
			// The storage should not have been updated in TokenApprovals pallet
			assert_eq!(ERC20Approvals::<Runtime>::get((&signer, payment_asset), bob()), None);

			// Unblock TokenApprovals erc20 approve
			assert_ok!(MaintenanceMode::block_call(
				RawOrigin::Root.into(),
				blocked_pallet.clone(),
				blocked_call.clone(),
				false
			));

			// EVM call should now work
			assert_ok!(call.dispatch(Some(signer).into()));
			assert_eq!(
				ERC20Approvals::<Runtime>::get((&signer, payment_asset), bob()),
				Some(approve_amount)
			);
		});
	}

	#[test]
	fn block_sudo_call_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			// Block Sudo pallet call fail
			let blocked_pallet = bounded_string("sudo");
			let blocked_call = bounded_string("test_call");
			assert_noop!(
				MaintenanceMode::block_call(
					RawOrigin::Root.into(),
					blocked_pallet,
					blocked_call,
					true
				),
				pallet_maintenance_mode::Error::<Runtime>::CannotBlock
			);
		});
	}

	#[test]
	fn block_timestamp_call_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			// Block Timestamp call should fail
			let blocked_pallet = bounded_string("timestamp");
			let blocked_call = bounded_string("test_call");
			assert_noop!(
				MaintenanceMode::block_call(
					RawOrigin::Root.into(),
					blocked_pallet,
					blocked_call,
					true
				),
				pallet_maintenance_mode::Error::<Runtime>::CannotBlock
			);
		});
	}

	#[test]
	fn block_im_online_call_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			// Block ImOnline call should fail
			let blocked_pallet = bounded_string("imonline");
			let blocked_call = bounded_string("test_call");
			assert_noop!(
				MaintenanceMode::block_call(
					RawOrigin::Root.into(),
					blocked_pallet,
					blocked_call,
					true
				),
				pallet_maintenance_mode::Error::<Runtime>::CannotBlock
			);
		});
	}

	#[test]
	fn block_ethy_call_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			// Block EthBridge call should fail
			let blocked_pallet = bounded_string("ethbridge");
			let blocked_call = bounded_string("test_call");
			assert_noop!(
				MaintenanceMode::block_call(
					RawOrigin::Root.into(),
					blocked_pallet,
					blocked_call,
					true
				),
				pallet_maintenance_mode::Error::<Runtime>::CannotBlock
			);
		});
	}

	#[test]
	fn block_maintenance_mode_call_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			// Block Maintenance Mode call should fail
			let blocked_pallet = bounded_string("maintenancemode");
			let blocked_call = bounded_string("test_call");
			assert_noop!(
				MaintenanceMode::block_call(
					RawOrigin::Root.into(),
					blocked_pallet,
					blocked_call,
					true
				),
				pallet_maintenance_mode::Error::<Runtime>::CannotBlock
			);
		});
	}
}

mod filtered_calls {
	use super::*;
	use pallet_staking::RewardDestination;

	#[test]
	fn pallet_assets_create_fails() {
		ExtBuilder::default().build().execute_with(|| {
			let signer = alice();

			let call = pallet_assets::Call::<Runtime>::create {
				id: 1,
				admin: signer,
				min_balance: Default::default(),
			};
			let call = <Runtime as frame_system::Config>::RuntimeCall::from(call);
			assert_noop!(
				call.dispatch(RawOrigin::Signed(signer).into()),
				SystemError::CallFiltered
			);
		});
	}

	#[test]
	fn pallet_xrpl_bridge_submit_challenge_fails() {
		ExtBuilder::default().build().execute_with(|| {
			let signer = alice();

			let call = pallet_xrpl_bridge::Call::<Runtime>::submit_challenge {
				transaction_hash: Default::default(),
			};
			let call = <Runtime as frame_system::Config>::RuntimeCall::from(call);
			assert_noop!(
				call.dispatch(RawOrigin::Signed(signer).into()),
				SystemError::CallFiltered
			);
		});
	}

	#[test]
	fn pallet_staking_bond_fails() {
		ExtBuilder::default().build().execute_with(|| {
			let signer = charlie();

			// RuntimeCall with RewardDestination::Staked gets filtered
			let call = pallet_staking::Call::<Runtime>::bond {
				value: Default::default(),
				payee: RewardDestination::Staked,
			};
			let call = <Runtime as frame_system::Config>::RuntimeCall::from(call);
			assert_noop!(
				call.dispatch(RawOrigin::Signed(signer).into()),
				SystemError::CallFiltered
			);

			// RuntimeCall with RewardDestination::Controller succeeds
			let call = pallet_staking::Call::<Runtime>::bond {
				value: 12,
				payee: RewardDestination::Controller,
			};
			let call = <Runtime as frame_system::Config>::RuntimeCall::from(call);
			assert_ok!(call.dispatch(RawOrigin::Signed(signer).into()));
		});
	}

	#[test]
	fn pallet_staking_payout_stakers_fails() {
		ExtBuilder::default().build().execute_with(|| {
			let signer = charlie();

			let call = pallet_staking::Call::<Runtime>::payout_stakers {
				validator_stash: alice(),
				era: Default::default(),
			};
			let call = <Runtime as frame_system::Config>::RuntimeCall::from(call);
			assert_noop!(
				call.dispatch(RawOrigin::Signed(signer).into()),
				SystemError::CallFiltered
			);
		});
	}

	#[test]
	fn pallet_proxy_add_proxy_fails() {
		ExtBuilder::default().build().execute_with(|| {
			let signer = charlie();

			let call = pallet_proxy::Call::<Runtime>::add_proxy {
				delegate: alice(),
				proxy_type: Default::default(),
				delay: Default::default(),
			};
			let call = <Runtime as frame_system::Config>::RuntimeCall::from(call);
			assert_noop!(
				call.dispatch(RawOrigin::Signed(signer).into()),
				SystemError::CallFiltered
			);
		});
	}
}
