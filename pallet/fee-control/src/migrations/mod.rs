pub mod v2 {
	use frame_support::{
		migration,
		pallet_prelude::StorageVersion,
		traits::{Get, OnRuntimeUpgrade, PalletInfoAccess},
	};

	use crate::{Config, Pallet};

	pub struct MigrationV2<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrationV2<T> {
		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<(), &'static str> {
			Ok(())
		}

		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			log::info!("Upgrading on chain storage version from v1 to v2.");

			let module = Pallet::<T>::name().as_bytes();
			let res = migration::clear_storage_prefix(module, b"EvmBaseFeePerGas", b"", None, None);

			if res.maybe_cursor.is_some() {
				// Unexpected due to this being a single storage value removal
				log::error!("EvmBaseFeePerGas storage item removal was not completed.");
				return T::DbWeight::get().reads(1)
			} else {
				log::info!("EvmBaseFeePerGas storage item successfully removed from db.")
			};

			let res =
				migration::clear_storage_prefix(module, b"ExtrinsicWeightToFee", b"", None, None);

			if res.maybe_cursor.is_some() {
				// Unexpected due to this being a single storage value removal
				log::error!("ExtrinsicWeightToFee storage item removal was not completed.");
				return T::DbWeight::get().reads(2)
			} else {
				log::info!("ExtrinsicWeightToFee storage item successfully removed from db.")
			};

			StorageVersion::new(2).put::<Pallet<T>>();

			log::info!("New on chain storage version is: 2");

			T::DbWeight::get().reads(2)
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade() -> Result<(), &'static str> {
			use frame_support::traits::GetStorageVersion;

			// new version must be set.
			assert_eq!(Pallet::<T>::on_chain_storage_version(), 2);
			Ok(())
		}
	}

	#[test]
	fn migrate_1_to_2() {
		use crate::tests::mock::*;
		use frame_support::migration::{have_storage_value, put_storage_value};

		new_test_ext().execute_with(|| {
			let item_1 = b"EvmBaseFeePerGas";
			let item_2 = b"ExtrinsicWeightToFee";
			let test_storage_key = b"";
			let module = Pallet::<Test>::name().as_bytes();

			put_storage_value(module, item_1, test_storage_key, 123);
			put_storage_value(module, item_2, test_storage_key, 123);

			MigrationV2::<Test>::on_runtime_upgrade();

			assert_eq!(have_storage_value(module, item_1, test_storage_key), false);
			assert_eq!(have_storage_value(module, item_2, test_storage_key), false);
		});
	}
}
