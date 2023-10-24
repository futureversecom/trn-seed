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

use crate::{
	mock::*, Config, Error, SftCollectionInfo, SftCollectionInformation, SftTokenBalance,
	SftTokenInformation, TokenInfo,
};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use seed_primitives::{
	AccountId, Balance, CollectionUuid, MetadataScheme, OriginChain, RoyaltiesSchedule,
	SerialNumber, TokenId,
};
use sp_core::H160;
use sp_runtime::{BoundedVec, Permill};

/// Helper function to create a collection used for tests
/// Returns the collectionUuid
pub fn create_test_collection(owner: <Test as frame_system::Config>::AccountId) -> CollectionUuid {
	let collection_uuid = next_collection_uuid();
	let collection_name = bounded_string("test-collection");
	let metadata_scheme = MetadataScheme::try_from(b"example.com/metadata".as_slice()).unwrap();

	assert_ok!(Sft::create_collection(
		Some(owner).into(),
		collection_name.clone(),
		None,
		metadata_scheme.clone(),
		None
	));

	collection_uuid
}

/// Helper function to create a token used for tests
/// Returns the TokenId (CollectionId, SerialNumber)
pub fn create_test_token(
	collection_owner: <Test as frame_system::Config>::AccountId,
	token_owner: <Test as frame_system::Config>::AccountId,
	initial_issuance: Balance,
) -> TokenId {
	let collection_id = create_test_collection(collection_owner);
	let token_name = bounded_string("test-token");

	assert_ok!(Sft::create_token(
		Some(collection_owner).into(),
		collection_id,
		token_name,
		initial_issuance,
		None,
		Some(token_owner),
	));

	let token_id = (collection_id, 0);

	// Sanity check
	assert_eq!(
		TokenInfo::<Test>::get(token_id).unwrap().free_balance_of(&token_owner),
		initial_issuance
	);

	token_id
}

/// Helper functions for creating accounts from a u64 seed
pub fn create_account(seed: u64) -> <Test as frame_system::Config>::AccountId {
	<Test as frame_system::Config>::AccountId::from(H160::from_low_u64_be(seed))
}

/// Common account Alice
pub fn alice() -> <Test as frame_system::Config>::AccountId {
	create_account(1)
}

/// Common account Bob
pub fn bob() -> <Test as frame_system::Config>::AccountId {
	create_account(2)
}

/// Common account Charlie
pub fn charlie() -> <Test as frame_system::Config>::AccountId {
	create_account(3)
}

/// Helper function for creating the collection name type
pub fn bounded_string(name: &str) -> BoundedVec<u8, <Test as Config>::StringLimit> {
	BoundedVec::truncate_from(name.as_bytes().to_vec())
}

/// Helper function for creating the bounded (SerialNumbers, Balance) type
pub fn bounded_combined(
	serial_numbers: Vec<SerialNumber>,
	quantities: Vec<Balance>,
) -> BoundedVec<(SerialNumber, Balance), <Test as Config>::MaxSerialsPerMint> {
	let combined: Vec<(SerialNumber, Balance)> =
		serial_numbers.into_iter().zip(quantities).collect();
	BoundedVec::truncate_from(combined)
}

/// Helper function for creating the bounded SerialNumbers type
pub fn bounded_serials(
	serial_numbers: Vec<SerialNumber>,
) -> BoundedVec<SerialNumber, <Test as Config>::MaxSerialsPerMint> {
	BoundedVec::truncate_from(serial_numbers)
}

/// Helper function for creating the bounded quantities type
pub fn bounded_quantities(
	quantities: Vec<Balance>,
) -> BoundedVec<Balance, <Test as Config>::MaxSerialsPerMint> {
	BoundedVec::truncate_from(quantities)
}

/// Helper function for creating the collection name type
pub fn create_owned_tokens(
	owned_tokens: Vec<(<Test as frame_system::Config>::AccountId, Balance)>,
) -> BoundedVec<
	(<Test as frame_system::Config>::AccountId, SftTokenBalance),
	<Test as Config>::MaxOwnersPerSftToken,
> {
	let owned_tokens =
		owned_tokens.into_iter().map(|(a, b)| (a, SftTokenBalance::new(b, 0))).collect();
	BoundedVec::truncate_from(owned_tokens)
}

/// Helper function to get the next collection Uuid from the NFT pallet
pub fn next_collection_uuid() -> CollectionUuid {
	<Test as Config>::NFTExt::next_collection_uuid().expect("Failed to get next collection uuid")
}

mod create_collection {
	use super::*;

	#[test]
	fn create_collection_works() {
		TestExt::default().build().execute_with(|| {
			// CollectionId stored in the NFT pallet, get here to check it is incremented
			// properly after we create a collection
			let nft_collection_id = pallet_nft::NextCollectionId::<Test>::get();
			// The actual collection_uuid (Different to the NextCollectionId in NFT pallet
			let collection_uuid = next_collection_uuid();
			let caller = alice();
			let collection_name = bounded_string("test");
			let collection_owner = bob();
			let metadata_scheme =
				MetadataScheme::try_from(b"example.com/metadata".as_slice()).unwrap();
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(collection_owner, Permill::one())]),
			};

			// Call works
			assert_ok!(Sft::create_collection(
				Some(caller).into(),
				collection_name.clone(),
				Some(collection_owner),
				metadata_scheme.clone(),
				Some(royalties_schedule.clone())
			));

			// CollectionId was incremented correctly in NFT pallet
			assert_eq!(pallet_nft::NextCollectionId::<Test>::get(), nft_collection_id + 1);

			// Storage correctly updated
			let expected_collection_info = SftCollectionInformation {
				collection_owner,
				collection_name: collection_name.clone(),
				metadata_scheme: metadata_scheme.clone(),
				royalties_schedule: Some(royalties_schedule.clone()),
				origin_chain: OriginChain::Root,
				next_serial_number: 0,
			};
			assert_eq!(
				SftCollectionInfo::<Test>::get(collection_uuid).unwrap(),
				expected_collection_info
			);

