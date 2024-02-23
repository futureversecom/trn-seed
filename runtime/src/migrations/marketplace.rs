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

use crate::{Marketplace, Runtime, Weight};
use frame_support::{
	dispatch::GetStorageVersion,
	traits::{OnRuntimeUpgrade, StorageVersion},
};
#[allow(unused_imports)]
use sp_std::vec::Vec;

pub struct Upgrade;

impl OnRuntimeUpgrade for Upgrade {
	fn on_runtime_upgrade() -> Weight {
		let current = Marketplace::current_storage_version();
		let onchain = Marketplace::on_chain_storage_version();
		log::info!(target: "Migration", "Marketplace: Running migration with current storage version {current:?} / on-chain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(2);

		if onchain == 1 {
			log::info!(target: "Migration", "Marketplace: Migrating from on-chain version 1 to on-chain version 2.");
			weight += v2::migrate::<Runtime>();

			StorageVersion::new(2).put::<Marketplace>();

			log::info!(target: "Migration", "Nft: Migration successfully completed.");
		} else {
			log::info!(target: "Migration", "Nft: No migration was done, however migration code needs to be removed.");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		v2::pre_upgrade()?;
		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), &'static str> {
		v2::post_upgrade()?;
		Ok(())
	}
}

#[allow(dead_code)]
#[allow(unused_imports)]
pub mod v2 {
	use super::*;
	use crate::migrations::{Map, Value};
	use codec::{Decode, Encode, MaxEncodedLen};
	use frame_support::{storage_alias, weights::Weight, BoundedVec, StorageHasher, Twox64Concat};
	use pallet_marketplace::{
		types::{
			AuctionListing, FixedPriceListing, Listing, Listing::FixedPrice, ListingTokens,
			Marketplace as MarketplaceS, MarketplaceId, NftListing, OfferId, OfferType,
			SimpleOffer,
		},
		Listings,
	};
	use scale_info::TypeInfo;
	use seed_primitives::{
		AssetId, Balance, CollectionUuid, ListingId, RoyaltiesSchedule, SerialNumber, TokenId,
	};

	type AccountId = <Runtime as frame_system::Config>::AccountId;
	type BlockNumber = <Runtime as frame_system::Config>::BlockNumber;

	/// A type of NFT sale listing
	#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub enum OldListing<T: pallet_marketplace::Config> {
		FixedPrice(OldFixedPriceListing<T>),
		Auction(OldAuctionListing<T>),
	}

