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

use super::*;
use crate::mock::{
	create_account, AssetsExt, Event as MockEvent, Marketplace, MarketplacePalletId,
	MaxTokensPerCollection, MaxTokensPerListing, NativeAssetId, Nft, System, Test, TestExt,
};
use frame_support::{
	assert_noop, assert_ok,
	traits::{fungibles::Inspect, OnInitialize},
};
use pallet_nft::CrossChainCompatibility;
use seed_primitives::{AccountId, MetadataScheme, RoyaltiesSchedule, TokenCount, TokenId};
use sp_runtime::{traits::Zero, BoundedVec, Permill};
// Create an NFT collection
// Returns the created `collection_id`
fn setup_collection(owner: AccountId) -> CollectionUuid {
	let collection_id = Nft::next_collection_uuid().unwrap();
	let collection_name = bounded_string("test-collection");
	let metadata_scheme = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
	assert_ok!(Nft::create_collection(
		Some(owner).into(),
		collection_name,
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

/// Setup a token, return collection id, token id, token owner
fn setup_token_with_royalties(
	royalties_schedule: RoyaltiesSchedule<AccountId>,
	quantity: TokenCount,
) -> (CollectionUuid, TokenId, AccountId) {
	let collection_owner = create_account(1);
	let collection_id = Nft::next_collection_uuid().unwrap();
	let collection_name = bounded_string("test-collection");
	let metadata_scheme = MetadataScheme::try_from(b"<CID>".as_slice()).unwrap();
	assert_ok!(Nft::create_collection(
		Some(collection_owner).into(),
		collection_name,
		0,
		None,
		None,
		metadata_scheme,
		Some(royalties_schedule),
		CrossChainCompatibility::default(),
	));

	let token_owner = create_account(2);
	let token_id = (collection_id, 0);
	assert_ok!(Nft::mint(
		Some(collection_owner).into(),
		collection_id,
		quantity,
		Some(token_owner),
	));

	(collection_id, token_id, token_owner)
}

/// Create an offer on a token. Return offer_id, offer
fn make_new_simple_offer(
	offer_amount: Balance,
	token_id: TokenId,
	buyer: AccountId,
	marketplace_id: Option<MarketplaceId>,
) -> (OfferId, SimpleOffer<AccountId>) {
	let next_offer_id = Marketplace::next_offer_id();

	assert_ok!(Marketplace::make_simple_offer(
		Some(buyer).into(),
		token_id,
		offer_amount,
		NativeAssetId::get(),
		marketplace_id
	));
	let offer = SimpleOffer {
		token_id,
		asset_id: NativeAssetId::get(),
		amount: offer_amount,
		buyer,
		marketplace_id,
	};

	// Check storage has been updated
	assert_eq!(Marketplace::next_offer_id(), next_offer_id + 1);
	assert_eq!(Marketplace::offers(next_offer_id), Some(OfferType::Simple(offer.clone())));
	System::assert_last_event(MockEvent::Marketplace(Event::<Test>::Offer {
		offer_id: next_offer_id,
		amount: offer_amount,
		asset_id: NativeAssetId::get(),
		marketplace_id,
		buyer,
	}));

	(next_offer_id, offer)
}

// Helper function for creating the collection name type
pub fn bounded_string(name: &str) -> BoundedVec<u8, <Test as pallet_nft::Config>::StringLimit> {
	BoundedVec::truncate_from(name.as_bytes().to_vec())
}

#[test]
fn sell() {
	let buyer = create_account(3);
	let initial_balance = 1_000;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance)])
		.build()
		.execute_with(|| {
			let collection_owner = create_account(1);
			let quantity = 5;
			let collection_id = Nft::next_collection_uuid().unwrap();

			assert_ok!(Nft::create_collection(
				Some(collection_owner).into(),
				bounded_string("test-collection"),
				quantity,
				None,
				None,
				MetadataScheme::try_from(b"https://example.com/metadata".as_slice()).unwrap(),
				None,
				CrossChainCompatibility::default(),
			));

			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
				BoundedVec::try_from(vec![1, 3, 4]).unwrap();
			let listing_id = Marketplace::next_listing_id();

			assert_ok!(Marketplace::sell_nft(
				Some(collection_owner).into(),
				collection_id,
				serial_numbers.clone(),
				None,
				NativeAssetId::get(),
				1_000,
				None,
				None,
			));

			for serial_number in serial_numbers.iter() {
				assert_eq!(
					Nft::token_locks((collection_id, serial_number)).unwrap(),
					TokenLockReason::Listed(listing_id)
				);
			}

			assert_ok!(Marketplace::buy(Some(buyer).into(), listing_id));
			assert_eq!(
				Nft::owned_tokens(collection_id, &buyer, 0, 1000),
				(0_u32, 3, vec![1, 3, 4])
			);
			assert_eq!(Nft::token_balance_of(&collection_owner, collection_id), 2);
			assert_eq!(
				Nft::token_balance_of(&buyer, collection_id),
				serial_numbers.len() as TokenCount
			);
		})
}

#[test]
fn sell_multiple_fails() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let collection_id = setup_collection(collection_owner);
		// mint some tokens
		assert_ok!(Nft::mint(Some(collection_owner).into(), collection_id, 2, None));

		// empty tokens fails
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![]).unwrap();
		assert_noop!(
			Marketplace::sell_nft(
				Some(collection_owner).into(),
				collection_id,
				serial_numbers,
				None,
				NativeAssetId::get(),
				1_000,
				None,
				None
			),
			Error::<Test>::NoToken
		);
	})
}

#[test]
fn sell_multiple() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();
		let listing_id = Marketplace::next_listing_id();

		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		let buyer = create_account(5);
		assert_ok!(Marketplace::sell_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			Some(buyer),
			NativeAssetId::get(),
			1_000,
			None,
			None,
		));
		System::assert_last_event(MockEvent::Marketplace(Event::<Test>::FixedPriceSaleList {
			collection_id,
			serial_numbers: vec![token_id.1],
			listing_id,
			marketplace_id: None,
			price: 1_000,
			payment_asset: NativeAssetId::get(),
			seller: token_owner,
		}));

		assert_eq!(Nft::token_locks(token_id).unwrap(), TokenLockReason::Listed(listing_id));
		assert!(Marketplace::open_collection_listings(collection_id, listing_id).unwrap());

		let expected = Listing::<Test>::FixedPrice(FixedPriceListing::<Test> {
			payment_asset: NativeAssetId::get(),
			fixed_price: 1_000,
			close: System::block_number() + <Test as Config>::DefaultListingDuration::get(),
			buyer: Some(buyer),
			collection_id,
			serial_numbers: BoundedVec::try_from(vec![token_id.1]).unwrap(),
			seller: token_owner,
			royalties_schedule: Default::default(),
			marketplace_id: None,
		});

		let listing = Listings::<Test>::get(listing_id).expect("token is listed");
		assert_eq!(listing, expected);

		// current block is 1 + duration
		assert!(Marketplace::listing_end_schedule(
			System::block_number() + <Test as Config>::DefaultListingDuration::get(),
			listing_id
		)
		.unwrap());

		// Can't transfer while listed for sale
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_noop!(
			Nft::transfer(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				create_account(9)
			),
			pallet_nft::Error::<Test>::TokenLocked
		);
	});
}

