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
		create_account, has_event, AssetsExt, FeePotId, MarketplaceNetworkFeePercentage,
		MaxTokensPerCollection, NativeAssetId, Nft, NftPalletId, RuntimeEvent as MockEvent, System,
		Test, TestExt, XRP_ASSET_ID,
	},
	Event as NftEvent,
};
use core::ops::Mul;
use frame_support::{
	assert_noop, assert_ok,
	traits::{fungibles::Inspect, OnInitialize},
};
use frame_system::RawOrigin;
use seed_primitives::{
	AccountId, MetadataScheme, OriginChain, RoyaltiesSchedule, TokenCount, TokenId,
};
use sp_core::H160;
use sp_runtime::{BoundedVec, DispatchError::BadOrigin, Permill};

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

/// Setup a token, return collection id, token id, token owner
fn setup_token() -> (CollectionUuid, TokenId, AccountId) {
	let collection_owner = create_account(1);
	let collection_id = setup_collection(collection_owner);
	let token_owner = create_account(2);
	let token_id = (collection_id, 0);
	assert_ok!(Nft::mint(Some(collection_owner).into(), collection_id, 1, Some(token_owner)));

	(collection_id, token_id, token_owner)
}

/// Setup a token, return collection id, token id, token owner
fn setup_token_with_royalties(
	royalties_schedule: RoyaltiesSchedule<AccountId>,
	quantity: TokenCount,
) -> (CollectionUuid, TokenId, AccountId) {
	let collection_owner = create_account(1);
	let collection_id = Nft::next_collection_uuid().unwrap();
	let collection_name = bounded_string("test-collection");
	let metadata_scheme = MetadataScheme::try_from(b"<CID>".as_slice()).unwrap();
	assert_ok!(Nft::create_collection(
		Some(collection_owner).into(),
		collection_name,
		0,
		None,
		None,
		metadata_scheme,
		Some(royalties_schedule),
		CrossChainCompatibility::default(),
	));

	let token_owner = create_account(2);
	let token_id = (collection_id, 0);
	assert_ok!(Nft::mint(
		Some(collection_owner).into(),
		collection_id,
		quantity,
		Some(token_owner),
	));

	(collection_id, token_id, token_owner)
}

/// Create an offer on a token. Return offer_id, offer
fn make_new_simple_offer(
	offer_amount: Balance,
	token_id: TokenId,
	buyer: AccountId,
	marketplace_id: Option<MarketplaceId>,
) -> (OfferId, SimpleOffer<AccountId>) {
	let next_offer_id = Nft::next_offer_id();

	assert_ok!(Nft::make_simple_offer(
		Some(buyer).into(),
		token_id,
		offer_amount,
		NativeAssetId::get(),
		marketplace_id
	));
	let offer = SimpleOffer {
		token_id,
		asset_id: NativeAssetId::get(),
		amount: offer_amount,
		buyer,
		marketplace_id,
	};

	// Check storage has been updated
	assert_eq!(Nft::next_offer_id(), next_offer_id + 1);
	assert_eq!(Nft::offers(next_offer_id), Some(OfferType::Simple(offer.clone())));
	assert!(has_event(Event::<Test>::Offer {
		offer_id: next_offer_id,
		amount: offer_amount,
		asset_id: NativeAssetId::get(),
		marketplace_id,
		buyer,
	}));

	(next_offer_id, offer)
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

		assert_eq!(Nft::collection_info(collection_id).unwrap(), expected_info);

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
		assert_eq!(Nft::collection_info(collection_id).unwrap().collection_issuance, quantity);
		assert_eq!(
			Nft::collection_info(collection_id).unwrap().royalties_schedule,
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
			Nft::collection_info(collection_id).unwrap().next_serial_number,
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
			Nft::collection_info(collection_id).unwrap().collection_issuance,
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
			serial_numbers: serial_numbers.into_inner()
		}));

		assert_eq!(Nft::token_balance_of(&token_owner, collection_id), 0);
		assert_eq!(Nft::token_balance_of(&new_owner, collection_id), 1);
		assert!(Nft::collection_info(collection_id).unwrap().is_token_owner(&new_owner, 0));
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

		assert_ok!(Nft::sell(
			Some(token_owner).into(),
			collection_id,
			serial_numbers.clone(),
			Some(create_account(5)),
			NativeAssetId::get(),
			1_000,
			None,
			None,
		));

		// cannot transfer while listed
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

		assert_eq!(Nft::collection_info(collection_id).unwrap().collection_issuance, 0);
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

		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![0]).unwrap();
		assert_ok!(Nft::sell(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			None,
			NativeAssetId::get(),
			1_000,
			None,
			None,
		));
		// cannot burn while listed
		assert_noop!(
			Nft::burn(Some(token_owner).into(), (collection_id, 0)),
			Error::<Test>::TokenLocked,
		);
	});
}

#[test]
fn sell() {
	let buyer = create_account(3);
	let initial_balance = 11_111_225;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance)])
		.build()
		.execute_with(|| {
			let collection_owner = create_account(1);
			let quantity = 5;
			let collection_id = Nft::next_collection_uuid().unwrap();

			assert_ok!(Nft::create_collection(
				Some(collection_owner).into(),
				bounded_string("test-collection"),
				quantity,
				None,
				None,
				MetadataScheme::try_from(b"https://example.com/metadata".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			));

			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![1, 3, 4]).unwrap();
			let listing_id = Nft::next_listing_id();

			assert_ok!(Nft::sell(
				Some(collection_owner).into(),
				collection_id,
				serial_numbers.clone(),
				None,
				NativeAssetId::get(),
				1_000,
				None,
				None,
			));

			for serial_number in serial_numbers.iter() {
				assert_eq!(
					Nft::token_locks((collection_id, serial_number)).unwrap(),
					TokenLockReason::Listed(listing_id)
				);
			}

			assert_ok!(Nft::buy(Some(buyer).into(), listing_id));
			assert_eq!(
				Nft::owned_tokens(collection_id, &buyer, 0, 1000),
				(0_u32, 3, vec![1, 3, 4])
			);
			assert_eq!(Nft::token_balance_of(&collection_owner, collection_id), 2);
			assert_eq!(
				Nft::token_balance_of(&buyer, collection_id),
				serial_numbers.len() as TokenCount
			);

			let fee_pot_account: AccountId = FeePotId::get().into_account_truncating();

			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &fee_pot_account, false),
				5, // 0.5% of 1000
			);
		})
}

#[test]
fn sell_with_empty_royalties() {
	let buyer = create_account(3);
	let initial_balance = 11_111_225;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance)])
		.build()
		.execute_with(|| {
			let collection_owner = create_account(1);
			let quantity = 5;
			let collection_id = Nft::next_collection_uuid().unwrap();

			assert_ok!(Nft::create_collection(
				Some(collection_owner).into(),
				bounded_string("test-collection"),
				quantity,
				None,
				None,
				MetadataScheme::try_from(b"https://example.com/metadata".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			));

			// Remove fee to account so there are no royalties on listed token
			// i.e. 100% of sale price goes to seller
			assert_ok!(Nft::set_fee_to(RawOrigin::Root.into(), None));

			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![1, 3, 4]).unwrap();

			assert_ok!(Nft::sell(
				Some(collection_owner).into(),
				collection_id,
				serial_numbers.clone(),
				None,
				NativeAssetId::get(),
				1_000,
				None,
				None,
			));
		})
}

#[test]
fn sell_multiple_fails() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let collection_id = setup_collection(collection_owner);
		// mint some tokens
		assert_ok!(Nft::mint(Some(collection_owner).into(), collection_id, 2, None));
		assert!(has_event(Event::<Test>::Mint {
			collection_id,
			start: 0,
			end: 1,
			owner: collection_owner,
		}));

		// empty tokens fails
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![]).unwrap();
		assert_noop!(
			Nft::sell(
				Some(collection_owner).into(),
				collection_id,
				serial_numbers,
				None,
				NativeAssetId::get(),
				1_000,
				None,
				None
			),
			Error::<Test>::NoToken
		);
	})
}

