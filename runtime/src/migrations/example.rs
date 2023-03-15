use super::mock::{Example, Runtime};
use crate::Weight;
use frame_support::{
	dispatch::GetStorageVersion,
	pallet_prelude::*,
	storage_alias,
	traits::{OnRuntimeUpgrade, StorageVersion},
};
use scale_info::TypeInfo;
use sp_std::prelude::*;

#[cfg(test)]
use super::{remove_map, remove_value, translate_map, translate_value};

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
		// Example on how to transform storage values
		translate_storage_value::<T>();
		translate_storage_map::<T>();

		// Example on how to remove storage values
		remove_storage_value::<T>();
		remove_storage_map::<T>();

		<Runtime as frame_system::Config>::DbWeight::get().reads_writes(1, 1)
	}

	pub fn remove_storage_value<T: pallet_example::Config>() {
		if OldMyValue::<T>::exists() {
			OldMyValue::<T>::kill();
			log::error!("Successfully removed MyValue");
		} else {
			log::error!("Failed to remove MyValue. It doesn't exist");
		}
	}

	pub fn remove_storage_map<T: pallet_example::Config>() {
		let res = OldMyMap::<T>::clear(u32::MAX, None);
		if res.maybe_cursor.is_some() {
			log::error!("Should not happen");
		} else {
			log::info!("All good with remove storage map");
		};
	}

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

		// Let's make sure that we don't have any corrupted data to begin with
		let keys: Vec<u32> = OldMyMap::<Runtime>::iter_keys().collect();
		for key in keys {
			assert!(OldMyMap::<Runtime>::try_get(key).is_ok());
		}

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Example", "Example Upgrade to V2 Post Upgrade.");
		// Storage Version Check
		let onchain = Example::on_chain_storage_version();
		assert_eq!(onchain, 2);

		// Let's make sure that after upgrade we don't have corrupted data
		let keys: Vec<u32> = pallet_example::MyMap::<Runtime>::iter_keys().collect();
		for key in keys {
			assert!(pallet_example::MyMap::<Runtime>::try_get(key).is_ok());
		}
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
				assert_eq!(OldMyValue::<Runtime>::exists(), false);
				assert_eq!(OldMyMap::<Runtime>::iter().count(), 0);

				// Last check: We have updated the storage version of pallet
				let onchain = Example::on_chain_storage_version();

				assert_eq!(onchain, 2);
				assert_eq!(StorageVersion::get::<Example>(), 2);
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

				// Inserting Corrupted Data
				let module = Example::name().as_bytes();
				let key = 3u32.twox_64_concat();
				put_storage_value(module, b"MyMap", &key, 123u8);

				let keys: Vec<u32> = pallet_example::MyMap::<Runtime>::iter_keys().collect();
				assert_eq!(keys.len(), 3);

				// Action
				translate_storage_map::<Runtime>();

				// Check
				let keys: Vec<u32> = pallet_example::MyMap::<Runtime>::iter_keys().collect();
				assert_eq!(keys.len(), 2);
				for key in keys {
					assert!(pallet_example::MyMap::<Runtime>::try_get(key).is_ok());
				}
			});
		}

		#[test]
		fn template_remove_example() {
			new_test_ext().execute_with(|| {
				// Populating Storage
				pallet_example::MyValue::<Runtime>::put(NewType { value: 100 });
				pallet_example::MyMap::<Runtime>::insert(100u32, NewType { value: 100 });

				// Making sure that we have actually write values to the storage
				assert!(pallet_example::MyValue::<Runtime>::exists());
				assert!(pallet_example::MyMap::<Runtime>::iter().count() > 0);

				// Remove them
				_ = remove_value::<pallet_example::MyValue<Runtime>, _>();
				_ = remove_map::<pallet_example::MyMap<Runtime>, _, _>();

				// Check that they are removed
				assert!(!pallet_example::MyValue::<Runtime>::exists());
				assert_eq!(pallet_example::MyMap::<Runtime>::iter().count(), 0);
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

				// Remove them
				_ = translate_value::<pallet_example::MyValue<Runtime>, _, _>(|old: OldType| {
					NewType { value: old.value }
				});
				_ = translate_map::<OldMyMap<Runtime>, pallet_example::MyMap<Runtime>, _, _, _, _>(
					|key: u32, old: OldType| (key, NewType { value: old.value }),
				);

				// Check that they are removed
				let new_value = NewType { value: 100 };
				assert!(pallet_example::MyValue::<Runtime>::exists());
				assert_eq!(pallet_example::MyValue::<Runtime>::get(), new_value);
				assert_eq!(pallet_example::MyMap::<Runtime>::iter().count(), 1);
				assert_eq!(pallet_example::MyMap::<Runtime>::get(key), new_value);
			});
		}

		#[test]
		fn using_raw_migration_functions() {
			use frame_support::{
				migration::{
					clear_storage_prefix, get_storage_value, have_storage_value, move_pallet,
					move_prefix, move_storage_from_pallet, put_storage_value, take_storage_value,
				},
				storage::storage_prefix,
			};

			new_test_ext().execute_with(|| {
				let module = Example::name().as_bytes();
				let value_name = b"MyValue";
				let map_name = b"MyMap";
				let value = OldType { value: 100, removed: 200 };

				// Adding Value
				put_storage_value(module, value_name, b"", value.clone());

				// Adding Map items
				let (key_1, key_2) = (0u32.twox_64_concat(), 1u32.twox_64_concat());
				put_storage_value(module, map_name, &key_1, value.clone());
				put_storage_value(module, map_name, &key_2, value.clone());

				// Checking and getting Values
				assert_eq!(have_storage_value(module, value_name, b""), true);
				assert_eq!(get_storage_value(module, value_name, b""), Some(value.clone()));
				// Checking and getting Map elements
				assert_eq!(have_storage_value(module, map_name, &key_1), true);
				assert_eq!(have_storage_value(module, map_name, &key_2), true);
				assert_eq!(get_storage_value(module, map_name, &key_1), Some(value.clone()));
				assert_eq!(get_storage_value(module, map_name, &key_2), Some(value.clone()));

				// Moving Map Storage from one place to another within the same pallet.
				// NOTE: The value at the key `from_prefix` is not moved !!!
				// Doesn't with with storage value!
				let new_map_name = b"NewMyMap";
				let from = storage_prefix(module, map_name);
				let to = storage_prefix(module, new_map_name);

				move_prefix(&from, &to);
				assert_eq!(have_storage_value(module, map_name, &key_1), false);
				assert_eq!(have_storage_value(module, map_name, &key_2), false);
				assert_eq!(have_storage_value(module, new_map_name, &key_1), true);
				assert_eq!(have_storage_value(module, new_map_name, &key_2), true);

				// Moving Storage Value from one place to another within the same pallet.
				let new_value_name = b"NewMyValue";
				let value: OldType = take_storage_value(module, value_name, b"").unwrap();
				put_storage_value(module, new_value_name, b"", value);
				assert_eq!(have_storage_value(module, value_name, b""), false);
				assert_eq!(have_storage_value(module, new_value_name, b""), true);

				// Move storage from one pallet to another pallet
				let new_module = b"newpallet";
				move_storage_from_pallet(new_map_name, &module, new_module); // Map
				move_storage_from_pallet(new_value_name, &module, new_module); // Value
				assert_eq!(have_storage_value(module, new_map_name, &key_1), false);
				assert_eq!(have_storage_value(module, new_map_name, &key_2), false);
				assert_eq!(have_storage_value(new_module, new_map_name, &key_1), true);
				assert_eq!(have_storage_value(new_module, new_map_name, &key_2), true);
				assert_eq!(have_storage_value(module, new_value_name, b""), false);
				assert_eq!(have_storage_value(new_module, new_value_name, b""), true);

				// Rename pallet
				let different_module = b"newnewpallet";
				move_pallet(new_module, different_module);
				assert_eq!(have_storage_value(new_module, new_map_name, &key_1), false);
				assert_eq!(have_storage_value(new_module, new_map_name, &key_2), false);
				assert_eq!(have_storage_value(different_module, new_map_name, &key_1), true);
				assert_eq!(have_storage_value(different_module, new_map_name, &key_2), true);
				assert_eq!(have_storage_value(new_module, new_value_name, b""), false);
				assert_eq!(have_storage_value(different_module, new_value_name, b""), true);

				// Remove Value
				assert_eq!(have_storage_value(different_module, new_value_name, b""), true);
				_ = clear_storage_prefix(different_module, new_value_name, b"", None, None);
				assert_eq!(have_storage_value(different_module, new_value_name, b""), false);

				// Remove Map
				assert_eq!(have_storage_value(different_module, new_map_name, &key_1), true);
				assert_eq!(have_storage_value(different_module, new_map_name, &key_2), true);
				_ = clear_storage_prefix(different_module, new_map_name, b"", None, None);
				assert_eq!(have_storage_value(different_module, new_map_name, &key_1), false);
				assert_eq!(have_storage_value(different_module, new_map_name, &key_2), false);

				// Additional functionality to check out if necessary
				// storage_key_iter
				// storage_iter
				// take_storage_item
			});
		}
	}
}
