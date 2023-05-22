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

use crate::{AccountId, Staking, Runtime, Weight, StakingRewardDestinationsVersionTmp};
use frame_support::{
	dispatch::GetStorageVersion,
	traits::{OnRuntimeUpgrade, StorageVersion},
};
use pallet_staking::{RewardDestination, Payee};
use sp_std::vec::Vec;

pub struct Upgrade;
impl OnRuntimeUpgrade for Upgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		v1::pre_upgrade()?;
		Ok(())
	}

	fn on_runtime_upgrade() -> Weight {
		let current = Staking::current_storage_version();
		let onchain = StakingRewardDestinationsVersionTmp::<Runtime>::get();
		log::info!(target: "Migration", "Staking: Running migration with current storage version {current:?} / onchain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads_writes(2, 0);

		if onchain == 0 {
			log::info!(target: "Migration", "Staking: Migrating from onchain version 0 to onchain version 1.");
			weight += v1::migrate::<Runtime>();

			log::info!(target: "Migration", "Staking: Migration successfully finished.");
            StakingRewardDestinationsVersionTmp::<Runtime>::set(1);
		} else {
			log::info!(target: "Migration", "Staking: No migration was done. If you are seeing this message, it means that you forgot to remove old existing migration code. Don't panic, it's not a big deal just don't forget it next time :)");
		}
		weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		v1::post_upgrade()?;

		Ok(())
	}
}

#[allow(dead_code)]
pub mod v1 {
	use super::*;

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Staking: Upgrade to v1 Pre Upgrade.");
		let onchain = Staking::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 1 {
			return Ok(())
		}
		assert_eq!(onchain, 0);

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Staking: Upgrade to v1 Post Upgrade.");

		let current = Staking::current_storage_version();
		let onchain = Staking::on_chain_storage_version();
		assert_eq!(current, 1);
		assert_eq!(onchain, 1);

		Ok(())
	}

	pub fn migrate<T: pallet_dex::Config>() -> Weight
	where
		<T as frame_system::Config>::AccountId: From<sp_core::H160>,
	{
        let mut weight = 0;

        // Same as `Map::iter()` but available for this file
        let existing_storage: Vec<(AccountId, RewardDestination<AccountId>)> = Payee::<Runtime>::iter_keys()
            .filter_map(|key| Payee::<Runtime>::try_get(key).and_then(|v| Ok((key.clone(), v))).ok())
            .collect();

        existing_storage.iter()
        .for_each(|(key, v)| {
            if let Ok(mut payee) = Payee::<Runtime>::try_get(&key) {
                if payee == RewardDestination::Staked {
                    // Try removing first
                    Payee::<Runtime>::remove(key);
                    Payee::<Runtime>::insert(key, RewardDestination::Stash);
                    weight += <Runtime as frame_system::Config>::DbWeight::get().reads_writes(0, 2);
                }
            }
            weight += <Runtime as frame_system::Config>::DbWeight::get().reads_writes(1, 0);
        });
        weight
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::new_test_ext;

		#[test]
		fn migration_test() {
			new_test_ext().execute_with(|| {
				StorageVersion::new(0).put::<Staking>();
                let alice = seed_primitives::AccountId20([1; 20]);

                pallet_staking::Payee::<Runtime>::insert(alice, RewardDestination::Staked);
				assert_eq!(pallet_staking::Payee::<Runtime>::get(alice), RewardDestination::Staked);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();

				assert_eq!(pallet_staking::Payee::<Runtime>::get(alice), RewardDestination::Stash);

				// Check if version has been set correctly
				let onchain = StakingRewardDestinationsVersionTmp::<Runtime>::get();
				assert_eq!(onchain, 1);
			});
		}
	}
}
