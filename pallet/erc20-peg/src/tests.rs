use super::*;
use crate::{
	mock::{
		AccountId, AssetsExt, DepositEventSignature, Erc20Peg, ExtBuilder, PegPalletId, System,
		Test,
	},
	types::{ClaimId, Erc20DepositEvent, PendingClaim, WithdrawMessage},
	EventClaimVerifier,
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

#[test]
fn deposit_claim() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));
		let account: AccountId = 1;
		let token_address: H160 = H160::default();
		let amount: Balance = 100;
		let beneficiary: H256 = H256::default();
		let claim = Erc20DepositEvent { token_address, amount: amount.into(), beneficiary };
		let tx_hash = H256::default();

		assert_ok!(Erc20Peg::deposit_claim(Some(account).into(), tx_hash, claim));
	});
}

#[test]
fn on_deposit_mints() {
	ExtBuilder::default().build().execute_with(|| {
		let contract_address: EthAddress = EthAddress::from(Erc20Peg::contract_address());
		assert_ok!(Erc20Peg::set_contract_address(
			frame_system::RawOrigin::Root.into(),
			contract_address
		));

		let token_address: H160 = H160::default();
		let amount: Balance = 100;
		let beneficiary: H256 = H256::default();
		let claim = Erc20DepositEvent { token_address, amount: amount.into(), beneficiary };

		let expected_asset_id = AssetsExt::next_asset_uuid().unwrap();

		// No assets expected at first
		assert_eq!(Erc20Peg::erc20_to_asset(claim.token_address), None);
		let event_claim_id: u64 = 0;
		let event_type: H256 = DepositEventSignature::get().into();
		// Given the deposit
		Erc20Peg::on_success(
			event_claim_id,
			&contract_address,
			&event_type,
			&crate::EthAbiCodec::encode(&claim),
		);

		assert_eq!(Erc20Peg::erc20_to_asset(claim.token_address), Some(expected_asset_id));

		let beneficiary: AccountId = Decode::decode(&mut &beneficiary.0[..]).unwrap();
		assert_eq!(AssetsExt::balance(expected_asset_id, &beneficiary), amount);
		assert_eq!(Erc20Peg::erc20_to_asset(contract_address), Some(expected_asset_id));
		assert_eq!(Erc20Peg::asset_to_erc20(expected_asset_id), Some(contract_address));
	});
}

#[test]
fn withdraw() {
	ExtBuilder::default().build().execute_with(|| {
		let account: AccountId = 1;
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

		// Check withdrawal hash is stored correctly
		let message = WithdrawMessage {
			token_address: cennz_eth_address,
			amount: amount.into(),
			beneficiary,
		};
		let event_proof_id: u64 =
			<Test as Config>::EthBridge::generate_event_proof(&message).unwrap();
		let withdrawal_hash =
			<Test as frame_system::Config>::Hashing::hash(&mut (message, event_proof_id).encode());
		assert_eq!(Erc20Peg::withdrawal_digests(event_proof_id), withdrawal_hash);
	});
}

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
fn deposit_claim_with_delay() {
	ExtBuilder::default().build().execute_with(|| {
		let account: AccountId = 1;
		let asset_id: AssetId = 1;
		let cennz_eth_address: EthAddress = H160::default();
		let amount: Balance = 100;
		let beneficiary: H256 = H256::default();
		let claim = Erc20DepositEvent {
			token_address: cennz_eth_address,
			amount: amount.into(),
			beneficiary,
		};
		let tx_hash = H256::default();
		let delay: u64 = 1000;
		let delayed_claim_weight: Weight = DbWeight::get()
			.reads(8 as Weight)
			.saturating_add(DbWeight::get().writes(10 as Weight));

		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));
		<AssetIdToErc20>::insert(asset_id, cennz_eth_address);
		<Erc20ToAssetId>::insert(cennz_eth_address, asset_id);
		assert_ok!(Erc20Peg::set_claim_delay(
			frame_system::RawOrigin::Root.into(),
			asset_id,
			amount,
			delay
		));
		let claim_id = <NextDelayedClaimId>::get();
		assert_ok!(Erc20Peg::deposit_claim(Some(account).into(), tx_hash, claim.clone()));
		let claim_block = <frame_system::Pallet<Test>>::block_number() + delay;
		// Check claim has been put into pending claims
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), vec![claim_id]);
		assert_eq!(
			Erc20Peg::delayed_claims(claim_id),
			Some(PendingClaim::Deposit((claim.clone(), tx_hash)))
		);
		// Check claim id has been increased
		assert_eq!(<NextDelayedClaimId>::get(), claim_id + 1);
		// Simulating block before with enough weight, claim shouldn't be removed
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
		assert_eq!(Erc20Peg::ready_blocks(), vec![claim_block]);
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), vec![claim_id]);
		assert_eq!(
			Erc20Peg::delayed_claims(claim_id),
			Some(PendingClaim::Deposit((claim.clone(), tx_hash)))
		);

		// Try again next block with enough weight
		assert_eq!(Erc20Peg::on_initialize(claim_block + 1), DbWeight::get().reads(1 as Weight));
		assert_eq!(
			Erc20Peg::on_idle(claim_block + 1, delayed_claim_weight * 2),
			delayed_claim_weight + DbWeight::get().reads(1 as Weight)
		);
		// Claim should be removed from storage
		let empty_blocks: Vec<u64> = vec![];
		assert_eq!(Erc20Peg::ready_blocks(), empty_blocks);
		let empty_claims: Vec<ClaimId> = vec![];
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), empty_claims);
		assert_eq!(Erc20Peg::delayed_claims(claim_id), None);
		let empty_blocks: Vec<u64> = vec![];
		assert_eq!(Erc20Peg::ready_blocks(), empty_blocks);
	});
}

