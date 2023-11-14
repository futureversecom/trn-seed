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

use crate::{Marketplace, Nft, Runtime, Weight};
use frame_support::{
	dispatch::GetStorageVersion,
	traits::{OnRuntimeUpgrade, StorageVersion},
};
use sp_std::vec::Vec;

pub struct Upgrade;

impl OnRuntimeUpgrade for Upgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		v7::pre_upgrade()?;
		Ok(Vec::new())
	}

	fn on_runtime_upgrade() -> Weight {
		let current = Nft::current_storage_version();
		let onchain = Nft::on_chain_storage_version();
		log::info!(target: "Migration", "Nft: Running migration with current storage version {current:?} / on-chain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(2);

		if onchain == 5 {
			log::info!(target: "Migration", "Nft: Migrating from on-chain version 5 to on-chain version 6.");
			weight += v7::migrate::<Runtime>();

			StorageVersion::new(6).put::<Nft>();
			StorageVersion::new(1).put::<Marketplace>();

			log::info!(target: "Migration", "Nft: Migration successfully finished.");
		} else {
			log::info!(target: "Migration", "Nft: No migration was done. If you are seeing this message, it means that you forgot to remove old existing migration code. Don't panic, it's not a big deal just don't forget it next time :)");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), &'static str> {
		v7::post_upgrade()?;
		Ok(())
	}
}

#[allow(dead_code)]
#[allow(unused_imports)]
pub mod v7 {
	use super::*;
	use crate::migrations::{Map, Value};
	use codec::{Decode, Encode, MaxEncodedLen};
	use frame_support::{
		storage_alias, weights::Weight, BoundedVec, CloneNoBound, PartialEqNoBound,
		RuntimeDebugNoBound, StorageHasher, Twox64Concat,
	};
	use pallet_marketplace::types::{
		FixedPriceListing, Listing, Listing::FixedPrice, Marketplace as MarketplaceS,
		MarketplaceId, OfferId, OfferType, SimpleOffer,
	};
	use pallet_nft::CrossChainCompatibility;
	use scale_info::TypeInfo;
	use seed_primitives::{
		Balance, CollectionUuid, ListingId, MetadataScheme, OriginChain, RoyaltiesSchedule,
		SerialNumber, TokenCount, TokenId,
	};
	use sp_core::Get;
	use std::fmt::Debug;

	type AccountId = <Runtime as frame_system::Config>::AccountId;
	type BlockNumber = <Runtime as frame_system::Config>::BlockNumber;

