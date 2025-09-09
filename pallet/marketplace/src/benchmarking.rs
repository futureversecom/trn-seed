// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the Apache License, Version 2.0 (the "License");
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

use crate::{Marketplace as RegisteredMarketplace, Pallet as Marketplace};
use codec::Encode;
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::{
	assert_ok,
	traits::{
		fungibles::Inspect,
		tokens::{Fortitude, Preservation},
	},
	BoundedVec,
};
use frame_system::RawOrigin;
use pallet_nft::Pallet as Nft;
use pallet_sft::Pallet as Sft;
use seed_primitives::{CrossChainCompatibility, MetadataScheme};
use sp_runtime::Permill;
use sp_std::{vec, vec::Vec};

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
	let collection_id = Nft::<T>::next_collection_uuid().unwrap();
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

	collection_id
}

pub fn build_sft_token<T: Config + pallet_nft::Config + pallet_sft::Config>(
	caller: Option<T::AccountId>,
) -> CollectionUuid {
	let collection_id = Nft::<T>::next_collection_uuid().unwrap();
	let caller = caller.unwrap_or_else(|| account::<T>("Alice"));
	let metadata_scheme = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
	assert_ok!(Sft::<T>::create_collection(
		origin::<T>(&caller).into(),
		BoundedVec::truncate_from("New SFT Collection".encode()),
		None,
		metadata_scheme,
		None,
	));

	// Create token with high initial issuance
	let initial_issuance = 1000;
	assert_ok!(Sft::<T>::create_token(
		origin::<T>(&caller).into(),
		collection_id,
		BoundedVec::truncate_from("SFT Token".encode()),
		initial_issuance,
		None,
		None
	));

	collection_id
}

