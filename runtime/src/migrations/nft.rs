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
#[cfg(feature = "try-runtime")]
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

		if onchain == 6 {
			log::info!(target: "Migration", "Nft: Migrating from on-chain version 6 to on-chain version 7.");
			weight += v7::migrate::<Runtime>();

			StorageVersion::new(7).put::<Nft>();

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
	use crate::{
		migrations::{Map, Value},
		CollectionNameStringLimit, MaxTokensPerCollection,
	};
	use codec::{Decode, Encode, MaxEncodedLen};
	use core::fmt::Debug;
	use frame_support::{
		storage_alias, weights::Weight, BoundedVec, CloneNoBound, PartialEqNoBound,
		RuntimeDebugNoBound, StorageHasher, Twox64Concat,
	};
	use pallet_marketplace::types::{
		FixedPriceListing, Listing, Listing::FixedPrice, Marketplace as MarketplaceS,
		MarketplaceId, OfferId, OfferType, SimpleOffer,
	};
	use pallet_nft::{
		CollectionInfo, CollectionInformation, CrossChainCompatibility, OwnedTokens, OwnershipInfo,
		TokenOwnership,
	};
	use scale_info::TypeInfo;
	use seed_primitives::{
		Balance, CollectionUuid, ListingId, MetadataScheme, OriginChain, RoyaltiesSchedule,
		SerialNumber, TokenCount, TokenId,
	};
	use sp_core::Get;

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
		pub owned_tokens: BoundedVec<
			OldTokenOwnership<AccountId, MaxTokensPerCollection>,
			MaxTokensPerCollection,
		>,
	}

	/// Struct that represents the owned serial numbers within a collection of an individual account
	#[derive(
		PartialEqNoBound, RuntimeDebugNoBound, Decode, Encode, CloneNoBound, TypeInfo, MaxEncodedLen,
	)]
	#[codec(mel_bound(AccountId: MaxEncodedLen))]
	#[scale_info(skip_type_params(MaxTokensPerCollection))]
	pub struct OldTokenOwnership<AccountId, MaxTokensPerCollection>
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
		let mut weight = Weight::zero();

		CollectionInfo::<Runtime>::translate::<
			OldCollectionInformation<AccountId, MaxTokensPerCollection, CollectionNameStringLimit>,
			_,
		>(|collection_id, collection_info| {
			// Reads: CollectionInfo
			// Writes: CollectionInfo, TokenOwnership
			weight += <Runtime as frame_system::Config>::DbWeight::get().reads_writes(1, 2);

			// Construct ownership info out of old ownership info
			let token_ownership_old = collection_info.owned_tokens;
			let mut token_ownership_new = TokenOwnership::default();
			token_ownership_old.iter().for_each(|ownership| {
				// TODO better error handling
				let _ = token_ownership_new
					.owned_tokens
					.try_push((ownership.owner, ownership.clone().owned_serials));
			});
			let collection_id_key = Twox64Concat::hash(&collection_id.encode());
			Map::unsafe_storage_put::<TokenOwnership<AccountId, MaxTokensPerCollection>>(
				b"Nft",
				b"OwnershipInfo",
				&collection_id_key,
				token_ownership_new,
			);

			// Construct new collection info without ownership info
			let new_collection_info = CollectionInformation {
				owner: collection_info.owner,
				name: collection_info.name,
				metadata_scheme: collection_info.metadata_scheme,
				royalties_schedule: collection_info.royalties_schedule,
				max_issuance: collection_info.max_issuance,
				origin_chain: collection_info.origin_chain,
				next_serial_number: collection_info.next_serial_number,
				collection_issuance: collection_info.collection_issuance,
				cross_chain_compatibility: collection_info.cross_chain_compatibility,
			};

			Some(new_collection_info)
		});

		log::info!(target: "Nft", "...Successfully migrated token ownership map");

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
				StorageVersion::new(6).put::<Nft>();

				let collection_id = 1124;
				let owned_tokens_1 = OldTokenOwnership {
					owner: create_account(5),
					owned_serials: BoundedVec::truncate_from(vec![1, 2, 3, 4, 5, 6, 8, 9]),
				};
				let owned_tokens_2 = OldTokenOwnership {
					owner: create_account(6),
					owned_serials: BoundedVec::truncate_from(vec![10, 20, 30, 40]),
				};
				let old_collection_info = OldCollectionInformation {
					owner: create_account(4),
					name: BoundedVec::truncate_from(vec![1, 2, 3]),
					metadata_scheme: MetadataScheme::try_from(b"example.com/metadata/".as_slice())
						.unwrap(),
					royalties_schedule: None,
					max_issuance: Some(100),
					origin_chain: OriginChain::Ethereum,
					next_serial_number: 123,
					collection_issuance: 123,
					cross_chain_compatibility: CrossChainCompatibility::default(),
					owned_tokens: BoundedVec::truncate_from(vec![owned_tokens_1, owned_tokens_2]),
				};
				let collection_id_key = Twox64Concat::hash(&collection_id.encode());
				Map::unsafe_storage_put::<
					OldCollectionInformation<
						AccountId,
						MaxTokensPerCollection,
						CollectionNameStringLimit,
					>,
				>(b"Nft", b"CollectionInfo", &collection_id_key, old_collection_info);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();

				// Check if version has been set correctly
				assert_eq!(Nft::on_chain_storage_version(), 7);

				// Check storage is correctly updated
				let new_collection_info = CollectionInformation {
					owner: create_account(4),
					name: BoundedVec::truncate_from(vec![1, 2, 3]),
					metadata_scheme: MetadataScheme::try_from(b"example.com/metadata/".as_slice())
						.unwrap(),
					royalties_schedule: None,
					max_issuance: Some(100),
					origin_chain: OriginChain::Ethereum,
					next_serial_number: 123,
					collection_issuance: 123,
					cross_chain_compatibility: CrossChainCompatibility::default(),
				};
				assert_eq!(
					Map::unsafe_storage_get::<
						CollectionInformation<AccountId, CollectionNameStringLimit>,
					>(b"Nft", b"CollectionInfo", &collection_id_key,),
					Some(new_collection_info)
				);

				let owned_tokens_1 =
					(create_account(5), BoundedVec::truncate_from(vec![1, 2, 3, 4, 5, 6, 8, 9]));
				let owned_tokens_2 =
					(create_account(6), BoundedVec::truncate_from(vec![10, 20, 30, 40]));
				let token_ownership = TokenOwnership {
					owned_tokens: BoundedVec::truncate_from(vec![owned_tokens_1, owned_tokens_2]),
				};
				assert_eq!(
					Map::unsafe_storage_get::<TokenOwnership<AccountId, MaxTokensPerCollection>>(
						b"Nft",
						b"OwnershipInfo",
						&collection_id_key,
					),
					Some(token_ownership)
				);
			});
		}
	}
}
