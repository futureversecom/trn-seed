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

use super::Event;
use crate::{
	mock::{RuntimeEvent as MockEvent, *},
	Data,
};
use frame_support::{
	dispatch::{DispatchClass, GetDispatchInfo},
	traits::fungibles::Mutate,
};
use frame_system::{limits::BlockWeights, RawOrigin};
use pallet_transaction_payment::ChargeTransactionPayment;
use seed_pallet_common::test_prelude::*;
use sp_runtime::{traits::SignedExtension, Perbill};

#[test]
fn charges_default_extrinsic_amount() {
	TestExt::<Test>::default().build().execute_with(|| {
		let account = AccountId::default();
		assert_ok!(AssetsExt::create(&account, None));

		let starting_fee_token_asset_balance = 4200000069;
		assert_ok!(AssetsExt::mint_into(100, &account, starting_fee_token_asset_balance));

		let fee_token_balance = Assets::balance(100, account);
		assert_eq!(fee_token_balance, starting_fee_token_asset_balance);

		assert_ok!(MockPallet::mock_charge_fee(RawOrigin::Signed(account).into()));

		let call = mock_pallet::pallet::Call::mock_charge_fee {};
		let dispatch_info = call.get_dispatch_info();

		assert_ok!(<ChargeTransactionPayment<Test> as SignedExtension>::pre_dispatch(
			ChargeTransactionPayment::from(0),
			&account,
			&call.into(),
			&dispatch_info,
			0,
		));

		let base_fee = FeeControl::weight_to_fee(
			&BlockWeights::default().get(DispatchClass::Normal).base_extrinsic,
		);
		let extrinsic_fee = dispatch_info.weight.ref_time();

		assert_eq!(
			Assets::balance(100, account),
			starting_fee_token_asset_balance - base_fee - extrinsic_fee as u128
		);
	});
}

#[test]
fn charges_extrinsic_fee_based_on_setting() {
	TestExt::<Test>::default().build().execute_with(|| {
		let account = AccountId::default();
		assert_ok!(AssetsExt::create(&account, None));

		let starting_fee_token_asset_balance = 4200000069;

		assert_ok!(AssetsExt::mint_into(100, &account, starting_fee_token_asset_balance));

		let fee_token_balance = Assets::balance(100, account);
		assert_eq!(fee_token_balance, starting_fee_token_asset_balance);
		assert_ok!(MockPallet::mock_charge_fee(RawOrigin::Signed(account).into()));

		assert_ok!(FeeControl::set_weight_multiplier(
			RawOrigin::Root.into(),
			Perbill::from_percent(42)
		));

		let call = mock_pallet::pallet::Call::mock_charge_fee {};
		let dispatch_info = call.get_dispatch_info();

		assert_ok!(<ChargeTransactionPayment<Test> as SignedExtension>::pre_dispatch(
			ChargeTransactionPayment::from(0),
			&account,
			&call.into(),
			&dispatch_info,
			0,
		));

		let base_fee = FeeControl::weight_to_fee(
			&BlockWeights::default().get(DispatchClass::Normal).base_extrinsic,
		);
		let extrinsic_fee = dispatch_info.weight.ref_time();

		assert_eq!(
			Assets::balance(100, account),
			starting_fee_token_asset_balance - base_fee - extrinsic_fee as u128
		);
	});
}

#[test]
fn set_evm_base_fee_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let new_value = U256::from(12345);
		assert_ok!(FeeControl::set_evm_base_fee(RawOrigin::Root.into(), new_value));

		assert_eq!(Data::<Test>::get().evm_base_fee_per_gas, new_value);

		System::assert_last_event(MockEvent::FeeControl(Event::<Test>::EvmBaseFeeSet {
			base_fee: new_value,
		}));
	});
}

#[test]
fn set_weight_multiplier_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let new_value = Perbill::from_parts(12345);
		assert_ok!(FeeControl::set_weight_multiplier(RawOrigin::Root.into(), new_value));

		assert_eq!(Data::<Test>::get().weight_multiplier, new_value);

		System::assert_last_event(MockEvent::FeeControl(Event::<Test>::WeightMultiplierSet {
			weight_multiplier: new_value,
		}));
	});
}

#[test]
fn set_length_multiplier_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let new_value: Balance = 12345;
		assert_ok!(FeeControl::set_length_multiplier(RawOrigin::Root.into(), new_value));

		assert_eq!(Data::<Test>::get().length_multiplier, new_value);

		System::assert_last_event(MockEvent::FeeControl(Event::<Test>::LengthMultiplierSet {
			length_multiplier: new_value,
		}));
	});
}
