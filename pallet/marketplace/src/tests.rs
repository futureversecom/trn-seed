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
	AssetsExt, DefaultListingDuration, FeePotId, Marketplace, MarketplaceNetworkFeePercentage,
	MarketplacePalletId, MaxTokensPerCollection, MaxTokensPerListing, NativeAssetId, Nft,
	RuntimeEvent as MockEvent, Sft, System, Test,
};
use core::ops::Mul;
use frame_support::traits::{fungibles::Inspect, OnInitialize};
use pallet_nft::{CrossChainCompatibility, TokenLocks};
use pallet_sft::{test_utils::sft_balance_of, TokenInfo};
use seed_pallet_common::test_prelude::*;
use seed_primitives::{MetadataScheme, RoyaltiesSchedule, TokenCount};
use sp_runtime::traits::{AccountIdConversion, Zero};

// Create an NFT collection
// Returns the created `collection_id`
fn create_nft_collection(owner: AccountId) -> CollectionUuid {
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

// Create an SFT collection
// Returns the created `collection_id`
fn create_sft_collection(owner: AccountId) -> CollectionUuid {
	let collection_id = Nft::next_collection_uuid().unwrap();
	let collection_name = bounded_string("test-sft-collection");
	let metadata_scheme = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
	assert_ok!(Sft::create_collection(
		Some(owner).into(),
		collection_name,
		None,
		metadata_scheme,
		None,
	));
	collection_id
}

/// Setup an SFT token, return collection id, token id, token owner
fn setup_sft_token(initial_issuance: Balance) -> (CollectionUuid, TokenId, AccountId) {
	let collection_owner = create_account(100);
	let collection_id = create_sft_collection(collection_owner);
	let token_name = bounded_string("test-sft-token");
	let token_owner = create_account(200);
	let token_id = (collection_id, 0);
	assert_ok!(Sft::create_token(
		Some(collection_owner).into(),
		collection_id,
		token_name,
		initial_issuance,
		None,
		Some(token_owner)
	));

	// Check free balance is correct
	let token_info = TokenInfo::<Test>::get(token_id).unwrap();
	assert_eq!(token_info.free_balance_of(&token_owner), initial_issuance);

	(collection_id, token_id, token_owner)
}

/// Setup an SFT token, return collection id, token id, token owner
fn setup_sft_token_with_royalties(
	initial_issuance: Balance,
	royalties: RoyaltiesSchedule<AccountId>,
) -> (CollectionUuid, TokenId, AccountId) {
	let collection_owner = create_account(100);
	let collection_id = Nft::next_collection_uuid().unwrap();
	let collection_name = bounded_string("test-sft-collection");
	let metadata_scheme = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
	assert_ok!(Sft::create_collection(
		Some(collection_owner).into(),
		collection_name,
		None,
		metadata_scheme,
		Some(royalties),
	));

	let token_name = bounded_string("test-sft-token");
	let token_owner = create_account(200);
	let token_id = (collection_id, 0);
	assert_ok!(Sft::create_token(
		Some(collection_owner).into(),
		collection_id,
		token_name,
		initial_issuance,
		None,
		Some(token_owner)
	));

	// Check free balance is correct
	let token_info = TokenInfo::<Test>::get(token_id).unwrap();
	assert_eq!(token_info.free_balance_of(&token_owner), initial_issuance);

	(collection_id, token_id, token_owner)
}

/// Setup an NFT token, return collection id, token id, token owner
fn setup_nft_token() -> (CollectionUuid, TokenId, AccountId) {
	let collection_owner = create_account(1);
	let collection_id = create_nft_collection(collection_owner);
	let token_owner = create_account(2);
	let token_id = (collection_id, 0);
	assert_ok!(Nft::mint(Some(collection_owner).into(), collection_id, 1, Some(token_owner)));

	(collection_id, token_id, token_owner)
}

/// Setup a token, return collection id, token id, token owner
fn setup_nft_token_with_royalties(
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
	let initial_balance = 11_111_225;

	TestExt::<Test>::default()
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
					TokenLocks::<Test>::get((collection_id, serial_number)).unwrap(),
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

			let fee_pot_account: AccountId = FeePotId::get().into_account_truncating();
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &fee_pot_account),
				5, // 0.5% of 1000
			);
		})
}

#[test]
fn sell_with_empty_royalties() {
	let buyer = create_account(3);
	let initial_balance = 11_111_225;

	TestExt::<Test>::default()
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

			// Remove fee to account so there are no royalties on listed token
			// i.e. 100% of sale price goes to seller
			assert_ok!(Marketplace::set_fee_to(RawOrigin::Root.into(), None));

			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
				BoundedVec::try_from(vec![1, 3, 4]).unwrap();

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
		})
}

#[test]
fn sell_multiple_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let collection_id = create_nft_collection(collection_owner);
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
			Error::<Test>::EmptyTokens
		);
	})
}

#[test]
fn sell_multiple() {
	TestExt::<Test>::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_nft_token();
		let listing_id = Marketplace::next_listing_id();

		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		let buyer = create_account(5);
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
		let tokens = ListingTokens::Nft(NftListing { collection_id, serial_numbers });
		System::assert_last_event(MockEvent::Marketplace(Event::<Test>::FixedPriceSaleList {
			tokens: tokens.clone(),
			listing_id,
			marketplace_id: None,
			price: 1_000,
			payment_asset: NativeAssetId::get(),
			seller: token_owner,
			close: System::block_number() + DefaultListingDuration::get(),
		}));

		assert_eq!(TokenLocks::<Test>::get(token_id).unwrap(), TokenLockReason::Listed(listing_id));
		assert!(Marketplace::open_collection_listings(collection_id, listing_id).unwrap());

		let fee_pot_account: AccountId = FeePotId::get().into_account_truncating();
		let royalties_schedule = RoyaltiesSchedule {
			entitlements: BoundedVec::truncate_from(vec![(
				fee_pot_account,
				MarketplaceNetworkFeePercentage::get(),
			)]),
		};
		let expected = Listing::<Test>::FixedPrice(FixedPriceListing::<Test> {
			payment_asset: NativeAssetId::get(),
			fixed_price: 1_000,
			close: System::block_number() + <Test as Config>::DefaultListingDuration::get(),
			buyer: Some(buyer),
			tokens,
			seller: token_owner,
			royalties_schedule,
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
	TestExt::<Test>::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_nft_token();
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
			pallet_nft::Error::<Test>::NotTokenOwner
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
			pallet_nft::Error::<Test>::TokenLocked
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
			pallet_nft::Error::<Test>::TokenLocked
		);
	});
}

#[test]
fn sell_zero_duration_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_nft_token();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_noop!(
			Marketplace::sell_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers.clone(),
				Some(create_account(5)),
				NativeAssetId::get(),
				1_000,
				Some(0), // Invalid duration
				None,
			),
			Error::<Test>::DurationTooShort
		);
	});
}

#[test]
fn cancel_sell() {
	TestExt::<Test>::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_nft_token();
		let listing_id = Marketplace::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		let buyer = create_account(5);
		assert_ok!(Marketplace::sell_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers.clone(),
			Some(buyer),
			NativeAssetId::get(),
			1_000,
			None,
			None
		));
		assert_ok!(Marketplace::cancel_sale(Some(token_owner).into(), listing_id));
		let tokens = ListingTokens::Nft(NftListing { collection_id, serial_numbers });
		System::assert_last_event(MockEvent::Marketplace(Event::<Test>::FixedPriceSaleClose {
			tokens,
			listing_id,
			marketplace_id: None,
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
	TestExt::<Test>::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_nft_token();
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
fn listing_price_splits_royalties_and_network_fee() {
	let buyer = create_account(5);
	let price = 1_000_000;
	let starting_balance = price * 2;
	let entitlement_amount = Permill::from_float(0.25);

	TestExt::<Test>::default()
		.with_balances(&[(buyer, starting_balance)])
		.build()
		.execute_with(|| {
			let beneficiary_1 = create_account(11);

			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(beneficiary_1, entitlement_amount)]),
			};
			let (collection_id, token_id, token_owner) =
				setup_nft_token_with_royalties(royalties_schedule.clone(), 2);

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

			// Buyer balance should be starting minus 1_000_000
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &buyer), starting_balance - price);

			// Owner balance should be 1_000_000 minus 25.5% of 1_000_000
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &token_owner),
				price
					- entitlement_amount.mul(price)
					- MarketplaceNetworkFeePercentage::get().mul(price)
			);

			// Beneficiary balance should be 25% of 1_000_000
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &beneficiary_1),
				entitlement_amount.mul(price)
			);

			let fee_pot_account: AccountId = FeePotId::get().into_account_truncating();
			// Network fee should be 0.5% of 1_000_000
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &fee_pot_account),
				MarketplaceNetworkFeePercentage::get().mul(price)
			);
		});
}

#[test]
fn listing_price_splits_multiple_royalties_and_network_fee() {
	let buyer = create_account(5);
	let price = 1_000_000;
	let starting_balance = price * 2;
	let entitlement_amount = Permill::from_float(0.25);
	let entitlement_amount_beneficiary_2 = Permill::from_float(0.5);

	TestExt::<Test>::default()
		.with_balances(&[(buyer, starting_balance)])
		.build()
		.execute_with(|| {
			let beneficiary_1 = create_account(11);
			let beneficiary_2 = create_account(22);

			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![
					(beneficiary_1, entitlement_amount),
					(beneficiary_2, entitlement_amount_beneficiary_2),
				]),
			};
			let (collection_id, token_id, token_owner) =
				setup_nft_token_with_royalties(royalties_schedule.clone(), 2);

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

			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &buyer), starting_balance - price);

			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &token_owner),
				price
					- (entitlement_amount.mul(price)
						+ entitlement_amount_beneficiary_2.mul(price)
						+ MarketplaceNetworkFeePercentage::get().mul(price))
			);

			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &beneficiary_1),
				entitlement_amount.mul(price)
			);

			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &beneficiary_2),
				entitlement_amount_beneficiary_2.mul(price)
			);

			let fee_pot_account: AccountId = FeePotId::get().into_account_truncating();

			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &fee_pot_account),
				MarketplaceNetworkFeePercentage::get().mul(price)
			);
		});
}

#[test]
fn network_fee_royalties_split_is_respected_xrpl() {
	let buyer = create_account(5);
	let price = 1_000_000;
	let starting_balance = price * 2;
	let entitlement_amount = Permill::from_float(0.25);
	let asset_used = XRP_ASSET_ID;

	TestExt::<Test>::default()
		.with_xrp_balances(&[(buyer, starting_balance)])
		.build()
		.execute_with(|| {
			let beneficiary_1 = create_account(11);

			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(beneficiary_1, entitlement_amount)]),
			};
			let (collection_id, token_id, token_owner) =
				setup_nft_token_with_royalties(royalties_schedule.clone(), 2);

			let listing_id = Marketplace::next_listing_id();
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Marketplace::sell_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				Some(buyer),
				asset_used,
				price,
				None,
				None
			));

			assert_ok!(Marketplace::buy(Some(buyer).into(), listing_id));
			assert_eq!(
				AssetsExt::balance(asset_used, &token_owner),
				price
					- entitlement_amount.mul(price)
					- MarketplaceNetworkFeePercentage::get().mul(price)
			);

			assert_eq!(
				AssetsExt::balance(asset_used, &beneficiary_1),
				entitlement_amount.mul(price)
			);

			let fee_pot_account: AccountId = FeePotId::get().into_account_truncating();

			assert_eq!(
				AssetsExt::balance(asset_used, &fee_pot_account),
				MarketplaceNetworkFeePercentage::get().mul(price)
			);
		});
}