#[test]
fn sell_multiple() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();
		let listing_id = Nft::next_listing_id();

		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		let buyer = create_account(5);
		assert_ok!(Nft::sell(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			Some(buyer),
			NativeAssetId::get(),
			1_000,
			None,
			None,
		));
		assert!(has_event(Event::<Test>::FixedPriceSaleList {
			collection_id,
			serial_numbers: vec![token_id.1],
			listing_id,
			marketplace_id: None,
			price: 1_000,
			payment_asset: NativeAssetId::get(),
			seller: token_owner,
		}));

		assert_eq!(Nft::token_locks(token_id).unwrap(), TokenLockReason::Listed(listing_id));
		assert!(Nft::open_collection_listings(collection_id, listing_id).unwrap());

		let fee_pot_account: AccountId = FeePotId::get().into_account_truncating();
		let royalties_schedule = RoyaltiesSchedule {
			entitlements: BoundedVec::truncate_from(vec![(
				fee_pot_account,
				MarketplaceNetworkFeePercentage::get(),
			)]),
		};
		let expected = Listing::<Test>::FixedPrice(FixedPriceListing::<Test> {
			payment_asset: NativeAssetId::get(),
			fixed_price: 1_000,
			close: System::block_number() + <Test as Config>::DefaultListingDuration::get(),
			buyer: Some(buyer),
			collection_id,
			serial_numbers: BoundedVec::try_from(vec![token_id.1]).unwrap(),
			seller: token_owner,
			royalties_schedule,
			marketplace_id: None,
		});

		let listing = Listings::<Test>::get(listing_id).expect("token is listed");
		assert_eq!(listing, expected);

		// current block is 1 + duration
		assert!(Nft::listing_end_schedule(
			System::block_number() + <Test as Config>::DefaultListingDuration::get(),
			listing_id
		)
		.unwrap());

		// Can't transfer while listed for sale
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_noop!(
			Nft::transfer(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				create_account(9)
			),
			Error::<Test>::TokenLocked
		);
	});
}

#[test]
fn sell_fails() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();
		// Not token owner
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		let bob = create_account(9);
		let buyer = create_account(5);
		assert_noop!(
			Nft::sell(
				Some(bob).into(),
				collection_id,
				serial_numbers,
				Some(buyer),
				NativeAssetId::get(),
				1_000,
				None,
				None
			),
			Error::<Test>::NotTokenOwner
		);

		// token listed already
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_ok!(Nft::sell(
			Some(token_owner).into(),
			collection_id,
			serial_numbers.clone(),
			Some(buyer),
			NativeAssetId::get(),
			1_000,
			None,
			None,
		));
		assert_noop!(
			Nft::sell(
				Some(token_owner).into(),
				collection_id,
				serial_numbers.clone(),
				Some(buyer),
				NativeAssetId::get(),
				1_000,
				None,
				None
			),
			Error::<Test>::TokenLocked
		);

		// can't auction, listed for fixed price sale
		assert_noop!(
			Nft::auction(
				Some(token_owner).into(),
				collection_id,
				serial_numbers.clone(),
				NativeAssetId::get(),
				1_000,
				None,
				None
			),
			Error::<Test>::TokenLocked
		);
	});
}

#[test]
fn cancel_sell() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();
		let listing_id = Nft::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		let buyer = create_account(5);
		assert_ok!(Nft::sell(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			Some(buyer),
			NativeAssetId::get(),
			1_000,
			None,
			None
		));
		assert_ok!(Nft::cancel_sale(Some(token_owner).into(), listing_id));
		assert!(has_event(Event::<Test>::FixedPriceSaleClose {
			collection_id,
			serial_numbers: vec![token_id.1],
			listing_id,
			reason: FixedPriceClosureReason::VendorCancelled
		}));

		// storage cleared up
		assert!(Listings::<Test>::get(listing_id).is_none());
		assert!(Nft::listing_end_schedule(
			System::block_number() + <Test as Config>::DefaultListingDuration::get(),
			listing_id
		)
		.is_none());

		// it should be free to operate on the token
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		let new_owner = create_account(6);
		assert_ok!(Nft::transfer(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			new_owner,
		));
	});
}

#[test]
fn sell_closes_on_schedule() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();
		let listing_duration = 100;
		let listing_id = Nft::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		let buyer = create_account(5);
		assert_ok!(Nft::sell(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			Some(buyer),
			NativeAssetId::get(),
			1_000,
			Some(listing_duration),
			None
		));

		// sale should close after the duration expires
		Nft::on_initialize(System::block_number() + listing_duration);

		// seller should have tokens
		assert!(Listings::<Test>::get(listing_id).is_none());
		assert!(Nft::listing_end_schedule(System::block_number() + listing_duration, listing_id)
			.is_none());

		// should be free to transfer now
		let new_owner = create_account(8);
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_ok!(Nft::transfer(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			new_owner,
		));
	});
}

#[test]
fn updates_fixed_price() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();
		let listing_id = Nft::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		let buyer = create_account(5);
		assert_ok!(Nft::sell(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			Some(buyer),
			NativeAssetId::get(),
			1_000,
			None,
			None
		));
		assert_ok!(Nft::update_fixed_price(Some(token_owner).into(), listing_id, 1_500));
		assert!(has_event(Event::<Test>::FixedPriceSalePriceUpdate {
			collection_id,
			serial_numbers: vec![token_id.1],
			listing_id,
			new_price: 1_500,
		}));

		let fee_pot_account: AccountId = FeePotId::get().into_account_truncating();
		let royalties_schedule = RoyaltiesSchedule {
			entitlements: BoundedVec::truncate_from(vec![(
				fee_pot_account,
				MarketplaceNetworkFeePercentage::get(),
			)]),
		};
		let expected = Listing::<Test>::FixedPrice(FixedPriceListing::<Test> {
			payment_asset: NativeAssetId::get(),
			fixed_price: 1_500,
			close: System::block_number() + <Test as Config>::DefaultListingDuration::get(),
			buyer: Some(buyer),
			seller: token_owner,
			collection_id,
			serial_numbers: BoundedVec::try_from(vec![token_id.1]).unwrap(),
			royalties_schedule,
			marketplace_id: None,
		});

		let listing = Listings::<Test>::get(listing_id).expect("token is listed");
		assert_eq!(listing, expected);
	});
}

#[test]
fn update_fixed_price_fails() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();

		let reserve_price = 1_000;
		let listing_id = Nft::next_listing_id();

		// can't update, token not listed
		assert_noop!(
			Nft::update_fixed_price(Some(token_owner).into(), listing_id, 1_500),
			Error::<Test>::NotForFixedPriceSale
		);
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_ok!(Nft::auction(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			NativeAssetId::get(),
			reserve_price,
			Some(System::block_number() + 1),
			None,
		));

		// can't update, listed for auction
		assert_noop!(
			Nft::update_fixed_price(Some(token_owner).into(), listing_id, 1_500),
			Error::<Test>::NotForFixedPriceSale
		);
	});
}

#[test]
fn update_fixed_price_fails_not_owner() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();
		let listing_id = Nft::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		let buyer = create_account(5);
		assert_ok!(Nft::sell(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			Some(buyer),
			NativeAssetId::get(),
			1_000,
			None,
			None
		));

		assert_noop!(
			Nft::update_fixed_price(Some(buyer).into(), listing_id, 1_500),
			Error::<Test>::NotSeller
		);
	});
}

#[test]
fn register_marketplace() {
	TestExt::default().build().execute_with(|| {
		let account = create_account(1);
		let entitlement: Permill = Permill::from_float(0.1);
		let marketplace_id = Nft::next_marketplace_id();
		assert_ok!(Nft::register_marketplace(Some(account).into(), None, entitlement));
		assert!(has_event(Event::<Test>::MarketplaceRegister {
			account,
			entitlement,
			marketplace_id
		}));
		assert_eq!(Nft::next_marketplace_id(), marketplace_id + 1);
	});
}

