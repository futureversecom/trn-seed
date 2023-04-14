#[allow(dead_code)]
pub mod v4 {
	use crate::{
		CollectionInformation, CollectionNameType, Config, CrossChainCompatibility, MetadataScheme,
		OriginChain, Pallet, RoyaltiesSchedule, TokenCount, TokenOwnership,
	};
	use codec::{Decode, Encode};
	use frame_support::{
		storage_alias,
		traits::{Get, GetStorageVersion, StorageVersion},
		weights::{constants::RocksDbWeight as DbWeight, Weight},
		BoundedVec, Twox64Concat,
	};
	use scale_info::TypeInfo;
	use seed_primitives::{CollectionUuid, SerialNumber};
	use sp_core::H160;
	use sp_std::vec::Vec;
	use std::fmt::Write;

	/// Denotes the metadata URI referencing scheme used by a collection
	/// Enable token metadata URI construction by clients
	#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo)]
	pub enum OldMetadataScheme {
		/// Collection metadata is hosted by an HTTPS server
		/// Inner value is the URI without protocol prefix 'https://' or trailing '/'
		/// full metadata URI construction: `https://<domain>/<path+>/<serial_number>`
		/// Https(b"example.com/metadata")
		Https(Vec<u8>),
		/// Collection metadata is hosted by an unsecured HTTP server
		/// Inner value is the URI without protocol prefix 'http://' or trailing '/'
		/// full metadata URI construction: `https://<domain>/<path+>/<serial_number>`
		/// Https(b"example.com/metadata")
		Http(Vec<u8>),
		/// Collection metadata is hosted by an IPFS directory
		/// Inner value is the directory's IPFS CID
		/// full metadata URI construction: `ipfs://<directory_CID>/<serial_number>`
		/// Ipfs(b"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi")
		Ipfs(Vec<u8>),
		// Collection metadata is located on Ethereum in the relevant field on the source token
		// ethereum://<contractaddress>/<originalid>
		Ethereum(H160),
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
		pub metadata_scheme: OldMetadataScheme,
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
		/// This collections compatibility with other chains
		pub cross_chain_compatibility: CrossChainCompatibility,
		/// All serial numbers owned by an account in a collection
		pub owned_tokens: BoundedVec<TokenOwnership<T>, <T as Config>::MaxTokensPerCollection>,
	}

	#[storage_alias]
	type CollectionInfo<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, CollectionUuid, OldCollectionInformation<T>>;

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade<T: Config>() -> Result<(), &'static str> {
		log::info!(target: "Nft", "Upgrade to V4 Pre Upgrade.");
		let onchain = Pallet::<T>::on_chain_storage_version();
		assert_eq!(onchain, 3);

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

