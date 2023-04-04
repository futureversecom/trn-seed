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

use crate::{mock::*, Config};
use frame_support::{assert_noop, assert_ok};
use seed_primitives::{CollectionUuid, MetadataScheme, RoyaltiesSchedule};
use sp_core::H160;
use sp_runtime::{BoundedVec, Permill};

// Helper functions for creating accounts from a u64 seed
pub fn create_account(seed: u64) -> <Test as frame_system::Config>::AccountId {
	<Test as frame_system::Config>::AccountId::from(H160::from_low_u64_be(seed))
}

// Common account Alice
pub fn alice() -> <Test as frame_system::Config>::AccountId {
	create_account(1)
}

// Common account Bob
pub fn bob() -> <Test as frame_system::Config>::AccountId {
	create_account(2)
}

// Helper function for creating the collection name type
pub fn bounded_string(name: &str) -> BoundedVec<u8, <Test as Config>::StringLimit> {
	BoundedVec::truncate_from(name.as_bytes().to_vec())
}

// Helper function to get the next collection Uuid from the NFT pallet
pub fn next_collection_uuid() -> CollectionUuid {
	<Test as Config>::NFTExt::next_collection_uuid().expect("Failed to get next collection uuid")
}

mod create_sft_collection {
	use super::*;
	use crate::{Error, SftCollectionInfo, SftCollectionInformation};
	use seed_primitives::OriginChain;

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
				name: collection_name.clone(),
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
				collection_uuid,
				collection_owner,
				metadata_scheme,
				name: collection_name.into_inner(),
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
				Error::<Test>::CollectionNameInvalid
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
				Error::<Test>::CollectionNameInvalid
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
