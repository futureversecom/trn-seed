use super::*;
use crate::{
	mock::{
		AssetsExt, Erc20Peg, ExtBuilder, MockEthereumEventRouter, System, Test, SPENDING_ASSET_ID,
	},
	types::{DelayedPaymentId, Erc20DepositEvent, PendingPayment, WithdrawMessage},
};
use frame_support::{
	assert_noop, assert_ok,
	traits::{
		fungibles::{Inspect, Mutate},
		OnIdle, OnInitialize,
	},
	weights::constants::RocksDbWeight as DbWeight,
};
use hex_literal::hex;
use seed_pallet_common::EthereumEventRouter;

fn make_account_id(seed: u64) -> AccountId {
	AccountId::from(H160::from_low_u64_be(seed))
}

#[test]
fn set_payment_delay() {
	ExtBuilder::default().build().execute_with(|| {
		let asset_id: AssetId = 1;
		let min_balance: Balance = 100;
		let delay: u64 = 1000;
		assert_ok!(Erc20Peg::set_payment_delay(
			frame_system::RawOrigin::Root.into(),
			asset_id,
			min_balance,
			delay
		));
		assert_eq!(Erc20Peg::payment_delay(asset_id), Some((min_balance, delay)));
	});
}

#[test]
fn deposit_payment_with_ethereum_event_router() {
	ExtBuilder::default().build().execute_with(|| {
		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));

		// Setup token mapping
		let token_address: H160 = H160::from_low_u64_be(666);
		Erc20ToAssetId::insert(token_address, SPENDING_ASSET_ID);

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
		assert_ok!(MockEthereumEventRouter::route(&source, &destination, data.clone().as_slice()));

		// Check beneficiary account received funds
		assert_eq!(
			AssetsExt::balance(SPENDING_ASSET_ID, &AccountId::from(beneficiary)),
			deposit_amount
		);
	});
}

#[test]
fn on_deposit_mints() {
	ExtBuilder::default().build().execute_with(|| {
		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));

		let token_address: H160 = H160::from_low_u64_be(666);
		let beneficiary: H160 = H160::from_low_u64_be(456);
		let deposit_amount: Balance = 100;
		let expected_asset_id = AssetsExt::next_asset_uuid().unwrap();

		// No assets expected at first
		assert!(Erc20Peg::erc20_to_asset(token_address).is_none());

		// Do the deposit
		assert_ok!(Erc20Peg::do_deposit(Erc20DepositEvent {
			token_address,
			amount: deposit_amount.into(),
			beneficiary
		}));
		// Check mapping has been updated
		assert_eq!(Erc20Peg::erc20_to_asset(token_address), Some(expected_asset_id));
		assert_eq!(Erc20Peg::asset_to_erc20(expected_asset_id), Some(token_address));

		// Check beneficiary account received funds
		assert_eq!(
			AssetsExt::balance(expected_asset_id, &AccountId::from(beneficiary)),
			deposit_amount
		);
	});
}

#[test]
fn deposit_payment_less_than_delay_goes_through() {
	ExtBuilder::default().build().execute_with(|| {
		let deposit_amount: Balance = 100;
		let beneficiary: H160 = H160::from_low_u64_be(456);

		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));

		// Setup token mapping
		let token_address: H160 = H160::from_low_u64_be(666);
		Erc20ToAssetId::insert(token_address, SPENDING_ASSET_ID);

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
		assert_ok!(Erc20Peg::do_deposit(Erc20DepositEvent {
			token_address,
			amount: deposit_amount.into(),
			beneficiary
		}));

		// Check payment has not been put in delayed payments
		let payment_block = <frame_system::Pallet<Test>>::block_number() + delay;
		let delayed_payment_id = <NextDelayedPaymentId>::get();
		assert_eq!(
			Erc20Peg::delayed_payment_schedule(payment_block),
			vec![] as Vec<DelayedPaymentId>
		);
		assert!(Erc20Peg::delayed_payments(delayed_payment_id).is_none());
		assert_eq!(Erc20Peg::ready_blocks(), vec![] as Vec<u64>);

		// Check beneficiary account received funds
		assert_eq!(
			AssetsExt::balance(SPENDING_ASSET_ID, &AccountId::from(beneficiary)),
			deposit_amount
		);
	});
}

