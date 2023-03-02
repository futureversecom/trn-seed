#[allow(dead_code)]
pub mod v2 {
	use crate::{
		CollectionInformation, CollectionNameType, Config, MetadataScheme, OriginChain, Pallet,
		RoyaltiesSchedule, TokenCount, TokenOwnership,
	};
	use codec::{Decode, Encode};
	use frame_support::{
		storage_alias,
		traits::{GetStorageVersion, StorageVersion},
		weights::{constants::RocksDbWeight as DbWeight, Weight},
		BoundedVec, Twox64Concat,
	};
	use scale_info::TypeInfo;
	use seed_primitives::{CollectionUuid, SerialNumber};
	use sp_core::H160;
	use sp_std::{vec, vec::Vec};

	/// Denotes the metadata URI referencing scheme used by a collection
	/// Enable token metadata URI construction by clients
	#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo)]
	pub enum OldMetadataScheme {
		/// Collection metadata is hosted by an HTTPS server
		/// Inner value is the URI without protocol prefix 'https://' or trailing '/'
		/// full metadata URI construction: `https://<domain>/<path+>/<serial_number>.json`
		/// Https(b"example.com/metadata")
		Https(Vec<u8>),
		/// Collection metadata is hosted by an unsecured HTTP server
		/// Inner value is the URI without protocol prefix 'http://' or trailing '/'
		/// full metadata URI construction: `https://<domain>/<path+>/<serial_number>.json`
		/// Https(b"example.com/metadata")
		Http(Vec<u8>),
		/// Collection metadata is hosted by an IPFS directory
		/// Inner value is the directory's IPFS CID
		/// full metadata URI construction: `ipfs://<directory_CID>/<serial_number>.json`
		/// IpfsDir(b"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi")
		IpfsDir(Vec<u8>),
		/// Collection metadata is hosted by an IPFS directory
		/// Inner value is the shared IPFS CID, each token in the collection shares the same CID
		/// full metadata URI construction: `ipfs://<shared_file_CID>.json`
		IpfsShared(Vec<u8>),
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

		assert_eq!(onchain, 1);
		Ok(())
	}

	pub fn on_runtime_upgrade<T: Config>() -> Weight {
		let current = Pallet::<T>::current_storage_version();
		let onchain = Pallet::<T>::on_chain_storage_version();
		log::info!(target: "Nft", "Running migration with current storage version {current:?} / onchain {onchain:?}");

		let mut weight = DbWeight::get().reads_writes(2, 0);

		if onchain == 1 {
			log::info!(target: "Nft", "Migrating from onchain version 0 to onchain version 2.");
			weight += migrate::<T>();

			log::info!(target: "Nft", "Migration successfully finished.");
			StorageVersion::new(2).put::<Pallet<T>>();
		} else {
			log::info!(target: "Nft", "No migration was done. If you are seeing this message, it means that you forgot to remove old existing migration code. Don't panic, it's not a big deal just don't forget it next time :)");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade<T: Config>() -> Result<(), &'static str> {
		log::info!(target: "Nft", "Upgrade to V2 Post Upgrade.");
		let onchain = Pallet::<T>::on_chain_storage_version();

		assert_eq!(onchain, 2);
		Ok(())
	}

	pub fn migrate<T: Config>() -> Weight {
		crate::CollectionInfo::<T>::translate(|_, old: OldCollectionInformation<T>| {
			let add_slash = |mut x: Vec<u8>| -> Vec<u8> {
				if x.last() != Some(&b'/') {
					x.push(b'/');
				}

				x
			};

			let metadata_scheme = match old.metadata_scheme {
				OldMetadataScheme::Https(x) => MetadataScheme::Https(x),
				OldMetadataScheme::Http(x) => MetadataScheme::Http(x),
				OldMetadataScheme::IpfsDir(x) => MetadataScheme::Ipfs(add_slash(x)),
				OldMetadataScheme::IpfsShared(x) => MetadataScheme::Ipfs(add_slash(x)),
				OldMetadataScheme::Ethereum(x) => MetadataScheme::Ethereum(x),
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
				owned_tokens: old.owned_tokens,
			};

			Some(new)
		});

		Weight::from(22u32)
	}

	#[cfg(test)]
	mod tests {
		use sp_runtime::Permill;

		use super::*;
		use crate::mock::{Test, TestExt};

		#[test]
		fn migration_test() {
			TestExt::default().build().execute_with(|| {
				StorageVersion::new(1).put::<Pallet<Test>>();
				let user_1 = 5_u64;

				let old_info = OldCollectionInformation::<Test> {
					owner: 123_u64,
					name: b"test-collection-1".to_vec(),
					royalties_schedule: Some(RoyaltiesSchedule {
						entitlements: vec![(user_1, Permill::one())],
					}),
					metadata_scheme: OldMetadataScheme::IpfsDir(b"Test1/".to_vec()),
					max_issuance: None,
					origin_chain: OriginChain::Root,
					next_serial_number: 0,
					collection_issuance: 100,
					owned_tokens: BoundedVec::default(),
				};

				// Collection 1
				let collection_id_1 = 678;
				CollectionInfo::<Test>::insert(collection_id_1, old_info.clone());

				let collection_id_2 = 679;
				let info = OldCollectionInformation::<Test> {
					metadata_scheme: OldMetadataScheme::IpfsShared(b"Test1/".to_vec()),
					..old_info.clone()
				};
				CollectionInfo::<Test>::insert(collection_id_2, info);

				let collection_id_3 = 680;
				let info = OldCollectionInformation::<Test> {
					metadata_scheme: OldMetadataScheme::IpfsDir(b"Test1".to_vec()),
					..old_info.clone()
				};
				CollectionInfo::<Test>::insert(collection_id_3, info);

				let collection_id_4 = 681;
				let info = OldCollectionInformation::<Test> {
					metadata_scheme: OldMetadataScheme::Https(b"Test1/".to_vec()),
					..old_info.clone()
				};
				CollectionInfo::<Test>::insert(collection_id_4, info);

				on_runtime_upgrade::<Test>();

				let expected_value = CollectionInformation::<Test> {
					owner: 123_u64,
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

				// This checks IpfsDir -> Ipfs
				let actual_value = crate::CollectionInfo::<Test>::get(collection_id_1).unwrap();
				assert_eq!(expected_value, actual_value);

				// This checks IpfsShared -> Ipfs
				let actual_value = crate::CollectionInfo::<Test>::get(collection_id_2).unwrap();
				assert_eq!(expected_value, actual_value);

				// This checks that a forward slash is added at the end
				let actual_value = crate::CollectionInfo::<Test>::get(collection_id_3).unwrap();
				assert_eq!(expected_value, actual_value);

				// this checks that other enum variants are not messed with
				let metadata = MetadataScheme::Https(b"Test1/".to_vec());
				let expected_value =
					CollectionInformation::<Test> { metadata_scheme: metadata, ..expected_value };
				let actual_value = crate::CollectionInfo::<Test>::get(collection_id_4).unwrap();
				assert_eq!(expected_value, actual_value);

				assert_eq!(StorageVersion::get::<Pallet<Test>>(), 2);
			});
		}
	}
}
