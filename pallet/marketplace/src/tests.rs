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

use super::*;
use crate::mock::{
	create_account, AssetsExt, Event as MockEvent, Marketplace, NativeAssetId, Nft, NftPalletId,
	System, Test, TestExt,
};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use pallet_nft::{CrossChainCompatibility, FeeTo, Listings};
use seed_primitives::{AccountId, MetadataScheme, TokenId};
use sp_runtime::{BoundedVec, DispatchError::BadOrigin, Permill};

// Create an NFT collection
// Returns the created `collection_id`
fn setup_collection(owner: AccountId) -> CollectionUuid {
	let collection_id = Nft::next_collection_uuid().unwrap();
	let collection_name = b"test-collection".to_vec();
	let metadata_scheme = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
	assert_ok!(Nft::create_collection(
		Some(owner).into(),
		BoundedVec::truncate_from(collection_name),
		0,
		None,
		None,
		metadata_scheme,
		None,
		CrossChainCompatibility::default(),
	));
	collection_id
}

/// Setup a token, return collection id, token id, token owner
fn setup_token() -> (CollectionUuid, TokenId, AccountId) {
	let collection_owner = create_account(1);
	let collection_id = setup_collection(collection_owner);
	let token_owner = create_account(2);
	let token_id = (collection_id, 0);
	assert_ok!(Nft::mint(Some(collection_owner).into(), collection_id, 1, Some(token_owner)));

	(collection_id, token_id, token_owner)
}

#[test]
fn register_marketplace_works() {
	TestExt::default().build().execute_with(|| {
		let account = create_account(1);
		let marketplace_id = Nft::next_marketplace_id();
		assert_ok!(Marketplace::register_marketplace(
			Some(account).into(),
			None,
			Permill::from_parts(0)
		));
		assert_eq!(Nft::next_marketplace_id(), marketplace_id + 1);
	});
}

#[test]
fn sell_nft_works() {
	TestExt::default().build().execute_with(|| {
		let listing_id = Nft::next_listing_id();
		let (collection_id, token_id, token_owner) = setup_token();
		let serial_numbers = BoundedVec::truncate_from(vec![token_id.1]);
		assert_ok!(Marketplace::sell_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			None,
			NativeAssetId::get(),
			10,
			None,
			None,
		));
		assert_eq!(Nft::next_listing_id(), listing_id + 1);
	});
}

#[test]
fn update_fixed_price_works() {
	TestExt::default().build().execute_with(|| {
		let listing_id = Nft::next_listing_id();
		let (collection_id, token_id, token_owner) = setup_token();
		let serial_numbers = BoundedVec::truncate_from(vec![token_id.1]);
		assert_ok!(Marketplace::sell_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			None,
			NativeAssetId::get(),
			10,
			None,
			None,
		));

		assert_ok!(Marketplace::update_fixed_price(Some(token_owner).into(), listing_id, 100,));

		System::assert_last_event(MockEvent::Nft(pallet_nft::Event::FixedPriceSalePriceUpdate {
			collection_id,
			serial_numbers: vec![token_id.1],
			listing_id,
			new_price: 100,
		}));
	});
}

#[test]
fn buy_works() {
	TestExt::default().build().execute_with(|| {
		let listing_id = Nft::next_listing_id();
		let (collection_id, token_id, token_owner) = setup_token();
		let serial_numbers = BoundedVec::truncate_from(vec![token_id.1]);
		assert_ok!(Marketplace::sell_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			None,
			NativeAssetId::get(),
			0,
			None,
			None,
		));

		let buyer = create_account(12);
		assert_ok!(Marketplace::buy(Some(buyer).into(), listing_id));
		assert_eq!(Nft::token_balance_of(&buyer, collection_id), 1);
	});
}

#[test]
fn auction_nft_works() {
	TestExt::default().build().execute_with(|| {
		let listing_id = Nft::next_listing_id();
		let (collection_id, token_id, token_owner) = setup_token();
		let serial_numbers = BoundedVec::truncate_from(vec![token_id.1]);
		assert_ok!(Marketplace::auction_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			NativeAssetId::get(),
			10,
			None,
			None,
		));

		assert_eq!(Nft::next_listing_id(), listing_id + 1);
	});
}

