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
use sp_runtime::DispatchError;
#[allow(unused_imports)]
use sp_std::vec::Vec;

pub struct Upgrade;

impl OnRuntimeUpgrade for Upgrade {
    fn on_runtime_upgrade() -> Weight {
        let current = Nft::current_storage_version();
        let onchain = Nft::on_chain_storage_version();
        log::info!(target: "Migration", "Nft: Running migration with current storage version {current:?} / on-chain {onchain:?}");

        let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(2);

        if onchain != 7 {
            log::info!(
				target: "Migration",
				"Nft: No migration was done, This migration should be on top of storage version 7. Migration code needs to be removed."
			);
            return weight;
        }

        log::info!(target: "Migration", "Nft: Migrating from on-chain version {onchain:?} to on-chain version 8.");
        weight += v8::migrate::<Runtime>();
        StorageVersion::new(8).put::<Nft>();
        log::info!(target: "Migration", "Nft: Migration successfully completed.");
        weight
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
        log::info!(target: "Migration", "Nft: Upgrade to v8 Pre Upgrade.");
        let onchain = Nft::on_chain_storage_version();
        // Return OK(()) if upgrade has already been done
        if onchain == 1 {
            return Ok(Vec::new());
        }
        assert_eq!(onchain, 0);

        Ok(Vec::new())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(_state: Vec<u8>) -> Result<(), DispatchError> {
        log::info!(target: "Migration", "Nft: Upgrade to v8 Post Upgrade.");
        let current = Nft::current_storage_version();
        let onchain = Nft::on_chain_storage_version();
        assert_eq!(current, 8);
        assert_eq!(onchain, 8);
        Ok(())
    }
}

#[allow(dead_code)]
#[allow(unused_imports)]
pub mod v8 {
    use core::fmt::Debug;
    use codec::{Decode, Encode, MaxEncodedLen};
    use super::*;
    use crate::migrations::Value;
    use frame_support::pallet_prelude::ValueQuery;
    use frame_support::{BoundedVec, Twox64Concat};
    use frame_support::weights::Weight;
    use seed_primitives::{Balance, CollectionUuid, SerialNumber, TokenId};
    use frame_support::{
        storage_alias, CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound,
    };
    use scale_info::TypeInfo;
    use sp_core::{Get, H160};
    use seed_pallet_common::utils::TokenBurnAuthority;

    #[frame_support::storage_alias]
    pub type PendingIssuances<T: pallet_nft::Config> = StorageMap<
        pallet_nft::Pallet<T>,
        Twox64Concat,
        CollectionUuid,
        CollectionPendingIssuances<<T as frame_system::Config>::AccountId, <T as pallet_nft::Config>::MaxPendingIssuances>,
        ValueQuery,
    >;

    type IssuanceId = u32;

    #[derive(
        PartialEqNoBound,
        RuntimeDebugNoBound,
        CloneNoBound,
        Encode,
        Decode,
        TypeInfo,
        MaxEncodedLen,
    )]
    pub struct PendingIssuance {
        pub issuance_id: IssuanceId,
        pub quantity: u32,
        pub burn_authority: TokenBurnAuthority,
    }

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
    #[scale_info(skip_type_params(MaxPendingIssuances))]
    pub struct CollectionPendingIssuances<AccountId, MaxPendingIssuances: Get<u32>>
    where
        AccountId: Debug + PartialEq + Clone,
        MaxPendingIssuances: Get<u32>,
    {
        pub next_issuance_id: IssuanceId,
        pub pending_issuances: BoundedVec<
            (AccountId, BoundedVec<PendingIssuance, MaxPendingIssuances>),
            MaxPendingIssuances,
        >,
    }

    impl<AccountId, MaxPendingIssuances> Default
    for CollectionPendingIssuances<AccountId, MaxPendingIssuances>
    where
        AccountId: Debug + PartialEq + Clone,
        MaxPendingIssuances: Get<u32>,
    {
        fn default() -> Self {
            CollectionPendingIssuances { next_issuance_id: 0, pending_issuances: BoundedVec::new() }
        }
    }


    pub fn migrate<T: frame_system::Config + pallet_nft::Config>() -> Weight {
        log::info!(target: "Migration", "Nft: removing PendingIssuances");

        let results = PendingIssuances::<T>::clear(u32::MAX, None);
        let weight: Weight = <T as frame_system::Config>::DbWeight::get().reads(results.loops as u64);

        log::info!(target: "Migration", "Nft: removal of PendingIssuance successful");
        weight
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::migrations::tests::new_test_ext;
        use crate::migrations::Map;
        use codec::{Decode, Encode};
        use frame_support::{BoundedVec, StorageHasher};
        use scale_info::TypeInfo;
        use sp_core::H256;
        use crate::migrations::tests::create_account;

        #[test]
        fn migrate_with_data() {
            new_test_ext().execute_with(|| {
                // Setup storage
                StorageVersion::new(7).put::<Nft>();
                let item_count = 10_u32;
                for i in 0..item_count {
                    let pending_issuance = CollectionPendingIssuances {
                        next_issuance_id: i + 5,
                        pending_issuances: BoundedVec::truncate_from(vec![(create_account(i as u64), BoundedVec::truncate_from(vec![PendingIssuance {
                            issuance_id: i + 4,
                            quantity: i,
                            burn_authority: TokenBurnAuthority::Both,
                        }]))]),
                    };
                    PendingIssuances::<Runtime>::insert(i, pending_issuance);
                }

                // Sanity check
                for i in 0..item_count {
                    assert!(PendingIssuances::<Runtime>::contains_key(i));
                }

                // Do runtime upgrade
                let used_weight = Upgrade::on_runtime_upgrade();
                assert_eq!(Nft::on_chain_storage_version(), 8);

                // Check storage is removed
                for i in 0..item_count {
                    assert!(!PendingIssuances::<Runtime>::contains_key(i));
                }
            });
        }
    }
}