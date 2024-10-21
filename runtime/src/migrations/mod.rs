// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

mod xrpl_bridge;

use codec::{Decode, Encode, FullCodec, FullEncode};
use frame_support::{
	migration::{
		clear_storage_prefix, get_storage_value, have_storage_value, move_prefix,
		move_storage_from_pallet, put_storage_value, storage_key_iter, take_storage_value,
	},
	storage::storage_prefix,
	traits::OnRuntimeUpgrade,
	weights::Weight,
	ReversibleStorageHasher,
};
#[cfg(feature = "try-runtime")]
use sp_runtime::DispatchError;
use sp_std::vec::Vec;

pub struct AllMigrations;
impl OnRuntimeUpgrade for AllMigrations {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
		xrpl_bridge::Upgrade::pre_upgrade()
	}

	fn on_runtime_upgrade() -> Weight {
		xrpl_bridge::Upgrade::on_runtime_upgrade()
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), DispatchError> {
		xrpl_bridge::Upgrade::post_upgrade(state.clone())
	}
}

#[cfg(test)]
mod tests {
	use crate::{Runtime, System};
	use sp_core::H160;
	use sp_runtime::BuildStorage;

	#[allow(dead_code)]
	pub fn create_account<AccountId: From<H160>>(seed: u64) -> AccountId {
		AccountId::from(H160::from_low_u64_be(seed))
	}

