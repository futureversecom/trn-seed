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
	mock::{
		has_event, MaxTokensPerCollection, Nft, RuntimeEvent as MockEvent, System, Test, TestExt,
	},
	CollectionInfo, Event as NftEvent, TokenLocks,
};
use seed_pallet_common::test_prelude::*;
use seed_primitives::{OriginChain, RoyaltiesSchedule, TokenCount};

type OwnedTokens = BoundedVec<
	TokenOwnership<
		<Test as frame_system::Config>::AccountId,
		<Test as Config>::MaxTokensPerCollection,
	>,
	<Test as Config>::MaxTokensPerCollection,
>;

// Create an NFT collection
// Returns the created `collection_id`
fn setup_collection(owner: AccountId) -> CollectionUuid {
	let collection_id = Nft::next_collection_uuid().unwrap();
	let collection_name = bounded_string("test-collection");
	let metadata_scheme = MetadataScheme::try_from(b"<CID>".as_slice()).unwrap();
	assert_ok!(Nft::create_collection(
		Some(owner).into(),
		collection_name,
		0,
		None,
		None,
		metadata_scheme,
		None,
		CrossChainCompatibility::default(),
	));
	collection_id
}

/// Helper function to create bounded vec of TokenOwnership
pub fn create_owned_tokens(owned_tokens: Vec<(AccountId, Vec<SerialNumber>)>) -> OwnedTokens {
	let mut token_ownership: OwnedTokens = BoundedVec::default();
	for (owner, serial_numbers) in owned_tokens {
		let serial_numbers_bounded: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(serial_numbers).unwrap();
		let new_token_ownership = TokenOwnership::new(owner, serial_numbers_bounded);
		token_ownership.try_push(new_token_ownership).unwrap();
	}
	token_ownership
}

// Helper function for creating the collection name type
pub fn bounded_string(name: &str) -> BoundedVec<u8, <Test as Config>::StringLimit> {
	BoundedVec::truncate_from(name.as_bytes().to_vec())
}

#[test]
fn next_collection_uuid_works() {
	TestExt::default().build().execute_with(|| {
		// This tests assumes parachain_id is set to 100 in mock

		// | 22 collection_id bits | 10 parachain_id bits |
		// |          1           |   100   |
		// 0b000000000000000000001_0001100100

		// Test with first collection_id (0)
		let account = create_account(1);
		let expected_result = 0b000000000000000000000_0001100100 as u32;
		assert_eq!(setup_collection(account), expected_result);

		// Test with max available for 22 bits
		let next_collection_id = (1 << 22) - 2;
		assert_eq!(next_collection_id, 0b0000000000_1111111111111111111110 as u32);
		<NextCollectionId<Test>>::put(next_collection_id);
		let expected_result = 0b1111111111111111111110_0001100100 as u32;
		assert_eq!(setup_collection(account), expected_result);

		// Next collection_uuid should fail (Reaches 22 bits max)
		assert_noop!(Nft::next_collection_uuid(), Error::<Test>::NoAvailableIds);
	});
}

#[test]
fn owned_tokens_works() {
	TestExt::default().build().execute_with(|| {
		let token_owner = create_account(2);
		let quantity = 5000;
		let collection_id = Nft::next_collection_uuid().unwrap();

		// mint token Ids 0-4999
		assert_ok!(Nft::create_collection(
			Some(token_owner).into(),
			bounded_string("test-collection"),
			quantity,
			None,
			Some(token_owner),
			MetadataScheme::try_from(b"https://example.com/metadata".as_slice()).unwrap(),
			None,
			CrossChainCompatibility::default(),
		));

		// First 100
		let cursor: u32 = 0;
		let limit: u16 = 100;
		let expected_tokens: Vec<SerialNumber> = (cursor..100).collect();
		assert_eq!(
			Nft::owned_tokens(collection_id, &token_owner, cursor, limit),
			(100_u32, quantity, expected_tokens)
		);

		// 100 - 300
		let cursor: u32 = 100;
		let limit: u16 = 200;
		let expected_tokens: Vec<SerialNumber> = (cursor..300).collect();
		assert_eq!(
			Nft::owned_tokens(collection_id, &token_owner, cursor, limit),
			(300_u32, quantity, expected_tokens)
		);

		// Limit higher than MAX_OWNED_TOKENS_LIMIT gets reduced
		let cursor: u32 = 1000;
		let limit: u16 = 10000;
		let expected_tokens: Vec<SerialNumber> =
			(cursor..cursor + MAX_OWNED_TOKENS_LIMIT as u32).collect();
		assert_eq!(
			Nft::owned_tokens(collection_id, &token_owner, cursor, limit),
			(cursor + MAX_OWNED_TOKENS_LIMIT as u32, quantity, expected_tokens)
		);

		// should return empty vec in unknown collection
		let cursor: u32 = 0;
		let limit: u16 = 100;
		let expected_tokens: Vec<SerialNumber> = vec![];
		assert_eq!(
			Nft::owned_tokens(collection_id + 1, &token_owner, cursor, limit),
			(0_u32, 0, expected_tokens)
		);

		// should return empty vec if cursor is set too high
		let cursor: u32 = 5000;
		let limit: u16 = 100;
		let expected_tokens: Vec<SerialNumber> = vec![];
		assert_eq!(
			Nft::owned_tokens(collection_id, &token_owner, cursor, limit),
			(0_u32, quantity, expected_tokens)
		);

		// Last 100 should return cursor of 0
		let cursor: u32 = 4900;
		let limit: u16 = 100;
		let expected_tokens: Vec<SerialNumber> = (cursor..5000).collect();
		assert_eq!(
			Nft::owned_tokens(collection_id, &token_owner, cursor, limit),
			(0, quantity, expected_tokens)
		);
	});
}

#[test]
fn set_owner() {
	TestExt::default().build().execute_with(|| {
		// setup token collection + one token
		let collection_owner = create_account(1);
		let collection_id = setup_collection(collection_owner);
		let new_owner = create_account(2);

		assert_ok!(Nft::set_owner(Some(collection_owner).into(), collection_id, new_owner));
		assert_noop!(
			Nft::set_owner(Some(collection_owner).into(), collection_id, new_owner),
			Error::<Test>::NotCollectionOwner
		);
		assert_noop!(
			Nft::set_owner(Some(collection_owner).into(), collection_id + 1, new_owner),
			Error::<Test>::NoCollectionFound
		);
	});
}

#[test]
fn create_collection() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let token_owner = create_account(2);
		let quantity = 5;
		let collection_id = Nft::next_collection_uuid().unwrap();
		let royalties_schedule = RoyaltiesSchedule {
			entitlements: BoundedVec::truncate_from(vec![(collection_owner, Permill::one())]),
		};

		let expected_tokens = create_owned_tokens(vec![(token_owner, vec![0, 1, 2, 3, 4])]);
		let expected_info = CollectionInformation {
			owner: collection_owner,
			name: bounded_string("test-collection"),
			metadata_scheme: MetadataScheme::try_from(b"https://example.com/metadata".as_slice())
				.unwrap(),
			royalties_schedule: Some(royalties_schedule.clone()),
			max_issuance: None,
			origin_chain: OriginChain::Root,
			next_serial_number: quantity,
			collection_issuance: quantity,
			owned_tokens: expected_tokens,
			cross_chain_compatibility: CrossChainCompatibility::default(),
		};

		// mint token Ids 0-4
		assert_ok!(Nft::create_collection(
			Some(expected_info.owner).into(),
			expected_info.name.clone(),
			expected_info.next_serial_number.clone(),
			None,
			Some(token_owner),
			expected_info.metadata_scheme.clone(),
			expected_info.royalties_schedule.clone(),
			expected_info.cross_chain_compatibility.clone(),
		));

		assert_eq!(CollectionInfo::<Test>::get(collection_id).unwrap(), expected_info);

		// EVM pallet should have account code for collection
		assert!(!pallet_evm::Pallet::<Test>::is_account_empty(
			&H160::from_low_u64_be(collection_id as u64).into()
		));

		assert!(has_event(Event::<Test>::CollectionCreate {
			collection_uuid: collection_id,
			initial_issuance: 5,
			max_issuance: None,
			collection_owner,
			metadata_scheme: MetadataScheme::try_from(b"https://example.com/metadata".as_slice())
				.unwrap(),
			name: b"test-collection".to_vec(),
			royalties_schedule: Some(royalties_schedule.clone()),
			origin_chain: OriginChain::Root,
			compatibility: CrossChainCompatibility::default(),
		}));

		// check token ownership
		assert_eq!(
			CollectionInfo::<Test>::get(collection_id).unwrap().collection_issuance,
			quantity
		);
		assert_eq!(
			CollectionInfo::<Test>::get(collection_id).unwrap().royalties_schedule,
			Some(royalties_schedule)
		);
		// We minted collection token 1, next collection token id is 2
		// Bit shifted to account for parachain_id
		assert_eq!(Nft::next_collection_uuid().unwrap(), collection_id + (1 << 10));
		assert_eq!(
			Nft::owned_tokens(collection_id, &token_owner, 0, 1000),
			(0_u32, quantity, vec![0, 1, 2, 3, 4])
		);
		assert_eq!(Nft::token_balance_of(&token_owner, collection_id), 5);

		// check we can mint some more
		// mint token Ids 5-7
		let additional_quantity = 3;
		let new_owner = create_account(3);
		assert_ok!(Nft::mint(
			Some(collection_owner).into(),
			collection_id,
			additional_quantity,
			Some(new_owner), // new owner this time
		));
		assert!(has_event(Event::<Test>::Mint {
			collection_id,
			start: 5,
			end: 7,
			owner: new_owner,
		}));
		assert_eq!(Nft::token_balance_of(&(new_owner), collection_id), 3);
		assert_eq!(
			CollectionInfo::<Test>::get(collection_id).unwrap().next_serial_number,
			quantity + additional_quantity
		);

		assert_eq!(
			Nft::owned_tokens(collection_id, &token_owner, 0, 1000),
			(0_u32, 5, vec![0, 1, 2, 3, 4])
		);
		assert_eq!(
			Nft::owned_tokens(collection_id, &(new_owner), 0, 1000),
			(0_u32, 3, vec![5, 6, 7])
		);
		assert_eq!(
			CollectionInfo::<Test>::get(collection_id).unwrap().collection_issuance,
			quantity + additional_quantity
		);
	});
}