#[test]
fn register_marketplace_separate_account() {
	TestExt::default().build().execute_with(|| {
		let account = create_account(1);
		let marketplace_account = create_account(2);
		let marketplace_id = Nft::next_marketplace_id();
		let entitlement: Permill = Permill::from_float(0.1);

		assert_ok!(Nft::register_marketplace(
			Some(account).into(),
			Some(marketplace_account).into(),
			entitlement
		));
		assert!(has_event(Event::<Test>::MarketplaceRegister {
			account: marketplace_account,
			entitlement,
			marketplace_id
		}));
	});
}

#[test]
fn buy_with_marketplace_royalties() {
	let buyer = create_account(5);
	let sale_price = 1_000_008;

	TestExt::default()
		.with_balances(&[(buyer, sale_price * 2)])
		.build()
		.execute_with(|| {
			let collection_owner = create_account(1);
			let beneficiary_1 = create_account(11);
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(
					beneficiary_1,
					Permill::from_float(0.1111),
				)]),
			};
			let (collection_id, _, token_owner) =
				setup_token_with_royalties(royalties_schedule.clone(), 2);

			let token_id = (collection_id, 0);

			let marketplace_account = create_account(20);
			let initial_balance_marketplace =
				AssetsExt::reducible_balance(NativeAssetId::get(), &marketplace_account, false);
			let marketplace_entitlement: Permill = Permill::from_float(0.5);
			assert_ok!(Nft::register_marketplace(
				Some(marketplace_account).into(),
				Some(marketplace_account).into(),
				marketplace_entitlement
			));
			let marketplace_id = 0;
			let listing_id = Nft::next_listing_id();
			assert_eq!(listing_id, 0);
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Nft::sell(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				Some(buyer),
				NativeAssetId::get(),
				sale_price,
				None,
				Some(marketplace_id).into(),
			));

			let initial_balance_owner =
				AssetsExt::reducible_balance(NativeAssetId::get(), &collection_owner, false);
			let initial_balance_b1 =
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_1, false);

			assert_ok!(Nft::buy(Some(buyer).into(), listing_id));
			let presale_issuance = AssetsExt::total_issuance(NativeAssetId::get());
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &marketplace_account, false),
				initial_balance_marketplace + marketplace_entitlement * sale_price
			);
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_1, false),
				initial_balance_b1 + royalties_schedule.clone().entitlements[0].1 * sale_price
			);
			// token owner gets:
			// sale_price - (marketplace_royalties + beneficiary_royalties + network_fee)
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				initial_balance_owner + sale_price -
					marketplace_entitlement * sale_price -
					royalties_schedule.clone().entitlements[0].1 * sale_price -
					MarketplaceNetworkFeePercentage::get().mul(sale_price)
			);
			assert_eq!(AssetsExt::total_issuance(NativeAssetId::get()), presale_issuance);
		});
}

#[test]
fn list_with_invalid_marketplace_royalties_should_fail() {
	let buyer = create_account(5);
	let sale_price = 1_000_008;

	TestExt::default()
		.with_balances(&[(buyer, sale_price * 2)])
		.build()
		.execute_with(|| {
			let beneficiary_1 = create_account(11);
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(
					beneficiary_1,
					Permill::from_float(0.51),
				)]),
			};
			let (collection_id, _, token_owner) =
				setup_token_with_royalties(royalties_schedule.clone(), 2);

			let token_id = (collection_id, 0);

			let marketplace_account = create_account(20);
			let marketplace_entitlement: Permill = Permill::from_float(0.5);
			assert_ok!(Nft::register_marketplace(
				Some(marketplace_account).into(),
				Some(marketplace_account).into(),
				marketplace_entitlement
			));
			let marketplace_id = 0;
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_noop!(
				Nft::sell(
					Some(token_owner).into(),
					collection_id,
					serial_numbers,
					Some(buyer),
					NativeAssetId::get(),
					sale_price,
					None,
					Some(marketplace_id).into(),
				),
				Error::<Test>::RoyaltiesInvalid,
			);
		});
}

#[test]
fn buy() {
	let buyer = create_account(5);
	let price = 1_000;
	let starting_balance = price * 2;

	TestExt::default()
		.with_balances(&[(buyer, starting_balance)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_token();
			let buyer = create_account(5);

			let listing_id = Nft::next_listing_id();
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Nft::sell(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				Some(buyer),
				NativeAssetId::get(),
				price,
				None,
				None
			));

			assert_ok!(Nft::buy(Some(buyer).into(), listing_id));
			// no royalties, all proceeds to token owner minus network fee
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				price - MarketplaceNetworkFeePercentage::get().mul(price)
			);
			// Buyer balance should be starting minus price (1000)
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false),
				starting_balance - price
			);

			// listing removed
			assert!(Listings::<Test>::get(listing_id).is_none());
			assert!(Nft::listing_end_schedule(
				System::block_number() + <Test as Config>::DefaultListingDuration::get(),
				listing_id
			)
			.is_none());

			// ownership changed
			assert!(Nft::token_locks(&token_id).is_none());
			assert!(Nft::open_collection_listings(collection_id, listing_id).is_none());
			assert_eq!(
				Nft::owned_tokens(collection_id, &buyer, 0, 1000),
				(0_u32, 1, vec![token_id.1])
			);

			// assert network fees accumulated
			let fee_pot_account: AccountId = FeePotId::get().into_account_truncating();

			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &fee_pot_account, false),
				5, // 0.5% of 1000
			);
		});
}

#[test]
fn listing_price_splits_royalties_and_network_fee() {
	let buyer = create_account(5);
	let price = 1_000_000;
	let starting_balance = price * 2;
	let entitlement_amount = Permill::from_float(0.25);

	TestExt::default()
		.with_balances(&[(buyer, starting_balance)])
		.build()
		.execute_with(|| {
			let beneficiary_1 = create_account(11);

			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(beneficiary_1, entitlement_amount)]),
			};
			let (collection_id, token_id, token_owner) =
				setup_token_with_royalties(royalties_schedule.clone(), 2);

			let listing_id = Nft::next_listing_id();
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Nft::sell(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				Some(buyer),
				NativeAssetId::get(),
				price,
				None,
				None
			));

			assert_ok!(Nft::buy(Some(buyer).into(), listing_id));

			// Buyer balance should be starting minus 1_000_000
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false),
				starting_balance - price
			);

			// Owner balance should be 1_000_000 minus 25.5% of 1_000_000
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				price -
					entitlement_amount.mul(price) -
					MarketplaceNetworkFeePercentage::get().mul(price)
			);

			// Beneficiary balance should be 25% of 1_000_000
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_1, false),
				entitlement_amount.mul(price)
			);

			let fee_pot_account: AccountId = FeePotId::get().into_account_truncating();
			// Network fee should be 0.5% of 1_000_000
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &fee_pot_account, false),
				MarketplaceNetworkFeePercentage::get().mul(price)
			);
		});
}

#[test]
fn listing_price_splits_multiple_royalties_and_network_fee() {
	let buyer = create_account(5);
	let price = 1_000_000;
	let starting_balance = price * 2;
	let entitlement_amount = Permill::from_float(0.25);
	let entitlement_amount_beneficiary_2 = Permill::from_float(0.5);

	TestExt::default()
		.with_balances(&[(buyer, starting_balance)])
		.build()
		.execute_with(|| {
			let beneficiary_1 = create_account(11);
			let beneficiary_2 = create_account(22);

			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![
					(beneficiary_1, entitlement_amount),
					(beneficiary_2, entitlement_amount_beneficiary_2),
				]),
			};
			let (collection_id, token_id, token_owner) =
				setup_token_with_royalties(royalties_schedule.clone(), 2);

			let listing_id = Nft::next_listing_id();
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Nft::sell(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				Some(buyer),
				NativeAssetId::get(),
				price,
				None,
				None
			));

			assert_ok!(Nft::buy(Some(buyer).into(), listing_id));

			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false),
				starting_balance - price
			);

			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				price -
					(entitlement_amount.mul(price) +
						entitlement_amount_beneficiary_2.mul(price) +
						MarketplaceNetworkFeePercentage::get().mul(price))
			);

			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_1, false),
				entitlement_amount.mul(price)
			);

			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_2, false),
				entitlement_amount_beneficiary_2.mul(price)
			);

			let fee_pot_account: AccountId = FeePotId::get().into_account_truncating();

			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &fee_pot_account, false),
				MarketplaceNetworkFeePercentage::get().mul(price)
			);
		});
}