#[test]
fn update_fixed_price() {
	TestExt::<Test>::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_nft_token();
		let listing_id = Marketplace::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		let buyer = create_account(5);
		assert_ok!(Marketplace::sell_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers.clone(),
			Some(buyer),
			NativeAssetId::get(),
			1_000,
			None,
			None
		));
		assert_ok!(Marketplace::update_fixed_price(Some(token_owner).into(), listing_id, 1_500));
		let tokens = ListingTokens::Nft(NftListing { collection_id, serial_numbers });
		System::assert_last_event(MockEvent::Marketplace(
			Event::<Test>::FixedPriceSalePriceUpdate {
				tokens: tokens.clone(),
				listing_id,
				marketplace_id: None,
				new_price: 1_500,
			},
		));

		let fee_pot_account: AccountId = FeePotId::get().into_account_truncating();
		let royalties_schedule = RoyaltiesSchedule {
			entitlements: BoundedVec::truncate_from(vec![(
				fee_pot_account,
				MarketplaceNetworkFeePercentage::get(),
			)]),
		};
		let expected = Listing::<Test>::FixedPrice(FixedPriceListing::<Test> {
			payment_asset: NativeAssetId::get(),
			fixed_price: 1_500,
			close: System::block_number() + <Test as Config>::DefaultListingDuration::get(),
			buyer: Some(buyer),
			seller: token_owner,
			tokens,
			royalties_schedule,
			marketplace_id: None,
		});

		let listing = Listings::<Test>::get(listing_id).expect("token is listed");
		assert_eq!(listing, expected);
	});
}

#[test]
fn update_fixed_price_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_nft_token();

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
	TestExt::<Test>::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_nft_token();
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
	TestExt::<Test>::default().build().execute_with(|| {
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
	TestExt::<Test>::default().build().execute_with(|| {
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

	TestExt::<Test>::default()
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
				setup_nft_token_with_royalties(royalties_schedule.clone(), 2);

			let token_id = (collection_id, 0);

			let marketplace_account = create_account(20);
			let initial_balance_marketplace =
				AssetsExt::balance(NativeAssetId::get(), &marketplace_account);
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

			let initial_balance_owner = AssetsExt::balance(NativeAssetId::get(), &collection_owner);
			let initial_balance_b1 = AssetsExt::balance(NativeAssetId::get(), &beneficiary_1);

			assert_ok!(Marketplace::buy(Some(buyer).into(), listing_id));
			let presale_issuance = AssetsExt::total_issuance(NativeAssetId::get());
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &marketplace_account),
				initial_balance_marketplace + marketplace_entitlement * sale_price
			);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &beneficiary_1),
				initial_balance_b1 + royalties_schedule.clone().entitlements[0].1 * sale_price
			);
			// token owner gets:
			// sale_price - (marketplace_royalties + beneficiary_royalties + network_fee)
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &token_owner),
				initial_balance_owner + sale_price
					- marketplace_entitlement * sale_price
					- royalties_schedule.clone().entitlements[0].1 * sale_price
					- MarketplaceNetworkFeePercentage::get().mul(sale_price)
			);
			assert_eq!(AssetsExt::total_issuance(NativeAssetId::get()), presale_issuance);
		});
}

#[test]
fn list_with_invalid_marketplace_royalties_should_fail() {
	let buyer = create_account(5);
	let sale_price = 1_000_008;

	TestExt::<Test>::default()
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
				setup_nft_token_with_royalties(royalties_schedule.clone(), 2);

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
	let starting_balance = price * 2;

	TestExt::<Test>::default()
		.with_balances(&[(buyer, starting_balance)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_nft_token();
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
			// no royalties, all proceeds to token owner minus network fee
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &token_owner),
				price - MarketplaceNetworkFeePercentage::get().mul(price)
			);
			// Buyer balance should be starting minus price (1000)
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &buyer), starting_balance - price);

			// listing removed
			assert!(Listings::<Test>::get(listing_id).is_none());
			assert!(Marketplace::listing_end_schedule(
				System::block_number() + <Test as Config>::DefaultListingDuration::get(),
				listing_id
			)
			.is_none());

			// ownership changed
			assert!(TokenLocks::<Test>::get(&token_id).is_none());
			assert!(Marketplace::open_collection_listings(collection_id, listing_id).is_none());
			assert_eq!(
				Nft::owned_tokens(collection_id, &buyer, 0, 1000),
				(0_u32, 1, vec![token_id.1])
			);

			// assert network fees accumulated
			let fee_pot_account: AccountId = FeePotId::get().into_account_truncating();

			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &fee_pot_account),
				5, // 0.5% of 1000
			);
		});
}

#[test]
fn buy_with_xrp() {
	let buyer = create_account(5);
	let price = 1_000;

	TestExt::<Test>::default()
		.with_asset(XRP_ASSET_ID, "XRP", &[(buyer, price)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_nft_token();
			let buyer = create_account(5);

			let listing_id = Marketplace::next_listing_id();
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Marketplace::sell_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers,
				Some(buyer),
				XRP_ASSET_ID,
				price,
				None,
				None
			));

			assert_ok!(Marketplace::buy(Some(buyer).into(), listing_id));
			// no royalties, all proceeds to token owner minus network fee
			assert_eq!(
				AssetsExt::balance(XRP_ASSET_ID, &token_owner),
				price - MarketplaceNetworkFeePercentage::get().mul(price)
			);
			// Buyer balance should be 0
			assert_eq!(AssetsExt::balance(XRP_ASSET_ID, &buyer), 0);

			// listing removed
			assert!(Listings::<Test>::get(listing_id).is_none());
			assert!(Marketplace::listing_end_schedule(
				System::block_number() + <Test as Config>::DefaultListingDuration::get(),
				listing_id
			)
			.is_none());

			// ownership changed
			assert!(TokenLocks::<Test>::get(&token_id).is_none());
			assert!(Marketplace::open_collection_listings(collection_id, listing_id).is_none());
			assert_eq!(
				Nft::owned_tokens(collection_id, &buyer, 0, 1000),
				(0_u32, 1, vec![token_id.1])
			);

			// assert network fees accumulated
			let fee_pot_account: AccountId = FeePotId::get().into_account_truncating();

			assert_eq!(
				AssetsExt::balance(XRP_ASSET_ID, &fee_pot_account),
				5, // 0.5% of 1000
			);
		});
}

#[test]
fn buy_with_royalties() {
	let buyer = create_account(5);
	let sale_price = 1_000_008;

	TestExt::<Test>::default()
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
				setup_nft_token_with_royalties(royalties_schedule.clone(), 2);

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

			let initial_balance_owner = AssetsExt::balance(NativeAssetId::get(), &collection_owner);
			let initial_balance_b1 = AssetsExt::balance(NativeAssetId::get(), &beneficiary_1);
			let initial_balance_b2 = AssetsExt::balance(NativeAssetId::get(), &beneficiary_2);
			let initial_balance_seller = AssetsExt::balance(NativeAssetId::get(), &token_owner);

			assert_ok!(Marketplace::buy(Some(buyer).into(), listing_id));
			let presale_issuance = AssetsExt::total_issuance(NativeAssetId::get());
			// royalties distributed according to `entitlements` map
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &collection_owner),
				initial_balance_owner + royalties_schedule.clone().entitlements[0].1 * sale_price
			);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &beneficiary_1),
				initial_balance_b1 + royalties_schedule.clone().entitlements[1].1 * sale_price
			);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &beneficiary_2),
				initial_balance_b2 + royalties_schedule.clone().entitlements[2].1 * sale_price
			);
			// token owner gets sale price - royalties - network fee
			let network_fee = MarketplaceNetworkFeePercentage::get().mul(sale_price);
			let royalties = royalties_schedule
				.clone()
				.entitlements
				.into_iter()
				.map(|(_, e)| e * sale_price)
				.sum::<Balance>();
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &token_owner),
				initial_balance_seller + sale_price - royalties - network_fee
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
	TestExt::<Test>::default()
		.with_balances(&[(buyer, price - 1)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_nft_token();
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
	TestExt::<Test>::default()
		.with_balances(&[(buyer, price + 995)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_nft_token();

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
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &buyer), 995);

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
fn buy_with_overcommitted_royalties() {
	let buyer = create_account(5);
	let price = 1_000;
	TestExt::<Test>::default()
		.with_balances(&[(buyer, 1995)])
		.build()
		.execute_with(|| {
			// royalties are > 100% total which could create funds out of nothing
			// in this case, default to 0 royalties.
			// royalty schedules should not make it into storage but we protect against it anyway
			let (collection_id, token_id, token_owner) = setup_nft_token();
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
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &token_owner),
				price - MarketplaceNetworkFeePercentage::get().mul(price)
			);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &buyer), 995);
			assert_eq!(AssetsExt::total_issuance(NativeAssetId::get()), presale_issuance);
		})
}

#[test]
fn cancel_auction() {
	TestExt::<Test>::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_nft_token();

		let reserve_price = 100_000;
		let listing_id = Marketplace::next_listing_id();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_ok!(Marketplace::auction_nft(
			Some(token_owner).into(),
			collection_id,
			serial_numbers.clone(),
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

		let tokens = ListingTokens::Nft(NftListing { collection_id, serial_numbers });
		System::assert_last_event(MockEvent::Marketplace(Event::<Test>::AuctionClose {
			tokens,
			listing_id,
			marketplace_id: None,
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
	TestExt::<Test>::default()
		.with_balances(&[(buyer, price)])
		.build()
		.execute_with(|| {
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
					TokenLocks::<Test>::get((collection_id, serial_number)).unwrap(),
					TokenLockReason::Listed(listing_id)
				);
			}

			assert_ok!(Marketplace::bid(Some(buyer).into(), listing_id, price));
			// end auction
			let _ = Marketplace::on_initialize(
				System::block_number() + AUCTION_EXTENSION_PERIOD as u64,
			);

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
fn auction_bundle_no_bids() {
	let buyer = create_account(5);
	let price = 1_000;
	TestExt::<Test>::default()
		.with_balances(&[(buyer, price)])
		.build()
		.execute_with(|| {
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
				None, //Some(1),
				None,
			));

			assert!(Marketplace::open_collection_listings(collection_id, listing_id).unwrap());
			for serial_number in serial_numbers.iter() {
				assert_eq!(
					TokenLocks::<Test>::get((collection_id, serial_number)).unwrap(),
					TokenLockReason::Listed(listing_id)
				);
			}

			// end auction with no bids
			let end_block = System::block_number() + DefaultListingDuration::get();
			let _ = Marketplace::on_initialize(end_block);

			// Listing should be successfully removed
			assert!(!OpenCollectionListings::<Test>::contains_key(collection_id, listing_id));
			// Token locks should be removed
			for serial_number in serial_numbers.iter() {
				assert_eq!(TokenLocks::<Test>::get((collection_id, serial_number)), None);
			}
		})
}

#[test]
fn auction_bundle_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_owner = create_account(1);
		let collection_id = create_nft_collection(collection_owner);
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
			Error::<Test>::EmptyTokens
		);
	})
}

