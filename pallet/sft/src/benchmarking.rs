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
//! SFT benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use crate::Pallet as Sft;
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, BoundedVec};
use frame_system::RawOrigin;
use sp_runtime::Permill;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

pub fn build_collection<T: Config>(caller: Option<T::AccountId>) -> CollectionUuid {
	let id = T::NFTExt::next_collection_uuid().expect("Failed to get next collection uuid");
	let caller = caller.unwrap_or_else(|| account::<T>("Alice"));
	let metadata_scheme = MetadataScheme::try_from(b"example.com/metadata/".as_slice()).unwrap();
	let collection_name = bounded_string::<T>("Collection");
	assert_ok!(Sft::<T>::create_collection(
		origin::<T>(&caller).into(),
		collection_name.clone(),
		None,
		metadata_scheme.clone(),
		None
	));

	id
}

/// Helper function to create a token
/// Returns the TokenId (CollectionId, SerialNumber)
pub fn build_token<T: Config>(caller: Option<T::AccountId>, initial_issuance: Balance) -> TokenId {
	let caller = caller.unwrap_or_else(|| account::<T>("Alice"));
	let collection_id = build_collection::<T>(Some(caller.clone()));
	let token_name = bounded_string::<T>("test-token");

	assert_ok!(Sft::<T>::create_token(
		origin::<T>(&caller).into(),
		collection_id,
		token_name,
		initial_issuance,
		None,
		None,
	));

	(collection_id, 0)
}

/// Helper function for creating the bounded (SerialNumbers, Balance) type
pub fn bounded_combined<T: Config>(
	serial_numbers: Vec<SerialNumber>,
	quantities: Vec<Balance>,
) -> BoundedVec<(SerialNumber, Balance), <T as Config>::MaxSerialsPerMint> {
	let combined: Vec<(SerialNumber, Balance)> =
		serial_numbers.into_iter().zip(quantities).collect();
	BoundedVec::truncate_from(combined)
}

/// Helper function for creating the collection name type
pub fn bounded_string<T: Config>(name: &str) -> BoundedVec<u8, <T as Config>::StringLimit> {
	BoundedVec::truncate_from(name.as_bytes().to_vec())
}