#[test]
fn network_fee_royalties_split_is_respected_xrpl() {
	let buyer = create_account(5);
	let price = 1_000_000;
	let starting_balance = price * 2;
	let entitlement_amount = Permill::from_float(0.25);
	let asset_used = XRP_ASSET_ID;

	TestExt::default()
		.with_xrp_balances(&[(buyer, starting_balance)])
		.build()
		.execute_with(|| {
			let beneficiary_1 = create_account(11);

			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(beneficiary_1, entitlement_amount)]),
			};
			let (collection_id, token_id, token_owner) =
				setup_token_with_royalties(royalties_schedule.clone(), 2);

			let listing_id = Nft::next_listing_id();
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Nft::sell(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				Some(buyer),
				asset_used,
				price,
				None,
				None
			));

			assert_ok!(Nft::buy(Some(buyer).into(), listing_id));
			assert_eq!(
				AssetsExt::reducible_balance(asset_used, &token_owner, false),
				price -
					entitlement_amount.mul(price) -
					MarketplaceNetworkFeePercentage::get().mul(price)
			);

			assert_eq!(
				AssetsExt::reducible_balance(asset_used, &beneficiary_1, false),
				entitlement_amount.mul(price)
			);

			let fee_pot_account: AccountId = FeePotId::get().into_account_truncating();

			assert_eq!(
				AssetsExt::reducible_balance(asset_used, &fee_pot_account, false),
				MarketplaceNetworkFeePercentage::get().mul(price)
			);
		});
}

#[test]
fn buy_with_royalties() {
	let buyer = create_account(5);
	let sale_price = 1_000_008;

	TestExt::default()
		.with_balances(&[(buyer, sale_price * 2)])
		.build()
		.execute_with(|| {
			let collection_owner = create_account(1);
			let beneficiary_1 = create_account(11);
			let beneficiary_2 = create_account(12);
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![
					(collection_owner, Permill::from_float(0.111)),
					(beneficiary_1, Permill::from_float(0.1111)),
					(beneficiary_2, Permill::from_float(0.3333)),
				]),
			};
			let (collection_id, token_id, token_owner) =
				setup_token_with_royalties(royalties_schedule.clone(), 2);

			let listing_id = Nft::next_listing_id();
			assert_eq!(listing_id, 0);
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Nft::sell(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				Some(buyer),
				NativeAssetId::get(),
				sale_price,
				None,
				None
			));

			let initial_balance_owner =
				AssetsExt::reducible_balance(NativeAssetId::get(), &collection_owner, false);
			let initial_balance_b1 =
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_1, false);
			let initial_balance_b2 =
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_2, false);
			let initial_balance_seller =
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false);

			assert_ok!(Nft::buy(Some(buyer).into(), listing_id));
			let presale_issuance = AssetsExt::total_issuance(NativeAssetId::get());
			// royalties distributed according to `entitlements` map
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &collection_owner, false),
				initial_balance_owner + royalties_schedule.clone().entitlements[0].1 * sale_price
			);
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_1, false),
				initial_balance_b1 + royalties_schedule.clone().entitlements[1].1 * sale_price
			);
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_2, false),
				initial_balance_b2 + royalties_schedule.clone().entitlements[2].1 * sale_price
			);
			// token owner gets sale price - royalties - network fee
			let network_fee = MarketplaceNetworkFeePercentage::get().mul(sale_price);
			let royalties = royalties_schedule
				.clone()
				.entitlements
				.into_iter()
				.map(|(_, e)| e * sale_price)
				.sum::<Balance>();
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				initial_balance_seller + sale_price - royalties - network_fee
			);
			assert_eq!(AssetsExt::total_issuance(NativeAssetId::get()), presale_issuance);

			// listing removed
			assert!(Listings::<Test>::get(listing_id).is_none());
			assert!(Nft::listing_end_schedule(
				System::block_number() + <Test as Config>::DefaultListingDuration::get(),
				listing_id
			)
			.is_none());

			// ownership changed
			assert_eq!(
				Nft::owned_tokens(collection_id, &buyer, 0, 1000),
				(0_u32, 1, vec![token_id.1])
			);
		});
}

#[test]
fn buy_fails_prechecks() {
	let buyer = create_account(5);
	let price = 1_000;
	TestExt::default()
		.with_balances(&[(buyer, price - 1)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_token();
			let buyer = create_account(5);

			let price = 1_000;
			let listing_id = Nft::next_listing_id();

			// not for sale
			assert_noop!(
				Nft::buy(Some(buyer).into(), listing_id),
				Error::<Test>::NotForFixedPriceSale,
			);
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Nft::sell(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				Some(buyer),
				NativeAssetId::get(),
				price,
				None,
				None
			));

			// no permission
			let not_buyer = create_account(6);
			assert_noop!(Nft::buy(Some(not_buyer).into(), listing_id), Error::<Test>::NotBuyer,);

			assert_noop!(
				Nft::buy(Some(buyer).into(), listing_id),
				pallet_assets_ext::Error::<Test>::BalanceLow,
			);
		});
}

#[test]
fn sell_to_anybody() {
	let buyer = create_account(5);
	let price = 1_000;
	TestExt::default()
		.with_balances(&[(buyer, price + 995)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_token();

			let price = 1_000;
			let listing_id = Nft::next_listing_id();
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Nft::sell(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				None,
				NativeAssetId::get(),
				price,
				None,
				None
			));

			assert_ok!(Nft::buy(Some(buyer).into(), listing_id));

			// paid
			assert_eq!(AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false), 995);

			// listing removed
			assert!(Listings::<Test>::get(listing_id).is_none());
			assert!(Nft::listing_end_schedule(
				System::block_number() + <Test as Config>::DefaultListingDuration::get(),
				listing_id
			)
			.is_none());

			// ownership changed
			assert_eq!(
				Nft::owned_tokens(collection_id, &buyer, 0, 1000),
				(0_u32, 1, vec![token_id.1])
			);
		});
}

#[test]
fn buy_with_overcommitted_royalties() {
	let buyer = create_account(5);
	let price = 1_000;
	TestExt::default().with_balances(&[(buyer, 1995)]).build().execute_with(|| {
		// royalties are > 100% total which could create funds out of nothing
		// in this case, default to 0 royalties.
		// royalty schedules should not make it into storage but we protect against it anyway
		let (collection_id, token_id, token_owner) = setup_token();
		let bad_schedule = RoyaltiesSchedule {
			entitlements: BoundedVec::truncate_from(vec![
				(11_u64, Permill::from_float(0.125)),
				(12_u64, Permill::from_float(0.9)),
			]),
		};
		let listing_id = Nft::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_ok!(Nft::sell(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			Some(buyer),
			NativeAssetId::get(),
			price,
			None,
			None
		));

		let presale_issuance = AssetsExt::total_issuance(NativeAssetId::get());

		assert_ok!(Nft::buy(Some(buyer).into(), listing_id));
		assert!(bad_schedule.calculate_total_entitlement().is_zero());
		assert_eq!(
			AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
			price - MarketplaceNetworkFeePercentage::get().mul(price)
		);
		assert_eq!(AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false), 995);
		assert_eq!(AssetsExt::total_issuance(NativeAssetId::get()), presale_issuance);
	})
}

#[test]
fn cancel_auction() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();

		let reserve_price = 100_000;
		let listing_id = Nft::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_ok!(Nft::auction(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			NativeAssetId::get(),
			reserve_price,
			Some(System::block_number() + 1),
			None,
		));

		let new_owner = create_account(6);
		assert_noop!(
			Nft::cancel_sale(Some(new_owner).into(), listing_id),
			Error::<Test>::NotSeller
		);

		assert_ok!(Nft::cancel_sale(Some(token_owner).into(), listing_id,));

		assert!(has_event(Event::<Test>::AuctionClose {
			collection_id,
			listing_id,
			reason: AuctionClosureReason::VendorCancelled
		}));

		// storage cleared up
		assert!(Listings::<Test>::get(listing_id).is_none());
		assert!(Nft::listing_end_schedule(System::block_number() + 1, listing_id).is_none());

		// it should be free to operate on the token
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_ok!(Nft::transfer(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			new_owner,
		));
	});
}

