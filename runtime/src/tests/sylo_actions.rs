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
	Assets, AssetsExt, Dex, Futurepass, Runtime, RuntimeOrigin, SyloActionPermissions, XrpCurrency,
};
use frame_support::{
	assert_err, assert_ok,
	dispatch::{GetDispatchInfo, RawOrigin},
	pallet_prelude::{InvalidTransaction, TransactionValidityError},
	traits::{fungible::Inspect, fungibles::Inspect as Inspects},
};
use pallet_sylo_action_permissions::Spender;

use crate::constants::XRP_ASSET_ID;
use pallet_transaction_payment::ChargeTransactionPayment;
use seed_primitives::{AccountId, Balance};
use sp_runtime::traits::SignedExtension;
use sp_runtime::BoundedBTreeSet;
use sp_std::collections::btree_set::BTreeSet;

fn create_transact_call() -> crate::RuntimeCall {
	let remark = b"hello";
	let inner_call = crate::RuntimeCall::System(frame_system::Call::remark_with_event {
		remark: remark.to_vec(),
	});

	crate::RuntimeCall::SyloActionPermissions(pallet_sylo_action_permissions::Call::transact {
		grantor: alice(),
		call: Box::new(inner_call),
	})
}

fn setup_asset_liquidity(new_account: AccountId) -> u32 {
	let payment_asset = AssetsExt::next_asset_uuid().unwrap();

	assert_ok!(AssetsExt::create_asset(
		RawOrigin::Signed(alice()).into(),
		b"Test".to_vec(),
		b"Test".to_vec(),
		6,
		None,
		None
	));

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

	payment_asset
}

#[test]
fn transact_spends_from_grantee() {
	ExtBuilder::default().build().execute_with(|| {
		let grantee_xrp_balance_before = XrpCurrency::balance(&bob());

		let transact_call = create_transact_call();

		let dispatch_info: frame_support::dispatch::DispatchInfo =
			transact_call.get_dispatch_info();

		assert_ok!(<ChargeTransactionPayment<Runtime> as SignedExtension>::pre_dispatch(
			ChargeTransactionPayment::from(0),
			&bob(),
			&transact_call,
			&dispatch_info,
			1,
		));

		let grantee_xrp_balance_after = XrpCurrency::balance(&bob());

		assert!(grantee_xrp_balance_before > grantee_xrp_balance_after);
	});
}

#[test]
fn transact_spends_from_futurepass() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Futurepass::create(RuntimeOrigin::signed(alice()), bob()));
		let futurepass = pallet_futurepass::Holders::<Runtime>::get(bob()).unwrap();

		assert_ok!(SyloActionPermissions::grant_transact_permission(
			RawOrigin::Signed(alice()).into(),
			futurepass.clone(),
			Spender::GRANTEE,
			None,
			BoundedBTreeSet::new(),
			None,
		));

		assert_ok!(AssetsExt::transfer(
			RuntimeOrigin::signed(alice()),
			XRP_ASSET_ID,
			futurepass.clone(),
			100_000_000,
			true
		));

		let holder_xrp_balance_before = XrpCurrency::balance(&bob());
		let futurepass_xrp_balance_before = XrpCurrency::balance(&futurepass);

		let transact_call = create_transact_call();

		let fp_proxy_call =
			crate::RuntimeCall::Futurepass(pallet_futurepass::Call::proxy_extrinsic {
				futurepass,
				call: Box::new(transact_call.clone()),
			});

		let dispatch_info: frame_support::dispatch::DispatchInfo =
			fp_proxy_call.get_dispatch_info();

		assert_ok!(<ChargeTransactionPayment<Runtime> as SignedExtension>::pre_dispatch(
			ChargeTransactionPayment::from(0),
			&bob(),
			&fp_proxy_call,
			&dispatch_info,
			1,
		));

		let holder_xrp_balance_after = XrpCurrency::balance(&bob());
		let futurepass_xrp_balance_after = XrpCurrency::balance(&futurepass);

		assert_eq!(holder_xrp_balance_before, holder_xrp_balance_after);
		assert!(futurepass_xrp_balance_before > futurepass_xrp_balance_after);
	});
}

