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

use crate::{AccountId, Runtime, Weight};
use frame_support::traits::OnRuntimeUpgrade;
use pallet_staking::{Payee, RewardDestination};
use sp_std::vec::Vec;

pub struct Upgrade;
impl OnRuntimeUpgrade for Upgrade {
	fn on_runtime_upgrade() -> Weight {
		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads_writes(2, 0);
		log::info!(target: "Migration", "Starting Staking migration");
		weight += v1::migrate::<Runtime>();
		log::info!(target: "Migration", "Staking: Migration successfully finished.");
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
	pub fn post_upgrade() -> Result<(), &'static str> {
		Payee::<Runtime>::iter().for_each(|(k, v)| {
			log::info!(target: "Migration", "Staking: Sanity checking {:?}, {:?}", k, v);
			if v == RewardDestination::Staked {
				log::error!("There was an error migrating Staker reward destinations: {:?} retained their `staked` designation", k);
			}
		});

		log::info!(target: "Migration", "Staking: Upgrade to v1 Post Upgrade.");
		Ok(())
	}

	pub fn migrate<T: pallet_dex::Config>() -> Weight
	where
		<T as frame_system::Config>::AccountId: From<sp_core::H160>,
	{
		let mut weight: Weight = Weight::zero();

		// Same as `Map::iter()` but available for this file
		let existing_storage: Vec<(AccountId, RewardDestination<AccountId>)> =
			Payee::<Runtime>::iter_keys()
				.filter_map(|key| {
					Payee::<Runtime>::try_get(key).and_then(|v| Ok((key.clone(), v))).ok()
				})
				.collect();

		existing_storage.iter().for_each(|(key, v)| {
			if v == &RewardDestination::Staked {
				Payee::<Runtime>::insert(key, RewardDestination::Stash);
				weight += <Runtime as frame_system::Config>::DbWeight::get().reads_writes(0, 2);
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
				let alice = seed_primitives::AccountId20([1; 20]);
				let bob = seed_primitives::AccountId20([2; 20]);
				let charlie = seed_primitives::AccountId20([3; 20]);

				pallet_staking::Payee::<Runtime>::insert(alice, RewardDestination::Staked);
				pallet_staking::Payee::<Runtime>::insert(bob, RewardDestination::Staked);
				pallet_staking::Payee::<Runtime>::insert(charlie, RewardDestination::Staked);

				assert_eq!(pallet_staking::Payee::<Runtime>::get(alice), RewardDestination::Staked);
				assert_eq!(pallet_staking::Payee::<Runtime>::get(bob), RewardDestination::Staked);
				assert_eq!(
					pallet_staking::Payee::<Runtime>::get(charlie),
					RewardDestination::Staked
				);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();

				assert_eq!(pallet_staking::Payee::<Runtime>::get(alice), RewardDestination::Stash);
				assert_eq!(pallet_staking::Payee::<Runtime>::get(bob), RewardDestination::Stash);
				assert_eq!(
					pallet_staking::Payee::<Runtime>::get(charlie),
					RewardDestination::Stash
				);
			});
		}
	}
}
