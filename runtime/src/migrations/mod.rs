mod example;
mod mock;

use codec::{Decode, Encode, FullCodec, FullEncode};
use frame_support::{
	migration::{
		clear_storage_prefix, get_storage_value, have_storage_value, move_pallet, move_prefix,
		move_storage_from_pallet, put_storage_value, storage_key_iter, take_storage_value,
	},
	storage::storage_prefix,
	traits::OnRuntimeUpgrade,
	weights::Weight,
	ReversibleStorageHasher,
};
use sp_std::{fmt::Debug, vec::Vec};

pub struct AllMigrations;
impl OnRuntimeUpgrade for AllMigrations {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		example::Upgrade::pre_upgrade()?;

		Ok(())
	}

	fn on_runtime_upgrade() -> Weight {
		let weight = Weight::from(0u32);
		//weight += example::Upgrade::on_runtime_upgrade();

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		example::Upgrade::post_upgrade()?;

		Ok(())
	}
}

pub struct Value;
impl Value {
	// Checks explicitly if a storage exist. Use this to check if storage has been actually written.
	// Just calling Storage::try_get() might return the default value in case the storage doesn't
	// exist.
	#[allow(dead_code)]
	pub fn exists<Storage, T>() -> bool
	where
		T: FullCodec,
		Storage: frame_support::storage::StorageValue<T>,
	{
		Storage::exists()
	}

	#[allow(dead_code)]
	pub fn unsafe_exists(module: &[u8], item: &[u8]) -> bool {
		have_storage_value(module, item, b"")
	}

	#[allow(dead_code)]
	pub fn valid<Storage, T>() -> Result<T, ()>
	where
		T: FullCodec,
		Storage: frame_support::storage::StorageValue<T>,
	{
		Storage::try_get()
	}

	#[allow(dead_code)]
	pub fn exists_valid<Storage, T>() -> Result<T, ()>
	where
		T: FullCodec,
		Storage: frame_support::storage::StorageValue<T>,
	{
		if !Self::exists::<Storage, T>() {
			return Err(())
		}

		Self::valid::<Storage, T>()
	}

	#[allow(dead_code)]
	pub fn clear<Storage, T>() -> Result<(), ()>
	where
		T: FullCodec,
		Storage: frame_support::storage::StorageValue<T>,
	{
		if Storage::exists() {
			Storage::kill();
			Ok(())
		} else {
			Err(())
		}
	}

	#[allow(dead_code)]
	pub fn unsafe_clear(module: &[u8], item: &[u8]) -> sp_io::MultiRemovalResults {
		clear_storage_prefix(module, item, b"", None, None)
	}

	#[allow(dead_code)]
	pub fn translate<Storage, OldT, NewT>(f: fn(OldT) -> NewT) -> Result<(), ()>
	where
		OldT: FullCodec,
		NewT: FullCodec,
		Storage: frame_support::storage::StorageValue<NewT>,
	{
		let res = Storage::translate::<OldT, _>(|old_data| {
			if let Some(data) = old_data {
				return Some(f(data))
			}

			None
		});

		if let Err(_) = res {
			return Err(())
		}

		Ok(())
	}

	#[allow(dead_code)]
	pub fn unsafe_rename<T>(module: &[u8], old_item: &[u8], new_item: &[u8]) -> bool
	where
		T: Decode + Sized + Encode,
	{
		let Some(value) = take_storage_value::<T>(module, old_item, b"") else {
			return false;
		};
		put_storage_value(module, new_item, b"", value);
		true
	}

	#[allow(dead_code)]
	pub fn unsafe_move(storage_name: &[u8], old_pallet_name: &[u8], new_pallet_name: &[u8]) {
		move_storage_from_pallet(storage_name, old_pallet_name, new_pallet_name);
	}

	#[allow(dead_code)]
	pub fn unsafe_get<T>(module: &[u8], item: &[u8]) -> Option<T>
	where
		T: Decode + Sized,
	{
		get_storage_value::<T>(module, item, b"")
	}

	#[allow(dead_code)]
	pub fn unsafe_put<T>(module: &[u8], item: &[u8], value: T)
	where
		T: Encode,
	{
		put_storage_value::<T>(module, item, b"", value)
	}
}

pub struct Map;
impl Map {
	// If no keys exist it means nothing is written in the stoage
	#[allow(dead_code)]
	pub fn exists<Storage, K, V>() -> bool
	where
		K: FullEncode,
		V: FullCodec,
		Storage: frame_support::storage::StorageMap<K, V>
			+ frame_support::storage::IterableStorageMap<K, V>,
	{
		Storage::iter_keys().count() != 0
	}

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
	pub fn valid<Storage, K, V>() -> Result<usize, K>
	where
		K: FullEncode,
		V: FullCodec,
		Storage: frame_support::storage::StorageMap<K, V>
			+ frame_support::storage::IterableStorageMap<K, V>,
	{
		let keys: Vec<K> = Storage::iter_keys().collect();
		let keys_len: usize = keys.len();
		for key in keys {
			if let Err(_) = Storage::try_get(&key) {
				return Err(key)
			}
		}

		Ok(keys_len)
	}