#[test]
fn sell_fails() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();
		// Not token owner
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		let bob = create_account(9);
		let buyer = create_account(5);
		assert_noop!(
			Marketplace::sell_nft(
				Some(bob).into(),
				collection_id,
				serial_numbers,
				Some(buyer),
				NativeAssetId::get(),
				1_000,
				None,
				None
			),
			Error::<Test>::NotTokenOwner
		);

		// token listed already
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_ok!(Marketplace::sell_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers.clone(),
			Some(buyer),
			NativeAssetId::get(),
			1_000,
			None,
			None,
		));
		assert_noop!(
			Marketplace::sell_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers.clone(),
				Some(buyer),
				NativeAssetId::get(),
				1_000,
				None,
				None
			),
			Error::<Test>::TokenLocked
		);

		// can't auction, listed for fixed price sale
		assert_noop!(
			Marketplace::auction_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers.clone(),
				NativeAssetId::get(),
				1_000,
				None,
				None
			),
			Error::<Test>::TokenLocked
		);
	});
}

#[test]
fn cancel_sell() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();
		let listing_id = Marketplace::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		let buyer = create_account(5);
		assert_ok!(Marketplace::sell_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			Some(buyer),
			NativeAssetId::get(),
			1_000,
			None,
			None
		));
		assert_ok!(Marketplace::cancel_sale(Some(token_owner).into(), listing_id));
		System::assert_last_event(MockEvent::Marketplace(Event::<Test>::FixedPriceSaleClose {
			collection_id,
			serial_numbers: vec![token_id.1],
			listing_id,
			reason: FixedPriceClosureReason::VendorCancelled,
		}));

		// storage cleared up
		assert!(Listings::<Test>::get(listing_id).is_none());
		assert!(Marketplace::listing_end_schedule(
			System::block_number() + <Test as Config>::DefaultListingDuration::get(),
			listing_id
		)
		.is_none());

		// it should be free to operate on the token
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		let new_owner = create_account(6);
		assert_ok!(Nft::transfer(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			new_owner,
		));
	});
}

#[test]
fn sell_closes_on_schedule() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();
		let listing_duration = 100;
		let listing_id = Marketplace::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		let buyer = create_account(5);
		assert_ok!(Marketplace::sell_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			Some(buyer),
			NativeAssetId::get(),
			1_000,
			Some(listing_duration),
			None
		));

		// sale should close after the duration expires
		Marketplace::on_initialize(System::block_number() + listing_duration);

		// seller should have tokens
		assert!(Listings::<Test>::get(listing_id).is_none());
		assert!(Marketplace::listing_end_schedule(
			System::block_number() + listing_duration,
			listing_id
		)
		.is_none());

		// should be free to transfer now
		let new_owner = create_account(8);
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_ok!(Nft::transfer(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			new_owner,
		));
	});
}

#[test]
fn updates_fixed_price() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();
		let listing_id = Marketplace::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		let buyer = create_account(5);
		assert_ok!(Marketplace::sell_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			Some(buyer),
			NativeAssetId::get(),
			1_000,
			None,
			None
		));
		assert_ok!(Marketplace::update_fixed_price(Some(token_owner).into(), listing_id, 1_500));
		System::assert_last_event(MockEvent::Marketplace(
			Event::<Test>::FixedPriceSalePriceUpdate {
				collection_id,
				serial_numbers: vec![token_id.1],
				listing_id,
				new_price: 1_500,
			},
		));

		let expected = Listing::<Test>::FixedPrice(FixedPriceListing::<Test> {
			payment_asset: NativeAssetId::get(),
			fixed_price: 1_500,
			close: System::block_number() + <Test as Config>::DefaultListingDuration::get(),
			buyer: Some(buyer),
			seller: token_owner,
			collection_id,
			serial_numbers: BoundedVec::try_from(vec![token_id.1]).unwrap(),
			royalties_schedule: Default::default(),
			marketplace_id: None,
		});

		let listing = Listings::<Test>::get(listing_id).expect("token is listed");
		assert_eq!(listing, expected);
	});
}

#[test]
fn update_fixed_price_fails() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();

		let reserve_price = 1_000;
		let listing_id = Marketplace::next_listing_id();

		// can't update, token not listed
		assert_noop!(
			Marketplace::update_fixed_price(Some(token_owner).into(), listing_id, 1_500),
			Error::<Test>::NotForFixedPriceSale
		);
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_ok!(Marketplace::auction_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			NativeAssetId::get(),
			reserve_price,
			Some(System::block_number() + 1),
			None,
		));

		// can't update, listed for auction
		assert_noop!(
			Marketplace::update_fixed_price(Some(token_owner).into(), listing_id, 1_500),
			Error::<Test>::NotForFixedPriceSale
		);
	});
}

#[test]
fn update_fixed_price_fails_not_owner() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();
		let listing_id = Marketplace::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		let buyer = create_account(5);
		assert_ok!(Marketplace::sell_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			Some(buyer),
			NativeAssetId::get(),
			1_000,
			None,
			None
		));

		assert_noop!(
			Marketplace::update_fixed_price(Some(buyer).into(), listing_id, 1_500),
			Error::<Test>::NotSeller
		);
	});
}

#[test]
fn register_marketplace() {
	TestExt::default().build().execute_with(|| {
		let account = create_account(1);
		let entitlement: Permill = Permill::from_float(0.1);
		let marketplace_id = Marketplace::next_marketplace_id();
		assert_ok!(Marketplace::register_marketplace(Some(account).into(), None, entitlement));
		System::assert_last_event(MockEvent::Marketplace(Event::<Test>::MarketplaceRegister {
			account,
			entitlement,
			marketplace_id,
		}));
		assert_eq!(Marketplace::next_marketplace_id(), marketplace_id + 1);
	});
}