pub fn build_asset<T: Config>(owner: &T::AccountId) -> AssetId {
	let asset_id = T::MultiCurrency::create(&owner, None).unwrap();
	assert_ok!(T::MultiCurrency::mint_into(asset_id, &owner, 1_000_000_000u32.into()));
	assert_eq!(
		T::MultiCurrency::reducible_balance(
			asset_id,
			&owner,
			Preservation::Expendable,
			Fortitude::Polite
		),
		1_000_000_000u32.into()
	);
	assert_eq!(T::MultiCurrency::minimum_balance(asset_id), 1u32.into());

	let beneficiary = account::<T>("Bob");
	assert_ok!(T::MultiCurrency::transfer(
		asset_id,
		&owner,
		&beneficiary,
		1_000_000u32.into(),
		Preservation::Expendable
	));
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
			Balance::from(0u128),
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
	where_clause { where T: pallet_nft::Config + pallet_sft::Config }

	register_marketplace {
		let marketplace = account::<T>("Marketplace");
		let entitlement = Permill::from_parts(123);
		let marketplace_id = NextMarketplaceId::<T>::get();
	}: _(origin::<T>(&account::<T>("Alice")), Some(marketplace), entitlement)
	verify {
		let expected = RegisteredMarketplace {
			account: marketplace,
			entitlement
		};
		assert_eq!(RegisteredMarketplaces::<T>::get(marketplace_id).unwrap(), expected);
	}

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
		assert_eq!(listing_id + 1, NextListingId::<T>::get());
		assert!(Listings::<T>::get(listing_id).is_some());
	}

	sell_sft {
		let p in 1 .. (50);
		let alice = account::<T>("Alice");
		let asset_id = build_asset::<T>(&alice);
		let listing_id = NextListingId::<T>::get();
		let collection_id = build_sft_token::<T>(None);

		// Create p tokens
		for i in 1..p {
			assert_ok!(Sft::<T>::create_token(
				origin::<T>(&alice).into(),
				collection_id,
				BoundedVec::truncate_from("SFT Token".encode()),
				1000,
				None,
				None
			));
		}
		let serial_numbers: Vec<SerialNumber> = (0..p).collect();
		let serials_combined: Vec<(SerialNumber, Balance)> = serial_numbers.iter().map(|s| (*s, 1000)).collect();
		let tokens = ListingTokens::<T>::Sft(SftListing {
			collection_id,
			serial_numbers: BoundedVec::truncate_from(serials_combined),
		});
	}: sell(origin::<T>(&alice), tokens, None, asset_id, Balance::from(100u32), None, None)
	verify {
		assert_eq!(listing_id + 1, NextListingId::<T>::get());
		assert!(Listings::<T>::get(listing_id).is_some());
	}

	buy {
		let collection_id = build_collection::<T>(None);
		let (asset_id, listing_id) = listing_builder::<T>(collection_id, false);
		assert_eq!(
		<T as Config>::MultiCurrency::reducible_balance(
			asset_id,
			&account::<T>("Bob"),
			Preservation::Expendable,
			Fortitude::Polite
		),
		1_000_000u32.into()
	);
	}: _(origin::<T>(&account::<T>("Bob")), listing_id)

	buy_multi {
		let p in 1 .. (50);
		let mut listing_ids: Vec<ListingId> = vec![];
		for i in 0..p {
			let collection_id = build_collection::<T>(None);
			let (asset_id, listing_id) = listing_builder::<T>(collection_id, false);
			listing_ids.push(listing_id);
		}
	}: _(origin::<T>(&account::<T>("Bob")), BoundedVec::truncate_from(listing_ids.clone()))
	verify {
		for listing_id in listing_ids {
			assert_eq!(Listings::<T>::get(listing_id).is_none(), true);
		}
	}

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
		assert_eq!(listing_id + 1, NextListingId::<T>::get());
		assert!(Listings::<T>::get(listing_id).is_some());
	}

	auction_sft {
		let p in 1 .. (50);
		let alice = account::<T>("Alice");
		let asset_id = build_asset::<T>(&alice);
		let listing_id = NextListingId::<T>::get();
		let collection_id = build_sft_token::<T>(None);

		// Create p tokens
		for i in 1..p {
			assert_ok!(Sft::<T>::create_token(
				origin::<T>(&alice).into(),
				collection_id,
				BoundedVec::truncate_from("SFT Token".encode()),
				1000,
				None,
				None
			));
		}
		let serial_numbers: Vec<SerialNumber> = (0..p).collect();
		let serials_combined: Vec<(SerialNumber, Balance)> = serial_numbers.iter().map(|s| (*s, 1000)).collect();
		let tokens = ListingTokens::<T>::Sft(SftListing {
			collection_id,
			serial_numbers: BoundedVec::truncate_from(serials_combined),
		});
	}: auction(origin::<T>(&alice), tokens, asset_id, Balance::from(1u32), Some(10u32.into()), None)
	verify {
		assert_eq!(listing_id + 1, NextListingId::<T>::get());
		assert!(Listings::<T>::get(listing_id).is_some());
	}

	bid {
		let collection_id = build_collection::<T>(None);
		let (_, listing_id) = listing_builder::<T>(collection_id, true);
	}: _(origin::<T>(&account::<T>("Bob")), listing_id, Balance::from(1_000u32))
	verify {
		assert_eq!(ListingWinningBid::<T>::get(listing_id).unwrap(), (account::<T>("Bob"), 1_000u32.into()));
	}

	cancel_sale {
		let collection_id = build_collection::<T>(None);
		let (_, listing_id) = listing_builder::<T>(collection_id, false);
	}: _(origin::<T>(&account::<T>("Alice")), listing_id)
	verify {
		assert!(Listings::<T>::get(listing_id).is_none());
	}

	update_fixed_price {
		let collection_id = build_collection::<T>(None);
		let (_, listing_id) = listing_builder::<T>(collection_id, false);
	}: _(origin::<T>(&account::<T>("Alice")), listing_id, Balance::from(122u32))
	verify {
		let listing = Listings::<T>::get(listing_id).unwrap();
		match listing {
			Listing::FixedPrice(listing) => assert_eq!(listing.fixed_price, 122u32.into()),
			_ => panic!("Invalid listing type"),
		}
	}

	make_simple_offer {
		let asset_id = build_asset::<T>(&account::<T>("Alice"));
		let collection_id = build_collection::<T>(None);
		let next_offer_id = NextOfferId::<T>::get();
	}: _(origin::<T>(&account::<T>("Bob")), TokenId::from((collection_id, 0)), 1u32.into(), asset_id, None)
	verify {
		assert_eq!(NextOfferId::<T>::get(), next_offer_id + 1);
		assert!(Offers::<T>::get(next_offer_id).is_some());
	}

	cancel_offer {
		let collection_id = build_collection::<T>(None);
		let offer_id = offer_builder::<T>(collection_id);
	}: _(origin::<T>(&account::<T>("Bob")), offer_id)
	verify {
		assert_eq!(NextOfferId::<T>::get(), offer_id + 1);
		assert!(Offers::<T>::get(offer_id).is_none());
	}

	accept_offer {
		let collection_id = build_collection::<T>(None);
		let offer_id = offer_builder::<T>(collection_id);
	}: _(origin::<T>(&account::<T>("Alice")), offer_id)
	verify {
		assert_eq!(NextOfferId::<T>::get(), offer_id + 1);
		assert!(Offers::<T>::get(offer_id).is_none());
	}

	remove_offer {
		let collection_id = build_collection::<T>(None);
		let offer_id = offer_builder::<T>(collection_id);
	}: _(origin::<T>(&account::<T>("Alice")), offer_id)
	verify {
		assert_eq!(NextOfferId::<T>::get(), offer_id + 1);
		assert!(Offers::<T>::get(offer_id).is_none());
	}

	set_fee_to {
		let fee_account = account::<T>("Alice");
	}: _(RawOrigin::Root, Some(fee_account))
	verify {
		assert_eq!(FeeTo::<T>::get().unwrap(), fee_account);
	}
}

impl_benchmark_test_suite!(
	Marketplace,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
