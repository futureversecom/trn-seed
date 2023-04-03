mod fee_control;

use codec::{Decode, Encode, FullCodec};
use frame_support::{
	migration::{get_storage_value, have_storage_value, put_storage_value},
	traits::OnRuntimeUpgrade,
	weights::Weight,
};

pub struct AllMigrations;
impl OnRuntimeUpgrade for AllMigrations {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		fee_control::Upgrade::pre_upgrade()?;

		Ok(())
	}

	fn on_runtime_upgrade() -> Weight {
		let mut weight = Weight::from(0u32);
		weight += fee_control::Upgrade::on_runtime_upgrade();

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		fee_control::Upgrade::post_upgrade()?;

		Ok(())
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
			return Err(())
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
	/// - `T`: The type of the value stored in `Storage`. Should be set to to the correct type
	///   otherwise None will be returned.
	///
	/// # Usage
	///
	/// let (module, item) = (b"MyPallet", b"MyStorageName");
	/// Value::unsafe_storage_put(module, item, 100u32);
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
	/// - `T`: The type of the value stored in `Storage`.
	///
	/// # Usage
	///
	///	// To check for a exact value we can use assert_eq.
	/// assert_eq!(Value::storage_get::<my_pallet::MyStorage::<Runtime>, _>(), Ok(expected_value));
	///
	///	// To verify that the storage is not corrupted without checking the exact value, use
	/// // assert
	/// assert!(Value::storage_get::<my_pallet::MyStorage::<Runtime>, _>().is_some());
	/// assert!(Value::storage_get::<MyStorage::<Runtime>, _>().is_none());
	#[allow(dead_code)]
	pub fn unsafe_storage_put<T>(module: &[u8], item: &[u8], value: T)
	where
		T: Encode,
	{
		put_storage_value::<T>(module, item, b"", value)
	}
}

#[cfg(test)]
mod value_tests {
	use super::{tests::new_test_ext, *};
	use crate::Runtime;
	use frame_support::{
		storage::generator::StorageValue as StorageValuePrefix, storage_alias, StorageValue,
	};

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

			// Calling get on existing storage with wrong type should return None.
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

			// Calling get on existing storage with wrong type should return None.
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
}