	/// Information related to a specific collection
	#[derive(
		PartialEqNoBound, RuntimeDebugNoBound, CloneNoBound, Encode, Decode, TypeInfo, MaxEncodedLen,
	)]
	#[codec(mel_bound(AccountId: MaxEncodedLen))]
	#[scale_info(skip_type_params(MaxTokensPerCollection, StringLimit))]
	pub struct OldCollectionInformation<AccountId, MaxTokensPerCollection, StringLimit>
	where
		AccountId: Debug + PartialEq + Clone,
		MaxTokensPerCollection: Get<u32>,
		StringLimit: Get<u32>,
	{
		/// The owner of the collection
		pub owner: AccountId,
		/// A human friendly name
		pub name: BoundedVec<u8, StringLimit>,
		/// Collection metadata reference scheme
		pub metadata_scheme: MetadataScheme,
		/// configured royalties schedule
		pub royalties_schedule: Option<RoyaltiesSchedule<AccountId>>,
		/// Maximum number of tokens allowed in a collection
		pub max_issuance: Option<TokenCount>,
		/// The chain in which the collection was minted originally
		pub origin_chain: OriginChain,
		/// The next available serial_number
		pub next_serial_number: SerialNumber,
		/// the total count of tokens in this collection
		pub collection_issuance: TokenCount,
		/// This collections compatibility with other chains
		pub cross_chain_compatibility: CrossChainCompatibility,
		/// All serial numbers owned by an account in a collection
		pub owned_tokens:
			BoundedVec<TokenOwnership<AccountId, MaxTokensPerCollection>, MaxTokensPerCollection>,
	}

	/// Struct that represents the owned serial numbers within a collection of an individual account
	#[derive(
		PartialEqNoBound, RuntimeDebugNoBound, Decode, Encode, CloneNoBound, TypeInfo, MaxEncodedLen,
	)]
	#[codec(mel_bound(AccountId: MaxEncodedLen))]
	#[scale_info(skip_type_params(MaxTokensPerCollection))]
	pub struct TokenOwnership<AccountId, MaxTokensPerCollection>
	where
		AccountId: Debug + PartialEq + Clone,
		MaxTokensPerCollection: Get<u32>,
	{
		pub owner: AccountId,
		pub owned_serials: BoundedVec<SerialNumber, MaxTokensPerCollection>,
	}

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Nft: Upgrade to v6 Pre Upgrade.");
		let onchain = Nft::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 7 {
			return Ok(())
		}
		assert_eq!(onchain, 6);

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Nft: Upgrade to v6 Post Upgrade.");

		let current = Nft::current_storage_version();
		let onchain = Nft::on_chain_storage_version();
		assert_eq!(current, 7);
		assert_eq!(onchain, 7);
		Ok(())
	}

	pub fn migrate<T: pallet_nft::Config + pallet_marketplace::Config>() -> Weight
	where
		AccountId: From<sp_core::H160>,
	{
		log::info!(target: "Migration", "Nft: Migrating token ownership to it's own storage item.");

		Value::unsafe_storage_move(b"NextMarketplaceId", b"Nft", b"Marketplace");
		Map::unsafe_storage_move::<MarketplaceId, MarketplaceS<AccountId>, Twox64Concat>(
			b"RegisteredMarketplaces",
			b"Nft",
			b"Marketplace",
		);
		Map::unsafe_storage_move::<ListingId, Listing<T>, Twox64Concat>(
			b"Listings",
			b"Nft",
			b"Marketplace",
		);
		Value::unsafe_storage_move(b"NextListingId", b"Nft", b"Marketplace");
		Map::unsafe_storage_move::<(CollectionUuid, ListingId), bool, Twox64Concat>(
			b"OpenCollectionListings",
			b"Nft",
			b"Marketplace",
		);
		Map::unsafe_storage_move::<ListingId, (AccountId, Balance), Twox64Concat>(
			b"ListingWinningBid",
			b"Nft",
			b"Marketplace",
		);
		Map::unsafe_storage_move::<(BlockNumber, ListingId), bool, Twox64Concat>(
			b"ListingEndSchedule",
			b"Nft",
			b"Marketplace",
		);
		Map::unsafe_storage_move::<OfferId, OfferType<AccountId>, Twox64Concat>(
			b"Offers",
			b"Nft",
			b"Marketplace",
		);
		// TODO Check whether this config trait bound for boundedVec works as expected
		Map::unsafe_storage_move::<
			TokenId,
			BoundedVec<OfferId, <T as pallet_marketplace::Config>::MaxOffers>,
			Twox64Concat,
		>(b"TokenOffers", b"Nft", b"Marketplace");
		Value::unsafe_storage_move(b"NextOfferId", b"Nft", b"Marketplace");

		log::info!(target: "Nft", "...Successfully migrated token ownership map");

		<Runtime as frame_system::Config>::DbWeight::get().writes(10_u64)
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::new_test_ext;
		use sp_core::H160;
		use sp_runtime::Permill;

		fn create_account(seed: u64) -> AccountId {
			AccountId::from(H160::from_low_u64_be(seed))
		}

		#[test]
		fn migration_test() {
			new_test_ext().execute_with(|| {
				// Setup storage
				StorageVersion::new(5).put::<Nft>();
				StorageVersion::new(0).put::<Marketplace>();

				// NextMarketplaceId
				let next_marketplace_id: MarketplaceId = 12;
				Value::unsafe_storage_put::<MarketplaceId>(
					b"Nft",
					b"NextMarketplaceId",
					next_marketplace_id,
				);

				// RegisteredMarketplaces
				let marketplace_key = Twox64Concat::hash(&(1 as MarketplaceId).encode());
				let registered_marketplace = MarketplaceS {
					account: create_account(1),
					entitlement: Permill::from_parts(123),
				};
				Map::unsafe_storage_put::<MarketplaceS<AccountId>>(
					b"Nft",
					b"RegisteredMarketplaces",
					&marketplace_key,
					registered_marketplace.clone(),
				);

				// Listings
				let listing_key = Twox64Concat::hash(&(1 as ListingId).encode());
				let listing = Listing::<Runtime>::FixedPrice(FixedPriceListing {
					payment_asset: 1,
					fixed_price: 2,
					close: 3,
					buyer: None,
					seller: create_account(4),
					collection_id: 5,
					serial_numbers: BoundedVec::truncate_from(vec![6, 7, 8]),
					royalties_schedule: RoyaltiesSchedule::default(),
					marketplace_id: None,
				});
				Map::unsafe_storage_put::<Listing<Runtime>>(
					b"Nft",
					b"Listings",
					&listing_key,
					listing.clone(),
				);

				// NextListingId
				let next_listing_id: ListingId = 9;
				Value::unsafe_storage_put::<ListingId>(b"Nft", b"NextListingId", next_listing_id);

				// OpenCollectionListings
				let mut open_collection_listings_key =
					Twox64Concat::hash(&(1 as CollectionUuid).encode());
				let open_collection_listings_key_2 = Twox64Concat::hash(&(2 as ListingId).encode());
				open_collection_listings_key.extend_from_slice(&open_collection_listings_key_2);
				let open_collection_listings = true;
				Map::unsafe_storage_put::<bool>(
					b"Nft",
					b"OpenCollectionListings",
					&open_collection_listings_key,
					open_collection_listings.clone(),
				);

				// ListingWinningBid
				let listing_winning_bid_key = Twox64Concat::hash(&(1 as ListingId).encode());
				let listing_winning_bid = (create_account(2), 3);
				Map::unsafe_storage_put::<(AccountId, Balance)>(
					b"Nft",
					b"ListingWinningBid",
					&listing_winning_bid_key,
					listing_winning_bid.clone(),
				);

				// ListingEndSchedule
				let mut listing_end_schedule_key =
					Twox64Concat::hash(&BlockNumber::default().encode());
				let listing_end_schedule_key_2 = Twox64Concat::hash(&(2 as ListingId).encode());
				listing_end_schedule_key.extend_from_slice(&listing_end_schedule_key_2);
				let listing_end_schedule = true;
				Map::unsafe_storage_put::<bool>(
					b"Nft",
					b"ListingEndSchedule",
					&listing_end_schedule_key,
					listing_end_schedule.clone(),
				);

				// Offers
				let offer_key = Twox64Concat::hash(&(1 as OfferId).encode());
				let offer = OfferType::Simple(SimpleOffer {
					token_id: (0, 1),
					asset_id: 2,
					amount: 3,
					buyer: create_account(4),
					marketplace_id: Some(5 as MarketplaceId),
				});
				Map::unsafe_storage_put::<OfferType<AccountId>>(
					b"Nft",
					b"Offers",
					&offer_key,
					offer.clone(),
				);

				// TokenOffers
				let token_offers_key = Twox64Concat::hash(&((0_u32, 1_u32) as TokenId).encode());
				let token_offers = BoundedVec::truncate_from(vec![1 as OfferId, 2 as OfferId]);
				Map::unsafe_storage_put::<
					BoundedVec<OfferId, <Runtime as pallet_marketplace::Config>::MaxOffers>,
				>(b"Nft", b"TokenOffers", &token_offers_key, token_offers.clone());

				// NextOfferId
				let next_offer_id: OfferId = 3;
				Value::unsafe_storage_put::<OfferId>(b"Nft", b"NextOfferId", next_offer_id);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();

				assert_eq!(
					Value::unsafe_storage_get::<MarketplaceId>(
						b"Marketplace",
						b"NextMarketplaceId",
					),
					Some(next_marketplace_id)
				);
				assert_eq!(
					Map::unsafe_storage_get::<MarketplaceS<AccountId>>(
						b"Marketplace",
						b"RegisteredMarketplaces",
						&marketplace_key,
					),
					Some(registered_marketplace)
				);
				assert_eq!(
					Map::unsafe_storage_get::<Listing<Runtime>>(
						b"Marketplace",
						b"Listings",
						&listing_key,
					),
					Some(listing)
				);
				assert_eq!(
					Value::unsafe_storage_get::<ListingId>(b"Marketplace", b"NextListingId"),
					Some(next_listing_id)
				);
				assert_eq!(
					Map::unsafe_storage_get::<bool>(
						b"Marketplace",
						b"OpenCollectionListings",
						&open_collection_listings_key,
					),
					Some(open_collection_listings)
				);
				assert_eq!(
					Map::unsafe_storage_get::<(AccountId, Balance)>(
						b"Marketplace",
						b"ListingWinningBid",
						&listing_winning_bid_key,
					),
					Some(listing_winning_bid)
				);
				assert_eq!(
					Map::unsafe_storage_get::<bool>(
						b"Marketplace",
						b"ListingEndSchedule",
						&listing_end_schedule_key,
					),
					Some(listing_end_schedule)
				);
				assert_eq!(
					Map::unsafe_storage_get::<OfferType<AccountId>>(
						b"Marketplace",
						b"Offers",
						&offer_key,
					),
					Some(offer)
				);
				assert_eq!(
					Map::unsafe_storage_get::<
						BoundedVec<OfferId, <Runtime as pallet_marketplace::Config>::MaxOffers>,
					>(b"Marketplace", b"TokenOffers", &token_offers_key),
					Some(token_offers)
				);
				assert_eq!(
					Value::unsafe_storage_get::<OfferId>(b"Marketplace", b"NextOfferId"),
					Some(next_offer_id)
				);

				// Check if version has been set correctly
				assert_eq!(Nft::on_chain_storage_version(), 6);
				assert_eq!(Marketplace::on_chain_storage_version(), 1);
			});
		}
	}
}
