// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

use crate::{
	mock::{Origin, *},
	Data, FeeConfig, LengthMultiplier,
};
use frame_support::{
	assert_noop, assert_ok, dispatch::GetDispatchInfo, traits::fungibles::Mutate,
	weights::DispatchClass,
};
use frame_system::{limits::BlockWeights, RawOrigin};
use pallet_transaction_payment::ChargeTransactionPayment;
use seed_pallet_common::CreateExt;
use sp_core::U256;
use sp_runtime::{traits::SignedExtension, Perbill};

mod set_evm_base_fee {
	use super::*;

	#[test]
	fn set_evm_base_fee() {
		TestExt::default().build().execute_with(|| {
			let data = Data::<Test>::get();
			let value = U256::from(1_000u128);
			assert!(data.evm_base_fee_per_gas != value);

			let ok = FeeControl::set_evm_base_fee(Origin::root(), value);
			assert_ok!(ok);

			let expected_data = FeeConfig { evm_base_fee_per_gas: value, ..data };
			assert_eq!(Data::<Test>::get(), expected_data);
		});
	}

	#[test]
	fn origin_needs_to_be_root() {
		TestExt::default().build().execute_with(|| {
			let account = AccountId::default();
			let err = FeeControl::set_evm_base_fee(Origin::signed(account), U256::from(1_000u128));
			assert_noop!(err, sp_runtime::DispatchError::BadOrigin);
		});
	}
}

mod set_weight_multiplier {
	use super::*;

	#[test]
	fn set_weight_multiplier() {
		TestExt::default().build().execute_with(|| {
			let data = Data::<Test>::get();
			let value = Perbill::from_rational(1u32, 10u32);
			assert!(data.weight_multiplier != value);

			let ok = FeeControl::set_weight_multiplier(Origin::root(), value);
			assert_ok!(ok);

			let expected_data = FeeConfig { weight_multiplier: value, ..data };
			assert_eq!(Data::<Test>::get(), expected_data);
		});
	}

	#[test]
	fn origin_needs_to_be_root() {
		TestExt::default().build().execute_with(|| {
			let account = AccountId::default();
			let value = Perbill::from_rational(1u32, 10u32);

			let err = FeeControl::set_weight_multiplier(Origin::signed(account), value);
			assert_noop!(err, sp_runtime::DispatchError::BadOrigin);
		});
	}
}

mod set_length_multiplier {
	use super::*;

	#[test]
	fn set_length_multiplier() {
		TestExt::default().build().execute_with(|| {
			let data = Data::<Test>::get();
			let value = LengthMultiplier::new(1_000_000);
			assert!(data.length_multiplier != value);

			let ok = FeeControl::set_length_multiplier(Origin::root(), value);
			assert_ok!(ok);

			let expected_data = FeeConfig { length_multiplier: value, ..data };
			assert_eq!(Data::<Test>::get(), expected_data);
		});
	}

	#[test]
	fn origin_needs_to_be_root() {
		TestExt::default().build().execute_with(|| {
			let account = AccountId::default();
			let value = LengthMultiplier::new(1_000_000);

			let err = FeeControl::set_length_multiplier(Origin::signed(account), value);
			assert_noop!(err, sp_runtime::DispatchError::BadOrigin);
		});
	}
}

mod set_xrp_price {
	use super::*;

	#[test]
	fn set_xrp_price() {
		TestExt::default().build().execute_with(|| {
			let old_data = Data::<Test>::get();

			let ok = FeeControl::set_xrp_price(Origin::root(), Balance::from(1_000_000u128));
			assert_ok!(ok);

			// Make sure that all fields have been updated
			let actual_data = Data::<Test>::get();
			assert_ne!(old_data.evm_base_fee_per_gas, actual_data.evm_base_fee_per_gas);
			assert_ne!(old_data.weight_multiplier, actual_data.weight_multiplier);
			assert_ne!(old_data.length_multiplier, actual_data.length_multiplier);

			let expected_data = FeeConfig {
				evm_base_fee_per_gas: U256::from(10u64),
				weight_multiplier: Perbill::from_rational(1u32, 1000u32),
				length_multiplier: LengthMultiplier { multiplier: 1000, scaling_factor: 1000 },
			};
			assert_eq!(actual_data, expected_data);
		});
	}

	#[test]
	fn xrp_price_lower_limit() {
		TestExt::default().build().execute_with(|| {
			let ok = FeeControl::set_xrp_price(Origin::root(), Balance::from(1_000u128));
			assert_ok!(ok);

			let actual_data = Data::<Test>::get();
			let expected_data = FeeConfig {
				evm_base_fee_per_gas: U256::from(10000u64),
				weight_multiplier: Perbill::from_rational(1u32, 1u32),
				length_multiplier: LengthMultiplier { multiplier: 1000000, scaling_factor: 1000 },
			};
			assert_eq!(actual_data, expected_data);
		});
	}

	#[test]
	fn xrp_price_upper_limit() {
		TestExt::default().build().execute_with(|| {
			let ok = FeeControl::set_xrp_price(Origin::root(), Balance::from(10_000_000u128));
			assert_ok!(ok);

			let actual_data = Data::<Test>::get();
			let expected_data = FeeConfig {
				evm_base_fee_per_gas: U256::from(1u64),
				weight_multiplier: Perbill::from_rational(1u32, 10000u32),
				length_multiplier: LengthMultiplier { multiplier: 100, scaling_factor: 1000 },
			};
			assert_eq!(actual_data, expected_data);
		});
	}

	#[test]
	fn origin_needs_to_be_root() {
		TestExt::default().build().execute_with(|| {
			let account = AccountId::default();

			let err = FeeControl::set_xrp_price(Origin::signed(account), Balance::from(1u128));
			assert_noop!(err, sp_runtime::DispatchError::BadOrigin);
		});
	}
}

#[test]
fn charges_default_extrinsic_amount() {
	TestExt::default().build().execute_with(|| {
		let account = AccountId::default();
		assert_ok!(AssetsExt::create(&account.into(), None));

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
			1,
		));

		let base_fee = FeeControl::weight_to_fee(
			&BlockWeights::default().get(DispatchClass::Normal).base_extrinsic,
		);
		let extrinsic_fee = dispatch_info.weight;

		assert_eq!(
			Assets::balance(100, account),
			starting_fee_token_asset_balance - base_fee - extrinsic_fee as u128
		);
	});
}

#[test]
fn charges_extrinsic_fee_based_on_setting() {
	TestExt::default().build().execute_with(|| {
		let account = AccountId::default();
		assert_ok!(AssetsExt::create(&account.into(), None));

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
			1,
		));

		let base_fee = FeeControl::weight_to_fee(
			&BlockWeights::default().get(DispatchClass::Normal).base_extrinsic,
		);
		let extrinsic_fee = dispatch_info.weight;

		assert_eq!(
			Assets::balance(100, account),
			starting_fee_token_asset_balance - base_fee - extrinsic_fee as u128
		);
	});
}