			// Event emitted
			System::assert_last_event(RuntimeEvent::Sft(crate::Event::CollectionCreate {
				collection_id: collection_uuid,
				collection_owner,
				metadata_scheme,
				name: collection_name,
				royalties_schedule: Some(royalties_schedule),
				origin_chain: OriginChain::Root,
			}));
		});
	}

	#[test]
	fn create_collection_no_specified_owner() {
		TestExt::default().build().execute_with(|| {
			let collection_uuid = next_collection_uuid();
			let caller = alice();
			let collection_name = bounded_string("test");
			let metadata_scheme =
				MetadataScheme::try_from(b"example.com/metadata".as_slice()).unwrap();

			// Call works
			assert_ok!(Sft::create_collection(
				Some(caller).into(),
				collection_name.clone(),
				None,
				metadata_scheme.clone(),
				None
			));

			// The collection owner is set to the caller as we did not specify a collection_owner
			assert_eq!(
				SftCollectionInfo::<Test>::get(collection_uuid).unwrap().collection_owner,
				caller
			)
		});
	}

	#[test]
	fn create_collection_invalid_collection_name_fails() {
		TestExt::default().build().execute_with(|| {
			let metadata_scheme =
				MetadataScheme::try_from(b"example.com/metadata".as_slice()).unwrap();

			// Empty Collection Name
			let empty_collection_name = bounded_string("");
			assert_noop!(
				Sft::create_collection(
					Some(alice()).into(),
					empty_collection_name,
					None,
					metadata_scheme.clone(),
					None
				),
				Error::<Test>::NameInvalid
			);

			// Non utf-8 Collection Name
			let non_utf8_collection_name = BoundedVec::truncate_from(vec![0xfe, 0xff]);
			assert_noop!(
				Sft::create_collection(
					Some(alice()).into(),
					non_utf8_collection_name,
					None,
					metadata_scheme.clone(),
					None
				),
				Error::<Test>::NameInvalid
			);
		});
	}

	#[test]
	fn create_collection_invalid_royalties_schedule_fails() {
		TestExt::default().build().execute_with(|| {
			let metadata_scheme =
				MetadataScheme::try_from(b"example.com/metadata".as_slice()).unwrap();

			// Empty RoyaltiesSchedule
			let empty_royalties_schedule =
				RoyaltiesSchedule { entitlements: BoundedVec::default() };
			assert_noop!(
				Sft::create_collection(
					Some(alice()).into(),
					bounded_string("test-collection"),
					None,
					metadata_scheme.clone(),
					Some(empty_royalties_schedule),
				),
				Error::<Test>::RoyaltiesInvalid
			);

			// Too Large RoyaltiesSchedule vec
			// MAX_ENTITLEMENTS is set to 8 so anything over 8 should fail
			let large_royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![
					(bob(), Permill::one()),
					(bob(), Permill::one()),
					(bob(), Permill::one()),
					(bob(), Permill::one()),
					(bob(), Permill::one()),
					(bob(), Permill::one()),
					(bob(), Permill::one()),
					(bob(), Permill::one()),
					(bob(), Permill::one()),
				]),
			};
			assert_noop!(
				Sft::create_collection(
					Some(alice()).into(),
					bounded_string("test-collection"),
					None,
					metadata_scheme.clone(),
					Some(large_royalties_schedule),
				),
				Error::<Test>::RoyaltiesInvalid
			);

			// Royalties over 100%
			// MAX_ENTITLEMENTS is set to 8 so anything over 8 should fail
			let large_royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![
					(bob(), Permill::from_parts(500_000)),
					(bob(), Permill::from_parts(500_001)),
				]),
			};
			assert_noop!(
				Sft::create_collection(
					Some(alice()).into(),
					bounded_string("test-collection"),
					None,
					metadata_scheme.clone(),
					Some(large_royalties_schedule),
				),
				Error::<Test>::RoyaltiesInvalid
			);
		});
	}
}

mod create_token {
	use super::*;

	#[test]
	fn create_token_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let collection_id = create_test_collection(collection_owner);
			let token_name = bounded_string("my-token");
			let max_issuance = 100;
			let initial_issuance = 10;
			let token_owner = bob();

			assert_ok!(Sft::create_token(
				Some(collection_owner).into(),
				collection_id,
				token_name.clone(),
				initial_issuance,
				Some(max_issuance),
				Some(token_owner.clone()),
			));

			// Check storage added correctly
			let expected_owned_tokens =
				create_owned_tokens(vec![(token_owner.clone(), initial_issuance)]);
			let expected_token_info = SftTokenInformation {
				token_name: token_name.clone(),
				max_issuance: Some(max_issuance),
				token_issuance: initial_issuance,
				owned_tokens: expected_owned_tokens,
			};
			assert_eq!(TokenInfo::<Test>::get((collection_id, 0)).unwrap(), expected_token_info);
			// Next serial number incremented
			assert_eq!(
				SftCollectionInfo::<Test>::get(collection_id).unwrap().next_serial_number,
				1
			);

			// Event emitted
			System::assert_last_event(RuntimeEvent::Sft(crate::Event::TokenCreate {
				token_id: (collection_id, 0),
				initial_issuance,
				max_issuance: Some(max_issuance),
				token_name,
				token_owner,
			}));
		});
	}

	#[test]
	fn do_create_token_returns_serial() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let collection_id = create_test_collection(collection_owner);
			let token_name = bounded_string("my-token");

			let serial_number = Sft::do_create_token(
				collection_owner,
				collection_id,
				token_name.clone(),
				0,
				None,
				None,
			)
			.unwrap();
			assert_eq!(serial_number, 0);

			let serial_number = Sft::do_create_token(
				collection_owner,
				collection_id,
				token_name.clone(),
				0,
				None,
				None,
			)
			.unwrap();
			assert_eq!(serial_number, 1);
		});
	}

	#[test]
	fn create_token_zero_initial_issuance_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let collection_id = create_test_collection(collection_owner);
			let token_name = bounded_string("my-token");
			let initial_issuance = 0;

			assert_ok!(Sft::create_token(
				Some(collection_owner).into(),
				collection_id,
				token_name.clone(),
				initial_issuance,
				None,
				None,
			));

			// Check storage added correctly
			// Zero initial issuance means the vec should be empty
			let expected_owned_tokens = create_owned_tokens(vec![]);
			let expected_token_info = SftTokenInformation {
				token_name: token_name.clone(),
				max_issuance: None,
				token_issuance: initial_issuance,
				owned_tokens: expected_owned_tokens,
			};
			assert_eq!(TokenInfo::<Test>::get((collection_id, 0)).unwrap(), expected_token_info);

			// Event emitted
			System::assert_last_event(RuntimeEvent::Sft(crate::Event::TokenCreate {
				token_id: (collection_id, 0),
				initial_issuance,
				max_issuance: None,
				token_name,
				token_owner: collection_owner,
			}));
		});
	}

	#[test]
	fn create_token_no_collection_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_id = 1;

			assert_noop!(
				Sft::create_token(
					Some(alice()).into(),
					collection_id,
					bounded_string("my-token"),
					0,
					None,
					None,
				),
				Error::<Test>::NoCollectionFound
			);
		});
	}

	#[test]
	fn create_token_not_collection_owner_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let collection_id = create_test_collection(collection_owner);
			let malicious_actor = bob();

			assert_noop!(
				Sft::create_token(
					Some(malicious_actor).into(),
					collection_id,
					bounded_string("my-token"),
					0,
					None,
					None,
				),
				Error::<Test>::NotCollectionOwner
			);
		});
	}

	#[test]
	fn create_token_invalid_token_name_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let collection_id = create_test_collection(collection_owner);

			// Empty Name
			let empty_token_name = bounded_string("");
			assert_noop!(
				Sft::create_token(
					Some(collection_owner).into(),
					collection_id,
					empty_token_name,
					0,
					None,
					None,
				),
				Error::<Test>::NameInvalid
			);

			// Non utf-8 Name
			let non_utf8_token_name = BoundedVec::truncate_from(vec![0xfe, 0xff]);
			assert_noop!(
				Sft::create_token(
					Some(collection_owner).into(),
					collection_id,
					non_utf8_token_name,
					0,
					None,
					None,
				),
				Error::<Test>::NameInvalid
			);
		});
	}

	#[test]
	fn create_token_invalid_max_issuance_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let collection_id = create_test_collection(collection_owner);

			// Zero max_issuance
			let max_issuance = 0;
			assert_noop!(
				Sft::create_token(
					Some(collection_owner).into(),
					collection_id,
					bounded_string("my-token"),
					0,
					Some(max_issuance),
					None,
				),
				Error::<Test>::InvalidMaxIssuance
			);

			// initial issuance higher than max issuance
			let max_issuance = 1000;
			let initial_issuance = 1001;
			assert_noop!(
				Sft::create_token(
					Some(collection_owner).into(),
					collection_id,
					bounded_string("my-token"),
					initial_issuance,
					Some(max_issuance),
					None,
				),
				Error::<Test>::InvalidMaxIssuance
			);
		});
	}

	#[test]
	fn create_token_invalid_next_serial_number_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let metadata_scheme =
				MetadataScheme::try_from(b"example.com/metadata".as_slice()).unwrap();

			// Create storage with max next serial number
			let dummy_collection_info = SftCollectionInformation {
				collection_owner,
				collection_name: bounded_string("my-collection"),
				metadata_scheme: metadata_scheme.clone(),
				royalties_schedule: None,
				origin_chain: OriginChain::Root,
				next_serial_number: u32::MAX,
			};
			let collection_id = 1;
			SftCollectionInfo::<Test>::insert(collection_id, dummy_collection_info);

			// Should fail as next_serial_number is at it's limit
			assert_noop!(
				Sft::create_token(
					Some(collection_owner).into(),
					collection_id,
					bounded_string("my-token"),
					0,
					None,
					None,
				),
				Error::<Test>::Overflow
			);
		});
	}
}

