// /* Copyright 2019-2021 Centrality Investments Limited
// *
// * Licensed under the LGPL, Version 3.0 (the "License");
// * you may not use this file except in compliance with the License.
// * Unless required by applicable law or agreed to in writing, software
// * distributed under the License is distributed on an "AS IS" BASIS,
// * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// * See the License for the specific language governing permissions and
// * limitations under the License.
// * You may obtain a copy of the License at the root of this project source code,
// * or at:
// * https://centrality.ai/licenses/gplv3.txt
// * https://centrality.ai/licenses/lgplv3.txt
// */
//! NFT benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use crate::TokenOwner;
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_runtime::Permill;

use crate::Pallet as Nft;

pub struct BenchmarkData<T: Config> {
	pub coll_owner: T::AccountId,
	pub coll_id: CollectionUuid,
	pub coll_tokens: Vec<TokenId>,
	pub asset_id: AssetId,
	pub asset_owner: T::AccountId,
	pub mp_id: u32,
	pub mp_owner: T::AccountId,
	pub token_id: TokenId,
}

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

// Create an NFT collection
// Returns the created `coll_id`
fn setup_benchmark<T: Config>() -> BenchmarkData<T> {
	let alice = account::<T>("Alice");
	let coll_owner = alice.clone();
	let coll_id = Nft::<T>::next_collection_uuid().unwrap();
	let collection_name = [1_u8; MAX_COLLECTION_NAME_LENGTH as usize].to_vec();
	let metadata_scheme = MetadataScheme::IpfsDir(
		b"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi".to_vec(),
	);
	let royalties = RoyaltiesSchedule::<T::AccountId> {
		entitlements: (0..(MAX_ENTITLEMENTS - 2))
			.map(|_| (coll_owner.clone(), Permill::from_percent(1)))
			.collect::<Vec<(T::AccountId, Permill)>>(),
	};

	assert_ok!(Nft::<T>::create_collection(
		RawOrigin::Signed(coll_owner.clone()).into(),
		collection_name,
		0,
		None,
		None,
		metadata_scheme,
		Some(royalties.clone()),
	));
	assert_ok!(Nft::<T>::mint(origin::<T>(&coll_owner).into(), coll_id, 1, None,));
	let coll_tokens: Vec<TokenId> =
		TokenOwner::<T>::iter_prefix(coll_id).map(|x| (coll_id, x.0)).collect();

	// Alice will be the owner of the Token
	let asset_owner = alice.clone();
	let asset_id = T::MultiCurrency::create(&asset_owner).unwrap();
	assert_ok!(T::MultiCurrency::mint_into(asset_id, &asset_owner, 1000000000u32.into()));

	// Bob will get some funds too
	assert_ok!(T::MultiCurrency::split_transfer(
		&asset_owner,
		asset_id,
		&vec![(account::<T>("Bob"), 10000u32.into())]
	));

	let mp_owner = alice.clone();
	let mp_id = NextMarketplaceId::<T>::get();
	assert_ok!(Nft::<T>::register_marketplace(
		RawOrigin::Signed(mp_owner.clone()).into(),
		None,
		Permill::zero()
	));

	let token_id: TokenId = coll_tokens[0].clone();

	BenchmarkData {
		coll_owner,
		coll_id,
		coll_tokens,
		asset_id,
		asset_owner,
		mp_id,
		mp_owner,
		token_id,
	}
}

