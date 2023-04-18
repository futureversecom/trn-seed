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
	storage_alias,
	traits::{OnRuntimeUpgrade, StorageVersion},
};

pub struct Upgrade;
impl OnRuntimeUpgrade for Upgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		v4::pre_upgrade()?;

		Ok(())
	}

	fn on_runtime_upgrade() -> Weight {
		let current = Nft::current_storage_version();
		let onchain = Nft::on_chain_storage_version();
		log::info!(target: "Nft", "Running migration with current storage version {current:?} / onchain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads_writes(2, 0);

		if onchain == 3 {
			log::info!(target: "Nft", "Migrating from onchain version 3 to onchain version 4.");
			weight += v4::migrate::<Runtime>();

			log::info!(target: "Nft", "Migration successfully finished.");
			StorageVersion::new(4).put::<Nft>();
		} else {
			log::info!(target: "Nft", "No migration was done. If you are seeing this message, it means that you forgot to remove old existing migration code. Don't panic, it's not a big deal just don't forget it next time :)");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		v4::post_upgrade()?;

		Ok(())
	}
}

#[allow(dead_code)]
pub mod v4 {
	use super::*;
	use codec::{Decode, Encode};
	use core::fmt::Write;
	use frame_support::{BoundedVec, Twox64Concat};
	use pallet_nft::{
		CollectionInformation, CollectionNameType, CrossChainCompatibility, OriginChain,
		RoyaltiesSchedule, TokenOwnership,
	};
	use scale_info::TypeInfo;
	use seed_primitives::{CollectionUuid, MetadataScheme, SerialNumber, TokenCount};
	use sp_core::H160;
	use sp_std::vec::Vec;

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
	pub struct OldCollectionInformation<T: pallet_nft::Config> {
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
		pub owned_tokens:
			BoundedVec<TokenOwnership<T>, <T as pallet_nft::Config>::MaxTokensPerCollection>,
	}

	#[storage_alias]
	type CollectionInfo<T: pallet_nft::Config> = StorageMap<
		pallet_nft::Pallet<T>,
		Twox64Concat,
		CollectionUuid,
		OldCollectionInformation<T>,
	>;

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Nft", "Upgrade to V4 Pre Upgrade.");
		let onchain = Nft::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 4 {
			return Ok(())
		}
		assert_eq!(onchain, 3);

		// Let's make sure that we don't have any corrupted data to begin with
		let keys: Vec<u32> = CollectionInfo::<Runtime>::iter_keys().collect();
		for key in keys {
			assert!(CollectionInfo::<Runtime>::try_get(key).is_ok());
		}

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Nft", "Upgrade to V4 Post Upgrade.");

		let current = Nft::current_storage_version();
		let onchain = Nft::on_chain_storage_version();
		assert_eq!(current, 4);
		assert_eq!(onchain, 4);

		// Let's make sure that we don't have any corrupted data to begin with
		let keys: Vec<u32> = CollectionInfo::<Runtime>::iter_keys().collect();
		for key in keys {
			assert!(CollectionInfo::<Runtime>::try_get(key).is_ok());
		}
		Ok(())
	}

	pub fn migrate<T: pallet_nft::Config>() -> Weight {
		log::info!(target: "Nft", "Translating CollectionInfo...");

		// Iterate all key-value pairs and change them accordingly
		let keys: Vec<u32> = CollectionInfo::<T>::iter_keys().collect();
		for key in keys {
			let old_op = CollectionInfo::<T>::take(key);

			match old_op {
				Some(old) => {
					let add_prefix_and_suffix = |x: Vec<u8>, prefix: &[u8]| -> Vec<u8> {
						let mut res: Vec<u8> = prefix.to_vec();
						res.extend(x);
						if res.last() != Some(&b'/') {
							res.push(b'/');
						}
						res
					};
					let metadata_scheme_res = match old.metadata_scheme {
						OldMetadataScheme::Https(x) => MetadataScheme::try_from(
							add_prefix_and_suffix(x, b"https://").as_slice(),
						),
						OldMetadataScheme::Http(x) => MetadataScheme::try_from(
							add_prefix_and_suffix(x, b"http://").as_slice(),
						),
						OldMetadataScheme::Ipfs(x) => MetadataScheme::try_from(
							add_prefix_and_suffix(x, b"ipfs://").as_slice(),
						),
						OldMetadataScheme::Ethereum(x) => {
							let mut h160_addr = sp_std::Writer::default();
							write!(&mut h160_addr, "{:?}", x).expect("Not written");
							MetadataScheme::try_from(
								add_prefix_and_suffix(h160_addr.inner().clone(), b"ethereum://")
									.as_slice(),
							)
						},
					};

					match metadata_scheme_res {
						Ok(metadata_scheme) => {
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
							// Insert the newly constructed collection
							pallet_nft::CollectionInfo::<T>::insert(key, new);
						},
						// Old data too large. Delete the old dat and skip inserting new collection
						// data
						Err(_) => continue,
					}
				},
				// Old value is none. Delete the old data and skip inserting new collection data
				None => continue,
			}
		}
		log::info!(target: "Nft", "...Successfully translated CollectionInfo");

		let key_count = CollectionInfo::<T>::iter().count();
		<Runtime as frame_system::Config>::DbWeight::get().writes(key_count as u64)
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::new_test_ext;

		use seed_primitives::AccountId;
		use sp_runtime::Permill;
		use sp_std::vec::Vec;

		fn create_account(seed: u64) -> AccountId {
			AccountId::from(H160::from_low_u64_be(seed))
		}

		#[test]
		fn migration_test() {
			new_test_ext().execute_with(|| {
				StorageVersion::new(3).put::<Nft>();
				let user_1 = create_account(5);
				let owner = create_account(123);

				let old_info_ipfs = OldCollectionInformation::<Runtime> {
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

				CollectionInfo::<Runtime>::insert(1, old_info_ipfs);

				let old_info_https = OldCollectionInformation::<Runtime> {
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

				CollectionInfo::<Runtime>::insert(2, old_info_https);

				let old_info_http = OldCollectionInformation::<Runtime> {
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

				CollectionInfo::<Runtime>::insert(3, old_info_http);

				let old_info_ethereum = OldCollectionInformation::<Runtime> {
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

				CollectionInfo::<Runtime>::insert(4, old_info_ethereum);

				// Test if old data length exceeds the limit of BoundedVec in the new MetadataScheme
				// definition
				let old_info_with_too_large_data = OldCollectionInformation::<Runtime> {
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

				CollectionInfo::<Runtime>::insert(5, old_info_with_too_large_data);

				let keys: Vec<u32> = CollectionInfo::<Runtime>::iter_keys().collect();
				for key in keys {
					assert!(CollectionInfo::<Runtime>::try_get(key).is_ok());
				}

				Upgrade::on_runtime_upgrade();

				let expected_value = CollectionInformation::<Runtime> {
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
				let actual_value = pallet_nft::CollectionInfo::<Runtime>::get(1).unwrap();
				assert_eq!(expected_value, actual_value);

				let expected_value = CollectionInformation::<Runtime> {
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
				let actual_value = pallet_nft::CollectionInfo::<Runtime>::get(2).unwrap();
				assert_eq!(expected_value, actual_value);

				let expected_value = CollectionInformation::<Runtime> {
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
				let actual_value = pallet_nft::CollectionInfo::<Runtime>::get(3).unwrap();
				assert_eq!(expected_value, actual_value);

				let expected_value = CollectionInformation::<Runtime> {
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
				let actual_value = pallet_nft::CollectionInfo::<Runtime>::get(4).unwrap();
				assert_eq!(expected_value, actual_value);

				// The old data will be removed if it exceeds the limit of the newly defined
				// MetadataScheme
				let actual_value = pallet_nft::CollectionInfo::<Runtime>::get(5);
				assert!(actual_value.is_none());

				let keys: Vec<u32> = pallet_nft::CollectionInfo::<Runtime>::iter_keys().collect();
				assert_eq!(keys.len(), 4);
				for key in keys {
					assert!(pallet_nft::CollectionInfo::<Runtime>::try_get(key).is_ok());
				}

				let onchain = Nft::on_chain_storage_version();
				assert_eq!(onchain, 4);
			});
		}
	}
}
