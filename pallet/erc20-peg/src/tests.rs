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
use crate::{
	mock::{AssetsExt, Erc20Peg, ExtBuilder, MockEthereumEventRouter, PegPalletId, Test},
	types::{DelayedPaymentId, Erc20DepositEvent, PendingPayment, WithdrawMessage},
};
use frame_support::traits::fungibles::{Inspect, Mutate};
use hex_literal::hex;
use seed_pallet_common::test_prelude::*;

#[test]
fn set_peg_contract_address_works() {
	ExtBuilder.build().execute_with(|| {
		let signer = create_account(22);
		let contract_address = H160::from_low_u64_be(123);

		// Setting as not sudo fails
		assert_noop!(
			Erc20Peg::set_erc20_peg_address(Some(signer).into(), contract_address),
			BadOrigin
		);

		// Sanity check
		assert_eq!(ContractAddress::<Test>::get(), H160::default());

		// Calling as sudo should work
		assert_ok!(Erc20Peg::set_erc20_peg_address(
			frame_system::RawOrigin::Root.into(),
			contract_address
		));

		// Storage updated
		assert_eq!(ContractAddress::<Test>::get(), contract_address);
	});
}

#[test]
fn set_deposit_delay_active() {
	ExtBuilder.build().execute_with(|| {
		let signer = create_account(22);

		// Setting as not sudo fails
		assert_noop!(Erc20Peg::activate_deposits_delay(Some(signer).into(), false), BadOrigin);

		// Sanity check
		assert_eq!(DepositsDelayActive::<Test>::get(), true);

		// Calling as sudo should work
		assert_ok!(Erc20Peg::activate_deposits_delay(frame_system::RawOrigin::Root.into(), false));

		// Storage updated
		assert_eq!(DepositsDelayActive::<Test>::get(), false);
	});
}

#[test]
fn set_withdrawal_delay_active() {
	ExtBuilder.build().execute_with(|| {
		let signer = create_account(22);

		// Setting as not sudo fails
		assert_noop!(Erc20Peg::activate_withdrawals_delay(Some(signer).into(), false), BadOrigin);

		// Sanity check
		assert_eq!(WithdrawalsDelayActive::<Test>::get(), true);

		// Calling as sudo should work
		assert_ok!(Erc20Peg::activate_withdrawals_delay(
			frame_system::RawOrigin::Root.into(),
			false
		));

		// Storage updated
		assert_eq!(WithdrawalsDelayActive::<Test>::get(), false);
	});
}

#[test]
fn set_root_peg_address_works() {
	ExtBuilder.build().execute_with(|| {
		let signer = create_account(22);
		let contract_address = H160::from_low_u64_be(123);

		// Setting as not sudo fails
		assert_noop!(
			Erc20Peg::set_root_peg_address(Some(signer).into(), contract_address),
			BadOrigin
		);

		// Sanity check
		assert_eq!(RootPegContractAddress::<Test>::get(), H160::default());

		// Calling as sudo should work
		assert_ok!(Erc20Peg::set_root_peg_address(
			frame_system::RawOrigin::Root.into(),
			contract_address
		));

		// Storage updated
		assert_eq!(RootPegContractAddress::<Test>::get(), contract_address);
	});
}

#[test]
fn set_erc20_asset_map_works() {
	ExtBuilder.build().execute_with(|| {
		let signer = create_account(22);
		let contract_address = H160::from_low_u64_be(123);
		let asset_id: AssetId = 12;

		// Setting as not sudo fails
		assert_noop!(
			Erc20Peg::set_erc20_asset_map(Some(signer).into(), asset_id, contract_address),
			BadOrigin
		);

		// Calling as sudo should work
		assert_ok!(Erc20Peg::set_erc20_asset_map(
			frame_system::RawOrigin::Root.into(),
			asset_id,
			contract_address
		));

		// Storage updated
		assert_eq!(Erc20ToAssetId::<Test>::get(contract_address).unwrap(), asset_id);
		assert_eq!(AssetIdToErc20::<Test>::get(asset_id).unwrap(), contract_address);
	});
}