#[test]
fn create_collection_invalid_name() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let metadata_scheme = MetadataScheme::try_from(b"<CID>".as_slice()).unwrap();

		// empty name
		assert_noop!(
			Nft::create_collection(
				Some(collection_owner).into(),
				bounded_string(""),
				1,
				None,
				None,
				metadata_scheme.clone(),
				None,
				CrossChainCompatibility::default(),
			),
			Error::<Test>::CollectionNameInvalid
		);

		// non UTF-8 chars
		// kudos: https://www.cl.cam.ac.uk/~mgk25/ucs/examples/UTF-8-test.txt
		let bad_collection_name = BoundedVec::truncate_from(vec![0xfe, 0xff]);
		assert_noop!(
			Nft::create_collection(
				Some(collection_owner).into(),
				bad_collection_name,
				1,
				None,
				None,
				metadata_scheme,
				None,
				CrossChainCompatibility::default(),
			),
			Error::<Test>::CollectionNameInvalid
		);
	});
}

#[test]
fn create_collection_royalties_invalid() {
	TestExt::default().build().execute_with(|| {
		let owner = create_account(1);
		let name = bounded_string("test-collection");
		let metadata_scheme = MetadataScheme::try_from(b"<CID>".as_slice()).unwrap();

		// Too big royalties should fail
		let royalty_schedule = RoyaltiesSchedule::<AccountId> {
			entitlements: BoundedVec::truncate_from(vec![
				(create_account(3), Permill::from_float(1.2)),
				(create_account(4), Permill::from_float(3.3)),
			]),
		};
		assert_noop!(
			Nft::create_collection(
				Some(owner).into(),
				name.clone(),
				1,
				None,
				None,
				metadata_scheme.clone(),
				Some(royalty_schedule),
				CrossChainCompatibility::default(),
			),
			Error::<Test>::RoyaltiesInvalid
		);

		// Empty vector should fail
		assert_noop!(
			Nft::create_collection(
				Some(owner).into(),
				name,
				1,
				None,
				None,
				metadata_scheme,
				Some(RoyaltiesSchedule::<AccountId> { entitlements: BoundedVec::default() }),
				CrossChainCompatibility::default(),
			),
			Error::<Test>::RoyaltiesInvalid
		);
	})
}

#[test]
fn transfer() {
	TestExt::default().build().execute_with(|| {
		// setup token collection + one token
		let collection_owner = create_account(1);
		let collection_id = Nft::next_collection_uuid().unwrap();
		let token_owner = create_account(2);
		assert_ok!(Nft::create_collection(
			Some(collection_owner).into(),
			bounded_string("test-collection"),
			1,
			None,
			Some(token_owner),
			MetadataScheme::try_from(b"<CID>".as_slice()).unwrap(),
			None,
			CrossChainCompatibility::default(),
		));

		let new_owner = create_account(3);
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![0]).unwrap();
		assert_ok!(Nft::transfer(
			Some(token_owner).into(),
			collection_id,
			serial_numbers.clone(),
			new_owner
		));
		assert!(has_event(Event::<Test>::Transfer {
			previous_owner: token_owner,
			collection_id,
			new_owner,
			serial_numbers: serial_numbers.into_inner(),
		}));

		assert_eq!(Nft::token_balance_of(&token_owner, collection_id), 0);
		assert_eq!(Nft::token_balance_of(&new_owner, collection_id), 1);
		assert!(CollectionInfo::<Test>::get(collection_id)
			.unwrap()
			.is_token_owner(&new_owner, 0));
	});
}

#[test]
fn transfer_fails_prechecks() {
	TestExt::default().build().execute_with(|| {
		// setup token collection + one token
		let collection_owner = create_account(1);
		let collection_id = Nft::next_collection_uuid().unwrap();
		let token_owner = create_account(2);
		let new_owner = create_account(3);
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![0]).unwrap();

		// no token yet
		assert_noop!(
			Nft::transfer(
				Some(token_owner).into(),
				collection_id,
				serial_numbers.clone(),
				new_owner
			),
			Error::<Test>::NoCollectionFound,
		);

		assert_ok!(Nft::create_collection(
			Some(collection_owner).into(),
			bounded_string("test-collection"),
			1,
			None,
			Some(token_owner),
			MetadataScheme::try_from(b"<CID>".as_slice()).unwrap(),
			None,
			CrossChainCompatibility::default(),
		));

		assert_noop!(
			Nft::transfer(
				Some(new_owner).into(),
				collection_id,
				serial_numbers.clone(),
				token_owner
			),
			Error::<Test>::NotTokenOwner,
		);

		// cannot transfer while listed
		<TokenLocks<Test>>::insert((collection_id, 0), TokenLockReason::Listed(1));
		assert_noop!(
			Nft::transfer(Some(token_owner).into(), collection_id, serial_numbers, new_owner),
			Error::<Test>::TokenLocked,
		);
	});
}

#[test]
fn burn() {
	TestExt::default().build().execute_with(|| {
		// setup token collection + one token
		let collection_owner = create_account(1);
		let collection_id = Nft::next_collection_uuid().unwrap();
		let token_owner = create_account(2);

		assert_ok!(Nft::create_collection(
			Some(collection_owner).into(),
			bounded_string("test-collection"),
			3,
			None,
			Some(token_owner),
			MetadataScheme::try_from(b"https://example.com/metadata".as_slice()).unwrap(),
			None,
			CrossChainCompatibility::default(),
		));

		// test
		assert_ok!(Nft::burn(Some(token_owner).into(), (collection_id, 0)));
		assert!(has_event(Event::<Test>::Burn { collection_id, serial_number: 0 }));
		assert_eq!(Nft::token_balance_of(&token_owner, collection_id), 2);

		assert_ok!(Nft::burn(Some(token_owner).into(), (collection_id, 1)));
		assert!(has_event(Event::<Test>::Burn { collection_id, serial_number: 1 }));
		assert_ok!(Nft::burn(Some(token_owner).into(), (collection_id, 2)));
		assert!(has_event(Event::<Test>::Burn { collection_id, serial_number: 2 }));

		assert_eq!(CollectionInfo::<Test>::get(collection_id).unwrap().collection_issuance, 0);
		assert_eq!(
			Nft::owned_tokens(collection_id, &token_owner, 0, 1000),
			(0_u32, 0_u32, vec![].into())
		);
		assert_eq!(Nft::token_balance_of(&token_owner, collection_id), 0);
	});
}

