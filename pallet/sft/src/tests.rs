/* Copyright 2019-2021 Centrality Investments Limited
 *
 * Licensed under the LGPL, Version 3.0 (the "License");
 * you may not use this file except in compliance with the License.
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * You may obtain a copy of the License at the root of this project source code,
 * or at:
 *     https://centrality.ai/licenses/gplv3.txt
 *     https://centrality.ai/licenses/lgplv3.txt
 */

use crate::{
	mock::*, Config, Error, SftCollectionInfo, SftCollectionInformation, SftTokenBalance,
	SftTokenInformation, TokenInfo,
};
use frame_support::{assert_noop, assert_ok};
use seed_primitives::{
	Balance, CollectionUuid, MetadataScheme, OriginChain, RoyaltiesSchedule, SerialNumber, TokenId,
};
use sp_core::H160;
use sp_runtime::{BoundedVec, Permill};

/// Helper function to create a collection used for tests
/// Returns the collectionUuid
pub fn create_test_collection(owner: <Test as frame_system::Config>::AccountId) -> CollectionUuid {
	let collection_uuid = next_collection_uuid();
	let collection_name = bounded_string("test-collection");
	let metadata_scheme = MetadataScheme::Https(b"example.com/metadata".to_vec());

	assert_ok!(Sft::create_sft_collection(
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

	(collection_id, 0)
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

mod create_sft_collection {
	use super::*;

	#[test]
	fn create_sft_collection_works() {
		TestExt::default().build().execute_with(|| {
			// CollectionId stored in the NFT pallet, get here to check it is incremented
			// properly after we create a collection
			let nft_collection_id = pallet_nft::NextCollectionId::<Test>::get();
			// The actual collection_uuid (Different to the NextCollectionId in NFT pallet
			let collection_uuid = next_collection_uuid();
			let caller = alice();
			let collection_name = bounded_string("test");
			let collection_owner = bob();
			let metadata_scheme = MetadataScheme::Https(b"example.com/metadata".to_vec());
			let royalties_schedule =
				RoyaltiesSchedule { entitlements: vec![(collection_owner, Permill::one())] };

			// Call works
			assert_ok!(Sft::create_sft_collection(
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
			System::assert_last_event(Event::Sft(crate::Event::CollectionCreate {
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
	fn create_sft_collection_no_specified_owner() {
		TestExt::default().build().execute_with(|| {
			let collection_uuid = next_collection_uuid();
			let caller = alice();
			let collection_name = bounded_string("test");
			let metadata_scheme = MetadataScheme::Https(b"example.com/metadata".to_vec());

			// Call works
			assert_ok!(Sft::create_sft_collection(
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
	fn create_sft_collection_invalid_collection_name_fails() {
		TestExt::default().build().execute_with(|| {
			let metadata_scheme = MetadataScheme::Https(b"example.com/metadata".to_vec());

			// Empty Collection Name
			let empty_collection_name = bounded_string("");
			assert_noop!(
				Sft::create_sft_collection(
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
				Sft::create_sft_collection(
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
	fn create_sft_collection_invalid_metadata_scheme_fails() {
		TestExt::default().build().execute_with(|| {
			// Empty MetadataScheme
			let empty_metadata_scheme = MetadataScheme::Https(b"".to_vec());
			assert_noop!(
				Sft::create_sft_collection(
					Some(alice()).into(),
					bounded_string("test-collection"),
					None,
					empty_metadata_scheme,
					None
				),
				Error::<Test>::InvalidMetadataPath
			);

			// Non utf-8 MetadataScheme
			let non_utf8_metadata_scheme = MetadataScheme::Https(vec![0xfe, 0xff]);
			assert_noop!(
				Sft::create_sft_collection(
					Some(alice()).into(),
					bounded_string("test-collection"),
					None,
					non_utf8_metadata_scheme,
					None
				),
				Error::<Test>::InvalidMetadataPath
			);
		});
	}

	#[test]
	fn create_sft_collection_invalid_royalties_schedule_fails() {
		TestExt::default().build().execute_with(|| {
			let metadata_scheme = MetadataScheme::Https(b"example.com/metadata".to_vec());

			// Empty RoyaltiesSchedule
			let empty_royalties_schedule = RoyaltiesSchedule { entitlements: vec![] };
			assert_noop!(
				Sft::create_sft_collection(
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
				entitlements: vec![
					(bob(), Permill::one()),
					(bob(), Permill::one()),
					(bob(), Permill::one()),
					(bob(), Permill::one()),
					(bob(), Permill::one()),
					(bob(), Permill::one()),
					(bob(), Permill::one()),
					(bob(), Permill::one()),
					(bob(), Permill::one()),
				],
			};
			assert_noop!(
				Sft::create_sft_collection(
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
				entitlements: vec![
					(bob(), Permill::from_parts(500_000)),
					(bob(), Permill::from_parts(500_001)),
				],
			};
			assert_noop!(
				Sft::create_sft_collection(
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
			System::assert_last_event(Event::Sft(crate::Event::TokenCreated {
				collection_id,
				serial_number: 0,
				initial_issuance,
				max_issuance: Some(max_issuance),
				token_name,
				token_owner,
			}));
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
			System::assert_last_event(Event::Sft(crate::Event::TokenCreated {
				collection_id,
				serial_number: 0,
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
			let metadata_scheme = MetadataScheme::Https(b"example.com/metadata".to_vec());

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

			// Sanity check, initial balance should be 0
			assert_eq!(TokenInfo::<Test>::get(token_id).unwrap().free_balance_of(&token_owner), 0);

			assert_ok!(Sft::mint(
				Some(collection_owner).into(),
				collection_id,
				bounded_serials(vec![serial_number]),
				bounded_quantities(vec![quantity]),
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
			System::assert_last_event(Event::Sft(crate::Event::Mint {
				collection_id,
				serial_numbers: bounded_serials(vec![serial_number]),
				quantities: bounded_quantities(vec![quantity]),
				owner: token_owner,
			}));

			// Mint some more to make sure the balance is added correctly to an existing owner
			let quantity2 = 1337;
			assert_ok!(Sft::mint(
				Some(collection_owner).into(),
				collection_id,
				bounded_serials(vec![serial_number]),
				bounded_quantities(vec![quantity2]),
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
			System::assert_last_event(Event::Sft(crate::Event::Mint {
				collection_id,
				serial_numbers: bounded_serials(vec![serial_number]),
				quantities: bounded_quantities(vec![quantity2]),
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
				bounded_serials(serial_numbers.clone()),
				bounded_quantities(quantities.clone()),
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
			System::assert_last_event(Event::Sft(crate::Event::Mint {
				collection_id,
				serial_numbers: bounded_serials(serial_numbers),
				quantities: bounded_quantities(quantities),
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
				bounded_serials(serial_numbers.clone()),
				bounded_quantities(quantities.clone()),
				Some(token_owner.clone()),
			));

			let token_info = TokenInfo::<Test>::get((collection_id, serial_number)).unwrap();
			assert_eq!(token_info.free_balance_of(&token_owner), sum);
			assert_eq!(token_info.token_issuance, sum);

			// Event emitted
			System::assert_last_event(Event::Sft(crate::Event::Mint {
				collection_id,
				serial_numbers: bounded_serials(serial_numbers),
				quantities: bounded_quantities(quantities),
				owner: token_owner,
			}));
		});
	}

	#[test]
	fn mint_different_input_lengths_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_id = create_test_token(collection_owner, collection_owner, 0);
			let (collection_id, serial_number) = token_id;
			let quantity = 1000;

			// Serial Numbers longer than quantity
			assert_noop!(
				Sft::mint(
					Some(collection_owner).into(),
					collection_id,
					bounded_serials(vec![serial_number, serial_number]),
					bounded_quantities(vec![quantity]),
					None,
				),
				Error::<Test>::MismatchedInputLength
			);

			// Quantity longer than serial Numbers
			assert_noop!(
				Sft::mint(
					Some(collection_owner).into(),
					collection_id,
					bounded_serials(vec![serial_number]),
					bounded_quantities(vec![quantity, quantity]),
					None,
				),
				Error::<Test>::MismatchedInputLength
			);

			// Empty serial numbers
			assert_noop!(
				Sft::mint(
					Some(collection_owner).into(),
					collection_id,
					bounded_serials(vec![]),
					bounded_quantities(vec![]),
					None,
				),
				Error::<Test>::NoToken
			);
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
					bounded_serials(vec![serial_number]),
					bounded_quantities(vec![100]),
					None,
				),
				Error::<Test>::NoCollectionFound
			);
		});
	}

	#[test]
	fn mint_not_collection_owner_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_id = create_test_token(collection_owner, collection_owner, 0);
			let (collection_id, serial_number) = token_id;

			// bob is not collection owner
			assert_noop!(
				Sft::mint(
					Some(bob()).into(),
					collection_id,
					bounded_serials(vec![serial_number]),
					bounded_quantities(vec![100]),
					None,
				),
				Error::<Test>::NotCollectionOwner
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
					bounded_serials(vec![serial_number, serial_number]),
					bounded_quantities(vec![100, 0]),
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
					bounded_serials(vec![serial_number, 12]),
					bounded_quantities(vec![100, 10]),
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
					bounded_serials(vec![serial_number]),
					bounded_quantities(vec![1]),
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
				bounded_serials(vec![serial_number]),
				bounded_quantities(vec![max_issuance]),
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
					bounded_serials(vec![serial_number]),
					bounded_quantities(vec![1]),
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
					bounded_serials(vec![serial_number]),
					bounded_quantities(vec![1]),
					Some(owner.clone()),
				));
			}

			// Minting to a new owner will now fail
			assert_noop!(
				Sft::mint(
					Some(collection_owner).into(),
					collection_id,
					bounded_serials(vec![serial_number]),
					bounded_quantities(vec![1]),
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

			// Sanity check of initial balances
			assert_eq!(
				TokenInfo::<Test>::get(token_id).unwrap().free_balance_of(&token_owner),
				initial_issuance
			);
			assert_eq!(TokenInfo::<Test>::get(token_id).unwrap().free_balance_of(&new_owner), 0);

			// Perform transfer
			assert_ok!(Sft::transfer(
				Some(token_owner.clone()).into(),
				collection_id,
				bounded_serials(vec![serial_number]),
				bounded_quantities(vec![quantity]),
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
			System::assert_last_event(Event::Sft(crate::Event::Transfer {
				previous_owner: token_owner,
				collection_id,
				serial_numbers: bounded_serials(vec![serial_number]),
				quantities: bounded_quantities(vec![quantity]),
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
				bounded_serials(serial_numbers.clone()),
				bounded_quantities(quantities.clone()),
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
			System::assert_last_event(Event::Sft(crate::Event::Transfer {
				previous_owner: collection_owner,
				collection_id,
				serial_numbers: bounded_serials(serial_numbers),
				quantities: bounded_quantities(quantities),
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
				bounded_serials(vec![serial_number]),
				bounded_quantities(vec![initial_issuance]),
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
					bounded_serials(vec![serial_number]),
					bounded_quantities(vec![initial_issuance + 1]),
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
					bounded_serials(vec![serial_number, serial_number_2]),
					bounded_quantities(vec![1, initial_issuance_2 + 1]),
					new_owner.clone(),
				),
				Error::<Test>::InsufficientBalance
			);
		});
	}

	#[test]
	fn transfer_different_input_lengths_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let token_id = create_test_token(collection_owner, collection_owner, 0);
			let (collection_id, serial_number) = token_id;
			let quantity = 1000;
			let new_owner = bob();

			// Serial Numbers longer than quantity
			assert_noop!(
				Sft::transfer(
					Some(collection_owner).into(),
					collection_id,
					bounded_serials(vec![serial_number, serial_number]),
					bounded_quantities(vec![quantity]),
					new_owner,
				),
				Error::<Test>::MismatchedInputLength
			);

			// Quantity longer than serial Numbers
			assert_noop!(
				Sft::transfer(
					Some(collection_owner).into(),
					collection_id,
					bounded_serials(vec![serial_number]),
					bounded_quantities(vec![quantity, quantity]),
					new_owner,
				),
				Error::<Test>::MismatchedInputLength
			);

			// Empty serial numbers
			assert_noop!(
				Sft::transfer(
					Some(collection_owner).into(),
					collection_id,
					bounded_serials(vec![]),
					bounded_quantities(vec![]),
					new_owner,
				),
				Error::<Test>::NoToken
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
					bounded_serials(vec![serial_number, serial_number]),
					bounded_quantities(vec![100, 0]),
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
					bounded_serials(vec![serial_number, 12]),
					bounded_quantities(vec![100, 10]),
					new_owner,
				),
				Error::<Test>::NoToken
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

			// Sanity check
			let token_info = TokenInfo::<Test>::get(token_id).unwrap();
			assert_eq!(token_info.free_balance_of(&collection_owner), initial_issuance);

			// Burn 100 tokens
			let burn_amount = 100;
			assert_ok!(Sft::burn(
				Some(collection_owner.clone()).into(),
				collection_id,
				bounded_serials(vec![serial_number]),
				bounded_quantities(vec![burn_amount]),
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
			System::assert_last_event(Event::Sft(crate::Event::Burn {
				collection_id,
				serial_numbers: bounded_serials(vec![serial_number]),
				quantities: bounded_quantities(vec![burn_amount]),
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
				bounded_serials(vec![serial_number, serial_number_2]),
				bounded_quantities(vec![burn_amount, burn_amount]),
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
					bounded_serials(vec![serial_number, serial_number]),
					bounded_quantities(vec![initial_issuance, 1]),
				),
				Error::<Test>::InsufficientBalance
			);

			// Bob can't burn anything
			assert_noop!(
				Sft::burn(
					Some(bob()).into(),
					collection_id,
					bounded_serials(vec![serial_number]),
					bounded_quantities(vec![1]),
				),
				Error::<Test>::InsufficientBalance
			);
		});
	}

	#[test]
	fn burn_different_input_lengths_fails() {
		TestExt::default().build().execute_with(|| {
			let collection_owner = alice();
			let initial_issuance = 1000;
			let token_id = create_test_token(collection_owner, collection_owner, initial_issuance);
			let (collection_id, serial_number) = token_id;
			let burn_amount = 1;

			// Serial Numbers longer than quantity
			assert_noop!(
				Sft::burn(
					Some(collection_owner).into(),
					collection_id,
					bounded_serials(vec![serial_number, serial_number]),
					bounded_quantities(vec![burn_amount]),
				),
				Error::<Test>::MismatchedInputLength
			);

			// Quantity longer than serial Numbers
			assert_noop!(
				Sft::burn(
					Some(collection_owner).into(),
					collection_id,
					bounded_serials(vec![serial_number]),
					bounded_quantities(vec![burn_amount, burn_amount]),
				),
				Error::<Test>::MismatchedInputLength
			);

			// Empty serial numbers
			assert_noop!(
				Sft::burn(
					Some(collection_owner).into(),
					collection_id,
					bounded_serials(vec![]),
					bounded_quantities(vec![]),
				),
				Error::<Test>::NoToken
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
					bounded_serials(vec![serial_number, 12]),
					bounded_quantities(vec![burn_amount, burn_amount]),
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
			let burn_amount = 100;
			assert_noop!(
				Sft::burn(
					Some(collection_owner.clone()).into(),
					collection_id,
					bounded_serials(vec![serial_number]),
					bounded_quantities(vec![0]),
				),
				Error::<Test>::InvalidQuantity
			);
		});
	}
}
