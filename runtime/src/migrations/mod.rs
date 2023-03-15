mod example;
mod mock;

use codec::{FullCodec, FullEncode};
use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};
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

// Checks explicitly if a storage exist. Use this to check if storage has been actually written.
// Just calling Storage::try_get() might return the default value in case the storage doesn't exist.
#[allow(dead_code)]
fn value_exists<Storage, T>() -> bool
where
	T: FullCodec,
	Storage: frame_support::storage::StorageValue<T>,
{
	Storage::exists()
}

// If no keys exist it means nothing is written in the stoage 
#[allow(dead_code)]
fn map_exists<Storage, K, V>() -> bool
where
	K: FullEncode,
	V: FullCodec,
	Storage:
		frame_support::storage::StorageMap<K, V> + frame_support::storage::IterableStorageMap<K, V>,
{
	Storage::iter_keys().count() != 0
}

#[allow(dead_code)]
fn value_valid<Storage, T>() -> Result<T, ()>
where
	T: FullCodec,
	Storage: frame_support::storage::StorageValue<T>,
{
	Storage::try_get()
}

#[allow(dead_code)]
fn map_valid<Storage, K, V>() -> Result<usize, K>
where
	K: FullEncode,
	V: FullCodec,
	Storage:
		frame_support::storage::StorageMap<K, V> + frame_support::storage::IterableStorageMap<K, V>,
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
fn value_exists_valid<Storage, T>() -> Result<T, ()>
where
	T: FullCodec,
	Storage: frame_support::storage::StorageValue<T>,
{
	if !value_exists::<Storage, T>() {
		return Err(())
	}

	value_valid::<Storage, T>()
}

#[allow(dead_code)]
fn map_exists_valid<Storage, K, V>() -> Result<usize, Option<K>>
where
	K: FullEncode,
	V: FullCodec,
	Storage:
		frame_support::storage::StorageMap<K, V> + frame_support::storage::IterableStorageMap<K, V>,
{
	if !map_exists::<Storage, K, V>() {
		return Err(None)
	}

	match map_valid::<Storage, K, V>() {
		Ok(len) => Ok(len),
		Err(key) => Err(Some(key)),
	}
}

#[allow(dead_code)]
fn remove_value<Storage, T>() -> Result<(), ()>
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
fn remove_map<Storage, K, V>() -> Result<(), ()>
where
	K: FullEncode,
	V: FullCodec,
	Storage:
		frame_support::storage::StorageMap<K, V> + frame_support::storage::StoragePrefixedMap<V>,
{
	let res = Storage::clear(u32::MAX, None);
	if res.maybe_cursor.is_some() {
		Ok(())
	} else {
		Err(())
	}
}

#[allow(dead_code)]
fn translate_value<Storage, OldT, NewT>(f: fn(OldT) -> NewT) -> Result<(), ()>
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
fn translate_map<OldStorage, NewStorage, OldK, OldV, NewK, NewV>(
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