#[test]
fn set_payment_delay() {
	ExtBuilder.build().execute_with(|| {
		let asset_id: AssetId = 1;
		let min_balance: Balance = 100;
		let delay: u64 = 1000;
		assert_ok!(Erc20Peg::set_payment_delay(
			frame_system::RawOrigin::Root.into(),
			asset_id,
			min_balance,
			delay
		));
		assert_eq!(PaymentDelay::<Test>::get(asset_id), Some((min_balance, delay)));
	});
}

#[test]
fn deposit_payment_with_ethereum_event_router() {
	ExtBuilder.build().execute_with(|| {
		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));
		// Set contract address
		let contract_address = H160::from_low_u64_be(123);
		assert_ok!(Erc20Peg::set_erc20_peg_address(
			frame_system::RawOrigin::Root.into(),
			contract_address
		));

		// Setup token mapping
		let token_address: H160 = H160::from_low_u64_be(666);
		Erc20ToAssetId::<Test>::insert(token_address, SPENDING_ASSET_ID);

		let destination = <Test as Config>::PegPalletId::get().into_account_truncating();
		let deposit_amount: Balance = 100;
		let beneficiary: H160 = H160::from_low_u64_be(456);

		// Encode data for bridge call
		let data = ethabi::encode(&[
			Token::Address(token_address),
			Token::Uint(deposit_amount.into()),
			Token::Address(beneficiary),
		]);
		assert_ok!(MockEthereumEventRouter::route(
			&contract_address,
			&destination,
			data.clone().as_slice()
		));

		// Check beneficiary account received funds
		assert_eq!(
			AssetsExt::balance(SPENDING_ASSET_ID, &AccountId::from(beneficiary)),
			deposit_amount
		);
	});
}

#[test]
fn deposit_payment_with_ethereum_event_router_source_address_not_set() {
	ExtBuilder.build().execute_with(|| {
		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));

		// Setup token mapping
		let token_address: H160 = H160::from_low_u64_be(666);
		Erc20ToAssetId::<Test>::insert(token_address, SPENDING_ASSET_ID);

		let source = H160::from_low_u64_be(123);
		let destination = <Test as Config>::PegPalletId::get().into_account_truncating();
		let deposit_amount: Balance = 100;
		let beneficiary: H160 = H160::from_low_u64_be(456);

		// Encode data for bridge call
		let data = ethabi::encode(&[
			Token::Address(token_address),
			Token::Uint(deposit_amount.into()),
			Token::Address(beneficiary),
		]);
		assert_noop!(
			MockEthereumEventRouter::route(&source, &destination, data.clone().as_slice()),
			(
				DbWeight::get().reads(2u64),
				EventRouterError::FailedProcessing(
					DispatchError::Other("Invalid source address")
				)
			)
		);
	});
}

#[test]
fn deposit_payment_with_ethereum_event_router_incorrect_source_address() {
	ExtBuilder.build().execute_with(|| {
		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));
		// Set contract address to different value
		let contract_address = H160::from_low_u64_be(8910);
		assert_ok!(Erc20Peg::set_erc20_peg_address(
			frame_system::RawOrigin::Root.into(),
			contract_address
		));

		// Setup token mapping
		let token_address: H160 = H160::from_low_u64_be(666);
		Erc20ToAssetId::<Test>::insert(token_address, SPENDING_ASSET_ID);

		let source = H160::from_low_u64_be(123);
		let destination = <Test as Config>::PegPalletId::get().into_account_truncating();
		let deposit_amount: Balance = 100;
		let beneficiary: H160 = H160::from_low_u64_be(456);

		// Encode data for bridge call
		let data = ethabi::encode(&[
			Token::Address(token_address),
			Token::Uint(deposit_amount.into()),
			Token::Address(beneficiary),
		]);
		assert_noop!(
			MockEthereumEventRouter::route(&source, &destination, data.clone().as_slice()),
			(
				DbWeight::get().reads(2u64),
				EventRouterError::FailedProcessing(
					DispatchError::Other("Invalid source address")
				)
			)
		);
	});
}

