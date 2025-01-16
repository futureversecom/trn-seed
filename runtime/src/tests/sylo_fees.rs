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

//! Integration tests for the sylo pallet. Ensures sylo extrinsics are correctly
//! charged with the set Sylo payment token.
#![cfg(test)]

use crate::{
	tests::{alice, bob, ExtBuilder},
	Assets, AssetsExt, Dex, Futurepass, Runtime, RuntimeOrigin, Sylo, XrpCurrency,
};
use frame_support::{
	assert_err, assert_ok,
	dispatch::{GetDispatchInfo, RawOrigin},
	pallet_prelude::{InvalidTransaction, TransactionValidityError},
	traits::{fungible::Inspect, fungibles::Inspect as Inspects},
};
use seed_pallet_common::test_prelude::create_account;

use crate::constants::XRP_ASSET_ID;
use pallet_transaction_payment::ChargeTransactionPayment;
use seed_primitives::{AccountId, Balance};
use sp_core::H256;
use sp_runtime::{traits::SignedExtension, BoundedVec};

#[test]
fn sylo_extrinsic_works_with_sylo_token() {
	ExtBuilder::default().build().execute_with(|| {
		let new_account = create_account(2);

		let payment_asset = setup_sylo_liquidity(new_account.clone());

		let calls = create_sylo_calls();

		for call in calls.iter() {
			let caller_token_balance_before = AssetsExt::balance(payment_asset, &new_account);

			let dispatch_info = call.get_dispatch_info();

			assert_ok!(<ChargeTransactionPayment<Runtime> as SignedExtension>::pre_dispatch(
				ChargeTransactionPayment::from(0),
				&new_account,
				&call,
				&dispatch_info,
				1,
			));

			let caller_token_balance_after = AssetsExt::balance(payment_asset, &new_account);

			// validate caller had their sylo token balance reduced
			assert!(caller_token_balance_before > caller_token_balance_after);
		}
	});
}

#[test]
fn sylo_extrinsic_works_with_futurepass_payment() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Futurepass::create(RuntimeOrigin::signed(alice()), alice()));
		let futurepass = pallet_futurepass::Holders::<Runtime>::get(alice()).unwrap();

		let payment_asset = setup_sylo_liquidity(futurepass.clone());

		let calls = create_sylo_calls();

		for call in calls.iter() {
			let caller_xrp_balance_before = XrpCurrency::balance(&alice());
			let caller_token_balance_before = AssetsExt::balance(payment_asset, &alice());
			let futurepass_token_balance_before = AssetsExt::balance(payment_asset, &futurepass);

			let fp_proxy_call =
				crate::RuntimeCall::Futurepass(pallet_futurepass::Call::proxy_extrinsic {
					futurepass,
					call: Box::new(call.clone()),
				});

			let dispatch_info = fp_proxy_call.get_dispatch_info();

			assert_ok!(<ChargeTransactionPayment<Runtime> as SignedExtension>::pre_dispatch(
				ChargeTransactionPayment::from(0),
				&alice(),
				&fp_proxy_call,
				&dispatch_info,
				1,
			));

			let caller_xrp_balance_after = XrpCurrency::balance(&alice());
			let caller_token_balance_after = AssetsExt::balance(payment_asset, &alice());
			let futurepass_token_balance_after = AssetsExt::balance(payment_asset, &futurepass);

			// validate futurepass should only have paid in tokens
			assert_eq!(caller_xrp_balance_before, caller_xrp_balance_after);
			assert_eq!(caller_token_balance_before, caller_token_balance_after);

			assert!(futurepass_token_balance_before > futurepass_token_balance_after);
		}
	});
}

#[test]
fn sylo_extrinsic_fails_without_sylo_funds() {
	ExtBuilder::default().build().execute_with(|| {
		// Test executing that calls without setting up the
		// liquidity prior
		let calls = create_sylo_calls();

		for call in calls.iter() {
			let dispatch_info = call.get_dispatch_info();

			assert_err!(
				<ChargeTransactionPayment<Runtime> as SignedExtension>::pre_dispatch(
					ChargeTransactionPayment::from(0),
					&alice(),
					&call,
					&dispatch_info,
					1,
				),
				TransactionValidityError::Invalid(InvalidTransaction::Payment)
			);
		}
	});
}

