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

use crate::{mock::*, *};
use frame_support::traits::Hooks;
use hex_literal::hex;
use pallet_nft::CollectionInfo;
use seed_pallet_common::test_prelude::*;

struct TestVals {
	source: H160,
	designated_function: usize,
	token_address: H160,
	destination: H160,
	inner_token_id: U256,
	data: Vec<u8>,
}

impl Default for TestVals {
	fn default() -> Self {
		let token_address_source = hex!["d9145cce52d386f254917e481eb44e9943f39138"];
		let destination_source = hex!["5b38da6a701c568545dcfcb03fcb875f56beddc4"];
		let token_address = H160::from(token_address_source);
		let destination = H160::from(destination_source);
		// A positional bit sent by the contract that the pallet looks at to determine which
		// function to call
		let designated_function = 1;
		// Represents a test token in a collection
		let inner_token_id = U256::from(1);

		TestVals {
			source: H160::zero(),
			designated_function,
			token_address,
			destination,
			inner_token_id: U256::from(1),
			data: ethabi::encode(&[
				Token::Uint(U256::from(designated_function)),
				Token::Array(vec![Token::Address(token_address)]),
				Token::Array(vec![Token::Array(vec![Token::Uint(inner_token_id)])]),
				Token::Address(destination),
			]),
		}
	}
}

fn deposit_max_tokens(owner: AccountId) {
	let test_vals = TestVals::default();

	let token_addresses =
		BoundedVec::<H160, MaxAddresses>::truncate_from(vec![test_vals.token_address]);

	for i in 0..200 {
		let mut token_ids = vec![];
		for n in 1..51 {
			token_ids.push((i * 50) + n);
		}

		let token_ids =
			BoundedVec::<BoundedVec<SerialNumber, MaxIdsPerMultipleMint>, MaxAddresses>::truncate_from(vec![
				BoundedVec::<SerialNumber, MaxIdsPerMultipleMint>::truncate_from(token_ids),
			]);

		let token_information = GroupedTokenInfo::new(token_ids, token_addresses.clone(), owner);

		assert_ok!(Pallet::<Test>::do_deposit(token_information, owner.into()));
	}
}

fn mock_token_information(
	destination: AccountId,
	serial_numbers: Vec<SerialNumber>,
) -> GroupedTokenInfo<Test> {
	let test_vals = TestVals::default();

	let token_addresses =
		BoundedVec::<H160, MaxAddresses>::truncate_from(vec![test_vals.token_address]);

	let token_ids =
		BoundedVec::<BoundedVec<SerialNumber, MaxIdsPerMultipleMint>, MaxAddresses>::truncate_from(
			vec![BoundedVec::<SerialNumber, MaxIdsPerMultipleMint>::truncate_from(serial_numbers)],
		);

	GroupedTokenInfo::<Test>::new(token_ids, token_addresses, destination)
}

#[test]
fn event_handler_decodes_correctly() {
	TestExt::<Test>::default().build().execute_with(|| {
		let test_vals = TestVals::default();
		assert_ok!(Pallet::<Test>::on_event(&test_vals.source, &test_vals.data));
	});
}

#[test]
fn decode_deposit_event_errs_too_many_tokens() {
	TestExt::<Test>::default().build().execute_with(|| {
		let test_vals = TestVals::default();

		// Too many tokens
		let excessive_inner = vec![Token::Uint(test_vals.inner_token_id); 1000];

		// NFT bridge data encoded
		let data = ethabi::encode(&[
			Token::Uint(U256::from(test_vals.designated_function)),
			Token::Array(vec![Token::Address(test_vals.token_address)]),
			Token::Array(vec![Token::Array(excessive_inner)]),
			Token::Address(test_vals.destination),
		]);

		assert_noop!(
			Pallet::<Test>::decode_deposit_event(&data),
			(Weight::zero(), Error::<Test>::ExceedsMaxTokens.into())
		);
	})
}

#[test]
fn decode_deposit_event_errs_too_many_addresses() {
	TestExt::<Test>::default().build().execute_with(|| {
		let test_vals = TestVals::default();

		let inner_token = vec![Token::Uint(test_vals.inner_token_id)];
		// Too many addresses
		let excessive_addresses = vec![Token::Array(inner_token); 1000];

		let data = ethabi::encode(&[
			Token::Uint(U256::from(test_vals.designated_function)),
			Token::Array(vec![Token::Address(test_vals.token_address)]),
			Token::Array(excessive_addresses),
			Token::Address(test_vals.destination),
		]);

		assert_noop!(
			Pallet::<Test>::decode_deposit_event(&data),
			(Weight::zero(), Error::<Test>::ExceedsMaxAddresses.into())
		);
	})
}