mod mint {
	use super::*;

	#[test]
	fn mint_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_owner = bob();
			let token_id = create_test_token(collection_owner, collection_owner, 0);
			let (collection_id, serial_number) = token_id;
			let quantity = 1000;

			assert_ok!(Sft::mint(
				Some(collection_owner).into(),
				collection_id,
				bounded_combined(vec![serial_number], vec![quantity]),
				Some(token_owner.clone()),
			));

			// Get updated token_info
			let token_info = TokenInfo::<Test>::get(token_id).unwrap();

			// free balance should now be quantity
			assert_eq!(token_info.free_balance_of(&token_owner), quantity);

			// Owned tokens is correct
			let expected_owned_tokens = create_owned_tokens(vec![(token_owner.clone(), quantity)]);
			assert_eq!(token_info.owned_tokens, expected_owned_tokens);

			// token_issuance updated
			assert_eq!(token_info.token_issuance, quantity);

			// Event emitted
			System::assert_last_event(RuntimeEvent::Sft(crate::Event::Mint {
				collection_id,
				serial_numbers: bounded_serials(vec![serial_number]),
				balances: bounded_quantities(vec![quantity]),
				owner: token_owner,
			}));

			// Mint some more to make sure the balance is added correctly to an existing owner
			let quantity2 = 1337;
			assert_ok!(Sft::mint(
				Some(collection_owner).into(),
				collection_id,
				bounded_combined(vec![serial_number], vec![quantity2]),
				Some(token_owner.clone()),
			));

			// Get updated token_info and check storage
			let token_info = TokenInfo::<Test>::get(token_id).unwrap();
			assert_eq!(token_info.free_balance_of(&token_owner), quantity + quantity2);
			let expected_owned_tokens =
				create_owned_tokens(vec![(token_owner.clone(), quantity + quantity2)]);
			assert_eq!(token_info.owned_tokens, expected_owned_tokens);
			assert_eq!(token_info.token_issuance, quantity + quantity2);