#[test]
fn register_marketplace_separate_account() {
	TestExt::default().build().execute_with(|| {
		let account = create_account(1);
		let marketplace_account = create_account(2);
		let marketplace_id = Marketplace::next_marketplace_id();
		let entitlement: Permill = Permill::from_float(0.1);

		assert_ok!(Marketplace::register_marketplace(
			Some(account).into(),
			Some(marketplace_account).into(),
			entitlement
		));
		System::assert_last_event(MockEvent::Marketplace(Event::<Test>::MarketplaceRegister {
			account: marketplace_account,
			entitlement,
			marketplace_id,
		}));
	});
}

#[test]
fn buy_with_marketplace_royalties() {
	let buyer = create_account(5);
	let sale_price = 1_000_008;

	TestExt::default()
		.with_balances(&[(buyer, sale_price * 2)])
		.build()
		.execute_with(|| {
			let collection_owner = create_account(1);
			let beneficiary_1 = create_account(11);
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(
					beneficiary_1,
					Permill::from_float(0.1111),
				)]),
			};
			let (collection_id, _, token_owner) =
				setup_token_with_royalties(royalties_schedule.clone(), 2);

			let token_id = (collection_id, 0);

			let marketplace_account = create_account(20);
			let initial_balance_marketplace =
				AssetsExt::reducible_balance(NativeAssetId::get(), &marketplace_account, false);
			let marketplace_entitlement: Permill = Permill::from_float(0.5);
			assert_ok!(Marketplace::register_marketplace(
				Some(marketplace_account).into(),
				Some(marketplace_account).into(),
				marketplace_entitlement
			));
			let marketplace_id = 0;
			let listing_id = Marketplace::next_listing_id();
			assert_eq!(listing_id, 0);
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Marketplace::sell_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				Some(buyer),
				NativeAssetId::get(),
				sale_price,
				None,
				Some(marketplace_id).into(),
			));

			let initial_balance_owner =
				AssetsExt::reducible_balance(NativeAssetId::get(), &collection_owner, false);
			let initial_balance_b1 =
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_1, false);

			assert_ok!(Marketplace::buy(Some(buyer).into(), listing_id));
			let presale_issuance = AssetsExt::total_issuance(NativeAssetId::get());
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &marketplace_account, false),
				initial_balance_marketplace + marketplace_entitlement * sale_price
			);
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_1, false),
				initial_balance_b1 + royalties_schedule.clone().entitlements[0].1 * sale_price
			);
			// token owner gets sale price less royalties
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				initial_balance_owner + sale_price -
					marketplace_entitlement * sale_price -
					royalties_schedule.clone().entitlements[0].1 * sale_price
			);
			assert_eq!(AssetsExt::total_issuance(NativeAssetId::get()), presale_issuance);
		});
}

#[test]
fn list_with_invalid_marketplace_royalties_should_fail() {
	let buyer = create_account(5);
	let sale_price = 1_000_008;

	TestExt::default()
		.with_balances(&[(buyer, sale_price * 2)])
		.build()
		.execute_with(|| {
			let beneficiary_1 = create_account(11);
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(
					beneficiary_1,
					Permill::from_float(0.51),
				)]),
			};
			let (collection_id, _, token_owner) =
				setup_token_with_royalties(royalties_schedule.clone(), 2);

			let token_id = (collection_id, 0);

			let marketplace_account = create_account(20);
			let marketplace_entitlement: Permill = Permill::from_float(0.5);
			assert_ok!(Marketplace::register_marketplace(
				Some(marketplace_account).into(),
				Some(marketplace_account).into(),
				marketplace_entitlement
			));
			let marketplace_id = 0;
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_noop!(
				Marketplace::sell_nft(
					Some(token_owner).into(),
					collection_id,
					serial_numbers,
					Some(buyer),
					NativeAssetId::get(),
					sale_price,
					None,
					Some(marketplace_id).into(),
				),
				Error::<Test>::RoyaltiesInvalid,
			);
		});
}

#[test]
fn buy() {
	let buyer = create_account(5);
	let price = 1_000;

	TestExt::default().with_balances(&[(buyer, price)]).build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();
		let buyer = create_account(5);

		let listing_id = Marketplace::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_ok!(Marketplace::sell_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			Some(buyer),
			NativeAssetId::get(),
			price,
			None,
			None
		));

		assert_ok!(Marketplace::buy(Some(buyer).into(), listing_id));
		// no royalties, all proceeds to token owner
		assert_eq!(AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false), price);

		// listing removed
		assert!(Listings::<Test>::get(listing_id).is_none());
		assert!(Marketplace::listing_end_schedule(
			System::block_number() + <Test as Config>::DefaultListingDuration::get(),
			listing_id
		)
		.is_none());

		// ownership changed
		assert!(Nft::token_locks(&token_id).is_none());
		assert!(Marketplace::open_collection_listings(collection_id, listing_id).is_none());
		assert_eq!(Nft::owned_tokens(collection_id, &buyer, 0, 1000), (0_u32, 1, vec![token_id.1]));
	});
}

#[test]
fn buy_with_royalties() {
	let buyer = create_account(5);
	let sale_price = 1_000_008;

	TestExt::default()
		.with_balances(&[(buyer, sale_price * 2)])
		.build()
		.execute_with(|| {
			let collection_owner = create_account(1);
			let beneficiary_1 = create_account(11);
			let beneficiary_2 = create_account(12);
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![
					(collection_owner, Permill::from_float(0.111)),
					(beneficiary_1, Permill::from_float(0.1111)),
					(beneficiary_2, Permill::from_float(0.3333)),
				]),
			};
			let (collection_id, token_id, token_owner) =
				setup_token_with_royalties(royalties_schedule.clone(), 2);

			let listing_id = Marketplace::next_listing_id();
			assert_eq!(listing_id, 0);
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Marketplace::sell_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				Some(buyer),
				NativeAssetId::get(),
				sale_price,
				None,
				None
			));

			let initial_balance_owner =
				AssetsExt::reducible_balance(NativeAssetId::get(), &collection_owner, false);
			let initial_balance_b1 =
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_1, false);
			let initial_balance_b2 =
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_2, false);
			let initial_balance_seller =
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false);

			assert_ok!(Marketplace::buy(Some(buyer).into(), listing_id));
			let presale_issuance = AssetsExt::total_issuance(NativeAssetId::get());
			// royalties distributed according to `entitlements` map
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &collection_owner, false),
				initial_balance_owner + royalties_schedule.clone().entitlements[0].1 * sale_price
			);
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_1, false),
				initial_balance_b1 + royalties_schedule.clone().entitlements[1].1 * sale_price
			);
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_2, false),
				initial_balance_b2 + royalties_schedule.clone().entitlements[2].1 * sale_price
			);
			// token owner gets sale price less royalties
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				initial_balance_seller + sale_price -
					royalties_schedule
						.clone()
						.entitlements
						.into_iter()
						.map(|(_, e)| e * sale_price)
						.sum::<Balance>()
			);
			assert_eq!(AssetsExt::total_issuance(NativeAssetId::get()), presale_issuance);

			// listing removed
			assert!(Listings::<Test>::get(listing_id).is_none());
			assert!(Marketplace::listing_end_schedule(
				System::block_number() + <Test as Config>::DefaultListingDuration::get(),
				listing_id
			)
			.is_none());

			// ownership changed
			assert_eq!(
				Nft::owned_tokens(collection_id, &buyer, 0, 1000),
				(0_u32, 1, vec![token_id.1])
			);
		});
}

