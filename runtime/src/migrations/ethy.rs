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

use crate::{EthBridge, Runtime, Weight};
use frame_support::{
	dispatch::GetStorageVersion,
	traits::{OnRuntimeUpgrade, StorageVersion},
};
#[allow(unused_imports)]
use sp_std::vec::Vec;

pub struct Upgrade;

impl OnRuntimeUpgrade for Upgrade {
	fn on_runtime_upgrade() -> Weight {
		let current = EthBridge::current_storage_version();
		let onchain = EthBridge::on_chain_storage_version();
		log::info!(target: "Migration", "Ethy: Running migration with current storage version {current:?} / on-chain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(2);

		if onchain == 0 {
			log::info!(target: "Migration", "Ethy: Migrating from on-chain version 0 to on-chain version 1.");
			weight += v1::migrate::<Runtime>();

			StorageVersion::new(1).put::<EthBridge>();

			log::info!(target: "Migration", "Ethy: Migration successfully completed.");
		} else {
			log::info!(target: "Migration", "Ethy: No migration was done, however migration code needs to be removed.");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		log::info!(target: "Migration", "Ethy: Upgrade to v1 Pre Upgrade.");
		let onchain = EthBridge::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 1 {
			return Ok(Vec::new())
		}
		assert_eq!(onchain, 0);

		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), &'static str> {
		log::info!(target: "Migration", "Ethy: Upgrade to v1 Post Upgrade.");
		let current = EthBridge::current_storage_version();
		let onchain = EthBridge::on_chain_storage_version();
		assert_eq!(current, 1);
		assert_eq!(onchain, 1);
		Ok(())
	}
}

#[allow(dead_code)]
#[allow(unused_imports)]
pub mod v1 {
	use super::*;
	use crate::migrations::{Map, Value};
	use codec::{Decode, Encode, MaxEncodedLen};
	use frame_support::{
		sp_runtime::RuntimeDebug, storage_alias, weights::Weight, BoundedVec, StorageHasher,
		Twox64Concat,
	};
	use pallet_ethy::ProcessedMessageIds;
	use scale_info::TypeInfo;
	use seed_primitives::ethy::EventClaimId;
	use sp_core::{Get, H160};

	type AccountId = <Runtime as frame_system::Config>::AccountId;

	pub fn migrate<T: frame_system::Config + pallet_ethy::Config>() -> Weight
	where
		AccountId: From<H160>,
	{
		log::info!(target: "Migration", "Ethy: migrating ProcessedMessageIds");
		let mut weight = Weight::zero();

		let mut processed_message_ids =
			Value::unsafe_storage_get::<Vec<EventClaimId>>(b"EthBridge", b"ProcessedMessageIds")
				.unwrap_or_default();
		let prune_weight = pallet_ethy::Pallet::<T>::prune_claim_ids(&mut processed_message_ids);
		weight = weight.saturating_add(prune_weight);
		let message_ids = BoundedVec::truncate_from(processed_message_ids);
		ProcessedMessageIds::<T>::put(message_ids);

		log::info!(target: "Migration", "Ethy: successfully migrated ProcessedMessageIds");

		weight
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::{migrations::tests::new_test_ext, MaxProcessedMessageIds};
		use pallet_ethy::MissedMessageIds;

		#[test]
		fn migration_test_1() {
			new_test_ext().execute_with(|| {
				// Setup storage
				StorageVersion::new(0).put::<EthBridge>();

				// token locks with no listings
				let event_ids: Vec<EventClaimId> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
				Value::unsafe_storage_put::<Vec<EventClaimId>>(
					b"EthBridge",
					b"ProcessedMessageIds",
					event_ids,
				);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();
				assert_eq!(EthBridge::on_chain_storage_version(), 1);

				let event_ids = ProcessedMessageIds::<Runtime>::get();
				assert_eq!(event_ids.into_inner(), vec![10]);
				let missed_ids = MissedMessageIds::<Runtime>::get();
				assert!(missed_ids.is_empty());
			});
		}

		#[test]
		fn migration_test_2() {
			new_test_ext().execute_with(|| {
				// Setup storage
				StorageVersion::new(0).put::<EthBridge>();

				// Create false data with all even numbers between 0 and 4000
				let max = MaxProcessedMessageIds::get() as u64;
				let event_ids: Vec<EventClaimId> =
					(0u64..max * 2).into_iter().map(|x| x * 2).collect();

				assert_eq!(event_ids.len(), (max * 2) as usize);
				Value::unsafe_storage_put::<Vec<EventClaimId>>(
					b"EthBridge",
					b"ProcessedMessageIds",
					event_ids,
				);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();
				assert_eq!(EthBridge::on_chain_storage_version(), 1);

				// ProcessedMessageIds should be the second half of the original list
				// i.e. all even numbers between 2000 and 4000
				let expected_event_ids: Vec<EventClaimId> =
					(max..max * 2).into_iter().map(|x| x * 2).collect();
				let event_ids = ProcessedMessageIds::<Runtime>::get();
				assert_eq!(event_ids.len(), max as usize);
				assert_eq!(event_ids.into_inner(), expected_event_ids);

				// MissedMessageIds should be all the odd message Ids in the first half of the
				// original list. i.e. 1,3,5,7
				let expected_missed_ids: Vec<EventClaimId> =
					(0..max).into_iter().map(|x| x * 2 + 1).collect();
				let missed_ids = MissedMessageIds::<Runtime>::get();
				assert_eq!(missed_ids.len(), max as usize);
				assert_eq!(missed_ids, expected_missed_ids);
			});
		}
	}
}