#[test]
fn burn_fails_prechecks() {
	TestExt::default().build().execute_with(|| {
		// setup token collection + one token
		let collection_owner = create_account(1);
		let collection_id = Nft::next_collection_uuid().unwrap();
		let token_owner = create_account(2);

		// token doesn't exist yet
		assert_noop!(
			Nft::burn(Some(token_owner).into(), (collection_id, 0)),
			Error::<Test>::NoCollectionFound
		);

		assert_ok!(Nft::create_collection(
			Some(collection_owner).into(),
			bounded_string("test-collection"),
			100,
			None,
			Some(token_owner),
			MetadataScheme::try_from(b"https://example.com/metadata".as_slice()).unwrap(),
			None,
			CrossChainCompatibility::default(),
		));

		// Not owner
		assert_noop!(
			Nft::burn(Some(create_account(3)).into(), (collection_id, 0)),
			Error::<Test>::NotTokenOwner,
		);

		// cannot burn while listed
		<TokenLocks<Test>>::insert((collection_id, 0), TokenLockReason::Listed(1));

		assert_noop!(
			Nft::burn(Some(token_owner).into(), (collection_id, 0)),
			Error::<Test>::TokenLocked,
		);
	});
}

#[test]
fn mint_over_max_issuance_should_fail() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let token_owner = create_account(2);
		let initial_issuance = 2;
		let max_issuance = 5;
		let collection_id = Nft::next_collection_uuid().unwrap();

		// mint token Ids 0-1
		assert_ok!(Nft::create_collection(
			Some(collection_owner).into(),
			bounded_string("test-collection"),
			initial_issuance,
			Some(max_issuance),
			Some(token_owner),
			MetadataScheme::try_from(b"https://example.com/metadata".as_slice()).unwrap(),
			None,
			CrossChainCompatibility::default(),
		));
		assert_eq!(
			CollectionInfo::<Test>::get(collection_id).unwrap().collection_issuance,
			initial_issuance
		);

		// Mint tokens 2-4
		assert_ok!(Nft::mint(Some(collection_owner).into(), collection_id, 3, Some(token_owner),));
		assert!(has_event(Event::<Test>::Mint {
			collection_id,
			start: 2,
			end: 4,
			owner: token_owner,
		}));
		assert_eq!(
			CollectionInfo::<Test>::get(collection_id).unwrap().collection_issuance,
			initial_issuance + 3
		);

		// No more can be minted as max issuance has been reached
		assert_noop!(
			Nft::mint(Some(collection_owner).into(), collection_id, 1, Some(token_owner)),
			Error::<Test>::MaxIssuanceReached
		);

		// Even if tokens are burned, more can't be minted
		assert_ok!(Nft::burn(Some(token_owner).into(), (collection_id, 0)));
		assert_noop!(
			Nft::mint(Some(collection_owner).into(), collection_id, 1, Some(token_owner)),
			Error::<Test>::MaxIssuanceReached
		);
	});
}

#[test]
fn invalid_max_issuance_should_fail() {
	TestExt::default().build().execute_with(|| {
		// Max issuance of 0 should fail
		assert_noop!(
			Nft::create_collection(
				Some(create_account(1)).into(),
				bounded_string("test-collection"),
				0,
				Some(0),
				None,
				MetadataScheme::try_from(b"https://example.com/metadata".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			),
			Error::<Test>::InvalidMaxIssuance
		);

		// Max issuance lower than initial issuance should fail
		assert_noop!(
			Nft::create_collection(
				Some(create_account(1)).into(),
				bounded_string("test-collection"),
				5,
				Some(2),
				None,
				MetadataScheme::try_from(b"https://example.com/metadata".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			),
			Error::<Test>::InvalidMaxIssuance
		);

		// Max issuance higher than maxTokensPerCollection should fail
		assert_noop!(
			Nft::create_collection(
				Some(create_account(1)).into(),
				bounded_string("test-collection"),
				5,
				Some(mock::MaxTokensPerCollection::get() + 1),
				None,
				MetadataScheme::try_from(b"https://example.com/metadata".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			),
			Error::<Test>::InvalidMaxIssuance
		);
	});
}

#[test]
fn mint_fails() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let collection_id = Nft::next_collection_uuid().unwrap();

		// mint token Ids 0-4
		assert_ok!(Nft::create_collection(
			Some(collection_owner).into(),
			bounded_string("test-collection"),
			5,
			None,
			None,
			MetadataScheme::try_from(b"https://example.com/metadata".as_slice()).unwrap(),
			None,
			CrossChainCompatibility::default(),
		));

		// add 0 additional fails
		assert_noop!(
			Nft::mint(Some(collection_owner).into(), collection_id, 0, None),
			Error::<Test>::NoToken
		);

		// add to non-existing collection fails
		assert_noop!(
			Nft::mint(Some(collection_owner).into(), collection_id + 1, 5, None),
			Error::<Test>::NoCollectionFound
		);

		// public mint not enabled
		assert_noop!(
			Nft::mint(Some(create_account(2)).into(), collection_id, 5, None),
			Error::<Test>::PublicMintDisabled
		);
	});
}

#[test]
fn mint_over_mint_limit_fails() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let collection_id = Nft::next_collection_uuid().unwrap();

		// mint token Ids 0-4
		assert_ok!(Nft::create_collection(
			Some(collection_owner).into(),
			bounded_string("test-collection"),
			5,
			None,
			None,
			MetadataScheme::try_from(b"https://example.com/metadata".as_slice()).unwrap(),
			None,
			CrossChainCompatibility::default(),
		));

		// Should fail attempting to mint MintLimit + 1
		assert_noop!(
			Nft::mint(
				Some(collection_owner).into(),
				collection_id,
				<Test as Config>::MintLimit::get() + 1,
				None
			),
			Error::<Test>::MintLimitExceeded
		);
	});
}

#[test]
fn create_collection_over_mint_limit_fails() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);

		// Initial issuance over mint limit should fail
		assert_noop!(
			Nft::create_collection(
				Some(collection_owner).into(),
				bounded_string("test-collection"),
				<Test as Config>::MintLimit::get() + 1,
				None,
				None,
				MetadataScheme::try_from(b"https://example.com/metadata".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			),
			Error::<Test>::MintLimitExceeded
		);
	});
}

#[test]
fn token_uri_construction() {
	TestExt::default().build().execute_with(|| {
		let owner = create_account(1);
		let quantity = 5;
		let collection_id = Nft::next_collection_uuid().unwrap();
		// mint token Ids
		assert_ok!(Nft::create_collection(
			Some(owner).into(),
			bounded_string("test-collection"),
			quantity,
			None,
			None,
			MetadataScheme::try_from(b"https://example.com/metadata/".as_slice()).unwrap(),
			None,
			CrossChainCompatibility::default(),
		));

		assert_eq!(Nft::token_uri((collection_id, 0)), b"https://example.com/metadata/0".to_vec(),);
		assert_eq!(Nft::token_uri((collection_id, 1)), b"https://example.com/metadata/1".to_vec(),);
	});
}

#[test]
fn transfer_to_signer_address() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let collection_id = Nft::next_collection_uuid().unwrap();
		let token_owner = create_account(2);
		let initial_quantity: u32 = 3;

		// Mint 3 tokens
		assert_ok!(Nft::create_collection(
			Some(collection_owner).into(),
			bounded_string("test-collection"),
			initial_quantity,
			None,
			Some(token_owner),
			MetadataScheme::try_from(b"<CID>".as_slice()).unwrap(),
			None,
			CrossChainCompatibility::default(),
		));

		assert_eq!(Nft::token_balance_of(&token_owner, collection_id), initial_quantity);

		// Transfer 2 tokens to signer address
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![0, 1]).unwrap();
		assert_noop!(
			Nft::transfer(Some(token_owner).into(), collection_id, serial_numbers, token_owner),
			Error::<Test>::InvalidNewOwner
		);

		// Check storage remains the same
		assert_eq!(Nft::token_balance_of(&token_owner, collection_id), initial_quantity);
	});
}

#[test]
fn transfer_changes_token_balance() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let collection_id = Nft::next_collection_uuid().unwrap();
		let token_owner = create_account(2);
		let new_owner = create_account(3);
		let initial_quantity: u32 = 1;

		// Mint token
		assert_ok!(Nft::create_collection(
			Some(collection_owner).into(),
			bounded_string("test-collection"),
			initial_quantity,
			None,
			Some(token_owner),
			MetadataScheme::try_from(b"<CID>".as_slice()).unwrap(),
			None,
			CrossChainCompatibility::default(),
		));

		assert_eq!(Nft::token_balance_of(&token_owner, collection_id), initial_quantity);
		assert_eq!(Nft::token_balance_of(&new_owner, collection_id), 0);

		// Mint an additional 2 tokens
		let additional_quantity: u32 = 2;
		assert_ok!(Nft::mint(
			Some(collection_owner).into(),
			collection_id,
			additional_quantity,
			Some(token_owner),
		));
		assert!(has_event(Event::<Test>::Mint {
			collection_id,
			start: 1,
			end: 2,
			owner: token_owner,
		}));

		assert_eq!(
			Nft::token_balance_of(&token_owner, collection_id),
			initial_quantity + additional_quantity
		);
		assert_eq!(Nft::token_balance_of(&new_owner, collection_id), 0);

		// Transfer 2 tokens
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![0, 1]).unwrap();
		let transfer_quantity: u32 = serial_numbers.len() as u32;
		assert_ok!(Nft::transfer(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			new_owner
		));

		assert_eq!(
			Nft::token_balance_of(&token_owner, collection_id),
			initial_quantity + additional_quantity - transfer_quantity
		);
		assert_eq!(Nft::token_balance_of(&new_owner, collection_id), transfer_quantity);
	});
}

