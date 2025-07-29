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

use crate::{FeeControl, Runtime, Weight};
use codec::{Decode, Encode};
use frame_support::{
	dispatch::GetStorageVersion,
	traits::{OnRuntimeUpgrade, StorageVersion},
};
use pallet_transaction_payment::Multiplier;
use scale_info::TypeInfo;
use seed_primitives::Balance;
use sp_core::U256;
#[cfg(feature = "try-runtime")]
use sp_runtime::DispatchError;
use sp_runtime::{FixedPointNumber, Perbill};
#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

/// Old FeeControlFeeConfig struct before minimum_multiplier field was added
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct OldFeeControlFeeConfig {
	pub evm_base_fee_per_gas: U256,
	pub weight_multiplier: Perbill,
	pub length_multiplier: Balance,
}

/// Migration to add minimum_multiplier field to FeeControlFeeConfig
pub struct FeeControlConfigMigration;

impl OnRuntimeUpgrade for FeeControlConfigMigration {
	fn on_runtime_upgrade() -> Weight {
		let current = FeeControl::current_storage_version();
		let onchain = FeeControl::on_chain_storage_version();
		log::info!(target: "Migration", "FeeControl: Running migration with current storage version {current:?} / on-chain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(2);

		// Check if migration has already been done
		if onchain >= 2 {
			log::info!(
				target: "Migration",
				"FeeControl: No migration needed, already at version {onchain:?}. Migration code can be removed."
			);
			return weight;
		}

		log::info!(target: "Migration", "FeeControl: Migrating from on-chain version {onchain:?} to version {current:?}.");

		// Read the old FeeControlFeeConfig data
		let old_data_key = frame_support::storage::storage_prefix(b"FeeControl", b"Data");

		if let Some(old_data_bytes) = frame_support::storage::unhashed::get_raw(&old_data_key) {
			if let Ok(old_config) = OldFeeControlFeeConfig::decode(&mut &old_data_bytes[..]) {
				log::info!(target: "Migration", "FeeControl: Found existing config data, migrating...");

				// Create new config with minimum_multiplier field
				let new_config = pallet_fee_control::FeeControlFeeConfig {
					evm_base_fee_per_gas: old_config.evm_base_fee_per_gas,
					weight_multiplier: old_config.weight_multiplier.deconstruct(),
					length_multiplier: old_config.length_multiplier,
					minimum_multiplier: Multiplier::saturating_from_rational(
						200_000_000,
						1_000_000_000u128,
					), // 20% default
				};

				// Store the new config
				pallet_fee_control::Data::<Runtime>::put(new_config);

				weight = weight
					.saturating_add(<Runtime as frame_system::Config>::DbWeight::get().writes(1));
				log::info!(target: "Migration", "FeeControl: Successfully migrated config data with minimum_multiplier");
			} else {
				log::warn!(target: "Migration", "FeeControl: Could not decode old config data, using default config");
			}
		} else {
			log::info!(target: "Migration", "FeeControl: No existing config data found, using default config");
		}

		// Add read weight for checking existing data
		weight = weight.saturating_add(<Runtime as frame_system::Config>::DbWeight::get().reads(1));

		// Update storage version
		StorageVersion::new(2).put::<FeeControl>();

		log::info!(target: "Migration", "FeeControl: Migration successfully completed.");
		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
		log::info!(target: "Migration", "FeeControl: Pre-upgrade check for FeeControlConfig migration.");
		let onchain = FeeControl::on_chain_storage_version();

		// Return early if migration has already been done
		if onchain >= 2 {
			log::info!(target: "Migration", "FeeControl: Migration already completed at version {onchain:?}");
			return Ok(Vec::new());
		}

		// Check if we have existing data to migrate
		let old_data_key = frame_support::storage::storage_prefix(b"FeeControl", b"Data");
		let has_data = frame_support::storage::unhashed::get_raw(&old_data_key).is_some();
		log::info!(target: "Migration", "FeeControl: Found existing data: {has_data}");

		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), DispatchError> {
		log::info!(target: "Migration", "FeeControl: Post-upgrade verification for FeeControlConfig migration.");

		let current = FeeControl::current_storage_version();
		let onchain = FeeControl::on_chain_storage_version();

		// Verify storage version was updated
		assert_eq!(current, 2);
		assert_eq!(onchain, 2);

		// Verify that the new config has the minimum_multiplier field
		let config = pallet_fee_control::Data::<Runtime>::get();
		log::info!(target: "Migration", "FeeControl: Verified config has minimum_multiplier: {:?}", config.minimum_multiplier);

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::migrations::tests::new_test_ext;
	use frame_support::traits::StorageVersion;

	#[test]
	fn migrate_with_no_existing_data() {
		new_test_ext().execute_with(|| {
			// Setup storage version to 1 (pre-migration)
			StorageVersion::new(1).put::<FeeControl>();

			// Run migration
			FeeControlConfigMigration::on_runtime_upgrade();

			// Verify storage version was updated
			assert_eq!(FeeControl::on_chain_storage_version(), 2);

			// Verify config has all 4 fields with expected default values
			let config = pallet_fee_control::Data::<Runtime>::get();
			let expected_minimum_multiplier =
				Multiplier::saturating_from_rational(200_000_000, 1_000_000_000u128);

			// Check all 4 fields are present and have expected values
			assert_eq!(config.evm_base_fee_per_gas, U256::from(7_500_000_000_000u128));
			assert_eq!(config.weight_multiplier, 100_000);
			assert_eq!(config.length_multiplier, 350u128);
			assert_eq!(config.minimum_multiplier, expected_minimum_multiplier);
		});
	}

	#[test]
	fn migrate_with_existing_data() {
		new_test_ext().execute_with(|| {
			// Setup storage version to 1 (pre-migration)
			StorageVersion::new(1).put::<FeeControl>();

			// Create old config data with 3 fields
			let old_config = OldFeeControlFeeConfig {
				evm_base_fee_per_gas: U256::from(2_000_000_000_000u128),
				weight_multiplier: Perbill::from_parts(25_000),
				length_multiplier: 50u128,
			};

			// Manually store the old config to simulate existing data
			let old_data_key = frame_support::storage::storage_prefix(b"FeeControl", b"Data");
			frame_support::storage::unhashed::put_raw(&old_data_key, &old_config.encode());

			// Run migration
			FeeControlConfigMigration::on_runtime_upgrade();

			// Verify storage version was updated
			assert_eq!(FeeControl::on_chain_storage_version(), 2);

			// Verify all 4 fields are present - 3 migrated + 1 new default
			let config = pallet_fee_control::Data::<Runtime>::get();
			let expected_minimum_multiplier =
				Multiplier::saturating_from_rational(200_000_000, 1_000_000_000u128);

			// Check migrated fields preserve original values
			assert_eq!(config.evm_base_fee_per_gas, U256::from(2_000_000_000_000u128));
			assert_eq!(config.weight_multiplier, 25_000u32);
			assert_eq!(config.length_multiplier, 50u128);
			// Check new field has default value
			assert_eq!(config.minimum_multiplier, expected_minimum_multiplier);
		});
	}

	#[test]
	fn migrate_already_completed() {
		new_test_ext().execute_with(|| {
			// Setup storage version to 2 (already migrated)
			StorageVersion::new(2).put::<FeeControl>();

			// Run migration again
			FeeControlConfigMigration::on_runtime_upgrade();

			// Verify storage version is still 2
			assert_eq!(FeeControl::on_chain_storage_version(), 2);
		});
	}
}