#[test]
fn on_deposit_mints() {
	ExtBuilder.build().execute_with(|| {
		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));
		let token_address: H160 = H160::from_low_u64_be(666);
		let beneficiary: H160 = H160::from_low_u64_be(456);
		let deposit_amount: Balance = 100;
		let expected_asset_id = AssetsExt::next_asset_uuid().unwrap();
		let root_peg_address: H160 = H160::from_low_u64_be(555);
		assert_ok!(Erc20Peg::set_root_peg_address(
			frame_system::RawOrigin::Root.into(),
			root_peg_address
		));

		// No assets expected at first
		assert!(Erc20ToAssetId::<Test>::get(token_address).is_none());

		// Do the deposit
		assert_ok!(Erc20Peg::do_deposit(
			&root_peg_address,
			Erc20DepositEvent { token_address, amount: deposit_amount.into(), beneficiary }
		));
		// Check mapping has been updated
		assert_eq!(Erc20ToAssetId::<Test>::get(token_address), Some(expected_asset_id));
		assert_eq!(AssetIdToErc20::<Test>::get(expected_asset_id), Some(token_address));

		// Check beneficiary account received funds
		assert_eq!(
			AssetsExt::balance(expected_asset_id, &AccountId::from(beneficiary)),
			deposit_amount
		);
	});
}

#[test]
fn on_deposit_fails_with_wrong_source() {
	ExtBuilder.build().execute_with(|| {
		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));

		let beneficiary: H160 = H160::from_low_u64_be(456);
		let deposit_amount: Balance = 0;
		let root_token_address: H160 = H160::from_low_u64_be(666);
		let erc20_token_address: H160 = H160::from_low_u64_be(667);
		let root_peg_address: H160 = H160::from_low_u64_be(555);
		let erc20_peg_address: H160 = H160::from_low_u64_be(444);

		// Set the contract addresses for each
		assert_ok!(Erc20Peg::set_root_peg_address(
			frame_system::RawOrigin::Root.into(),
			root_peg_address
		));
		assert_ok!(Erc20Peg::set_erc20_peg_address(
			frame_system::RawOrigin::Root.into(),
			erc20_peg_address
		));

		// Insert mappings for root and xrp token addresses
		Erc20ToAssetId::<Test>::insert(root_token_address, ROOT_ASSET_ID);
		Erc20ToAssetId::<Test>::insert(erc20_token_address, XRP_ASSET_ID);

		// Do deposit fails with incorrect source
		assert_noop!(
			Erc20Peg::do_deposit(
				&erc20_peg_address,
				Erc20DepositEvent {
					token_address: root_token_address,
					amount: deposit_amount.into(),
					beneficiary
				}
			),
			Error::<Test>::InvalidSourceAddress
		);
		assert_noop!(
			Erc20Peg::do_deposit(
				&root_peg_address,
				Erc20DepositEvent {
					token_address: erc20_token_address,
					amount: deposit_amount.into(),
					beneficiary
				}
			),
			Error::<Test>::InvalidSourceAddress
		);

		// Do deposit works when correct source is supplied
		assert_ok!(Erc20Peg::do_deposit(
			&root_peg_address,
			Erc20DepositEvent {
				token_address: root_token_address,
				amount: deposit_amount.into(),
				beneficiary
			}
		));
		assert_ok!(Erc20Peg::do_deposit(
			&erc20_peg_address,
			Erc20DepositEvent {
				token_address: erc20_token_address,
				amount: deposit_amount.into(),
				beneficiary
			}
		));
	});
}

#[test]
fn on_deposit_transfers_root_token() {
	ExtBuilder.build().execute_with(|| {
		let token_address: H160 = H160::from_low_u64_be(666);
		let beneficiary: H160 = H160::from_low_u64_be(456);
		let deposit_amount: Balance = 1_000_000;
		let root_peg_address: H160 = H160::from_low_u64_be(555);
		assert_ok!(Erc20Peg::set_root_peg_address(
			frame_system::RawOrigin::Root.into(),
			root_peg_address
		));

		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));

		// Setup storage values
		Erc20ToAssetId::<Test>::insert(token_address, ROOT_ASSET_ID);
		AssetIdToErc20::<Test>::insert(ROOT_ASSET_ID, token_address);

		assert_ok!(Erc20Peg::set_root_peg_address(
			frame_system::RawOrigin::Root.into(),
			root_peg_address
		));

		// Mint tokens to peg address (To simulate a withdrawal)
		let pallet_address: AccountId = PegPalletId::get().into_account_truncating();

		let _ =
			<Test as Config>::MultiCurrency::mint_into(ROOT_ASSET_ID, &pallet_address, 1_000_000);
		let root_issuance = AssetsExt::total_issuance(ROOT_ASSET_ID);

		// Do the deposit
		assert_ok!(Erc20Peg::do_deposit(
			&root_peg_address,
			Erc20DepositEvent { token_address, amount: deposit_amount.into(), beneficiary }
		));

		// Check beneficiary account received funds
		assert_eq!(
			AssetsExt::balance(ROOT_ASSET_ID, &AccountId::from(beneficiary)),
			deposit_amount
		);
		// Check peg address has no funds
		assert_eq!(AssetsExt::balance(ROOT_ASSET_ID, &pallet_address), 0);
		// Check total issuance is unchanged
		assert_eq!(AssetsExt::total_issuance(ROOT_ASSET_ID), root_issuance);
	});
}