#[test]
fn transfer_many_tokens_changes_token_balance() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let collection_id = Nft::next_collection_uuid().unwrap();
		let token_owner = create_account(2);
		let new_owner = create_account(3);
		let initial_quantity: u32 = 100;

		// Mint tokens
		assert_ok!(Nft::create_collection(
			Some(collection_owner).into(),
			bounded_string("test-collection"),
			initial_quantity,
			None,
			Some(token_owner),
			MetadataScheme::try_from(b"<CID>".as_slice()).unwrap(),
			None,
			CrossChainCompatibility::default(),
		));

		assert_eq!(Nft::token_balance_of(&token_owner, collection_id), initial_quantity);
		assert_eq!(Nft::token_balance_of(&new_owner, collection_id), 0);

		for i in 0_u32..initial_quantity {
			// Transfer token
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![i]).unwrap();
			assert_ok!(Nft::transfer(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				new_owner,
			));

			// Check storage
			let changed_quantity = i + 1;
			assert_eq!(
				Nft::token_balance_of(&token_owner, collection_id),
				initial_quantity - changed_quantity
			);
			assert_eq!(Nft::token_balance_of(&new_owner, collection_id), changed_quantity);
		}
	});
}

#[test]
fn transfer_many_tokens_at_once_changes_token_balance() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let collection_id = Nft::next_collection_uuid().unwrap();
		let token_owner = create_account(2);
		let new_owner = create_account(3);
		let initial_quantity: u32 = 100;
		let transfer_quantity: u32 = 66;

		// Mint tokens
		assert_ok!(Nft::create_collection(
			Some(collection_owner).into(),
			bounded_string("test-collection"),
			initial_quantity,
			None,
			Some(token_owner),
			MetadataScheme::try_from(b"<CID>".as_slice()).unwrap(),
			None,
			CrossChainCompatibility::default(),
		));
		assert_eq!(Nft::token_balance_of(&token_owner, collection_id), initial_quantity);
		assert_eq!(Nft::token_balance_of(&new_owner, collection_id), 0);

		// Transfer tokens
		let serial_numbers_unbounded: Vec<SerialNumber> = (0..transfer_quantity).collect();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(serial_numbers_unbounded.clone()).unwrap();
		assert_ok!(Nft::transfer(
			Some(token_owner).into(),
			collection_id,
			serial_numbers.clone(),
			new_owner
		));

		assert!(has_event(Event::<Test>::Transfer {
			previous_owner: token_owner,
			collection_id,
			new_owner,
			serial_numbers: serial_numbers_unbounded,
		}));

		// Check storage
		assert_eq!(
			Nft::token_balance_of(&token_owner, collection_id),
			initial_quantity - transfer_quantity
		);
		assert_eq!(Nft::token_balance_of(&new_owner, collection_id), transfer_quantity);
	});
}

#[test]
fn cannot_mint_bridged_collections() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let token_owner = create_account(2);

		let collection_id = Pallet::<Test>::do_create_collection(
			collection_owner,
			bounded_string("test-collection"),
			0,
			None,
			None,
			MetadataScheme::try_from(H160::zero().as_bytes()).unwrap(),
			None,
			// "From ethereum"
			OriginChain::Ethereum,
			CrossChainCompatibility::default(),
		)
		.unwrap();

		// Collection already exists on origin chain; not allowed to be minted here
		assert_noop!(
			Nft::mint(Some(collection_owner).into(), collection_id, 420, Some(token_owner)),
			Error::<Test>::AttemptedMintOnBridgedToken
		);
	});
}

#[test]
fn mints_multiple_specified_tokens_by_id() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let token_owner = create_account(2);
		let token_ids: Vec<SerialNumber> = vec![0, 2, 5, 9, 1000];
		let collection_id = Nft::next_collection_uuid().unwrap();

		assert_ok!(Nft::do_create_collection(
			collection_owner,
			bounded_string("test-collection"),
			0,
			None,
			None,
			MetadataScheme::try_from(b"<CID>".as_slice()).unwrap(),
			None,
			OriginChain::Ethereum,
			CrossChainCompatibility::default(),
		));

		// Do mint with Ethereum as origin chain
		let _ = Nft::mint_bridged_token(&token_owner, collection_id, token_ids.clone());

		// Event is thrown
		assert!(has_event(Event::<Test>::BridgedMint {
			collection_id,
			serial_numbers: BoundedVec::truncate_from(token_ids.clone()),
			owner: token_owner,
		}));

		// Ownership checks
		assert_eq!(Nft::token_balance_of(&token_owner, collection_id), token_ids.len() as u32);
		let collection_info = CollectionInfo::<Test>::get(collection_id).unwrap();
		token_ids.iter().for_each(|&serial_number| {
			assert!(collection_info.is_token_owner(&token_owner, serial_number));
		});

		// Next serial number should be 0, origin chain is Ethereum so we don't count this
		assert_eq!(CollectionInfo::<Test>::get(collection_id).unwrap().next_serial_number, 0);
	});
}

#[test]
fn mint_duplicate_token_id_should_fail_silently() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let token_owner = create_account(2);
		let token_ids: Vec<SerialNumber> = vec![0, 2, 5, 9, 1000, 0, 2, 5, 9, 1000];
		let collection_id = Nft::next_collection_uuid().unwrap();

		assert_ok!(Nft::do_create_collection(
			collection_owner,
			bounded_string("test-collection"),
			0,
			None,
			None,
			MetadataScheme::try_from(b"<CID>".as_slice()).unwrap(),
			None,
			OriginChain::Ethereum,
			CrossChainCompatibility::default(),
		));

		// Do mint with Ethereum as origin chain
		let _ = Nft::mint_bridged_token(&token_owner, collection_id, token_ids.clone());
		// Minting to another account_id should still succeed, but the token balance of this account
		// will be 0. This is because the tokens are already minted and each token will be silently
		// skipped
		let other_owner = create_account(4);
		let _ = Nft::mint_bridged_token(&other_owner, collection_id, token_ids.clone());

		// Ownership checks
		// We expect the token balance to be 5 as that is the number of unique token_ids in the vec
		assert_eq!(Nft::token_balance_of(&token_owner, collection_id), 5);

		let collection_info = CollectionInfo::<Test>::get(collection_id).unwrap();
		token_ids.iter().for_each(|&serial_number| {
			assert!(collection_info.is_token_owner(&token_owner, serial_number));
		});

		// Collection issuance should be 5 to indicate the 5 unique tokens
		assert_eq!(CollectionInfo::<Test>::get(collection_id).unwrap().collection_issuance, 5_u32);
		// Other owner shouldn't have any tokens
		assert_eq!(Nft::token_balance_of(&other_owner, collection_id), 0);

		// Now try with 3 more unique tokens
		let token_ids: Vec<SerialNumber> = vec![0, 2, 3000, 40005, 5, 1234, 9, 1000];
		let _ = Nft::mint_bridged_token(&other_owner, collection_id, token_ids.clone());

		// Collection issuance should now be 8 to indicate the 3 additional unique tokens
		assert_eq!(CollectionInfo::<Test>::get(collection_id).unwrap().collection_issuance, 8_u32);
		// We expect the token balance to be 3
		assert_eq!(Nft::token_balance_of(&other_owner, collection_id), 3);

		let collection_info = CollectionInfo::<Test>::get(collection_id).unwrap();
		vec![3000, 40005, 1234].iter().for_each(|&serial_number| {
			assert!(collection_info.is_token_owner(&other_owner, serial_number));
		});
	});
}

#[test]
fn token_exists_works() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let quantity: TokenCount = 100;
		let collection_id = Nft::next_collection_uuid().unwrap();

		assert_ok!(Nft::do_create_collection(
			collection_owner,
			bounded_string("test-collection"),
			quantity,
			None,
			None,
			MetadataScheme::try_from(b"<CID>".as_slice()).unwrap(),
			None,
			OriginChain::Root,
			CrossChainCompatibility::default(),
		));

		let collection_info = CollectionInfo::<Test>::get(collection_id).unwrap();

		// Check that the tokens exist
		for serial_number in 0..quantity {
			assert!(collection_info.token_exists(serial_number));
		}

		// Check that a non-existent token does not exist
		for serial_number in quantity..1000 {
			assert!(!collection_info.token_exists(serial_number));
		}
	});
}

