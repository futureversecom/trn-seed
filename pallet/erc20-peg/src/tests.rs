use super::*;
use crate::{
	mock::{
		AssetsExt, Erc20Peg, ExtBuilder, MockEthereumEventRouter, System, Test, SPENDING_ASSET_ID,
	},
	types::{ClaimId, Erc20DepositEvent, PendingClaim, WithdrawMessage},
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

#[test]
fn set_claim_delay() {
	ExtBuilder::default().build().execute_with(|| {
		let asset_id: AssetId = 1;
		let min_balance: Balance = 100;
		let delay: u64 = 1000;
		assert_ok!(Erc20Peg::set_claim_delay(
			frame_system::RawOrigin::Root.into(),
			asset_id,
			min_balance,
			delay
		));
		assert_eq!(Erc20Peg::claim_delay(asset_id), Some((min_balance, delay)));
	});
}

#[test]
fn deposit_claim() {
	ExtBuilder::default().build().execute_with(|| {
		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));

		// Setup token mapping
		let token_address: H160 = H160::from_low_u64_be(666);
		Erc20ToAssetId::insert(token_address, SPENDING_ASSET_ID);

		let source = H160::from_low_u64_be(123);
		let destination = <Test as Config>::PalletId::get().into_account_truncating();
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

		let source = H160::from_low_u64_be(123);
		let token_address: H160 = H160::from_low_u64_be(666);
		let beneficiary: H160 = H160::from_low_u64_be(456);
		let destination = <Test as Config>::PalletId::get().into_account_truncating();
		let deposit_amount: Balance = 100;
		let expected_asset_id = AssetsExt::next_asset_uuid().unwrap();

		// No assets expected at first
		assert!(Erc20Peg::erc20_to_asset(token_address).is_none());

		// Encode data for bridge call
		let data = ethabi::encode(&[
			Token::Address(token_address),
			Token::Uint(deposit_amount.into()),
			Token::Address(beneficiary),
		]);
		assert_ok!(MockEthereumEventRouter::route(&source, &destination, data.clone().as_slice()));

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
fn deposit_claim_less_than_delay_goes_through() {
	ExtBuilder::default().build().execute_with(|| {
		let source = H160::from_low_u64_be(123);
		let destination = <Test as Config>::PalletId::get().into_account_truncating();
		let deposit_amount: Balance = 100;
		let beneficiary: H160 = H160::from_low_u64_be(456);

		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));

		// Setup token mapping
		let token_address: H160 = H160::from_low_u64_be(666);
		Erc20ToAssetId::insert(token_address, SPENDING_ASSET_ID);

		// Set claim delay with higher value than deposit_amount
		let delay: u64 = 1000;
		assert_ok!(Erc20Peg::set_claim_delay(
			frame_system::RawOrigin::Root.into(),
			SPENDING_ASSET_ID,
			deposit_amount + 1,
			delay
		));
		let claim_id = <NextDelayedClaimId>::get();

		// Encode data for bridge call
		let data = ethabi::encode(&[
			Token::Address(token_address),
			Token::Uint(deposit_amount.into()),
			Token::Address(beneficiary),
		]);
		// Process deposit, this should go through as the value is less than the claim_delay amount
		assert_ok!(MockEthereumEventRouter::route(&source, &destination, data.clone().as_slice()));

		// Check claim has not been put in delayed claims
		let claim_block = <frame_system::Pallet<Test>>::block_number() + delay;
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), vec![] as Vec<ClaimId>);
		assert!(Erc20Peg::delayed_claims(claim_id).is_none());
		assert_eq!(Erc20Peg::ready_blocks(), vec![] as Vec<u64>);

		// Check beneficiary account received funds
		assert_eq!(
			AssetsExt::balance(SPENDING_ASSET_ID, &AccountId::from(beneficiary)),
			deposit_amount
		);
	});
}