			// Event emitted
			System::assert_last_event(RuntimeEvent::Sft(crate::Event::Mint {
				collection_id,
				serial_numbers: bounded_serials(vec![serial_number]),
				balances: bounded_quantities(vec![quantity2]),
				owner: token_owner,
			}));
		});
	}

	#[test]
	fn mint_over_multiple_tokens_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_owner = bob();
			let collection_id = create_test_collection(collection_owner);
			let serial_numbers: Vec<SerialNumber> = vec![0, 1, 2, 3, 4, 5, 6];
			let quantities: Vec<Balance> = vec![1000, 2000, 3000, 4000, 5000, 6000, 7000];

			// Create each token with initial_issuance = 0
			for _ in serial_numbers.iter() {
				assert_ok!(Sft::create_token(
					Some(collection_owner).into(),
					collection_id,
					bounded_string("my-token"),
					0,
					None,
					None,
				));
			}

			// Mint the quantities to the token_owner for each serial
			assert_ok!(Sft::mint(
				Some(collection_owner).into(),
				collection_id,
				bounded_combined(serial_numbers.clone(), quantities.clone()),
				Some(token_owner.clone()),
			));

			// Check each token has the correct free balance and token issuance
			for (serial_number, quantity) in serial_numbers.iter().zip(quantities.iter()) {
				let token_id = (collection_id, *serial_number);
				let token_info = TokenInfo::<Test>::get(token_id).unwrap();
				assert_eq!(token_info.free_balance_of(&token_owner), *quantity);
				assert_eq!(token_info.token_issuance, *quantity);
			}

			// Event emitted
			System::assert_last_event(RuntimeEvent::Sft(crate::Event::Mint {
				collection_id,
				serial_numbers: bounded_serials(serial_numbers),
				balances: bounded_quantities(quantities),
				owner: token_owner,
			}));
		});
	}

	#[test]
	fn mint_with_duplicate_serial_numbers_work() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_owner = bob();
			let (collection_id, serial_number) =
				create_test_token(collection_owner, collection_owner, 0);
			let serial_numbers: Vec<SerialNumber> =
				vec![serial_number, serial_number, serial_number, serial_number];
			let quantities: Vec<Balance> = vec![1, 50, 3000, 10000];
			let sum = quantities.iter().sum::<u128>();

			// Mint the quantities to the token_owner for each serial
			assert_ok!(Sft::mint(
				Some(collection_owner).into(),
				collection_id,
				bounded_combined(serial_numbers.clone(), quantities.clone()),
				Some(token_owner.clone()),
			));

			let token_info = TokenInfo::<Test>::get((collection_id, serial_number)).unwrap();
			assert_eq!(token_info.free_balance_of(&token_owner), sum);
			assert_eq!(token_info.token_issuance, sum);

			// Event emitted
			System::assert_last_event(RuntimeEvent::Sft(crate::Event::Mint {
				collection_id,
				serial_numbers: bounded_serials(serial_numbers),
				balances: bounded_quantities(quantities),
				owner: token_owner,
			}));
		});
	}

	#[test]
	fn mint_no_collection_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let collection_id = 0;
			let serial_number = 0;

			// Collection doesn't exist
			assert_noop!(
				Sft::mint(
					Some(collection_owner).into(),
					collection_id,
					bounded_combined(vec![serial_number], vec![100]),
					None,
				),
				Error::<Test>::NoCollectionFound
			);
		});
	}

	#[test]
	fn mint_not_collection_owner_public_mint_disabled_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_id = create_test_token(collection_owner, collection_owner, 0);
			let (collection_id, serial_number) = token_id;

			// bob is not collection owner
			assert_noop!(
				Sft::mint(
					Some(bob()).into(),
					collection_id,
					bounded_combined(vec![serial_number], vec![100]),
					None,
				),
				Error::<Test>::PublicMintDisabled
			);
		});
	}

	#[test]
	fn mint_invalid_quantity_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_id = create_test_token(collection_owner, collection_owner, 0);
			let (collection_id, serial_number) = token_id;

			// mint into serial number twice, second one with 0
			// This ensures the storage isn't changed if the second serial fails
			assert_noop!(
				Sft::mint(
					Some(collection_owner).into(),
					collection_id,
					bounded_combined(vec![serial_number, serial_number], vec![100, 0]),
					None,
				),
				Error::<Test>::InvalidQuantity
			);
		});
	}

	#[test]
	fn mint_invalid_serial_number_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_id = create_test_token(collection_owner, collection_owner, 0);
			let (collection_id, serial_number) = token_id;

			// Second serial number does not exist so should fail
			assert_noop!(
				Sft::mint(
					Some(collection_owner).into(),
					collection_id,
					bounded_combined(vec![serial_number, 12], vec![100, 10]),
					None,
				),
				Error::<Test>::NoToken
			);
		});
	}

	#[test]
	fn mint_over_u128_max_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let collection_id = create_test_collection(collection_owner);
			let initial_issuance = u128::MAX;

			// mint u128::MAX tokens
			assert_ok!(Sft::create_token(
				Some(collection_owner).into(),
				collection_id,
				bounded_string("my-token"),
				initial_issuance,
				None,
				None,
			));
			let serial_number = 0;

			// Check balance is correct
			let token_info = TokenInfo::<Test>::get((collection_id, serial_number)).unwrap();
			assert_eq!(token_info.free_balance_of(&collection_owner), initial_issuance);

			// Mint any more should fail
			assert_noop!(
				Sft::mint(
					Some(collection_owner).into(),
					collection_id,
					bounded_combined(vec![serial_number], vec![1]),
					None,
				),
				Error::<Test>::Overflow
			);
		});
	}

	#[test]
	fn mint_over_max_issuance_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let collection_id = create_test_collection(collection_owner);
			let max_issuance = 100;

			assert_ok!(Sft::create_token(
				Some(collection_owner).into(),
				collection_id,
				bounded_string("my-token"),
				0,
				Some(max_issuance),
				None,
			));
			let serial_number = 0;

			// Mint up to max issuance should pass
			assert_ok!(Sft::mint(
				Some(collection_owner).into(),
				collection_id,
				bounded_combined(vec![serial_number], vec![max_issuance]),
				None,
			));

			// Check balance is correct
			let token_info = TokenInfo::<Test>::get((collection_id, serial_number)).unwrap();
			assert_eq!(token_info.free_balance_of(&collection_owner), max_issuance);

			// Mint any more should fail
			assert_noop!(
				Sft::mint(
					Some(collection_owner).into(),
					collection_id,
					bounded_combined(vec![serial_number], vec![1]),
					None,
				),
				Error::<Test>::MaxIssuanceReached
			);
		});
	}

	#[test]
	fn mint_over_max_owners_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_owner = bob();
			let token_id = create_test_token(collection_owner, collection_owner, 0);
			let (collection_id, serial_number) = token_id;
			let max_owners = <Test as crate::Config>::MaxOwnersPerSftToken::get();

			// Mint some tokens up to max owners per token
			for i in 0..max_owners {
				let owner = create_account((i + 10) as u64);
				assert_ok!(Sft::mint(
					Some(collection_owner).into(),
					collection_id,
					bounded_combined(vec![serial_number], vec![1]),
					Some(owner.clone()),
				));
			}

			// Minting to a new owner will now fail
			assert_noop!(
				Sft::mint(
					Some(collection_owner).into(),
					collection_id,
					bounded_combined(vec![serial_number], vec![1]),
					Some(token_owner.clone()),
				),
				Error::<Test>::MaxOwnersReached
			);
		});
	}
}

mod transfer {
	use super::*;

	#[test]
	fn transfer_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_owner = bob();
			let initial_issuance = 1000;
			let token_id = create_test_token(collection_owner, token_owner, initial_issuance);
			let (collection_id, serial_number) = token_id;
			let quantity = 460;
			let new_owner = charlie();

			// Perform transfer
			assert_ok!(Sft::transfer(
				Some(token_owner.clone()).into(),
				collection_id,
				bounded_combined(vec![serial_number], vec![quantity]),
				new_owner.clone(),
			));

			// Get updated token_info
			let token_info = TokenInfo::<Test>::get(token_id).unwrap();

			// free balance of original owner and new owner should be updated
			assert_eq!(token_info.free_balance_of(&token_owner), initial_issuance - quantity);
			assert_eq!(token_info.free_balance_of(&new_owner), quantity);

			// Owned tokens is correct
			let expected_owned_tokens = create_owned_tokens(vec![
				(token_owner.clone(), initial_issuance - quantity),
				(new_owner.clone(), quantity),
			]);
			assert_eq!(token_info.owned_tokens, expected_owned_tokens);

			// token_issuance still the same
			assert_eq!(token_info.token_issuance, initial_issuance);

