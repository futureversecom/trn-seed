use crate::*;
use codec::{Decode, Encode, MaxEncodedLen};
use core::fmt::Debug;
use frame_support::{dispatch::GetStorageVersion, traits::StorageVersion, DefaultNoBound};
use frame_support::{
    storage_alias, CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound, Twox64Concat,
};
use pallet_migration::WeightInfo;
use pallet_nft::{CollectionInformation, TokenOwnership};
use scale_info::TypeInfo;
use seed_primitives::migration::{MigrationStep, MigrationStepResult};
use seed_primitives::{
    CollectionUuid, CrossChainCompatibility, MetadataScheme, OriginChain, RoyaltiesSchedule,
    SerialNumber, TokenCount,
};
use sp_runtime::BoundedVec;

use sp_std::marker::PhantomData;

#[allow(dead_code)]
pub(crate) const LOG_TARGET: &str = "migration";

mod old {
    use super::*;

    #[storage_alias]
    pub type CollectionInfo<T: pallet_nft::Config> = StorageMap<
        pallet_nft::Pallet<T>,
        Twox64Concat,
        CollectionUuid,
        CollectionInformation<
            <T as frame_system::Config>::AccountId,
            <T as pallet_nft::Config>::MaxTokensPerCollection,
            <T as pallet_nft::Config>::StringLimit,
        >,
    >;

    /// Information related to a specific collection
    #[derive(
        PartialEqNoBound,
        RuntimeDebugNoBound,
        CloneNoBound,
        Encode,
        Decode,
        TypeInfo,
        MaxEncodedLen,
    )]
    #[codec(mel_bound(AccountId: MaxEncodedLen))]
    #[scale_info(skip_type_params(MaxTokensPerCollection, StringLimit))]
    pub struct CollectionInformation<AccountId, MaxTokensPerCollection, StringLimit>
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
            old::TokenOwnership<AccountId, MaxTokensPerCollection>,
            MaxTokensPerCollection,
        >,
    }

    #[derive(
        PartialEqNoBound,
        RuntimeDebugNoBound,
        Decode,
        Encode,
        CloneNoBound,
        TypeInfo,
        MaxEncodedLen,
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
}

/// Convert from old CollectionInfo type to new type
fn convert<T: pallet_nft::Config>(
    old: old::CollectionInformation<T::AccountId, T::MaxTokensPerCollection, T::StringLimit>,
) -> Result<
    (
        CollectionInformation<T::AccountId, T::StringLimit>,
        TokenOwnership<T::AccountId, T::MaxTokensPerCollection>,
    ),
    &'static str,
> {
    // Construct ownership info out of old ownership info
    let token_ownership_old = old.owned_tokens;
    let mut token_ownership_new = TokenOwnership::default();
    token_ownership_old.iter().for_each(|ownership| {
        token_ownership_new
            .owned_tokens
            .force_push((ownership.owner.clone(), ownership.owned_serials.clone()));
    });
    let new_collection_info = CollectionInformation {
        owner: old.owner,
        name: old.name,
        metadata_scheme: old.metadata_scheme,
        royalties_schedule: old.royalties_schedule,
        max_issuance: old.max_issuance,
        origin_chain: old.origin_chain,
        next_serial_number: old.next_serial_number,
        collection_issuance: old.collection_issuance,
        cross_chain_compatibility: old.cross_chain_compatibility,
    };
    Ok((new_collection_info, token_ownership_new))
}

#[derive(Encode, Decode, MaxEncodedLen, DefaultNoBound)]
pub struct NftMigration<T: pallet_nft::Config> {
    phantom: PhantomData<T>,
}

impl<T: pallet_nft::Config + pallet_migration::Config> MigrationStep for NftMigration<T> {
    const TARGET_VERSION: u16 = 8;

    fn version_check() -> bool {
        Nft::on_chain_storage_version() == Self::TARGET_VERSION
    }

    fn on_complete() {
        StorageVersion::new(Self::TARGET_VERSION).put::<Nft>();
    }

    fn max_step_weight() -> Weight {
        <T as pallet_migration::Config>::WeightInfo::current_migration_step()
    }