#[test]
fn deposit_payment_less_than_delay_goes_through() {
	ExtBuilder.build().execute_with(|| {
		let deposit_amount: Balance = 100;
		let beneficiary: H160 = H160::from_low_u64_be(456);
		let erc20_peg_address: H160 = H160::from_low_u64_be(555);
		assert_ok!(Erc20Peg::set_erc20_peg_address(
			frame_system::RawOrigin::Root.into(),
			erc20_peg_address
		));

		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));

		// Setup token mapping
		let token_address: H160 = H160::from_low_u64_be(666);
		Erc20ToAssetId::<Test>::insert(token_address, SPENDING_ASSET_ID);

		// Set payment delay with higher value than deposit_amount
		let delay: u64 = 1000;
		assert_ok!(Erc20Peg::set_payment_delay(
			frame_system::RawOrigin::Root.into(),
			SPENDING_ASSET_ID,
			deposit_amount + 1,
			delay
		));

		// Process deposit, this should go through as the value is less than the payment_delay
		// amount
		assert_ok!(Erc20Peg::do_deposit(
			&erc20_peg_address,
			Erc20DepositEvent { token_address, amount: deposit_amount.into(), beneficiary }
		));

		// Check payment has not been put in delayed payments
		let payment_block = <frame_system::Pallet<Test>>::block_number() + delay;
		let delayed_payment_id = <NextDelayedPaymentId<Test>>::get();
		assert_eq!(
			DelayedPaymentSchedule::<Test>::get(payment_block),
			vec![] as Vec<DelayedPaymentId>
		);
		assert!(DelayedPayments::<Test>::get(delayed_payment_id).is_none());
		assert_eq!(ReadyBlocks::<Test>::get(), vec![] as Vec<u64>);

		// Check beneficiary account received funds
		assert_eq!(
			AssetsExt::balance(SPENDING_ASSET_ID, &AccountId::from(beneficiary)),
			deposit_amount
		);
	});
}

