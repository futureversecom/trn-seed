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

use crate::{Futurepass, Runtime, Weight};
use frame_support::{
	dispatch::GetStorageVersion,
	traits::{OnRuntimeUpgrade, StorageVersion},
};

pub struct Upgrade;
impl OnRuntimeUpgrade for Upgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<crate::Vec<u8>, &'static str> {
		Ok(v1::pre_upgrade()?)
	}

	fn on_runtime_upgrade() -> Weight {
		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads_writes(2, 0);

		let current = Futurepass::current_storage_version();
		let onchain = Futurepass::on_chain_storage_version();

		log::info!(target: "⛔️ Migration", "Futurepass: Running migration with current storage version {current:?} / onchain {onchain:?}");

		if current == 1 && onchain == 0 {
			log::info!(target: "🛠️ Migration", "Futurepass: Migrating from onchain version 0 to onchain version 1.");
			weight += v1::migrate::<Runtime>();

			log::info!(target: "✅ Migration", "Futurepass: Migration successfully completed.");
			StorageVersion::new(1).put::<Futurepass>();
		} else {
			log::info!(target: "⛔️ Migration", "Futurepass: No migration was done. If you are seeing this message, it means that you forgot to remove old existing migration code. Don't panic, it's not a big deal just don't forget it next time :)");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: crate::Vec<u8>) -> Result<(), &'static str> {
		Ok(v1::post_upgrade()?)
	}
}

#[allow(dead_code)]
pub mod v1 {
	use super::*;
	use crate::Vec;
	use frame_support::{dispatch::EncodeLike, migration, StorageHasher, Twox64Concat};
	use pallet_futurepass::Holders;
	use sp_io::hashing::twox_128;

	#[cfg(feature = "try-runtime")]
	use codec::Encode;
	#[cfg(feature = "try-runtime")]
	use frame_support::Blake2_128Concat;

	const MODULE_PREFIX: &[u8] = b"Futurepass";
	const STORAGE_ITEM_NAME: &[u8] = b"Holders";

	fn generate_storage_key<H: StorageHasher<Output = Vec<u8>>>(account: &[u8]) -> Vec<u8> {
		// generate the hashes for the pallet name and storage item name
		let pallet_name_hash = twox_128(MODULE_PREFIX);
		let storage_name_hash = twox_128(STORAGE_ITEM_NAME);
		let account_hash = H::hash(account);

		// concatenate the above hashes to form the final storage key
		let mut storage_key = Vec::new();
		storage_key.extend_from_slice(&pallet_name_hash);
		storage_key.extend_from_slice(&storage_name_hash);
		storage_key.extend_from_slice(&account_hash);

		storage_key
	}

	/// perform pre-upgrade checks to:
	/// - validate value is retrievable from the key via using twoxconcat hashing algorithm
	/// - validate value is not retrievable from the key using black2_128concat hashing algorithm
	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		log::info!(target: "🛠️ Migration", "Futurepass: Upgrade to v1 Pre Upgrade.");

		let onchain = Futurepass::on_chain_storage_version();
		// return early if upgrade has already been done
		if onchain == 1 {
			return Ok(crate::Vec::new())
		}
		assert_eq!(onchain, 0);

		// NOTE: ensure an account (key) must exist in current (twoxconcat) the storage map
		let account = migration::storage_key_iter::<
			seed_primitives::AccountId,
			seed_primitives::AccountId,
			Twox64Concat,
		>(MODULE_PREFIX, STORAGE_ITEM_NAME)
		.next()
		.map(|(k, _)| k)
		.ok_or("🛑 Futurepass: Account not found in pre-upgrade check, this should not happen")?;

		// check if the value is retrievable for the key using twox64concat hashing algorithm
		// NOTE: this is pretty much the same check as the above, but we are validating at a lower
		// level
		let storage_location_twox64concat = generate_storage_key::<Twox64Concat>(&account.encode());
		sp_io::storage::get(&storage_location_twox64concat)
      .ok_or("🛑 Futurepass: Value not found for the key using twox64concat hashing algorithm in pre-upgrade check")?;

		// TODO: figure out why this pre-check causes an error
		// no accounts should be retrievable from new storage map (blake2_128concat)
		// if let Some(_) = Holders::<Runtime>::iter().next().map(|(k, _)| k) {
		//   return Err("🛑 Futurepass: Account found in pre-upgrade check, this should not
		// happen"); };

		// check if the value is not retrievable for the key using black2_128concat hashing
		// algorithm NOTE: this is pretty much the same check as the `Holders::<Runtime>:` above but
		// we are validating at a lower level
		let storage_location_blake2_128concat =
			generate_storage_key::<Blake2_128Concat>(&account.encode());
		if sp_io::storage::get(&storage_location_blake2_128concat).is_some() {
			return Err("🛑 Futurepass: Value found for the key using blake2_128concat hashing algorithm in pre-upgrade check");
		}