	#[allow(dead_code)]
	pub fn exists_valid<Storage, K, V>() -> Result<usize, Option<K>>
	where
		K: FullEncode,
		V: FullCodec,
		Storage: frame_support::storage::StorageMap<K, V>
			+ frame_support::storage::IterableStorageMap<K, V>,
	{
		if !Self::exists::<Storage, K, V>() {
			return Err(None)
		}

		match Self::valid::<Storage, K, V>() {
			Ok(len) => Ok(len),
			Err(key) => Err(Some(key)),
		}
	}

	#[allow(dead_code)]
	pub fn clear<Storage, K, V>() -> Result<(), ()>
	where
		K: FullEncode,
		V: FullCodec,
		Storage: frame_support::storage::StorageMap<K, V>
			+ frame_support::storage::StoragePrefixedMap<V>,
	{
		let res = Storage::clear(u32::MAX, None);
		if res.maybe_cursor.is_some() {
			Ok(())
		} else {
			Err(())
		}
	}

	#[allow(dead_code)]
	pub fn unsafe_clear(module: &[u8], item: &[u8]) -> sp_io::MultiRemovalResults {
		clear_storage_prefix(module, item, b"", None, None)
	}

	#[allow(dead_code)]
	pub fn translate<OldStorage, NewStorage, OldK, OldV, NewK, NewV>(
		f: fn(OldK, OldV) -> (NewK, NewV),
	) -> Result<usize, (usize, usize)>
	where
		OldStorage: frame_support::storage::StorageMap<OldK, OldV>
			+ frame_support::storage::IterableStorageMap<OldK, OldV>
			+ frame_support::storage::StoragePrefixedMap<OldV>,
		NewStorage: frame_support::storage::StorageMap<NewK, NewV>
			+ frame_support::storage::IterableStorageMap<NewK, NewV>
			+ frame_support::storage::StoragePrefixedMap<NewV>,
		OldK: FullEncode + Debug + Clone,
		OldV: FullCodec,
		NewK: FullEncode,
		NewV: FullCodec,
	{
		let original_count = OldStorage::iter_keys().count();
		let keys_values: Vec<(OldK, OldV)> = OldStorage::iter_keys()
			.filter_map(|key| {
				if let Ok(value) = OldStorage::try_get(key.clone()) {
					return Some((key, value))
				} else {
					log::error!("Removed undecodable value: {:?}", key);
					return None
				}
			})
			.collect();

		// Delete whole storage
		let res = OldStorage::clear(u32::MAX, None);
		if res.maybe_cursor.is_some() {
			log::error!("Should not happen");
		} else {
			log::info!("All good with remove storage map");
		};

		// Translate
		for (old_key, old_value) in keys_values {
			let (new_key, new_value) = f(old_key, old_value);
			NewStorage::insert(new_key, new_value);
		}

		let new_count = NewStorage::iter_keys().count();
		if original_count != new_count {
			return Err((original_count, new_count))
		}

		Ok(new_count)
	}

	#[allow(dead_code)]
	pub fn unsafe_rename(module: &[u8], old_item: &[u8], new_item: &[u8]) {
		let from = storage_prefix(module, old_item);
		let to = storage_prefix(module, new_item);

		move_prefix(&from, &to);
	}

	#[allow(dead_code)]
	pub fn unsafe_move(storage_name: &[u8], old_pallet_name: &[u8], new_pallet_name: &[u8]) {
		move_storage_from_pallet(storage_name, old_pallet_name, new_pallet_name);
	}

	#[allow(dead_code)]
	pub fn unsafe_get<T>(module: &[u8], item: &[u8], key: &[u8]) -> Option<T>
	where
		T: Decode + Sized,
	{
		get_storage_value::<T>(module, item, key)
	}

	#[allow(dead_code)]
	pub fn unsafe_put<T>(module: &[u8], item: &[u8], key: &[u8], value: T)
	where
		T: Encode,
	{
		put_storage_value::<T>(module, item, key, value)
	}
}

pub struct Pallet;
impl Pallet {
	pub fn unsafe_move_pallet(old_pallet_name: &[u8], new_pallet_name: &[u8]) {
		move_pallet(old_pallet_name, new_pallet_name)
	}
}

#[cfg(test)]
mod tests {
	use crate::{Runtime, System};

	pub fn new_test_ext() -> sp_io::TestExternalities {
		let t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