			// Event emitted
			System::assert_last_event(RuntimeEvent::Sft(crate::Event::Transfer {
				previous_owner: token_owner,
				collection_id,
				serial_numbers: bounded_serials(vec![serial_number]),
				balances: bounded_quantities(vec![quantity]),
				new_owner,
			}));
		});
	}

	#[test]
	fn transfer_multiple_tokens_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_owner = bob();
			let collection_id = create_test_collection(collection_owner);
			let serial_numbers: Vec<SerialNumber> = vec![0, 1, 2, 3, 4, 5, 6];
			let quantities: Vec<Balance> = vec![1000, 2000, 3000, 4000, 5000, 6000, 7000];
			let initial_issuance = 10_000;

			// Create each token with initial_issuance = 10,000
			for _ in serial_numbers.iter() {
				assert_ok!(Sft::create_token(
					Some(collection_owner).into(),
					collection_id,
					bounded_string("my-token"),
					initial_issuance,
					None,
					None,
				));
			}

			// Transfer the quantities to the token_owner for each serial
			assert_ok!(Sft::transfer(
				Some(collection_owner).into(),
				collection_id,
				bounded_combined(serial_numbers.clone(), quantities.clone()),
				token_owner.clone()
			));

			// Check each token has the correct free balance for both accounts
			for (serial_number, quantity) in serial_numbers.iter().zip(quantities.iter()) {
				let token_id = (collection_id, *serial_number);
				let token_info = TokenInfo::<Test>::get(token_id).unwrap();
				assert_eq!(token_info.free_balance_of(&token_owner), *quantity);
				assert_eq!(
					token_info.free_balance_of(&collection_owner),
					initial_issuance - *quantity
				);
			}

			// Event emitted
			System::assert_last_event(RuntimeEvent::Sft(crate::Event::Transfer {
				previous_owner: collection_owner,
				collection_id,
				serial_numbers: bounded_serials(serial_numbers),
				balances: bounded_quantities(quantities),
				new_owner: token_owner,
			}));
		});
	}

	#[test]
	fn transfer_entire_balance_clears_storage() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let initial_issuance = 1000;
			let token_id = create_test_token(collection_owner, collection_owner, initial_issuance);
			let (collection_id, serial_number) = token_id;
			let new_owner = bob();

			// Perform transfer
			assert_ok!(Sft::transfer(
				Some(collection_owner.clone()).into(),
				collection_id,
				bounded_combined(vec![serial_number], vec![initial_issuance]),
				new_owner.clone(),
			));

			// Get updated token_info
			let token_info = TokenInfo::<Test>::get(token_id).unwrap();

			// free balance of original owner and new owner should be updated
			assert_eq!(token_info.free_balance_of(&collection_owner), 0);
			assert_eq!(token_info.free_balance_of(&new_owner), initial_issuance);

			// Owned tokens is correct, the collection_owner should be fully removed
			let expected_owned_tokens =
				create_owned_tokens(vec![(new_owner.clone(), initial_issuance)]);
			assert_eq!(token_info.owned_tokens, expected_owned_tokens);
		});
	}

	#[test]
	fn transfer_insufficient_balance_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let new_owner = bob();
			let initial_issuance = 1000;
			let token_id = create_test_token(collection_owner, collection_owner, initial_issuance);
			let (collection_id, serial_number) = token_id;

			// Perform transfer
			assert_noop!(
				Sft::transfer(
					Some(collection_owner.clone()).into(),
					collection_id,
					bounded_combined(vec![serial_number], vec![initial_issuance + 1]),
					new_owner.clone(),
				),
				Error::<Test>::InsufficientBalance
			);
		});
	}

	#[test]
	fn transfer_multiple_insufficient_balance_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let new_owner = bob();
			let initial_issuance = 1000;
			let initial_issuance_2 = 2000;
			let token_id = create_test_token(collection_owner, collection_owner, initial_issuance);
			let (collection_id, serial_number) = token_id;

			// Create another token
			assert_ok!(Sft::create_token(
				Some(collection_owner).into(),
				collection_id,
				bounded_string("my-token"),
				initial_issuance_2,
				None,
				None,
			));
			let serial_number_2 = 1;

			// Perform transfer but second serial has insufficient balance
			assert_noop!(
				Sft::transfer(
					Some(collection_owner.clone()).into(),
					collection_id,
					bounded_combined(
						vec![serial_number, serial_number_2],
						vec![1, initial_issuance_2 + 1]
					),
					new_owner.clone(),
				),
				Error::<Test>::InsufficientBalance
			);
		});
	}

	#[test]
	fn transfer_invalid_quantity_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let new_owner = bob();
			let token_id = create_test_token(collection_owner, collection_owner, 1000);
			let (collection_id, serial_number) = token_id;

			// transfer into serial number twice, second one with 0
			// This ensures the storage isn't changed if the second serial fails
			assert_noop!(
				Sft::transfer(
					Some(collection_owner).into(),
					collection_id,
					bounded_combined(vec![serial_number, serial_number], vec![100, 0]),
					new_owner,
				),
				Error::<Test>::InvalidQuantity
			);
		});
	}

	#[test]
	fn transfer_invalid_serial_number_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let new_owner = bob();
			let token_id = create_test_token(collection_owner, collection_owner, 1000);
			let (collection_id, serial_number) = token_id;

			// Second serial number does not exist so should fail
			assert_noop!(
				Sft::transfer(
					Some(collection_owner).into(),
					collection_id,
					bounded_combined(vec![serial_number, 12], vec![100, 10]),
					new_owner,
				),
				Error::<Test>::NoToken
			);
		});
	}

	#[test]
	fn transfer_new_owner_is_signer_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_id = create_test_token(collection_owner, collection_owner, 1000);
			let (collection_id, serial_number) = token_id;

			// Second serial number does not exist so should fail
			assert_noop!(
				Sft::transfer(
					Some(collection_owner).into(),
					collection_id,
					bounded_combined(vec![serial_number, 12], vec![100, 10]),
					collection_owner,
				),
				Error::<Test>::InvalidNewOwner
			);
		});
	}
}

mod burn {
	use super::*;

	#[test]
	fn burn_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let initial_issuance = 1000;
			let token_id = create_test_token(collection_owner, collection_owner, initial_issuance);
			let (collection_id, serial_number) = token_id;

			// Burn 100 tokens
			let burn_amount = 100;
			assert_ok!(Sft::burn(
				Some(collection_owner.clone()).into(),
				collection_id,
				bounded_combined(vec![serial_number], vec![burn_amount]),
			));

			// Check token info
			let token_info = TokenInfo::<Test>::get(token_id).unwrap();
			assert_eq!(
				token_info.free_balance_of(&collection_owner),
				initial_issuance - burn_amount
			);
			// Total issuance is correct
			assert_eq!(token_info.token_issuance, initial_issuance - burn_amount);

			// Owned tokens is correct, the collection_owner should be fully removed
			let expected_owned_tokens = create_owned_tokens(vec![(
				collection_owner.clone(),
				initial_issuance - burn_amount,
			)]);
			assert_eq!(token_info.owned_tokens, expected_owned_tokens);

