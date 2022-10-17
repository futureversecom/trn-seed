use crate::{
	mock::{Event as MockEvent, *},
	*,
};
use frame_support::{assert_noop, assert_ok, traits::Hooks};
use hex_literal::hex;
use pallet_nft::{CollectionInformation, MetadataScheme};
use seed_primitives::AccountId20;

struct TestVals {
	source: H160,
	designated_function: usize,
	token_address: H160,
	destination: H160,
	inner_token_id: U256,
	data: Vec<u8>
}

impl Default for TestVals {
	fn default() -> Self {
		let token_address_source = hex!["d9145cce52d386f254917e481eb44e9943f39138"];
		let destination_source = hex!["5b38da6a701c568545dcfcb03fcb875f56beddc4"];
		let token_address = H160::from(token_address_source);
		let destination = H160::from(destination_source);
		// A positional bit sent by the contract that the pallet looks at to determine which function to clal
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
			])
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
fn deposit_bridge_events_schedule_a_mint() {
	ExtBuilder::default().build().execute_with(|| {
		let mint_delay_length = 6;
		let test_vals = TestVals::default();

		// Event is sent
		assert_ok!(Pallet::<Test>::decode_deposit_event(&test_vals.source, &test_vals.data));
		// Mint of bridged tokens are scheduled for some configured point in the future
		assert_eq!(DelayedMints::<Test>::contains_key(mint_delay_length), true);
	})
}

#[test]
fn decode_deposit_event_errs_too_many_tokens() {
	ExtBuilder::default().build().execute_with(|| {
		let mint_delay_length = 6;
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
			Pallet::<Test>::decode_deposit_event(&test_vals.source, &data),
			(0_u64, Error::<Test>::ExceedsMaxTokens.into())
		);
		assert_eq!(DelayedMints::<Test>::contains_key(mint_delay_length), false);
	})
}

#[test]
fn decode_deposit_event_errs_too_many_addresses() {
	ExtBuilder::default().build().execute_with(|| {
		let test_vals = TestVals::default();

		let mint_delay_length = 6;

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
			Pallet::<Test>::decode_deposit_event(&test_vals.source, &data),
			(0_u64, Error::<Test>::ExceedsMaxAddresses.into())
		);
		assert_eq!(DelayedMints::<Test>::contains_key(mint_delay_length), false);
	})
}

#[test]
fn scheduled_mint_events_create_nfts() {
	ExtBuilder::default().build().execute_with(|| {
		let empty_name = "".encode();
		let test_vals = TestVals::default();
		let peg_info = PeggedNftInfo::<Test> {
			source: test_vals.source,
			token_addresses: BoundedVec::<H160, MaxAddresses>::try_from(vec![test_vals.token_address])
				.unwrap(),
			token_ids:
				BoundedVec::<BoundedVec<U256, MaxIdsPerMultipleMint>, MaxAddresses>::try_from(vec![
					BoundedVec::<U256, MaxIdsPerMultipleMint>::try_from(vec![U256::from(
						test_vals.designated_function,
					)])
					.unwrap(),
				])
				.unwrap(),
			destination: test_vals.destination,
		};

		DelayedMints::<Test>::insert(0, peg_info);

		let collection_id = Nft::next_collection_uuid().unwrap();

		// Simulate a wait period for the mint operation
		NftPeg::on_initialize(0);

		assert_eq!(Nft::next_collection_uuid().unwrap(), 1124);
		assert_eq!(
			Nft::collection_info(collection_id).unwrap(),
			CollectionInformation {
				owner: <Test as pallet_nft::Config>::PalletId::get().into_account_truncating(),
				name: empty_name,
				metadata_scheme: MetadataScheme::Ethereum(H160::zero()),
				royalties_schedule: None,
				max_issuance: None,
				source_chain: OriginChain::Ethereum
			}
		);
	})
}