#[test]
fn auction_bundle() {
	let buyer = create_account(5);
	let price = 1_000;
	TestExt::default().with_balances(&[(buyer, price)]).build().execute_with(|| {
		let collection_owner = create_account(1);
		let collection_id = Nft::next_collection_uuid().unwrap();
		let quantity = 5;

		assert_ok!(Nft::create_collection(
			Some(collection_owner).into(),
			bounded_string("test-collection"),
			quantity,
			None,
			None,
			MetadataScheme::try_from(b"https://example.com/metadata".as_slice()).unwrap(),
			None,
			CrossChainCompatibility::default(),
		));
		assert_eq!(Nft::token_balance_of(&collection_owner, collection_id), 5);

		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![1, 3, 4]).unwrap();
		let listing_id = Nft::next_listing_id();

		assert_ok!(Nft::auction(
			Some(collection_owner).into(),
			collection_id,
			serial_numbers.clone(),
			NativeAssetId::get(),
			price,
			Some(1),
			None,
		));

		assert!(Nft::open_collection_listings(collection_id, listing_id).unwrap());
		for serial_number in serial_numbers.iter() {
			assert_eq!(
				Nft::token_locks((collection_id, serial_number)).unwrap(),
				TokenLockReason::Listed(listing_id)
			);
		}

		assert_ok!(Nft::bid(Some(buyer).into(), listing_id, price));
		// end auction
		let _ = Nft::on_initialize(System::block_number() + AUCTION_EXTENSION_PERIOD as u64);

		assert_eq!(Nft::owned_tokens(collection_id, &buyer, 0, 1000), (0_u32, 3, vec![1, 3, 4]));
		assert_eq!(Nft::token_balance_of(&collection_owner, collection_id), 2);
		assert_eq!(
			Nft::token_balance_of(&buyer, collection_id),
			serial_numbers.len() as TokenCount
		);
	})
}

#[test]
fn auction_bundle_fails() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let collection_id = setup_collection(collection_owner);
		assert_ok!(Nft::mint(Some(collection_owner).into(), collection_id, 2, None));
		assert!(has_event(Event::<Test>::Mint {
			collection_id,
			start: 0,
			end: 1,
			owner: collection_owner,
		}));

		// empty tokens fails
		assert_noop!(
			Nft::auction(
				Some(collection_owner).into(),
				collection_id,
				Default::default(),
				NativeAssetId::get(),
				1_000,
				None,
				None
			),
			Error::<Test>::NoToken
		);
	})
}

#[test]
fn auction() {
	let bidder_1 = create_account(5);
	let bidder_2 = create_account(6);
	let reserve_price = 100_000;
	let winning_bid = reserve_price + 1;

	TestExt::default()
		.with_balances(&[(bidder_1, reserve_price), (bidder_2, winning_bid)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_token();

			let listing_id = Nft::next_listing_id();
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Nft::auction(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				NativeAssetId::get(),
				reserve_price,
				Some(1),
				None,
			));
			assert_eq!(Nft::token_locks(&token_id).unwrap(), TokenLockReason::Listed(listing_id));
			assert_eq!(Nft::next_listing_id(), listing_id + 1);
			assert!(Nft::open_collection_listings(collection_id, listing_id).unwrap());

			// first bidder at reserve price
			assert_ok!(Nft::bid(Some(bidder_1).into(), listing_id, reserve_price,));
			assert_eq!(
				AssetsExt::hold_balance(&NftPalletId::get(), &bidder_1, &NativeAssetId::get()),
				reserve_price
			);

			// second bidder raises bid
			assert_ok!(Nft::bid(Some(bidder_2).into(), listing_id, winning_bid,));
			assert_eq!(
				AssetsExt::hold_balance(&NftPalletId::get(), &bidder_2, &NativeAssetId::get()),
				winning_bid
			);
			assert!(AssetsExt::hold_balance(&NftPalletId::get(), &bidder_1, &NativeAssetId::get())
				.is_zero());

			// end auction
			let _ = Nft::on_initialize(System::block_number() + AUCTION_EXTENSION_PERIOD as u64);

			// no royalties, all proceeds to token owner minus network fees
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				winning_bid - MarketplaceNetworkFeePercentage::get().mul(winning_bid)
			);
			// bidder2 funds should be all gone (unreserved and transferred)
			assert!(AssetsExt::reducible_balance(NativeAssetId::get(), &bidder_2, false).is_zero());
			assert!(AssetsExt::hold_balance(&NftPalletId::get(), &bidder_2, &NativeAssetId::get())
				.is_zero());
			// listing metadata removed
			assert!(Listings::<Test>::get(listing_id).is_none());
			assert!(Nft::listing_end_schedule(System::block_number() + 1, listing_id).is_none());

			// ownership changed
			assert!(Nft::token_locks(&token_id).is_none());
			assert_eq!(
				Nft::owned_tokens(collection_id, &bidder_2, 0, 1000),
				(0_u32, 1, vec![token_id.1])
			);
			assert!(Nft::open_collection_listings(collection_id, listing_id).is_none());

			// event logged
			assert!(has_event(Event::<Test>::AuctionSold {
				collection_id,
				listing_id,
				payment_asset: NativeAssetId::get(),
				hammer_price: winning_bid,
				winner: bidder_2
			}));
		});
}

#[test]
fn bid_auto_extends() {
	let bidder_1 = create_account(5);
	let reserve_price = 100_000;

	TestExt::default()
		.with_balances(&[(bidder_1, reserve_price)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_token();
			let reserve_price = 100_000;
			let listing_id = Nft::next_listing_id();
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Nft::auction(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				NativeAssetId::get(),
				reserve_price,
				Some(2),
				None,
			));

			// Place bid
			assert_ok!(Nft::bid(Some(bidder_1).into(), listing_id, reserve_price,));

			if let Some(Listing::Auction(listing)) = Listings::<Test>::get(listing_id) {
				assert_eq!(listing.close, System::block_number() + AUCTION_EXTENSION_PERIOD as u64);
			}
			assert!(Nft::listing_end_schedule(
				System::block_number() + AUCTION_EXTENSION_PERIOD as u64,
				listing_id
			)
			.unwrap());
		});
}

#[test]
fn auction_royalty_payments() {
	let bidder = create_account(5);
	let reserve_price = 100_004;

	TestExt::default()
		.with_balances(&[(bidder, reserve_price)])
		.build()
		.execute_with(|| {
			let beneficiary_1 = create_account(11);
			let beneficiary_2 = create_account(12);
			let collection_owner = create_account(1);
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![
					(collection_owner, Permill::from_float(0.1111)),
					(beneficiary_1, Permill::from_float(0.1111)),
					(beneficiary_2, Permill::from_float(0.1111)),
				]),
			};
			let (collection_id, token_id, token_owner) =
				setup_token_with_royalties(royalties_schedule.clone(), 1);
			let listing_id = Nft::next_listing_id();
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Nft::auction(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				NativeAssetId::get(),
				reserve_price,
				Some(1),
				None,
			));

			// first bidder at reserve price
			assert_ok!(Nft::bid(Some(bidder).into(), listing_id, reserve_price,));

			// end auction
			let _ = Nft::on_initialize(System::block_number() + AUCTION_EXTENSION_PERIOD as u64);

			// royalties paid out
			let presale_issuance = AssetsExt::total_issuance(NativeAssetId::get());
			// royalties distributed according to `entitlements` map
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &collection_owner, false),
				royalties_schedule.entitlements[0].1 * reserve_price
			);
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_1, false),
				royalties_schedule.entitlements[1].1 * reserve_price
			);
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_2, false),
				royalties_schedule.entitlements[2].1 * reserve_price
			);
			// token owner gets sale price - (royalties + network fee)
			let royalties = royalties_schedule
				.entitlements
				.into_iter()
				.map(|(_, e)| e * reserve_price)
				.sum::<Balance>();
			let network_fee = MarketplaceNetworkFeePercentage::get().mul(reserve_price);
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				reserve_price - (royalties + network_fee)
			);
			assert!(AssetsExt::reducible_balance(NativeAssetId::get(), &bidder, false).is_zero());
			assert!(AssetsExt::hold_balance(&NftPalletId::get(), &bidder, &NativeAssetId::get())
				.is_zero());

			assert_eq!(AssetsExt::total_issuance(NativeAssetId::get()), presale_issuance);

			// listing metadata removed
			assert!(!Listings::<Test>::contains_key(listing_id));
			assert!(!ListingEndSchedule::<Test>::contains_key(
				System::block_number() + 1,
				listing_id,
			));

			// ownership changed
			assert_eq!(
				Nft::owned_tokens(collection_id, &bidder, 0, 1000),
				(0_u32, 1, vec![token_id.1])
			);
		});
}

