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
		});
	}
}