#[test]
fn deposit_payment_with_delay() {
	ExtBuilder.build().execute_with(|| {
		let deposit_amount: Balance = 100;
		let beneficiary: H160 = H160::from_low_u64_be(456);
		let erc20_peg_address: H160 = H160::from_low_u64_be(555);
		assert_ok!(Erc20Peg::set_erc20_peg_address(
			frame_system::RawOrigin::Root.into(),
			erc20_peg_address
		));

		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));

		// Activate deposits delays
		assert_ok!(Erc20Peg::activate_deposits_delay(frame_system::RawOrigin::Root.into(), true));
		// Setup token mapping
		let token_address: H160 = H160::from_low_u64_be(666);
		Erc20ToAssetId::<Test>::insert(token_address, SPENDING_ASSET_ID);

		// Set payment delay with deposit_amount, this should delay the payment
		let delay: u64 = 1000;
		assert_ok!(Erc20Peg::set_payment_delay(
			frame_system::RawOrigin::Root.into(),
			SPENDING_ASSET_ID,
			deposit_amount,
			delay
		));
		let delayed_payment_id = <NextDelayedPaymentId<Test>>::get();

		// Process deposit, this should not go through and be added to delays
		assert_ok!(Erc20Peg::do_deposit(
			&erc20_peg_address,
			Erc20DepositEvent { token_address, amount: deposit_amount.into(), beneficiary }
		));

		// Check payment has been put in delayed payments
		let payment_block = <frame_system::Pallet<Test>>::block_number() + delay;
		let payment =
			Erc20DepositEvent { token_address, amount: deposit_amount.into(), beneficiary };
		assert_eq!(DelayedPaymentSchedule::<Test>::get(payment_block), vec![delayed_payment_id]);
		assert_eq!(
			DelayedPayments::<Test>::get(delayed_payment_id),
			Some(PendingPayment::Deposit(payment.clone()))
		);
		// Check beneficiary account hasn't received funds
		assert_eq!(AssetsExt::balance(SPENDING_ASSET_ID, &AccountId::from(beneficiary)), 0);

		// Simulating block before with enough weight, payment shouldn't be removed
		let delayed_payment_weight: Weight =
			DbWeight::get().reads(8u64).saturating_add(DbWeight::get().writes(10u64));
		assert_eq!(Erc20Peg::on_initialize(payment_block - 1), DbWeight::get().reads(1u64));
		assert_eq!(
			Erc20Peg::on_idle(payment_block - 1, delayed_payment_weight.mul(2u64)),
			Weight::zero()
		);

		// Simulating not enough weight left in block, payment shouldn't be removed
		assert_eq!(
			Erc20Peg::on_initialize(payment_block),
			DbWeight::get().reads(1u64) + DbWeight::get().writes(1u64)
		);
		assert_eq!(
			Erc20Peg::on_idle(payment_block, delayed_payment_weight.div(2)),
			DbWeight::get().reads(1u64)
		);

		// Ensure payment isn't removed from storage after either of the above
		assert_eq!(ReadyBlocks::<Test>::get(), vec![payment_block]);
		assert_eq!(DelayedPaymentSchedule::<Test>::get(payment_block), vec![delayed_payment_id]);
		assert_eq!(
			DelayedPayments::<Test>::get(delayed_payment_id),
			Some(PendingPayment::Deposit(payment.clone()))
		);

		// Try again next block with enough weight
		assert_eq!(Erc20Peg::on_initialize(payment_block + 1), DbWeight::get().reads(1u64));
		assert_eq!(
			Erc20Peg::on_idle(payment_block + 1, delayed_payment_weight * 2),
			delayed_payment_weight + DbWeight::get().reads(1u64)
		);

		// Check payments removed from storage
		assert_eq!(ReadyBlocks::<Test>::get(), vec![] as Vec<u64>);
		assert_eq!(
			DelayedPaymentSchedule::<Test>::get(payment_block),
			vec![] as Vec<DelayedPaymentId>
		);
		assert!(DelayedPayments::<Test>::get(delayed_payment_id).is_none());
		// Check beneficiary account has now received funds
		assert_eq!(
			AssetsExt::balance(SPENDING_ASSET_ID, &AccountId::from(beneficiary)),
			deposit_amount
		);
	});
}

#[test]
fn withdraw() {
	ExtBuilder.build().execute_with(|| {
		let account = create_account(123);
		let asset_id: AssetId = 1;
		let cennz_eth_address: EthAddress = H160::default();
		<AssetIdToErc20<Test>>::insert(asset_id, cennz_eth_address);

		let amount: Balance = 100;
		let _ = <Test as Config>::MultiCurrency::mint_into(asset_id, &account, amount);
		let beneficiary: H160 = H160::from_slice(&hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));

		assert_ok!(Erc20Peg::activate_withdrawals(frame_system::RawOrigin::Root.into(), true));
		assert_eq!(AssetsExt::balance(asset_id, &account), amount);
		assert_ok!(Erc20Peg::withdraw(Some(account).into(), asset_id, amount, beneficiary));
		assert_eq!(AssetsExt::balance(asset_id, &account), 0);
	})
}

