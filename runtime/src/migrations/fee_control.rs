pub mod v2 {
	use crate::FeeControl;
	use frame_support::{
		dispatch::GetStorageVersion,
		migration,
		traits::{OnRuntimeUpgrade, PalletInfoAccess, StorageVersion},
		weights::Weight,
	};

	pub struct Upgrade;
	impl OnRuntimeUpgrade for Upgrade {
		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<(), &'static str> {
			log::info!("Fee Control Upgrade to V2 Pre Upgrade.");
			let onchain = FeeControl::on_chain_storage_version();

			assert_eq!(onchain, 0);
			Ok(())
		}

		fn on_runtime_upgrade() -> Weight {
			let current = FeeControl::current_storage_version();
			let onchain = FeeControl::on_chain_storage_version();
			let mut weight = Weight::from(0u32);

			log::info!(target: "Fee Control", "Running migration with current storage version {current:?} / onchain {onchain:?}");

			if onchain == 0 {
				log::info!(target: "Fee Control", "Migrating from onchain version 0 to onchain version 2.");
				weight += migrate();

				log::info!(target: "Fee Control", "Migration successfully finished.");
				StorageVersion::new(2).put::<FeeControl>();
			} else {
				log::info!(target: "Fee Control", "No migration was done. If you are seeing this message, it means that you forgot to remove old existing migration code. Don't panic, it's not a big deal just don't forget it next time.");
			}

			weight
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade() -> Result<(), &'static str> {
			log::info!("Fee Control Upgrade to V2 Post Upgrade.");
			let onchain = FeeControl::on_chain_storage_version();

			assert_eq!(onchain, 2);
			Ok(())
		}
	}

	fn migrate() -> Weight {
		let module = FeeControl::name().as_bytes();
		let removal_1 =
			migration::clear_storage_prefix(module, b"EvmBaseFeePerGas", b"", None, None);
		let removal_2 =
			migration::clear_storage_prefix(module, b"ExtrinsicWeightToFee", b"", None, None);

		if removal_1.maybe_cursor.is_some() {
			// Unexpected due to this being a single storage value removal
			log::error!(target: "Fee Control", "EvmBaseFeePerGas storage item removal was not completed.");
		} else {
			log::info!(target: "Fee Control", "EvmBaseFeePerGas storage item successfully removed from db.")
		};

		if removal_2.maybe_cursor.is_some() {
			// Unexpected due to this being a single storage value removal
			log::error!(target: "Fee Control", "ExtrinsicWeightToFee storage item removal was not completed.");
		} else {
			log::info!(target: "Fee Control", "ExtrinsicWeightToFee storage item successfully removed from db.")
		};

		// TODO
		Weight::from(100u32)
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::new_test_ext;

		#[test]
		fn migration_test() {
			use frame_support::migration::{have_storage_value, put_storage_value};

			new_test_ext().execute_with(|| {
				let item_1 = b"EvmBaseFeePerGas";
				let item_2 = b"ExtrinsicWeightToFee";
				let test_storage_key = b"";
				let module = FeeControl::name().as_bytes();

				put_storage_value(module, item_1, test_storage_key, 123);
				put_storage_value(module, item_2, test_storage_key, 123);

				Upgrade::on_runtime_upgrade();

				assert_eq!(have_storage_value(module, item_1, test_storage_key), false);
				assert_eq!(have_storage_value(module, item_2, test_storage_key), false);
			});
		}
	}
}