#[test]
fn deposit_claim_with_delay() {
	ExtBuilder::default().build().execute_with(|| {
		let source = H160::from_low_u64_be(123);
		let destination = <Test as Config>::PalletId::get().into_account_truncating();
		let deposit_amount: Balance = 100;
		let beneficiary: H160 = H160::from_low_u64_be(456);

		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));

		// Setup token mapping
		let token_address: H160 = H160::from_low_u64_be(666);
		Erc20ToAssetId::insert(token_address, SPENDING_ASSET_ID);

		// Set claim delay with deposit_amount, this should delay the claim
		let delay: u64 = 1000;
		assert_ok!(Erc20Peg::set_claim_delay(
			frame_system::RawOrigin::Root.into(),
			SPENDING_ASSET_ID,
			deposit_amount,
			delay
		));
		let claim_id = <NextDelayedClaimId>::get();

		// Encode data for bridge call
		let data = ethabi::encode(&[
			Token::Address(token_address),
			Token::Uint(deposit_amount.into()),
			Token::Address(beneficiary),
		]);
		// Process deposit, this should not go through and be added to delays
		assert_ok!(MockEthereumEventRouter::route(&source, &destination, data.clone().as_slice()));

		// Check claim has been put in delayed claims
		let claim_block = <frame_system::Pallet<Test>>::block_number() + delay;
		let claim = Erc20DepositEvent { token_address, amount: deposit_amount.into(), beneficiary };
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), vec![claim_id]);
		assert_eq!(Erc20Peg::delayed_claims(claim_id), Some(PendingClaim::Deposit(claim.clone())));
		// Check beneficiary account hasn't received funds
		assert_eq!(AssetsExt::balance(SPENDING_ASSET_ID, &AccountId::from(beneficiary)), 0);

		// Simulating block before with enough weight, claim shouldn't be removed
		let delayed_claim_weight: Weight = DbWeight::get()
			.reads(8 as Weight)
			.saturating_add(DbWeight::get().writes(10 as Weight));
		assert_eq!(Erc20Peg::on_initialize(claim_block - 1), DbWeight::get().reads(1 as Weight));
		assert_eq!(Erc20Peg::on_idle(claim_block - 1, delayed_claim_weight * 2), 0);

		// Simulating not enough weight left in block, claim shouldn't be removed
		assert_eq!(
			Erc20Peg::on_initialize(claim_block),
			DbWeight::get().reads(1 as Weight) + DbWeight::get().writes(1 as Weight)
		);
		assert_eq!(
			Erc20Peg::on_idle(claim_block, delayed_claim_weight / 2),
			DbWeight::get().reads(1 as Weight)
		);

		// Ensure claim isn't removed from storage after either of the above
		assert_eq!(Erc20Peg::ready_blocks(), vec![claim_block]);
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), vec![claim_id]);
		assert_eq!(Erc20Peg::delayed_claims(claim_id), Some(PendingClaim::Deposit(claim.clone())));

		// Try again next block with enough weight
		assert_eq!(Erc20Peg::on_initialize(claim_block + 1), DbWeight::get().reads(1 as Weight));
		assert_eq!(
			Erc20Peg::on_idle(claim_block + 1, delayed_claim_weight * 2),
			delayed_claim_weight + DbWeight::get().reads(1 as Weight)
		);

		// Check claims removed from storage
		assert_eq!(Erc20Peg::ready_blocks(), vec![] as Vec<u64>);
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), vec![] as Vec<ClaimId>);
		assert!(Erc20Peg::delayed_claims(claim_id).is_none());
		// Check beneficiary account has now received funds
		assert_eq!(
			AssetsExt::balance(SPENDING_ASSET_ID, &AccountId::from(beneficiary)),
			deposit_amount
		);
	});
}

#[test]
fn multiple_deposit_claims_with_delay() {
	ExtBuilder::default().build().execute_with(|| {
		let source = H160::from_low_u64_be(123);
		let destination = <Test as Config>::PalletId::get().into_account_truncating();
		let deposit_amount: Balance = 100;
		let beneficiary: H160 = H160::from_low_u64_be(456);

		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));

		// Setup token mapping
		let token_address: H160 = H160::from_low_u64_be(666);
		Erc20ToAssetId::insert(token_address, SPENDING_ASSET_ID);

		// Set claim delay with deposit_amount, this should delay the claim
		let delay: u64 = 1000;
		assert_ok!(Erc20Peg::set_claim_delay(
			frame_system::RawOrigin::Root.into(),
			SPENDING_ASSET_ID,
			deposit_amount,
			delay
		));

		// Encode data for bridge call
		let data = ethabi::encode(&[
			Token::Address(token_address),
			Token::Uint(deposit_amount.into()),
			Token::Address(beneficiary),
		]);
		let claim_block = <frame_system::Pallet<Test>>::block_number() + delay;
		let claim = Erc20DepositEvent { token_address, amount: deposit_amount.into(), beneficiary };

		// Deposit more claims than u8::MAX
		let num_claims: u64 = 300;
		let mut claim_ids: Vec<ClaimId> = vec![];
		for _ in 0..num_claims {
			let claim_id = <NextDelayedClaimId>::get();
			claim_ids.push(claim_id);
			assert_ok!(MockEthereumEventRouter::route(
				&source,
				&destination,
				data.clone().as_slice()
			));

			// Check claim has been put into pending claims
			assert_eq!(
				Erc20Peg::delayed_claims(claim_id),
				Some(PendingClaim::Deposit(claim.clone()))
			);
		}
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), claim_ids.clone());

		// Call on_idle with room for all claims
		let delayed_claim_weight: Weight = DbWeight::get()
			.reads(8 as Weight)
			.saturating_add(DbWeight::get().writes(10 as Weight));
		assert_eq!(
			Erc20Peg::on_initialize(claim_block),
			DbWeight::get().reads(1 as Weight) + DbWeight::get().writes(1 as Weight)
		);
		assert_eq!(
			Erc20Peg::on_idle(
				claim_block,
				num_claims * delayed_claim_weight + DbWeight::get().reads(1 as Weight)
			),
			u8::MAX as u64 * delayed_claim_weight + DbWeight::get().reads(1 as Weight)
		);

		// Check that we have processed u8::MAX claims
		let mut changed_count = 0;
		for i in 0..num_claims {
			if Erc20Peg::delayed_claims(claim_ids[i as usize]) == None {
				changed_count += 1;
			}
		}
		assert_eq!(changed_count, u8::MAX);
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), claim_ids[u8::MAX as usize..]);
		assert_eq!(Erc20Peg::ready_blocks(), vec![claim_block]);

		// Now process the rest of the claims
		assert_eq!(Erc20Peg::on_initialize(claim_block + 1), DbWeight::get().reads(1 as Weight));
		assert_eq!(
			Erc20Peg::on_idle(
				claim_block + 1,
				num_claims * delayed_claim_weight + DbWeight::get().reads(1 as Weight)
			),
			(num_claims - u8::MAX as u64) * delayed_claim_weight +
				DbWeight::get().reads(1 as Weight)
		);

		// All claims should now be processed
		for i in 0..num_claims {
			assert!(Erc20Peg::delayed_claims(claim_ids[i as usize]).is_none());
		}
		assert_eq!(Erc20Peg::ready_blocks(), vec![] as Vec<u64>);
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), vec![] as Vec<u64>);
		// Check beneficiary account is now rich with funds from all claims
		assert_eq!(
			AssetsExt::balance(SPENDING_ASSET_ID, &AccountId::from(beneficiary)),
			deposit_amount * (num_claims as Balance)
		);
	});
}