			// Event emitted
			System::assert_last_event(RuntimeEvent::Sft(crate::Event::Burn {
				collection_id,
				serial_numbers: bounded_serials(vec![serial_number]),
				balances: bounded_quantities(vec![burn_amount]),
				owner: collection_owner,
			}));
		});
	}

	#[test]
	fn burn_multiple_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let initial_issuance = 1000;
			let initial_issuance_2 = 3000;
			let token_id = create_test_token(collection_owner, collection_owner, initial_issuance);
			let (collection_id, serial_number) = token_id;

			// Create another token
			assert_ok!(Sft::create_token(
				Some(collection_owner).into(),
				collection_id,
				bounded_string("my-token"),
				initial_issuance_2,
				None,
				None,
			));
			let serial_number_2 = 1;

			// Burn 100 tokens
			let burn_amount = 100;
			assert_ok!(Sft::burn(
				Some(collection_owner.clone()).into(),
				collection_id,
				bounded_combined(
					vec![serial_number, serial_number_2],
					vec![burn_amount, burn_amount]
				),
			));

			// Check token info
			let token_info = TokenInfo::<Test>::get(token_id).unwrap();
			assert_eq!(
				token_info.free_balance_of(&collection_owner),
				initial_issuance - burn_amount
			);

			// Check token info for second token
			let token_info = TokenInfo::<Test>::get((collection_id, serial_number_2)).unwrap();
			assert_eq!(
				token_info.free_balance_of(&collection_owner),
				initial_issuance_2 - burn_amount
			);
		});
	}

	#[test]
	fn burn_insufficient_balance_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let initial_issuance = 1000;
			let token_id = create_test_token(collection_owner, collection_owner, initial_issuance);
			let (collection_id, serial_number) = token_id;

			// Burn initial issuance + 1 tokens
			assert_noop!(
				Sft::burn(
					Some(collection_owner.clone()).into(),
					collection_id,
					bounded_combined(vec![serial_number, serial_number], vec![initial_issuance, 1]),
				),
				Error::<Test>::InsufficientBalance
			);

			// Bob can't burn anything
			assert_noop!(
				Sft::burn(
					Some(bob()).into(),
					collection_id,
					bounded_combined(vec![serial_number], vec![1]),
				),
				Error::<Test>::InsufficientBalance
			);
		});
	}

	#[test]
	fn burn_invalid_serial_number_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let initial_issuance = 1000;
			let token_id = create_test_token(collection_owner, collection_owner, initial_issuance);
			let (collection_id, serial_number) = token_id;

			// Burn 100 tokens from serial 12 which doesn't exist
			let burn_amount = 100;
			assert_noop!(
				Sft::burn(
					Some(collection_owner.clone()).into(),
					collection_id,
					bounded_combined(vec![serial_number, 12], vec![burn_amount, burn_amount]),
				),
				Error::<Test>::NoToken
			);
		});
	}

	#[test]
	fn burn_invalid_quantity_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let initial_issuance = 1000;
			let token_id = create_test_token(collection_owner, collection_owner, initial_issuance);
			let (collection_id, serial_number) = token_id;

			// Burn 100 tokens
			assert_noop!(
				Sft::burn(
					Some(collection_owner.clone()).into(),
					collection_id,
					bounded_combined(vec![serial_number], vec![0]),
				),
				Error::<Test>::InvalidQuantity
			);
		});
	}
}

mod set_owner {
	use crate::{
		mock::{Sft, Test, TestExt},
		tests::{alice, bob, create_test_collection},
		Error, SftCollectionInfo,
	};

	use frame_support::{assert_noop, assert_ok};

	#[test]
	fn transfers_ownership() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let new_owner = bob();
			let collection_id = create_test_collection(collection_owner);

			assert_ok!(Sft::set_owner(Some(collection_owner).into(), collection_id, new_owner));

			let collection = SftCollectionInfo::<Test>::get(collection_id).unwrap();

			assert_eq!(collection.collection_owner, new_owner)
		});
	}

	#[test]
	fn cannot_transfer_ownership_if_not_owner() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let not_owner = bob();
			let collection_id = create_test_collection(collection_owner);

			assert_noop!(
				Sft::set_owner(Some(not_owner).into(), collection_id, collection_owner),
				Error::<Test>::NotCollectionOwner
			);
		});
	}

	#[test]
	fn errors_if_no_collection() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let other_account = bob();

			assert_noop!(
				Sft::set_owner(Some(collection_owner).into(), 1, other_account),
				Error::<Test>::NoCollectionFound
			);
		});
	}
}

mod set_max_issuance {
	use super::*;

	#[test]
	fn set_max_issuance_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_id = create_test_token(collection_owner, collection_owner, 1000);
			let new_max_issuance = 2000;

			// Set max issuance
			assert_ok!(Sft::set_max_issuance(
				Some(collection_owner).into(),
				token_id,
				new_max_issuance
			));

			// Max issuance is correct
			let token_info = TokenInfo::<Test>::get(token_id).unwrap();
			assert_eq!(token_info.max_issuance.unwrap(), new_max_issuance);
		});
	}

	#[test]
	fn set_max_issuance_not_collection_owner_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_id = create_test_token(collection_owner, collection_owner, 1000);
			let new_max_issuance = 2000;

			// Set max issuance
			assert_noop!(
				Sft::set_max_issuance(Some(bob()).into(), token_id, new_max_issuance),
				Error::<Test>::NotCollectionOwner
			);
		});
	}

	#[test]
	fn set_max_issuance_invalid_token_id_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_id = create_test_token(collection_owner, collection_owner, 1000);
			let new_max_issuance = 2000;

			// Set max issuance
			assert_noop!(
				Sft::set_max_issuance(
					Some(collection_owner).into(),
					(token_id.0, 1),
					new_max_issuance
				),
				Error::<Test>::NoToken
			);
		});
	}

	#[test]
	fn set_max_issuance_less_than_token_issuance_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_id = create_test_token(collection_owner, collection_owner, 1000);
			let new_max_issuance = 999;

			// Set max issuance but it is less than the current issuance
			assert_noop!(
				Sft::set_max_issuance(Some(collection_owner).into(), token_id, new_max_issuance),
				Error::<Test>::InvalidMaxIssuance
			);
		});
	}

	// Max issuance already set fails
	#[test]
	fn set_max_issuance_already_set_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_id = create_test_token(collection_owner, collection_owner, 1000);
			let new_max_issuance = 2000;

			// Set max issuance
			assert_ok!(Sft::set_max_issuance(
				Some(collection_owner).into(),
				token_id,
				new_max_issuance
			));

			// Set max issuance again
			assert_noop!(
				Sft::set_max_issuance(Some(collection_owner).into(), token_id, new_max_issuance),
				Error::<Test>::MaxIssuanceAlreadySet
			);
		});
	}
}

mod set_base_uri {
	use super::*;

	#[test]
	fn set_base_uri_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_id = create_test_token(collection_owner, collection_owner, 1000);

			let metadata_scheme =
				MetadataScheme::try_from(b"cool.new.scheme.com/metadata".as_slice()).unwrap();

			// Set base uri
			assert_ok!(Sft::set_base_uri(
				Some(collection_owner).into(),
				token_id.0,
				metadata_scheme.clone()
			));

			// Base uri is correct
			let collection_info = SftCollectionInfo::<Test>::get(token_id.0).unwrap();
			assert_eq!(collection_info.metadata_scheme, metadata_scheme);
		});
	}

	#[test]
	fn set_base_uri_not_collection_owner_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_id = create_test_token(collection_owner, collection_owner, 1000);
			let metadata_scheme =
				MetadataScheme::try_from(b"cool.new.scheme.com/metadata".as_slice()).unwrap();

			// Set base uri fails because not collection owner
			assert_noop!(
				Sft::set_base_uri(Some(bob()).into(), token_id.0, metadata_scheme.clone()),
				Error::<Test>::NotCollectionOwner
			);
		});
	}
}

mod set_name {
	use super::*;

