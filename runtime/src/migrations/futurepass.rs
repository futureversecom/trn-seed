// Copyright 2024-2025 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

use crate::{Futurepass, Runtime, Weight};
use frame_support::{
	dispatch::GetStorageVersion,
	traits::{OnRuntimeUpgrade, StorageVersion},
};
#[allow(unused_imports)]
use sp_std::vec::Vec;

pub struct Upgrade;

impl OnRuntimeUpgrade for Upgrade {
	fn on_runtime_upgrade() -> Weight {
		let current = Futurepass::current_storage_version();
		let onchain = Futurepass::on_chain_storage_version();
		log::info!(target: "Migration", "Futurepass: Running migration with current storage version {current:?} / on-chain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(2);

		if onchain == 0 {
			log::info!(target: "Migration", "Futurepass: Migrating from on-chain version 0 to on-chain version 1.");
			weight += v1::migrate();

			StorageVersion::new(1).put::<Futurepass>();

			log::info!(target: "Migration", "Futurepass: Migration successfully completed.");
		} else {
			log::info!(target: "Migration", "Futurepass: No migration was done, however migration code needs to be removed.");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		v1::pre_upgrade()?;
		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), &'static str> {
		v1::post_upgrade()?;
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
		sp_runtime::RuntimeDebug, storage_alias, traits::IsType, weights::Weight, BoundedVec,
		StorageHasher, Twox64Concat,
	};
	use frame_system::pallet_prelude::BlockNumberFor;
	use scale_info::TypeInfo;
	use seed_primitives::{AssetId, Balance, CollectionUuid};

	type AccountId = <Runtime as frame_system::Config>::AccountId;
	type BlockNumber = BlockNumberFor<Runtime>;

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Futurepass: Upgrade to v1 Pre Upgrade.");
		let onchain = Futurepass::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 1 {
			return Ok(())
		}
		assert_eq!(onchain, 0);

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Futurepass: Upgrade to v1 Post Upgrade.");
		let current = Futurepass::current_storage_version();
		let onchain = Futurepass::on_chain_storage_version();
		assert_eq!(current, 1);
		assert_eq!(onchain, 1);
		Ok(())
	}

	pub fn migrate() -> Weight {
		use frame_support::{traits::PalletInfoAccess, Twox128};
		use sp_io::storage;

		// Construct the storage key for the MigrationAdmin storage item
		let pallet_prefix = Twox128::hash(b"Futurepass");
		let storage_item_name = Twox128::hash(b"MigrationAdmin");
		let storage_key = [pallet_prefix, storage_item_name].concat();

		// Remove the storage item
		storage::clear(&storage_key);

		log::info!(target: "Migration", "Futurepass: successfully killed MigrationAdmin storage item");

		Weight::from_parts(1u64, 0u64)
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::{tests::new_test_ext, Value};

		#[test]
		fn migration_test() {
			new_test_ext().execute_with(|| {
				// Setup storage
				StorageVersion::new(0).put::<Futurepass>();

				let storage_value = AccountId::default();

				// Insert a value into the storage item
				Value::unsafe_storage_put::<AccountId>(
					b"Futurepass",
					b"MigrationAdmin",
					storage_value.clone(),
				);

				// Assert that the value was inserted correctly
				assert_eq!(
					Value::unsafe_storage_get::<AccountId>(b"Futurepass", b"MigrationAdmin"),
					Some(storage_value.clone())
				);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();
				assert_eq!(Futurepass::on_chain_storage_version(), 1);

				// Assert that the value was removed
				assert_eq!(
					Value::unsafe_storage_get::<AccountId>(b"Futurepass", b"MigrationAdmin",),
					None
				);
			});
		}
	}
}
