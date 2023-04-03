use crate::{FeeControl, Runtime, Weight};
use frame_support::{
	dispatch::GetStorageVersion,
	storage_alias,
	traits::{OnRuntimeUpgrade, StorageVersion},
};
use pallet_fee_control::DefaultValues;

#[allow(unused_imports)]
use super::Value;
#[allow(unused_imports)]
use frame_support::assert_ok;

pub struct Upgrade;
impl OnRuntimeUpgrade for Upgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		v2::pre_upgrade()?;

		Ok(())
	}

	fn on_runtime_upgrade() -> Weight {
		let current = FeeControl::current_storage_version();
		let onchain = FeeControl::on_chain_storage_version();
		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads_writes(2, 0);

		log::info!(target: "FeeControl", "Running migration with current storage version {current:?} / onchain {onchain:?}");

		if onchain == 1 {
			log::info!(target: "FeeControl", "Migrating from onchain version 1 to onchain version 2.");
			weight += v2::migrate::<Runtime>();

			log::info!(target: "FeeControl", "Migration successfully finished.");
			StorageVersion::new(2).put::<FeeControl>();
		} else {
			log::info!(target: "FeeControl", "No migration was done. If you are seeing this message, it means that you forgot to remove old existing migration code. Don't panic, it's not a big deal just don't forget it next time :)");
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
	use super::*;
	use frame_support::weights::Weight;
	use sp_core::U256;
	use sp_runtime::Perbill;

	#[storage_alias]
	pub type EvmBaseFeePerGas<T: pallet_fee_control::Config> =
		StorageValue<pallet_fee_control::Pallet<T>, U256>;

	#[storage_alias]
	pub type ExtrinsicWeightToFee<T: pallet_fee_control::Config> =
		StorageValue<pallet_fee_control::Pallet<T>, Perbill>;

	pub fn migrate<T: pallet_fee_control::Config>() -> Weight {
		// We don't care about EvmBaseFeePerGas and ExtrinsicWeightToFee defaults
		let evm = EvmBaseFeePerGas::<T>::take();
		let weight = ExtrinsicWeightToFee::<T>::take();

		let value = pallet_fee_control::PalletData {
			evm_base_fee_per_gas: evm.unwrap_or_else(|| T::DefaultValues::evm_base_fee_per_gas()),
			weight_multiplier: weight.unwrap_or_else(|| T::DefaultValues::weight_multiplier()),
			length_multiplier: T::DefaultValues::length_multiplier(),
		};
		pallet_fee_control::Data::<T>::put(value);

		<Runtime as frame_system::Config>::DbWeight::get().reads_writes(0, 3)
	}

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade() -> Result<(), &'static str> {
		log::info!(target: "FeeControl", "FeeControl Upgrade to V2 Pre Upgrade.");
		// Storage Version Check
		let onchain = FeeControl::on_chain_storage_version();
		assert_eq!(onchain, 1);

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade() -> Result<(), &'static str> {
		log::info!(target: "FeeControl", "FeeControl Upgrade to V2 Post Upgrade.");
		// Storage Version Check
		let onchain = FeeControl::on_chain_storage_version();
		assert_eq!(onchain, 2);

		assert_eq!(Value::exists::<EvmBaseFeePerGas::<Runtime>, _>(), false);
		assert_eq!(Value::exists::<ExtrinsicWeightToFee::<Runtime>, _>(), false);
		assert_ok!(Value::storage_get::<pallet_fee_control::Data::<Runtime>, _>());

		Ok(())
	}

	#[cfg(test)]
	mod tests {
		use frame_support::Hashable;

		use super::*;
		use crate::migrations::tests::new_test_ext;

		#[test]
		fn storage_version_is_incremented() {
			new_test_ext().execute_with(|| {
				// Preparation
				StorageVersion::new(1).put::<FeeControl>();

				// Action
				Upgrade::on_runtime_upgrade();

				// Check
				assert_eq!(FeeControl::on_chain_storage_version(), 2);
			});
		}

		#[test]
		fn storage_is_removed() {
			new_test_ext().execute_with(|| {
				// Preparation
				StorageVersion::new(1).put::<FeeControl>();
				// Insert storage
				EvmBaseFeePerGas::<Runtime>::put(U256::from(10u128));
				ExtrinsicWeightToFee::<Runtime>::put(Perbill::from_parts(100));
				assert_eq!(Value::exists::<EvmBaseFeePerGas::<Runtime>, _>(), true);
				assert_eq!(Value::exists::<ExtrinsicWeightToFee::<Runtime>, _>(), true);

				// Action
				Upgrade::on_runtime_upgrade();

				// Check
				assert_eq!(Value::exists::<EvmBaseFeePerGas::<Runtime>, _>(), false);
				assert_eq!(Value::exists::<ExtrinsicWeightToFee::<Runtime>, _>(), false);
			});
		}

		#[test]
		fn new_storage_is_created_with_defaults() {
			new_test_ext().execute_with(|| {
				// Preparation
				StorageVersion::new(1).put::<FeeControl>();
				assert_eq!(EvmBaseFeePerGas::<Runtime>::exists(), false);
				assert_eq!(ExtrinsicWeightToFee::<Runtime>::exists(), false);

				// Action
				Upgrade::on_runtime_upgrade();

				// Check
				let expected_value = pallet_fee_control::PalletData {
					evm_base_fee_per_gas:
						<Runtime as pallet_fee_control::Config>::DefaultValues::evm_base_fee_per_gas(
						),
					weight_multiplier:
						<Runtime as pallet_fee_control::Config>::DefaultValues::weight_multiplier(),
					length_multiplier:
						<Runtime as pallet_fee_control::Config>::DefaultValues::length_multiplier(),
				};

				assert_eq!(
					Value::storage_get::<pallet_fee_control::Data::<Runtime>, _>(),
					Ok(expected_value)
				);
			});
		}

		#[test]
		fn new_storage_is_created_with_actual_storage() {
			new_test_ext().execute_with(|| {
				// Preparation
				StorageVersion::new(1).put::<FeeControl>();
				// Insert storage
				let evm_base_fee_per_gas = U256::from(321u128);
				let weight_multiplier = Perbill::from_parts(555);
				EvmBaseFeePerGas::<Runtime>::put(evm_base_fee_per_gas);
				ExtrinsicWeightToFee::<Runtime>::put(weight_multiplier);

				// Action
				Upgrade::on_runtime_upgrade();

				// Check
				let expected_value = pallet_fee_control::PalletData {
					evm_base_fee_per_gas,
					weight_multiplier,
					length_multiplier:
						<Runtime as pallet_fee_control::Config>::DefaultValues::length_multiplier(),
				};

				assert_eq!(
					Value::storage_get::<pallet_fee_control::Data::<Runtime>, _>(),
					Ok(expected_value)
				);
			});
		}
	}
}
