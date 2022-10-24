use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok, traits::Hooks};
use frame_system::RawOrigin;
use hex_literal::hex;
use seed_primitives::AccountId;

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

#[test]
fn event_handler_decodes_correctly() {
	ExtBuilder::default().build().execute_with(|| {
		let test_vals = TestVals::default();
		assert_ok!(Pallet::<Test>::on_event(&test_vals.source, &test_vals.data));
	});
}

#[test]
fn decode_deposit_event_errs_too_many_tokens() {
	ExtBuilder::default().build().execute_with(|| {
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
			(0_u64, Error::<Test>::ExceedsMaxTokens.into())
		);
	})
}

#[test]
fn decode_deposit_event_errs_too_many_addresses() {
	ExtBuilder::default().build().execute_with(|| {
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
			(0_u64, Error::<Test>::ExceedsMaxAddresses.into())
		);
	})
}

#[test]
fn do_deposit_creates_tokens_and_collection() {
	ExtBuilder::default().build().execute_with(|| {
		let expected_collection_id = Nft::next_collection_uuid().unwrap();
		let test_vals = TestVals::default();
		let token_ids =
			BoundedVec::<BoundedVec<SerialNumber, MaxIdsPerMultipleMint>, MaxAddresses>::try_from(
				vec![BoundedVec::<SerialNumber, MaxIdsPerMultipleMint>::try_from(vec![1_u32])
					.unwrap()],
			)
			.unwrap();

		let token_addresses =
			BoundedVec::<H160, MaxAddresses>::try_from(vec![test_vals.token_address]).unwrap();

		let token_information =
			GroupedTokenInfo::new(token_ids, token_addresses, test_vals.destination.into());

		assert_ok!(Pallet::<Test>::do_deposit(token_information, test_vals.destination));

		assert_eq!(
			Pallet::<Test>::eth_to_root_nft(test_vals.token_address),
			Some(expected_collection_id)
		);
		assert_eq!(
			Pallet::<Test>::root_to_eth_nft(expected_collection_id),
			Some(test_vals.token_address)
		);
		assert_eq!(Nft::collection_exists(expected_collection_id), true);
		// Token balance should be 1 as one token was deposited
		assert_eq!(
			Nft::token_balance(AccountId::from(test_vals.destination))
				.unwrap()
				.get(&expected_collection_id),
			Some(&(1))
		);
	})
}

#[test]
fn do_deposit_works_with_existing_bridged_collection() {
	ExtBuilder::default().build().execute_with(|| {
		let test_vals = TestVals::default();
		let expected_collection_id = Nft::next_collection_uuid().unwrap();

		let token_ids =
			BoundedVec::<BoundedVec<SerialNumber, MaxIdsPerMultipleMint>, MaxAddresses>::try_from(
				vec![BoundedVec::<SerialNumber, MaxIdsPerMultipleMint>::try_from(vec![1_u32])
					.unwrap()],
			)
			.unwrap();

		let token_addresses =
			BoundedVec::<H160, MaxAddresses>::try_from(vec![test_vals.token_address]).unwrap();

		let token_information =
			GroupedTokenInfo::new(token_ids, token_addresses.clone(), test_vals.destination.into());

		assert_ok!(Pallet::<Test>::do_deposit(token_information, test_vals.destination));
		assert_eq!(
			Pallet::<Test>::eth_to_root_nft(test_vals.token_address),
			Some(expected_collection_id)
		);
		assert_eq!(
			Pallet::<Test>::root_to_eth_nft(expected_collection_id),
			Some(test_vals.token_address)
		);
		Nft::collection_exists(expected_collection_id);
		// Token balance should be 1 as one token was deposited
		assert_eq!(
			Nft::token_balance(AccountId::from(test_vals.destination))
				.unwrap()
				.get(&expected_collection_id),
			Some(&1)
		);

		let new_token_ids =
			BoundedVec::<BoundedVec<SerialNumber, MaxIdsPerMultipleMint>, MaxAddresses>::try_from(
				vec![BoundedVec::<SerialNumber, MaxIdsPerMultipleMint>::try_from(vec![2_u32])
					.unwrap()],
			)
			.unwrap();

		let token_information =
			GroupedTokenInfo::new(new_token_ids, token_addresses, test_vals.destination.into());

		// When bridged tokens are sent for existing collection
		assert_ok!(Pallet::<Test>::do_deposit(token_information, test_vals.destination));
		assert_eq!(
			Pallet::<Test>::eth_to_root_nft(test_vals.token_address),
			Some(expected_collection_id)
		);
		assert_eq!(
			Pallet::<Test>::root_to_eth_nft(expected_collection_id),
			Some(test_vals.token_address)
		);
		// Then balance should now be 2 as another token was deposited
		assert_eq!(
			Nft::token_balance(AccountId::from(test_vals.destination))
				.unwrap()
				.get(&expected_collection_id),
			Some(&2)
		);
	})
}