benchmarks! {
	create_collection {
		let id = T::NFTExt::next_collection_uuid().expect("Failed to get next collection uuid");
		let metadata = MetadataScheme::try_from(b"example.com/".as_slice()).unwrap();
	}: _(origin::<T>(&account::<T>("Alice")), bounded_string::<T>("Collection"), None, metadata, None)
	verify {
		assert!( SftCollectionInfo::<T>::get(id).is_some());
	}

	create_token {
		let id = build_collection::<T>(None);
		let initial_issuance = u128::MAX;
	}: _(origin::<T>(&account::<T>("Alice")), id, bounded_string::<T>("Token"), initial_issuance, None, None)
	verify {
		let token = TokenInfo::<T>::get((id, 0));
		assert!(token.is_some());
	}

	toggle_public_mint {
		let owner = account::<T>("Alice");
		let token_id = build_token::<T>(Some(owner.clone()), 0);
	}: _(origin::<T>(&account::<T>("Alice")), token_id, true)
	verify {
		let token = TokenInfo::<T>::get(token_id);
		assert!(token.is_some());
		let is_enabled = PublicMintInfo::<T>::get(token_id).unwrap().enabled;
		assert_eq!(is_enabled, true);
	}

	set_mint_fee {
		let owner = account::<T>("Alice");
		let token_id = build_token::<T>(Some(owner.clone()), 0);
		let pricing_details = Some((1, 100));
	}: _(origin::<T>(&account::<T>("Alice")), token_id, pricing_details)
	verify {
		let token = TokenInfo::<T>::get(token_id);
		assert!(token.is_some());
		let pricing_details = PublicMintInfo::<T>::get(token_id).unwrap().pricing_details;
		let expected_pricing_details = Some((1, 100));
		assert_eq!(pricing_details, expected_pricing_details);
	}

	mint {
		let owner = account::<T>("Alice");
		let (collection_id, serial_number) = build_token::<T>(Some(owner.clone()), 0);
		let serial_numbers = bounded_combined::<T>(vec![serial_number], vec![u128::MAX]);
	}: _(origin::<T>(&owner), collection_id, serial_numbers, None)
	verify {
		let token = TokenInfo::<T>::get((collection_id, serial_number));
		assert!(token.is_some());
		let token = token.unwrap();
		assert_eq!(token.token_issuance, u128::MAX);
	}

	transfer {
		let p in 1 .. (50);
		let owner = account::<T>("Alice");
		let (collection_id, serial_number) = build_token::<T>(Some(owner.clone()), u128::MAX);
		for i in 1..p {
			assert_ok!(Sft::<T>::create_token(
				origin::<T>(&owner).into(),
				collection_id,
				bounded_string::<T>("SFT Token"),
				u128::MAX,
				None,
				None,
			));
		}
		let serial_numbers: Vec<SerialNumber> = (0..p).collect();
		let serials_combined: Vec<(SerialNumber, Balance)> = serial_numbers.iter().map(|s| (*s, u128::MAX)).collect();
		let serial_numbers_bounded = BoundedVec::truncate_from(serials_combined);
	}: _(origin::<T>(&owner), collection_id, serial_numbers_bounded.clone(), account::<T>("Bob"))
	verify {
		for (serial_number, amount) in serial_numbers_bounded.into_inner() {
			let token = TokenInfo::<T>::get((collection_id, serial_number)).unwrap();
			assert_eq!(token.free_balance_of(&account::<T>("Alice")), 0);
			assert_eq!(token.free_balance_of(&account::<T>("Bob")), amount);
		}
	}

	burn {
		let owner = account::<T>("Alice");
		let initial_issuance = 1000;
		let (collection_id, serial_number) = build_token::<T>(Some(owner.clone()), initial_issuance);
		let serial_numbers = bounded_combined::<T>(vec![serial_number], vec![initial_issuance]);
	}: _(origin::<T>(&owner), collection_id, serial_numbers)
	verify {
		let token = TokenInfo::<T>::get((collection_id, serial_number));
		assert!(token.is_some());
		let token = token.unwrap();
		assert_eq!(token.token_issuance, 0);
	}

	set_owner {
		let owner = account::<T>("Alice");
		let collection_id = build_collection::<T>(Some(owner.clone()));
	}: _(origin::<T>(&owner), collection_id, account::<T>("Bob"))
	verify {
		let collection = SftCollectionInfo::<T>::get(collection_id);
		assert!(collection.is_some());
		let collection = collection.unwrap();
		assert_eq!(collection.collection_owner, account::<T>("Bob"));
	}

	set_max_issuance {
		let owner = account::<T>("Alice");
		let token_id = build_token::<T>(Some(owner.clone()), 0);
	}: _(origin::<T>(&owner), token_id, 32)
	verify {
		let token = TokenInfo::<T>::get(token_id);
		assert!(token.is_some());
		let token = token.unwrap();
		assert_eq!(token.max_issuance, Some(32));
	}

	set_base_uri {
		let owner = account::<T>("Alice");
		let id = build_collection::<T>(Some(owner.clone()));
		let metadata_scheme = MetadataScheme::try_from(b"example.com/changed/".as_slice()).unwrap();
	}: _(origin::<T>(&owner), id, metadata_scheme.clone())
	verify {
		let collection = SftCollectionInfo::<T>::get(id);
		assert!(collection.is_some());
		let collection = collection.unwrap();
		assert_eq!(collection.metadata_scheme, metadata_scheme);
	}

	set_name {
		let owner = account::<T>("Alice");
		let id = build_collection::<T>(Some(owner.clone()));
		let collection_name = bounded_string::<T>("Collection");
	}: _(origin::<T>(&owner), id, collection_name.clone())
	verify {
		let collection = SftCollectionInfo::<T>::get(id);
		assert!(collection.is_some());
		let collection = collection.unwrap();
		assert_eq!(collection.collection_name, collection_name);
	}

	set_royalties_schedule {
		let collection_owner = account::<T>("Alice");
		let id = build_collection::<T>(Some(collection_owner.clone()));
		let royalties_schedule = RoyaltiesSchedule {
			entitlements: BoundedVec::truncate_from(vec![(collection_owner.clone(), Permill::one())]),
		};
	}: _(origin::<T>(&collection_owner), id, royalties_schedule.clone())
	verify {
		let collection = SftCollectionInfo::<T>::get(id);
		assert!(collection.is_some());
		let collection = collection.unwrap();
		assert_eq!(collection.royalties_schedule, Some(royalties_schedule));
	}

	set_utility_flags {
		let collection_id = build_collection::<T>(None);
		let utility_flags = CollectionUtilityFlags {
			transferable: false,
			burnable: false,
			mintable: false,
		};
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, utility_flags)
	verify {
		assert_eq!(UtilityFlags::<T>::get(collection_id), utility_flags)
	}

	set_token_name {
		let owner = account::<T>("Alice");
		let token_id = build_token::<T>(Some(owner.clone()), 1);
		let token_name = bounded_string::<T>("Token");
	}: _(origin::<T>(&owner), token_id, token_name.clone())
	verify {
		let token = TokenInfo::<T>::get(token_id).unwrap();
		assert_eq!(token.token_name, token_name);
	}
}

impl_benchmark_test_suite!(
	Sft,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
