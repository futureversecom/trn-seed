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

use crate::{Nft, Runtime, Weight};
use frame_support::{
	dispatch::GetStorageVersion,
	traits::{OnRuntimeUpgrade, StorageVersion},
};
#[allow(unused_imports)]
use sp_std::vec::Vec;

pub struct Upgrade;

impl OnRuntimeUpgrade for Upgrade {
	fn on_runtime_upgrade() -> Weight {
		let current = Nft::current_storage_version();
		let onchain = Nft::on_chain_storage_version();
		log::info!(target: "Migration", "NFT: Running migration with current storage version {current:?} / on-chain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(2);

		if onchain == 6 {
			log::info!(target: "Migration", "NFT: Migrating from on-chain version 6 to on-chain version 7.");
			weight += v7::migrate::<Runtime>();

			StorageVersion::new(7).put::<Nft>();

			log::info!(target: "Migration", "NFT: Migration successfully completed.");
		} else {
			log::info!(target: "Migration", "NFT: No migration was done, however migration code needs to be removed.");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		v7::pre_upgrade()?;
		Ok(Vec::new())
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
		sp_runtime::RuntimeDebug, storage_alias, weights::Weight, BoundedVec, StorageHasher,
		Twox64Concat,
	};
	use pallet_evm_precompiles_futurepass::Action::Default;
	use pallet_marketplace::types::{
		AuctionListing, FixedPriceListing, Listing, ListingTokens, NftListing,
	};
	use pallet_nft::TokenLocks;
	use scale_info::TypeInfo;
	use seed_primitives::{
		AssetId, Balance, CollectionUuid, RoyaltiesSchedule, TokenId, TokenLockReason,
	};
	use sp_core::{Get, H160};
	use sp_runtime::Permill;

	type AccountId = <Runtime as frame_system::Config>::AccountId;
	type BlockNumber = <Runtime as frame_system::Config>::BlockNumber;

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Nft: Upgrade to v7 Pre Upgrade.");
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
		log::info!(target: "Migration", "Nft: Upgrade to v7 Post Upgrade.");
		let current = Nft::current_storage_version();
		let onchain = Nft::on_chain_storage_version();
		assert_eq!(current, 7);
		assert_eq!(onchain, 7);
		Ok(())
	}

	pub fn migrate<T: frame_system::Config + pallet_nft::Config + pallet_marketplace::Config>(
	) -> Weight
	where
		AccountId: From<sp_core::H160>,
	{
		log::info!(target: "Migration", "Nft: migrating locked tokens");
		let mut weight = Weight::zero();

		TokenLocks::<T>::translate::<TokenLockReason, _>(|_token_id, token_lock_reason| {
			weight = weight
				.saturating_add(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1));
			let TokenLockReason::Listed(listing_id) = token_lock_reason;
			if pallet_marketplace::Listings::<T>::contains_key(listing_id) {
				Some(token_lock_reason)
			} else {
				None
			}
		});

		log::info!(target: "Migration", "Nft: successfully migrated SaleInfo");

		weight
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::new_test_ext;
		use pallet_marketplace::types::AuctionListing;

		fn create_account(seed: u64) -> AccountId {
			AccountId::from(H160::from_low_u64_be(seed))
		}

		#[test]
		fn migration_test() {
			new_test_ext().execute_with(|| {
				// Setup storage
				StorageVersion::new(6).put::<Nft>();

				// token locks with no listings
				let token_id_1: TokenId = (1124, 1);
				let lock_reason_1 = TokenLockReason::Listed(1);
				TokenLocks::<Runtime>::insert(token_id_1, lock_reason_1);
				let token_id_2: TokenId = (1124, 2);
				let lock_reason_2 = TokenLockReason::Listed(2);
				TokenLocks::<Runtime>::insert(token_id_2, lock_reason_2);
				let token_id_3: TokenId = (2256, 3);
				let lock_reason_3 = TokenLockReason::Listed(3);
				TokenLocks::<Runtime>::insert(token_id_3, lock_reason_3);

				// Token locks with assosciated listing
				// Fixed price
				let token_id_4: TokenId = (2256, 4);
				let lock_reason_4 = TokenLockReason::Listed(4);
				TokenLocks::<Runtime>::insert(token_id_4, lock_reason_4);
				let listing_4 = Listing::FixedPrice(FixedPriceListing {
					payment_asset: 1,
					fixed_price: 2,
					close: 3,
					buyer: None,
					seller: create_account(4),
					tokens: ListingTokens::Nft(NftListing {
						collection_id: 2256,
						serial_numbers: BoundedVec::truncate_from(vec![4]),
					}),
					royalties_schedule: RoyaltiesSchedule::default(),
					marketplace_id: None,
				});
				pallet_marketplace::Listings::<Runtime>::insert(4, listing_4);

				// Auction
				let token_id_5: TokenId = (3316, 5);
				let lock_reason_5 = TokenLockReason::Listed(5);
				TokenLocks::<Runtime>::insert(token_id_5, lock_reason_5);
				let listing_5 = Listing::Auction(AuctionListing {
					payment_asset: 1,
					reserve_price: 0,
					close: 3,
					seller: create_account(5),
					tokens: ListingTokens::Nft(NftListing {
						collection_id: 3316,
						serial_numbers: BoundedVec::truncate_from(vec![5]),
					}),
					royalties_schedule: RoyaltiesSchedule::default(),
					marketplace_id: None,
				});
				pallet_marketplace::Listings::<Runtime>::insert(5, listing_5);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();
				assert_eq!(Nft::on_chain_storage_version(), 7);

				// Check token locks removed for tokens without a listing
				assert!(!TokenLocks::<Runtime>::contains_key(token_id_1));
				assert!(!TokenLocks::<Runtime>::contains_key(token_id_2));
				assert!(!TokenLocks::<Runtime>::contains_key(token_id_3));

				// token locks not removed for tokens with a listing
				assert!(TokenLocks::<Runtime>::contains_key(token_id_4));
				assert!(TokenLocks::<Runtime>::contains_key(token_id_5));
			});
		}
	}
}
