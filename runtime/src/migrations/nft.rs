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

pub struct Upgrade;
impl OnRuntimeUpgrade for Upgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		v5::pre_upgrade()?;
		Ok(())
	}

	fn on_runtime_upgrade() -> Weight {
		let current = Nft::current_storage_version();
		let onchain = Nft::on_chain_storage_version();
		log::info!(target: "Migration", "Nft: Running migration with current storage version {current:?} / onchain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads_writes(2, 0);

		if onchain == 4 {
			log::info!(target: "Migration", "Nft: Migrating from onchain version 4 to onchain version 5.");
			weight += v5::migrate::<Runtime>();

			log::info!(target: "Migration", "Nft: Migration successfully finished.");
			StorageVersion::new(5).put::<Nft>();
		} else {
			log::info!(target: "Migration", "Nft: No migration was done. If you are seeing this message, it means that you forgot to remove old existing migration code. Don't panic, it's not a big deal just don't forget it next time :)");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		v5::post_upgrade()?;
		Ok(())
	}
}

#[allow(dead_code)]
pub mod v5 {
	use super::*;
	use codec::{Decode, Encode};
	use frame_support::{storage_alias, weights::Weight, Twox64Concat};
	use pallet_nft::{
		CollectionInformation, Config, CrossChainCompatibility, Pallet, TokenOwnership,
	};
	use scale_info::TypeInfo;
	use seed_primitives::{
		CollectionUuid, MetadataScheme, OriginChain, RoyaltiesSchedule, SerialNumber, TokenCount,
	};
	use sp_runtime::{BoundedVec, Permill};
	use sp_std::vec::Vec;

	/// old NFT collection name type
	pub type CollectionNameType = Vec<u8>;

	/// Old unbounded Royalties Schedule
	#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
	pub struct OldRoyaltiesSchedule<AccountId> {
		pub entitlements: Vec<(AccountId, Permill)>,
	}

	/// Information related to a specific collection
	#[derive(Debug, Clone, Encode, Decode, PartialEq, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct OldCollectionInformation<T: Config> {
		/// The owner of the collection
		pub owner: T::AccountId,
		/// A human friendly name
		pub name: CollectionNameType,
		/// Collection metadata reference scheme
		pub metadata_scheme: MetadataScheme,
		/// configured royalties schedule
		pub royalties_schedule: Option<OldRoyaltiesSchedule<T::AccountId>>,
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
			TokenOwnership<T::AccountId, T::MaxTokensPerCollection>,
			<T as Config>::MaxTokensPerCollection,
		>,
	}

	#[storage_alias]
	type CollectionInfo<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, CollectionUuid, OldCollectionInformation<T>>;

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Nft: Upgrade to v5 Pre Upgrade.");
		let onchain = Nft::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 5 {
			return Ok(())
		}
		assert_eq!(onchain, 4);

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Nft: Upgrade to v5 Post Upgrade.");

		let current = Nft::current_storage_version();
		let onchain = Nft::on_chain_storage_version();
		assert_eq!(current, 5);
		assert_eq!(onchain, 5);

		Ok(())
	}

	pub fn migrate<T: pallet_nft::Config>() -> Weight
	where
		<T as frame_system::Config>::AccountId: From<sp_core::H160>,
	{
		log::info!(target: "Migration", "Nft: updating royalties schedule and collection name");
		pallet_nft::CollectionInfo::<T>::translate(|_, old: OldCollectionInformation<T>| {
			// Bound new name vec, if it is from Ethereum, change the name to "bridged-collection"
			let new_name = match old.origin_chain {
				OriginChain::Root => BoundedVec::truncate_from(old.name),
				OriginChain::Ethereum => BoundedVec::truncate_from(b"bridged-collection".to_vec()),
			};
			let new_royalties = match old.royalties_schedule {
				Some(old_royalties) => Some(RoyaltiesSchedule {
					entitlements: BoundedVec::truncate_from(old_royalties.entitlements),
				}),
				None => None,
			};

			let new = CollectionInformation {
				owner: old.owner,
				name: new_name,
				metadata_scheme: old.metadata_scheme,
				royalties_schedule: new_royalties,
				max_issuance: old.max_issuance,
				origin_chain: old.origin_chain,
				next_serial_number: old.next_serial_number,
				collection_issuance: old.collection_issuance,
				owned_tokens: old.owned_tokens,
				cross_chain_compatibility: old.cross_chain_compatibility,
			};

			Some(new)
		});
		log::info!(target: "Nft", "...Successfully translated CollectionInfo with new name and royalties schedule");

		let key_count = pallet_nft::CollectionInfo::<T>::iter().count();
		<Runtime as frame_system::Config>::DbWeight::get().writes(key_count as u64)
	}
	#[cfg(test)]
	mod tests {
		use super::{OldCollectionInformation, OldRoyaltiesSchedule, *};
		use crate::migrations::tests::new_test_ext;
		use sp_core::H160;

		fn create_account(seed: u64) -> <Runtime as frame_system::Config>::AccountId {
			<Runtime as frame_system::Config>::AccountId::from(H160::from_low_u64_be(seed))
		}

		#[test]
		fn migration_test() {
			new_test_ext().execute_with(|| {
				StorageVersion::new(4).put::<Nft>();

				// Insert collection info
				let user_1 = create_account(5);
				let user_2 = create_account(6);
				let owner = create_account(123);

				let old_info_root = OldCollectionInformation {
					owner: owner.clone(),
					name: b"test-collection-1".to_vec(),
					royalties_schedule: Some(OldRoyaltiesSchedule {
						entitlements: vec![
							(user_1, Permill::one()),
							(user_2, Permill::from_parts(123)),
						],
					}),
					metadata_scheme: MetadataScheme::try_from(b"Test".as_slice()).unwrap(),
					max_issuance: None,
					origin_chain: OriginChain::Root,
					next_serial_number: 0,
					collection_issuance: 100,
					cross_chain_compatibility: CrossChainCompatibility::default(),
					owned_tokens: BoundedVec::default(),
				};
				CollectionInfo::<Runtime>::insert(12, old_info_root);

				let old_info_eth = OldCollectionInformation {
					owner: owner.clone(),
					name: "".encode(),
					royalties_schedule: Some(OldRoyaltiesSchedule {
						entitlements: vec![(user_1, Permill::one())],
					}),
					metadata_scheme: MetadataScheme::try_from(b"Test".as_slice()).unwrap(),
					max_issuance: None,
					origin_chain: OriginChain::Ethereum,
					next_serial_number: 0,
					collection_issuance: 100,
					cross_chain_compatibility: CrossChainCompatibility::default(),
					owned_tokens: BoundedVec::default(),
				};
				CollectionInfo::<Runtime>::insert(13, old_info_eth);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();

				// Check if inserted data is correct
				// Root collection
				let actual_value = pallet_nft::CollectionInfo::<Runtime>::get(12).unwrap();
				let expected_name: BoundedVec<u8, <Runtime as Config>::StringLimit> =
					BoundedVec::truncate_from(b"test-collection-1".to_vec());
				assert_eq!(actual_value.name, expected_name);
				let expected_royalties = Some(RoyaltiesSchedule {
					entitlements: BoundedVec::truncate_from(vec![
						(user_1, Permill::one()),
						(user_2, Permill::from_parts(123)),
					]),
				});
				assert_eq!(actual_value.royalties_schedule, expected_royalties);

				// Eth collection
				let actual_value = pallet_nft::CollectionInfo::<Runtime>::get(13).unwrap();
				let expected_name: BoundedVec<u8, <Runtime as Config>::StringLimit> =
					BoundedVec::truncate_from(b"bridged-collection".to_vec());
				assert_eq!(actual_value.name, expected_name);
				let expected_royalties = Some(RoyaltiesSchedule {
					entitlements: BoundedVec::truncate_from(vec![(user_1, Permill::one())]),
				});
				assert_eq!(actual_value.royalties_schedule, expected_royalties);

				// Check if version has been set correctly
				let onchain = Nft::on_chain_storage_version();
				assert_eq!(onchain, 5);
			});
		}
	}
}