#[test]
fn close_listings_at_removes_listing_data() {
	TestExt::default().build().execute_with(|| {
		let collection_id = Nft::next_collection_uuid().unwrap();
		let price = 123_456;
		let token_1 = (collection_id, 0);
		let seller = create_account(1);
		let listings = vec![
			// an open sale which won't be bought before closing
			Listing::<Test>::FixedPrice(FixedPriceListing::<Test> {
				payment_asset: NativeAssetId::get(),
				fixed_price: price,
				buyer: None,
				close: System::block_number() + 1,
				seller: seller.clone(),
				collection_id,
				serial_numbers: BoundedVec::try_from(vec![token_1.1]).unwrap(),
				royalties_schedule: Default::default(),
				marketplace_id: None,
			}),
			// an open auction which has no bids before closing
			Listing::<Test>::Auction(AuctionListing::<Test> {
				payment_asset: NativeAssetId::get(),
				reserve_price: price,
				close: System::block_number() + 1,
				seller: seller.clone(),
				collection_id,
				serial_numbers: BoundedVec::try_from(vec![token_1.1]).unwrap(),
				royalties_schedule: Default::default(),
				marketplace_id: None,
			}),
			// an open auction which has a winning bid before closing
			Listing::<Test>::Auction(AuctionListing::<Test> {
				payment_asset: NativeAssetId::get(),
				reserve_price: price,
				close: System::block_number() + 1,
				seller: seller.clone(),
				collection_id,
				serial_numbers: BoundedVec::try_from(vec![token_1.1]).unwrap(),
				royalties_schedule: Default::default(),
				marketplace_id: None,
			}),
		];

		// setup listings storage
		for (listing_id, listing) in listings.iter().enumerate() {
			let listing_id = listing_id as ListingId;
			Listings::<Test>::insert(listing_id, listing.clone());
			ListingEndSchedule::<Test>::insert(System::block_number() + 1, listing_id, true);
		}
		// winning bidder has no funds, this should cause settlement failure
		ListingWinningBid::<Test>::insert(2, (create_account(11), 100u128));

		// Close the listings
		Nft::close_listings_at(System::block_number() + 1);

		// Storage clear
		assert!(ListingEndSchedule::<Test>::iter_prefix_values(System::block_number() + 1)
			.count()
			.is_zero());
		for listing_id in 0..listings.len() as ListingId {
			assert!(Listings::<Test>::get(listing_id).is_none());
			assert!(Nft::listing_winning_bid(listing_id).is_none());
			assert!(Nft::listing_end_schedule(System::block_number() + 1, listing_id).is_none());
		}

		assert!(has_event(Event::<Test>::AuctionClose {
			collection_id,
			listing_id: 1,
			reason: AuctionClosureReason::ExpiredNoBids
		}));
		assert!(has_event(Event::<Test>::AuctionClose {
			collection_id,
			listing_id: 2,
			reason: AuctionClosureReason::SettlementFailed
		}));
		assert!(has_event(Event::<Test>::FixedPriceSaleClose {
			collection_id,
			serial_numbers: vec![token_1.1],
			listing_id: 0,
			reason: FixedPriceClosureReason::Expired
		}));
	});
}

#[test]
fn auction_fails_prechecks() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();

		let reserve_price = 100_000;

		// token doesn't exist
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![2]).unwrap();
		assert_noop!(
			Nft::auction(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				NativeAssetId::get(),
				reserve_price,
				Some(1),
				None,
			),
			Error::<Test>::NotTokenOwner
		);

		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		// not owner
		let bob = create_account(6);
		assert_noop!(
			Nft::auction(
				Some(bob).into(),
				collection_id,
				serial_numbers.clone(),
				NativeAssetId::get(),
				reserve_price,
				Some(1),
				None,
			),
			Error::<Test>::NotTokenOwner
		);

		// setup listed token, and try list it again
		assert_ok!(Nft::auction(
			Some(token_owner).into(),
			collection_id,
			serial_numbers.clone(),
			NativeAssetId::get(),
			reserve_price,
			Some(1),
			None,
		));
		// already listed
		assert_noop!(
			Nft::auction(
				Some(token_owner).into(),
				collection_id,
				serial_numbers.clone(),
				NativeAssetId::get(),
				reserve_price,
				Some(1),
				None,
			),
			Error::<Test>::TokenLocked
		);

		// listed for auction
		assert_noop!(
			Nft::sell(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				None,
				NativeAssetId::get(),
				reserve_price,
				None,
				None,
			),
			Error::<Test>::TokenLocked
		);
	});
}

#[test]
fn bid_fails_prechecks() {
	let bidder = create_account(5);
	let reserve_price = 100_004;

	TestExt::default()
		.with_balances(&[(bidder, reserve_price)])
		.build()
		.execute_with(|| {
			let missing_listing_id = 5;
			assert_noop!(
				Nft::bid(Some(create_account(1)).into(), missing_listing_id, 100),
				Error::<Test>::NotForAuction
			);

			let (collection_id, token_id, token_owner) = setup_token();
			let listing_id = Nft::next_listing_id();
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Nft::auction(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				NativeAssetId::get(),
				reserve_price,
				Some(1),
				None,
			));

			// < reserve
			assert_noop!(
				Nft::bid(Some(bidder).into(), listing_id, reserve_price - 1),
				Error::<Test>::BidTooLow
			);

			// balance already reserved for other reasons
			assert_ok!(AssetsExt::place_hold(
				NftPalletId::get(),
				&bidder,
				NativeAssetId::get(),
				reserve_price
			));
			assert_noop!(
				Nft::bid(Some(bidder).into(), listing_id, reserve_price),
				pallet_balances::Error::<Test>::InsufficientBalance
			);
			assert_ok!(AssetsExt::release_hold(
				NftPalletId::get(),
				&bidder,
				NativeAssetId::get(),
				reserve_price
			));

			// <= current bid
			assert_ok!(Nft::bid(Some(bidder).into(), listing_id, reserve_price,));
			assert_noop!(
				Nft::bid(Some(bidder).into(), listing_id, reserve_price),
				Error::<Test>::BidTooLow
			);
		});
}

#[test]
fn bid_no_balance_should_fail() {
	let bidder = create_account(5);

	TestExt::default().build().execute_with(|| {
		let missing_listing_id = 5;
		assert_noop!(
			Nft::bid(Some(create_account(1)).into(), missing_listing_id, 100),
			Error::<Test>::NotForAuction
		);

		let (collection_id, token_id, token_owner) = setup_token();
		let reserve_price = 100_000;
		let listing_id = Nft::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_ok!(Nft::auction(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			NativeAssetId::get(),
			reserve_price,
			Some(1),
			None,
		));

		// no free balance
		assert_noop!(
			Nft::bid(Some(bidder).into(), listing_id, reserve_price),
			pallet_balances::Error::<Test>::InsufficientBalance
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
			Nft::collection_info(collection_id).unwrap().collection_issuance,
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
			Nft::collection_info(collection_id).unwrap().collection_issuance,
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

		// not collection owner
		assert_noop!(
			Nft::mint(Some(create_account(2)).into(), collection_id, 5, None),
			Error::<Test>::NotCollectionOwner
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
fn make_simple_offer() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, _) = setup_token();
			let offer_amount: Balance = 100;
			let (offer_id, _) = make_new_simple_offer(offer_amount, token_id, buyer, None);
			assert_eq!(Nft::token_offers(token_id).unwrap(), vec![offer_id]);
			// Check funds have been locked
			assert_eq!(
				AssetsExt::hold_balance(&NftPalletId::get(), &buyer, &NativeAssetId::get()),
				offer_amount
			);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &buyer),
				initial_balance_buyer - offer_amount
			);
		});
}