		Ok(crate::Vec::new())
	}

	pub fn migrate<T: pallet_futurepass::Config>() -> Weight
	where
		<T as frame_system::Config>::AccountId:
			From<sp_core::H160> + EncodeLike<seed_primitives::AccountId>,
	{
		let mut weight = Weight::from_ref_time(0u64);
		for (key, value) in migration::storage_key_iter::<T::AccountId, T::AccountId, Twox64Concat>(
			MODULE_PREFIX,
			STORAGE_ITEM_NAME,
		)
		.drain()
		{
			// log::info!(target: "🛠️ Migration", "Futurepass: Migrating account {key:?} with value
			// {value:?} from twox64concat to blake2_128concat");
			Holders::<Runtime>::insert(key, value);

			// 1 read for reading the key/value from the drain
			// 1 write for deleting the key/value from the drain
			// 1 write for inserting the key/value into the map (with updated hasher)
			weight += <Runtime as frame_system::Config>::DbWeight::get().reads_writes(1, 2);
		}
		weight
	}

	/// perform post-upgrade checks to:
	/// - validate value is retrievable from the key via using black2_128concat hashing algorithm
	/// - validate value is not retrievable from the key using twoxconcat hashing algorithm
	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade() -> Result<(), &'static str> {
		log::info!(target: "🛠️ Migration", "Futurepass: Upgrade to v1 Post Upgrade.");

		let current = Futurepass::current_storage_version();
		let onchain = Futurepass::on_chain_storage_version();
		assert_eq!(current, 1);
		assert_eq!(onchain, 1);

		// account(s) should be retrievable from the storage
		let Some(account) = Holders::<Runtime>::iter().next().map(|(k, _)| k) else {
      return Err("🛑 Futurepass: Account not found in post-upgrade check, this should not happen");
    };

		// validate the value is retrievable for the key using black2_128concat hashing algorithm
		// NOTE: this is pretty much the same check as the `Holders::<Runtime>:` above but we are
		// validating at a lower level
		let storage_location_blake2_128concat =
			generate_storage_key::<Blake2_128Concat>(&account.encode());
		if sp_io::storage::get(&storage_location_blake2_128concat).is_none() {
			return Err("🛑 Futurepass: Value not found for the key using blake2_128concat hashing algorithm in pre-upgrade check");
		}

		// validate if the value is not retrievable for the key using twox64concat hashing algorithm
		let storage_location_twox64concat = generate_storage_key::<Twox64Concat>(&account.encode());
		if sp_io::storage::get(&storage_location_twox64concat).is_some() {
			Err("🛑 Futurepass: Value found for the key using twox64concat hashing algorithm in post-upgrade check")?;
		}
		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::new_test_ext;

		#[test]
		fn storage_key_test() {
			new_test_ext().execute_with(|| {
				let bob = seed_primitives::AccountId20::from(hex_literal::hex!("25451A4de12dcCc2D166922fA938E900fCc4ED24"));

				let storage_location_twox64concat =
					generate_storage_key::<Twox64Concat>(&bob.encode());
				assert_eq!(
					hex::encode(&storage_location_twox64concat),
					"f87116ea87fb5ad5ef31218b9eb2d0f5410831cea04b01ca98929af04f2caf29864aab6abdc56c6625451a4de12dccc2d166922fa938e900fcc4ed24",
				);

				let storage_location_blake2_128concat =
					generate_storage_key::<Blake2_128Concat>(&bob.encode());
				assert_eq!(
					hex::encode(&storage_location_blake2_128concat),
					"f87116ea87fb5ad5ef31218b9eb2d0f5410831cea04b01ca98929af04f2caf297967b3a70b6512ff5cca5be992f2399a25451a4de12dccc2d166922fa938e900fcc4ed24",
				);
			});
		}

		#[test]
		fn migration_test() {
			new_test_ext().execute_with(|| {
				let bob = seed_primitives::AccountId20::from(hex_literal::hex!(
					"25451A4de12dcCc2D166922fA938E900fCc4ED24"
				));
				let bob_futurepass = seed_primitives::AccountId20([255; 20]);

				// simulate the storage key for the bob account using legacy hashing algorithm
				// (twox64concat) this is analogous to `Holders::<Runtime>::insert(bob,
				// bob_futurepass);` - using the old hashing algorithm ^ we cannot do this as that
				// will store the item with the new hashing algorithm (blake2_128concat)
				let storage_location_twox64concat =
					generate_storage_key::<Twox64Concat>(&bob.encode());
				sp_io::storage::set(&storage_location_twox64concat, &bob_futurepass.encode());

				// validate pre-upgrade checks pass
				let state_data = Upgrade::pre_upgrade().unwrap();

				// perform runtime upgrade
				Upgrade::on_runtime_upgrade();

				// validate post-upgrade checks pass
				Upgrade::post_upgrade(state_data).unwrap();
			});
		}
	}
}