	pub fn new_test_ext() -> sp_io::TestExternalities {
		let t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

pub struct Value;
impl Value {
	/// Checks if a value explicitly exist in storage.
	///
	/// This function is helpful for determining whether a storage value has been explicitly set, as
	/// calling `get()` or `try_get()` might return the default value, giving no indication whether
	/// the storage value is actually allocated or not.
	///
	/// # Additional info
	///
	/// To check if the data is corrupted or not, you can call `storage_get()`.
	///
	/// # Type Parameters
	///
	/// - `Storage`: The storage item to check for existence.
	/// - `T`: The type of the value stored in `Storage`. Should be set to `_`
	///
	/// # Usage
	///
	/// assert_eq!(Value::exists::<MyStorage::<Runtime>, _>(), false);
	/// assert_eq!(Value::exists::<my_pallet::MyStorage::<Runtime>, _>(), true);
	#[allow(dead_code)]
	pub fn exists<Storage, T>() -> bool
	where
		T: FullCodec,
		Storage: frame_support::storage::StorageValue<T>,
	{
		Storage::exists()
	}

	/// This function has two roles:
	/// 1. It returns a value explicitly from the storage.
	/// 2. It checks if the storage is corrupted or not.
	///
	/// The function returns a Result with the retrieved value if successful or an Err in the
	/// following cases:
	/// - If a value is not explicitly stored in the storage.
	/// - If a value is stored in the storage, but it is corrupted (different type/size).
	///
	/// # Type Parameters
	///
	/// - `Storage`: The storage item to check for existence.
	/// - `T`: The type of the value stored in `Storage`. Should be set to `_`
	///
	/// # Usage
	///
	/// // To check for a exact value we can use assert_eq.
	/// assert_eq!(Value::storage_get::<my_pallet::MyStorage::<Runtime>, _>(), Ok(expected_value));
	///
	/// // To verify that the storage is not corrupted without checking the exact value, use
	/// // assert_ok or assert.
	//	assert_ok!(Value::storage_get::<my_pallet::MyStorage::<Runtime>,_>());
	/// assert!(Value::storage_get::<MyStorage::<Runtime>, _>().is_err());
	#[allow(dead_code)]
	pub fn storage_get<Storage, T>() -> Result<T, ()>
	where
		T: FullCodec,
		Storage: frame_support::storage::StorageValue<T>,
	{
		if !Self::exists::<Storage, T>() {
			return Err(());
		}

		Storage::try_get()
	}

	/// Checks if a value explicitly exist in storage.
	///
	/// This function is unsafe since the caller is responsible for passing the right module and
	/// item name. For the safe/typed approach check out the `exists` function
	///
	/// # Additional info
	///
	/// To see if the data is corrupted or not you can call unsafe_storage_get.
	///
	/// # Usage
	///
	/// let (module, item) = (b"MyPallet", b"MyStorageName");
	/// assert_eq!(Value::unsafe_exists(module, item), false);
	#[allow(dead_code)]
	pub fn unsafe_exists(module: &[u8], item: &[u8]) -> bool {
		have_storage_value(module, item, b"")
	}

	/// This function has two roles:
	/// 1. It returns a value explicitly from the storage
	/// 2. It checks if the storage is corrupted or not
	///
	/// This function is unsafe since the caller is responsible for passing the right module and
	/// item name. For the safe/typed approach check out the `storage_get` function
	///
	/// The function returns a Some with the retrieved value if successful or an None in the
	/// following cases:
	/// - If a value is not explicitly stored in the storage.
	/// - If a value is stored in the storage, but it is corrupted (different type/size).
	///
	/// # Type Parameters
	///
	/// - `T`: The type of the value stored in storage. Should be set to to the correct type
	///   otherwise None will be returned.
	///
	/// # Usage
	///
	/// let (module, item) = (b"MyPallet", b"MyStorageName");
	/// Value::unsafe_storage_get(module, item);
	#[allow(dead_code)]
	pub fn unsafe_storage_get<T>(module: &[u8], item: &[u8]) -> Option<T>
	where
		T: Decode + Sized,
	{
		get_storage_value::<T>(module, item, b"")
	}

	/// Inserts a value to a specific storage location
	///
	/// This function is unsafe since the caller is responsible for passing the right module and
	/// item name.
	///
	/// # Type Parameters
	///
	/// - `T`: The type of the value stored in storage.
	///
	/// # Usage
	///
	/// let (module, item) = (b"MyPallet", b"MyStorageName");
	/// Value::unsafe_storage_put(module, item, 100u128);
	#[allow(dead_code)]
	pub fn unsafe_storage_put<T>(module: &[u8], item: &[u8], value: T)
	where
		T: Encode,
	{
		put_storage_value::<T>(module, item, b"", value)
	}

	/// Changes the name of an existing storage value.
	///
	/// This function is unsafe since the caller is responsible for passing the right module and
	/// item name.
	///
	/// The function return false in two situations:
	/// - If a value is not explicitly stored in the storage.
	/// - If a value is stored in the storage, but it is corrupted (different type/size).
	///
	///
	/// # Type Parameters
	///
	/// - `T`: The type of the value stored in storage. Should be set to to the correct type
	///   otherwise corruption can occur.
	///
	/// # Usage
	///
	///	let (module, item, new_item) = (b"MyPallet", b"MyStorageName", b"NewStorage");
	/// Value::unsafe_storage_put(module, item, 100u128);
	/// assert_eq!(Value::unsafe_storage_rename::<u128>(module, item, new_item), true);
	///
	/// // Renaming a non-existing storage will return false
	/// assert_eq!(Value::unsafe_storage_rename::<u128>(module, b"ThisDoesNotExist", new_item),
	/// false);
	#[allow(dead_code)]
	pub fn unsafe_storage_rename<T>(module: &[u8], old_item: &[u8], new_item: &[u8]) -> bool
	where
		T: Decode + Sized + Encode,
	{
		let Some(value) = take_storage_value::<T>(module, old_item, b"") else {
			return false;
		};
		put_storage_value(module, new_item, b"", value);
		true
	}

	/// Moves a storage value from one pallet to another one.
	///
	/// This function is unsafe since the caller is responsible for passing the right pallet and
	/// storage name.
	///
	/// The function return false If the value is not explicitly stored in the storage.
	///
	/// # Usage
	///
	///	let (storage, pallet, new_pallet) = (b"MyStorageName", b"MyPallet", b"MyNewPallet");
	/// Value::unsafe_storage_put(pallet, storage, 100u128);
	/// assert_eq!(Value::unsafe_storage_move(storage, pallet, new_pallet), true);
	///
	/// // moving a non-existing storage will return false
	/// assert_eq!(Value::unsafe_storage_move(b"RandomStorage", pallet, new_pallet), false)
	#[allow(dead_code)]
	pub fn unsafe_storage_move(
		storage_name: &[u8],
		old_pallet_name: &[u8],
		new_pallet_name: &[u8],
	) -> bool {
		if !Self::unsafe_exists(old_pallet_name, storage_name) {
			return false;
		}

		move_storage_from_pallet(storage_name, old_pallet_name, new_pallet_name);
		true
	}

	/// Kills the storage value
	///
	/// This function is unsafe since the caller is responsible for passing the right module and
	/// item name.
	///
	/// The function return false If no value is not explicitly stored in the storage.
	///
	/// # Usage
	///
	///	/// let (module, item) = (b"MyPallet", b"MyStorageName");
	/// Value::unsafe_storage_put(module, item, 100u128);
	/// assert_eq!(Value::unsafe_clear(module, item), true);
	///
	/// // killing a non-existing storage will return false
	/// assert_eq!(Value::unsafe_clear(module, b"DoesNotExist"), false);
	#[allow(dead_code)]
	pub fn unsafe_clear(module: &[u8], item: &[u8]) -> bool {
		if !Self::unsafe_exists(module, item) {
			return false;
		}

		clear_storage_prefix(module, item, b"", None, None).maybe_cursor.is_none()
	}
}

pub struct Map;
impl Map {
	#[allow(dead_code)]
	pub fn unsafe_exists<K, T, H>(module: &[u8], item: &[u8]) -> bool
	where
		K: Decode + Sized,
		T: Decode + Sized,
		H: ReversibleStorageHasher,
	{
		storage_key_iter::<K, T, H>(module, item).count() > 0
	}

	#[allow(dead_code)]
	pub fn unsafe_elem_exists(module: &[u8], item: &[u8], hash: &[u8]) -> bool {
		have_storage_value(module, item, hash)
	}

	#[allow(dead_code)]
	pub fn unsafe_clear(module: &[u8], item: &[u8]) -> bool {
		clear_storage_prefix(module, item, b"", None, None).maybe_cursor.is_none()
	}

	#[allow(dead_code)]
	pub fn unsafe_storage_rename<K, T, H>(module: &[u8], old_item: &[u8], new_item: &[u8]) -> bool
	where
		K: Decode + Sized,
		T: Decode + Sized,
		H: ReversibleStorageHasher,
	{
		if !Self::unsafe_exists::<K, T, H>(module, old_item) {
			return false;
		}

		let from = storage_prefix(module, old_item);
		let to = storage_prefix(module, new_item);

		move_prefix(&from, &to);
		true
	}

	#[allow(dead_code)]
	pub fn unsafe_storage_move<K, T, H>(
		storage_name: &[u8],
		old_pallet_name: &[u8],
		new_pallet_name: &[u8],
	) -> bool
	where
		K: Decode + Sized,
		T: Decode + Sized,
		H: ReversibleStorageHasher,
	{
		if !Self::unsafe_exists::<K, T, H>(old_pallet_name, storage_name) {
			return false;
		}

		move_storage_from_pallet(storage_name, old_pallet_name, new_pallet_name);
		true
	}

	#[allow(dead_code)]
	pub fn unsafe_storage_get<T>(module: &[u8], item: &[u8], hash: &[u8]) -> Option<T>
	where
		T: Decode + Sized,
	{
		get_storage_value::<T>(module, item, hash)
	}

	#[allow(dead_code)]
	pub fn unsafe_storage_take<T>(module: &[u8], item: &[u8], hash: &[u8]) -> Option<T>
	where
		T: Decode + Sized,
	{
		take_storage_value::<T>(module, item, hash)
	}

	#[allow(dead_code)]
	pub fn unsafe_keys_get<K, T, H>(module: &[u8], item: &[u8]) -> Vec<K>
	where
		K: Decode + Sized,
		T: Decode + Sized,
		H: ReversibleStorageHasher,
	{
		storage_key_iter::<K, T, H>(module, item).map(|key_value| key_value.0).collect()
	}

	#[allow(dead_code)]
	pub fn unsafe_storage_put<T>(module: &[u8], item: &[u8], hash: &[u8], value: T)
	where
		T: Encode,
	{
		put_storage_value::<T>(module, item, hash, value)
	}

	#[allow(dead_code)]
	pub fn iter<Storage, K, V>() -> Vec<(K, V)>
	where
		Storage: frame_support::storage::StorageMap<K, V>
			+ frame_support::storage::IterableStorageMap<K, V>,
		K: FullEncode + Clone,
		V: FullCodec,
	{
		let keys: Vec<K> = Storage::iter_keys().collect();
		keys.iter()
			.filter_map(|key| Storage::try_get(key).and_then(|v| Ok((key.clone(), v))).ok())
			.collect()
	}
}

#[cfg(test)]
mod map_tests {
	/* 	use super::{tests::new_test_ext, *};
	use crate::Runtime;
	use frame_support::{
		storage::generator::StorageValue as StorageValuePrefix, storage_alias, Hashable,
		Twox64Concat,
	}; */

	// TODO Tests to be added in the next PR
	/* 	#[storage_alias]
	pub type MyStorage<T: pallet_fee_control::Config> =
		StorageValue<pallet_fee_control::Pallet<T>, u32>;

	#[test]
	fn abba() {
		new_test_ext().execute_with(|| {
			let (module, item, key) = (b"Module", b"Item", 1u32.twox_64_concat());

			assert_eq!(Map::unsafe_exists::<u32, u32, Twox64Concat>(module, item), false);
			assert_eq!(Map::unsafe_storage_get::<u32>(module, item, &key), None);
			Map::unsafe_storage_put(module, item, &key, 100u32);
			assert_eq!(Map::unsafe_exists::<u32, u32, Twox64Concat>(module, item), true);
			assert_eq!(Map::unsafe_storage_get::<u32>(module, item, &key), Some(100u32));
			assert_eq!(Map::unsafe_storage_get::<u128>(module, item, &key), None);

			assert_eq!(Map::unsafe_keys_get::<u32, u32, Twox64Concat>(module, item), vec![1u32]);

			let key_2 = 2u32.twox_64_concat();
			Map::unsafe_storage_put(module, item, &key_2, 2u8);
			assert_eq!(Map::unsafe_keys_get::<u32, u32, Twox64Concat>(module, item), vec![1u32]);
		});
	} */
}

#[cfg(test)]
mod value_tests {
	use super::{tests::new_test_ext, *};
	use crate::Runtime;
	use frame_support::{storage::generator::StorageValue as StorageValuePrefix, storage_alias};

	#[storage_alias]
	pub type MyStorage<T: pallet_fee_control::Config> =
		StorageValue<pallet_fee_control::Pallet<T>, u32>;

	#[test]
	fn exists() {
		new_test_ext().execute_with(|| {
			// Calling exists on non-existing storage should have no effect and return false.
			assert_eq!(Value::exists::<MyStorage::<Runtime>, _>(), false);

			// Calling exists on existing storage should return true.
			MyStorage::<Runtime>::put(100u32);
			assert_eq!(Value::exists::<MyStorage::<Runtime>, _>(), true);
		});
	}

	#[test]
	fn unsafe_exists() {
		new_test_ext().execute_with(|| {
			let (module, item) = (b"Module", b"Item");

			// Calling exists on non-existing storage should have no effect and return false.
			assert_eq!(Value::unsafe_exists(module, item), false);

			// Calling exists on existing storage should return true.
			Value::unsafe_storage_put(module, item, 100u32);
			assert_eq!(Value::unsafe_exists(module, item), true);
		});
	}

	#[test]
	fn storage_get() {
		new_test_ext().execute_with(|| {
			let (module, item, value) = (
				MyStorage::<Runtime>::module_prefix(),
				MyStorage::<Runtime>::storage_prefix(),
				100u32,
			);

			// Calling get on non-existing storage should have no effect and return None.
			assert_eq!(Value::exists::<MyStorage::<Runtime>, _>(), false);
			assert!(Value::storage_get::<MyStorage::<Runtime>, _>().is_err());

			// Calling get on existing storage should return the value.
			MyStorage::<Runtime>::put(100u32);
			assert_eq!(Value::storage_get::<MyStorage::<Runtime>, _>(), Ok(value));

			// Calling get on existing storage with wrong type might return None.
			// Here we will intentionally corrupt the data!
			Value::unsafe_storage_put(module, item, true);
			assert_eq!(Value::exists::<MyStorage::<Runtime>, _>(), true);
			assert!(Value::storage_get::<MyStorage::<Runtime>, _>().is_err());
		});
	}

	#[test]
	fn unsafe_storage_get() {
		new_test_ext().execute_with(|| {
			let (module, item, value) = (b"Module", b"Item", 100u32);

			// Calling get on non-existing storage should have no effect and return None.
			assert_eq!(Value::unsafe_exists(module, item), false);
			assert_eq!(Value::unsafe_storage_get::<u32>(module, item), None);

			// Calling get on existing storage should return the value.
			Value::unsafe_storage_put(module, item, value);
			assert_eq!(Value::unsafe_storage_get(module, item), Some(value));

			// Calling get on existing storage with wrong type might return None.
			assert_eq!(Value::unsafe_exists(module, item), true);
			assert_eq!(Value::unsafe_storage_get::<u128>(module, item), None);
		});
	}

	#[test]
	fn unsafe_storage_put() {
		new_test_ext().execute_with(|| {
			let (module, item) = (b"Module", b"Item");

			// Calling put on non-existing storage creates a new storage entry.
			let value = 100u32;
			assert_eq!(Value::unsafe_exists(module, item), false);
			Value::unsafe_storage_put(module, item, value);
			assert_eq!(Value::unsafe_storage_get(module, item), Some(value));

			// Calling put on existing storage updates the existing storage entry.
			let value_2 = 200u32;
			Value::unsafe_storage_put(module, item, value_2);
			assert_eq!(Value::unsafe_storage_get(module, item), Some(value_2));

			// Calling put on existing storage with a different data size updates the existing
			// storage entry.
			let value_3 = 300u128;
			Value::unsafe_storage_put(module, item, value_3);
			assert_eq!(Value::unsafe_storage_get(module, item), Some(value_3));
		});
	}

	#[test]
	fn unsafe_storage_rename() {
		new_test_ext().execute_with(|| {
			let (module, item, new_item) = (b"Module", b"Item", b"NewItem");

			// Calling rename on non-existing storage should have no effect and return false.
			assert_eq!(Value::unsafe_storage_rename::<u32>(module, item, new_item), false);

			// Calling rename on existing storage should rename the storage.
			let value = 200u32;
			Value::unsafe_storage_put(module, item, value);
			assert_eq!(Value::unsafe_storage_rename::<u32>(module, item, new_item), true);
			assert_eq!(Value::unsafe_exists(module, item), false);
			assert_eq!(Value::unsafe_storage_get::<u32>(module, new_item), Some(value));

			// Calling rename on existing storage with a different data size might return false.
			assert_eq!(Value::unsafe_clear(module, new_item), true);
			Value::unsafe_storage_put(module, item, value);
			assert_eq!(Value::unsafe_storage_rename::<u128>(module, item, new_item), false);
			assert_eq!(Value::unsafe_exists(module, item), true);
		});
	}

	#[test]
	fn unsafe_storage_move() {
		new_test_ext().execute_with(|| {
			let (storage, pallet, new_pallet) = (b"Item", b"Pallet", b"NewPallet");

			// Calling move on non-existing storage should have no effect and return false.
			assert_eq!(Value::unsafe_storage_move(storage, pallet, new_pallet), false);

			// Calling move on existing storage should rename the storage.
			let value = 200u32;
			Value::unsafe_storage_put(pallet, storage, value);
			assert_eq!(Value::unsafe_storage_move(storage, pallet, new_pallet), true);
			assert_eq!(Value::unsafe_exists(pallet, storage), false);
			assert_eq!(Value::unsafe_storage_get::<u32>(new_pallet, storage), Some(value));
		});
	}

	#[test]
	fn unsafe_clear() {
		new_test_ext().execute_with(|| {
			let (module, item) = (b"Module", b"Item");

			// Calling clear on non-existing storage should have no effect and return false.
			assert_eq!(Value::unsafe_clear(module, item), false);

			// Calling clear on existing storage should kill the storage.
			Value::unsafe_storage_put(module, item, 200u32);
			assert_eq!(Value::unsafe_exists(module, item), true);
			assert_eq!(Value::unsafe_clear(module, item), true);
			assert_eq!(Value::unsafe_exists(module, item), false);
		});
	}
}

#[cfg(all(test, feature = "try-runtime"))]
mod remote_tests {
	use super::*;
	use crate::{migrations::AllMigrations, Block};
	use frame_remote_externalities::{Builder, Mode, OfflineConfig};
	use std::env::var;

	#[tokio::test]
	#[ignore]
	async fn run_migrations() {
		//std::env::set_var("SNAP", "/full/path/to/snap.top");
		let Some(state_snapshot) = var("SNAP").map(|s| s.into()).ok() else {
			return;
		};
		let mode = Mode::Offline(OfflineConfig { state_snapshot });
		let mut ext = Builder::<Block>::default().mode(mode).build().await.unwrap();
		ext.execute_with(|| {
			AllMigrations::pre_upgrade().unwrap();
			AllMigrations::on_runtime_upgrade();
			AllMigrations::post_upgrade(vec![]).unwrap();
		});
	}
}