#[test]
fn token_balance_of_works() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let token_owner = create_account(2);
		let quantity: TokenCount = 100;
		let collection_id = setup_collection(collection_owner);

		// Check that token_owner has 0 tokens initially
		assert_eq!(Nft::token_balance_of(&token_owner, collection_id), 0);

		assert_ok!(Nft::mint(
			Some(collection_owner).into(),
			collection_id,
			quantity,
			Some(token_owner),
		));
		assert!(has_event(Event::<Test>::Mint {
			collection_id,
			start: 0,
			end: 99,
			owner: token_owner,
		}));

		// Check that token_owner has 100 tokens
		assert_eq!(Nft::token_balance_of(&token_owner, collection_id), quantity);
		// Check that collection_owner has 0 tokens
		assert_eq!(Nft::token_balance_of(&collection_owner, collection_id), 0);
		// Check that random accounts have 0 tokens
		for i in 4..1000 {
			let owner = create_account(i);
			assert_eq!(Nft::token_balance_of(&owner, collection_id), 0);
		}
	});
}

#[test]
fn add_user_tokens_works() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let token_owner = create_account(2);
		let tokens: Vec<SerialNumber> = vec![0, 1, 2, 3, 900, 1000, 101010101];
		let collection_id = setup_collection(collection_owner);
		let mut collection_info = CollectionInfo::<Test>::get(collection_id).unwrap();
		let expected_owned_tokens: OwnedTokens = BoundedVec::default();
		// Initially, owned tokens should be empty
		assert_eq!(collection_info.owned_tokens, expected_owned_tokens);

		// Add tokens to token_owner
		let tokens_bounded: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(tokens.clone()).unwrap();
		assert_ok!(collection_info.add_user_tokens(&token_owner, tokens_bounded.clone()));

		let expected_owned_tokens = create_owned_tokens(vec![(token_owner, tokens.clone())]);
		assert_eq!(collection_info.owned_tokens, expected_owned_tokens);

		// Add tokens to token_owner_2
		let token_owner_2 = create_account(3);
		let tokens_2: Vec<SerialNumber> = vec![6, 9, 4, 2, 0];
		let tokens_2_bounded: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(tokens_2.clone()).unwrap();
		assert_ok!(collection_info.add_user_tokens(&token_owner_2, tokens_2_bounded.clone()));

		let expected_owned_tokens =
			create_owned_tokens(vec![(token_owner, tokens), (token_owner_2, tokens_2.clone())]);
		assert_eq!(collection_info.owned_tokens, expected_owned_tokens);

		// Now remove some tokens from token_owner
		let tokens_to_remove: Vec<SerialNumber> = vec![0, 1, 2, 3];
		let tokens_to_remove_bounded: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(tokens_to_remove.clone()).unwrap();
		collection_info.remove_user_tokens(&token_owner, tokens_to_remove_bounded);
		let expected_owned_tokens = create_owned_tokens(vec![
			(token_owner, vec![900, 1000, 101010101]),
			(token_owner_2, tokens_2),
		]);
		assert_eq!(collection_info.owned_tokens, expected_owned_tokens);
	});
}

#[test]
fn add_user_tokens_over_token_limit_should_fail() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let token_owner = create_account(2);
		let token_owner_2 = create_account(3);
		let collection_id = setup_collection(collection_owner);
		let mut collection_info = CollectionInfo::<Test>::get(collection_id).unwrap();
		let max = mock::MaxTokensPerCollection::get();

		// Add tokens to token_owner
		let serial_numbers_unbounded: Vec<SerialNumber> = (0..max).collect();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(serial_numbers_unbounded).unwrap();
		assert_ok!(collection_info.add_user_tokens(&token_owner, serial_numbers.clone()));

		// Adding one more token to token_owner should fail
		let serial_numbers_max: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![max]).unwrap();
		assert_noop!(
			collection_info.add_user_tokens(&token_owner, serial_numbers_max.clone()),
			TokenOwnershipError::TokenLimitExceeded
		);
		// Adding tokens to different user still works
		assert_ok!(collection_info.add_user_tokens(&token_owner_2, serial_numbers_max.clone()));

		// Now let's remove a token
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![1]).unwrap();
		collection_info.remove_user_tokens(&token_owner, serial_numbers);
		// Adding one more token to token_owner should now work
		assert_ok!(collection_info.add_user_tokens(&token_owner, serial_numbers_max));
	});
}

#[test]
fn add_user_tokens_over_user_limit_should_fail() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let collection_id = setup_collection(collection_owner);
		let mut collection_info = CollectionInfo::<Test>::get(collection_id).unwrap();
		let max = mock::MaxTokensPerCollection::get();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![100]).unwrap();

		// Adding users up to max should work
		for i in 0..max as u64 {
			let account = create_account(i);
			assert_ok!(collection_info.add_user_tokens(&account, serial_numbers.clone()));
		}

		// adding another user should fail
		assert_noop!(
			collection_info.add_user_tokens(&create_account(max as u64), serial_numbers),
			TokenOwnershipError::TokenLimitExceeded
		);
	});
}

mod claim_unowned_collection {
	use super::*;

	#[test]
	fn can_claim_ownership() {
		TestExt::default().build().execute_with(|| {
			let metadata = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
			let collection_id = Nft::next_collection_uuid().unwrap();
			let pallet_account = Nft::account_id();
			let new_owner = create_account(10);

			assert_ne!(new_owner, pallet_account);
			assert_ok!(Nft::create_collection(
				RawOrigin::Signed(pallet_account.clone()).into(),
				bounded_string("test-collection"),
				0,
				None,
				None,
				metadata,
				None,
				CrossChainCompatibility::default(),
			));
			assert_ok!(Nft::claim_unowned_collection(
				RawOrigin::Root.into(),
				collection_id,
				new_owner.clone()
			));

			// Storage
			assert_eq!(CollectionInfo::<Test>::get(collection_id).unwrap().owner, new_owner);

			// Events
			let event = NftEvent::CollectionClaimed { account: new_owner, collection_id };
			let event = MockEvent::Nft(event);
			assert_eq!(System::events().last().unwrap().event, event);
		});
	}

	#[test]
	fn origin_needs_to_be_root() {
		TestExt::default().build().execute_with(|| {
			let metadata = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
			let collection_id = Nft::next_collection_uuid().unwrap();
			let pallet_account = Nft::account_id();
			let new_owner = create_account(10);

			assert_ok!(Nft::create_collection(
				RawOrigin::Signed(pallet_account.clone()).into(),
				bounded_string("test-collection"),
				0,
				None,
				None,
				metadata,
				None,
				CrossChainCompatibility::default(),
			));
			let ok = Nft::claim_unowned_collection(
				RawOrigin::Signed(new_owner.clone()).into(),
				collection_id,
				new_owner.clone(),
			);
			assert_noop!(ok, BadOrigin);
		});
	}

	#[test]
	fn collection_needs_to_exist() {
		TestExt::default().build().execute_with(|| {
			let collection_id = Nft::next_collection_uuid().unwrap();
			let new_owner = create_account(10);

			let ok = Nft::claim_unowned_collection(
				RawOrigin::Root.into(),
				collection_id,
				new_owner.clone(),
			);
			assert_noop!(ok, Error::<Test>::NoCollectionFound);
		});
	}

	#[test]
	fn collection_needs_to_be_owned_by_pallet() {
		TestExt::default().build().execute_with(|| {
			let metadata = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
			let collection_id = Nft::next_collection_uuid().unwrap();
			let new_owner = create_account(10);
			let old_owner = create_account(10);

			assert_ok!(Nft::create_collection(
				RawOrigin::Signed(old_owner.clone()).into(),
				bounded_string("test-collection"),
				0,
				None,
				None,
				metadata,
				None,
				CrossChainCompatibility::default(),
			));
			let ok = Nft::claim_unowned_collection(
				RawOrigin::Root.into(),
				collection_id,
				new_owner.clone(),
			);
			assert_noop!(ok, Error::<Test>::CannotClaimNonClaimableCollections);
		});
	}
}