		if onchain == 3 {
			log::info!(target: "Nft", "Migrating from onchain version 3 to onchain version 4.");
			weight += migrate::<T>();

			log::info!(target: "Nft", "Migration successfully finished.");
			StorageVersion::new(4).put::<Pallet<T>>();
		} else {
			log::info!(target: "Nft", "No migration was done. If you are seeing this message, it means that you forgot to remove old existing migration code. Don't panic, it's not a big deal just don't forget it next time :)");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade<T: Config>() -> Result<(), &'static str> {
		log::info!(target: "Nft", "Upgrade to V4 Post Upgrade.");

		let current = Pallet::<T>::current_storage_version();
		let onchain = Pallet::<T>::on_chain_storage_version();
		assert_eq!(current, 4);
		assert_eq!(onchain, 4);

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
			let add_prefix_and_suffix = |x: Vec<u8>, prefix: &[u8]| -> Vec<u8> {
				let mut res: Vec<u8> = prefix.to_vec();
				res.extend(x);
				if res.last() != Some(&b'/') {
					res.push(b'/');
				}
				res
			};

			let metadata_scheme = match old.metadata_scheme {
				OldMetadataScheme::Https(x) =>
					MetadataScheme::try_from(add_prefix_and_suffix(x, b"https://").as_slice())
						.ok()?,
				OldMetadataScheme::Http(x) =>
					MetadataScheme::try_from(add_prefix_and_suffix(x, b"http://").as_slice())
						.ok()?,
				OldMetadataScheme::Ipfs(x) =>
					MetadataScheme::try_from(add_prefix_and_suffix(x, b"ipfs://").as_slice())
						.ok()?,
				OldMetadataScheme::Ethereum(x) => {
					let mut h160_addr = sp_std::Writer::default();
					write!(&mut h160_addr, "{:?}", x).expect("Not written");
					MetadataScheme::try_from(
						add_prefix_and_suffix(h160_addr.inner().clone(), b"ethereum://").as_slice(),
					)
					.ok()?
				},
			};

			let new = CollectionInformation {
				owner: old.owner,
				name: old.name,
				metadata_scheme,
				royalties_schedule: old.royalties_schedule,
				max_issuance: old.max_issuance,
				origin_chain: old.origin_chain,
				next_serial_number: old.next_serial_number,
				collection_issuance: old.collection_issuance,
				cross_chain_compatibility: old.cross_chain_compatibility,
				owned_tokens: old.owned_tokens,
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
				StorageVersion::new(3).put::<Pallet<Test>>();
				let user_1 = create_account(5);
				let owner = create_account(123);

				let old_info_ipfs = OldCollectionInformation::<Test> {
					owner: owner.clone(),
					name: b"test-collection-1".to_vec(),
					royalties_schedule: Some(RoyaltiesSchedule {
						entitlements: vec![(user_1, Permill::one())],
					}),
					metadata_scheme: OldMetadataScheme::Ipfs(b"Test1/".to_vec()),
					max_issuance: None,
					origin_chain: OriginChain::Root,
					next_serial_number: 0,
					collection_issuance: 100,
					cross_chain_compatibility: CrossChainCompatibility::default(),
					owned_tokens: BoundedVec::default(),
				};

				CollectionInfo::<Test>::insert(1, old_info_ipfs);

				let old_info_https = OldCollectionInformation::<Test> {
					owner: owner.clone(),
					name: b"test-collection-2".to_vec(),
					royalties_schedule: Some(RoyaltiesSchedule {
						entitlements: vec![(user_1, Permill::one())],
					}),
					metadata_scheme: OldMetadataScheme::Https(b"google.com".to_vec()),
					max_issuance: None,
					origin_chain: OriginChain::Root,
					next_serial_number: 0,
					collection_issuance: 100,
					cross_chain_compatibility: CrossChainCompatibility::default(),
					owned_tokens: BoundedVec::default(),
				};

				CollectionInfo::<Test>::insert(2, old_info_https);

				let old_info_http = OldCollectionInformation::<Test> {
					owner: owner.clone(),
					name: b"test-collection-3".to_vec(),
					royalties_schedule: Some(RoyaltiesSchedule {
						entitlements: vec![(user_1, Permill::one())],
					}),
					metadata_scheme: OldMetadataScheme::Http(b"google.com".to_vec()),
					max_issuance: None,
					origin_chain: OriginChain::Root,
					next_serial_number: 0,
					collection_issuance: 100,
					cross_chain_compatibility: CrossChainCompatibility::default(),
					owned_tokens: BoundedVec::default(),
				};

				CollectionInfo::<Test>::insert(3, old_info_http);

				let old_info_ethereum = OldCollectionInformation::<Test> {
					owner: owner.clone(),
					name: b"test-collection-4".to_vec(),
					royalties_schedule: Some(RoyaltiesSchedule {
						entitlements: vec![(user_1, Permill::one())],
					}),
					metadata_scheme: OldMetadataScheme::Ethereum(H160::from_slice(
						&hex::decode("E04CC55ebEE1cBCE552f250e85c57B70B2E2625b").unwrap(),
					)),
					max_issuance: None,
					origin_chain: OriginChain::Root,
					next_serial_number: 0,
					collection_issuance: 100,
					cross_chain_compatibility: CrossChainCompatibility::default(),
					owned_tokens: BoundedVec::default(),
				};

				CollectionInfo::<Test>::insert(4, old_info_ethereum);

				// Test if old data length exceeds the limit of BoundedVec in the new MetadataScheme
				// definition
				let old_info_with_too_large_data = OldCollectionInformation::<Test> {
					owner: owner.clone(),
					name: b"test-collection-5".to_vec(),
					royalties_schedule: Some(RoyaltiesSchedule {
						entitlements: vec![(user_1, Permill::one())],
					}),
					metadata_scheme: OldMetadataScheme::Https(vec![0; 200]),
					max_issuance: None,
					origin_chain: OriginChain::Root,
					next_serial_number: 0,
					collection_issuance: 100,
					cross_chain_compatibility: CrossChainCompatibility::default(),
					owned_tokens: BoundedVec::default(),
				};

				CollectionInfo::<Test>::insert(5, old_info_with_too_large_data);

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
					metadata_scheme: MetadataScheme::try_from(b"ipfs://Test1/".as_slice()).unwrap(),
					max_issuance: None,
					origin_chain: OriginChain::Root,
					next_serial_number: 0,
					collection_issuance: 100,
					cross_chain_compatibility: CrossChainCompatibility::default(),
					owned_tokens: BoundedVec::default(),
				};
				let actual_value = crate::CollectionInfo::<Test>::get(1).unwrap();
				assert_eq!(expected_value, actual_value);

				let expected_value = CollectionInformation::<Test> {
					owner,
					name: b"test-collection-2".to_vec(),
					royalties_schedule: Some(RoyaltiesSchedule {
						entitlements: vec![(user_1, Permill::one())],
					}),
					metadata_scheme: MetadataScheme::try_from(b"https://google.com/".as_slice())
						.unwrap(),
					max_issuance: None,
					origin_chain: OriginChain::Root,
					next_serial_number: 0,
					collection_issuance: 100,
					cross_chain_compatibility: CrossChainCompatibility::default(),
					owned_tokens: BoundedVec::default(),
				};
				let actual_value = crate::CollectionInfo::<Test>::get(2).unwrap();
				assert_eq!(expected_value, actual_value);

				let expected_value = CollectionInformation::<Test> {
					owner,
					name: b"test-collection-3".to_vec(),
					royalties_schedule: Some(RoyaltiesSchedule {
						entitlements: vec![(user_1, Permill::one())],
					}),
					metadata_scheme: MetadataScheme::try_from(b"http://google.com/".as_slice())
						.unwrap(),
					max_issuance: None,
					origin_chain: OriginChain::Root,
					next_serial_number: 0,
					collection_issuance: 100,
					cross_chain_compatibility: CrossChainCompatibility::default(),
					owned_tokens: BoundedVec::default(),
				};
				let actual_value = crate::CollectionInfo::<Test>::get(3).unwrap();
				assert_eq!(expected_value, actual_value);

				let expected_value = CollectionInformation::<Test> {
					owner,
					name: b"test-collection-4".to_vec(),
					royalties_schedule: Some(RoyaltiesSchedule {
						entitlements: vec![(user_1, Permill::one())],
					}),
					metadata_scheme: MetadataScheme::try_from(
						b"ethereum://0xe04cc55ebee1cbce552f250e85c57b70b2e2625b/".as_slice(),
					)
					.unwrap(),
					max_issuance: None,
					origin_chain: OriginChain::Root,
					next_serial_number: 0,
					collection_issuance: 100,
					cross_chain_compatibility: CrossChainCompatibility::default(),
					owned_tokens: BoundedVec::default(),
				};
				let actual_value = crate::CollectionInfo::<Test>::get(4).unwrap();
				assert_eq!(expected_value, actual_value);

				// The old data will be removed if it exceeds the limit of the newly defined
				// MetadataScheme
				let actual_value = crate::CollectionInfo::<Test>::get(5);
				assert!(actual_value.is_none());

				let keys: Vec<u32> = crate::CollectionInfo::<Test>::iter_keys().collect();
				for key in keys {
					assert!(crate::CollectionInfo::<Test>::try_get(key).is_ok());
				}

				let onchain = Pallet::<Test>::on_chain_storage_version();
				assert_eq!(onchain, 4);
			});
		}
	}
}