benchmarks! {
	claim_unowned_collection {
		let metadata = MetadataScheme::Https("google.com".into());
		let coll_id = Nft::<T>::next_collection_uuid().unwrap();
		let pallet_account = Nft::<T>::account_id();

		assert_ok!(Nft::<T>::create_collection(origin::<T>(&pallet_account).into(), "My Collection".into(), 0, None, None, metadata, None));

		let new_owner = account::<T>("Alice");
	}: _(RawOrigin::Root, coll_id, new_owner.clone())
	verify {
		assert_eq!(CollectionInfo::<T>::get(&coll_id).unwrap().owner, new_owner);
	}

	set_owner {
		let BenchmarkData {coll_id, ..} = setup_benchmark::<T>();
		let new_owner = account::<T>("Bob");

		// Sanity check
		let current_owner = CollectionInfo::<T>::get(coll_id).unwrap().owner;
		assert_ne!(current_owner, new_owner);

	}: _(origin::<T>(&current_owner), coll_id, new_owner.clone())
	verify {
		assert_eq!(CollectionInfo::<T>::get(&coll_id).unwrap().owner, new_owner);
	}

	register_marketplace {
		let owner = account::<T>("Alice");
		let mp_id = NextMarketplaceId::<T>::get();

	}: _(origin::<T>(&owner), None, Permill::zero())
	verify {
		assert!(RegisteredMarketplaces::<T>::get(&mp_id).is_some());
	}

	create_collection {
		let owner = account::<T>("Alice");
		let coll_id = Nft::<T>::next_collection_uuid().unwrap();
		let metadata = MetadataScheme::Https("google.com".into());

	}: _(origin::<T>(&owner), "My Collection".into(), 0, None, None, metadata, None)
	verify {
		let collection = CollectionInfo::<T>::get(&coll_id).unwrap();
		assert_eq!(collection.owner, owner);
	}

	mint {
		let BenchmarkData {coll_id, coll_owner, .. } = setup_benchmark::<T>();
		let beneficiary = account::<T>("Bob");
		let quantity = 2u32; // TODO_MARKO What to do with this? 1000q is too much
		let old_serial_number = NextSerialNumber::<T>::get(coll_id).unwrap();
		let new_serial_number = old_serial_number + quantity;

	}: _(origin::<T>(&coll_owner), coll_id, quantity.into(), Some(beneficiary))
	verify {
		let serial_number = NextSerialNumber::<T>::get(coll_id).unwrap();
		assert_eq!(serial_number, new_serial_number);
	}

	transfer {
		let BenchmarkData {coll_owner, token_id, ..} = setup_benchmark::<T>();
		let new_owner = account::<T>("Bob");

		// Sanity check
		assert_ne!(coll_owner, new_owner);

	}: _(origin::<T>(&coll_owner), token_id.clone(), new_owner.clone())
	verify {
		let token_owner = TokenOwner::<T>::get(token_id.0, token_id.1).unwrap();
		assert_eq!(token_owner, new_owner);
	}

	burn {
		let BenchmarkData {coll_owner, token_id, ..} = setup_benchmark::<T>();

	}: _(origin::<T>(&coll_owner), token_id.clone())
	verify {
		let token_owner = TokenOwner::<T>::get(token_id.0, token_id.1);
		assert_eq!(token_owner, None);
	}

	sell {
		let BenchmarkData {coll_owner, coll_tokens, asset_id, token_id, ..} = setup_benchmark::<T>();
		let listing_id = NextListingId::<T>::get();

		// Sanity check
		assert!(Listings::<T>::get(listing_id).is_none());

	}: _(origin::<T>(&coll_owner), coll_tokens, None, asset_id, 100u32.into(), None, None)
	verify {
		let listing = Listings::<T>::get(listing_id);
		assert!(listing.is_some());
	}

	buy {
		let BenchmarkData {coll_tokens, coll_owner, asset_id, token_id, ..} = setup_benchmark::<T>();
		let listing_id = NextListingId::<T>::get();
		let new_owner = account::<T>("Bob");

		assert_ok!(Nft::<T>::sell(origin::<T>(&coll_owner).into(), coll_tokens, None, asset_id, 1u32.into(), None, None));
	}: _(origin::<T>(&new_owner), listing_id)
	verify {
		let token_owner = TokenOwner::<T>::get(token_id.0, token_id.1).unwrap();
		assert_eq!(token_owner, new_owner);
	}

	auction {
		let BenchmarkData {coll_owner, coll_tokens, asset_id, mp_id, ..} = setup_benchmark::<T>();
		let tokens = coll_tokens;
		let listing_id = NextListingId::<T>::get();

	}: _(origin::<T>(&coll_owner), tokens.clone(), asset_id, 1u32.into(), Some(10u32.into()), Some(mp_id))
	verify {
		let listing = Listings::<T>::get(listing_id);
		assert!(listing.is_some());
	}

	bid {
		let BenchmarkData {coll_owner, coll_tokens, asset_id, mp_id, ..} = setup_benchmark::<T>();
		let tokens = coll_tokens;
		let new_bidder = account::<T>("Bob");
		let reserve_price = 10u32.into();
		let auction_bid = reserve_price + 100;
		let listing_id = NextListingId::<T>::get();

		assert_ok!(Nft::<T>::auction(origin::<T>(&coll_owner).into(), tokens.clone(), asset_id, reserve_price, Some(10u32.into()), Some(mp_id)));
	}: _(origin::<T>(&new_bidder), listing_id.clone(), auction_bid.clone().into())
	verify {
		let listing = ListingWinningBid::<T>::get(listing_id);
		assert!(listing.is_some());
	}

	cancel_sale {
		let BenchmarkData {coll_owner, coll_tokens, asset_id, mp_id, ..} = setup_benchmark::<T>();
		let tokens = coll_tokens;
		let listing_id = NextListingId::<T>::get();

		assert_ok!(Nft::<T>::auction(origin::<T>(&coll_owner).into(), tokens.clone(), asset_id, 1u32.into(), Some(10u32.into()), Some(mp_id)));
	}: _(origin::<T>(&coll_owner), listing_id.clone())
	verify {
		let listing = Listings::<T>::get(listing_id);
		assert!(listing.is_none());
	}

	update_fixed_price {
		let BenchmarkData {coll_owner, coll_tokens, asset_id, ..} = setup_benchmark::<T>();
		let listing_id = NextListingId::<T>::get();
		let old_price = 100u32.into();
		let new_price = old_price + 200;

		assert_ok!(Nft::<T>::sell(origin::<T>(&coll_owner).into(), coll_tokens, None, asset_id, old_price, None, None));
	}: _(origin::<T>(&coll_owner), listing_id.clone(), new_price)
	verify {
		let listing = Listings::<T>::get(listing_id).unwrap();
		match listing {
			Listing::FixedPrice(x) => {
				assert_eq!(x.fixed_price, new_price);
			},
			_ => panic!("Cannot be here"),
		}
	}

	make_simple_offer {
		let BenchmarkData {asset_id, mp_id, token_id, ..} = setup_benchmark::<T>();
		let offer_owner = account::<T>("Bob");

		// Sanity check
		let token_offer = TokenOffers::<T>::get(token_id);
		assert!(token_offer.is_none());

	}: _(origin::<T>(&offer_owner), token_id, 1u32.into(), asset_id, Some(mp_id))
	verify {
		let token_offer = TokenOffers::<T>::get(token_id);
		assert!(token_offer.is_some());
	}

	cancel_offer {
		let BenchmarkData {asset_id, mp_id, token_id, ..} = setup_benchmark::<T>();
		let offer_owner = account::<T>("Bob");
		let offer_id = NextOfferId::<T>::get();

		assert_ok!(Nft::<T>::make_simple_offer(origin::<T>(&offer_owner).into(), token_id, 1u32.into(), asset_id, Some(mp_id)));
	}: _(origin::<T>(&offer_owner), offer_id)
	verify {
		let token_offer = TokenOffers::<T>::get(token_id);
		assert!(token_offer.is_none());
	}

	accept_offer {
		let BenchmarkData {coll_owner, asset_id, mp_id, token_id, ..} = setup_benchmark::<T>();
		let offer_owner = account::<T>("Bob");
		let offer_id = NextOfferId::<T>::get();

		assert_ok!(Nft::<T>::make_simple_offer(origin::<T>(&offer_owner).into(), token_id, 1u32.into(), asset_id, Some(mp_id)));
	}: _(origin::<T>(&coll_owner), offer_id)
	verify {
		let token_offer = TokenOffers::<T>::get(token_id);
		assert!(token_offer.is_none());
	}

}

impl_benchmark_test_suite!(Nft, crate::mock::new_test_ext(), crate::mock::Test,);