#[test]
fn bid_works() {
	let bidder = create_account(12);
	let bid_price = 100;

	TestExt::default()
		.with_balances(&[(bidder, bid_price)])
		.build()
		.execute_with(|| {
			let listing_id = Nft::next_listing_id();
			let (collection_id, token_id, token_owner) = setup_token();
			let serial_numbers = BoundedVec::truncate_from(vec![token_id.1]);
			assert_ok!(Marketplace::auction_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				NativeAssetId::get(),
				0,
				None,
				None,
			));

			let bidder = create_account(12);
			assert_ok!(Marketplace::bid(Some(bidder).into(), listing_id, bid_price));
			assert_eq!(
				AssetsExt::hold_balance(&NftPalletId::get(), &bidder, &NativeAssetId::get()),
				bid_price
			);
		});
}

#[test]
fn cancel_sale_works() {
	TestExt::default().build().execute_with(|| {
		let listing_id = Nft::next_listing_id();
		let (collection_id, token_id, token_owner) = setup_token();
		let serial_numbers = BoundedVec::truncate_from(vec![token_id.1]);
		assert_ok!(Marketplace::auction_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			NativeAssetId::get(),
			0,
			None,
			None,
		));
		assert!(Listings::<Test>::get(listing_id).is_some());
		assert_ok!(Marketplace::cancel_sale(Some(token_owner).into(), listing_id));
		assert!(Listings::<Test>::get(listing_id).is_none());
	});
}

#[test]
fn make_simple_offer_works() {
	let buyer = create_account(12);
	let offer_price = 100;

	TestExt::default()
		.with_balances(&[(buyer, offer_price)])
		.build()
		.execute_with(|| {
			let offer_id = Nft::next_offer_id();
			let (_, token_id, _) = setup_token();
			assert_ok!(Marketplace::make_simple_offer(
				Some(buyer).into(),
				token_id,
				offer_price,
				NativeAssetId::get(),
				None
			));

			assert_eq!(Nft::next_offer_id(), offer_id + 1);
		});
}

#[test]
fn cancel_offer_works() {
	let buyer = create_account(12);
	let offer_price = 100;

	TestExt::default()
		.with_balances(&[(buyer, offer_price)])
		.build()
		.execute_with(|| {
			let offer_id = Nft::next_offer_id();
			let (_, token_id, _) = setup_token();
			assert_ok!(Marketplace::make_simple_offer(
				Some(buyer).into(),
				token_id,
				offer_price,
				NativeAssetId::get(),
				None
			));

			assert!(Nft::token_offers(token_id).is_some());
			assert_ok!(Marketplace::cancel_offer(Some(buyer).into(), offer_id));
			assert!(Nft::token_offers(token_id).is_none());
		});
}

#[test]
fn accept_offer_works() {
	let buyer = create_account(12);
	let offer_price = 100;

	TestExt::default()
		.with_balances(&[(buyer, offer_price)])
		.build()
		.execute_with(|| {
			let offer_id = Nft::next_offer_id();
			let (_, token_id, token_owner) = setup_token();
			assert_ok!(Marketplace::make_simple_offer(
				Some(buyer).into(),
				token_id,
				offer_price,
				NativeAssetId::get(),
				None
			));

			assert!(Nft::token_offers(token_id).is_some());
			assert_ok!(Marketplace::accept_offer(Some(token_owner).into(), offer_id));
			assert!(Nft::token_offers(token_id).is_none());
		});
}

mod set_fee_to {
	use super::*;

	#[test]
	fn set_fee_to_works() {
		let new_fee_to = create_account(13);

		TestExt::default().build().execute_with(|| {
			assert_ok!(Marketplace::set_fee_to(RawOrigin::Root.into(), new_fee_to.into()));

			assert_eq!(FeeTo::<Test>::get().unwrap(), new_fee_to);
		});
	}

	#[test]
	fn set_fee_to_not_root_fails() {
		TestExt::default().build().execute_with(|| {
			let new_fee_to = create_account(10);

			assert_noop!(
				Marketplace::set_fee_to(Some(create_account(11)).into(), Some(new_fee_to)),
				BadOrigin
			);
		});
	}
}