	#[test]
	fn set_name_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_id = create_test_token(collection_owner, collection_owner, 1000);
			let collection_name = bounded_string("test-collection");

			// Set name
			assert_ok!(Sft::set_name(
				Some(collection_owner).into(),
				token_id.0,
				collection_name.clone()
			));

			// Name is correct
			let collection_info = SftCollectionInfo::<Test>::get(token_id.0).unwrap();
			assert_eq!(collection_info.collection_name, collection_name);
		});
	}

	#[test]
	fn set_name_no_collection_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let collection_id: u32 = 1;
			let new_name = bounded_string("yeet");

			// Call to unknown collection should fail
			assert_noop!(
				Sft::set_name(Some(collection_owner).into(), collection_id, new_name),
				Error::<Test>::NoCollectionFound
			);
		});
	}

	#[test]
	fn set_name_not_collection_owner_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_id = create_test_token(collection_owner, collection_owner, 1000);
			let collection_name = bounded_string("test-collection");

			// Set name fails because not collection owner
			assert_noop!(
				Sft::set_name(Some(bob()).into(), token_id.0, collection_name),
				Error::<Test>::NotCollectionOwner
			);
		});
	}

	#[test]
	fn set_name_invalid_name_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_id = create_test_token(collection_owner, collection_owner, 1000);

			// Calls with no name should fail
			assert_noop!(
				Sft::set_name(Some(collection_owner).into(), token_id.0, bounded_string("")),
				Error::<Test>::NameInvalid
			);

			// non UTF-8 chars
			assert_noop!(
				Sft::set_name(
					Some(collection_owner).into(),
					token_id.0,
					BoundedVec::truncate_from(vec![0xfe, 0xff])
				),
				Error::<Test>::NameInvalid
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
			let token_id = create_test_token(collection_owner, collection_owner, 1000);
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(collection_owner, Permill::one())]),
			};

			assert_ok!(Sft::set_royalties_schedule(
				Some(collection_owner).into(),
				token_id.0,
				royalties_schedule.clone()
			));

			let collection_info = SftCollectionInfo::<Test>::get(token_id.0).unwrap();

			// Storage updated
			assert_eq!(collection_info.royalties_schedule.unwrap(), royalties_schedule);
		});
	}

	#[test]
	fn set_royalties_no_collection_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(11);
			let collection_id = 1;
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(collection_owner, Permill::one())]),
			};

			// Call to unknown collection should fail
			assert_noop!(
				Sft::set_royalties_schedule(
					Some(collection_owner).into(),
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
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(collection_owner, Permill::one())]),
			};
			let token_id = create_test_token(collection_owner, collection_owner, 1000);

			// Set royalties schedule fails because not collection owner
			assert_noop!(
				Sft::set_royalties_schedule(
					Some(bob()).into(),
					token_id.0,
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
			let token_id = create_test_token(collection_owner, collection_owner, 8100);

			// Too big royalties should fail
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![
					(create_account(3), Permill::from_float(1.2)),
					(create_account(4), Permill::from_float(3.3)),
				]),
			};

			// Calls with invalid royalties should fail
			assert_noop!(
				Sft::set_royalties_schedule(
					Some(collection_owner).into(),
					token_id.0,
					royalties_schedule.clone()
				),
				Error::<Test>::RoyaltiesInvalid
			);
		});
	}
}

mod set_mint_fee {
	use super::*;
	use crate::{Event, PublicMintInfo};
	use pallet_nft::PublicMintInformation;
	use seed_primitives::AssetId;

	#[test]
	fn set_mint_fee_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			// let collection_id = setup_collection(collection_owner);
			let collection_id = create_test_collection(collection_owner);
			let pricing_details: (AssetId, Balance) = (1, 100);

			assert_ok!(Sft::set_mint_fee(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				Some(pricing_details)
			));

			let expected_mint_info =
				PublicMintInformation { enabled: false, pricing_details: Some(pricing_details) };
			assert_eq!(PublicMintInfo::<Test>::get(collection_id).unwrap(), expected_mint_info);

			// Setting to different value works
			let pricing_details: (AssetId, Balance) = (2, 234);

			assert_ok!(Sft::set_mint_fee(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				Some(pricing_details)
			));

			let expected_mint_info =
				PublicMintInformation { enabled: false, pricing_details: Some(pricing_details) };
			assert_eq!(PublicMintInfo::<Test>::get(collection_id).unwrap(), expected_mint_info);