#[test]
fn withdraw_with_delay() {
	ExtBuilder.build().execute_with(|| {
		let account: AccountId = create_account(123);
		let asset_id: AssetId = 1;
		let cennz_eth_address: EthAddress = H160::default();
		let amount: Balance = 100;
		let beneficiary: H160 = H160::from_slice(&hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));
		let delay: u64 = 1000;
		let _ = <Test as Config>::MultiCurrency::mint_into(asset_id, &account, amount);
		let delayed_payment_weight: Weight =
			DbWeight::get().reads(8u64).saturating_add(DbWeight::get().writes(10u64));

		<AssetIdToErc20<Test>>::insert(asset_id, cennz_eth_address);
		<Erc20ToAssetId<Test>>::insert(cennz_eth_address, asset_id);
		assert_ok!(Erc20Peg::activate_withdrawals(frame_system::RawOrigin::Root.into(), true));
		// Activate withdrawal delays
		assert_ok!(Erc20Peg::activate_withdrawals_delay(
			frame_system::RawOrigin::Root.into(),
			true
		));

		assert_ok!(Erc20Peg::set_payment_delay(
			frame_system::RawOrigin::Root.into(),
			asset_id,
			amount,
			delay
		));

		let delayed_payment_id = <NextDelayedPaymentId<Test>>::get();
		let payment_block = <frame_system::Pallet<Test>>::block_number() + delay;
		assert_ok!(Erc20Peg::withdraw(Some(account).into(), asset_id, amount, beneficiary));

		// Balance should be withdrawn straight away
		assert_eq!(AssetsExt::balance(asset_id, &account), 0);
		let message = WithdrawMessage {
			token_address: cennz_eth_address,
			amount: amount.into(),
			beneficiary,
		};

		assert_eq!(DelayedPaymentSchedule::<Test>::get(payment_block), vec![delayed_payment_id]);
		assert_eq!(
			DelayedPayments::<Test>::get(delayed_payment_id),
			Some(PendingPayment::Withdrawal((account, message)))
		);
		// Check payment id has been increased
		assert_eq!(<NextDelayedPaymentId<Test>>::get(), delayed_payment_id + 1);
		assert_eq!(
			Erc20Peg::on_initialize(payment_block),
			DbWeight::get().reads(1u64) + DbWeight::get().writes(1u64)
		);
		assert_eq!(
			Erc20Peg::on_idle(payment_block, delayed_payment_weight * 2),
			delayed_payment_weight + DbWeight::get().reads(1u64)
		);
		// Payment should be removed from storage
		assert_eq!(
			DelayedPaymentSchedule::<Test>::get(payment_block),
			vec![] as Vec<DelayedPaymentId>
		);
		assert!(DelayedPayments::<Test>::get(delayed_payment_id).is_none());
	});
}

#[test]
fn root_can_claim_delayed_payment() {
	ExtBuilder::default().build().execute_with(|| {
		let account: AccountId = create_account(123);
		let asset_id: AssetId = 1;
		let cennz_eth_address: EthAddress = H160::default();
		let amount: Balance = 100;
		let beneficiary: H160 = H160::from_slice(&hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));
		let delay: u64 = 1000;
		let _ = <Test as Config>::MultiCurrency::mint_into(asset_id, &account, amount);

		<AssetIdToErc20<Test>>::insert(asset_id, cennz_eth_address);
		<Erc20ToAssetId<Test>>::insert(cennz_eth_address, asset_id);

		assert_ok!(Erc20Peg::activate_withdrawals(frame_system::RawOrigin::Root.into(), true));
		// Activate withdrawal delays
		assert_ok!(Erc20Peg::activate_withdrawals_delay(
			frame_system::RawOrigin::Root.into(),
			true
		));

		assert_ok!(Erc20Peg::set_payment_delay(
			frame_system::RawOrigin::Root.into(),
			asset_id,
			amount,
			delay
		));

		let delayed_payment_id = <NextDelayedPaymentId<Test>>::get();
		let payment_block = <frame_system::Pallet<Test>>::block_number() + delay;
		assert_ok!(Erc20Peg::withdraw(Some(account.clone()).into(), asset_id, amount, beneficiary));
		let message = WithdrawMessage {
			token_address: cennz_eth_address,
			amount: amount.into(),
			beneficiary,
		};

		assert_eq!(DelayedPaymentSchedule::<Test>::get(payment_block), vec![delayed_payment_id]);
		assert_eq!(
			DelayedPayments::<Test>::get(delayed_payment_id),
			Some(PendingPayment::Withdrawal((account, message)))
		);

		assert_ok!(Erc20Peg::claim_delayed_payment(
			frame_system::RawOrigin::Root.into(),
			payment_block,
			delayed_payment_id,
		));

		// Payment should be removed from storage
		assert_eq!(
			DelayedPaymentSchedule::<Test>::get(payment_block),
			vec![] as Vec<DelayedPaymentId>
		);
		assert!(DelayedPayments::<Test>::get(delayed_payment_id).is_none());
	})
}

