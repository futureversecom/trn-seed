#[allow(dead_code)]
pub mod v3 {
	use crate::{
		CollectionInformation, CollectionNameType, Config, CrossChainCompatibility, OriginChain,
		Pallet, RoyaltiesSchedule, TokenOwnership,
	};
	use codec::{Decode, Encode};
	use frame_support::{
		storage_alias,
		traits::{Get, GetStorageVersion, StorageVersion},
		weights::{constants::RocksDbWeight as DbWeight, Weight},
		BoundedVec, Twox64Concat,
	};
	use scale_info::TypeInfo;
	use seed_primitives::{CollectionUuid, MetadataScheme, SerialNumber, TokenCount};

	#[cfg(feature = "try-runtime")]
	use sp_std::vec::Vec;

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
		pub royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
		/// Maximum number of tokens allowed in a collection
		pub max_issuance: Option<TokenCount>,
		/// The chain in which the collection was minted originally
		pub origin_chain: OriginChain,
		/// The next available serial_number
		pub next_serial_number: SerialNumber,
		/// the total count of tokens in this collection
		pub collection_issuance: TokenCount,
		/// All serial numbers owned by an account in a collection
		pub owned_tokens: BoundedVec<TokenOwnership<T>, <T as Config>::MaxTokensPerCollection>,
	}

	#[storage_alias]
	type CollectionInfo<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, CollectionUuid, OldCollectionInformation<T>>;

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade<T: Config>() -> Result<(), &'static str> {
		log::info!(target: "Nft", "Upgrade to V2 Pre Upgrade.");

		let onchain = Pallet::<T>::on_chain_storage_version();
		assert_eq!(onchain, 2);

		// Let's make sure that we don't have any corrupted data to begin with
		let keys: Vec<u32> = CollectionInfo::<T>::iter_keys().collect();
		for key in keys {
			assert!(CollectionInfo::<T>::try_get(key).is_ok());
		}

		Ok(())
	}

	pub fn on_runtime_upgrade<T: Config>() -> Weight {
		let current = Pallet::<T>::current_storage_version();
		let onchain = Pallet::<T>::on_chain_storage_version();
		log::info!(target: "Nft", "Running migration with current storage version {current:?} / onchain {onchain:?}");

		let mut weight = DbWeight::get().reads_writes(2, 0);

		if onchain == 2 {
			log::info!(target: "Nft", "Migrating from onchain version 2 to onchain version 3.");
			weight += migrate::<T>();

			log::info!(target: "Nft", "Migration successfully finished.");
			StorageVersion::new(3).put::<Pallet<T>>();
		} else {
			log::info!(target: "Nft", "No migration was done. If you are seeing this message, it means that you forgot to remove old existing migration code. Don't panic, it's not a big deal just don't forget it next time :)");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade<T: Config>() -> Result<(), &'static str> {
		log::info!(target: "Nft", "Upgrade to V3 Post Upgrade.");

		let current = Pallet::<T>::current_storage_version();
		let onchain = Pallet::<T>::on_chain_storage_version();
		assert_eq!(current, 3);
		assert_eq!(onchain, 3);

		// Let's make sure that we don't have any corrupted data to begin with
		let keys: Vec<u32> = crate::CollectionInfo::<T>::iter_keys().collect();
		for key in keys {
			assert!(crate::CollectionInfo::<T>::try_get(key).is_ok());
		}
		Ok(())
	}

	pub fn migrate<T: Config>() -> Weight {
		log::info!(target: "Nft", "Translating CollectionInfo...");
		crate::CollectionInfo::<T>::translate(|_, old: OldCollectionInformation<T>| {
			let cross_chain_compatibility = CrossChainCompatibility { xrpl: false };

			let new = CollectionInformation {
				owner: old.owner,
				name: old.name,
				metadata_scheme: old.metadata_scheme,
				royalties_schedule: old.royalties_schedule,
				max_issuance: old.max_issuance,
				origin_chain: old.origin_chain,
				next_serial_number: old.next_serial_number,
				collection_issuance: old.collection_issuance,
				owned_tokens: old.owned_tokens,
				cross_chain_compatibility,
			};

			Some(new)
		});
		log::info!(target: "Nft", "...Successfully translated CollectionInfo");

		let key_count = crate::CollectionInfo::<T>::iter().count();
		<T as frame_system::Config>::DbWeight::get().writes(key_count as u64)
	}

	#[cfg(test)]
	mod tests {
		use sp_runtime::Permill;

		use super::*;
		use crate::mock::{create_account, Test, TestExt};
		use sp_std::vec::Vec;

		#[test]
		fn migration_test() {
			TestExt::default().build().execute_with(|| {
				StorageVersion::new(2).put::<Pallet<Test>>();
				let user_1 = create_account(5);
				let owner = create_account(123);

				let old_info = OldCollectionInformation::<Test> {
					owner: owner.clone(),
					name: b"test-collection-1".to_vec(),
					royalties_schedule: Some(RoyaltiesSchedule {
						entitlements: vec![(user_1, Permill::one())],
					}),
					metadata_scheme: MetadataScheme::Ipfs(b"Test1/".to_vec()),
					max_issuance: None,
					origin_chain: OriginChain::Root,
					next_serial_number: 0,
					collection_issuance: 100,
					owned_tokens: BoundedVec::default(),
				};

				let collection_id = 678;
				CollectionInfo::<Test>::insert(collection_id, old_info.clone());

				let keys: Vec<u32> = CollectionInfo::<Test>::iter_keys().collect();
				for key in keys {
					assert!(CollectionInfo::<Test>::try_get(key).is_ok());
				}

				on_runtime_upgrade::<Test>();

				let expected_value = CollectionInformation::<Test> {
					owner,
					name: b"test-collection-1".to_vec(),
					royalties_schedule: Some(RoyaltiesSchedule {
						entitlements: vec![(user_1, Permill::one())],
					}),
					metadata_scheme: MetadataScheme::Ipfs(b"Test1/".to_vec()),
					max_issuance: None,
					origin_chain: OriginChain::Root,
					next_serial_number: 0,
					collection_issuance: 100,
					owned_tokens: BoundedVec::default(),
					cross_chain_compatibility: CrossChainCompatibility { xrpl: false },
				};
				let actual_value = crate::CollectionInfo::<Test>::get(collection_id).unwrap();
				assert_eq!(expected_value, actual_value);

				let keys: Vec<u32> = crate::CollectionInfo::<Test>::iter_keys().collect();
				for key in keys {
					assert!(crate::CollectionInfo::<Test>::try_get(key).is_ok());
				}

				let onchain = Pallet::<Test>::on_chain_storage_version();
				assert_eq!(onchain, 3);
			});
		}
	}
}