#[test]
fn buy_fails_prechecks() {
	let buyer = create_account(5);
	let price = 1_000;
	TestExt::default()
		.with_balances(&[(buyer, price - 1)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_token();
			let buyer = create_account(5);

			let price = 1_000;
			let listing_id = Marketplace::next_listing_id();

			// not for sale
			assert_noop!(
				Marketplace::buy(Some(buyer).into(), listing_id),
				Error::<Test>::NotForFixedPriceSale,
			);
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Marketplace::sell_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				Some(buyer),
				NativeAssetId::get(),
				price,
				None,
				None
			));

			// no permission
			let not_buyer = create_account(6);
			assert_noop!(
				Marketplace::buy(Some(not_buyer).into(), listing_id),
				Error::<Test>::NotBuyer,
			);

			assert_noop!(
				Marketplace::buy(Some(buyer).into(), listing_id),
				pallet_assets_ext::Error::<Test>::BalanceLow,
			);
		});
}

#[test]
fn sell_to_anybody() {
	let buyer = create_account(5);
	let price = 1_000;
	TestExt::default().with_balances(&[(buyer, price)]).build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();

		let price = 1_000;
		let listing_id = Marketplace::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_ok!(Marketplace::sell_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			None,
			NativeAssetId::get(),
			price,
			None,
			None
		));

		assert_ok!(Marketplace::buy(Some(buyer).into(), listing_id));

		// paid
		assert!(AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false).is_zero());

		// listing removed
		assert!(Listings::<Test>::get(listing_id).is_none());
		assert!(Marketplace::listing_end_schedule(
			System::block_number() + <Test as Config>::DefaultListingDuration::get(),
			listing_id
		)
		.is_none());

		// ownership changed
		assert_eq!(Nft::owned_tokens(collection_id, &buyer, 0, 1000), (0_u32, 1, vec![token_id.1]));
	});
}

#[test]
fn buy_with_overcommitted_royalties() {
	let buyer = create_account(5);
	let price = 1_000;
	TestExt::default().with_balances(&[(buyer, price)]).build().execute_with(|| {
		// royalties are > 100% total which could create funds out of nothing
		// in this case, default to 0 royalties.
		// royalty schedules should not make it into storage but we protect against it anyway
		let (collection_id, token_id, token_owner) = setup_token();
		let bad_schedule = RoyaltiesSchedule {
			entitlements: BoundedVec::truncate_from(vec![
				(11_u64, Permill::from_float(0.125)),
				(12_u64, Permill::from_float(0.9)),
			]),
		};
		let listing_id = Marketplace::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_ok!(Marketplace::sell_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			Some(buyer),
			NativeAssetId::get(),
			price,
			None,
			None
		));

		let presale_issuance = AssetsExt::total_issuance(NativeAssetId::get());

		assert_ok!(Marketplace::buy(Some(buyer).into(), listing_id));
		assert!(bad_schedule.calculate_total_entitlement().is_zero());
		assert_eq!(AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false), price);
		assert!(AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false).is_zero());
		assert_eq!(AssetsExt::total_issuance(NativeAssetId::get()), presale_issuance);
	})
}

#[test]
fn cancel_auction() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();

		let reserve_price = 100_000;
		let listing_id = Marketplace::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_ok!(Marketplace::auction_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			NativeAssetId::get(),
			reserve_price,
			Some(System::block_number() + 1),
			None,
		));

		let new_owner = create_account(6);
		assert_noop!(
			Marketplace::cancel_sale(Some(new_owner).into(), listing_id),
			Error::<Test>::NotSeller
		);

		assert_ok!(Marketplace::cancel_sale(Some(token_owner).into(), listing_id,));

		System::assert_last_event(MockEvent::Marketplace(Event::<Test>::AuctionClose {
			collection_id,
			listing_id,
			reason: AuctionClosureReason::VendorCancelled,
		}));

		// storage cleared up
		assert!(Listings::<Test>::get(listing_id).is_none());
		assert!(Marketplace::listing_end_schedule(System::block_number() + 1, listing_id).is_none());

		// it should be free to operate on the token
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerCollection> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_ok!(Nft::transfer(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			new_owner,
		));
	});
}

#[test]
fn auction_bundle() {
	let buyer = create_account(5);
	let price = 1_000;
	TestExt::default().with_balances(&[(buyer, price)]).build().execute_with(|| {
		let collection_owner = create_account(1);
		let collection_id = Nft::next_collection_uuid().unwrap();
		let quantity = 5;

		assert_ok!(Nft::create_collection(
			Some(collection_owner).into(),
			bounded_string("test-collection"),
			quantity,
			None,
			None,
			MetadataScheme::try_from(b"https://example.com/metadata".as_slice()).unwrap(),
			None,
			CrossChainCompatibility::default(),
		));
		assert_eq!(Nft::token_balance_of(&collection_owner, collection_id), 5);

		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![1, 3, 4]).unwrap();
		let listing_id = Marketplace::next_listing_id();

		assert_ok!(Marketplace::auction_nft(
			Some(collection_owner).into(),
			collection_id,
			serial_numbers.clone(),
			NativeAssetId::get(),
			price,
			Some(1),
			None,
		));

		assert!(Marketplace::open_collection_listings(collection_id, listing_id).unwrap());
		for serial_number in serial_numbers.iter() {
			assert_eq!(
				Nft::token_locks((collection_id, serial_number)).unwrap(),
				TokenLockReason::Listed(listing_id)
			);
		}

		assert_ok!(Marketplace::bid(Some(buyer).into(), listing_id, price));
		// end auction
		let _ =
			Marketplace::on_initialize(System::block_number() + AUCTION_EXTENSION_PERIOD as u64);

		assert_eq!(Nft::owned_tokens(collection_id, &buyer, 0, 1000), (0_u32, 3, vec![1, 3, 4]));
		assert_eq!(Nft::token_balance_of(&collection_owner, collection_id), 2);
		assert_eq!(
			Nft::token_balance_of(&buyer, collection_id),
			serial_numbers.len() as TokenCount
		);
	})
}