#[test]
fn deposit_payment_with_delay() {
	ExtBuilder::default().build().execute_with(|| {
		let deposit_amount: Balance = 100;
		let beneficiary: H160 = H160::from_low_u64_be(456);

		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));

		// Setup token mapping
		let token_address: H160 = H160::from_low_u64_be(666);
		Erc20ToAssetId::insert(token_address, SPENDING_ASSET_ID);

		// Set payment delay with deposit_amount, this should delay the payment
		let delay: u64 = 1000;
		assert_ok!(Erc20Peg::set_payment_delay(
			frame_system::RawOrigin::Root.into(),
			SPENDING_ASSET_ID,
			deposit_amount,
			delay
		));
		let delayed_payment_id = <NextDelayedPaymentId>::get();

		// Process deposit, this should not go through and be added to delays
		assert_ok!(Erc20Peg::do_deposit(Erc20DepositEvent {
			token_address,
			amount: deposit_amount.into(),
			beneficiary
		}));

		// Check payment has been put in delayed payments
		let payment_block = <frame_system::Pallet<Test>>::block_number() + delay;
		let payment =
			Erc20DepositEvent { token_address, amount: deposit_amount.into(), beneficiary };
		assert_eq!(Erc20Peg::delayed_payment_schedule(payment_block), vec![delayed_payment_id]);
		assert_eq!(
			Erc20Peg::delayed_payments(delayed_payment_id),
			Some(PendingPayment::Deposit(payment.clone()))
		);
		// Check beneficiary account hasn't received funds
		assert_eq!(AssetsExt::balance(SPENDING_ASSET_ID, &AccountId::from(beneficiary)), 0);

		// Simulating block before with enough weight, payment shouldn't be removed
		let delayed_payment_weight: Weight = DbWeight::get()
			.reads(8 as Weight)
			.saturating_add(DbWeight::get().writes(10 as Weight));
		assert_eq!(Erc20Peg::on_initialize(payment_block - 1), DbWeight::get().reads(1 as Weight));
		assert_eq!(Erc20Peg::on_idle(payment_block - 1, delayed_payment_weight * 2), 0);

		// Simulating not enough weight left in block, payment shouldn't be removed
		assert_eq!(
			Erc20Peg::on_initialize(payment_block),
			DbWeight::get().reads(1 as Weight) + DbWeight::get().writes(1 as Weight)
		);
		assert_eq!(
			Erc20Peg::on_idle(payment_block, delayed_payment_weight / 2),
			DbWeight::get().reads(1 as Weight)
		);

		// Ensure payment isn't removed from storage after either of the above
		assert_eq!(Erc20Peg::ready_blocks(), vec![payment_block]);
		assert_eq!(Erc20Peg::delayed_payment_schedule(payment_block), vec![delayed_payment_id]);
		assert_eq!(
			Erc20Peg::delayed_payments(delayed_payment_id),
			Some(PendingPayment::Deposit(payment.clone()))
		);

		// Try again next block with enough weight
		assert_eq!(Erc20Peg::on_initialize(payment_block + 1), DbWeight::get().reads(1 as Weight));
		assert_eq!(
			Erc20Peg::on_idle(payment_block + 1, delayed_payment_weight * 2),
			delayed_payment_weight + DbWeight::get().reads(1 as Weight)
		);

		// Check payments removed from storage
		assert_eq!(Erc20Peg::ready_blocks(), vec![] as Vec<u64>);
		assert_eq!(
			Erc20Peg::delayed_payment_schedule(payment_block),
			vec![] as Vec<DelayedPaymentId>
		);
		assert!(Erc20Peg::delayed_payments(delayed_payment_id).is_none());
		// Check beneficiary account has now received funds
		assert_eq!(
			AssetsExt::balance(SPENDING_ASSET_ID, &AccountId::from(beneficiary)),
			deposit_amount
		);
	});
}