#[test]
fn create_xls20_collection_works() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(10);
		let collection_name = bounded_string("test-xls20-collection");
		let collection_id = Nft::next_collection_uuid().unwrap();
		let metadata_scheme = MetadataScheme::try_from(b"https://example.com".as_slice()).unwrap();
		let cross_chain_compatibility = CrossChainCompatibility { xrpl: true };
		let initial_issuance: TokenCount = 0;

		assert_ok!(Nft::create_collection(
			Some(collection_owner).into(),
			collection_name.clone(),
			initial_issuance,
			None,
			None,
			metadata_scheme.clone(),
			None,
			cross_chain_compatibility.clone(),
		));
		let expected_tokens = create_owned_tokens(vec![]);

		assert!(has_event(Event::<Test>::CollectionCreate {
			collection_uuid: collection_id,
			initial_issuance,
			max_issuance: None,
			collection_owner,
			metadata_scheme: metadata_scheme.clone(),
			name: collection_name.clone().into_inner(),
			royalties_schedule: None,
			origin_chain: OriginChain::Root,
			compatibility: cross_chain_compatibility,
		}));

		// Check storage is correct
		assert_eq!(
			CollectionInfo::<Test>::get(collection_id).unwrap(),
			CollectionInformation {
				owner: collection_owner,
				name: collection_name,
				metadata_scheme,
				royalties_schedule: None,
				max_issuance: None,
				origin_chain: OriginChain::Root,
				next_serial_number: 0,
				collection_issuance: 0,
				owned_tokens: expected_tokens,
				cross_chain_compatibility,
			}
		);
	});
}

#[test]
fn create_xls20_collection_with_initial_issuance_fails() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(10);
		let collection_name = bounded_string("test-xls20-collection");
		let metadata_scheme = MetadataScheme::try_from(b"https://example.com".as_slice()).unwrap();
		let cross_chain_compatibility = CrossChainCompatibility { xrpl: true };
		let initial_issuance: TokenCount = 1;

		assert_noop!(
			Nft::create_collection(
				Some(collection_owner).into(),
				collection_name,
				initial_issuance,
				None,
				None,
				metadata_scheme,
				None,
				cross_chain_compatibility,
			),
			Error::<Test>::InitialIssuanceNotZero
		);
	});
}

mod set_max_issuance {
	use super::*;

	#[test]
	fn set_max_issuance_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = Nft::next_collection_uuid().unwrap();

			// Setup collection with no Max issuance
			assert_ok!(Nft::create_collection(
				RawOrigin::Signed(collection_owner).into(),
				bounded_string("test-collection"),
				0,
				None,
				None,
				MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			));

			// Sanity check
			assert_eq!(CollectionInfo::<Test>::get(collection_id).unwrap().max_issuance, None);

			let max_issuance: TokenCount = 100;
			assert_ok!(Nft::set_max_issuance(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				max_issuance
			));

			// Storage updated
			assert_eq!(
				CollectionInfo::<Test>::get(collection_id).unwrap().max_issuance,
				Some(max_issuance)
			);

			// Event thrown
			assert!(has_event(Event::<Test>::MaxIssuanceSet { collection_id, max_issuance }));
		});
	}

	#[test]
	fn set_max_issuance_prevents_further_minting_when_reached() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = Nft::next_collection_uuid().unwrap();
			let max_issuance: TokenCount = 100;

			// Setup collection with no Max issuance and initial issuance of 100
			assert_ok!(Nft::create_collection(
				RawOrigin::Signed(collection_owner).into(),
				bounded_string("test-collection"),
				max_issuance,
				None,
				None,
				MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			));

			assert_ok!(Nft::set_max_issuance(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				max_issuance
			));

			// Storage updated
			assert_eq!(
				CollectionInfo::<Test>::get(collection_id).unwrap().max_issuance,
				Some(max_issuance)
			);

			// Further NFTs can't be minted
			assert_noop!(
				Nft::mint(Some(collection_owner).into(), collection_id, 1, None),
				Error::<Test>::MaxIssuanceReached
			);
		});
	}

	#[test]
	fn set_max_issuance_not_owner_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = Nft::next_collection_uuid().unwrap();
			let max_issuance: TokenCount = 100;

			// Setup collection with no Max issuance
			assert_ok!(Nft::create_collection(
				RawOrigin::Signed(collection_owner).into(),
				bounded_string("test-collection"),
				0,
				None,
				None,
				MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			));

			// Bob isn't collection owner, should fail
			let bob = create_account(11);
			assert_noop!(
				Nft::set_max_issuance(RawOrigin::Signed(bob).into(), collection_id, max_issuance),
				Error::<Test>::NotCollectionOwner
			);
		});
	}

	#[test]
	fn set_max_issuance_zero_issuance_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = Nft::next_collection_uuid().unwrap();
			let max_issuance: TokenCount = 0;

			// Setup collection with no Max issuance
			assert_ok!(Nft::create_collection(
				RawOrigin::Signed(collection_owner).into(),
				bounded_string("test-collection"),
				0,
				None,
				None,
				MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			));

			// Max issuance set to 0 should fail
			assert_noop!(
				Nft::set_max_issuance(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					max_issuance
				),
				Error::<Test>::InvalidMaxIssuance
			);
		});
	}

	#[test]
	fn set_max_issuance_no_collection_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = 1;
			let max_issuance: TokenCount = 100;

			// No collection exists, should fail
			assert_noop!(
				Nft::set_max_issuance(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					max_issuance
				),
				Error::<Test>::NoCollectionFound
			);
		});
	}

	#[test]
	fn set_max_issuance_already_set_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = Nft::next_collection_uuid().unwrap();
			let max_issuance: TokenCount = 100;

			// Setup collection with some Max issuance
			assert_ok!(Nft::create_collection(
				RawOrigin::Signed(collection_owner).into(),
				bounded_string("test-collection"),
				0,
				Some(max_issuance),
				None,
				MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			));

			// Call should fail as it was set when collection created
			assert_noop!(
				Nft::set_max_issuance(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					max_issuance
				),
				Error::<Test>::MaxIssuanceAlreadySet
			);
		});
	}

	#[test]
	fn set_max_issuance_twice_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = Nft::next_collection_uuid().unwrap();

			// Setup collection with no Max issuance
			assert_ok!(Nft::create_collection(
				RawOrigin::Signed(collection_owner).into(),
				bounded_string("test-collection"),
				0,
				None,
				None,
				MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			));

			// Call first time should work
			let max_issuance: TokenCount = 100;
			assert_ok!(Nft::set_max_issuance(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				max_issuance
			));

			// Storage updated
			assert_eq!(
				CollectionInfo::<Test>::get(collection_id).unwrap().max_issuance,
				Some(max_issuance)
			);

			// Second call should fail
			assert_noop!(
				Nft::set_max_issuance(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					max_issuance + 1
				),
				Error::<Test>::MaxIssuanceAlreadySet
			);
		});
	}

	#[test]
	fn set_max_issuance_too_low_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = Nft::next_collection_uuid().unwrap();
			let initial_issuance = 10;

			// Setup collection with no max issuance but initial issuance of 10
			assert_ok!(Nft::create_collection(
				RawOrigin::Signed(collection_owner).into(),
				bounded_string("test-collection"),
				initial_issuance,
				None,
				None,
				MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			));

			// Call should fail as max_issuance is below initial issuance
			let max_issuance: TokenCount = 1;
			assert_noop!(
				Nft::set_max_issuance(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					max_issuance
				),
				Error::<Test>::InvalidMaxIssuance
			);

			// Call should fail as max_issuance is below initial issuance
			let max_issuance: TokenCount = 9;
			assert_noop!(
				Nft::set_max_issuance(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					max_issuance
				),
				Error::<Test>::InvalidMaxIssuance
			);

			// Call should work as max issuance = initial issuance
			let max_issuance: TokenCount = 10;
			assert_ok!(Nft::set_max_issuance(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				max_issuance
			));

			// Storage updated
			assert_eq!(
				CollectionInfo::<Test>::get(collection_id).unwrap().max_issuance,
				Some(max_issuance)
			);
		});
	}
}

mod set_base_uri {
	use super::*;

	#[test]
	fn set_base_uri_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = Nft::next_collection_uuid().unwrap();
			let metadata_scheme =
				MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();

			// Setup collection with no Max issuance
			assert_ok!(Nft::create_collection(
				RawOrigin::Signed(collection_owner).into(),
				bounded_string("test-collection"),
				0,
				None,
				None,
				metadata_scheme.clone(),
				None,
				CrossChainCompatibility::default(),
			));

			// Sanity check
			assert_eq!(
				CollectionInfo::<Test>::get(collection_id).unwrap().metadata_scheme,
				metadata_scheme
			);

			let new_metadata_scheme: Vec<u8> = "http://zeeshan.com".into();
			assert_ok!(Nft::set_base_uri(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				new_metadata_scheme.clone()
			));

			// Storage updated
			assert_eq!(
				CollectionInfo::<Test>::get(collection_id).unwrap().metadata_scheme,
				MetadataScheme::try_from(b"http://zeeshan.com".as_slice()).unwrap()
			);