#[test]
fn auction() {
	let bidder_1 = create_account(5);
	let bidder_2 = create_account(6);
	let reserve_price = 100_000;
	let winning_bid = reserve_price + 1;

	TestExt::<Test>::default()
		.with_balances(&[(bidder_1, reserve_price), (bidder_2, winning_bid)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_nft_token();

			let listing_id = Marketplace::next_listing_id();
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Marketplace::auction_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers.clone(),
				NativeAssetId::get(),
				reserve_price,
				Some(1),
				None,
			));
			assert_eq!(
				TokenLocks::<Test>::get(&token_id).unwrap(),
				TokenLockReason::Listed(listing_id)
			);
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

			// no royalties, all proceeds to token owner minus network fees
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &token_owner),
				winning_bid - MarketplaceNetworkFeePercentage::get().mul(winning_bid)
			);
			// bidder2 funds should be all gone (unreserved and transferred)
			assert!(AssetsExt::balance(NativeAssetId::get(), &bidder_2).is_zero());
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
			assert!(TokenLocks::<Test>::get(&token_id).is_none());
			assert_eq!(
				Nft::owned_tokens(collection_id, &bidder_2, 0, 1000),
				(0_u32, 1, vec![token_id.1])
			);
			assert!(Marketplace::open_collection_listings(collection_id, listing_id).is_none());

			// event logged
			let tokens = ListingTokens::Nft(NftListing { collection_id, serial_numbers });
			System::assert_last_event(MockEvent::Marketplace(Event::<Test>::AuctionSold {
				tokens,
				listing_id,
				marketplace_id: None,
				payment_asset: NativeAssetId::get(),
				hammer_price: winning_bid,
				winner: bidder_2,
			}));
		});
}

#[test]
fn auction_with_xrp_asset() {
	let bidder_1 = create_account(5);
	let bidder_2 = create_account(6);
	let reserve_price = 100_000;
	let winning_bid = reserve_price + 1;

	TestExt::<Test>::default()
		.with_asset(XRP_ASSET_ID, "XRP", &[(bidder_1, reserve_price), (bidder_2, winning_bid)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_nft_token();

			let listing_id = Marketplace::next_listing_id();
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			assert_ok!(Marketplace::auction_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers.clone(),
				XRP_ASSET_ID,
				reserve_price,
				Some(1),
				None,
			));
			assert_eq!(
				TokenLocks::<Test>::get(&token_id).unwrap(),
				TokenLockReason::Listed(listing_id)
			);
			assert_eq!(Marketplace::next_listing_id(), listing_id + 1);
			assert!(Marketplace::open_collection_listings(collection_id, listing_id).unwrap());

			// first bidder at reserve price
			assert_ok!(Marketplace::bid(Some(bidder_1).into(), listing_id, reserve_price,));
			assert_eq!(
				AssetsExt::hold_balance(&MarketplacePalletId::get(), &bidder_1, &XRP_ASSET_ID),
				reserve_price
			);

			// second bidder raises bid
			assert_ok!(Marketplace::bid(Some(bidder_2).into(), listing_id, winning_bid,));
			assert_eq!(
				AssetsExt::hold_balance(&MarketplacePalletId::get(), &bidder_2, &XRP_ASSET_ID),
				winning_bid
			);
			assert!(AssetsExt::hold_balance(&MarketplacePalletId::get(), &bidder_1, &XRP_ASSET_ID)
				.is_zero());

			// end auction
			let _ = Marketplace::on_initialize(
				System::block_number() + AUCTION_EXTENSION_PERIOD as u64,
			);

			// no royalties, all proceeds to token owner minus network fees
			assert_eq!(
				AssetsExt::balance(XRP_ASSET_ID, &token_owner),
				winning_bid - MarketplaceNetworkFeePercentage::get().mul(winning_bid)
			);
			// bidder2 funds should be all gone (unreserved and transferred)
			assert!(AssetsExt::balance(XRP_ASSET_ID, &bidder_2).is_zero());
			assert!(AssetsExt::hold_balance(&MarketplacePalletId::get(), &bidder_2, &XRP_ASSET_ID)
				.is_zero());
			// listing metadata removed
			assert!(Listings::<Test>::get(listing_id).is_none());
			assert!(
				Marketplace::listing_end_schedule(System::block_number() + 1, listing_id).is_none()
			);

			// ownership changed
			assert!(TokenLocks::<Test>::get(&token_id).is_none());
			assert_eq!(
				Nft::owned_tokens(collection_id, &bidder_2, 0, 1000),
				(0_u32, 1, vec![token_id.1])
			);
			assert!(Marketplace::open_collection_listings(collection_id, listing_id).is_none());

			// event logged
			let tokens = ListingTokens::Nft(NftListing { collection_id, serial_numbers });
			System::assert_last_event(MockEvent::Marketplace(Event::<Test>::AuctionSold {
				tokens,
				listing_id,
				marketplace_id: None,
				payment_asset: XRP_ASSET_ID,
				hammer_price: winning_bid,
				winner: bidder_2,
			}));
		});
}

#[test]
fn bid_auto_extends() {
	let bidder_1 = create_account(5);
	let reserve_price = 100_000;

	TestExt::<Test>::default()
		.with_balances(&[(bidder_1, reserve_price)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_nft_token();
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

	TestExt::<Test>::default()
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
				setup_nft_token_with_royalties(royalties_schedule.clone(), 1);
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
				AssetsExt::balance(NativeAssetId::get(), &collection_owner),
				royalties_schedule.entitlements[0].1 * reserve_price
			);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &beneficiary_1),
				royalties_schedule.entitlements[1].1 * reserve_price
			);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &beneficiary_2),
				royalties_schedule.entitlements[2].1 * reserve_price
			);
			// token owner gets sale price - (royalties + network fee)
			let royalties = royalties_schedule
				.entitlements
				.into_iter()
				.map(|(_, e)| e * reserve_price)
				.sum::<Balance>();
			let network_fee = MarketplaceNetworkFeePercentage::get().mul(reserve_price);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &token_owner),
				reserve_price - (royalties + network_fee)
			);
			assert!(AssetsExt::balance(NativeAssetId::get(), &bidder).is_zero());
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
	TestExt::<Test>::default().build().execute_with(|| {
		let collection_id = Nft::next_collection_uuid().unwrap();
		let price = 123_456;
		let token_1 = (collection_id, 0);
		let seller = create_account(1);
		let serial_numbers = BoundedVec::truncate_from(vec![token_1.1]);
		let tokens = ListingTokens::Nft(NftListing { collection_id, serial_numbers });
		let listings = vec![
			// an open sale which won't be bought before closing
			Listing::<Test>::FixedPrice(FixedPriceListing::<Test> {
				payment_asset: NativeAssetId::get(),
				fixed_price: price,
				buyer: None,
				close: System::block_number() + 1,
				seller: seller.clone(),
				tokens: tokens.clone(),
				royalties_schedule: Default::default(),
				marketplace_id: None,
			}),
			// an open auction which has no bids before closing
			Listing::<Test>::Auction(AuctionListing::<Test> {
				payment_asset: NativeAssetId::get(),
				reserve_price: price,
				close: System::block_number() + 1,
				seller: seller.clone(),
				tokens: tokens.clone(),
				royalties_schedule: Default::default(),
				marketplace_id: None,
			}),
			// an open auction which has a winning bid before closing
			Listing::<Test>::Auction(AuctionListing::<Test> {
				payment_asset: NativeAssetId::get(),
				reserve_price: price,
				close: System::block_number() + 1,
				seller: seller.clone(),
				tokens: tokens.clone(),
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
	TestExt::<Test>::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_nft_token();

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
			pallet_nft::Error::<Test>::NotTokenOwner
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
			pallet_nft::Error::<Test>::NotTokenOwner
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
			pallet_nft::Error::<Test>::TokenLocked
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
			pallet_nft::Error::<Test>::TokenLocked
		);
	});
}

#[test]
fn auction_zero_duration_fails() {
	TestExt::<Test>::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_nft_token();
		let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
			BoundedVec::try_from(vec![token_id.1]).unwrap();
		assert_noop!(
			Marketplace::auction_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers.clone(),
				NativeAssetId::get(),
				100_000,
				Some(0), // Invalid duration
				None,
			),
			Error::<Test>::DurationTooShort
		);
	});
}

#[test]
fn bid_fails_prechecks() {
	let bidder = create_account(5);
	let reserve_price = 100_004;

	TestExt::<Test>::default()
		.with_balances(&[(bidder, reserve_price)])
		.build()
		.execute_with(|| {
			let missing_listing_id = 5;
			assert_noop!(
				Marketplace::bid(Some(create_account(1)).into(), missing_listing_id, 100),
				Error::<Test>::NotForAuction
			);

			let (collection_id, token_id, token_owner) = setup_nft_token();
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
				TokenError::FundsUnavailable
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

	TestExt::<Test>::default().build().execute_with(|| {
		let missing_listing_id = 5;
		assert_noop!(
			Marketplace::bid(Some(create_account(1)).into(), missing_listing_id, 100),
			Error::<Test>::NotForAuction
		);

		let (collection_id, token_id, token_owner) = setup_nft_token();
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
			ArithmeticError::Underflow
		);
	});
}

#[test]
fn make_simple_offer() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::<Test>::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, _) = setup_nft_token();
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
fn make_simple_offer_on_burnt_token_should_fail() {
	let buyer = create_account(7);

	TestExt::<Test>::default().build().execute_with(|| {
		let (collection_id, token_id, token_owner) = setup_nft_token();
		assert_eq!(
			Nft::owned_tokens(collection_id, &token_owner, 0, 1000),
			(token_id.1, 1, vec![token_id.1])
		);
		assert_ok!(Nft::burn(Some(token_owner).into(), token_id));
		let offer_amount: Balance = 100;
		assert_noop!(
			Marketplace::make_simple_offer(
				Some(buyer).into(),
				token_id,
				offer_amount,
				NativeAssetId::get(),
				None
			),
			Error::<Test>::NoToken
		);
	});
}

#[test]
fn make_simple_offer_on_non_existent_token_should_fail() {
	let buyer = create_account(7);

	TestExt::<Test>::default().build().execute_with(|| {
		let (collection_id, _, _) = setup_nft_token();
		let offer_amount: Balance = 100;
		assert_noop!(
			Marketplace::make_simple_offer(
				Some(buyer).into(),
				(collection_id, 456), // non existent token
				offer_amount,
				NativeAssetId::get(),
				None
			),
			Error::<Test>::NoToken
		);
	});
}

#[test]
fn make_simple_offer_insufficient_funds_should_fail() {
	TestExt::<Test>::default().build().execute_with(|| {
		let (_, token_id, _) = setup_nft_token();
		let buyer = create_account(3);
		let offer_amount: Balance = 100;
		assert_eq!(AssetsExt::balance(NativeAssetId::get(), &buyer), 0);

		assert_noop!(
			Marketplace::make_simple_offer(
				Some(buyer).into(),
				token_id,
				offer_amount,
				NativeAssetId::get(),
				None
			),
			ArithmeticError::Underflow
		);
	});
}

