use crate::*;
use frame_support::{
    dispatch::{GetStorageVersion},
    traits::StorageVersion,
    DefaultNoBound,
};
use pallet_migration::WeightInfo;
use pallet_nft::{CollectionInformation, TokenOwnership};
use seed_primitives::migration::{MigrationStep, MigrationStepResult};
use seed_primitives::{
    CollectionUuid, CrossChainCompatibility, MetadataScheme, OriginChain, RoyaltiesSchedule,
    SerialNumber, TokenCount,
};
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{storage_alias, CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound, Twox64Concat};
use scale_info::TypeInfo;
use sp_runtime::BoundedVec;
use core::fmt::Debug;

#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;
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
        CollectionInformation<<T as frame_system::Config>::AccountId, <T as pallet_nft::Config>::MaxTokensPerCollection, <T as pallet_nft::Config>::StringLimit>,
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
        pub owned_tokens:
            BoundedVec<TokenOwnership<AccountId, MaxTokensPerCollection>, MaxTokensPerCollection>,
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
                }
                Err(e) => {
                    // If we encounter an error during the conversion, we must insert some
                    // default value, we can't remove the token as it will cause unexpected results
                    // with the iter process
                    log::error!(target: LOG_TARGET, "🦆 Error migrating collection_id {:?} : {:?}", key, e);
                    // pallet_nft::CollectionInfo::<T>::insert(key, CollectionInfo::default());
                }
            }
            let last_key = old::CollectionInfo::<T>::hashed_key_for(key);
            MigrationStepResult::continue_step(Self::max_step_weight(), last_key)
        } else {
            log::debug!(target: LOG_TARGET, "🦆 No more tokens to migrate");
            MigrationStepResult::finish_step(Self::max_step_weight())
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::migrations::{tests::new_test_ext, Map};
//     use frame_support::{StorageHasher, Twox64Concat};
//     use hex_literal::hex;
//
//     /// Helper function to manually insert fake data into storage map
//     fn insert_old_data(token_id: TokenId, old_value: old::Xls20TokenId) {
//         let mut key = Twox64Concat::hash(&(token_id.0).encode());
//         let key_2 = Twox64Concat::hash(&(token_id.1).encode());
//         key.extend_from_slice(&key_2);
//         Map::unsafe_storage_put::<old::Xls20TokenId>(b"Xls20", b"Xls20TokenMap", &key, old_value);
//     }
//
//     #[test]
//     fn convert_works() {
//         new_test_ext().execute_with(|| {
//             let old: [u8; 64] = "000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66"
//                 .as_bytes()
//                 .try_into()
//                 .unwrap();
//             let expected: [u8; 32] =
//                 hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66");
//             let new = convert(old).unwrap();
//             assert_eq!(new, expected);
//         });
//     }
//
//     #[test]
//     fn convert_works_explicit() {
//         new_test_ext().execute_with(|| {
//             // Original string: "000800003AE03CAAE14B04F03ACC3DB34EE0B13362C533A016E5C2F800000001"
//             let old: [u8; 64] = [
//                 48, 48, 48, 56, 48, 48, 48, 48, 51, 65, 69, 48, 51, 67, 65, 65, 69, 49, 52, 66, 48,
//                 52, 70, 48, 51, 65, 67, 67, 51, 68, 66, 51, 52, 69, 69, 48, 66, 49, 51, 51, 54, 50,
//                 67, 53, 51, 51, 65, 48, 49, 54, 69, 53, 67, 50, 70, 56, 48, 48, 48, 48, 48, 48, 48,
//                 49,
//             ];
//             //  Manually convert above u8 array to hex array
//             //  0,  0,  0,  8,  0,  0,  0,  0,  3,  A,  E,  0,  3,  C,  A,  A,  E,  1,  4,  B ...
//             //  0x00,   0x08,   0x00,   0x00,   0x3A,   0xE0,   0x3C,   0xAA,   0xE1,   0x4B  ...
//             //  0,      8,      0,      0,      58,     224,    60,     170,    225,    75    ...
//
//             let expected: [u8; 32] = [
//                 0, 8, 0, 0, 58, 224, 60, 170, 225, 75, 4, 240, 58, 204, 61, 179, 78, 224, 177, 51,
//                 98, 197, 51, 160, 22, 229, 194, 248, 0, 0, 0, 1,
//             ];
//             let new = convert(old).unwrap();
//             assert_eq!(new, expected);
//         });
//     }
//
//     #[test]
//     fn migrate_single_step() {
//         new_test_ext().execute_with(|| {
//             let old: [u8; 64] = "000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66"
//                 .as_bytes()
//                 .try_into()
//                 .unwrap();
//             let token_id: TokenId = (1, 2);
//             insert_old_data(token_id, old);
//
//             let result = Xls20Migration::<Runtime>::step(None);
//             assert!(!result.is_finished());
//             let expected: [u8; 32] =
//                 hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66");
//             let new = pallet_xls20::Xls20TokenMap::<Runtime>::get(token_id.0, token_id.1).unwrap();
//             assert_eq!(new, expected);
//
//             // Attempting to perform one more step should return Finished
//             let last_key = result.last_key;
//             let result = Xls20Migration::<Runtime>::step(last_key.clone());
//             assert!(result.is_finished());
//         });
//     }
//
//     #[test]
//     fn migrate_many_steps() {
//         new_test_ext().execute_with(|| {
//             // Insert 100 tokens in 10 different collections
//             let collection_count = 10;
//             let token_count = 100;
//             for i in 0..collection_count {
//                 for j in 0..token_count {
//                     let token_id: TokenId = (i, j);
//                     // insert collection_id and serial_number into first 2 bytes of old
//                     let string = format!(
//                         "{:0>8}{:0>8}{:0>48}",
//                         token_id.0.to_string(),
//                         token_id.1.to_string(),
//                         0
//                     );
//                     let old: [u8; 64] = string.as_bytes().try_into().unwrap();
//                     insert_old_data(token_id, old);
//                 }
//             }
//
//             // Perform migration
//             let mut last_key = None;
//             for _ in 0..collection_count * token_count {
//                 let result = Xls20Migration::<Runtime>::step(last_key.clone());
//                 assert!(!result.is_finished());
//                 last_key = result.last_key;
//             }
//             // One last step to finish migration
//             let result = Xls20Migration::<Runtime>::step(last_key.clone());
//             assert!(result.is_finished());
//
//             // Check that all tokens have been migrated
//             for i in 0..collection_count {
//                 for j in 0..token_count {
//                     let token_id: TokenId = (i, j);
//                     let string = format!(
//                         "{:0>8}{:0>8}{:0>48}",
//                         token_id.0.to_string(),
//                         token_id.1.to_string(),
//                         0
//                     );
//                     let old: [u8; 64] = string.as_bytes().try_into().unwrap();
//                     let expected = convert(old).unwrap();
//                     let new = pallet_xls20::Xls20TokenMap::<Runtime>::get(token_id.0, token_id.1)
//                         .unwrap();
//                     assert_eq!(new, expected);
//                 }
//             }
//         });
//     }
// }
