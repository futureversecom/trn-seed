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
		sp_runtime::RuntimeDebug,
		storage_alias,
		weights::{constants::RocksDbWeight, Weight},
		BoundedVec, StorageHasher, Twox64Concat,
	};
	use pallet_ethy::{BridgePauseStatus, BridgePaused};
	use scale_info::TypeInfo;
	use seed_primitives::ethy::EventClaimId;
	use sp_core::{Get, H160};

	type AccountId = <Runtime as frame_system::Config>::AccountId;

	pub fn migrate<T: frame_system::Config + pallet_ethy::Config>() -> Weight
	where
		AccountId: From<H160>,
	{
		log::info!(target: "Migration", "Ethy: migrating BridgePaused");
		let weight: Weight = RocksDbWeight::get().reads_writes(1, 1);

		let bridge_paused =
			Value::unsafe_storage_get::<bool>(b"EthBridge", b"BridgePaused").unwrap_or_default();

		let paused_status =
			BridgePauseStatus { manual_pause: bridge_paused, authorities_change: false };
		BridgePaused::<T>::put(paused_status);

		log::info!(target: "Migration", "Ethy: successfully migrated BridgePaused");

		weight
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::new_test_ext;

		#[test]
		fn migration_test_bridge_paused_1() {
			new_test_ext().execute_with(|| {
				// Setup storage
				StorageVersion::new(0).put::<EthBridge>();

				// token locks with no listings
				Value::unsafe_storage_put::<bool>(b"EthBridge", b"BridgePaused", false);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();
				assert_eq!(EthBridge::on_chain_storage_version(), 1);

				let pause_status = BridgePaused::<Runtime>::get();
				assert_eq!(
					pause_status,
					BridgePauseStatus { manual_pause: false, authorities_change: false }
				);
			});
		}

		#[test]
		fn migration_test_bridge_paused_2() {
			new_test_ext().execute_with(|| {
				// Setup storage
				StorageVersion::new(0).put::<EthBridge>();

				// token locks with no listings
				Value::unsafe_storage_put::<bool>(b"EthBridge", b"BridgePaused", true);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();
				assert_eq!(EthBridge::on_chain_storage_version(), 1);

				let pause_status = BridgePaused::<Runtime>::get();
				assert_eq!(
					pause_status,
					BridgePauseStatus { manual_pause: true, authorities_change: false }
				);
			});
		}
	}
}