#[test]
fn auction_bundle_fails() {
	TestExt::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let collection_id = setup_collection(collection_owner);
		assert_ok!(Nft::mint(Some(collection_owner).into(), collection_id, 2, None));

		// empty tokens fails
		assert_noop!(
			Marketplace::auction_nft(
				Some(collection_owner).into(),
				collection_id,
				Default::default(),
				NativeAssetId::get(),
				1_000,
				None,
				None
			),
			Error::<Test>::NoToken
		);
	})
}

#[test]
fn auction() {
	let bidder_1 = create_account(5);
	let bidder_2 = create_account(6);
	let reserve_price = 100_000;
	let winning_bid = reserve_price + 1;

	TestExt::default()
		.with_balances(&[(bidder_1, reserve_price), (bidder_2, winning_bid)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_token();

			let listing_id = Marketplace::next_listing_id();
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Marketplace::auction_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				NativeAssetId::get(),
				reserve_price,
				Some(1),
				None,
			));
			assert_eq!(Nft::token_locks(&token_id).unwrap(), TokenLockReason::Listed(listing_id));
			assert_eq!(Marketplace::next_listing_id(), listing_id + 1);
			assert!(Marketplace::open_collection_listings(collection_id, listing_id).unwrap());

			// first bidder at reserve price
			assert_ok!(Marketplace::bid(Some(bidder_1).into(), listing_id, reserve_price,));
			assert_eq!(
				AssetsExt::hold_balance(
					&MarketplacePalletId::get(),
					&bidder_1,
					&NativeAssetId::get()
				),
				reserve_price
			);

			// second bidder raises bid
			assert_ok!(Marketplace::bid(Some(bidder_2).into(), listing_id, winning_bid,));
			assert_eq!(
				AssetsExt::hold_balance(
					&MarketplacePalletId::get(),
					&bidder_2,
					&NativeAssetId::get()
				),
				winning_bid
			);
			assert!(AssetsExt::hold_balance(
				&MarketplacePalletId::get(),
				&bidder_1,
				&NativeAssetId::get()
			)
			.is_zero());

			// end auction
			let _ = Marketplace::on_initialize(
				System::block_number() + AUCTION_EXTENSION_PERIOD as u64,
			);

			// no royalties, all proceeds to token owner
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				winning_bid
			);
			// bidder2 funds should be all gone (unreserved and transferred)
			assert!(AssetsExt::reducible_balance(NativeAssetId::get(), &bidder_2, false).is_zero());
			assert!(AssetsExt::hold_balance(
				&MarketplacePalletId::get(),
				&bidder_2,
				&NativeAssetId::get()
			)
			.is_zero());
			// listing metadata removed
			assert!(Listings::<Test>::get(listing_id).is_none());
			assert!(
				Marketplace::listing_end_schedule(System::block_number() + 1, listing_id).is_none()
			);

			// ownership changed
			assert!(Nft::token_locks(&token_id).is_none());
			assert_eq!(
				Nft::owned_tokens(collection_id, &bidder_2, 0, 1000),
				(0_u32, 1, vec![token_id.1])
			);
			assert!(Marketplace::open_collection_listings(collection_id, listing_id).is_none());

			// event logged
			System::assert_last_event(MockEvent::Marketplace(Event::<Test>::AuctionSold {
				collection_id,
				listing_id,
				payment_asset: NativeAssetId::get(),
				hammer_price: winning_bid,
				winner: bidder_2,
			}));
		});
}

#[test]
fn bid_auto_extends() {
	let bidder_1 = create_account(5);
	let reserve_price = 100_000;

	TestExt::default()
		.with_balances(&[(bidder_1, reserve_price)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_token();
			let reserve_price = 100_000;
			let listing_id = Marketplace::next_listing_id();
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Marketplace::auction_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				NativeAssetId::get(),
				reserve_price,
				Some(2),
				None,
			));

			// Place bid
			assert_ok!(Marketplace::bid(Some(bidder_1).into(), listing_id, reserve_price,));

			if let Some(Listing::Auction(listing)) = Listings::<Test>::get(listing_id) {
				assert_eq!(listing.close, System::block_number() + AUCTION_EXTENSION_PERIOD as u64);
			}
			assert!(Marketplace::listing_end_schedule(
				System::block_number() + AUCTION_EXTENSION_PERIOD as u64,
				listing_id
			)
			.unwrap());
		});
}

#[test]
fn auction_royalty_payments() {
	let bidder = create_account(5);
	let reserve_price = 100_004;

	TestExt::default()
		.with_balances(&[(bidder, reserve_price)])
		.build()
		.execute_with(|| {
			let beneficiary_1 = create_account(11);
			let beneficiary_2 = create_account(12);
			let collection_owner = create_account(1);
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![
					(collection_owner, Permill::from_float(0.1111)),
					(beneficiary_1, Permill::from_float(0.1111)),
					(beneficiary_2, Permill::from_float(0.1111)),
				]),
			};
			let (collection_id, token_id, token_owner) =
				setup_token_with_royalties(royalties_schedule.clone(), 1);
			let listing_id = Marketplace::next_listing_id();
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Marketplace::auction_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				NativeAssetId::get(),
				reserve_price,
				Some(1),
				None,
			));

			// first bidder at reserve price
			assert_ok!(Marketplace::bid(Some(bidder).into(), listing_id, reserve_price,));

			// end auction
			let _ = Marketplace::on_initialize(
				System::block_number() + AUCTION_EXTENSION_PERIOD as u64,
			);

			// royalties paid out
			let presale_issuance = AssetsExt::total_issuance(NativeAssetId::get());
			// royalties distributed according to `entitlements` map
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &collection_owner, false),
				royalties_schedule.entitlements[0].1 * reserve_price
			);
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_1, false),
				royalties_schedule.entitlements[1].1 * reserve_price
			);
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &beneficiary_2, false),
				royalties_schedule.entitlements[2].1 * reserve_price
			);
			// token owner gets sale price less royalties
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				reserve_price -
					royalties_schedule
						.entitlements
						.into_iter()
						.map(|(_, e)| e * reserve_price)
						.sum::<Balance>()
			);
			assert!(AssetsExt::reducible_balance(NativeAssetId::get(), &bidder, false).is_zero());
			assert!(AssetsExt::hold_balance(
				&MarketplacePalletId::get(),
				&bidder,
				&NativeAssetId::get()
			)
			.is_zero());

			assert_eq!(AssetsExt::total_issuance(NativeAssetId::get()), presale_issuance);

			// listing metadata removed
			assert!(!Listings::<Test>::contains_key(listing_id));
			assert!(!ListingEndSchedule::<Test>::contains_key(
				System::block_number() + 1,
				listing_id,
			));

			// ownership changed
			assert_eq!(
				Nft::owned_tokens(collection_id, &bidder, 0, 1000),
				(0_u32, 1, vec![token_id.1])
			);
		});
}