#[test]
fn root_claim_on_delayed_payment_doesnt_effect_prior_delayed_payments() {
	ExtBuilder::default().build().execute_with(|| {
		let account: AccountId = create_account(123);
		let asset_id: AssetId = 1;
		let cennz_eth_address: EthAddress = H160::default();
		let amount: Balance = 200;
		let half_amount: Balance = 100;
		let beneficiary: H160 = H160::from_slice(&hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));
		let delay: u64 = 1000;
		let _ = <Test as Config>::MultiCurrency::mint_into(asset_id, &account, amount);

		<AssetIdToErc20<Test>>::insert(asset_id, cennz_eth_address);
		<Erc20ToAssetId<Test>>::insert(cennz_eth_address, asset_id);

		assert_ok!(Erc20Peg::activate_withdrawals(frame_system::RawOrigin::Root.into(), true));

		// Activate withdrawal delays
		assert_ok!(Erc20Peg::activate_withdrawals_delay(
			frame_system::RawOrigin::Root.into(),
			true
		));

		assert_ok!(Erc20Peg::set_payment_delay(
			frame_system::RawOrigin::Root.into(),
			asset_id,
			half_amount,
			delay
		));

		let payment_block = <frame_system::Pallet<Test>>::block_number() + delay;

		let delayed_payment_id = <NextDelayedPaymentId<Test>>::get();
		let delayed_payment_id_two = delayed_payment_id.saturating_add(1);

		assert_ok!(Erc20Peg::withdraw(
			Some(account.clone()).into(),
			asset_id,
			half_amount,
			beneficiary
		));
		assert_ok!(Erc20Peg::withdraw(
			Some(account.clone()).into(),
			asset_id,
			half_amount,
			beneficiary
		));

		let message = WithdrawMessage {
			token_address: cennz_eth_address,
			amount: half_amount.into(),
			beneficiary,
		};

		assert_eq!(
			DelayedPaymentSchedule::<Test>::get(payment_block),
			vec![delayed_payment_id, delayed_payment_id_two]
		);
		assert_eq!(
			DelayedPayments::<Test>::get(delayed_payment_id),
			Some(PendingPayment::Withdrawal((account, message.clone())))
		);
		assert_eq!(
			DelayedPayments::<Test>::get(delayed_payment_id_two),
			Some(PendingPayment::Withdrawal((account, message)))
		);

		assert_ok!(Erc20Peg::claim_delayed_payment(
			frame_system::RawOrigin::Root.into(),
			payment_block,
			delayed_payment_id,
		));

		assert_eq!(
			DelayedPaymentSchedule::<Test>::get(payment_block),
			vec![delayed_payment_id_two]
		);
		assert!(DelayedPayments::<Test>::get(delayed_payment_id).is_none());
		assert!(DelayedPayments::<Test>::get(delayed_payment_id_two).is_some());
	})
}

#[test]
fn root_claim_fails_with_non_existant_block() {
	ExtBuilder::default().build().execute_with(|| {
		let payment_block = <frame_system::Pallet<Test>>::block_number();
		let delayed_payment_id = <NextDelayedPaymentId<Test>>::get();

		assert_noop!(
			Erc20Peg::claim_delayed_payment(
				frame_system::RawOrigin::Root.into(),
				payment_block,
				delayed_payment_id,
			),
			Error::<Test>::PaymentIdNotFound
		);
	})
}

#[test]
fn root_claim_fails_with_non_existant_payment_id() {
	ExtBuilder::default().build().execute_with(|| {
		let payment_block = <frame_system::Pallet<Test>>::block_number();
		let delayed_payment_id = <NextDelayedPaymentId<Test>>::get();

		let non_existant_payment_id: u64 = 999;

		DelayedPaymentSchedule::<Test>::insert(
			payment_block,
			WeakBoundedVec::try_from(vec![delayed_payment_id]).unwrap(),
		);

		assert_noop!(
			Erc20Peg::claim_delayed_payment(
				frame_system::RawOrigin::Root.into(),
				payment_block,
				non_existant_payment_id, // This payment id doesn't exist at this key
			),
			Error::<Test>::PaymentIdNotFound
		);
	})
}

