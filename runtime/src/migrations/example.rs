// DO NOT FORGET TO REPLACE THE MOCK RUNTIME WITH THE REAL ONE!!!!!!
// Uncomment this!!!  use crate::{Example, Runtime};
use super::mock::{Example, Runtime};
use crate::Weight;
use frame_support::{
	assert_ok,
	dispatch::GetStorageVersion,
	pallet_prelude::*,
	storage_alias,
	traits::{OnRuntimeUpgrade, StorageVersion},
};
use scale_info::TypeInfo;
use sp_std::prelude::*;

#[allow(unused_imports)]
use super::{Map, Value, Pallet};

// Source:
//	https://substrate.stackexchange.com/questions/6097/substrate-translate-function <- How to properly translate values
//	https://substrate.stackexchange.com/questions/3252/migrate-keytype-of-storagemap-without-breakin-api/3267#3267 <- How to use storage_alias
// 	https://substrate.stackexchange.com/questions/6133/are-pallet-migrations-triggered-automatically-by-default-or-we-need-to-pass-each <- Where and how to call migrations

// This example will teach you:
// - How to structure your pallet migration code
// - Where the place your migrations
// - How to remove storage values and maps
// - How to translate storage values and maps
// - How to move storage from one pallet to another pallet
// - How to move storage from within pallet
// - How to rename a pallet

/// This is the main structure that handles all migrations for a specific pallet.
pub struct Upgrade;
impl OnRuntimeUpgrade for Upgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		v2::pre_upgrade()?;

		Ok(())
	}

	fn on_runtime_upgrade() -> Weight {
		let current = Example::current_storage_version();
		let onchain = Example::on_chain_storage_version();
		let mut weight = Weight::from(0u32);

		log::info!(target: "Example", "Running migration with current storage version {current:?} / onchain {onchain:?}");

		if onchain == 1 {
			log::info!(target: "Example", "Migrating from onchain version 1 to onchain version 2.");
			weight += v2::migrate::<Runtime>();

			log::info!(target: "Example", "Migration successfully finished.");
			StorageVersion::new(2).put::<Example>();
		} else {
			log::info!(target: "Example", "No migration was done. If you are seeing this message, it means that you forgot to remove old existing migration code. Don't panic, it's not a big deal just don't forget it next time :)");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		v2::post_upgrade()?;

		Ok(())
	}
}

mod v2 {
	use core::fmt::Debug;

	use super::*;
	use frame_support::{weights::Weight, Twox64Concat};

	#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo, MaxEncodedLen, Default)]
	pub struct OldType {
		pub value: u32,
		pub removed: u32,
	}

	#[storage_alias]
	pub type MyValue<T: pallet_example::Config> = StorageValue<pallet_example::Pallet<T>, OldType>;

	#[storage_alias]
	pub type MyMap<T: pallet_example::Config> =
		StorageMap<pallet_example::Pallet<T>, Twox64Concat, u32, OldType>;

	pub type OldMyValue<T> = MyValue<T>;
	pub type OldMyMap<T> = MyMap<T>;

	pub fn migrate<T: pallet_example::Config>() -> Weight {
		// Warning: Here we are using custom made translate and remove functions.
		// You should use the template ones from mod.rs
		// Check template_remove_example and template_translate_example tests
		//
		// In case the existing template ones are insufficient then you can
		// modify the existing ones and use them

		// Example on how to transform storage values
		translate_storage_value::<T>();
		translate_storage_map::<T>();

		// Example on how to remove storage values
		remove_storage_value::<T>();
		remove_storage_map::<T>();

		<Runtime as frame_system::Config>::DbWeight::get().reads_writes(1, 1)
	}

	// Use remove_value instead of this
	pub fn remove_storage_value<T: pallet_example::Config>() {
		if OldMyValue::<T>::exists() {
			OldMyValue::<T>::kill();
			log::error!("Successfully removed MyValue");
		} else {
			log::error!("Failed to remove MyValue. It doesn't exist");
		}
	}