#[test]
fn multiple_deposit_payments_with_delay() {
	ExtBuilder::default().build().execute_with(|| {
		let deposit_amount: Balance = 100;
		let beneficiary: H160 = H160::from_low_u64_be(456);

		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));

		// Setup token mapping
		let token_address: H160 = H160::from_low_u64_be(666);
		Erc20ToAssetId::insert(token_address, SPENDING_ASSET_ID);

		// Set payment delay with deposit_amount, this should delay the payment
		let delay: u64 = 1000;
		assert_ok!(Erc20Peg::set_payment_delay(
			frame_system::RawOrigin::Root.into(),
			SPENDING_ASSET_ID,
			deposit_amount,
			delay
		));

		let payment_block = <frame_system::Pallet<Test>>::block_number() + delay;
		let payment =
			Erc20DepositEvent { token_address, amount: deposit_amount.into(), beneficiary };

		// Deposit more payments than u8::MAX
		let payment_count: u64 = 300;
		let mut delayed_payment_ids: Vec<DelayedPaymentId> = vec![];
		for _ in 0..payment_count {
			let delayed_payment_id = <NextDelayedPaymentId>::get();
			delayed_payment_ids.push(delayed_payment_id);
			assert_ok!(Erc20Peg::do_deposit(payment.clone()));

			// Check payment has been put into pending payments
			assert_eq!(
				Erc20Peg::delayed_payments(delayed_payment_id),
				Some(PendingPayment::Deposit(payment.clone()))
			);
		}
		assert_eq!(Erc20Peg::delayed_payment_schedule(payment_block), delayed_payment_ids.clone());

		// Call on_idle with room for all payments
		let delayed_payment_weight: Weight = DbWeight::get()
			.reads(8 as Weight)
			.saturating_add(DbWeight::get().writes(10 as Weight));
		assert_eq!(
			Erc20Peg::on_initialize(payment_block),
			DbWeight::get().reads(1 as Weight) + DbWeight::get().writes(1 as Weight)
		);
		assert_eq!(
			Erc20Peg::on_idle(
				payment_block,
				payment_count * delayed_payment_weight + DbWeight::get().reads(1 as Weight)
			),
			u8::MAX as u64 * delayed_payment_weight + DbWeight::get().reads(1 as Weight)
		);

		// Check that we have processed u8::MAX payments
		let mut changed_count = 0;
		for i in 0..payment_count {
			if Erc20Peg::delayed_payments(delayed_payment_ids[i as usize]) == None {
				changed_count += 1;
			}
		}
		assert_eq!(changed_count, u8::MAX);
		assert_eq!(
			Erc20Peg::delayed_payment_schedule(payment_block),
			delayed_payment_ids[u8::MAX as usize..]
		);
		assert_eq!(Erc20Peg::ready_blocks(), vec![payment_block]);

		// Now process the rest of the payments
		assert_eq!(Erc20Peg::on_initialize(payment_block + 1), DbWeight::get().reads(1 as Weight));
		assert_eq!(
			Erc20Peg::on_idle(
				payment_block + 1,
				payment_count * delayed_payment_weight + DbWeight::get().reads(1 as Weight)
			),
			(payment_count - u8::MAX as u64) * delayed_payment_weight +
				DbWeight::get().reads(1 as Weight)
		);

		// All payments should now be processed
		for i in 0..payment_count {
			assert!(Erc20Peg::delayed_payments(delayed_payment_ids[i as usize]).is_none());
		}
		assert_eq!(Erc20Peg::ready_blocks(), vec![] as Vec<u64>);
		assert_eq!(Erc20Peg::delayed_payment_schedule(payment_block), vec![] as Vec<u64>);
		// Check beneficiary account is now rich with funds from all payments
		assert_eq!(
			AssetsExt::balance(SPENDING_ASSET_ID, &AccountId::from(beneficiary)),
			deposit_amount * (payment_count as Balance)
		);
	});
}

