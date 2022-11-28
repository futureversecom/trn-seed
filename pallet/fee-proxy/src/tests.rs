/* Copyright 2019-2021 Centrality Investments Limited
 *
 * Licensed under the LGPL, Version 3.0 (the "License");
 * you may not use this file except in compliance with the License.
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * You may obtain a copy of the License at the root of this project source code,
 * or at:
 *     https://centrality.ai/licenses/gplv3.txt
 *     https://centrality.ai/licenses/lgplv3.txt
 */

use super::*;
use crate::mock::{AccountId, FeeProxy, System, Test, TestExt, XRP_ASSET_ID};
use frame_support::{assert_noop, assert_ok};
use seed_primitives::{AssetId, Balance};

#[test]
fn call_with_fee_preferences_works() {
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 1;
		let payment_asset: AssetId = 10;
		let max_payment: Balance = 100;
		let call =
			mock::Call::System(frame_system::Call::remark { remark: b"Mischief Managed".to_vec() });

		assert_ok!(FeeProxy::call_with_fee_preferences(
			Some(caller).into(),
			payment_asset,
			max_payment,
			Box::new(call)
		));

		System::assert_has_event(
			Event::CallWithFeePreferences { who: caller, payment_asset, max_payment }.into(),
		);
	});
}

#[test]
fn call_with_fee_preferences_fee_asset_should_fail() {
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 1;
		let payment_asset: AssetId = XRP_ASSET_ID;
		let max_payment: Balance = 100;
		let call =
			mock::Call::System(frame_system::Call::remark { remark: b"Mischief Managed".to_vec() });

		// Should fail as the payment token is the fee asset
		assert_noop!(
			FeeProxy::call_with_fee_preferences(
				Some(caller).into(),
				payment_asset,
				max_payment,
				Box::new(call)
			),
			Error::<Test>::FeeTokenIsGasToken
		);
	});
}

#[test]
fn call_with_fee_preferences_inner_call_fails() {
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 1;
		let payment_asset: AssetId = 10;
		let max_payment: Balance = 100;
		let call = mock::Call::System(frame_system::Call::fill_block { ratio: Default::default() });

		// Test that the error returned is the error from the inner call. In this case it is BadOrigin
		// as fill_block requires root. This is the easiest example to use without
		// pulling in more dev dependencies
		assert_noop!(
			FeeProxy::call_with_fee_preferences(
				Some(caller).into(),
				payment_asset,
				max_payment,
				Box::new(call)
			),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn call_with_fee_preferences_nested_call_should_fail() {
	TestExt::default().build().execute_with(|| {
		let caller: AccountId = 1;
		let payment_asset: AssetId = 10;
		let max_payment: Balance = 100;

		let call_inner =
			mock::Call::System(frame_system::Call::remark { remark: b"Mischief Managed".to_vec() });

		let call = mock::Call::FeeProxy(crate::Call::call_with_fee_preferences {
			payment_asset,
			max_payment,
			call: Box::new(call_inner),
		});

		// Should fail as the inner call is call_with_fee_preferences
		assert_noop!(
			FeeProxy::call_with_fee_preferences(
				Some(caller).into(),
				payment_asset,
				max_payment,
				Box::new(call)
			),
			Error::<Test>::NestedFeePreferenceCall
		);
	});
}