#[test]
fn close_listings_at_removes_listing_data() {
	TestExt::default().build().execute_with(|| {
		let collection_id = Nft::next_collection_uuid().unwrap();
		let price = 123_456;
		let token_1 = (collection_id, 0);
		let seller = create_account(1);
		let listings = vec![
			// an open sale which won't be bought before closing
			Listing::<Test>::FixedPrice(FixedPriceListing::<Test> {
				payment_asset: NativeAssetId::get(),
				fixed_price: price,
				buyer: None,
				close: System::block_number() + 1,
				seller: seller.clone(),
				collection_id,
				serial_numbers: BoundedVec::try_from(vec![token_1.1]).unwrap(),
				royalties_schedule: Default::default(),
				marketplace_id: None,
			}),
			// an open auction which has no bids before closing
			Listing::<Test>::Auction(AuctionListing::<Test> {
				payment_asset: NativeAssetId::get(),
				reserve_price: price,
				close: System::block_number() + 1,
				seller: seller.clone(),
				collection_id,
				serial_numbers: BoundedVec::try_from(vec![token_1.1]).unwrap(),
				royalties_schedule: Default::default(),
				marketplace_id: None,
			}),
			// an open auction which has a winning bid before closing
			Listing::<Test>::Auction(AuctionListing::<Test> {
				payment_asset: NativeAssetId::get(),
				reserve_price: price,
				close: System::block_number() + 1,
				seller: seller.clone(),
				collection_id,
				serial_numbers: BoundedVec::try_from(vec![token_1.1]).unwrap(),
				royalties_schedule: Default::default(),
				marketplace_id: None,
			}),
		];

		// setup listings storage
		for (listing_id, listing) in listings.iter().enumerate() {
			let listing_id = listing_id as ListingId;
			Listings::<Test>::insert(listing_id, listing.clone());
			ListingEndSchedule::<Test>::insert(System::block_number() + 1, listing_id, true);
		}
		// winning bidder has no funds, this should cause settlement failure
		ListingWinningBid::<Test>::insert(2, (create_account(11), 100u128));

		// Close the listings
		Marketplace::close_listings_at(System::block_number() + 1);

		// Storage clear
		assert!(ListingEndSchedule::<Test>::iter_prefix_values(System::block_number() + 1)
			.count()
			.is_zero());
		for listing_id in 0..listings.len() as ListingId {
			assert!(Listings::<Test>::get(listing_id).is_none());
			assert!(Marketplace::listing_winning_bid(listing_id).is_none());
			assert!(
				Marketplace::listing_end_schedule(System::block_number() + 1, listing_id).is_none()
			);
		}
	});
}

#[test]
fn auction_fails_prechecks() {
	TestExt::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_token();

		let reserve_price = 100_000;

		// token doesn't exist
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![2]).unwrap();
		assert_noop!(
			Marketplace::auction_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				NativeAssetId::get(),
				reserve_price,
				Some(1),
				None,
			),
			Error::<Test>::NotTokenOwner
		);

		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		// not owner
		let bob = create_account(6);
		assert_noop!(
			Marketplace::auction_nft(
				Some(bob).into(),
				collection_id,
				serial_numbers.clone(),
				NativeAssetId::get(),
				reserve_price,
				Some(1),
				None,
			),
			Error::<Test>::NotTokenOwner
		);

		// setup listed token, and try list it again
		assert_ok!(Marketplace::auction_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers.clone(),
			NativeAssetId::get(),
			reserve_price,
			Some(1),
			None,
		));
		// already listed
		assert_noop!(
			Marketplace::auction_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers.clone(),
				NativeAssetId::get(),
				reserve_price,
				Some(1),
				None,
			),
			Error::<Test>::TokenLocked
		);

		// listed for auction
		assert_noop!(
			Marketplace::sell_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				None,
				NativeAssetId::get(),
				reserve_price,
				None,
				None,
			),
			Error::<Test>::TokenLocked
		);
	});
}

#[test]
fn bid_fails_prechecks() {
	let bidder = create_account(5);
	let reserve_price = 100_004;

	TestExt::default()
		.with_balances(&[(bidder, reserve_price)])
		.build()
		.execute_with(|| {
			let missing_listing_id = 5;
			assert_noop!(
				Marketplace::bid(Some(create_account(1)).into(), missing_listing_id, 100),
				Error::<Test>::NotForAuction
			);

			let (collection_id, token_id, token_owner) = setup_token();
			let listing_id = Marketplace::next_listing_id();
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Marketplace::auction_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				NativeAssetId::get(),
				reserve_price,
				Some(1),
				None,
			));

			// < reserve
			assert_noop!(
				Marketplace::bid(Some(bidder).into(), listing_id, reserve_price - 1),
				Error::<Test>::BidTooLow
			);

			// balance already reserved for other reasons
			assert_ok!(AssetsExt::place_hold(
				MarketplacePalletId::get(),
				&bidder,
				NativeAssetId::get(),
				reserve_price
			));
			assert_noop!(
				Marketplace::bid(Some(bidder).into(), listing_id, reserve_price),
				pallet_balances::Error::<Test>::InsufficientBalance
			);
			assert_ok!(AssetsExt::release_hold(
				MarketplacePalletId::get(),
				&bidder,
				NativeAssetId::get(),
				reserve_price
			));

			// <= current bid
			assert_ok!(Marketplace::bid(Some(bidder).into(), listing_id, reserve_price,));
			assert_noop!(
				Marketplace::bid(Some(bidder).into(), listing_id, reserve_price),
				Error::<Test>::BidTooLow
			);
		});
}