			// Setting to None removes from storage
			assert_ok!(Sft::set_mint_fee(
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
			// let collection_id = setup_collection(collection_owner);
			let collection_id = create_test_collection(collection_owner);
			let pricing_details: (AssetId, Balance) = (1, 100);

			// Toggle mint should set enabled to true
			assert_ok!(Sft::toggle_public_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				true
			));

			// Set mint price should update pricing details but keep enabled as true
			assert_ok!(Sft::set_mint_fee(
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
			// let collection_id = setup_collection(collection_owner);
			let collection_id = create_test_collection(collection_owner);
			let pricing_details: (AssetId, Balance) = (1, 100);

			assert_ok!(Sft::set_mint_fee(
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
			let collection_id = create_test_collection(collection_owner);
			let pricing_details: (AssetId, Balance) = (1, 100);
			let bobby = create_account(11);

			assert_noop!(
				Sft::set_mint_fee(
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
				Sft::set_mint_fee(
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
	use crate::{Event, PublicMintInfo};
	use pallet_nft::PublicMintInformation;
	use seed_primitives::AssetId;

	#[test]
	fn toggle_public_mint_works() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			// let collection_id = setup_collection(collection_owner);
			let collection_id = create_test_collection(collection_owner);
			let enabled = true;

			assert_ok!(Sft::toggle_public_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				enabled
			));

			assert_eq!(PublicMintInfo::<Test>::get(collection_id).unwrap().enabled, enabled);

			// Disable again should work and clear storage
			let enabled = false;
			assert_ok!(Sft::toggle_public_mint(
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
			// let collection_id = setup_collection(collection_owner);
			let collection_id = create_test_collection(collection_owner);
			let enabled = true;

			assert_ok!(Sft::toggle_public_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				enabled
			));

			assert!(has_event(Event::<Test>::PublicMintToggle { collection_id, enabled }));

			// Disable again should work and still throw event
			let enabled = false;
			assert_ok!(Sft::toggle_public_mint(
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
			// let collection_id = setup_collection(collection_owner);
			let collection_id = create_test_collection(collection_owner);
			let enabled = true;

			// Set up pricing details
			let pricing_details: (AssetId, Balance) = (2, 234);
			assert_ok!(Sft::set_mint_fee(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				Some(pricing_details)
			));

			// Toggle mint should set enabled to true but keep pricing_details in tact
			assert_ok!(Sft::toggle_public_mint(
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
	use crate::{
		mock::{AssetsExt, XRP_ASSET_ID},
		Event,
	};
	use frame_support::traits::fungibles::Inspect;
	use seed_primitives::AssetId;

	#[test]
	fn public_mint_should_let_user_mint() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			// let collection_id = setup_collection(collection_owner);
			let collection_id = create_test_collection(collection_owner);
			let minter = create_account(11);
			let max_issuance = 100;

			assert_ok!(Sft::create_token(
				Some(collection_owner).into(),
				collection_id,
				bounded_string("my-token"),
				0,
				Some(max_issuance),
				None,
			));
			let serial_number = 0;

			// Minter should not be able to mint token
			assert_noop!(
				Sft::mint(
					Some(minter).into(),
					collection_id,
					bounded_combined(vec![serial_number], vec![max_issuance]),
					None
				),
				Error::<Test>::PublicMintDisabled
			);

			// Enable public minting
			assert_ok!(Sft::toggle_public_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				true
			));

			let serial_numbers: Vec<SerialNumber> = vec![0, 1, 2, 3, 4, 5, 6];
			let quantities: Vec<Balance> = vec![1000, 2000, 3000, 4000, 5000, 6000, 7000];

			// Create each token with initial_issuance = 0
			for _ in serial_numbers.iter() {
				assert_ok!(Sft::create_token(
					Some(collection_owner).into(),
					collection_id,
					bounded_string("my-token"),
					0,
					None,
					None,
				));
			}

			// Mint the quantities to the token_owner for each serial
			assert_ok!(Sft::mint(
				Some(minter).into(),
				collection_id,
				bounded_combined(serial_numbers.clone(), quantities.clone()),
				None,
			));

			// Should emit event
			assert!(has_event(Event::<Test>::Mint {
				collection_id,
				serial_numbers: bounded_serials(serial_numbers.clone()),
				balances: bounded_quantities(quantities.clone()),
				owner: minter,
				// balances: Default::default()
			}));

			// Check that minter has 100 token
			for (serial_number, quantity) in serial_numbers.iter().zip(quantities.iter()) {
				let token_id = (collection_id, *serial_number);
				let token_info = TokenInfo::<Test>::get(token_id).unwrap();
				assert_eq!(token_info.free_balance_of(&minter), *quantity);
				assert_eq!(token_info.token_issuance, *quantity);
			}
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
				let collection_id = create_test_collection(collection_owner);
				let quantity = 100;
				let mint_price = 25;
				let payment_asset = XRP_ASSET_ID;

				// Set up pricing details
				let pricing_details: (AssetId, Balance) = (payment_asset, mint_price);
				assert_ok!(Sft::set_mint_fee(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					Some(pricing_details)
				));

				// Enable public minting
				assert_ok!(Sft::toggle_public_mint(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					true
				));
				let serial_numbers = 0;

				// Minter should be able to mint
				// assert_ok!(Sft::mint(Some(minter).into(), collection_id, quantity, None));
				// Mint the quantities to the token_owner for each serial
				assert_ok!(Sft::mint(
					Some(minter).into(),
					collection_id,
					bounded_combined(vec![serial_numbers], vec![quantity]),
					None,
				));
				// Check that minter has 100 token
				// assert_eq!(Nft::token_balance_of(&minter, collection_id), quantity);

				// Should emit both mint and payment event
				assert!(has_event(Event::<Test>::Mint {
					collection_id,
					serial_numbers: Default::default(),
					owner: minter,
					balances: Default::default()
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
				let collection_id = create_test_collection(collection_owner);
				let quantity = 1;
				let mint_price = 100;
				let payment_asset = XRP_ASSET_ID;

				// Set up pricing details
				let pricing_details: (AssetId, Balance) = (payment_asset, mint_price);
				assert_ok!(Sft::set_mint_fee(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					Some(pricing_details)
				));

				// Enable public minting
				assert_ok!(Sft::toggle_public_mint(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					true
				));

				// Minter doesn't have enough XRP to cover mint
				// assert_noop!(
				// 	Sft::mint(Some(minter).into(), collection_id, quantity, None),
				// 	pallet_assets::Error::<Test>::BalanceLow
				// );
				let serial_numbers = 0;
				assert_ok!(Sft::mint(
					Some(minter).into(),
					collection_id,
					bounded_combined(vec![serial_numbers], vec![quantity]),
					None,
				));
			});
	}

	#[test]
	fn public_mint_collection_owner_should_not_be_charged() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = create_account(10);
			let collection_id = create_test_collection(collection_owner);
			// let collection_id = setup_collection(collection_owner);
			let quantity = 1;
			let mint_price = 100000000;
			let payment_asset = XRP_ASSET_ID;
			let owner_balance_before =
				AssetsExt::reducible_balance(payment_asset, &collection_owner, false);

			// Set up pricing details
			let pricing_details: (AssetId, Balance) = (payment_asset, mint_price);
			assert_ok!(Sft::set_mint_fee(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				Some(pricing_details)
			));

			// Enable public minting
			assert_ok!(Sft::toggle_public_mint(
				RawOrigin::Signed(collection_owner).into(),
				collection_id,
				true
			));

			let serial_numbers = 0;
			assert_ok!(Sft::mint(
				Some(collection_owner).into(),
				collection_id,
				bounded_combined(vec![serial_numbers], vec![quantity]),
				None,
			));
			// Collection owner mints
			// assert_ok!(Sft::mint(Some(collection_owner).into(), collection_id, quantity, None));
			// Check that minter has 100 token
			// assert_eq!(Sft::token_balance_of(&collection_owner, collection_id), quantity);

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
				let collection_id = create_test_collection(collection_owner);
				let quantity: Balance = 3;
				let mint_price = 200;
				let payment_asset = XRP_ASSET_ID;

				let token_owner_balance_before =
					AssetsExt::reducible_balance(payment_asset, &token_owner, false);

				// Set up pricing details
				let pricing_details: (AssetId, Balance) = (payment_asset, mint_price);
				assert_ok!(Sft::set_mint_fee(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					Some(pricing_details)
				));
				// Enable public minting
				assert_ok!(Sft::toggle_public_mint(
					RawOrigin::Signed(collection_owner).into(),
					collection_id,
					true
				));
				let max_issuance = 100;

				assert_ok!(Sft::create_token(
					Some(minter).into(),
					collection_id,
					bounded_string("my-token"),
					0,
					Some(max_issuance),
					None,
				));
				let serial_number = 0;

				// Minter should be able to mint
				assert_ok!(Sft::mint(
					Some(minter).into(),
					collection_id,
					bounded_combined(vec![serial_number], vec![max_issuance]),
					None,
				));

				let token_id = (collection_id, serial_number);
				let token_info = TokenInfo::<Test>::get(token_id).unwrap();
				assert_eq!(token_info.free_balance_of(&minter), quantity);
				assert_eq!(token_info.free_balance_of(&token_owner), quantity);
				// assert_eq!(token_info.token_issuance, *quantity);
				// Check that token_owner has tokens, but minter has none

				// Should emit both mint and payment event
				assert!(has_event(Event::<Test>::Mint {
					collection_id,
					serial_numbers: Default::default(),
					owner: token_owner,
					balances: Default::default()
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