#[test]
fn multiple_deposit_claims_with_delay() {
	ExtBuilder::default().build().execute_with(|| {
		let account: AccountId = 1;
		let asset_id: AssetId = 1;
		let cennz_eth_address: EthAddress = H160::default();
		let amount: Balance = 100;
		let beneficiary: H256 = H256::default();
		let claim = Erc20DepositEvent {
			token_address: cennz_eth_address,
			amount: amount.into(),
			beneficiary,
		};
		let tx_hash = H256::default();
		let delay: u64 = 1000;
		let delayed_claim_weight: Weight = DbWeight::get()
			.reads(8 as Weight)
			.saturating_add(DbWeight::get().writes(10 as Weight));

		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));
		<AssetIdToErc20>::insert(asset_id, cennz_eth_address);
		<Erc20ToAssetId>::insert(cennz_eth_address, asset_id);
		assert_ok!(Erc20Peg::set_claim_delay(
			frame_system::RawOrigin::Root.into(),
			asset_id,
			amount,
			delay
		));
		let mut claim_ids: Vec<ClaimId> = vec![];
		// Try deposit more claims than u8::MAX
		let num_claims: u64 = 300;
		let claim_block = <frame_system::Pallet<Test>>::block_number() + delay;

		for _ in 0..num_claims {
			let claim_id = <NextDelayedClaimId>::get();
			claim_ids.push(claim_id);
			assert_ok!(Erc20Peg::deposit_claim(
				Some(account.clone()).into(),
				tx_hash,
				claim.clone()
			));
			// Check claim has been put into pending claims
			assert_eq!(
				Erc20Peg::delayed_claims(claim_id),
				Some(PendingClaim::Deposit((claim.clone(), tx_hash)))
			);
		}
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), claim_ids.clone());

		// Call on_idle with room for only 5 claims
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
		let mut changed_count = 0;
		for i in 0..num_claims {
			if Erc20Peg::delayed_claims(claim_ids[i as usize]) == None {
				changed_count += 1;
			}
		}
		assert_eq!(changed_count, u8::MAX);
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), claim_ids[u8::MAX as usize..]);
		assert_eq!(Erc20Peg::ready_blocks(), vec![claim_block]);

		assert_eq!(Erc20Peg::on_initialize(claim_block + 1), DbWeight::get().reads(1 as Weight));
		assert_eq!(
			Erc20Peg::on_idle(
				claim_block + 1,
				num_claims * delayed_claim_weight + DbWeight::get().reads(1 as Weight)
			),
			(num_claims - u8::MAX as u64) * delayed_claim_weight +
				DbWeight::get().reads(1 as Weight)
		);
		for i in 0..num_claims {
			assert_eq!(Erc20Peg::delayed_claims(claim_ids[i as usize]), None);
		}
		let empty_blocks: Vec<u64> = vec![];
		assert_eq!(Erc20Peg::ready_blocks(), empty_blocks);
		let empty_claims: Vec<u64> = vec![];
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), empty_claims);
	});
}