			// Event thrown
			assert!(has_event(Event::<Test>::BaseUriSet {
				collection_id,
				base_uri: new_metadata_scheme,
			}));
		});
	}

	#[test]
	fn set_base_uri_all_variants_work() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = setup_collection(collection_owner);

			assert_ok!(Nft::set_base_uri(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				"https://zeeshan.com".into()
			));
			assert_eq!(
				CollectionInfo::<Test>::get(collection_id).unwrap().metadata_scheme,
				MetadataScheme::try_from(b"https://zeeshan.com".as_slice()).unwrap()
			);
		});
	}

	#[test]
	fn set_base_uri_no_collection_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = 1;
			let new_metadata_scheme: Vec<u8> = "http://zeeshan.com".into();

			// Call to unknown collection should fail
			assert_noop!(
				Nft::set_base_uri(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					new_metadata_scheme.clone()
				),
				Error::<Test>::NoCollectionFound
			);
		});
	}

	#[test]
	fn set_base_uri_not_owner_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = setup_collection(collection_owner);
			let new_metadata_scheme: Vec<u8> = "http://zeeshan.com".into();

			// Call from not owner should fail
			let bob = create_account(11);
			assert_noop!(
				Nft::set_base_uri(
					RawOrigin::Signed(bob).into(),
					collection_id,
					new_metadata_scheme.clone()
				),
				Error::<Test>::NotCollectionOwner
			);
		});
	}

	#[test]
	fn set_base_uri_invalid_path_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = setup_collection(collection_owner);

			// Calls with invalid path should fail
			assert_noop!(
				Nft::set_base_uri(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					vec![0; 2000]
				),
				Error::<Test>::InvalidMetadataPath
			);
		});
	}
}

mod set_name {
	use super::*;

	#[test]
	fn set_name_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = Nft::next_collection_uuid().unwrap();
			let name = bounded_string("test-collection");

			// Setup collection with no Max issuance
			assert_ok!(Nft::create_collection(
				RawOrigin::Signed(collection_owner).into(),
				name.clone(),
				0,
				None,
				None,
				MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			));

			// Sanity check
			assert_eq!(CollectionInfo::<Test>::get(collection_id).unwrap().name, name);

			let new_name = bounded_string("yeet");
			assert_ok!(Nft::set_name(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				new_name.clone()
			));

			// Storage updated
			assert_eq!(CollectionInfo::<Test>::get(collection_id).unwrap().name, new_name);

			// Event thrown
			assert!(has_event(Event::<Test>::NameSet { collection_id, name: new_name }));
		});
	}

	#[test]
	fn set_name_no_collection_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = 1;
			let new_name = bounded_string("yeet");

			// Call to unknown collection should fail
			assert_noop!(
				Nft::set_name(RawOrigin::Signed(collection_owner).into(), collection_id, new_name),
				Error::<Test>::NoCollectionFound
			);
		});
	}

	#[test]
	fn set_name_not_owner_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = setup_collection(collection_owner);
			let new_name = bounded_string("yeet");

			// Call from not owner should fail
			let bob = create_account(11);
			assert_noop!(
				Nft::set_name(RawOrigin::Signed(bob).into(), collection_id, new_name),
				Error::<Test>::NotCollectionOwner
			);
		});
	}

	#[test]
	fn set_name_invalid_name_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = setup_collection(collection_owner);

			// Calls with no name should fail
			assert_noop!(
				Nft::set_name(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					bounded_string("")
				),
				Error::<Test>::CollectionNameInvalid
			);

			// non UTF-8 chars
			assert_noop!(
				Nft::set_name(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					BoundedVec::truncate_from(vec![0xfe, 0xff])
				),
				Error::<Test>::CollectionNameInvalid
			);
		});
	}
}

mod set_royalties_schedule {
	use super::*;

	#[test]
	fn set_royalties_schedule_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = Nft::next_collection_uuid().unwrap();
			let name = bounded_string("test-collection");
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(collection_owner, Permill::one())]),
			};
			// Setup collection with no Max issuance
			assert_ok!(Nft::create_collection(
				RawOrigin::Signed(collection_owner).into(),
				name.clone(),
				0,
				None,
				None,
				MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			));

			// Sanity check
			assert!(CollectionInfo::<Test>::get(collection_id)
				.unwrap()
				.royalties_schedule
				.is_none());

			assert_ok!(Nft::set_royalties_schedule(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				royalties_schedule.clone()
			));

			// Storage updated
			assert_eq!(
				CollectionInfo::<Test>::get(collection_id).unwrap().royalties_schedule.unwrap(),
				royalties_schedule
			);

			// Event thrown
			assert!(has_event(Event::<Test>::RoyaltiesScheduleSet {
				collection_id,
				royalties_schedule
			}));
		});
	}

	#[test]
	fn set_royalties_no_collection_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = 1;
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(collection_owner, Permill::one())]),
			};

			// Call to unknown collection should fail
			assert_noop!(
				Nft::set_royalties_schedule(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					royalties_schedule.clone()
				),
				Error::<Test>::NoCollectionFound
			);
		});
	}

	#[test]
	fn set_royalties_not_owner_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = setup_collection(collection_owner);
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(collection_owner, Permill::one())]),
			};

			// Call from not owner should fail
			let bob = create_account(11);
			assert_noop!(
				Nft::set_royalties_schedule(
					RawOrigin::Signed(bob).into(),
					collection_id,
					royalties_schedule.clone()
				),
				Error::<Test>::NotCollectionOwner
			);
		});
	}

	#[test]
	fn set_royalties_invalid_royalties_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = setup_collection(collection_owner);

			// Too big royalties should fail
			let royalties_schedule = RoyaltiesSchedule::<AccountId> {
				entitlements: BoundedVec::truncate_from(vec![
					(create_account(3), Permill::from_float(1.2)),
					(create_account(4), Permill::from_float(3.3)),
				]),
			};

			// Calls with invalid royalties should fail
			assert_noop!(
				Nft::set_royalties_schedule(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					royalties_schedule.clone()
				),
				Error::<Test>::RoyaltiesInvalid
			);
		});
	}
}

mod set_mint_fee {
	use super::*;
	use seed_pallet_common::utils::PublicMintInformation;

	#[test]
	fn set_mint_fee_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = setup_collection(collection_owner);
			let pricing_details: (AssetId, Balance) = (1, 100);

			assert_ok!(Nft::set_mint_fee(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				Some(pricing_details)
			));

			let expected_mint_info =
				PublicMintInformation { enabled: false, pricing_details: Some(pricing_details) };
			assert_eq!(PublicMintInfo::<Test>::get(collection_id).unwrap(), expected_mint_info);

			// Setting to different value works
			let pricing_details: (AssetId, Balance) = (2, 234);

			assert_ok!(Nft::set_mint_fee(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				Some(pricing_details)
			));

			let expected_mint_info =
				PublicMintInformation { enabled: false, pricing_details: Some(pricing_details) };
			assert_eq!(PublicMintInfo::<Test>::get(collection_id).unwrap(), expected_mint_info);

			// Setting to None removes from storage
			assert_ok!(Nft::set_mint_fee(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				None
			));
			assert_eq!(PublicMintInfo::<Test>::get(collection_id), None);
		});
	}

	#[test]
	fn set_mint_fee_should_keep_enabled_flag_intact() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = setup_collection(collection_owner);
			let pricing_details: (AssetId, Balance) = (1, 100);

			// Toggle mint should set enabled to true
			assert_ok!(Nft::toggle_public_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				true
			));

			// Set mint price should update pricing details but keep enabled as true
			assert_ok!(Nft::set_mint_fee(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				Some(pricing_details)
			));

			let expected_mint_info =
				PublicMintInformation { enabled: true, pricing_details: Some(pricing_details) };
			assert_eq!(PublicMintInfo::<Test>::get(collection_id).unwrap(), expected_mint_info);
		});
	}

	#[test]
	fn set_mint_fee_emits_event() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = setup_collection(collection_owner);
			let pricing_details: (AssetId, Balance) = (1, 100);

			assert_ok!(Nft::set_mint_fee(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				Some(pricing_details)
			));

			assert!(has_event(Event::<Test>::MintPriceSet {
				collection_id,
				payment_asset: Some(pricing_details.0),
				mint_price: Some(pricing_details.1),
			}));
		});
	}

	#[test]
	fn set_mint_fee_not_collection_owner_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = setup_collection(collection_owner);
			let pricing_details: (AssetId, Balance) = (1, 100);
			let bobby = create_account(11);

			assert_noop!(
				Nft::set_mint_fee(
					RawOrigin::Signed(bobby).into(),
					collection_id,
					Some(pricing_details)
				),
				Error::<Test>::NotCollectionOwner
			);
		});
	}

	#[test]
	fn set_mint_fee_no_collection_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = 1; // No collection
			let pricing_details: (AssetId, Balance) = (1, 100);

			assert_noop!(
				Nft::set_mint_fee(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					Some(pricing_details)
				),
				Error::<Test>::NoCollectionFound
			);
		});
	}
}