#[test]
fn make_simple_offer_zero_amount_should_fail() {
	TestExt::<Test>::default().build().execute_with(|| {
		let (_, token_id, _) = setup_nft_token();
		let buyer = create_account(3);
		let offer_amount: Balance = 0;
		assert_eq!(AssetsExt::balance(NativeAssetId::get(), &buyer), 0);

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
	TestExt::<Test>::default().build().execute_with(|| {
		let (_, token_id, token_owner) = setup_nft_token();
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

	TestExt::<Test>::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_nft_token();
			let offer_amount: Balance = 100;
			let sell_price = 100_000;
			let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
				BoundedVec::try_from(vec![token_id.1]).unwrap();
			let listing_id = Marketplace::next_listing_id();

			assert_ok!(Marketplace::sell_nft(
				Some(token_owner).into(),
				collection_id,
				serial_numbers.clone(),
				None,
				NativeAssetId::get(),
				sell_price,
				None,
				None,
			));
			// Sanity check
			assert!(Listings::<Test>::get(listing_id).is_some());
			assert!(TokenLocks::<Test>::get(token_id).is_some());

			let (offer_id, _) = make_new_simple_offer(offer_amount, token_id, buyer, None);
			// Check funds have been locked
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &buyer),
				initial_balance_buyer - offer_amount
			);
			assert_eq!(
				AssetsExt::hold_balance(&MarketplacePalletId::get(), &buyer, &NativeAssetId::get()),
				offer_amount
			);

			assert_ok!(Marketplace::accept_offer(Some(token_owner).into(), offer_id,));

			// Check that fixed price listing and locks are now removed
			assert!(Listings::<Test>::get(listing_id).is_none());
			assert!(TokenLocks::<Test>::get(token_id).is_none());
			// Check offer storage has been removed
			assert!(Marketplace::token_offers(token_id).is_none());
			assert!(Marketplace::offers(offer_id).is_none());

			// Check funds have been transferred
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &buyer),
				initial_balance_buyer - offer_amount
			);
			assert!(AssetsExt::hold_balance(
				&MarketplacePalletId::get(),
				&buyer,
				&NativeAssetId::get()
			)
			.is_zero());
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &token_owner), offer_amount);

			System::assert_has_event(MockEvent::Marketplace(Event::<Test>::FixedPriceSaleClose {
				tokens: ListingTokens::Nft(NftListing { collection_id, serial_numbers }),
				listing_id,
				marketplace_id: None,
				reason: FixedPriceClosureReason::OfferAccepted,
			}));
		});
}

#[test]
fn make_simple_offer_on_auction_should_fail() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::<Test>::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_nft_token();
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

	TestExt::<Test>::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, _) = setup_nft_token();
			let offer_amount: Balance = 100;

			let (offer_id, _) = make_new_simple_offer(offer_amount, token_id, buyer, None);
			assert_ok!(Marketplace::cancel_offer(Some(buyer).into(), offer_id));

			System::assert_last_event(MockEvent::Marketplace(Event::<Test>::OfferCancel {
				offer_id,
				marketplace_id: None,
				token_id,
			}));

			// Check storage has been removed
			assert!(Marketplace::token_offers(token_id).is_none());
			assert_eq!(Marketplace::offers(offer_id), None);
			// Check funds have been unlocked after offer cancelled
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &buyer), initial_balance_buyer);
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

	TestExt::<Test>::default()
		.with_balances(&[(buyer_1, initial_balance_buyer_1), (buyer_2, initial_balance_buyer_2)])
		.build()
		.execute_with(|| {
			let (_, token_id, _) = setup_nft_token();

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
				marketplace_id: None,
				token_id,
			}));

			// Check storage has been removed
			let offer_vector: Vec<OfferId> = vec![offer_id_2];
			assert_eq!(Marketplace::token_offers(token_id).unwrap(), offer_vector);
			assert_eq!(Marketplace::offers(offer_id_2), Some(OfferType::Simple(offer_2.clone())));
			assert_eq!(Marketplace::offers(offer_id_1), None);

			// Check funds have been unlocked after offer cancelled
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &buyer_1), initial_balance_buyer_1);
			assert!(AssetsExt::hold_balance(
				&MarketplacePalletId::get(),
				&buyer_1,
				&NativeAssetId::get()
			)
			.is_zero());
			// Check buyer_2 funds have not been unlocked
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &buyer_2),
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

	TestExt::<Test>::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, _) = setup_nft_token();
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

	TestExt::<Test>::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, token_owner) = setup_nft_token();
			let offer_amount: Balance = 100;
			let (offer_id, _) = make_new_simple_offer(offer_amount, token_id, buyer, None);
			assert_ok!(Marketplace::accept_offer(Some(token_owner).into(), offer_id));
			System::assert_last_event(MockEvent::Marketplace(Event::<Test>::OfferAccept {
				offer_id,
				marketplace_id: None,
				token_id,
				amount: offer_amount,
				asset_id: NativeAssetId::get(),
			}));

			// Check storage has been removed
			assert!(Marketplace::token_offers(token_id).is_none());
			assert!(Marketplace::offers(offer_id).is_none());
			// Check funds have been transferred
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &buyer),
				initial_balance_buyer - offer_amount
			);
			assert!(AssetsExt::hold_balance(
				&MarketplacePalletId::get(),
				&buyer,
				&NativeAssetId::get()
			)
			.is_zero());
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &token_owner), offer_amount);
		});
}

#[test]
fn accept_offer_multiple_offers() {
	let buyer_1 = create_account(3);
	let buyer_2 = create_account(4);
	let initial_balance_buyer_1: Balance = 1000;
	let initial_balance_buyer_2: Balance = 1000;

	TestExt::<Test>::default()
		.with_balances(&[(buyer_1, initial_balance_buyer_1), (buyer_2, initial_balance_buyer_2)])
		.build()
		.execute_with(|| {
			let (_, token_id, token_owner) = setup_nft_token();

			let offer_amount_1: Balance = 100;
			let offer_amount_2: Balance = 150;

			let (offer_id_1, offer_1) =
				make_new_simple_offer(offer_amount_1, token_id, buyer_1, None);
			let (offer_id_2, _) = make_new_simple_offer(offer_amount_2, token_id, buyer_2, None);

			// Accept second offer
			assert_ok!(Marketplace::accept_offer(Some(token_owner).into(), offer_id_2));
			System::assert_last_event(MockEvent::Marketplace(Event::<Test>::OfferAccept {
				offer_id: offer_id_2,
				marketplace_id: None,
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
				AssetsExt::balance(NativeAssetId::get(), &buyer_2),
				initial_balance_buyer_2 - offer_amount_2
			);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &buyer_1),
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
				AssetsExt::balance(NativeAssetId::get(), &token_owner),
				offer_amount_2 - MarketplaceNetworkFeePercentage::get().mul(offer_amount_2)
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

	TestExt::<Test>::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, token_owner) = setup_nft_token();
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
				AssetsExt::balance(NativeAssetId::get(), &buyer),
				initial_balance_buyer - offer_amount
			);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &marketplace_account),
				entitlements * offer_amount
			);
			assert!(AssetsExt::hold_balance(
				&MarketplacePalletId::get(),
				&buyer,
				&NativeAssetId::get()
			)
			.is_zero());
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &token_owner),
				offer_amount - (entitlements * offer_amount)
			);
		});
}

#[test]
fn accept_offer_not_token_owner_should_fail() {
	let buyer = create_account(5);
	let initial_balance_buyer = 1000;

	TestExt::<Test>::default()
		.with_balances(&[(buyer, initial_balance_buyer)])
		.build()
		.execute_with(|| {
			let (_, token_id, _) = setup_nft_token();
			let offer_amount: Balance = 100;

			let (offer_id, _) = make_new_simple_offer(offer_amount, token_id, buyer, None);
			assert_noop!(
				Marketplace::accept_offer(Some(create_account(4)).into(), offer_id),
				Error::<Test>::NotTokenOwner
			);
		});
}

mod set_fee_to {
	use super::*;

	#[test]
	fn set_fee_to_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Ensure default is correct
			let default_fee_to: AccountId =
				mock::DefaultFeeTo::get().unwrap().into_account_truncating();
			assert_eq!(FeeTo::<Test>::get().unwrap(), default_fee_to);

			// Change fee_to account
			let new_fee_to = create_account(10);
			assert_ok!(Marketplace::set_fee_to(RawOrigin::Root.into(), Some(new_fee_to.clone())));

			// Event thrown
			System::assert_last_event(MockEvent::Marketplace(Event::<Test>::FeeToSet {
				account: Some(new_fee_to),
			}));
			// Storage updated
			assert_eq!(FeeTo::<Test>::get().unwrap(), new_fee_to);
		});
	}

	#[test]
	fn set_fee_to_not_root_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Change fee_to account from not sudo should fail
			let new_fee_to = create_account(10);
			assert_noop!(
				Marketplace::set_fee_to(Some(create_account(11)).into(), Some(new_fee_to)),
				BadOrigin
			);
		});
	}
}

mod sell_sft {
	use super::*;

	#[test]
	fn sell_sft_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let balance = 100;
			let (collection_id, token_id, token_owner) = setup_sft_token(balance);
			let price = 100_000;
			let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
				BoundedVec::truncate_from(vec![(token_id.1, balance)]);
			let listing_id = Marketplace::next_listing_id();
			let sft_token = ListingTokens::Sft(SftListing {
				collection_id,
				serial_numbers: serial_numbers.clone(),
			});
			// Remove fee to
			assert_ok!(Marketplace::set_fee_to(RawOrigin::Root.into(), None));

			assert_ok!(Marketplace::sell(
				Some(token_owner).into(),
				sft_token.clone(),
				None,
				NativeAssetId::get(),
				price,
				None,
				None,
			));

			// Event thrown
			System::assert_last_event(MockEvent::Marketplace(Event::<Test>::FixedPriceSaleList {
				tokens: sft_token.clone(),
				listing_id,
				marketplace_id: None,
				price,
				payment_asset: NativeAssetId::get(),
				seller: token_owner,
				close: System::block_number() + DefaultListingDuration::get(),
			}));