#[test]
fn many_deposit_claims_with_delay() {
	ExtBuilder::default().build().execute_with(|| {
		let source = H160::from_low_u64_be(123);
		let destination = <Test as Config>::PalletId::get().into_account_truncating();
		let deposit_amount: Balance = 100;
		let beneficiary: H160 = H160::from_low_u64_be(456);

		// Activate deposits
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));

		// Setup token mapping
		let token_address: H160 = H160::from_low_u64_be(666);
		Erc20ToAssetId::insert(token_address, SPENDING_ASSET_ID);

		// Set claim delay with deposit_amount, this should delay the claim
		let delay: u64 = 1000;
		assert_ok!(Erc20Peg::set_claim_delay(
			frame_system::RawOrigin::Root.into(),
			SPENDING_ASSET_ID,
			deposit_amount,
			delay
		));

		// Encode data for bridge call
		let data = ethabi::encode(&[
			Token::Address(token_address),
			Token::Uint(deposit_amount.into()),
			Token::Address(beneficiary),
		]);
		let claim_block = <frame_system::Pallet<Test>>::block_number() + delay;
		let claim = Erc20DepositEvent { token_address, amount: deposit_amount.into(), beneficiary };
		let delayed_claim_weight: Weight = DbWeight::get()
			.reads(8 as Weight)
			.saturating_add(DbWeight::get().writes(10 as Weight));

		let num_claims: u64 = 50;
		let mut claim_ids: Vec<ClaimId> = vec![];
		let mut claim_blocks: Vec<u64> = vec![];

		// Process all claims, this time incrementing the block number between each claim
		for i in 0..num_claims {
			let claim_id = <NextDelayedClaimId>::get();
			claim_ids.push(claim_id);
			assert_ok!(MockEthereumEventRouter::route(
				&source,
				&destination,
				data.clone().as_slice()
			));
			// Check claim has been put into pending claims
			assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block + i), vec![claim_id.clone()]);
			assert_eq!(
				Erc20Peg::delayed_claims(claim_id),
				Some(PendingClaim::Deposit(claim.clone()))
			);
			// Go to next block
			claim_blocks.push(claim_block + i);
			System::set_block_number(System::block_number() + 1);
		}

		// Go through each block and process claim with on_idle
		for i in 0..num_claims {
			assert_eq!(
				Erc20Peg::on_initialize(claim_blocks[i as usize]),
				DbWeight::get().reads(1 as Weight) + DbWeight::get().writes(1 as Weight)
			);
			assert_eq!(
				Erc20Peg::on_idle(
					claim_blocks[i as usize],
					delayed_claim_weight + DbWeight::get().reads(1 as Weight)
				),
				delayed_claim_weight + DbWeight::get().reads(1 as Weight)
			);
			// Check storage is removed at this block
			assert!(Erc20Peg::delayed_claims(claim_ids[i as usize]).is_none());
			assert_eq!(Erc20Peg::ready_blocks(), vec![] as Vec<u64>);
			assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), vec![] as Vec<u64>);
		}

		// Check beneficiary account is now rich with funds from all claims
		assert_eq!(
			AssetsExt::balance(SPENDING_ASSET_ID, &AccountId::from(beneficiary)),
			deposit_amount * (num_claims as Balance)
		);
	});
}

