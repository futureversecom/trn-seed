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
use frame_support::{assert_ok, traits::Get, BoundedVec};
use frame_system::RawOrigin;
use seed_pallet_common::utils::TokenBurnAuthority;
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
	let collection_name = max_bounded_vec::<T::StringLimit>();
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
	let token_name = max_bounded_vec::<T::StringLimit>();

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

pub fn max_bounded_vec<B: Get<u32>>() -> BoundedVec<u8, B> {
	let v = vec![b'a'; B::get() as usize];
	BoundedVec::truncate_from(v)
}

/// Helper function to create and issue a token
/// Returns the TokenId (CollectionId, SerialNumber)
pub fn issue_token<T: Config>(owner: T::AccountId, receiver: T::AccountId) -> TokenId {
	let (collection_id, serial_number) = build_token::<T>(Some(owner.clone()), 0);

	assert_ok!(Sft::<T>::set_token_burn_authority(
		origin::<T>(&owner).into(),
		(collection_id, serial_number),
		TokenBurnAuthority::Both,
	));

	assert_ok!(Sft::<T>::issue_soulbound(
		origin::<T>(&owner).into(),
		collection_id,
		BoundedVec::try_from(vec![(serial_number, 1)]).unwrap(),
		receiver.clone(),
	));

	(collection_id, serial_number)
}

benchmarks! {
	create_collection {
		let id = T::NFTExt::next_collection_uuid().expect("Failed to get next collection uuid");
		let metadata = MetadataScheme::try_from(b"example.com/".as_slice()).unwrap();
		let collection_name = max_bounded_vec::<T::StringLimit>();
	}: _(origin::<T>(&account::<T>("Alice")), collection_name, None, metadata, None)
	verify {
		assert!(SftCollectionInfo::<T>::contains_key(id));
	}

	create_token {
		let id = build_collection::<T>(None);
		let initial_issuance = u128::MAX;
		let token_name = max_bounded_vec::<T::StringLimit>();
	}: _(origin::<T>(&account::<T>("Alice")), id, token_name, initial_issuance, None, None)
	verify {
		assert!(TokenInfo::<T>::contains_key((id, 0)));
	}

	toggle_public_mint {
		let owner = account::<T>("Alice");
		let token_id = build_token::<T>(Some(owner.clone()), 0);
	}: _(origin::<T>(&account::<T>("Alice")), token_id, true)
	verify {
		assert!(TokenInfo::<T>::contains_key(token_id));
		let is_enabled = PublicMintInfo::<T>::get(token_id).unwrap().enabled;
		assert_eq!(is_enabled, true);
	}

	set_mint_fee {
		let owner = account::<T>("Alice");
		let token_id = build_token::<T>(Some(owner.clone()), 0);
		let pricing_details = Some((1, 100));
	}: _(origin::<T>(&account::<T>("Alice")), token_id, pricing_details)
	verify {
		assert!(TokenInfo::<T>::contains_key(token_id));
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
		let token_name = max_bounded_vec::<T::StringLimit>();
		for i in 1..p {
			assert_ok!(Sft::<T>::create_token(
				origin::<T>(&owner).into(),
				collection_id,
				token_name.clone(),
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
		// benchmark string at max len, will be truncated in bounded_string
		let collection_name = max_bounded_vec::<T::StringLimit>();
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
		// benchmark string at max len, will be truncated in bounded_string
		let token_name = max_bounded_vec::<T::StringLimit>();
	}: _(origin::<T>(&owner), token_id, token_name.clone())
	verify {
		let token = TokenInfo::<T>::get(token_id).unwrap();
		assert_eq!(token.token_name, token_name);
	}

	set_token_transferable_flag {
		let owner = account::<T>("Alice");
		let token_id = build_token::<T>(Some(owner.clone()), 1);
	}: _(origin::<T>(&owner), token_id, true)
	verify {
		assert_eq!(TokenUtilityFlags::<T>::get(token_id).transferable, true);
	}

	set_token_burn_authority {
		let owner = account::<T>("Alice");
		let token_id = build_token::<T>(Some(owner.clone()), 0);
		let burn_authority = TokenBurnAuthority::Both;
	}: _(origin::<T>(&owner), token_id, TokenBurnAuthority::Both)
	verify {
		assert_eq!(TokenUtilityFlags::<T>::get(token_id).burn_authority, Some(burn_authority));
	}

	issue_soulbound {
		let p in 1 .. T::MaxSerialsPerMint::get();

		let owner = account::<T>("Alice");

		let mut tokens = vec![];

		let collection_id = build_collection::<T>(Some(owner.clone()));
		let token_name = max_bounded_vec::<T::StringLimit>();

		for serial_number in 0..p {
			assert_ok!(Sft::<T>::create_token(
				origin::<T>(&owner).into(),
				collection_id,
				token_name.clone(),
				0,
				None,
				None,
			));

			let serial_numbers = (serial_number, u128::MAX);

			tokens.push(serial_numbers);

			assert_ok!(Sft::<T>::set_token_burn_authority(
				origin::<T>(&owner).into(),
				(collection_id, serial_number),
				TokenBurnAuthority::Both,
			));
		}
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, BoundedVec::try_from(tokens).unwrap(), account::<T>("Bob"))
	verify {
		let pending_issuances =
			&PendingIssuances::<T>::get(collection_id).pending_issuances[0].1;

		assert_eq!(
			pending_issuances.len(),
			1,
		)
	}

	accept_soulbound_issuance {
		let owner = account::<T>("Alice");
		let receiver = account::<T>("Bob");

		let (collection_id, serial_number) = issue_token::<T>(owner.clone(), receiver.clone());
	}: _(origin::<T>(&receiver.clone()), collection_id, 0)
	verify {
		let token = TokenInfo::<T>::get((collection_id, serial_number)).unwrap();
		assert_eq!(token.free_balance_of(&receiver), 1);
	}

	burn_as_collection_owner {
		let owner = account::<T>("Alice");
		let receiver = account::<T>("Bob");

		let (collection_id, serial_number) = issue_token::<T>(owner.clone(), receiver.clone());

		assert_ok!(Sft::<T>::accept_soulbound_issuance(
			origin::<T>(&receiver).into(),
			collection_id,
			0
		));

	}: _(origin::<T>(&owner), receiver, collection_id, BoundedVec::try_from(vec![(serial_number, 1)]).unwrap())
	verify {
		let token = TokenInfo::<T>::get((collection_id, serial_number));
		assert!(token.is_some());
		let token = token.unwrap();
		assert_eq!(token.token_issuance, 0);
	}

	set_additional_data {
		let owner = account::<T>("Alice");
		let token_id = build_token::<T>(Some(owner.clone()), 1);
		let additional_data = max_bounded_vec::<T::MaxDataLength>();
	}: _(origin::<T>(&account::<T>("Alice")), token_id, Some(additional_data.clone()))
	verify {
		assert_eq!(AdditionalTokenData::<T>::get(token_id), additional_data);
	}

	create_token_with_additional_data {
		let owner = account::<T>("Alice");
		let collection_id = build_collection::<T>(Some(owner.clone()));
		let additional_data = max_bounded_vec::<T::MaxDataLength>();
		let initial_issuance = u128::MAX;
		let token_name = max_bounded_vec::<T::StringLimit>();
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, token_name, initial_issuance, None, None, additional_data.clone())
	verify {
		let token_id = (collection_id, 0);
		assert_eq!(AdditionalTokenData::<T>::get(token_id), additional_data);
		assert!(TokenInfo::<T>::contains_key(token_id));
	}
}

impl_benchmark_test_suite!(
	Sft,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