mod toggle_public_mint {
	use super::*;
	use seed_pallet_common::utils::PublicMintInformation;

	#[test]
	fn toggle_public_mint_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = setup_collection(collection_owner);
			let enabled = true;

			assert_ok!(Nft::toggle_public_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				enabled
			));

			assert_eq!(PublicMintInfo::<Test>::get(collection_id).unwrap().enabled, enabled);

			// Disable again should work and clear storage
			let enabled = false;
			assert_ok!(Nft::toggle_public_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				enabled
			));

			assert_eq!(PublicMintInfo::<Test>::get(collection_id), None);
		});
	}

	#[test]
	fn toggle_public_mint_emits_event() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = setup_collection(collection_owner);
			let enabled = true;

			assert_ok!(Nft::toggle_public_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				enabled
			));

			assert!(has_event(Event::<Test>::PublicMintToggle { collection_id, enabled }));

			// Disable again should work and still throw event
			let enabled = false;
			assert_ok!(Nft::toggle_public_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				enabled
			));

			assert!(has_event(Event::<Test>::PublicMintToggle { collection_id, enabled }));
		});
	}

	#[test]
	fn toggle_public_mint_should_keep_pricing_details() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = setup_collection(collection_owner);
			let enabled = true;

			// Set up pricing details
			let pricing_details: (AssetId, Balance) = (2, 234);
			assert_ok!(Nft::set_mint_fee(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				Some(pricing_details)
			));

			// Toggle mint should set enabled to true but keep pricing_details in tact
			assert_ok!(Nft::toggle_public_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				enabled
			));

			let expected_mint_info =
				PublicMintInformation { enabled: true, pricing_details: Some(pricing_details) };
			assert_eq!(PublicMintInfo::<Test>::get(collection_id).unwrap(), expected_mint_info);
		});
	}
}

mod public_minting {
	use super::*;
	use crate::mock::AssetsExt;
	use frame_support::traits::fungibles::Inspect;

	#[test]
	fn public_mint_should_let_user_mint() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = setup_collection(collection_owner);
			let minter = create_account(11);
			let quantity = 100;

			// Minter should not be able to mint token
			assert_noop!(
				Nft::mint(Some(minter).into(), collection_id, quantity, None),
				Error::<Test>::PublicMintDisabled
			);

			// Enable public minting
			assert_ok!(Nft::toggle_public_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				true
			));

			// Minter should have no troubles minting now
			assert_ok!(Nft::mint(Some(minter).into(), collection_id, quantity, None));

			// Should emit event
			assert!(has_event(Event::<Test>::Mint {
				collection_id,
				start: 0,
				end: 99,
				owner: minter,
			}));

			// Check that minter has 100 token
			assert_eq!(Nft::token_balance_of(&minter, collection_id), quantity);
		});
	}

	#[test]
	fn public_mint_with_price_should_charge_user() {
		let minter = create_account(11);
		let initial_balance = 100000;
		TestExt::default()
			.with_xrp_balances(&[(minter, initial_balance)])
			.build()
			.execute_with(|| {
				let collection_owner = create_account(10);
				let collection_id = setup_collection(collection_owner);
				let quantity = 100;
				let mint_price = 25;
				let payment_asset = XRP_ASSET_ID;

				// Set up pricing details
				let pricing_details: (AssetId, Balance) = (payment_asset, mint_price);
				assert_ok!(Nft::set_mint_fee(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					Some(pricing_details)
				));

				// Enable public minting
				assert_ok!(Nft::toggle_public_mint(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					true
				));

				// Minter should be able to mint
				assert_ok!(Nft::mint(Some(minter).into(), collection_id, quantity, None));
				// Check that minter has 100 token
				assert_eq!(Nft::token_balance_of(&minter, collection_id), quantity);

				// Should emit both mint and payment event
				assert!(has_event(Event::<Test>::Mint {
					collection_id,
					start: 0,
					end: 99,
					owner: minter,
				}));

				let payment_amount: Balance = mint_price * quantity as u128;
				assert!(has_event(Event::<Test>::MintFeePaid {
					who: minter,
					collection_id,
					payment_asset,
					payment_amount,
					token_count: quantity,
				}));

				// Check minter was charged the correct amount
				let minter_balance = AssetsExt::reducible_balance(payment_asset, &minter, false);
				assert_eq!(minter_balance, initial_balance - payment_amount);
			});
	}

	#[test]
	fn public_mint_insufficient_balance_should_fail() {
		let minter = create_account(11);
		let initial_balance = 99; // Not enough
		TestExt::default()
			.with_xrp_balances(&[(minter, initial_balance)])
			.build()
			.execute_with(|| {
				let collection_owner = create_account(10);
				let collection_id = setup_collection(collection_owner);
				let quantity = 1;
				let mint_price = 100;
				let payment_asset = XRP_ASSET_ID;

				// Set up pricing details
				let pricing_details: (AssetId, Balance) = (payment_asset, mint_price);
				assert_ok!(Nft::set_mint_fee(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					Some(pricing_details)
				));

				// Enable public minting
				assert_ok!(Nft::toggle_public_mint(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					true
				));

				// Minter doesn't have enough XRP to cover mint
				assert_noop!(
					Nft::mint(Some(minter).into(), collection_id, quantity, None),
					pallet_assets::Error::<Test>::BalanceLow
				);
			});
	}

	#[test]
	fn public_mint_collection_owner_should_not_be_charged() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = setup_collection(collection_owner);
			let quantity = 1;
			let mint_price = 100000000;
			let payment_asset = XRP_ASSET_ID;
			let owner_balance_before =
				AssetsExt::reducible_balance(payment_asset, &collection_owner, false);

			// Set up pricing details
			let pricing_details: (AssetId, Balance) = (payment_asset, mint_price);
			assert_ok!(Nft::set_mint_fee(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				Some(pricing_details)
			));

			// Enable public minting
			assert_ok!(Nft::toggle_public_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				true
			));

			// Collection owner mints
			assert_ok!(Nft::mint(Some(collection_owner).into(), collection_id, quantity, None));
			// Check that minter has 100 token
			assert_eq!(Nft::token_balance_of(&collection_owner, collection_id), quantity);

			let owner_balance_after =
				AssetsExt::reducible_balance(payment_asset, &collection_owner, false);

			// Should not have been charged
			assert_eq!(owner_balance_before, owner_balance_after);
		});
	}

	#[test]
	fn public_mint_token_owner_not_charged() {
		// Title is confusing, but basically this test checks that if a token owner is specified,
		// the caller is charged, not the specified owner
		let minter = create_account(11);
		let initial_balance = 1000;
		TestExt::default()
			.with_xrp_balances(&[(minter, initial_balance)])
			.build()
			.execute_with(|| {
				let collection_owner = create_account(10);
				let token_owner = create_account(12);
				let collection_id = setup_collection(collection_owner);
				let quantity = 3;
				let mint_price = 200;
				let payment_asset = XRP_ASSET_ID;

				let token_owner_balance_before =
					AssetsExt::reducible_balance(payment_asset, &token_owner, false);

				// Set up pricing details
				let pricing_details: (AssetId, Balance) = (payment_asset, mint_price);
				assert_ok!(Nft::set_mint_fee(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					Some(pricing_details)
				));
				// Enable public minting
				assert_ok!(Nft::toggle_public_mint(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					true
				));

				// Minter should be able to mint
				assert_ok!(Nft::mint(
					Some(minter).into(),
					collection_id,
					quantity,
					Some(token_owner)
				));

				// Check that token_owner has tokens, but minter has none
				assert_eq!(Nft::token_balance_of(&token_owner, collection_id), quantity);
				assert_eq!(Nft::token_balance_of(&minter, collection_id), 0);

				// Should emit both mint and payment event
				assert!(has_event(Event::<Test>::Mint {
					collection_id,
					start: 0,
					end: 2,
					owner: token_owner,
				}));
				let payment_amount: Balance = mint_price * quantity as u128;
				assert!(has_event(Event::<Test>::MintFeePaid {
					who: minter,
					collection_id,
					payment_asset,
					payment_amount,
					token_count: quantity,
				}));

				// Check minter was charged the correct amount
				let minter_balance = AssetsExt::reducible_balance(payment_asset, &minter, false);
				assert_eq!(minter_balance, initial_balance - payment_amount);

				// Token owner should not have been charged
				let token_owner_balance_after =
					AssetsExt::reducible_balance(payment_asset, &token_owner, false);
				assert_eq!(token_owner_balance_before, token_owner_balance_after);
			});
	}
}