#[test]
fn do_deposit_creates_tokens_and_collection() {
	TestExt::<Test>::default().build().execute_with(|| {
		let test_vals = TestVals::default();
		let expected_collection_id = Nft::next_collection_uuid().unwrap();

		let serial_numbers = vec![1_u32];
		let token_information =
			mock_token_information(test_vals.destination.into(), serial_numbers.clone());

		assert_ok!(Pallet::<Test>::do_deposit(token_information, test_vals.destination));
		System::assert_has_event(
			Event::<Test>::Erc721Mint {
				collection_id: expected_collection_id,
				owner: test_vals.destination.into(),
				serial_numbers: BoundedVec::truncate_from(serial_numbers),
			}
			.into(),
		);

		assert_eq!(
			EthToRootNft::<Test>::get(test_vals.token_address),
			Some(expected_collection_id)
		);
		assert_eq!(
			RootNftToErc721::<Test>::get(expected_collection_id),
			Some(test_vals.token_address)
		);
		assert!(Nft::collection_exists(expected_collection_id));

		let collection_info = CollectionInfo::<Test>::get(expected_collection_id).unwrap();
		let mut h160_addr = sp_std::Writer::default();
		write!(&mut h160_addr, "ethereum://{:?}/", test_vals.token_address).expect("Not written");
		assert_eq!(
			collection_info.metadata_scheme,
			MetadataScheme::try_from(h160_addr.inner().clone().as_slice()).unwrap()
		);

		// Token balance should be 1 as one token was deposited
		assert_eq!(
			Nft::token_balance_of(&AccountId::from(test_vals.destination), expected_collection_id),
			1
		);
	})
}

#[test]
fn do_deposit_works_with_existing_bridged_collection() {
	TestExt::<Test>::default().build().execute_with(|| {
		let test_vals = TestVals::default();
		let expected_collection_id = Nft::next_collection_uuid().unwrap();

		let serial_numbers = vec![1_u32];
		let token_information =
			mock_token_information(test_vals.destination.into(), serial_numbers.clone());

		assert_ok!(Pallet::<Test>::do_deposit(token_information, test_vals.destination));
		System::assert_has_event(
			Event::<Test>::Erc721Mint {
				collection_id: expected_collection_id,
				owner: test_vals.destination.into(),
				serial_numbers: BoundedVec::truncate_from(serial_numbers),
			}
			.into(),
		);

		assert_eq!(
			EthToRootNft::<Test>::get(test_vals.token_address),
			Some(expected_collection_id)
		);
		assert_eq!(
			RootNftToErc721::<Test>::get(expected_collection_id),
			Some(test_vals.token_address)
		);
		Nft::collection_exists(expected_collection_id);
		// Token balance should be 1 as one token was deposited
		assert_eq!(
			Nft::token_balance_of(&AccountId::from(test_vals.destination), expected_collection_id),
			1
		);

		let new_serial_numbers = vec![2_u32];
		let token_information =
			mock_token_information(test_vals.destination.into(), new_serial_numbers.clone());

		// When bridged tokens are sent for existing collection
		assert_ok!(Pallet::<Test>::do_deposit(token_information, test_vals.destination));
		System::assert_has_event(
			Event::<Test>::Erc721Mint {
				collection_id: expected_collection_id,
				owner: test_vals.destination.into(),
				serial_numbers: BoundedVec::truncate_from(new_serial_numbers),
			}
			.into(),
		);

		assert_eq!(
			EthToRootNft::<Test>::get(test_vals.token_address),
			Some(expected_collection_id)
		);
		assert_eq!(
			RootNftToErc721::<Test>::get(expected_collection_id),
			Some(test_vals.token_address)
		);
		// Then balance should now be 2 as another token was deposited
		assert_eq!(
			Nft::token_balance_of(&AccountId::from(test_vals.destination), expected_collection_id),
			2
		);
	})
}

#[test]
fn handles_duplicated_tokens_sent() {
	TestExt::<Test>::default().build().execute_with(|| {
		let test_vals = TestVals::default();
		let expected_collection_id = Nft::next_collection_uuid().unwrap();

		let token_set = vec![0, 1, 2, 3, 4];
		let token_set_duplicates = vec![4, 5, 6, 7]; // One duplicate token

		let token_information =
			mock_token_information(test_vals.destination.into(), token_set.clone());

		assert_ok!(Pallet::<Test>::do_deposit(token_information, test_vals.destination));
		System::assert_has_event(
			Event::<Test>::Erc721Mint {
				collection_id: expected_collection_id,
				owner: test_vals.destination.into(),
				serial_numbers: BoundedVec::truncate_from(token_set),
			}
			.into(),
		);

		assert_eq!(
			EthToRootNft::<Test>::get(test_vals.token_address),
			Some(expected_collection_id)
		);
		assert_eq!(
			RootNftToErc721::<Test>::get(expected_collection_id),
			Some(test_vals.token_address)
		);
		Nft::collection_exists(expected_collection_id);

		assert_eq!(
			Nft::token_balance_of(&AccountId::from(test_vals.destination), expected_collection_id),
			5
		);

		let token_information =
			mock_token_information(test_vals.destination.into(), token_set_duplicates.clone());

		// When bridged tokens are sent for existing collection
		assert_ok!(Pallet::<Test>::do_deposit(token_information, test_vals.destination));
		System::assert_has_event(
			Event::<Test>::Erc721Mint {
				collection_id: expected_collection_id,
				owner: test_vals.destination.into(),
				serial_numbers: BoundedVec::truncate_from(token_set_duplicates),
			}
			.into(),
		);

		assert_eq!(
			EthToRootNft::<Test>::get(test_vals.token_address),
			Some(expected_collection_id)
		);
		assert_eq!(
			RootNftToErc721::<Test>::get(expected_collection_id),
			Some(test_vals.token_address)
		);

		// Expected amount == 8, as duplicated token is never counted
		assert_eq!(
			Nft::token_balance_of(&AccountId::from(test_vals.destination), expected_collection_id),
			8
		);
	})
}