#[test]
fn do_deposit_creates_tokens_and_collection() {
	ExtBuilder::default().build().execute_with(|| {
		let expected_collection_id = Nft::next_collection_uuid().unwrap();
		let test_vals = TestVals::default();
		let token_ids =
			BoundedVec::<BoundedVec<U256, MaxIdsPerMultipleMint>, MaxAddresses>::try_from(vec![
				BoundedVec::<U256, MaxIdsPerMultipleMint>::try_from(vec![U256::from(1)]).unwrap(),
			])
			.unwrap();

		let token_addresses =
			BoundedVec::<H160, MaxAddresses>::try_from(vec![test_vals.token_address]).unwrap();

		assert_ok!(Pallet::<Test>::do_deposit(
			&test_vals.source,
			token_addresses,
			token_ids,
			test_vals.destination
		));

		assert_eq!(Pallet::<Test>::eth_to_root_nft(test_vals.token_address), Some(expected_collection_id));
		assert_eq!(Pallet::<Test>::root_to_eth_nft(expected_collection_id), Some(test_vals.token_address));
		Nft::collection_exists(expected_collection_id);
		assert_eq!(
			Nft::token_balance(AccountId20::from(test_vals.destination))
				.unwrap()
				.get(&expected_collection_id),
			Some(&(2))
		);
	})
}

#[test]
fn do_deposit_works_with_existing_bridged_collection() {
	ExtBuilder::default().build().execute_with(|| {
		let test_vals = TestVals::default();
		let expected_collection_id = Nft::next_collection_uuid().unwrap();

		let token_ids =
			BoundedVec::<BoundedVec<U256, MaxIdsPerMultipleMint>, MaxAddresses>::try_from(vec![
				BoundedVec::<U256, MaxIdsPerMultipleMint>::try_from(vec![U256::from(1)]).unwrap(),
			])
			.unwrap();

		let token_addresses =
			BoundedVec::<H160, MaxAddresses>::try_from(vec![test_vals.token_address]).unwrap();

		// Given existing collection
		assert_ok!(Pallet::<Test>::do_deposit(
			&test_vals.source,
			token_addresses.clone(),
			token_ids,
			test_vals.destination
		));

		assert_eq!(Pallet::<Test>::eth_to_root_nft(test_vals.token_address), Some(expected_collection_id));
		assert_eq!(Pallet::<Test>::root_to_eth_nft(expected_collection_id), Some(test_vals.token_address));
		Nft::collection_exists(expected_collection_id);
		assert_eq!(
			Nft::token_balance(AccountId20::from(test_vals.destination))
				.unwrap()
				.get(&expected_collection_id),
			Some(&(2))
		);

		let new_token_ids =
			BoundedVec::<BoundedVec<U256, MaxIdsPerMultipleMint>, MaxAddresses>::try_from(vec![
				BoundedVec::<U256, MaxIdsPerMultipleMint>::try_from(vec![U256::from(2)]).unwrap(),
			])
			.unwrap();

		// When bridged tokens are sent for existing collection
		assert_ok!(Pallet::<Test>::do_deposit(
			&test_vals.source,
			token_addresses,
			new_token_ids,
			test_vals.destination
		));

		assert_eq!(Pallet::<Test>::eth_to_root_nft(test_vals.token_address), Some(expected_collection_id));
		assert_eq!(Pallet::<Test>::root_to_eth_nft(expected_collection_id), Some(test_vals.token_address));
		// Then balance is increased. Existing collection was updated with new token
		assert_eq!(
			Nft::token_balance(AccountId20::from(test_vals.destination))
				.unwrap()
				.get(&expected_collection_id),
			Some(&(3))
		);
	})
}

#[test]
fn do_withdraw_works() {
	ExtBuilder::default().build().execute_with(|| {
		let collection_id = Nft::next_collection_uuid().unwrap();
		let test_vals = TestVals::default();

		// NFT bridge data encoded
		let data = ethabi::encode(&[
			Token::Uint(U256::from(1)),
			Token::Array(vec![Token::Address(test_vals.token_address)]),
			Token::Array(vec![Token::Array(vec![Token::Uint(test_vals.inner_token_id)])]),
			Token::Address(test_vals.destination),
		]);

		assert_ok!(Pallet::<Test>::on_event(&test_vals.source, &data));
		// Wait for mint to occur
		NftPeg::on_initialize(6);

		let collection_ids = vec![collection_id];

		assert_ok!(Pallet::<Test>::do_withdraw(
			test_vals.destination,
			collection_ids.clone(),
			vec![vec![1]],
			test_vals.source
		));

		assert_eq!(
			Nft::token_balance(AccountId20::from(test_vals.destination)).unwrap().get(&collection_id),
			Some(&1)
		);
	});
}