#[test]
fn many_deposit_claims_with_delay() {
	ExtBuilder::default().build().execute_with(|| {
		let account: AccountId = 1;
		let asset_id: AssetId = 1;
		let cennz_eth_address: EthAddress = H160::default();
		let amount: Balance = 100;
		let beneficiary: H256 = H256::default();
		let claim = Erc20DepositEvent {
			token_address: cennz_eth_address,
			amount: amount.into(),
			beneficiary,
		};
		let tx_hash = H256::default();
		let delay: u64 = 1000;
		let delayed_claim_weight: Weight = DbWeight::get()
			.reads(8 as Weight)
			.saturating_add(DbWeight::get().writes(10 as Weight));

		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));
		<AssetIdToErc20>::insert(asset_id, cennz_eth_address);
		<Erc20ToAssetId>::insert(cennz_eth_address, asset_id);
		assert_ok!(Erc20Peg::set_claim_delay(
			frame_system::RawOrigin::Root.into(),
			asset_id,
			amount,
			delay
		));
		let mut claim_ids: Vec<ClaimId> = vec![];
		// Try deposit more claims than u8::MAX
		let num_claims: u64 = 50;
		let claim_block = System::block_number() + delay;
		let mut claim_blocks: Vec<u64> = vec![];

		for i in 0..num_claims {
			let claim_id = <NextDelayedClaimId>::get();
			claim_ids.push(claim_id);
			assert_ok!(Erc20Peg::deposit_claim(
				Some(account.clone()).into(),
				tx_hash,
				claim.clone()
			));
			assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block + i), vec![claim_id.clone()]);

			// Go to next block
			claim_blocks.push(claim_block + i);
			System::set_block_number(System::block_number() + 1);
		}

		for i in 0..num_claims {
			// Go through each block and process claim
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
			assert_eq!(Erc20Peg::delayed_claims(claim_ids[i as usize]), None);
			let empty_blocks: Vec<u64> = vec![];
			assert_eq!(Erc20Peg::ready_blocks(), empty_blocks);
			let empty_claims: Vec<u64> = vec![];
			assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), empty_claims);
		}
	});
}

#[test]
fn deposit_claim_less_than_delay_goes_through() {
	ExtBuilder::default().build().execute_with(|| {
		let account: AccountId = 1;
		let asset_id: AssetId = 1;
		let cennz_eth_address: EthAddress = H160::default();
		let amount: Balance = 100;
		let beneficiary: H256 = H256::default();
		let claim = Erc20DepositEvent {
			token_address: cennz_eth_address,
			amount: (amount - 1).into(),
			beneficiary,
		};
		let tx_hash = H256::default();
		let delay: u64 = 1000;

		assert_ok!(Erc20Peg::activate_deposits(frame_system::RawOrigin::Root.into(), true));
		<AssetIdToErc20>::insert(asset_id, cennz_eth_address);
		<Erc20ToAssetId>::insert(cennz_eth_address, asset_id);
		assert_ok!(Erc20Peg::set_claim_delay(
			frame_system::RawOrigin::Root.into(),
			asset_id,
			amount,
			delay
		));
		let claim_id = <NextDelayedClaimId>::get();
		assert_ok!(Erc20Peg::deposit_claim(Some(account).into(), tx_hash, claim.clone()));
		let claim_block = <frame_system::Pallet<Test>>::block_number() + delay;
		// Check claim has not been put into pending claims
		let empty_claims: Vec<ClaimId> = vec![];
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), empty_claims);
		assert_eq!(Erc20Peg::delayed_claims(claim_id), None);
	});
}

#[test]
fn withdraw_with_delay() {
	ExtBuilder::default().build().execute_with(|| {
		let account: AccountId = 1;
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
		let empty_claims: Vec<ClaimId> = vec![];
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), empty_claims);
		assert_eq!(Erc20Peg::delayed_claims(claim_id), None);
	});
}

#[test]
fn withdraw_less_than_delay_goes_through() {
	ExtBuilder::default().build().execute_with(|| {
		let account: AccountId = 1;
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
		let empty_claims: Vec<ClaimId> = vec![];
		assert_eq!(Erc20Peg::delayed_claim_schedule(claim_block), empty_claims);
		assert_eq!(Erc20Peg::delayed_claims(claim_id), None);
	});
}

#[test]
fn withdraw_unsupported_asset_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let account: AccountId = 1;
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
		let account: AccountId = 1;
		let asset_id: AssetId = 1;
		let amount: Balance = 100;
		let beneficiary: H160 = H160::from_slice(&hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d"));

		assert_noop!(
			Erc20Peg::withdraw(Some(account.clone()).into(), asset_id, amount, beneficiary),
			Error::<Test>::WithdrawalsPaused
		);
	});
}