			// Storage updated
			assert_eq!(
				Listings::<Test>::get(listing_id).unwrap(),
				Listing::FixedPrice(FixedPriceListing {
					payment_asset: NativeAssetId::get(),
					fixed_price: price,
					buyer: None,
					close: System::block_number() + DefaultListingDuration::get(),
					seller: token_owner,
					tokens: sft_token,
					royalties_schedule: Default::default(),
					marketplace_id: None,
				})
			);
			assert_eq!(
				ListingEndSchedule::<Test>::get(
					System::block_number() + DefaultListingDuration::get(),
					listing_id
				)
				.unwrap(),
				true
			);
			// Check the SFT reserved and free balance
			let token_balance = sft_balance_of::<Test>(token_id, &token_owner);
			assert_eq!(token_balance.free_balance, 0);
			assert_eq!(token_balance.reserved_balance, balance);
		});
	}

	#[test]
	fn sell_sft_with_nft_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_nft_token();
			let reserve_price = 100_000;
			let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
				BoundedVec::truncate_from(vec![(token_id.1, 1)]);
			let sft_token = ListingTokens::Sft(SftListing {
				collection_id,
				serial_numbers: serial_numbers.clone(),
			});

			assert_noop!(
				Marketplace::sell(
					Some(token_owner).into(),
					sft_token,
					None,
					NativeAssetId::get(),
					reserve_price,
					None,
					None,
				),
				pallet_sft::Error::<Test>::NoCollectionFound
			);
		});
	}

	#[test]
	fn sell_sft_with_empty_tokens_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let initial_balance = 1000;
			let (collection_id, _, token_owner) = setup_sft_token(initial_balance);

			// Empty tokens
			let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
				BoundedVec::truncate_from(vec![]);
			let sft_token = ListingTokens::Sft(SftListing {
				collection_id,
				serial_numbers: serial_numbers.clone(),
			});

			assert_noop!(
				Marketplace::sell(
					Some(token_owner).into(),
					sft_token,
					None,
					NativeAssetId::get(),
					1_000,
					None,
					None,
				),
				Error::<Test>::EmptyTokens
			);
		})
	}

	#[test]
	fn sell_sft_with_zero_balance_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let initial_balance = 1000;
			let (collection_id, token_id, token_owner) = setup_sft_token(initial_balance);

			// Zero balance in tokens
			let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
				BoundedVec::try_from(vec![(token_id.1, 0)]).unwrap();
			let sft_token = ListingTokens::Sft(SftListing {
				collection_id,
				serial_numbers: serial_numbers.clone(),
			});

			assert_noop!(
				Marketplace::sell(
					Some(token_owner).into(),
					sft_token,
					None,
					NativeAssetId::get(),
					1_000,
					None,
					None,
				),
				Error::<Test>::ZeroBalance
			);
		})
	}

	#[test]
	fn sell_sft_with_insufficient_balance_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let initial_balance = 1000;
			let (collection_id, token_id, token_owner) = setup_sft_token(initial_balance);

			// More tokens than balance
			let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
				BoundedVec::try_from(vec![(token_id.1, initial_balance + 1)]).unwrap();
			let sft_token = ListingTokens::Sft(SftListing {
				collection_id,
				serial_numbers: serial_numbers.clone(),
			});

			assert_noop!(
				Marketplace::sell(
					Some(token_owner).into(),
					sft_token,
					None,
					NativeAssetId::get(),
					1_000,
					None,
					None,
				),
				pallet_sft::Error::<Test>::InsufficientBalance
			);
		})
	}

	#[test]
	fn sell_sft_invalid_royalties_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let balance = 100;
			// Create royalties with 0.99, which will fail when adding network fee
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(
					create_account(5),
					Permill::from_float(0.9951),
				)]),
			};
			let (collection_id, token_id, token_owner) =
				setup_sft_token_with_royalties(balance, royalties_schedule);
			let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
				BoundedVec::truncate_from(vec![(token_id.1, balance)]);
			let sft_token = ListingTokens::Sft(SftListing {
				collection_id,
				serial_numbers: serial_numbers.clone(),
			});

			assert_noop!(
				Marketplace::sell(
					Some(token_owner).into(),
					sft_token.clone(),
					None,
					NativeAssetId::get(),
					100,
					None,
					None,
				),
				Error::<Test>::RoyaltiesInvalid
			);
		});
	}

	#[test]
	fn sell_sft_duplicate_serial_numbers() {
		TestExt::<Test>::default().build().execute_with(|| {
			let balance = 100;
			let (collection_id, token_id, token_owner) = setup_sft_token(balance);
			let price = 100_000;
			// Serial numbers are duplicate with total of 90
			let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
				BoundedVec::truncate_from(vec![
					(token_id.1, 50),
					(token_id.1, 30),
					(token_id.1, 10),
				]);
			let sft_token = ListingTokens::Sft(SftListing {
				collection_id,
				serial_numbers: serial_numbers.clone(),
			});

			let listing_id = Marketplace::next_listing_id();
			assert_ok!(Marketplace::sell(
				Some(token_owner).into(),
				sft_token.clone(),
				None,
				NativeAssetId::get(),
				price,
				None,
				None,
			));

			// Event thrown
			System::assert_last_event(MockEvent::Marketplace(Event::<Test>::FixedPriceSaleList {
				tokens: sft_token.clone(),
				listing_id,
				marketplace_id: None,
				price,
				payment_asset: NativeAssetId::get(),
				seller: token_owner,
				close: System::block_number() + DefaultListingDuration::get(),
			}));

			// Check the SFT reserved and free balance
			let token_balance = sft_balance_of::<Test>(token_id, &token_owner);
			assert_eq!(token_balance.free_balance, 10);
			assert_eq!(token_balance.reserved_balance, 90); // 50 + 30 + 10
		});
	}

	#[test]
	fn sell_sft_duplicate_serial_numbers_above_free_balance_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let balance = 100;
			let (collection_id, token_id, token_owner) = setup_sft_token(balance);
			let price = 100_000;
			// Serial numbers are duplicate with total of 101 (Above initial_issuance)
			let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
				BoundedVec::truncate_from(vec![
					(token_id.1, 50),
					(token_id.1, 30),
					(token_id.1, 21),
				]);
			let sft_token = ListingTokens::Sft(SftListing {
				collection_id,
				serial_numbers: serial_numbers.clone(),
			});

			assert_noop!(
				Marketplace::sell(
					Some(token_owner).into(),
					sft_token.clone(),
					None,
					NativeAssetId::get(),
					price,
					None,
					None,
				),
				pallet_sft::Error::<Test>::InsufficientBalance
			);
		});
	}
}

mod buy_sft {
	use super::*;

	#[test]
	fn buy_sft() {
		let buyer = create_account(5);
		let price = 1_000;
		let starting_balance = price * 2;

		TestExt::<Test>::default()
			.with_balances(&[(buyer, starting_balance)])
			.build()
			.execute_with(|| {
				let initial_issuance = 100;
				let (collection_id, token_id, token_owner) = setup_sft_token(initial_issuance);
				let buyer = create_account(5);

				let listing_id = Marketplace::next_listing_id();
				let sell_quantity = 60;
				let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
					BoundedVec::truncate_from(vec![(token_id.1, sell_quantity)]);
				let sft_token = ListingTokens::Sft(SftListing {
					collection_id,
					serial_numbers: serial_numbers.clone(),
				});
				assert_ok!(Marketplace::sell(
					Some(token_owner).into(),
					sft_token.clone(),
					None,
					NativeAssetId::get(),
					price,
					None,
					None,
				));

				assert_ok!(Marketplace::buy(Some(buyer).into(), listing_id));
				// no royalties, all proceeds to token owner minus network fee
				assert_eq!(
					AssetsExt::balance(NativeAssetId::get(), &token_owner),
					price - MarketplaceNetworkFeePercentage::get().mul(price)
				);
				// Buyer balance should be starting minus price (1000)
				assert_eq!(
					AssetsExt::balance(NativeAssetId::get(), &buyer),
					starting_balance - price
				);

				// listing removed
				assert!(Listings::<Test>::get(listing_id).is_none());
				assert!(Marketplace::listing_end_schedule(
					System::block_number() + <Test as Config>::DefaultListingDuration::get(),
					listing_id
				)
				.is_none());
				assert!(Marketplace::open_collection_listings(collection_id, listing_id).is_none());

				// Check SFT balances of both seller and buyer
				let seller_balance = sft_balance_of::<Test>(token_id, &token_owner);
				assert_eq!(seller_balance.free_balance, initial_issuance - sell_quantity);
				assert_eq!(seller_balance.reserved_balance, 0);

				let buyer_balance = sft_balance_of::<Test>(token_id, &buyer);
				assert_eq!(buyer_balance.free_balance, sell_quantity);
				assert_eq!(buyer_balance.reserved_balance, 0);

				// assert network fees accumulated
				let fee_pot_account: AccountId = FeePotId::get().into_account_truncating();
				assert_eq!(
					AssetsExt::balance(NativeAssetId::get(), &fee_pot_account),
					5, // 0.5% of 1000
				);

				System::assert_has_event(MockEvent::Marketplace(
					Event::<Test>::FixedPriceSaleComplete {
						tokens: sft_token.clone(),
						listing_id,
						marketplace_id: None,
						price,
						payment_asset: NativeAssetId::get(),
						seller: token_owner,
						buyer,
					},
				));

				System::assert_has_event(
					pallet_sft::Event::<Test>::Transfer {
						previous_owner: token_owner,
						collection_id,
						serial_numbers: BoundedVec::truncate_from(vec![token_id.1]),
						balances: BoundedVec::truncate_from(vec![sell_quantity]),
						new_owner: buyer,
					}
					.into(),
				);
			});
	}

	#[test]
	fn buy_sft_with_xrp() {
		let buyer = create_account(5);
		let price = 1_000;

		TestExt::<Test>::default()
			.with_asset(XRP_ASSET_ID, "XRP", &[(buyer, price)])
			.build()
			.execute_with(|| {
				let initial_issuance = 100;
				let (collection_id, token_id, token_owner) = setup_sft_token(initial_issuance);
				let buyer = create_account(5);

				let listing_id = Marketplace::next_listing_id();
				let sell_quantity = 60;
				let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
					BoundedVec::truncate_from(vec![(token_id.1, sell_quantity)]);
				let sft_token = ListingTokens::Sft(SftListing {
					collection_id,
					serial_numbers: serial_numbers.clone(),
				});
				assert_ok!(Marketplace::sell(
					Some(token_owner).into(),
					sft_token.clone(),
					None,
					XRP_ASSET_ID,
					price,
					None,
					None,
				));

				assert_ok!(Marketplace::buy(Some(buyer).into(), listing_id));
				// no royalties, all proceeds to token owner minus network fee
				assert_eq!(
					AssetsExt::balance(XRP_ASSET_ID, &token_owner),
					price - MarketplaceNetworkFeePercentage::get().mul(price)
				);
				// Buyer balance should be zero
				assert_eq!(AssetsExt::balance(XRP_ASSET_ID, &buyer), 0);

				// listing removed
				assert!(Listings::<Test>::get(listing_id).is_none());
				assert!(Marketplace::listing_end_schedule(
					System::block_number() + <Test as Config>::DefaultListingDuration::get(),
					listing_id
				)
				.is_none());
				assert!(Marketplace::open_collection_listings(collection_id, listing_id).is_none());

				// Check SFT balances of both seller and buyer
				let seller_balance = sft_balance_of::<Test>(token_id, &token_owner);
				assert_eq!(seller_balance.free_balance, initial_issuance - sell_quantity);
				assert_eq!(seller_balance.reserved_balance, 0);

				let buyer_balance = sft_balance_of::<Test>(token_id, &buyer);
				assert_eq!(buyer_balance.free_balance, sell_quantity);
				assert_eq!(buyer_balance.reserved_balance, 0);

				// assert network fees accumulated
				let fee_pot_account: AccountId = FeePotId::get().into_account_truncating();
				assert_eq!(
					AssetsExt::balance(XRP_ASSET_ID, &fee_pot_account),
					5, // 0.5% of 1000
				);

				System::assert_has_event(MockEvent::Marketplace(
					Event::<Test>::FixedPriceSaleComplete {
						tokens: sft_token.clone(),
						listing_id,
						marketplace_id: None,
						price,
						payment_asset: XRP_ASSET_ID,
						seller: token_owner,
						buyer,
					},
				));

				System::assert_has_event(
					pallet_sft::Event::<Test>::Transfer {
						previous_owner: token_owner,
						collection_id,
						serial_numbers: BoundedVec::truncate_from(vec![token_id.1]),
						balances: BoundedVec::truncate_from(vec![sell_quantity]),
						new_owner: buyer,
					}
					.into(),
				);
			});
	}

