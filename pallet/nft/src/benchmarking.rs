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

//! NFT benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use crate::Pallet as Nft;
use codec::Encode;
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, BoundedVec};
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
	let id = Nft::<T>::next_collection_uuid().unwrap();
	let caller = caller.unwrap_or_else(|| account::<T>("Alice"));
	let metadata_scheme = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
	let cross_chain_compatibility = CrossChainCompatibility::default();

	assert_ok!(Nft::<T>::create_collection(
		origin::<T>(&caller).into(),
		BoundedVec::truncate_from("New Collection".encode()),
		1,
		None,
		None,
		metadata_scheme,
		None,
		cross_chain_compatibility,
	));

	id
}

benchmarks! {
	claim_unowned_collection {
		let collection_id = build_collection::<T>(Some(Nft::<T>::account_id()));
	}: _(RawOrigin::Root, collection_id, account::<T>("Alice"))

	set_owner {
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, account::<T>("Bob"))

	set_max_issuance {
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, 32)

	set_base_uri {
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, "https://example.com/tokens/".into())

	set_name {
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, BoundedVec::truncate_from("New Name".encode()))

	set_royalties_schedule {
		let collection_id = build_collection::<T>(None);
		let collection_owner = account::<T>("Alice");
		let royalties_schedule = RoyaltiesSchedule {
			entitlements: BoundedVec::truncate_from(vec![(collection_owner, Permill::one())]),
		};
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, royalties_schedule)

	create_collection {
		let metadata = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
		let ccc = CrossChainCompatibility { xrpl: false };
	}: _(origin::<T>(&account::<T>("Alice")), BoundedVec::truncate_from("Collection".encode()), 0, None, None, metadata, None, ccc)

	toggle_public_mint {
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, true)

	set_mint_fee {
		let collection_id = build_collection::<T>(None);
		let pricing_details = Some((1, 100));
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, pricing_details)

	mint {
		let p in 1 .. (1000);
		let owner = account::<T>("Alice");
		let collection_id = build_collection::<T>(Some(owner.clone()));
		let asset_id = 1;
		let mint_fee: Balance = 100;

		// Toggle public mint to traverse worst case scenario
		assert_ok!(Nft::<T>::toggle_public_mint(
		   origin::<T>(&owner).into(),
		   collection_id,
		   true
		));
		assert_ok!(Nft::<T>::set_mint_fee(
		   origin::<T>(&owner).into(),
		   collection_id,
		   Some((1, 100))
		));

		// fund the mint account
		let minter = account::<T>("Bob");
		assert_ok!(T::MultiCurrency::mint_into(asset_id, &minter, mint_fee * 5u128 * p as u128));
	}: _(origin::<T>(&minter), collection_id, p, Some(minter.clone()))
	verify {
		assert_eq!(Nft::<T>::token_balance_of(&minter, collection_id), p);
		for i in 1..=p {
			assert_eq!(TokenInfo::<T>::get(collection_id, i).unwrap().owner, minter);
		}
	}

	transfer {
		let collection_id = build_collection::<T>(None);
		let p in 1 .. (500);
		assert_ok!(Nft::<T>::mint(
			origin::<T>(&account::<T>("Alice")).into(),
			collection_id,
			p,
			None,
		));
		let serial_numbers: Vec<SerialNumber> = (0..p).collect();
		let serial_numbers = BoundedVec::try_from(serial_numbers).unwrap();
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, serial_numbers.clone(), account::<T>("Bob"))
	verify {
		for serial_number in serial_numbers.iter() {
			assert_eq!(TokenInfo::<T>::get(collection_id, *serial_number).unwrap().owner, account::<T>("Bob"));
		}
	}

	burn {
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Alice")), TokenId::from((collection_id, 0)))

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

	set_token_transferable_flag {
		let collection_id = build_collection::<T>(None);
		let token_id = (collection_id, 0);
	}: _(origin::<T>(&account::<T>("Alice")), token_id, true)
	verify {
		assert!(TokenInfo::<T>::get(collection_id, 0).unwrap().utility_flags.transferable);
	}

	issue_soulbound {
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, 1, account::<T>("Bob"), TokenBurnAuthority::Both)
	verify {
		let collection_issuances =
			PendingIssuances::<T>::get(collection_id).pending_issuances;

		let pending_issuances = &collection_issuances[0].1;

		assert_eq!(
			pending_issuances.len(),
			1,
		)
	}

	accept_soulbound_issuance {
		let collection_id = build_collection::<T>(None);
		let receiver = account::<T>("Bob");

		assert_ok!(Nft::<T>::issue_soulbound(
			origin::<T>(&account::<T>("Alice")).into(),
			collection_id,
			1,
			receiver.clone(),
			TokenBurnAuthority::Both,
		));
	}: _(origin::<T>(&receiver.clone()), collection_id, 0)
	verify {
		assert_eq!(TokenInfo::<T>::get(collection_id, 1).unwrap().owner, receiver);
	}
}

impl_benchmark_test_suite!(
	Nft,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