    /// Migrate one token
    fn step(last_key: Option<Vec<u8>>) -> MigrationStepResult {
        let mut iter = if let Some(last_key) = last_key {
            old::CollectionInfo::<T>::iter_from(last_key)
        } else {
            old::CollectionInfo::<T>::iter()
        };

        if let Some((key, old)) = iter.next() {
            match convert::<T>(old) {
                Ok((collection_info, token_ownership)) => {
                    pallet_nft::CollectionInfo::<T>::insert(key, collection_info);
                    pallet_nft::OwnershipInfo::<T>::insert(key, token_ownership);
                },
                Err(e) => {
                    // If we encounter an error during the conversion, we must insert some
                    // default value, we can't remove the token as it will cause unexpected results
                    // with the iter process
                    log::error!(target: LOG_TARGET, "🦆 Error migrating collection_id {:?} : {:?}", key, e);
                    // pallet_nft::CollectionInfo::<T>::insert(key, CollectionInfo::default());
                },
            }
            let last_key = old::CollectionInfo::<T>::hashed_key_for(key);
            MigrationStepResult::continue_step(Self::max_step_weight(), last_key)
        } else {
            log::debug!(target: LOG_TARGET, "🦆 No more tokens to migrate");
            MigrationStepResult::finish_step(Self::max_step_weight())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::{tests::new_test_ext, Map};
    use frame_support::{StorageHasher, Twox64Concat};
    use pallet_nft::OwnedTokens;
    use seed_pallet_common::test_prelude::create_account;

    type AccountId = <Runtime as frame_system::Config>::AccountId;

    /// Helper function to manually insert fake data into storage map
    fn insert_old_data(
        collection_id: CollectionUuid,
        old_value: old::CollectionInformation<
            AccountId,
            MaxTokensPerCollection,
            CollectionNameStringLimit,
        >,
    ) {
        let key = Twox64Concat::hash(&(collection_id).encode());
        Map::unsafe_storage_put::<
            old::CollectionInformation<
                AccountId,
                MaxTokensPerCollection,
                CollectionNameStringLimit,
            >,
        >(b"Nft", b"CollectionInfo", &key, old_value);
    }

    #[test]
    fn convert_works() {
        new_test_ext().execute_with(|| {
            let old_token_ownership = old::TokenOwnership::<AccountId, MaxTokensPerCollection> {
                owner: create_account(123),
                owned_serials: BoundedVec::<SerialNumber, MaxTokensPerCollection>::truncate_from(
                    vec![1, 2, 3],
                ),
            };
            let old = old::CollectionInformation {
                owner: create_account(123),
                name: BoundedVec::<u8, CollectionNameStringLimit>::truncate_from(vec![1, 2, 3]),
                metadata_scheme: MetadataScheme::try_from(b"metadata".as_slice()).unwrap(),
                royalties_schedule: None,
                max_issuance: Some(100),
                origin_chain: OriginChain::Root,
                next_serial_number: 1,
                collection_issuance: 1,
                cross_chain_compatibility: CrossChainCompatibility::default(),
                owned_tokens: BoundedVec::<
                    old::TokenOwnership<AccountId, MaxTokensPerCollection>,
                    MaxTokensPerCollection,
                >::truncate_from(vec![old_token_ownership]),
            };
            let (new_collection_info, new_token_ownership) = convert::<Runtime>(old).unwrap();
            let expected_collection_info = CollectionInformation {
                owner: create_account(123),
                name: BoundedVec::<u8, CollectionNameStringLimit>::truncate_from(vec![1, 2, 3]),
                metadata_scheme: MetadataScheme::try_from(b"metadata".as_slice()).unwrap(),
                royalties_schedule: None,
                max_issuance: Some(100),
                origin_chain: OriginChain::Root,
                next_serial_number: 1,
                collection_issuance: 1,
                cross_chain_compatibility: CrossChainCompatibility::default(),
            };
            let expected_token_ownership = TokenOwnership {
                owned_tokens: BoundedVec::<
                    OwnedTokens<AccountId, MaxTokensPerCollection>,
                    MaxTokensPerCollection,
                >::truncate_from(vec![(
                    create_account(123),
                    BoundedVec::<SerialNumber, MaxTokensPerCollection>::truncate_from(vec![
                        1, 2, 3,
                    ]),
                )]),
            };

            assert_eq!(new_collection_info, expected_collection_info);
            assert_eq!(new_token_ownership, expected_token_ownership);
        });
    }

    #[test]
    fn migrate_single_step() {
        new_test_ext().execute_with(|| {
            let old_token_ownership_1 = old::TokenOwnership::<AccountId, MaxTokensPerCollection> {
                owner: create_account(123),
                owned_serials: BoundedVec::<SerialNumber, MaxTokensPerCollection>::truncate_from(
                    vec![1, 2, 3, 5],
                ),
            };
            let old_token_ownership_2 = old::TokenOwnership::<AccountId, MaxTokensPerCollection> {
                owner: create_account(126),
                owned_serials: BoundedVec::<SerialNumber, MaxTokensPerCollection>::truncate_from(
                    vec![6, 7, 8, 9],
                ),
            };
            let old = old::CollectionInformation {
                owner: create_account(126),
                name: BoundedVec::<u8, CollectionNameStringLimit>::truncate_from(vec![1, 2, 3, 4]),
                metadata_scheme: MetadataScheme::try_from(b"metadata".as_slice()).unwrap(),
                royalties_schedule: None,
                max_issuance: Some(500),
                origin_chain: OriginChain::Root,
                next_serial_number: 2,
                collection_issuance: 5,
                cross_chain_compatibility: CrossChainCompatibility::default(),
                owned_tokens: BoundedVec::<
                    old::TokenOwnership<AccountId, MaxTokensPerCollection>,
                    MaxTokensPerCollection,
                >::truncate_from(vec![old_token_ownership_1, old_token_ownership_2]),
            };
            let collection_id = 123;
            insert_old_data(collection_id, old);

            let result = NftMigration::<Runtime>::step(None);
            assert!(!result.is_finished());
            let expected_collection_info = CollectionInformation {
                owner: create_account(126),
                name: BoundedVec::<u8, CollectionNameStringLimit>::truncate_from(vec![1, 2, 3, 4]),
                metadata_scheme: MetadataScheme::try_from(b"metadata".as_slice()).unwrap(),
                royalties_schedule: None,
                max_issuance: Some(500),
                origin_chain: OriginChain::Root,
                next_serial_number: 2,
                collection_issuance: 5,
                cross_chain_compatibility: CrossChainCompatibility::default(),
            };
            let expected_token_ownership = TokenOwnership {
                owned_tokens: BoundedVec::<
                    OwnedTokens<AccountId, MaxTokensPerCollection>,
                    MaxTokensPerCollection,
                >::truncate_from(vec![
                    (
                        create_account(123),
                        BoundedVec::<SerialNumber, MaxTokensPerCollection>::truncate_from(vec![
                            1, 2, 3, 5,
                        ]),
                    ),
                    (
                        create_account(126),
                        BoundedVec::<SerialNumber, MaxTokensPerCollection>::truncate_from(vec![
                            6, 7, 8, 9,
                        ]),
                    ),
                ]),
            };
            let new_collection_info =
                pallet_nft::CollectionInfo::<Runtime>::get(collection_id).unwrap();
            assert_eq!(new_collection_info, expected_collection_info);

            let new_token_ownership =
                pallet_nft::OwnershipInfo::<Runtime>::get(collection_id).unwrap();
            assert_eq!(new_token_ownership, expected_token_ownership);

            // Attempting to perform one more step should return Finished
            let last_key = result.last_key;
            let result = NftMigration::<Runtime>::step(last_key.clone());
            assert!(result.is_finished());
        });
    }

    #[test]
    fn migrate_many_steps() {
        new_test_ext().execute_with(|| {
            // Insert 100 collections
            let collection_count: CollectionUuid = 100;
            for i in 0..collection_count {
                let old_token_ownership = old::TokenOwnership::<AccountId, MaxTokensPerCollection> {
                    owner: create_account(1 + i as u64),
                    owned_serials:
                    BoundedVec::<SerialNumber, MaxTokensPerCollection>::truncate_from(vec![
                        1, 2, 3, i,
                    ]),
                };
                let old = old::CollectionInformation {
                    owner: create_account(2 + i as u64),
                    name: BoundedVec::<u8, CollectionNameStringLimit>::truncate_from(vec![
                        1, 2, 3, 4,
                    ]),
                    metadata_scheme: MetadataScheme::try_from(b"metadata".as_slice()).unwrap(),
                    royalties_schedule: None,
                    max_issuance: Some(i),
                    origin_chain: OriginChain::Root,
                    next_serial_number: i + 4,
                    collection_issuance: i + 5,
                    cross_chain_compatibility: CrossChainCompatibility::default(),
                    owned_tokens: BoundedVec::<
                        old::TokenOwnership<AccountId, MaxTokensPerCollection>,
                        MaxTokensPerCollection,
                    >::truncate_from(vec![old_token_ownership]),
                };
                insert_old_data(i, old);
            }

            // Perform migration
            let mut last_key = None;
            for _ in 0..collection_count {
                let result = NftMigration::<Runtime>::step(last_key.clone());
                assert!(!result.is_finished());
                last_key = result.last_key;
            }
            // One last step to finish migration
            let result = NftMigration::<Runtime>::step(last_key.clone());
            assert!(result.is_finished());

            // Check that all collections have been migrated
            for i in 0..collection_count {
                let new_collection_info = pallet_nft::CollectionInfo::<Runtime>::get(i).unwrap();
                let new_token_ownership = pallet_nft::OwnershipInfo::<Runtime>::get(i).unwrap();
                let expected_token_ownership = TokenOwnership {
                    owned_tokens: BoundedVec::<
                        OwnedTokens<AccountId, MaxTokensPerCollection>,
                        MaxTokensPerCollection,
                    >::truncate_from(vec![(
                        create_account(1 + i as u64),
                        BoundedVec::<SerialNumber, MaxTokensPerCollection>::truncate_from(vec![
                            1, 2, 3, i,
                        ]),
                    )]),
                };
                let expected_collection_info = CollectionInformation {
                    owner: create_account(2 + i as u64),
                    name: BoundedVec::<u8, CollectionNameStringLimit>::truncate_from(vec![
                        1, 2, 3, 4,
                    ]),
                    metadata_scheme: MetadataScheme::try_from(b"metadata".as_slice()).unwrap(),
                    royalties_schedule: None,
                    max_issuance: Some(i),
                    origin_chain: OriginChain::Root,
                    next_serial_number: i + 4,
                    collection_issuance: i + 5,
                    cross_chain_compatibility: CrossChainCompatibility::default(),
                };

                assert_eq!(new_collection_info, expected_collection_info);
                assert_eq!(new_token_ownership, expected_token_ownership);
            }
        });
    }
}