#[test]
fn many_deposit_payments_with_delay() {
	ExtBuilder::default().build().execute_with(|| {
		let deposit_amount: Balance = 100;
		let beneficiary: H160 = H160::from_low_u64_be(456);

		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));

		// Setup token mapping
		let token_address: H160 = H160::from_low_u64_be(666);
		Erc20ToAssetId::insert(token_address, SPENDING_ASSET_ID);

		// Set payment delay with deposit_amount, this should delay the payment
		let delay: u64 = 1000;
		assert_ok!(Erc20Peg::set_payment_delay(
			frame_system::RawOrigin::Root.into(),
			SPENDING_ASSET_ID,
			deposit_amount,
			delay
		));

		let payment_block = <frame_system::Pallet<Test>>::block_number() + delay;
		let payment =
			Erc20DepositEvent { token_address, amount: deposit_amount.into(), beneficiary };
		let delayed_payment_weight: Weight = DbWeight::get()
			.reads(8 as Weight)
			.saturating_add(DbWeight::get().writes(10 as Weight));
		let payment_count: u64 = 50;
		let mut delayed_payment_ids: Vec<DelayedPaymentId> = vec![];
		let mut payment_blocks: Vec<u64> = vec![];

		// Process all payments, this time incrementing the block number between each payment
		for i in 0..payment_count {
			let delayed_payment_id = <NextDelayedPaymentId>::get();
			delayed_payment_ids.push(delayed_payment_id);
			assert_ok!(Erc20Peg::do_deposit(payment.clone()));

			// Check payment has been put into pending payments
			assert_eq!(
				Erc20Peg::delayed_payment_schedule(payment_block + i),
				vec![delayed_payment_id.clone()]
			);
			assert_eq!(
				Erc20Peg::delayed_payments(delayed_payment_id),
				Some(PendingPayment::Deposit(payment.clone()))
			);
			// Go to next block
			payment_blocks.push(payment_block + i);
			System::set_block_number(System::block_number() + 1);
		}

		// Go through each block and process payment with on_idle
		for i in 0..payment_count {
			assert_eq!(
				Erc20Peg::on_initialize(payment_blocks[i as usize]),
				DbWeight::get().reads(1 as Weight) + DbWeight::get().writes(1 as Weight)
			);
			assert_eq!(
				Erc20Peg::on_idle(
					payment_blocks[i as usize],
					delayed_payment_weight + DbWeight::get().reads(1 as Weight)
				),
				delayed_payment_weight + DbWeight::get().reads(1 as Weight)
			);
			// Check storage is removed at this block
			assert!(Erc20Peg::delayed_payments(delayed_payment_ids[i as usize]).is_none());
			assert_eq!(Erc20Peg::ready_blocks(), vec![] as Vec<u64>);
			assert_eq!(Erc20Peg::delayed_payment_schedule(payment_block), vec![] as Vec<u64>);
		}

		// Check beneficiary account is now rich with funds from all payments
		assert_eq!(
			AssetsExt::balance(SPENDING_ASSET_ID, &AccountId::from(beneficiary)),
			deposit_amount * (payment_count as Balance)
		);
	});
}

#[test]
fn withdraw() {
	ExtBuilder::default().build().execute_with(|| {
		let account = make_account_id(123);
		let asset_id: AssetId = 1;
		let cennz_eth_address: EthAddress = H160::default();
		<AssetIdToErc20>::insert(asset_id, cennz_eth_address);

		let amount: Balance = 100;
		let _ = <Test as Config>::MultiCurrency::mint_into(asset_id, &account, amount);
		let beneficiary: H160 = H160::from_slice(&hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));

		assert_ok!(Erc20Peg::activate_withdrawals(frame_system::RawOrigin::Root.into(), true));
		assert_eq!(AssetsExt::balance(asset_id, &account), amount);
		assert_ok!(Erc20Peg::withdraw(Some(account.clone()).into(), asset_id, amount, beneficiary));
		assert_eq!(AssetsExt::balance(asset_id, &account), 0);
	})
}