#[test]
fn bid_no_balance_should_fail() {
	let bidder = create_account(5);

	TestExt::default().build().execute_with(|| {
		let missing_listing_id = 5;
		assert_noop!(
			Marketplace::bid(Some(create_account(1)).into(), missing_listing_id, 100),
			Error::<Test>::NotForAuction
		);

		let (collection_id, token_id, token_owner) = setup_token();
		let reserve_price = 100_000;
		let listing_id = Marketplace::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_ok!(Marketplace::auction_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers,
			NativeAssetId::get(),
			reserve_price,
			Some(1),
			None,
		));

		// no free balance
		assert_noop!(
			Marketplace::bid(Some(bidder).into(), listing_id, reserve_price),
			pallet_balances::Error::<Test>::InsufficientBalance
		);
	});
}

#[test]
fn make_simple_offer() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, _) = setup_token();
			let offer_amount: Balance = 100;
			let (offer_id, _) = make_new_simple_offer(offer_amount, token_id, buyer, None);
			assert_eq!(Marketplace::token_offers(token_id).unwrap(), vec![offer_id]);
			// Check funds have been locked
			assert_eq!(
				AssetsExt::hold_balance(&MarketplacePalletId::get(), &buyer, &NativeAssetId::get()),
				offer_amount
			);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &buyer),
				initial_balance_buyer - offer_amount
			);
		});
}

#[test]
fn make_simple_offer_insufficient_funds_should_fail() {
	TestExt::default().build().execute_with(|| {
		let (_, token_id, _) = setup_token();
		let buyer = create_account(3);
		let offer_amount: Balance = 100;
		assert_eq!(AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false), 0);

		assert_noop!(
			Marketplace::make_simple_offer(
				Some(buyer).into(),
				token_id,
				offer_amount,
				NativeAssetId::get(),
				None
			),
			pallet_balances::Error::<Test>::InsufficientBalance
		);
	});
}

#[test]
fn make_simple_offer_zero_amount_should_fail() {
	TestExt::default().build().execute_with(|| {
		let (_, token_id, _) = setup_token();
		let buyer = create_account(3);
		let offer_amount: Balance = 0;
		assert_eq!(AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false), 0);

		assert_noop!(
			Marketplace::make_simple_offer(
				Some(buyer).into(),
				token_id,
				offer_amount,
				NativeAssetId::get(),
				None
			),
			Error::<Test>::ZeroOffer
		);
	});
}

#[test]
fn make_simple_offer_token_owner_should_fail() {
	TestExt::default().build().execute_with(|| {
		let (_, token_id, token_owner) = setup_token();
		let offer_amount: Balance = 100;

		assert_noop!(
			Marketplace::make_simple_offer(
				Some(token_owner).into(),
				token_id,
				offer_amount,
				NativeAssetId::get(),
				None
			),
			Error::<Test>::IsTokenOwner
		);
	});
}

#[test]
fn make_simple_offer_on_fixed_price_listing() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_token();
			let offer_amount: Balance = 100;
			let sell_price = 100_000;
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			let listing_id = Marketplace::next_listing_id();

			assert_ok!(Marketplace::sell_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				None,
				NativeAssetId::get(),
				sell_price,
				None,
				None,
			));
			// Sanity check
			assert!(Listings::<Test>::get(listing_id).is_some());
			assert!(Nft::token_locks(token_id).is_some());

			let (offer_id, _) = make_new_simple_offer(offer_amount, token_id, buyer, None);
			// Check funds have been locked
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false),
				initial_balance_buyer - offer_amount
			);
			assert_eq!(
				AssetsExt::hold_balance(&MarketplacePalletId::get(), &buyer, &NativeAssetId::get()),
				offer_amount
			);

			assert_ok!(Marketplace::accept_offer(Some(token_owner).into(), offer_id,));

			// Check that fixed price listing and locks are now removed
			assert!(Listings::<Test>::get(listing_id).is_none());
			assert!(Nft::token_locks(token_id).is_none());
			// Check offer storage has been removed
			assert!(Marketplace::token_offers(token_id).is_none());
			assert!(Marketplace::offers(offer_id).is_none());

			// Check funds have been transferred
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false),
				initial_balance_buyer - offer_amount
			);
			assert!(AssetsExt::hold_balance(
				&MarketplacePalletId::get(),
				&buyer,
				&NativeAssetId::get()
			)
			.is_zero());
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				offer_amount
			);
		});
}

#[test]
fn make_simple_offer_on_auction_should_fail() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_token();
			let offer_amount: Balance = 100;
			let reserve_price = 100_000;
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Marketplace::auction_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				NativeAssetId::get(),
				reserve_price,
				Some(System::block_number() + 1),
				None,
			));

			assert_noop!(
				Marketplace::make_simple_offer(
					Some(buyer).into(),
					token_id,
					offer_amount,
					NativeAssetId::get(),
					None
				),
				Error::<Test>::TokenOnAuction
			);
		});
}

#[test]
fn cancel_offer() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, _) = setup_token();
			let offer_amount: Balance = 100;

			let (offer_id, _) = make_new_simple_offer(offer_amount, token_id, buyer, None);
			assert_ok!(Marketplace::cancel_offer(Some(buyer).into(), offer_id));

			System::assert_last_event(MockEvent::Marketplace(Event::<Test>::OfferCancel {
				offer_id,
				token_id,
			}));

			// Check storage has been removed
			assert!(Marketplace::token_offers(token_id).is_none());
			assert_eq!(Marketplace::offers(offer_id), None);
			// Check funds have been unlocked after offer cancelled
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false),
				initial_balance_buyer
			);
			assert!(AssetsExt::hold_balance(
				&MarketplacePalletId::get(),
				&buyer,
				&NativeAssetId::get()
			)
			.is_zero());
		});
}