	#[test]
	fn buy_sft_with_royalties() {
		let buyer = create_account(5);
		let sale_price = 1_000_008;

		TestExt::<Test>::default()
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
				let initial_issuance = 1000;
				let (collection_id, token_id, token_owner) =
					setup_sft_token_with_royalties(initial_issuance, royalties_schedule.clone());

				let listing_id = Marketplace::next_listing_id();
				let sell_quantity = 100;
				let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
					BoundedVec::truncate_from(vec![(token_id.1, sell_quantity)]);
				let sft_token = ListingTokens::Sft(SftListing {
					collection_id,
					serial_numbers: serial_numbers.clone(),
				});

				// Setup marketplace
				let marketplace_account = create_account(4);
				let marketplace_entitlements: Permill = Permill::from_float(0.1);
				let marketplace_id = Marketplace::next_marketplace_id();
				assert_ok!(Marketplace::register_marketplace(
					Some(marketplace_account).into(),
					None,
					marketplace_entitlements
				));

				// Sell
				assert_ok!(Marketplace::sell(
					Some(token_owner).into(),
					sft_token.clone(),
					None,
					NativeAssetId::get(),
					sale_price,
					None,
					Some(marketplace_id),
				));

				let initial_balance_owner =
					AssetsExt::balance(NativeAssetId::get(), &collection_owner);
				let initial_balance_b1 = AssetsExt::balance(NativeAssetId::get(), &beneficiary_1);
				let initial_balance_b2 = AssetsExt::balance(NativeAssetId::get(), &beneficiary_2);
				let initial_balance_seller = AssetsExt::balance(NativeAssetId::get(), &token_owner);
				let initial_balance_marketplace =
					AssetsExt::balance(NativeAssetId::get(), &marketplace_account);

				assert_ok!(Marketplace::buy(Some(buyer).into(), listing_id));
				let presale_issuance = AssetsExt::total_issuance(NativeAssetId::get());

				// royalties distributed according to `entitlements` map
				assert_eq!(
					AssetsExt::balance(NativeAssetId::get(), &collection_owner),
					initial_balance_owner
						+ royalties_schedule.clone().entitlements[0].1 * sale_price
				);
				assert_eq!(
					AssetsExt::balance(NativeAssetId::get(), &beneficiary_1),
					initial_balance_b1 + royalties_schedule.clone().entitlements[1].1 * sale_price
				);
				assert_eq!(
					AssetsExt::balance(NativeAssetId::get(), &beneficiary_2),
					initial_balance_b2 + royalties_schedule.clone().entitlements[2].1 * sale_price
				);
				let marketplace_royalties = marketplace_entitlements.mul(sale_price);
				assert_eq!(
					AssetsExt::balance(NativeAssetId::get(), &marketplace_account),
					initial_balance_marketplace + marketplace_royalties
				);

				// token owner gets sale price - royalties - network fee - marketplace
				let network_fee = MarketplaceNetworkFeePercentage::get().mul(sale_price);
				let royalties = royalties_schedule
					.clone()
					.entitlements
					.into_iter()
					.map(|(_, e)| e * sale_price)
					.sum::<Balance>();
				assert_eq!(
					AssetsExt::balance(NativeAssetId::get(), &token_owner),
					initial_balance_seller + sale_price
						- royalties - network_fee
						- marketplace_royalties
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
				let seller_balance = sft_balance_of::<Test>(token_id, &token_owner);
				assert_eq!(seller_balance.free_balance, initial_issuance - sell_quantity);
				assert_eq!(seller_balance.reserved_balance, 0);

				let buyer_balance = sft_balance_of::<Test>(token_id, &buyer);
				assert_eq!(buyer_balance.free_balance, sell_quantity);
				assert_eq!(buyer_balance.reserved_balance, 0);
			});
	}

	#[test]
	fn buy_sft_fails_prechecks() {
		let buyer = create_account(5);
		let price = 1_000;
		TestExt::<Test>::default()
			.with_balances(&[(buyer, price - 1)])
			.build()
			.execute_with(|| {
				let initial_issuance = 1000;
				let (collection_id, token_id, token_owner) = setup_sft_token(initial_issuance);
				let buyer = create_account(5);
				let price = 1_000;
				let listing_id = Marketplace::next_listing_id();
				let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
					BoundedVec::truncate_from(vec![(token_id.1, 100)]);
				let sft_token = ListingTokens::Sft(SftListing {
					collection_id,
					serial_numbers: serial_numbers.clone(),
				});

				assert_ok!(Marketplace::sell(
					Some(token_owner).into(),
					sft_token.clone(),
					Some(buyer),
					NativeAssetId::get(),
					price,
					None,
					None,
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
}

mod auction_sft {
	use super::*;

	#[test]
	fn auction_sft_works() {
		TestExt::<Test>::default().build().execute_with(|| {
			let balance = 100;
			let (collection_id, token_id, token_owner) = setup_sft_token(balance);
			let reserve_price = 100_000;
			let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
				BoundedVec::truncate_from(vec![(token_id.1, balance)]);
			let listing_id = Marketplace::next_listing_id();
			let sft_token = ListingTokens::Sft(SftListing {
				collection_id,
				serial_numbers: serial_numbers.clone(),
			});
			// Remove fee to
			assert_ok!(Marketplace::set_fee_to(RawOrigin::Root.into(), None));

			assert_ok!(Marketplace::auction(
				Some(token_owner).into(),
				sft_token.clone(),
				NativeAssetId::get(),
				reserve_price,
				None,
				None,
			));

			// Event thrown
			System::assert_last_event(MockEvent::Marketplace(Event::<Test>::AuctionOpen {
				tokens: sft_token.clone(),
				listing_id,
				marketplace_id: None,
				payment_asset: NativeAssetId::get(),
				reserve_price,
				seller: token_owner,
				close: System::block_number() + DefaultListingDuration::get(),
			}));

			// Storage updated
			assert_eq!(
				Listings::<Test>::get(listing_id).unwrap(),
				Listing::Auction(AuctionListing {
					payment_asset: NativeAssetId::get(),
					reserve_price,
					close: System::block_number() + DefaultListingDuration::get(),
					seller: token_owner,
					tokens: sft_token,
					royalties_schedule: Default::default(),
					marketplace_id: None,
				})
			);
			assert_eq!(
				ListingEndSchedule::<Test>::get(
					System::block_number() + DefaultListingDuration::get(),
					listing_id
				)
				.unwrap(),
				true
			);
			// Check the SFT reserved and free balance
			let token_balance = sft_balance_of::<Test>(token_id, &token_owner);
			assert_eq!(token_balance.free_balance, 0);
			assert_eq!(token_balance.reserved_balance, balance);
		});
	}

	#[test]
	fn auction_sft_with_nft_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_nft_token();
			let reserve_price = 100_000;
			let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
				BoundedVec::truncate_from(vec![(token_id.1, 1)]);
			let sft_token = ListingTokens::Sft(SftListing {
				collection_id,
				serial_numbers: serial_numbers.clone(),
			});

			assert_noop!(
				Marketplace::auction(
					Some(token_owner).into(),
					sft_token.clone(),
					NativeAssetId::get(),
					reserve_price,
					None,
					None,
				),
				pallet_sft::Error::<Test>::NoCollectionFound
			);
		});
	}

	#[test]
	fn auction_sft_with_empty_tokens_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let initial_balance = 1000;
			let (collection_id, _, token_owner) = setup_sft_token(initial_balance);

			// Empty tokens
			let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
				BoundedVec::truncate_from(vec![]);
			let sft_token = ListingTokens::Sft(SftListing {
				collection_id,
				serial_numbers: serial_numbers.clone(),
			});

			assert_noop!(
				Marketplace::auction(
					Some(token_owner).into(),
					sft_token,
					NativeAssetId::get(),
					1_000,
					None,
					None,
				),
				Error::<Test>::EmptyTokens
			);
		})
	}

	#[test]
	fn auction_sft_with_zero_balance_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let initial_balance = 1000;
			let (collection_id, token_id, token_owner) = setup_sft_token(initial_balance);

			// Zero balance in tokens
			let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
				BoundedVec::try_from(vec![(token_id.1, 0)]).unwrap();
			let sft_token = ListingTokens::Sft(SftListing {
				collection_id,
				serial_numbers: serial_numbers.clone(),
			});

			assert_noop!(
				Marketplace::auction(
					Some(token_owner).into(),
					sft_token,
					NativeAssetId::get(),
					1_000,
					None,
					None,
				),
				Error::<Test>::ZeroBalance
			);
		})
	}

	#[test]
	fn auction_sft_with_insufficient_balance_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let initial_balance = 1000;
			let (collection_id, token_id, token_owner) = setup_sft_token(initial_balance);

			// More tokens than balance
			let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
				BoundedVec::try_from(vec![(token_id.1, initial_balance + 1)]).unwrap();
			let sft_token = ListingTokens::Sft(SftListing {
				collection_id,
				serial_numbers: serial_numbers.clone(),
			});

			assert_noop!(
				Marketplace::auction(
					Some(token_owner).into(),
					sft_token,
					NativeAssetId::get(),
					1_000,
					None,
					None,
				),
				pallet_sft::Error::<Test>::InsufficientBalance
			);
		})
	}

	#[test]
	fn auction_sft_invalid_royalties_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let balance = 100;
			// Create royalties with 0.99, which will fail when adding network fee
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(
					create_account(5),
					Permill::from_float(0.9951),
				)]),
			};
			let (collection_id, token_id, token_owner) =
				setup_sft_token_with_royalties(balance, royalties_schedule);
			let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
				BoundedVec::truncate_from(vec![(token_id.1, balance)]);
			let sft_token = ListingTokens::Sft(SftListing {
				collection_id,
				serial_numbers: serial_numbers.clone(),
			});

			assert_noop!(
				Marketplace::auction(
					Some(token_owner).into(),
					sft_token.clone(),
					NativeAssetId::get(),
					100,
					None,
					None,
				),
				Error::<Test>::RoyaltiesInvalid
			);
		});
	}

	#[test]
	fn auction_sft_duplicate_serial_numbers() {
		TestExt::<Test>::default().build().execute_with(|| {
			let balance = 100;
			let (collection_id, token_id, token_owner) = setup_sft_token(balance);
			let price = 100_000;
			// Serial numbers are duplicate with total of 90
			let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
				BoundedVec::truncate_from(vec![
					(token_id.1, 50),
					(token_id.1, 30),
					(token_id.1, 10),
				]);
			let sft_token = ListingTokens::Sft(SftListing {
				collection_id,
				serial_numbers: serial_numbers.clone(),
			});

			assert_ok!(Marketplace::auction(
				Some(token_owner).into(),
				sft_token.clone(),
				NativeAssetId::get(),
				price,
				None,
				None,
			));

			// Check the SFT reserved and free balance
			let token_balance = sft_balance_of::<Test>(token_id, &token_owner);
			assert_eq!(token_balance.free_balance, 10);
			assert_eq!(token_balance.reserved_balance, 90); // 50 + 30 + 10
		});
	}

	#[test]
	fn auction_sft_duplicate_serial_numbers_above_free_balance_fails() {
		TestExt::<Test>::default().build().execute_with(|| {
			let balance = 100;
			let (collection_id, token_id, token_owner) = setup_sft_token(balance);
			let price = 100_000;
			// Serial numbers are duplicate with total of 101 (Above initial_issuance)
			let serial_numbers: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
				BoundedVec::truncate_from(vec![
					(token_id.1, 50),
					(token_id.1, 30),
					(token_id.1, 21),
				]);
			let sft_token = ListingTokens::Sft(SftListing {
				collection_id,
				serial_numbers: serial_numbers.clone(),
			});

			assert_noop!(
				Marketplace::auction(
					Some(token_owner).into(),
					sft_token.clone(),
					NativeAssetId::get(),
					price,
					None,
					None,
				),
				pallet_sft::Error::<Test>::InsufficientBalance
			);
		});
	}
}