#[test]
fn withdraw_with_delay() {
	ExtBuilder::default().build().execute_with(|| {
		let account: AccountId = make_account_id(123);
		let asset_id: AssetId = 1;
		let cennz_eth_address: EthAddress = H160::default();
		let amount: Balance = 100;
		let beneficiary: H160 = H160::from_slice(&hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));
		let delay: u64 = 1000;
		let _ = <Test as Config>::MultiCurrency::mint_into(asset_id, &account, amount);
		let delayed_payment_weight: Weight = DbWeight::get()
			.reads(8 as Weight)
			.saturating_add(DbWeight::get().writes(10 as Weight));

		<AssetIdToErc20>::insert(asset_id, cennz_eth_address);
		<Erc20ToAssetId>::insert(cennz_eth_address, asset_id);
		assert_ok!(Erc20Peg::activate_withdrawals(frame_system::RawOrigin::Root.into(), true));

		assert_ok!(Erc20Peg::set_payment_delay(
			frame_system::RawOrigin::Root.into(),
			asset_id,
			amount,
			delay
		));

		let delayed_payment_id = <NextDelayedPaymentId>::get();
		let payment_block = <frame_system::Pallet<Test>>::block_number() + delay;
		assert_ok!(Erc20Peg::withdraw(Some(account.clone()).into(), asset_id, amount, beneficiary));

		// Balance should be withdrawn straight away
		assert_eq!(AssetsExt::balance(asset_id, &account), 0);
		let message = WithdrawMessage {
			token_address: cennz_eth_address,
			amount: amount.into(),
			beneficiary,
		};

		assert_eq!(Erc20Peg::delayed_payment_schedule(payment_block), vec![delayed_payment_id]);
		assert_eq!(
			Erc20Peg::delayed_payments(delayed_payment_id),
			Some(PendingPayment::Withdrawal(message))
		);
		// Check payment id has been increased
		assert_eq!(<NextDelayedPaymentId>::get(), delayed_payment_id + 1);
		assert_eq!(
			Erc20Peg::on_initialize(payment_block),
			DbWeight::get().reads(1 as Weight) + DbWeight::get().writes(1 as Weight)
		);
		assert_eq!(
			Erc20Peg::on_idle(payment_block, delayed_payment_weight * 2),
			delayed_payment_weight + DbWeight::get().reads(1 as Weight)
		);
		// Payment should be removed from storage
		assert_eq!(
			Erc20Peg::delayed_payment_schedule(payment_block),
			vec![] as Vec<DelayedPaymentId>
		);
		assert!(Erc20Peg::delayed_payments(delayed_payment_id).is_none());
	});
}

#[test]
fn withdraw_less_than_delay_goes_through() {
	ExtBuilder::default().build().execute_with(|| {
		let account: AccountId = make_account_id(123);
		let asset_id: AssetId = 1;
		let cennz_eth_address: EthAddress = H160::default();
		let amount: Balance = 100;
		let beneficiary: H160 = H160::from_slice(&hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));
		let delay: u64 = 1000;
		let _ = <Test as Config>::MultiCurrency::mint_into(asset_id, &account, amount);

		<AssetIdToErc20>::insert(asset_id, cennz_eth_address);
		<Erc20ToAssetId>::insert(cennz_eth_address, asset_id);
		assert_ok!(Erc20Peg::activate_withdrawals(frame_system::RawOrigin::Root.into(), true));

		assert_ok!(Erc20Peg::set_payment_delay(
			frame_system::RawOrigin::Root.into(),
			asset_id,
			amount,
			delay
		));

		let delayed_payment_id = <NextDelayedPaymentId>::get();
		let payment_block = <frame_system::Pallet<Test>>::block_number() + delay;
		assert_ok!(Erc20Peg::withdraw(
			Some(account.clone()).into(),
			asset_id,
			amount - 1,
			beneficiary
		));
		assert_eq!(
			Erc20Peg::delayed_payment_schedule(payment_block),
			vec![] as Vec<DelayedPaymentId>
		);
		assert!(Erc20Peg::delayed_payments(delayed_payment_id).is_none());
	});
}

#[test]
fn withdraw_unsupported_asset_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let account: AccountId = make_account_id(123);
		let asset_id: AssetId = 1;
		let amount: Balance = 100;
		let beneficiary: H160 = H160::from_slice(&hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));

		assert_ok!(Erc20Peg::activate_withdrawals(frame_system::RawOrigin::Root.into(), true));

		assert_noop!(
			Erc20Peg::withdraw(Some(account.clone()).into(), asset_id, amount, beneficiary),
			Error::<Test>::UnsupportedAsset
		);
	});
}

#[test]
fn withdraw_not_active_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let account: AccountId = make_account_id(123);
		let asset_id: AssetId = 1;
		let amount: Balance = 100;
		let beneficiary: H160 = H160::from_slice(&hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));

		assert_noop!(
			Erc20Peg::withdraw(Some(account.clone()).into(), asset_id, amount, beneficiary),
			Error::<Test>::WithdrawalsPaused
		);
	});
}