#[test]
fn cancel_offer_multiple_offers() {
	let buyer_1 = create_account(3);
	let buyer_2 = create_account(4);
	let initial_balance_buyer_1: Balance = 1000;
	let initial_balance_buyer_2: Balance = 1000;

	TestExt::default()
		.with_balances(&[(buyer_1, initial_balance_buyer_1), (buyer_2, initial_balance_buyer_2)])
		.build()
		.execute_with(|| {
			let (_, token_id, _) = setup_token();

			let offer_amount_1: Balance = 100;
			let offer_amount_2: Balance = 150;

			let (offer_id_1, _) = make_new_simple_offer(offer_amount_1, token_id, buyer_1, None);
			let (offer_id_2, offer_2) =
				make_new_simple_offer(offer_amount_2, token_id, buyer_2, None);

			// Can't cancel other offer
			assert_noop!(
				Marketplace::cancel_offer(Some(buyer_1).into(), offer_id_2),
				Error::<Test>::NotBuyer
			);
			// Can cancel their offer
			assert_ok!(Marketplace::cancel_offer(Some(buyer_1).into(), offer_id_1));
			System::assert_last_event(MockEvent::Marketplace(Event::<Test>::OfferCancel {
				offer_id: offer_id_1,
				token_id,
			}));

			// Check storage has been removed
			let offer_vector: Vec<OfferId> = vec![offer_id_2];
			assert_eq!(Marketplace::token_offers(token_id).unwrap(), offer_vector);
			assert_eq!(Marketplace::offers(offer_id_2), Some(OfferType::Simple(offer_2.clone())));
			assert_eq!(Marketplace::offers(offer_id_1), None);

			// Check funds have been unlocked after offer cancelled
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer_1, false),
				initial_balance_buyer_1
			);
			assert!(AssetsExt::hold_balance(
				&MarketplacePalletId::get(),
				&buyer_1,
				&NativeAssetId::get()
			)
			.is_zero());
			// Check buyer_2 funds have not been unlocked
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer_2, false),
				initial_balance_buyer_2 - offer_amount_2
			);
			assert_eq!(
				AssetsExt::hold_balance(
					&MarketplacePalletId::get(),
					&buyer_2,
					&NativeAssetId::get()
				),
				offer_amount_2
			);
		});
}

#[test]
fn cancel_offer_not_buyer_should_fail() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, _) = setup_token();
			let offer_amount: Balance = 100;
			let (offer_id, _) = make_new_simple_offer(offer_amount, token_id, buyer, None);

			assert_noop!(
				Marketplace::cancel_offer(Some(create_account(4)).into(), offer_id),
				Error::<Test>::NotBuyer
			);
		});
}

#[test]
fn accept_offer() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, token_owner) = setup_token();
			let offer_amount: Balance = 100;
			let (offer_id, _) = make_new_simple_offer(offer_amount, token_id, buyer, None);
			assert_ok!(Marketplace::accept_offer(Some(token_owner).into(), offer_id));
			System::assert_last_event(MockEvent::Marketplace(Event::<Test>::OfferAccept {
				offer_id,
				token_id,
				amount: offer_amount,
				asset_id: NativeAssetId::get(),
			}));

			// Check storage has been removed
			assert!(Marketplace::token_offers(token_id).is_none());
			assert!(Marketplace::offers(offer_id).is_none());
			// Check funds have been transferred
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false),
				initial_balance_buyer - offer_amount
			);
			assert!(AssetsExt::hold_balance(
				&MarketplacePalletId::get(),
				&buyer,
				&NativeAssetId::get()
			)
			.is_zero());
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				offer_amount
			);
		});
}

#[test]
fn accept_offer_multiple_offers() {
	let buyer_1 = create_account(3);
	let buyer_2 = create_account(4);
	let initial_balance_buyer_1: Balance = 1000;
	let initial_balance_buyer_2: Balance = 1000;

	TestExt::default()
		.with_balances(&[(buyer_1, initial_balance_buyer_1), (buyer_2, initial_balance_buyer_2)])
		.build()
		.execute_with(|| {
			let (_, token_id, token_owner) = setup_token();

			let offer_amount_1: Balance = 100;
			let offer_amount_2: Balance = 150;

			let (offer_id_1, offer_1) =
				make_new_simple_offer(offer_amount_1, token_id, buyer_1, None);
			let (offer_id_2, _) = make_new_simple_offer(offer_amount_2, token_id, buyer_2, None);

			// Accept second offer
			assert_ok!(Marketplace::accept_offer(Some(token_owner).into(), offer_id_2));
			System::assert_last_event(MockEvent::Marketplace(Event::<Test>::OfferAccept {
				offer_id: offer_id_2,
				token_id,
				amount: offer_amount_2,
				asset_id: NativeAssetId::get(),
			}));
			// Check storage has been removed
			let offer_vector: Vec<OfferId> = vec![offer_id_1];
			assert_eq!(Marketplace::token_offers(token_id).unwrap(), offer_vector);
			assert_eq!(Marketplace::offers(offer_id_1), Some(OfferType::Simple(offer_1.clone())));
			assert_eq!(Marketplace::offers(offer_id_2), None);

			// Check funds have been transferred
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer_2, false),
				initial_balance_buyer_2 - offer_amount_2
			);
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer_1, false),
				initial_balance_buyer_1 - offer_amount_1
			);
			assert_eq!(
				AssetsExt::hold_balance(
					&MarketplacePalletId::get(),
					&buyer_1,
					&NativeAssetId::get()
				),
				offer_amount_1
			);
			assert!(AssetsExt::hold_balance(
				&MarketplacePalletId::get(),
				&buyer_2,
				&NativeAssetId::get()
			)
			.is_zero());
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				offer_amount_2
			);

			// Accept first offer should fail as token_owner is no longer owner
			assert_noop!(
				Marketplace::accept_offer(Some(token_owner).into(), offer_id_1),
				Error::<Test>::NotTokenOwner
			);
		});
}

#[test]
fn accept_offer_pays_marketplace_royalties() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, token_owner) = setup_token();
			let offer_amount: Balance = 100;

			let marketplace_account = create_account(4);
			let entitlements: Permill = Permill::from_float(0.1);
			let marketplace_id = Marketplace::next_marketplace_id();
			assert_ok!(Marketplace::register_marketplace(
				Some(marketplace_account).into(),
				None,
				entitlements
			));

			let (offer_id, _) =
				make_new_simple_offer(offer_amount, token_id, buyer, Some(marketplace_id));
			assert_ok!(Marketplace::accept_offer(Some(token_owner).into(), offer_id));

			// Check storage has been removed
			assert!(Marketplace::token_offers(token_id).is_none());
			assert_eq!(Marketplace::offers(offer_id), None);
			// Check funds have been transferred with royalties
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &buyer, false),
				initial_balance_buyer - offer_amount
			);
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &marketplace_account, false),
				entitlements * offer_amount
			);
			assert!(AssetsExt::hold_balance(
				&MarketplacePalletId::get(),
				&buyer,
				&NativeAssetId::get()
			)
			.is_zero());
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &token_owner, false),
				offer_amount - (entitlements * offer_amount)
			);
		});
}

#[test]
fn accept_offer_not_token_owner_should_fail() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, _) = setup_token();
			let offer_amount: Balance = 100;

			let (offer_id, _) = make_new_simple_offer(offer_amount, token_id, buyer, None);
			assert_noop!(
				Marketplace::accept_offer(Some(create_account(4)).into(), offer_id),
				Error::<Test>::NotTokenOwner
			);
		});
}
