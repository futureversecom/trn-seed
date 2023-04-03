mod fee_control;

use codec::FullCodec;
use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};

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
	/// This is helpful since calling get() or try_get() might return the default value which gives
	/// no indication if an actual storage is allocated or not.
	///
	/// Additional info:
	/// 	To see if the data is corrupted or not you can call storage_get.
	///
	/// Usage:
	/// 	assert_eq!(Value::exists::<MyStorage::<Runtime>, _>(), false);
	/// 	assert_eq!(Value::exists::<my_pallet::MyStorage::<Runtime>, _>(), true);
	#[allow(dead_code)]
	pub fn exists<Storage, T>() -> bool
	where
		T: FullCodec,
		Storage: frame_support::storage::StorageValue<T>,
	{
		Storage::exists()
	}

	/// This function has two roles:
	/// 1. It returns a value explicitly from the storage
	/// 2. It checks if the storage is corrupted or not
	///
	/// If a value is not explicitly stored in the storage it will return an Err.
	/// If a value is stored in the storage but it is corrupted (of different type/size) it will
	/// return an Err.
	///
	/// Usage:
	/// 	// To get the exact value we can use assert_eq
	/// 	assert_eq!(Value::storage_get::<my_pallet::MyStorage::<Runtime>, _>(), Ok(expected_value));
	///
	/// 	// If we don't care what's the value and just wanna know that the storage is not corrupted
	/// 	assert_ok!(Value::storage_get::<my_pallet::MyStorage::<Runtime>, _>());
	/// 	assert_err!(Value::storage_get::<MyStorage::<Runtime>, _>());
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
	/// Additional info:
	/// 	To see if the data is corrupted or not you can call unsafe_storage_get.
	///
	/// Usage:
	/// 	let module = b"MyPallet";
	/// 	let item = b"MyStorage";
	///
	/// 	assert_eq!(Value::unsafe_exists(module, item), false);
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
	/// If a value is not explicitly stored in the storage it will return None.
	/// If a value is stored in the storage but it is corrupted (of different type/size) it will
	/// return None.
	///
	/// Usage:
	/// 	assert_eq!(Value::storage_get::<my_pallet::MyStorage::<Runtime>, _>(), Ok(expected_value));
	/// 	assert_ok!(Value::storage_get::<my_pallet::MyStorage::<Runtime>, _>());
	/// 	assert_err!(Value::storage_get::<MyStorage::<Runtime>, _>());
	#[allow(dead_code)]
	pub fn unsafe_storage_get<T>(module: &[u8], item: &[u8]) -> Option<T>
	where
		T: Decode + Sized,
	{
		get_storage_value::<T>(module, item, b"")
	}

	#[allow(dead_code)]
	pub fn unsafe_storage_put<T>(module: &[u8], item: &[u8], value: T)
	where
		T: Encode,
	{
		put_storage_value::<T>(module, item, b"", value)
	}
}
