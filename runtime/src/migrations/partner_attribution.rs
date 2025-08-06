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

use crate::{PartnerAttribution, Runtime, Weight};
use frame_support::{
	dispatch::GetStorageVersion,
	traits::{OnRuntimeUpgrade, StorageVersion},
};
#[cfg(feature = "try-runtime")]
use sp_runtime::DispatchError;
#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

/// Migration to set PartnerCount to the current number of partners
pub struct PartnerCountMigration;

impl OnRuntimeUpgrade for PartnerCountMigration {
	fn on_runtime_upgrade() -> Weight {
		let current = PartnerAttribution::current_storage_version();
		let onchain = PartnerAttribution::on_chain_storage_version();
		log::info!(target: "Migration", "PartnerAttribution: Running migration with current storage version {current:?} / on-chain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(2);

		// Check if migration has already been done
		if onchain >= 1 {
			log::info!(
				target: "Migration",
				"PartnerAttribution: No migration needed, already at version {onchain:?}. Migration code can be removed."
			);
			return weight;
		}

		log::info!(target: "Migration", "PartnerAttribution: Migrating from on-chain version {onchain:?} to version {current:?}.");

		// Count existing partners and set the PartnerCount
		let partner_count = pallet_partner_attribution::Partners::<Runtime>::iter().count() as u32;

		// Add weight for iterating through partners
		weight = weight.saturating_add(
			<Runtime as frame_system::Config>::DbWeight::get().reads(partner_count as u64),
		);

		// Set the PartnerCount
		pallet_partner_attribution::PartnerCount::<Runtime>::put(partner_count);

		// Add weight for writing the PartnerCount
		weight =
			weight.saturating_add(<Runtime as frame_system::Config>::DbWeight::get().writes(1));

		// Update storage version
		StorageVersion::new(1).put::<PartnerAttribution>();

		log::info!(target: "Migration", "PartnerAttribution: Migration successfully completed. Partner count: {partner_count}");
		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
		log::info!(target: "Migration", "PartnerAttribution: Pre-upgrade check for PartnerCount migration.");
		let onchain = PartnerAttribution::on_chain_storage_version();

		// Return early if migration has already been done
		if onchain >= 1 {
			log::info!(target: "Migration", "PartnerAttribution: Migration already completed at version {onchain:?}");
			return Ok(Vec::new());
		}

		// Count existing partners before migration
		let partner_count = pallet_partner_attribution::Partners::<Runtime>::iter().count() as u32;
		log::info!(target: "Migration", "PartnerAttribution: Found {partner_count} partners before migration");

		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), DispatchError> {
		log::info!(target: "Migration", "PartnerAttribution: Post-upgrade verification for PartnerCount migration.");

		let current = PartnerAttribution::current_storage_version();
		let onchain = PartnerAttribution::on_chain_storage_version();

		// Verify storage version was updated
		assert_eq!(current, 1);
		assert_eq!(onchain, 1);

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::migrations::tests::new_test_ext;
	use pallet_partner_attribution::{PartnerCount, PartnerInformation, Partners};
	use seed_primitives::AccountId;
	use sp_core::H160;

	#[test]
	fn migrate_with_no_partners() {
		new_test_ext().execute_with(|| {
			// Setup storage version to 0 (pre-migration)
			StorageVersion::new(0).put::<PartnerAttribution>();

			// Ensure no partners exist initially (Partners storage is empty)
			assert_eq!(Partners::<Runtime>::iter().count(), 0);
			assert_eq!(PartnerCount::<Runtime>::get(), 0);

			// Run migration
			PartnerCountMigration::on_runtime_upgrade();

			// Verify storage version was updated
			assert_eq!(PartnerAttribution::on_chain_storage_version(), 1);

			// Verify PartnerCount is set to 0
			assert_eq!(PartnerCount::<Runtime>::get(), 0);
		});
	}

	#[test]
	fn migrate_with_existing_partners() {
		new_test_ext().execute_with(|| {
			// Setup storage version to 0 (pre-migration)
			StorageVersion::new(0).put::<PartnerAttribution>();

			// Create some test partners
			let partner1: AccountId = H160::from_low_u64_be(1).into();
			let partner2: AccountId = H160::from_low_u64_be(2).into();
			let partner3: AccountId = H160::from_low_u64_be(3).into();

			let partner_data1 = PartnerInformation {
				owner: partner1.clone(),
				account: partner1.clone(),
				accumulated_fees: 1000,
				fee_percentage: Some(sp_runtime::Permill::from_percent(5)),
			};
			let partner_data2 = PartnerInformation {
				owner: partner2.clone(),
				account: partner2.clone(),
				accumulated_fees: 2000,
				fee_percentage: Some(sp_runtime::Permill::from_percent(10)),
			};
			let partner_data3 = PartnerInformation {
				owner: partner3.clone(),
				account: partner3.clone(),
				accumulated_fees: 3000,
				fee_percentage: Some(sp_runtime::Permill::from_percent(15)),
			};
			Partners::<Runtime>::insert(1, partner_data1);
			Partners::<Runtime>::insert(2, partner_data2);
			Partners::<Runtime>::insert(3, partner_data3);

			// Verify partners exist but PartnerCount is still 0
			assert_eq!(Partners::<Runtime>::iter().count(), 3);
			assert_eq!(PartnerCount::<Runtime>::get(), 0);

			// Run migration
			PartnerCountMigration::on_runtime_upgrade();

			// Verify storage version was updated
			assert_eq!(PartnerAttribution::on_chain_storage_version(), 1);

			// Verify PartnerCount is set to 3
			assert_eq!(PartnerCount::<Runtime>::get(), 3);
		});
	}

	#[test]
	fn migrate_already_completed() {
		new_test_ext().execute_with(|| {
			// Setup storage version to 1 (already migrated)
			StorageVersion::new(1).put::<PartnerAttribution>();

			// Set some PartnerCount value
			PartnerCount::<Runtime>::put(5);

			// Run migration again
			PartnerCountMigration::on_runtime_upgrade();

			// Verify storage version is still 1
			assert_eq!(PartnerAttribution::on_chain_storage_version(), 1);

			// Verify PartnerCount is unchanged
			assert_eq!(PartnerCount::<Runtime>::get(), 5);
		});
	}
}
