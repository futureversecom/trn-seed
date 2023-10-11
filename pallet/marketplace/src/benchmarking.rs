// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

//! Marketplace benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use crate::Pallet as Marketplace;
use codec::Encode;
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, BoundedVec};
use frame_system::RawOrigin;
use pallet_nft::{CrossChainCompatibility, Pallet as Nft};
use seed_primitives::MetadataScheme;
use sp_runtime::Permill;
use sp_std::vec;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

pub fn build_collection<T: Config + pallet_nft::Config>(
	caller: Option<T::AccountId>,
) -> CollectionUuid {
	let id = Nft::<T>::next_collection_uuid().unwrap();
	let caller = caller.unwrap_or_else(|| account::<T>("Alice"));
	let metadata_scheme = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
	let cross_chain_compatibility = CrossChainCompatibility::default();

	assert_ok!(Nft::<T>::create_collection(
		origin::<T>(&caller).into(),
		BoundedVec::truncate_from("New Collection".encode()),
		1000,
		None,
		None,
		metadata_scheme,
		None,
		cross_chain_compatibility,
	));

	id
}

pub fn build_asset<T: Config>(owner: &T::AccountId) -> AssetId {
	let asset_id = T::MultiCurrency::create(&owner, None).unwrap();
	assert_ok!(T::MultiCurrency::mint_into(asset_id, &owner, 1_000_000_000u32.into()));

	let beneficiary = vec![(account::<T>("Bob"), 1_000u32.into())];
	assert_ok!(T::MultiCurrency::split_transfer(&owner, asset_id, &beneficiary));
	asset_id
}

pub fn listing_builder<T: Config>(
	collection_id: CollectionUuid,
	is_auction: bool,
) -> (AssetId, ListingId) {
	let alice = account::<T>("Alice");
	let asset_id = build_asset::<T>(&alice);
	let listing_id = NextListingId::<T>::get();
	let serial_numbers = BoundedVec::try_from(vec![0]).unwrap();

	if is_auction {
		assert_ok!(Marketplace::<T>::auction_nft(
			origin::<T>(&alice).into(),
			collection_id,
			serial_numbers,
			asset_id.clone(),
			Balance::from(1u32),
			Some(10u32.into()),
			None
		));
	} else {
		assert_ok!(Marketplace::<T>::sell_nft(
			origin::<T>(&alice).into(),
			collection_id,
			serial_numbers,
			None,
			asset_id.clone(),
			Balance::from(100u128),
			Some(100u32.into()),
			None
		));
	}

	(asset_id, listing_id)
}

pub fn offer_builder<T: Config>(collection_id: CollectionUuid) -> OfferId {
	let asset_id = build_asset::<T>(&account::<T>("Alice"));
	let token_id = TokenId::from((collection_id, 0));
	let offer_id = NextOfferId::<T>::get();

	assert_ok!(Marketplace::<T>::make_simple_offer(
		origin::<T>(&account::<T>("Bob")).into(),
		token_id,
		1u32.into(),
		asset_id,
		None,
	));

	offer_id
}

benchmarks! {
	where_clause { where T: pallet_nft::Config }
	register_marketplace {
	}: _(origin::<T>(&account::<T>("Alice")), None, Permill::zero())

	sell_nft {
		let p in 1 .. (50);
		let alice = account::<T>("Alice");
		let asset_id = build_asset::<T>(&alice);
		let listing_id = NextListingId::<T>::get();
		let collection_id = build_collection::<T>(None);
		let serial_numbers: Vec<SerialNumber> = (0..p).collect();
		let serial_numbers = BoundedVec::try_from(serial_numbers).unwrap();
	}: _(origin::<T>(&alice), collection_id, serial_numbers, None, asset_id, Balance::from(100u32), None, None)
	verify {
		assert_eq!(listing_id + 1, NextListingId::<T>::get())
	}

	buy {
		let collection_id = build_collection::<T>(None);
		let (asset_id, listing_id) = listing_builder::<T>(collection_id, false);
	}: _(origin::<T>(&account::<T>("Bob")), listing_id)

	auction_nft {
		let p in 1 .. (50);
		let alice = account::<T>("Alice");
		let asset_id = build_asset::<T>(&alice);
		let listing_id = NextListingId::<T>::get();
		let collection_id = build_collection::<T>(None);
		let serial_numbers: Vec<SerialNumber> = (0..p).collect();
		let serial_numbers = BoundedVec::try_from(serial_numbers).unwrap();
	}: _(origin::<T>(&alice), collection_id, serial_numbers, asset_id, Balance::from(1u32), Some(10u32.into()), None)
	verify {
		assert_eq!(listing_id + 1, NextListingId::<T>::get())
	}

	bid {
		let collection_id = build_collection::<T>(None);
		let (_, listing_id) = listing_builder::<T>(collection_id, true);
	}: _(origin::<T>(&account::<T>("Bob")), listing_id, Balance::from(1_000u32))

	cancel_sale {
		let collection_id = build_collection::<T>(None);
		let (_, listing_id) = listing_builder::<T>(collection_id, false);
	}: _(origin::<T>(&account::<T>("Alice")), listing_id)

	update_fixed_price {
		let collection_id = build_collection::<T>(None);
		let (_, listing_id) = listing_builder::<T>(collection_id, false);
	}: _(origin::<T>(&account::<T>("Alice")), listing_id, Balance::from(122u32))

	make_simple_offer {
		let asset_id = build_asset::<T>(&account::<T>("Alice"));
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Bob")), TokenId::from((collection_id, 0)), 1u32.into(), asset_id, None)

	cancel_offer {
		let collection_id = build_collection::<T>(None);
		let offer_id = offer_builder::<T>(collection_id);
	}: _(origin::<T>(&account::<T>("Bob")), offer_id)

	accept_offer {
		let collection_id = build_collection::<T>(None);
		let offer_id = offer_builder::<T>(collection_id);
	}: _(origin::<T>(&account::<T>("Alice")), offer_id)

	set_fee_to {
		let fee_account = account::<T>("Alice");
	}: _(RawOrigin::Root, Some(fee_account))
}

impl_benchmark_test_suite!(Marketplace, crate::mock::new_test_ext(), crate::mock::Test,);