#[test]
fn handles_duplicated_tokens_sent() {
	ExtBuilder::default().build().execute_with(|| {
		let test_vals = TestVals::default();
		let expected_collection_id = Nft::next_collection_uuid().unwrap();

		let token_set = vec![0, 1, 2, 3, 4];
		let token_set_duplicates = vec![4, 5, 6, 7]; // One duplicate token

		let token_ids =
			BoundedVec::<BoundedVec<SerialNumber, MaxIdsPerMultipleMint>, MaxAddresses>::try_from(
				vec![
					BoundedVec::<SerialNumber, MaxIdsPerMultipleMint>::try_from(token_set).unwrap()
				],
			)
			.unwrap();

		let token_addresses =
			BoundedVec::<H160, MaxAddresses>::try_from(vec![test_vals.token_address]).unwrap();

		let token_information =
			GroupedTokenInfo::new(token_ids, token_addresses.clone(), test_vals.destination.into());

		assert_ok!(Pallet::<Test>::do_deposit(token_information, test_vals.destination));
		assert_eq!(
			Pallet::<Test>::eth_to_root_nft(test_vals.token_address),
			Some(expected_collection_id)
		);
		assert_eq!(
			Pallet::<Test>::root_to_eth_nft(expected_collection_id),
			Some(test_vals.token_address)
		);
		Nft::collection_exists(expected_collection_id);

		assert_eq!(
			Nft::token_balance(AccountId::from(test_vals.destination))
				.unwrap()
				.get(&expected_collection_id),
			Some(&5)
		);

		let new_token_ids = BoundedVec::<
			BoundedVec<SerialNumber, MaxIdsPerMultipleMint>,
			MaxAddresses,
		>::try_from(vec![
			BoundedVec::<SerialNumber, MaxIdsPerMultipleMint>::try_from(token_set_duplicates)
				.unwrap(),
		])
		.unwrap();

		let token_information =
			GroupedTokenInfo::new(new_token_ids, token_addresses, test_vals.destination.into());

		// When bridged tokens are sent for existing collection
		assert_ok!(Pallet::<Test>::do_deposit(token_information, test_vals.destination));
		assert_eq!(
			Pallet::<Test>::eth_to_root_nft(test_vals.token_address),
			Some(expected_collection_id)
		);
		assert_eq!(
			Pallet::<Test>::root_to_eth_nft(expected_collection_id),
			Some(test_vals.token_address)
		);

		// Expected amount == 8, as duplicated token is never counted
		assert_eq!(
			Nft::token_balance(AccountId::from(test_vals.destination))
				.unwrap()
				.get(&expected_collection_id),
			Some(&8)
		);
	})
}

#[test]
fn do_withdraw_works() {
	ExtBuilder::default().build().execute_with(|| {
		let collection_id = Nft::next_collection_uuid().unwrap();
		let test_vals = TestVals::default();

		assert_ok!(Pallet::<Test>::on_event(&test_vals.source, &test_vals.data));
		// Wait for mint to occur
		NftPeg::on_initialize(6);

		let collection_ids = vec![collection_id];

		assert_ok!(Pallet::<Test>::withdraw(
			Origin::signed(AccountId::from(test_vals.destination)),
			collection_ids,
			vec![vec![1]],
			H160::from_low_u64_be(123),
		));

		// Token should be burnt
		assert!(Nft::token_balance(AccountId::from(test_vals.source)).is_none());
		assert!(Nft::token_owner(collection_id, 1).is_none());
	});
}

#[test]
fn do_withdraw_invalid_token_length_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			Pallet::<Test>::do_withdraw(
				&AccountId::from(H160::default()),
				&vec![1, 2, 3],
				&vec![vec![1]],
				H160::default()
			),
			Error::<Test>::TokenListLengthMismatch
		);
	});
}

#[test]
fn sets_contract_address() {
	ExtBuilder::default().build().execute_with(|| {
		let address = H160::from_low_u64_be(123);
		assert_ok!(Pallet::<Test>::set_contract_address(tests::RawOrigin::Root.into(), address,));

		assert_eq!(NftPeg::contract_address(), address);
	});
}


#[test]
fn errs_when_uint_too_large() {
	ExtBuilder::default().build().execute_with(|| {
		let test_vals = TestVals::default();

		let tokens = vec![
			// Some large Uint > u32
			Token::Uint(U256([1, 1, 1, 1])),
			// Some normal sized Uint
			Token::Uint(test_vals.inner_token_id)
		];

		let expected_collection_id = Nft::next_collection_uuid().unwrap();

		// NFT bridge data encoded
		let data = ethabi::encode(&[
			Token::Uint(U256::from(test_vals.designated_function)),
			Token::Array(vec![Token::Address(test_vals.token_address)]),
			Token::Array(tokens),
			Token::Address(test_vals.destination),
		]);

		assert_noop!(
			Pallet::<Test>::decode_deposit_event(&data),
			(0_u64, Error::<Test>::InvalidAbiEncoding.into())
		);

		// No values should exist, as decode_deposit_event failed
		assert_eq!(
			Pallet::<Test>::eth_to_root_nft(test_vals.token_address),
			None
		);
		assert_eq!(
			Pallet::<Test>::root_to_eth_nft(expected_collection_id),
			None
		);
		assert_eq!(Nft::collection_exists(expected_collection_id), false);
		assert_eq!(
			Nft::token_balance(AccountId::from(test_vals.destination)),
			None
		);

	});
}