#[test]
fn withdraw() {
	ExtBuilder::default().build().execute_with(|| {
		let account = AccountId::from(H160::from_low_u64_be(123));
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
		let account: AccountId = AccountId::from(H160::from_low_u64_be(123));
		let asset_id: AssetId = 1;
		let cennz_eth_address: EthAddress = H160::default();
		let amount: Balance = 100;
		let beneficiary: H160 = H160::from_slice(&hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));
		let delay: u64 = 1000;
		let _ = <Test as Config>::MultiCurrency::mint_into(asset_id, &account, amount);
		let delayed_claim_weight: Weight = DbWeight::get()
			.reads(8 as Weight)
			.saturating_add(DbWeight::get().writes(10 as Weight));

		<AssetIdToErc20>::insert(asset_id, cennz_eth_address);
		<Erc20ToAssetId>::insert(cennz_eth_address, asset_id);
		assert_ok!(Erc20Peg::activate_withdrawals(frame_system::RawOrigin::Root.into(), true));

		assert_ok!(Erc20Peg::set_claim_delay(
			frame_system::RawOrigin::Root.into(),
			asset_id,
			amount,
			delay
		));

		let claim_id = <NextDelayedClaimId>::get();
		let claim_block = <frame_system::Pallet<Test>>::block_number() + delay;
		assert_ok!(Erc20Peg::withdraw(Some(account.clone()).into(), asset_id, amount, beneficiary));

		// Balance should be withdrawn straight away
		assert_eq!(AssetsExt::balance(asset_id, &account), 0);
		let message = WithdrawMessage {
			token_address: cennz_eth_address,
			amount: amount.into(),
			beneficiary,
		};

		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), vec![claim_id]);
		assert_eq!(Erc20Peg::delayed_claims(claim_id), Some(PendingClaim::Withdrawal(message)));
		// Check claim id has been increased
		assert_eq!(<NextDelayedClaimId>::get(), claim_id + 1);
		assert_eq!(
			Erc20Peg::on_initialize(claim_block),
			DbWeight::get().reads(1 as Weight) + DbWeight::get().writes(1 as Weight)
		);
		assert_eq!(
			Erc20Peg::on_idle(claim_block, delayed_claim_weight * 2),
			delayed_claim_weight + DbWeight::get().reads(1 as Weight)
		);
		// Claim should be removed from storage
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), vec![] as Vec<ClaimId>);
		assert!(Erc20Peg::delayed_claims(claim_id).is_none());
	});
}

#[test]
fn withdraw_less_than_delay_goes_through() {
	ExtBuilder::default().build().execute_with(|| {
		let account: AccountId = AccountId::from(H160::from_low_u64_be(123));
		let asset_id: AssetId = 1;
		let cennz_eth_address: EthAddress = H160::default();
		let amount: Balance = 100;
		let beneficiary: H160 = H160::from_slice(&hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));
		let delay: u64 = 1000;
		let _ = <Test as Config>::MultiCurrency::mint_into(asset_id, &account, amount);

		<AssetIdToErc20>::insert(asset_id, cennz_eth_address);
		<Erc20ToAssetId>::insert(cennz_eth_address, asset_id);
		assert_ok!(Erc20Peg::activate_withdrawals(frame_system::RawOrigin::Root.into(), true));

		assert_ok!(Erc20Peg::set_claim_delay(
			frame_system::RawOrigin::Root.into(),
			asset_id,
			amount,
			delay
		));

		let claim_id = <NextDelayedClaimId>::get();
		let claim_block = <frame_system::Pallet<Test>>::block_number() + delay;
		assert_ok!(Erc20Peg::withdraw(
			Some(account.clone()).into(),
			asset_id,
			amount - 1,
			beneficiary
		));
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), vec![] as Vec<ClaimId>);
		assert!(Erc20Peg::delayed_claims(claim_id).is_none());
	});
}

#[test]
fn withdraw_unsupported_asset_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let account: AccountId = AccountId::from(H160::from_low_u64_be(123));
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
		let account: AccountId = AccountId::from(H160::from_low_u64_be(123));
		let asset_id: AssetId = 1;
		let amount: Balance = 100;
		let beneficiary: H160 = H160::from_slice(&hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));

		assert_noop!(
			Erc20Peg::withdraw(Some(account.clone()).into(), asset_id, amount, beneficiary),
			Error::<Test>::WithdrawalsPaused
		);
	});
}