#[test]
fn sylo_extrinsic_fails_without_fee_proxy() {
	ExtBuilder::default().build().execute_with(|| {
		let calls = create_sylo_calls();

		for call in calls.iter() {
			let dispatch_info = call.get_dispatch_info();

			assert_err!(
				<ChargeTransactionPayment<Runtime> as SignedExtension>::pre_dispatch(
					ChargeTransactionPayment::from(0),
					&alice(),
					&call,
					&dispatch_info,
					1,
				),
				TransactionValidityError::Invalid(InvalidTransaction::Payment)
			);
		}
	});
}

#[test]
fn sylo_extrinsic_fails_using_call_with_fee_preferences() {
	ExtBuilder::default().build().execute_with(|| {
		let new_account = create_account(2);

		let payment_asset = setup_sylo_liquidity(new_account.clone());

		let calls = create_sylo_calls();

		for call in calls.iter() {
			let max_payment: Balance = 10_000_000_000_000_000;
			let fee_proxy_call =
				crate::RuntimeCall::FeeProxy(pallet_fee_proxy::Call::call_with_fee_preferences {
					payment_asset,
					max_payment,
					call: Box::new(call.clone()),
				});

			let dispatch_info = fee_proxy_call.get_dispatch_info();
			assert_err!(
				<ChargeTransactionPayment<Runtime> as SignedExtension>::pre_dispatch(
					ChargeTransactionPayment::from(0),
					&new_account,
					&fee_proxy_call,
					&dispatch_info,
					1,
				),
				TransactionValidityError::Invalid(InvalidTransaction::Payment)
			);
		}
	});
}

fn setup_sylo_liquidity(new_account: AccountId) -> u32 {
	let payment_asset = AssetsExt::next_asset_uuid().unwrap();

	assert_ok!(AssetsExt::create_asset(
		RawOrigin::Signed(alice()).into(),
		b"Test".to_vec(),
		b"Test".to_vec(),
		6,
		None,
		None
	));

	assert_eq!(AssetsExt::balance(payment_asset, &bob()), 0);

	// Mint these assets into Alice and new_account
	assert_ok!(Assets::mint(
		RawOrigin::Signed(alice()).into(),
		payment_asset,
		alice(),
		10_000_000_000_000_000
	));
	assert_ok!(Assets::mint(
		RawOrigin::Signed(alice()).into(),
		payment_asset,
		new_account,
		10_000_000_000_000_000
	));

	// Add liquidity to the dex
	assert_ok!(Dex::add_liquidity(
		RawOrigin::Signed(alice()).into(),
		XRP_ASSET_ID,
		payment_asset,
		1_000_000_000_000,
		1_000_000_000_000,
		1,
		1,
		None,
		None,
	));

	assert_ok!(Sylo::set_payment_asset(RawOrigin::Root.into(), payment_asset));

	payment_asset
}

/// Creates a list of calls for all sylo extrinsics which should be charged in Sylo Tokens
fn create_sylo_calls() -> Vec<<Runtime as pallet_sylo::Config>::RuntimeCall> {
	vec![
		crate::RuntimeCall::Sylo(pallet_sylo::Call::register_resolver {
			identifier: BoundedVec::new(),
			service_endpoints: BoundedVec::new(),
		}),
		crate::RuntimeCall::Sylo(pallet_sylo::Call::update_resolver {
			identifier: BoundedVec::new(),
			service_endpoints: BoundedVec::new(),
		}),
		crate::RuntimeCall::Sylo(pallet_sylo::Call::deregister_resolver {
			identifier: BoundedVec::new(),
		}),
		crate::RuntimeCall::Sylo(pallet_sylo::Call::create_validation_record {
			data_id: BoundedVec::new(),
			resolvers: BoundedVec::new(),
			data_type: BoundedVec::new(),
			tags: BoundedVec::new(),
			checksum: H256::from_low_u64_be(123),
		}),
		crate::RuntimeCall::Sylo(pallet_sylo::Call::add_validation_record_entry {
			data_id: BoundedVec::new(),
			checksum: H256::from_low_u64_be(123),
		}),
		crate::RuntimeCall::Sylo(pallet_sylo::Call::update_validation_record {
			data_id: BoundedVec::new(),
			resolvers: None,
			data_type: None,
			tags: None,
		}),
		crate::RuntimeCall::Sylo(pallet_sylo::Call::delete_validation_record {
			data_id: BoundedVec::new(),
		}),
	]
}