#[test]
fn transact_works_with_fee_proxy() {
	ExtBuilder::default().build().execute_with(|| {
		let payment_asset = setup_asset_liquidity(bob());

		let transact_call = create_transact_call();

		let fee_proxy_call =
			crate::RuntimeCall::FeeProxy(pallet_fee_proxy::Call::call_with_fee_preferences {
				payment_asset,
				max_payment: 10_000_000,
				call: Box::new(transact_call.clone()),
			});

		let dispatch_info: frame_support::dispatch::DispatchInfo =
			fee_proxy_call.get_dispatch_info();

		let grantee_xrp_balance_before = XrpCurrency::balance(&bob());
		let grantee_token_balance_before = AssetsExt::balance(payment_asset, &bob());

		assert_ok!(<ChargeTransactionPayment<Runtime> as SignedExtension>::pre_dispatch(
			ChargeTransactionPayment::from(0),
			&bob(),
			&fee_proxy_call,
			&dispatch_info,
			1,
		));

		let grantee_xrp_balance_after = XrpCurrency::balance(&bob());
		let grantee_token_balance_after = AssetsExt::balance(payment_asset, &bob());

		assert_eq!(grantee_xrp_balance_before, grantee_xrp_balance_after);
		assert!(grantee_token_balance_before > grantee_token_balance_after);
	});
}

#[test]
fn transact_works_with_fee_proxy_and_futurepass() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Futurepass::create(RuntimeOrigin::signed(alice()), bob()));
		let futurepass = pallet_futurepass::Holders::<Runtime>::get(bob()).unwrap();

		let payment_asset = setup_asset_liquidity(futurepass.clone());

		let transact_call = create_transact_call();

		let holder_token_balance_before = AssetsExt::balance(payment_asset, &bob());
		let futurepass_token_balance_before = AssetsExt::balance(payment_asset, &futurepass);

		let fp_proxy_call =
			crate::RuntimeCall::Futurepass(pallet_futurepass::Call::proxy_extrinsic {
				futurepass,
				call: Box::new(transact_call.clone()),
			});

		let fee_proxy_call =
			crate::RuntimeCall::FeeProxy(pallet_fee_proxy::Call::call_with_fee_preferences {
				payment_asset,
				max_payment: 10_000_000,
				call: Box::new(fp_proxy_call.clone()),
			});

		let dispatch_info: frame_support::dispatch::DispatchInfo =
			fee_proxy_call.get_dispatch_info();

		assert_ok!(<ChargeTransactionPayment<Runtime> as SignedExtension>::pre_dispatch(
			ChargeTransactionPayment::from(0),
			&bob(),
			&fee_proxy_call,
			&dispatch_info,
			1,
		));

		let holder_token_balance_after = AssetsExt::balance(payment_asset, &bob());
		let futurepass_token_balance_after = AssetsExt::balance(payment_asset, &futurepass);

		assert_eq!(holder_token_balance_before, holder_token_balance_after);
		assert!(futurepass_token_balance_before > futurepass_token_balance_after);
	});
}

#[test]
fn transact_spends_from_grantor() {
	ExtBuilder::default().build().execute_with(|| {
		// configure the permission record to state the grantor is the spender
		assert_ok!(SyloActionPermissions::grant_transact_permission(
			RawOrigin::Signed(alice()).into(),
			bob(),
			Spender::GRANTOR,
			None,
			BoundedBTreeSet::new(),
			None,
		));

		let grantor_xrp_balance_before = XrpCurrency::balance(&alice());

		let transact_call = create_transact_call();

		let dispatch_info: frame_support::dispatch::DispatchInfo =
			transact_call.get_dispatch_info();

		assert_ok!(<ChargeTransactionPayment<Runtime> as SignedExtension>::pre_dispatch(
			ChargeTransactionPayment::from(0),
			&bob(),
			&transact_call,
			&dispatch_info,
			1,
		));

		let grantor_xrp_balance_after = XrpCurrency::balance(&alice());

		assert!(grantor_xrp_balance_before > grantor_xrp_balance_after);
	});
}