mod listing_tokens {
	use super::*;

	#[test]
	fn listing_tokens_validate_nft() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Valid
			let tokens = ListingTokens::<Test>::Nft(NftListing {
				collection_id: 0,
				serial_numbers: BoundedVec::truncate_from(vec![1, 2, 3]),
			});
			assert_ok!(tokens.validate());

			// Invalid
			let tokens = ListingTokens::<Test>::Nft(NftListing {
				collection_id: 0,
				serial_numbers: BoundedVec::truncate_from(vec![]),
			});
			assert_noop!(tokens.validate(), Error::<Test>::EmptyTokens);
		});
	}

	#[test]
	fn listing_tokens_validate_sft() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Valid
			let tokens = ListingTokens::<Test>::Sft(SftListing {
				collection_id: 0,
				serial_numbers: BoundedVec::truncate_from(vec![(1, 100), (2, 2000), (3, 2)]),
			});
			assert_ok!(tokens.validate());

			// Invalid due to empty tokens
			let tokens = ListingTokens::<Test>::Sft(SftListing {
				collection_id: 0,
				serial_numbers: BoundedVec::truncate_from(vec![]),
			});
			assert_noop!(tokens.validate(), Error::<Test>::EmptyTokens);

			// Invalid due to zero balance
			let tokens = ListingTokens::<Test>::Sft(SftListing {
				collection_id: 0,
				serial_numbers: BoundedVec::truncate_from(vec![(1, 100), (2, 0)]),
			});
			assert_noop!(tokens.validate(), Error::<Test>::ZeroBalance);
		});
	}

	#[test]
	fn listing_tokens_get_collection_id() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Nft
			let tokens = ListingTokens::<Test>::Nft(NftListing {
				collection_id: 123,
				serial_numbers: BoundedVec::truncate_from(vec![1, 2, 3]),
			});
			assert_eq!(tokens.get_collection_id(), 123);

			// Sft
			let tokens = ListingTokens::<Test>::Sft(SftListing {
				collection_id: 124,
				serial_numbers: BoundedVec::truncate_from(vec![(1, 100)]),
			});
			assert_eq!(tokens.get_collection_id(), 124);
		});
	}

	#[test]
	fn listing_tokens_lock_tokens_nft() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Lock tokens, token doesn't exist
			let tokens = ListingTokens::<Test>::Nft(NftListing {
				collection_id: 1,
				serial_numbers: BoundedVec::truncate_from(vec![1]),
			});
			assert_noop!(
				tokens.lock_tokens(&create_account(1), 1),
				pallet_nft::Error::<Test>::NotTokenOwner
			);

			// Lock tokens not token owner
			let (collection_id, token_id, token_owner) = setup_nft_token();
			let tokens = ListingTokens::<Test>::Nft(NftListing {
				collection_id,
				serial_numbers: BoundedVec::truncate_from(vec![token_id.1]),
			});
			assert_noop!(
				tokens.lock_tokens(&create_account(1), 1),
				pallet_nft::Error::<Test>::NotTokenOwner
			);

			// Lock tokens works
			assert_ok!(tokens.lock_tokens(&token_owner, 1));
			assert_eq!(TokenLocks::<Test>::get(token_id).unwrap(), TokenLockReason::Listed(1));

			// Lock tokens token already locked
			assert_noop!(
				tokens.lock_tokens(&token_owner, 1),
				pallet_nft::Error::<Test>::TokenLocked
			);
		});
	}

	#[test]
	fn listing_tokens_lock_tokens_sft() {
		TestExt::<Test>::default().build().execute_with(|| {
			// Lock tokens, token doesn't exist
			let tokens = ListingTokens::<Test>::Sft(SftListing {
				collection_id: 1,
				serial_numbers: BoundedVec::truncate_from(vec![(1, 100)]),
			});
			assert_noop!(
				tokens.lock_tokens(&create_account(1), 1),
				pallet_sft::Error::<Test>::NoToken
			);

			// Lock tokens not token owner
			let balance = 100;
			let (collection_id, token_id, token_owner) = setup_sft_token(balance);
			let tokens = ListingTokens::<Test>::Sft(SftListing {
				collection_id,
				serial_numbers: BoundedVec::truncate_from(vec![(token_id.1, 50)]),
			});
			assert_noop!(
				tokens.lock_tokens(&create_account(1), 1),
				pallet_sft::Error::<Test>::InsufficientBalance
			);

			// Lock tokens works
			assert_ok!(tokens.lock_tokens(&token_owner, 1));

			// Lock tokens not enough free balance
			let tokens = ListingTokens::<Test>::Sft(SftListing {
				collection_id,
				serial_numbers: BoundedVec::truncate_from(vec![(token_id.1, 51)]),
			});
			assert_noop!(
				tokens.lock_tokens(&token_owner, 1),
				pallet_sft::Error::<Test>::InsufficientBalance
			);
		});
	}

	#[test]
	fn listing_tokens_unlock_tokens_nft() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_nft_token();
			let tokens = ListingTokens::<Test>::Nft(NftListing {
				collection_id,
				serial_numbers: BoundedVec::truncate_from(vec![token_id.1]),
			});

			// Lock tokens
			assert_ok!(tokens.lock_tokens(&token_owner, 1));

			// Sanity check
			assert_eq!(TokenLocks::<Test>::get(token_id).unwrap(), TokenLockReason::Listed(1));

			// Unlock tokens works
			assert_ok!(tokens.unlock_tokens(&token_owner));

			assert!(!TokenLocks::<Test>::contains_key(token_id));
		});
	}

	#[test]
	fn listing_tokens_unlock_tokens_sft() {
		TestExt::<Test>::default().build().execute_with(|| {
			let balance = 100;
			let (collection_id, token_id, token_owner) = setup_sft_token(balance);
			let tokens = ListingTokens::<Test>::Sft(SftListing {
				collection_id,
				serial_numbers: BoundedVec::truncate_from(vec![(token_id.1, 60)]),
			});

			// Lock tokens
			assert_ok!(tokens.lock_tokens(&token_owner, 1));

			// Sanity check
			let sft_balance_owner = sft_balance_of::<Test>(token_id, &token_owner);
			assert_eq!(sft_balance_owner.free_balance, 40);
			assert_eq!(sft_balance_owner.reserved_balance, 60);

			// Unlock tokens not token owner
			assert_noop!(
				tokens.unlock_tokens(&create_account(1)),
				pallet_sft::Error::<Test>::InsufficientBalance
			);

			// Unlock tokens unlocks all tokens
			assert_ok!(tokens.unlock_tokens(&token_owner));

			let sft_balance_owner = sft_balance_of::<Test>(token_id, &token_owner);
			assert_eq!(sft_balance_owner.free_balance, 100);
			assert_eq!(sft_balance_owner.reserved_balance, 0);

			// Unlock tokens fails as there is no balance
			assert_noop!(
				tokens.unlock_tokens(&token_owner),
				pallet_sft::Error::<Test>::InsufficientBalance
			);
		});
	}

	#[test]
	fn listing_tokens_unlock_and_transfer_nft() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_nft_token();
			let tokens = ListingTokens::<Test>::Nft(NftListing {
				collection_id,
				serial_numbers: BoundedVec::truncate_from(vec![token_id.1]),
			});

			// Lock tokens
			assert_ok!(tokens.lock_tokens(&token_owner, 1));

			// Sanity check
			assert_eq!(TokenLocks::<Test>::get(token_id).unwrap(), TokenLockReason::Listed(1));

			// Unlock and transfer tokens works
			let recipient = create_account(1123);
			assert_ok!(tokens.unlock_and_transfer(&token_owner, &recipient));

			assert!(!TokenLocks::<Test>::contains_key(token_id));

			// Verify owner of token is recipient
			assert_eq!(
				Nft::owned_tokens(collection_id, &recipient, 0, 1000),
				(0_u32, 1, vec![token_id.1])
			);
		});
	}

	#[test]
	fn listing_tokens_unlock_and_transfer_nft_multiple() {
		TestExt::<Test>::default().build().execute_with(|| {
			let (collection_id, token_id, token_owner) = setup_nft_token();

			// Mint max amount
			assert_ok!(Nft::mint(
				Some(create_account(1)).into(),
				collection_id,
				MaxTokensPerListing::get(),
				Some(token_owner)
			));

			let serial_numbers: Vec<SerialNumber> = (0..MaxTokensPerListing::get()).collect();
			let tokens = ListingTokens::<Test>::Nft(NftListing {
				collection_id,
				serial_numbers: BoundedVec::truncate_from(serial_numbers.clone()),
			});

			// Lock tokens
			assert_ok!(tokens.lock_tokens(&token_owner, 1));

			// Sanity check
			for serial_number in serial_numbers.clone() {
				assert_eq!(
					TokenLocks::<Test>::get((collection_id, serial_number)).unwrap(),
					TokenLockReason::Listed(1)
				);
			}

			// Unlock and transfer tokens works
			let recipient = create_account(1123);
			assert_ok!(tokens.unlock_and_transfer(&token_owner, &recipient));

			assert!(!TokenLocks::<Test>::contains_key(token_id));

			// Verify owner of token is recipient
			assert_eq!(
				Nft::owned_tokens(collection_id, &recipient, 0, 1000),
				(0_u32, MaxTokensPerListing::get(), serial_numbers)
			);
		});
	}

	#[test]
	fn listing_tokens_unlock_and_transfer_sft() {
		TestExt::<Test>::default().build().execute_with(|| {
			let balance = 100;
			let (collection_id, token_id, token_owner) = setup_sft_token(balance);
			let tokens = ListingTokens::<Test>::Sft(SftListing {
				collection_id,
				serial_numbers: BoundedVec::truncate_from(vec![(token_id.1, balance)]),
			});

			// Lock tokens
			assert_ok!(tokens.lock_tokens(&token_owner, 1));

			// Sanity check
			let sft_balance_owner = sft_balance_of::<Test>(token_id, &token_owner);
			assert_eq!(sft_balance_owner.free_balance, 0);
			assert_eq!(sft_balance_owner.reserved_balance, balance);

			// Unlock and transfer tokens works
			let recipient = create_account(1123);
			assert_ok!(tokens.unlock_and_transfer(&token_owner, &recipient));

			let sft_balance_owner = sft_balance_of::<Test>(token_id, &token_owner);
			assert_eq!(sft_balance_owner.free_balance, 0);
			assert_eq!(sft_balance_owner.reserved_balance, 0);

			// Verify owner of token is recipient
			let sft_balance_recipient = sft_balance_of::<Test>(token_id, &recipient);
			assert_eq!(sft_balance_recipient.free_balance, balance);
			assert_eq!(sft_balance_recipient.reserved_balance, 0);
		});
	}

	#[test]
	fn listing_tokens_unlock_and_transfer_sft_multiple() {
		TestExt::<Test>::default().build().execute_with(|| {
			let balance = 100000;
			let (collection_id, _, token_owner) = setup_sft_token(balance);

			// Create max amount of tokens
			for _ in 0..MaxTokensPerListing::get() {
				assert_ok!(Sft::create_token(
					Some(create_account(100)).into(),
					collection_id,
					bounded_string("test-sft-token"),
					balance,
					None,
					Some(token_owner)
				));
			}

			let serial_numbers: Vec<SerialNumber> = (0..MaxTokensPerListing::get()).collect();
			let serials_combined: Vec<(SerialNumber, Balance)> =
				serial_numbers.iter().map(|s| (*s, balance)).collect();
			let tokens = ListingTokens::<Test>::Sft(SftListing {
				collection_id,
				serial_numbers: BoundedVec::truncate_from(serials_combined.clone()),
			});

			// Lock tokens
			assert_ok!(tokens.lock_tokens(&token_owner, 1));

			// Sanity check
			for serial_number in serial_numbers.clone() {
				let token_id = (collection_id, serial_number);
				let sft_balance_owner = sft_balance_of::<Test>(token_id, &token_owner);
				assert_eq!(sft_balance_owner.free_balance, 0);
				assert_eq!(sft_balance_owner.reserved_balance, balance);
			}

			// Unlock and transfer tokens works
			let recipient = create_account(1123);
			assert_ok!(tokens.unlock_and_transfer(&token_owner, &recipient));

			for serial_number in serial_numbers.clone() {
				let token_id = (collection_id, serial_number);
				let sft_balance_owner = sft_balance_of::<Test>(token_id, &token_owner);
				assert_eq!(sft_balance_owner.free_balance, 0);
				assert_eq!(sft_balance_owner.reserved_balance, 0);

				let sft_balance_owner = sft_balance_of::<Test>(token_id, &recipient);
				assert_eq!(sft_balance_owner.free_balance, balance);
				assert_eq!(sft_balance_owner.reserved_balance, 0);
			}
		});
	}

	#[test]
	fn listing_tokens_get_royalties_schedule_nft() {
		TestExt::<Test>::default().build().execute_with(|| {
			let entitlement_amount = Permill::from_float(0.25);
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(bob(), entitlement_amount)]),
			};
			let (collection_id, _, _) =
				setup_nft_token_with_royalties(royalties_schedule.clone(), 2);
			let tokens = ListingTokens::<Test>::Nft(NftListing {
				collection_id,
				serial_numbers: BoundedVec::truncate_from(vec![0]),
			});
			assert_eq!(tokens.get_royalties_schedule(), Ok(Some(royalties_schedule)));
		});
	}

	#[test]
	fn listing_tokens_get_royalties_schedule_sft() {
		TestExt::<Test>::default().build().execute_with(|| {
			let entitlement_amount = Permill::from_float(0.25);
			let royalties_schedule = RoyaltiesSchedule {
				entitlements: BoundedVec::truncate_from(vec![(bob(), entitlement_amount)]),
			};
			let (collection_id, token_id, _) =
				setup_sft_token_with_royalties(100, royalties_schedule.clone());
			let tokens = ListingTokens::<Test>::Sft(SftListing {
				collection_id,
				serial_numbers: BoundedVec::truncate_from(vec![(token_id.1, 100)]),
			});
			assert_eq!(tokens.get_royalties_schedule(), Ok(Some(royalties_schedule)));
		});
	}
}