#[test]
fn make_simple_offer_insufficient_funds_should_fail() {
	TestExt::default().build().execute_with(|| {
		let (_, token_id, _) = setup_token();
		let buyer = create_account(3);
		let offer_amount: Balance = 100;
		assert_eq!(AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false), 0);

		assert_noop!(
			Nft::make_simple_offer(
				Some(buyer).into(),
				token_id,
				offer_amount,
				NativeAssetId::get(),
				None
			),
			pallet_balances::Error::<Test>::InsufficientBalance
		);
	});
}

#[test]
fn make_simple_offer_zero_amount_should_fail() {
	TestExt::default().build().execute_with(|| {
		let (_, token_id, _) = setup_token();
		let buyer = create_account(3);
		let offer_amount: Balance = 0;
		assert_eq!(AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false), 0);

		assert_noop!(
			Nft::make_simple_offer(
				Some(buyer).into(),
				token_id,
				offer_amount,
				NativeAssetId::get(),
				None
			),
			Error::<Test>::ZeroOffer
		);
	});
}

#[test]
fn make_simple_offer_token_owner_should_fail() {
	TestExt::default().build().execute_with(|| {
		let (_, token_id, token_owner) = setup_token();
		let offer_amount: Balance = 100;

		assert_noop!(
			Nft::make_simple_offer(
				Some(token_owner).into(),
				token_id,
				offer_amount,
				NativeAssetId::get(),
				None
			),
			Error::<Test>::IsTokenOwner
		);
	});
}

#[test]
fn make_simple_offer_on_fixed_price_listing() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_token();
			let offer_amount: Balance = 100;
			let sell_price = 100_000;
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			let listing_id = Nft::next_listing_id();

			assert_ok!(Nft::sell(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				None,
				NativeAssetId::get(),
				sell_price,
				None,
				None,
			));
			// Sanity check
			assert!(Listings::<Test>::get(listing_id).is_some());
			assert!(Nft::token_locks(token_id).is_some());

			let (offer_id, _) = make_new_simple_offer(offer_amount, token_id, buyer, None);
			// Check funds have been locked
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false),
				initial_balance_buyer - offer_amount
			);
			assert_eq!(
				AssetsExt::hold_balance(&NftPalletId::get(), &buyer, &NativeAssetId::get()),
				offer_amount
			);

			assert_ok!(Nft::accept_offer(Some(token_owner).into(), offer_id,));

			// Check that fixed price listing and locks are now removed
			assert!(Listings::<Test>::get(listing_id).is_none());
			assert!(Nft::token_locks(token_id).is_none());
			// Check offer storage has been removed
			assert!(Nft::token_offers(token_id).is_none());
			assert!(Nft::offers(offer_id).is_none());

			// Check funds have been transferred
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false),
				initial_balance_buyer - offer_amount
			);
			assert!(AssetsExt::hold_balance(&NftPalletId::get(), &buyer, &NativeAssetId::get())
				.is_zero());
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				offer_amount
			);
		});
}

#[test]
fn make_simple_offer_on_auction_should_fail() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_token();
			let offer_amount: Balance = 100;
			let reserve_price = 100_000;
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Nft::auction(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				NativeAssetId::get(),
				reserve_price,
				Some(System::block_number() + 1),
				None,
			));

			assert_noop!(
				Nft::make_simple_offer(
					Some(buyer).into(),
					token_id,
					offer_amount,
					NativeAssetId::get(),
					None
				),
				Error::<Test>::TokenOnAuction
			);
		});
}

#[test]
fn cancel_offer() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, _) = setup_token();
			let offer_amount: Balance = 100;

			let (offer_id, _) = make_new_simple_offer(offer_amount, token_id, buyer, None);
			assert_ok!(Nft::cancel_offer(Some(buyer).into(), offer_id));

			assert!(has_event(Event::<Test>::OfferCancel { offer_id, token_id }));

			// Check storage has been removed
			assert!(Nft::token_offers(token_id).is_none());
			assert_eq!(Nft::offers(offer_id), None);
			// Check funds have been unlocked after offer cancelled
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false),
				initial_balance_buyer
			);
			assert!(AssetsExt::hold_balance(&NftPalletId::get(), &buyer, &NativeAssetId::get())
				.is_zero());
		});
}

#[test]
fn cancel_offer_multiple_offers() {
	let buyer_1 = create_account(3);
	let buyer_2 = create_account(4);
	let initial_balance_buyer_1: Balance = 1000;
	let initial_balance_buyer_2: Balance = 1000;

	TestExt::default()
		.with_balances(&[(buyer_1, initial_balance_buyer_1), (buyer_2, initial_balance_buyer_2)])
		.build()
		.execute_with(|| {
			let (_, token_id, _) = setup_token();

			let offer_amount_1: Balance = 100;
			let offer_amount_2: Balance = 150;

			let (offer_id_1, _) = make_new_simple_offer(offer_amount_1, token_id, buyer_1, None);
			let (offer_id_2, offer_2) =
				make_new_simple_offer(offer_amount_2, token_id, buyer_2, None);

			// Can't cancel other offer
			assert_noop!(
				Nft::cancel_offer(Some(buyer_1).into(), offer_id_2),
				Error::<Test>::NotBuyer
			);
			// Can cancel their offer
			assert_ok!(Nft::cancel_offer(Some(buyer_1).into(), offer_id_1));
			assert!(has_event(Event::<Test>::OfferCancel { offer_id: offer_id_1, token_id }));

			// Check storage has been removed
			let offer_vector: Vec<OfferId> = vec![offer_id_2];
			assert_eq!(Nft::token_offers(token_id).unwrap(), offer_vector);
			assert_eq!(Nft::offers(offer_id_2), Some(OfferType::Simple(offer_2.clone())));
			assert_eq!(Nft::offers(offer_id_1), None);

			// Check funds have been unlocked after offer cancelled
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer_1, false),
				initial_balance_buyer_1
			);
			assert!(AssetsExt::hold_balance(&NftPalletId::get(), &buyer_1, &NativeAssetId::get())
				.is_zero());
			// Check buyer_2 funds have not been unlocked
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer_2, false),
				initial_balance_buyer_2 - offer_amount_2
			);
			assert_eq!(
				AssetsExt::hold_balance(&NftPalletId::get(), &buyer_2, &NativeAssetId::get()),
				offer_amount_2
			);
		});
}

#[test]
fn cancel_offer_not_buyer_should_fail() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, _) = setup_token();
			let offer_amount: Balance = 100;
			let (offer_id, _) = make_new_simple_offer(offer_amount, token_id, buyer, None);

			assert_noop!(
				Nft::cancel_offer(Some(create_account(4)).into(), offer_id),
				Error::<Test>::NotBuyer
			);
		});
}

#[test]
fn accept_offer() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, token_owner) = setup_token();
			let offer_amount: Balance = 100;
			let (offer_id, _) = make_new_simple_offer(offer_amount, token_id, buyer, None);
			assert_ok!(Nft::accept_offer(Some(token_owner).into(), offer_id));
			assert!(has_event(Event::<Test>::OfferAccept {
				offer_id,
				token_id,
				amount: offer_amount,
				asset_id: NativeAssetId::get()
			}));

			// Check storage has been removed
			assert!(Nft::token_offers(token_id).is_none());
			assert!(Nft::offers(offer_id).is_none());
			// Check funds have been transferred
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false),
				initial_balance_buyer - offer_amount
			);
			assert!(AssetsExt::hold_balance(&NftPalletId::get(), &buyer, &NativeAssetId::get())
				.is_zero());
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				offer_amount
			);
		});
}