#[test]
fn withdraw_less_than_delay_goes_through() {
	ExtBuilder.build().execute_with(|| {
		let account: AccountId = create_account(123);
		let asset_id: AssetId = 1;
		let cennz_eth_address: EthAddress = H160::default();
		let amount: Balance = 100;
		let beneficiary: H160 = H160::from_slice(&hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));
		let delay: u64 = 1000;
		let _ = <Test as Config>::MultiCurrency::mint_into(asset_id, &account, amount);

		<AssetIdToErc20<Test>>::insert(asset_id, cennz_eth_address);
		<Erc20ToAssetId<Test>>::insert(cennz_eth_address, asset_id);
		assert_ok!(Erc20Peg::activate_withdrawals(frame_system::RawOrigin::Root.into(), true));

		assert_ok!(Erc20Peg::set_payment_delay(
			frame_system::RawOrigin::Root.into(),
			asset_id,
			amount,
			delay
		));

		let delayed_payment_id = <NextDelayedPaymentId<Test>>::get();
		let payment_block = <frame_system::Pallet<Test>>::block_number() + delay;
		assert_ok!(Erc20Peg::withdraw(
			Some(account).into(),
			asset_id,
			amount - 1,
			beneficiary
		));
		assert_eq!(
			DelayedPaymentSchedule::<Test>::get(payment_block),
			vec![] as Vec<DelayedPaymentId>
		);
		assert!(DelayedPayments::<Test>::get(delayed_payment_id).is_none());
	});
}

#[test]
fn withdraw_unsupported_asset_should_fail() {
	ExtBuilder.build().execute_with(|| {
		let account: AccountId = create_account(123);
		let asset_id: AssetId = 1;
		let amount: Balance = 100;
		let beneficiary: H160 = H160::from_slice(&hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));

		assert_ok!(Erc20Peg::activate_withdrawals(frame_system::RawOrigin::Root.into(), true));

		assert_noop!(
			Erc20Peg::withdraw(Some(account).into(), asset_id, amount, beneficiary),
			Error::<Test>::UnsupportedAsset
		);
	});
}

#[test]
fn withdraw_not_active_should_fail() {
	ExtBuilder.build().execute_with(|| {
		let account: AccountId = create_account(123);
		let asset_id: AssetId = 1;
		let amount: Balance = 100;
		let beneficiary: H160 = H160::from_slice(&hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));

		assert_noop!(
			Erc20Peg::withdraw(Some(account).into(), asset_id, amount, beneficiary),
			Error::<Test>::WithdrawalsPaused
		);
	});
}

#[test]
fn withdraw_transfers_root_token() {
	ExtBuilder.build().execute_with(|| {
		let token_address: H160 = H160::from_low_u64_be(666);
		let account: AccountId = create_account(456);
		let beneficiary: H160 = H160::from_low_u64_be(457);
		let withdraw_amount: Balance = 1_000_000;

		// Activate withdrawals
		assert_ok!(Erc20Peg::activate_withdrawals(frame_system::RawOrigin::Root.into(), true));

		// Setup storage values
		AssetIdToErc20::<Test>::insert(ROOT_ASSET_ID, token_address);

		// Mint tokens to account
		let _ =
			<Test as Config>::MultiCurrency::mint_into(ROOT_ASSET_ID, &account, withdraw_amount);
		let root_issuance = AssetsExt::total_issuance(ROOT_ASSET_ID);
		let pallet_address: AccountId = PegPalletId::get().into_account_truncating();
		// Check initial balances
		assert_eq!(AssetsExt::balance(ROOT_ASSET_ID, &pallet_address), 0);
		assert_eq!(AssetsExt::balance(ROOT_ASSET_ID, &account), withdraw_amount);

		// Do the withdrawal
		assert_ok!(Erc20Peg::withdraw(
			Some(account).into(),
			ROOT_ASSET_ID,
			withdraw_amount,
			beneficiary
		));

		// Check account has no funds
		assert_eq!(AssetsExt::balance(ROOT_ASSET_ID, &account), 0);
		// Check peg address has withdrawn funds
		assert_eq!(AssetsExt::balance(ROOT_ASSET_ID, &pallet_address), withdraw_amount);
		// Check total issuance is unchanged
		assert_eq!(AssetsExt::total_issuance(ROOT_ASSET_ID), root_issuance);
	});
}