mod buy_multi {
	use super::*;
	use crate::mock::MaxListingsPerMultiBuy;

	#[test]
	fn buy_multi_works() {
		let buyer = create_account(5);
		let starting_balance = 1_000_000;

		TestExt::<Test>::default()
			.with_balances(&[(buyer, starting_balance)])
			.with_asset(XRP_ASSET_ID, "XRP", &[(buyer, starting_balance)])
			.build()
			.execute_with(|| {
				// Remove fee to
				assert_ok!(Marketplace::set_fee_to(RawOrigin::Root.into(), None));

				// Setup first token which is an NFT sale
				let (collection_id1, token_id1, token_owner1) = setup_nft_token();
				let serial_numbers1: BoundedVec<SerialNumber, MaxTokensPerListing> =
					BoundedVec::truncate_from(vec![token_id1.1]);
				let nft_token = ListingTokens::Nft(NftListing {
					collection_id: collection_id1,
					serial_numbers: serial_numbers1.clone(),
				});
				let price1 = 1_000;
				let asset_id1 = NativeAssetId::get();
				let listing_id1 = Marketplace::next_listing_id();
				assert_ok!(Marketplace::sell(
					Some(token_owner1).into(),
					nft_token.clone(),
					None,
					asset_id1,
					price1,
					None,
					None,
				));

				// Setup second token which is an SFT
				let (collection_id2, token_id2, token_owner2) = setup_sft_token(1000);
				let serial_numbers2: BoundedVec<(SerialNumber, Balance), MaxTokensPerListing> =
					BoundedVec::truncate_from(vec![(token_id2.1, 1000)]);
				let sft_token = ListingTokens::Sft(SftListing {
					collection_id: collection_id2,
					serial_numbers: serial_numbers2.clone(),
				});
				let price2 = 2_000;
				let asset_id2 = XRP_ASSET_ID;
				let listing_id2 = Marketplace::next_listing_id();
				assert_ok!(Marketplace::sell(
					Some(token_owner2).into(),
					sft_token.clone(),
					None,
					asset_id2,
					price2,
					None,
					None,
				));

				// Buy multi with both listing ids
				assert_ok!(Marketplace::buy_multi(
					Some(buyer).into(),
					BoundedVec::truncate_from(vec![listing_id1, listing_id2])
				));

				// Events thrown for both buys
				System::assert_has_event(MockEvent::Marketplace(
					Event::<Test>::FixedPriceSaleComplete {
						tokens: nft_token.clone(),
						listing_id: listing_id1,
						marketplace_id: None,
						price: price1,
						payment_asset: asset_id1,
						seller: token_owner1,
						buyer,
					},
				));
				System::assert_has_event(MockEvent::Marketplace(
					Event::<Test>::FixedPriceSaleComplete {
						tokens: sft_token.clone(),
						listing_id: listing_id2,
						marketplace_id: None,
						price: price2,
						payment_asset: asset_id2,
						seller: token_owner2,
						buyer,
					},
				));

				// Check the SFT free balance of owner and buyer
				let sft_balance_owner = sft_balance_of::<Test>(token_id2, &token_owner2);
				assert_eq!(sft_balance_owner.free_balance, 0);
				assert_eq!(sft_balance_owner.reserved_balance, 0);
				let sft_balance_buyer = sft_balance_of::<Test>(token_id2, &buyer);
				assert_eq!(sft_balance_buyer.free_balance, 1000);

				// Check NFT ownership
				assert_eq!(Nft::token_balance_of(&buyer, collection_id1), 1);
				assert_eq!(Nft::token_balance_of(&token_owner1, collection_id1), 0);

				// Check balance of buyer and token owner for NFT part of the sale
				assert_eq!(AssetsExt::balance(asset_id1, &buyer), starting_balance - price1);
				assert_eq!(AssetsExt::balance(asset_id1, &token_owner1), price1);
				// Check balance of buyer and token owner for SFT part of the sale
				assert_eq!(AssetsExt::balance(asset_id2, &buyer), starting_balance - price2);
				assert_eq!(AssetsExt::balance(asset_id2, &token_owner2), price2);
			});
	}

	#[test]
	fn buy_multi_up_to_limit_works() {
		let buyer = create_account(5);
		let starting_balance = 1_000_000_000;

		TestExt::<Test>::default()
			.with_balances(&[(buyer, starting_balance)])
			.build()
			.execute_with(|| {
				// Remove fee to
				assert_ok!(Marketplace::set_fee_to(RawOrigin::Root.into(), None));

				let listing_price = 1_000;
				let mut listing_ids: Vec<ListingId> = vec![];
				let mut tokens: Vec<TokenId> = vec![];
				let token_owner = create_account(2);

				for _ in 0..MaxListingsPerMultiBuy::get() {
					let (collection_id, token_id, token_owner) = setup_nft_token();
					let serial_numbers: BoundedVec<SerialNumber, MaxTokensPerListing> =
						BoundedVec::truncate_from(vec![token_id.1]);
					let nft_token = ListingTokens::Nft(NftListing {
						collection_id,
						serial_numbers: serial_numbers.clone(),
					});
					let listing_id = Marketplace::next_listing_id();
					assert_ok!(Marketplace::sell(
						Some(token_owner).into(),
						nft_token.clone(),
						None,
						NativeAssetId::get(),
						listing_price,
						None,
						None,
					));

					listing_ids.push(listing_id);
					tokens.push(token_id);
				}

				// Buy multi with all listing ids
				assert_ok!(Marketplace::buy_multi(
					Some(buyer).into(),
					BoundedVec::try_from(listing_ids.clone()).unwrap()
				));

				// Verify data for each listing
				for (i, listing_id) in listing_ids.iter().enumerate() {
					let token_id = tokens[i];
					let nft_token = ListingTokens::Nft(NftListing {
						collection_id: token_id.0,
						serial_numbers: BoundedVec::truncate_from(vec![token_id.1]),
					});
					System::assert_has_event(MockEvent::Marketplace(
						Event::<Test>::FixedPriceSaleComplete {
							tokens: nft_token.clone(),
							listing_id: *listing_id,
							marketplace_id: None,
							price: listing_price,
							payment_asset: NativeAssetId::get(),
							seller: token_owner,
							buyer,
						},
					));

					// Check the NFT ownership
					assert_eq!(Nft::token_balance_of(&token_owner, token_id.0), 0);
					assert_eq!(Nft::token_balance_of(&buyer, token_id.0), 1);
					assert!(!Listings::<Test>::contains_key(*listing_id));
				}

				// Check balance of buyer and token owner for NFT part of the sale
				assert_eq!(
					AssetsExt::balance(NativeAssetId::get(), &buyer),
					starting_balance - (listing_price * MaxListingsPerMultiBuy::get() as u128)
				);
				assert_eq!(
					AssetsExt::balance(NativeAssetId::get(), &token_owner),
					(listing_price * MaxListingsPerMultiBuy::get() as u128)
				);
			});
	}
}