	// Use remove_map instead of this
	pub fn remove_storage_map<T: pallet_example::Config>() {
		let res = OldMyMap::<T>::clear(u32::MAX, None);
		if res.maybe_cursor.is_some() {
			log::error!("Should not happen");
		} else {
			log::info!("All good with remove storage map");
		};
	}

	// Use translate_value instead of this
	pub fn translate_storage_value<T: pallet_example::Config>() {
		let res = pallet_example::MyValue::<T>::translate::<OldType, _>(|old_data| {
			if let Some(data) = old_data {
				let new_value = pallet_example::NewType { value: data.value };
				return Some(new_value)
			}

			None
		});

		if let Err(_) = res {
			log::error!("Failed to decode MyValue");
		} else {
			log::info!("All good with translate storage value")
		}
	}

	// Use translate_map instead of this
	pub fn translate_storage_map<T: pallet_example::Config>() {
		let original_count = OldMyMap::<T>::iter_keys().count();
		let keys_values: Vec<(u32, OldType)> = OldMyMap::<T>::iter_keys()
			.filter_map(|key| {
				if let Ok(value) = OldMyMap::<T>::try_get(key) {
					return Some((key, value))
				} else {
					log::error!("Removed undecodable MyMap value: {:?}", key);
					return None
				}
			})
			.collect();

		// Delete whole storage
		remove_storage_map::<T>();

		// Translate
		for (key, old_value) in keys_values {
			let new_value = pallet_example::NewType { value: old_value.value };
			pallet_example::MyMap::<T>::insert(key, new_value);
		}

		// Check
		let new_count = pallet_example::MyMap::<T>::iter_keys().count();
		if original_count == new_count {
			log::info!("All good with translate storage map")
		} else {
			log::error!("Something went wrong with translating MyMap. Old Count {original_count:?}, New Count: {new_count:?}");
		}
	}

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Example", "Example Upgrade to V2 Pre Upgrade.");
		// Storage Version Check
		let onchain = Example::on_chain_storage_version();
		assert_eq!(onchain, 1);

		// Check that we actually have some data and that it is not corrupted
		assert_ok!(Value::exists_valid::<OldMyValue::<Runtime>, _>());
		assert_ok!(Map::exists_valid::<OldMyMap::<Runtime>, _, _>());

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Example", "Example Upgrade to V2 Post Upgrade.");
		// Storage Version Check
		let onchain = Example::on_chain_storage_version();
		assert_eq!(onchain, 2);

		// Check that we actually have some data and that it is not corrupted
		assert_ok!(Value::exists_valid::<pallet_example::MyValue::<Runtime>, _>());
		assert_ok!(Map::exists_valid::<pallet_example::MyMap::<Runtime>, _, _>());