#[test]
fn transact_deducts_from_spending_balance() {
	ExtBuilder::default().build().execute_with(|| {
		let initial_spending_balance: Balance = 100_000_000;

		// configure the permission record to state the grantor is the spender
		assert_ok!(SyloActionPermissions::grant_transact_permission(
			RawOrigin::Signed(alice()).into(),
			bob(),
			Spender::GRANTOR,
			Some(initial_spending_balance),
			BoundedBTreeSet::new(),
			None,
		));

		let caller_xrp_balance_before = XrpCurrency::balance(&alice());

		let transact_call = create_transact_call();

		let dispatch_info: frame_support::dispatch::DispatchInfo =
			transact_call.get_dispatch_info();

		assert_ok!(<ChargeTransactionPayment<Runtime> as SignedExtension>::pre_dispatch(
			ChargeTransactionPayment::from(0),
			&bob(),
			&transact_call,
			&dispatch_info,
			1,
		));

		let caller_xrp_balance_after = XrpCurrency::balance(&alice());

		let updated_spending_balance =
			<pallet_sylo_action_permissions::TransactPermissions<Runtime>>::get(&alice(), &bob())
				.unwrap()
				.spending_balance
				.unwrap();

		assert!(caller_xrp_balance_before > caller_xrp_balance_after);

		// validate spending balance was reduced by the fee charged
		assert_eq!(
			(caller_xrp_balance_before - caller_xrp_balance_after),
			(initial_spending_balance - updated_spending_balance)
		);
	});
}

#[test]
fn transact_fails_with_insufficient_spending_balance() {
	ExtBuilder::default().build().execute_with(|| {
		// configure the permission record to state the grantor is the spender
		assert_ok!(SyloActionPermissions::grant_transact_permission(
			RawOrigin::Signed(alice()).into(),
			bob(),
			Spender::GRANTOR,
			Some(1000), // Set a low spending balance
			BoundedBTreeSet::new(),
			None,
		));

		let transact_call = create_transact_call();

		let dispatch_info: frame_support::dispatch::DispatchInfo =
			transact_call.get_dispatch_info();

		assert_err!(
			<ChargeTransactionPayment<Runtime> as SignedExtension>::pre_dispatch(
				ChargeTransactionPayment::from(0),
				&bob(),
				&transact_call,
				&dispatch_info,
				1,
			),
			TransactionValidityError::Invalid(InvalidTransaction::Payment)
		);
	});
}

#[test]
fn transact_deducts_from_spending_balance_with_fee_proxy() {
	ExtBuilder::default().build().execute_with(|| {
		let payment_asset = setup_asset_liquidity(bob());

		let initial_spending_balance: Balance = 100_000_000;

		// configure the permission record to state the grantor is the spender
		assert_ok!(SyloActionPermissions::grant_transact_permission(
			RawOrigin::Signed(alice()).into(),
			bob(),
			Spender::GRANTOR,
			Some(initial_spending_balance),
			BoundedBTreeSet::new(),
			None,
		));

		let transact_call = create_transact_call();

		let fee_proxy_call =
			crate::RuntimeCall::FeeProxy(pallet_fee_proxy::Call::call_with_fee_preferences {
				payment_asset,
				max_payment: 10_000_000,
				call: Box::new(transact_call.clone()),
			});

		let dispatch_info: frame_support::dispatch::DispatchInfo =
			fee_proxy_call.get_dispatch_info();

		let grantee_xrp_balance_before = XrpCurrency::balance(&bob());

		assert_ok!(<ChargeTransactionPayment<Runtime> as SignedExtension>::pre_dispatch(
			ChargeTransactionPayment::from(0),
			&bob(),
			&fee_proxy_call,
			&dispatch_info,
			1,
		));

		let grantee_xrp_balance_after = XrpCurrency::balance(&bob());

		let updated_spending_balance =
			<pallet_sylo_action_permissions::TransactPermissions<Runtime>>::get(&alice(), &bob())
				.unwrap()
				.spending_balance
				.unwrap();

		assert_eq!(grantee_xrp_balance_before, grantee_xrp_balance_after);

		// validate spending balance was reduced by the fee charged
		assert!(initial_spending_balance > updated_spending_balance);
	});
}