#[test]
fn accept_offer_multiple_offers() {
	let buyer_1 = create_account(3);
	let buyer_2 = create_account(4);
	let initial_balance_buyer_1: Balance = 1000;
	let initial_balance_buyer_2: Balance = 1000;

	TestExt::default()
		.with_balances(&[(buyer_1, initial_balance_buyer_1), (buyer_2, initial_balance_buyer_2)])
		.build()
		.execute_with(|| {
			let (_, token_id, token_owner) = setup_token();

			let offer_amount_1: Balance = 100;
			let offer_amount_2: Balance = 150;

			let (offer_id_1, offer_1) =
				make_new_simple_offer(offer_amount_1, token_id, buyer_1, None);
			let (offer_id_2, _) = make_new_simple_offer(offer_amount_2, token_id, buyer_2, None);

			// Accept second offer
			assert_ok!(Nft::accept_offer(Some(token_owner).into(), offer_id_2));
			assert!(has_event(Event::<Test>::OfferAccept {
				offer_id: offer_id_2,
				token_id,
				amount: offer_amount_2,
				asset_id: NativeAssetId::get()
			}));
			// Check storage has been removed
			let offer_vector: Vec<OfferId> = vec![offer_id_1];
			assert_eq!(Nft::token_offers(token_id).unwrap(), offer_vector);
			assert_eq!(Nft::offers(offer_id_1), Some(OfferType::Simple(offer_1.clone())));
			assert_eq!(Nft::offers(offer_id_2), None);

			// Check funds have been transferred
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer_2, false),
				initial_balance_buyer_2 - offer_amount_2
			);
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer_1, false),
				initial_balance_buyer_1 - offer_amount_1
			);
			assert_eq!(
				AssetsExt::hold_balance(&NftPalletId::get(), &buyer_1, &NativeAssetId::get()),
				offer_amount_1
			);
			assert!(AssetsExt::hold_balance(&NftPalletId::get(), &buyer_2, &NativeAssetId::get())
				.is_zero());
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				offer_amount_2 - MarketplaceNetworkFeePercentage::get().mul(offer_amount_2)
			);

			// Accept first offer should fail as token_owner is no longer owner
			assert_noop!(
				Nft::accept_offer(Some(token_owner).into(), offer_id_1),
				Error::<Test>::NotTokenOwner
			);
		});
}

#[test]
fn accept_offer_pays_marketplace_royalties() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, token_owner) = setup_token();
			let offer_amount: Balance = 100;

			let marketplace_account = create_account(4);
			let entitlements: Permill = Permill::from_float(0.1);
			let marketplace_id = Nft::next_marketplace_id();
			assert_ok!(Nft::register_marketplace(
				Some(marketplace_account).into(),
				None,
				entitlements
			));

			let (offer_id, _) =
				make_new_simple_offer(offer_amount, token_id, buyer, Some(marketplace_id));
			assert_ok!(Nft::accept_offer(Some(token_owner).into(), offer_id));

			// Check storage has been removed
			assert!(Nft::token_offers(token_id).is_none());
			assert_eq!(Nft::offers(offer_id), None);
			// Check funds have been transferred with royalties
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false),
				initial_balance_buyer - offer_amount
			);
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &marketplace_account, false),
				entitlements * offer_amount
			);
			assert!(AssetsExt::hold_balance(&NftPalletId::get(), &buyer, &NativeAssetId::get())
				.is_zero());
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				offer_amount - (entitlements * offer_amount)
			);
		});
}

#[test]
fn accept_offer_not_token_owner_should_fail() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, _) = setup_token();
			let offer_amount: Balance = 100;

			let (offer_id, _) = make_new_simple_offer(offer_amount, token_id, buyer, None);
			assert_noop!(
				Nft::accept_offer(Some(create_account(4)).into(), offer_id),
				Error::<Test>::NotTokenOwner
			);
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
			serial_numbers: serial_numbers_unbounded
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
		Nft::mint_bridged_token(&token_owner, collection_id, token_ids.clone());

		// Event is thrown
		assert!(has_event(Event::<Test>::BridgedMint {
			collection_id,
			serial_numbers: BoundedVec::truncate_from(token_ids.clone()),
			owner: token_owner,
		}));

		// Ownership checks
		assert_eq!(Nft::token_balance_of(&token_owner, collection_id), token_ids.len() as u32);
		let collection_info = Nft::collection_info(collection_id).unwrap();
		token_ids.iter().for_each(|&serial_number| {
			assert!(collection_info.is_token_owner(&token_owner, serial_number));
		});

		// Next serial number should be 0, origin chain is Ethereum so we don't count this
		assert_eq!(Nft::collection_info(collection_id).unwrap().next_serial_number, 0);
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
		Nft::mint_bridged_token(&token_owner, collection_id, token_ids.clone());
		// Minting to another account_id should still succeed, but the token balance of this account
		// will be 0. This is because the tokens are already minted and each token will be silently
		// skipped
		let other_owner = create_account(4);
		Nft::mint_bridged_token(&other_owner, collection_id, token_ids.clone());

		// Ownership checks
		// We expect the token balance to be 5 as that is the number of unique token_ids in the vec
		assert_eq!(Nft::token_balance_of(&token_owner, collection_id), 5);

		let collection_info = Nft::collection_info(collection_id).unwrap();
		token_ids.iter().for_each(|&serial_number| {
			assert!(collection_info.is_token_owner(&token_owner, serial_number));
		});

		// Collection issuance should be 5 to indicate the 5 unique tokens
		assert_eq!(Nft::collection_info(collection_id).unwrap().collection_issuance, 5_u32);
		// Other owner shouldn't have any tokens
		assert_eq!(Nft::token_balance_of(&other_owner, collection_id), 0);

		// Now try with 3 more unique tokens
		let token_ids: Vec<SerialNumber> = vec![0, 2, 3000, 40005, 5, 1234, 9, 1000];
		Nft::mint_bridged_token(&other_owner, collection_id, token_ids.clone());

		// Collection issuance should now be 8 to indicate the 3 additional unique tokens
		assert_eq!(Nft::collection_info(collection_id).unwrap().collection_issuance, 8_u32);
		// We expect the token balance to be 3
		assert_eq!(Nft::token_balance_of(&other_owner, collection_id), 3);

		let collection_info = Nft::collection_info(collection_id).unwrap();
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

		let collection_info = Nft::collection_info(collection_id).unwrap();

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
		let mut collection_info = Nft::collection_info(collection_id).unwrap();
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
		let mut collection_info = Nft::collection_info(collection_id).unwrap();
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
		let mut collection_info = Nft::collection_info(collection_id).unwrap();
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
			Nft::collection_info(collection_id).unwrap(),
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

mod set_fee_to {
	use super::*;

	#[test]
	fn set_fee_to_works() {
		TestExt::default().build().execute_with(|| {
			// Ensure default is correct
			let default_fee_to: AccountId =
				mock::DefaultFeeTo::get().unwrap().into_account_truncating();
			assert_eq!(FeeTo::<Test>::get().unwrap(), default_fee_to);

			// Change fee_to account
			let new_fee_to = create_account(10);
			assert_ok!(Nft::set_fee_to(RawOrigin::Root.into(), Some(new_fee_to.clone())));

			// Event thrown
			assert!(has_event(Event::<Test>::FeeToSet { account: Some(new_fee_to) }));
			// Storage updated
			assert_eq!(FeeTo::<Test>::get().unwrap(), new_fee_to);
		});
	}

	#[test]
	fn set_fee_to_not_root_fails() {
		TestExt::default().build().execute_with(|| {
			// Change fee_to account from not sudo should fail
			let new_fee_to = create_account(10);
			assert_noop!(
				Nft::set_fee_to(Some(create_account(11)).into(), Some(new_fee_to)),
				BadOrigin
			);
		});
	}
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
				base_uri: new_metadata_scheme
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
