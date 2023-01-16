use crate::{weights::WeightInfo, Config, Pallet};
use frame_support::{
	dispatch::GetStorageVersion, pallet_prelude::StorageVersion, storage::migration,
	traits::PalletInfoAccess,
};

/// The current storage version.
pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

pub fn try_migrate<T: Config>() -> u64 {
	if Pallet::<T>::on_chain_storage_version() == 0 {
		log::info!("Beginning removal of DoorNonce storage item for xrpl-bridge");
		let res = migration::clear_storage_prefix(
			<Pallet<T>>::name().as_bytes(),
			b"DoorNonce",
			b"",
			None,
			None,
		);

		if res.maybe_cursor.is_some() {
			// Unexpected due to this being a single storage value removal
			log::error!("DoorNonce storage item removal was not completed");
			return T::WeightInfo::on_runtime_upgrade_no_change()
		} else {
			log::info!("DoorNonce storage item migration completed")
		};

		StorageVersion::new(1).put::<Pallet<T>>();
		return T::WeightInfo::on_runtime_upgrade()
	}

	T::WeightInfo::on_runtime_upgrade_no_change()
}

#[test]
fn migrate_0_to_1() {
	use crate::mock::*;
	use frame_support::migration::{have_storage_value, put_storage_value};

	new_test_ext().execute_with(|| {
		let storage_item_name = b"DoorNonce";
		let test_storage_key = b"";

		assert_eq!(
			have_storage_value(
				<Pallet<Test>>::name().as_bytes(),
				storage_item_name,
				test_storage_key
			),
			false
		);
		put_storage_value(
			<Pallet<Test>>::name().as_bytes(),
			storage_item_name,
			test_storage_key,
			123,
		);
		assert_eq!(
			have_storage_value(
				<Pallet<Test>>::name().as_bytes(),
				storage_item_name,
				test_storage_key
			),
			true
		);
		try_migrate::<Test>();
		assert_eq!(
			have_storage_value(
				<Pallet<Test>>::name().as_bytes(),
				storage_item_name,
				test_storage_key
			),
			false
		);
	});
}