#[test]
fn do_withdraw_works() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_id = Nft::next_collection_uuid().unwrap();
		let test_vals = TestVals::default();

		assert_ok!(Pallet::<Test>::on_event(&test_vals.source, &test_vals.data));
		// Wait for mint to occur
		NftPeg::on_initialize(6);

		let collection_ids = BoundedVec::truncate_from(vec![collection_id]);
		let serial_numbers = BoundedVec::truncate_from(vec![BoundedVec::truncate_from(vec![1])]);
		assert_ok!(Pallet::<Test>::withdraw(
			RuntimeOrigin::signed(AccountId::from(test_vals.destination)),
			collection_ids,
			serial_numbers,
			H160::from_low_u64_be(123),
		));

		// Token should be burnt
		assert_eq!(Nft::token_balance_of(&AccountId::from(test_vals.source), collection_id), 0);
	});
}

#[test]
fn do_withdraw_invalid_token_length_should_fail() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_ids = BoundedVec::truncate_from(vec![1, 2, 3]);
		let serial_numbers = BoundedVec::truncate_from(vec![BoundedVec::truncate_from(vec![1])]);
		assert_noop!(
			Pallet::<Test>::withdraw(
				RuntimeOrigin::signed(AccountId::from(H160::default())),
				collection_ids,
				serial_numbers,
				H160::default()
			),
			Error::<Test>::TokenListLengthMismatch
		);
	});
}

#[test]
fn do_deposit_adds_to_blocked_on_fail() {
	TestExt::<Test>::default().build().execute_with(|| {
		let test_vals = TestVals::default();
		let blocked_mint_id = NextBlockedMintId::<Test>::get();
		let collection_id = Nft::next_collection_uuid().unwrap();

		let collection_owner = create_account(1);

		deposit_max_tokens(collection_owner);

		// Attempt to deposit tokens that exceed limit
		let serial_numbers = vec![10_001_u32, 10_002_u32];
		let token_information = mock_token_information(collection_owner, serial_numbers.clone());

		let (_, err) =
			Pallet::<Test>::do_deposit(token_information, test_vals.destination).unwrap_err();

		assert_eq!(err, pallet_nft::Error::<Test>::BlockedMint.into());

		System::assert_last_event(
			Event::<Test>::ERC721Blocked {
				blocked_mint_id,
				collection_id,
				serial_numbers: BoundedVec::truncate_from(serial_numbers.clone()),
				destination_address: test_vals.destination.into(),
			}
			.into(),
		);

		let blocked = BlockedTokens::<Test>::get(blocked_mint_id).unwrap();

		assert_eq!(blocked.collection_id, collection_id);
		assert_eq!(blocked.serial_numbers, serial_numbers);
		assert_eq!(blocked.destination_address, test_vals.destination.into());
	})
}

#[test]
fn reclaim_blocked_nfts() {
	TestExt::<Test>::default().build().execute_with(|| {
		let blocked_mint_id = NextBlockedMintId::<Test>::get();

		let collection_owner = create_account(1);

		deposit_max_tokens(collection_owner);

		let token_information =
			mock_token_information(collection_owner, vec![10_001_u32, 10_002_u32]);

		let (_, err) =
			Pallet::<Test>::do_deposit(token_information, collection_owner.into()).unwrap_err();

		assert_eq!(err, pallet_nft::Error::<Test>::BlockedMint.into());

		assert_ok!(Pallet::<Test>::reclaim_blocked_nfts(
			Some(collection_owner).into(),
			blocked_mint_id,
			collection_owner.into()
		));
	})
}

#[test]
fn reclaim_blocked_nfts_called_by_wrong_account_should_fail() {
	TestExt::<Test>::default().build().execute_with(|| {
		let blocked_mint_id = NextBlockedMintId::<Test>::get();

		let collection_owner = create_account(1);
		let not_destination = create_account(2);

		deposit_max_tokens(collection_owner);

		let token_information =
			mock_token_information(collection_owner, vec![10_001_u32, 10_002_u32]);

		let (_, err) =
			Pallet::<Test>::do_deposit(token_information, collection_owner.into()).unwrap_err();

		assert_eq!(err, pallet_nft::Error::<Test>::BlockedMint.into());

		assert_noop!(
			Pallet::<Test>::reclaim_blocked_nfts(
				Some(not_destination).into(),
				blocked_mint_id,
				not_destination.into()
			),
			Error::<Test>::NotBlockedTokenDestination
		);
	})
}