	/// Information about an auction listing
	#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct OldAuctionListing<T: pallet_marketplace::Config> {
		pub payment_asset: AssetId,
		pub reserve_price: Balance,
		pub close: T::BlockNumber,
		pub seller: T::AccountId,
		pub collection_id: CollectionUuid,
		pub serial_numbers:
			BoundedVec<SerialNumber, <T as pallet_marketplace::Config>::MaxTokensPerListing>,
		pub royalties_schedule: RoyaltiesSchedule<T::AccountId>,
		pub marketplace_id: Option<MarketplaceId>,
	}

	/// Information about a fixed price listing
	#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct OldFixedPriceListing<T: pallet_marketplace::Config> {
		pub payment_asset: AssetId,
		pub fixed_price: Balance,
		pub close: T::BlockNumber,
		pub buyer: Option<T::AccountId>,
		pub seller: T::AccountId,
		pub collection_id: CollectionUuid,
		pub serial_numbers:
			BoundedVec<SerialNumber, <T as pallet_marketplace::Config>::MaxTokensPerListing>,
		pub royalties_schedule: RoyaltiesSchedule<T::AccountId>,
		pub marketplace_id: Option<MarketplaceId>,
	}

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Marketplace: Upgrade to v2 Pre Upgrade.");
		let onchain = Marketplace::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 2 {
			return Ok(())
		}
		assert_eq!(onchain, 1);

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Marketplace: Upgrade to v2 Post Upgrade.");
		let current = Marketplace::current_storage_version();
		let onchain = Marketplace::on_chain_storage_version();
		assert_eq!(current, 2);
		assert_eq!(onchain, 2);
		Ok(())
	}

	pub fn migrate<T: pallet_nft::Config + pallet_marketplace::Config>() -> Weight
	where
		AccountId: From<sp_core::H160>,
	{
		log::info!(target: "Migration", "Marketplace: migrating listing tokens");
		let mut weight = Weight::zero();

		Listings::<Runtime>::translate::<OldListing<Runtime>, _>(|_listing_id, listing| {
			// Reads: Listings
			// Writes: Listings
			weight += <Runtime as frame_system::Config>::DbWeight::get().reads_writes(1, 1);

			let new_listing = match listing {
				OldListing::FixedPrice(old_listing) => {
					let nft_tokens = ListingTokens::Nft(NftListing {
						collection_id: old_listing.collection_id,
						serial_numbers: old_listing.serial_numbers,
					});
					let new_listing = Listing::FixedPrice(FixedPriceListing {
						payment_asset: old_listing.payment_asset,
						fixed_price: old_listing.fixed_price,
						close: old_listing.close,
						buyer: old_listing.buyer,
						seller: old_listing.seller,
						tokens: nft_tokens,
						royalties_schedule: old_listing.royalties_schedule,
						marketplace_id: old_listing.marketplace_id,
					});
					new_listing
				},
				OldListing::Auction(old_listing) => {
					let nft_tokens = ListingTokens::Nft(NftListing {
						collection_id: old_listing.collection_id,
						serial_numbers: old_listing.serial_numbers,
					});
					let new_listing = Listing::Auction(AuctionListing {
						payment_asset: old_listing.payment_asset,
						reserve_price: old_listing.reserve_price,
						close: old_listing.close,
						seller: old_listing.seller,
						tokens: nft_tokens,
						royalties_schedule: old_listing.royalties_schedule,
						marketplace_id: old_listing.marketplace_id,
					});
					new_listing
				},
			};

			Some(new_listing)
		});

		log::info!(target: "Migration", "Marketplace: successfully migrated listing tokens");

		weight
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
				StorageVersion::new(1).put::<Marketplace>();

				// Fixed Price listing
				let listing_key_1 = Twox64Concat::hash(&(1 as ListingId).encode());
				let fixed_listing = OldListing::<Runtime>::FixedPrice(OldFixedPriceListing {
					payment_asset: 1,
					fixed_price: 2,
					close: 3,
					buyer: Some(create_account(4)),
					seller: create_account(5),
					collection_id: 6,
					serial_numbers: BoundedVec::truncate_from(vec![7, 8, 9]),
					royalties_schedule: RoyaltiesSchedule::default(),
					marketplace_id: Some(10),
				});
				Map::unsafe_storage_put::<OldListing<Runtime>>(
					b"Marketplace",
					b"Listings",
					&listing_key_1,
					fixed_listing.clone(),
				);

				// Auction Listing
				let listing_key_2 = Twox64Concat::hash(&(2 as ListingId).encode());
				let auction_listing = OldListing::<Runtime>::Auction(OldAuctionListing {
					payment_asset: 11,
					reserve_price: 12,
					close: 13,
					seller: create_account(14),
					collection_id: 15,
					serial_numbers: BoundedVec::truncate_from(vec![16, 17, 18]),
					royalties_schedule: RoyaltiesSchedule::default(),
					marketplace_id: Some(19),
				});
				Map::unsafe_storage_put::<OldListing<Runtime>>(
					b"Marketplace",
					b"Listings",
					&listing_key_2,
					auction_listing.clone(),
				);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();
				assert_eq!(Marketplace::on_chain_storage_version(), 2);

				let expected_fixed_listing = Listing::FixedPrice(FixedPriceListing {
					payment_asset: 1,
					fixed_price: 2,
					close: 3,
					buyer: Some(create_account(4)),
					seller: create_account(5),
					tokens: ListingTokens::Nft(NftListing {
						collection_id: 6,
						serial_numbers: BoundedVec::truncate_from(vec![7, 8, 9]),
					}),
					royalties_schedule: RoyaltiesSchedule::default(),
					marketplace_id: Some(10),
				});
				assert_eq!(
					Map::unsafe_storage_get::<Listing<Runtime>>(
						b"Marketplace",
						b"Listings",
						&listing_key_1,
					),
					Some(expected_fixed_listing)
				);

				let expected_auction_listing = Listing::Auction(AuctionListing {
					payment_asset: 11,
					reserve_price: 12,
					close: 13,
					seller: create_account(14),
					tokens: ListingTokens::Nft(NftListing {
						collection_id: 15,
						serial_numbers: BoundedVec::truncate_from(vec![16, 17, 18]),
					}),
					royalties_schedule: RoyaltiesSchedule::default(),
					marketplace_id: Some(19),
				});
				assert_eq!(
					Map::unsafe_storage_get::<Listing<Runtime>>(
						b"Marketplace",
						b"Listings",
						&listing_key_2,
					),
					Some(expected_auction_listing)
				);
			});
		}
	}
}