		Ok(())
	}

	#[cfg(test)]
	mod tests {
		use frame_support::Hashable;
		use pallet_example::NewType;

		use super::*;
		use crate::migrations::tests::new_test_ext;

		#[test]
		fn migration_test() {
			new_test_ext().execute_with(|| {
				// Preparation
				StorageVersion::new(1).put::<Example>();

				let old_value = OldType { value: 10, removed: 20 };
				OldMyValue::<Runtime>::put(old_value);

				let old_value = OldType { value: 10, removed: 20 };
				let old_value_2 = OldType { value: 20, removed: 30 };
				OldMyMap::<Runtime>::insert(0, old_value);
				OldMyMap::<Runtime>::insert(1, old_value_2);

				// Action
				Upgrade::on_runtime_upgrade();

				// Check
				assert_eq!(Value::exists::<OldMyValue::<Runtime>, _>(), false);
				assert_eq!(Map::exists::<OldMyMap::<Runtime>, _, _>(), false);

				// Last check: We have updated the storage version of pallet
				let onchain = Example::on_chain_storage_version();
				assert_eq!(onchain, 2);
				// or assert_eq!(StorageVersion::get::<Example>(), 2);
			});
		}

		#[test]
		fn testing_corrupted_storage() {
			use frame_support::migration::put_storage_value;

			new_test_ext().execute_with(|| {
				let old_value = OldType { value: 10, removed: 20 };
				let old_value_2 = OldType { value: 20, removed: 30 };
				OldMyMap::<Runtime>::insert(0, old_value);
				OldMyMap::<Runtime>::insert(1, old_value_2);

				// Map should be valid at this point
				assert_eq!(Map::valid::<OldMyMap::<Runtime>, _, _>(), Ok(2));

				// Inserting Corrupted Data
				let module = Example::name().as_bytes();
				let key = 3u32.twox_64_concat();
				put_storage_value(module, b"MyMap", &key, 123u8);

				// Map should be corrupted at this point
				assert_eq!(Map::valid::<OldMyMap::<Runtime>, _, _>(), Err(3u32));

				let keys: Vec<u32> = pallet_example::MyMap::<Runtime>::iter_keys().collect();
				let keys_len = keys.len();
				assert_eq!(keys_len, 3);

				// Action
				translate_storage_map::<Runtime>();

				// Check that we have removed the corrupted key and that we are left with just two
				// keys
				assert_eq!(
					Map::valid::<pallet_example::MyMap::<Runtime>, _, _>(),
					Ok(keys_len - 1)
				);
			});
		}

		#[test]
		fn template_remove_example() {
			new_test_ext().execute_with(|| {
				// Populating Storage
				let value = NewType { value: 100 };
				pallet_example::MyValue::<Runtime>::put(value.clone());
				pallet_example::MyMap::<Runtime>::insert(100u32, value.clone());

				// Making sure that we have actually write values to these storages
				assert_eq!(
					Value::exists_valid::<pallet_example::MyValue::<Runtime>, _>(),
					Ok(value)
				);
				assert_eq!(Map::exists_valid::<pallet_example::MyMap::<Runtime>, _, _>(), Ok(1));

				// Remove them
				_ = Value::clear::<pallet_example::MyValue<Runtime>, _>();
				_ = Map::clear::<pallet_example::MyMap<Runtime>, _, _>();

				// Check that they are removed
				assert_eq!(Value::exists::<pallet_example::MyValue::<Runtime>, _>(), false);
				assert_eq!(Map::exists::<pallet_example::MyMap::<Runtime>, _, _>(), false);
			});
		}

		#[test]
		fn template_translate_example() {
			new_test_ext().execute_with(|| {
				// Populating Storage
				let old_value = OldType { value: 100, removed: 50 };
				let key = 100u32;
				OldMyValue::<Runtime>::put(old_value.clone());
				OldMyMap::<Runtime>::insert(key, old_value.clone());

				// Making sure that we have actually write values to the storage
				assert!(OldMyValue::<Runtime>::exists());
				assert_eq!(OldMyValue::<Runtime>::get(), Some(old_value.clone()));
				assert_eq!(OldMyMap::<Runtime>::iter().count(), 1);
				assert_eq!(OldMyMap::<Runtime>::get(key), Some(old_value));

				// Translate them
				_ = Value::translate::<pallet_example::MyValue<Runtime>, _, _>(|old: OldType| {
					NewType { value: old.value }
				});
				_ = Map::translate::<OldMyMap<Runtime>, pallet_example::MyMap<Runtime>, _, _, _, _>(
					|key: u32, old: OldType| (key, NewType { value: old.value }),
				);

				// Check that they are removed
				let new_value = NewType { value: 100 };
				assert!(pallet_example::MyValue::<Runtime>::exists());
				assert_eq!(pallet_example::MyValue::<Runtime>::try_get(), Ok(new_value.clone()));
				assert_eq!(pallet_example::MyMap::<Runtime>::iter().count(), 1);
				assert_eq!(pallet_example::MyMap::<Runtime>::try_get(key), Ok(new_value));
			});
		}

		#[test]
		fn using_raw_migration_functions() {
			new_test_ext().execute_with(|| {
				let module = Example::name().as_bytes();
				let value_name = b"MyValue";
				let map_name = b"MyMap";
				let value = OldType { value: 100, removed: 200 };

				// Adding Value
				Value::unsafe_put(module, value_name, value.clone());

				// Adding Map items
				let (key_1, key_2) = (0u32.twox_64_concat(), 1u32.twox_64_concat());
				Map::unsafe_put(module, map_name, &key_1, value.clone());
				Map::unsafe_put(module, map_name, &key_2, value.clone());

				// Checking Values
				assert_eq!(Value::unsafe_get(module, value_name), Some(value.clone()));
				// Checking Map elements
				assert_eq!(Map::unsafe_get(module, map_name, &key_1), Some(value.clone()));
				assert_eq!(Map::unsafe_get(module, map_name, &key_2), Some(value.clone()));

				// Moving Map Storage from one place to another within the same pallet.
				// NOTE: The value at the key `from_prefix` is not moved !!!
				// Doesn't work with storage value!
				let new_map_name = b"NewMyMap";
				Map::unsafe_rename(module, map_name, new_map_name);
				// have_storage_value internally calls get_storage_value
				assert_eq!(Map::unsafe_elem_exists(module, map_name, &key_1), false);
				assert_eq!(Map::unsafe_elem_exists(module, map_name, &key_2), false);
				assert_eq!(Map::unsafe_elem_exists(module, new_map_name, &key_1), true);
				assert_eq!(Map::unsafe_elem_exists(module, new_map_name, &key_2), true);

				// Moving Storage Value from one place to another within the same pallet.
				let new_value_name = b"NewMyValue";
				assert_eq!(
					Value::unsafe_rename::<OldType>(module, value_name, new_value_name),
					true
				);
				assert_eq!(Value::unsafe_exists(module, value_name), false);
				assert_eq!(Value::unsafe_exists(module, new_value_name), true);

				// Move storage from one pallet to another pallet
				let new_module = b"newpallet";
				Map::unsafe_move(new_map_name, &module, new_module); // Map
				Value::unsafe_move(new_value_name, &module, new_module); // Value
				assert_eq!(Map::unsafe_elem_exists(module, new_map_name, &key_1), false);
				assert_eq!(Map::unsafe_elem_exists(module, new_map_name, &key_2), false);
				assert_eq!(Map::unsafe_elem_exists(new_module, new_map_name, &key_1), true);
				assert_eq!(Map::unsafe_elem_exists(new_module, new_map_name, &key_2), true);
				assert_eq!(Value::unsafe_exists(module, new_value_name), false);
				assert_eq!(Value::unsafe_exists(new_module, new_value_name), true);

				// Rename pallet
				let different_module = b"newnewpallet";
				Pallet::unsafe_move_pallet(new_module, different_module);
				assert_eq!(Map::unsafe_elem_exists(new_module, new_map_name, &key_1), false);
				assert_eq!(Map::unsafe_elem_exists(new_module, new_map_name, &key_2), false);
				assert_eq!(Map::unsafe_elem_exists(different_module, new_map_name, &key_1), true);
				assert_eq!(Map::unsafe_elem_exists(different_module, new_map_name, &key_2), true);
				assert_eq!(Value::unsafe_exists(new_module, new_value_name), false);
				assert_eq!(Value::unsafe_exists(different_module, new_value_name), true);

				// Remove Value
				assert_eq!(Value::unsafe_exists(different_module, new_value_name), true);
				_ = Value::unsafe_clear(different_module, new_value_name);
				assert_eq!(Value::unsafe_exists(different_module, new_value_name), false);

				// Remove Map
				assert_eq!(Map::unsafe_elem_exists(different_module, new_map_name, &key_1), true);
				assert_eq!(Map::unsafe_elem_exists(different_module, new_map_name, &key_2), true);
				_ = Map::unsafe_clear(different_module, new_map_name);
				assert_eq!(Map::unsafe_elem_exists(different_module, new_map_name, &key_1), false);
				assert_eq!(Map::unsafe_elem_exists(different_module, new_map_name, &key_2), false);

				// Additional functionality to check out if necessary
				// storage_key_iter
				// storage_iter
				// take_storage_item
			});
		}
	}
}
