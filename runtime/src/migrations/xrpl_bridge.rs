// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

use crate::{Runtime, Weight, XRPLBridge};
use frame_support::{
	dispatch::GetStorageVersion,
	traits::{OnRuntimeUpgrade, StorageVersion},
};
use frame_system::pallet_prelude::BlockNumberFor;

#[allow(unused_imports)]
use sp_runtime::DispatchError;
#[allow(unused_imports)]
use sp_std::vec::Vec;

pub struct Upgrade;

impl OnRuntimeUpgrade for Upgrade {
	fn on_runtime_upgrade() -> Weight {
		let current = XRPLBridge::current_storage_version();
		let onchain = XRPLBridge::on_chain_storage_version();
		log::info!(target: "Migration", "XRPLBridge: Running migration with current storage version {current:?} / on-chain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(2);

		if onchain < 4 {
			log::info!(target: "Migration", "XRPLBridge: Migrating from on-chain version {onchain:?} to on-chain version {current:?}.");
			weight += v4::migrate::<Runtime>();

			StorageVersion::new(4).put::<XRPLBridge>();

			log::info!(target: "Migration", "XRPLBridge: Migration successfully completed.");
		} else {
			log::info!(target: "Migration", "XRPLBridge: No migration was done, however migration code needs to be removed.");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
		log::info!(target: "Migration", "XRPLBridge: Upgrade to v4 Pre Upgrade.");
		let onchain = XRPLBridge::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 4 {
			return Ok(Vec::new());
		}
		assert_eq!(onchain, 3);

		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), DispatchError> {
		log::info!(target: "Migration", "XRPLBridge: Upgrade to v4 Post Upgrade.");
		let current = XRPLBridge::current_storage_version();
		let onchain = XRPLBridge::on_chain_storage_version();
		assert_eq!(current, 4);
		assert_eq!(onchain, 4);
		Ok(())
	}
}

#[allow(dead_code)]
#[allow(unused_imports)]
pub mod v4 {
	use super::*;
	use crate::migrations::Value;

	use frame_support::weights::Weight;
	use pallet_xrpl_bridge::PaymentDelay;

	use seed_primitives::Balance;
	use sp_core::{Get, H160};

	type AccountId = <Runtime as frame_system::Config>::AccountId;

	type XrpAssetId = <Runtime as pallet_xrpl_bridge::Config>::XrpAssetId;

	type V3PaymentDelay<T> = (Balance, BlockNumberFor<T>);

	pub fn migrate<T: frame_system::Config + pallet_xrpl_bridge::Config>() -> Weight
	where
		AccountId: From<H160>,
	{
		log::info!(target: "Migration", "XRPLBridge: migrating PaymentDelay");
		let mut weight: Weight = Weight::zero();

		weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().reads(1));

		if let Some(payment_delay) =
			Value::unsafe_storage_get::<V3PaymentDelay<T>>(b"XRPLBridge", b"PaymentDelay")
		{
			weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().writes(2));

			Value::unsafe_clear(b"XRPLBridge", b"PaymentDelay");
			PaymentDelay::<T>::insert(XrpAssetId::get(), payment_delay);
		}

		log::info!(target: "Migration", "XRPLBridge: successfully migrated PaymentDelay");

		weight
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::new_test_ext;

		#[test]
		fn migrate_with_existing_payment_delay() {
			new_test_ext().execute_with(|| {
				// Setup storage
				StorageVersion::new(3).put::<XRPLBridge>();

				let payment_delay = (111, 3344);

				Value::unsafe_storage_put::<V3PaymentDelay<Runtime>>(
					b"XRPLBridge",
					b"PaymentDelay",
					payment_delay,
				);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();
				assert_eq!(XRPLBridge::on_chain_storage_version(), 4);

				assert_eq!(PaymentDelay::<Runtime>::get(XrpAssetId::get()), Some(payment_delay));

				assert_eq!(
					Value::unsafe_storage_get::<V3PaymentDelay<Runtime>>(
						b"XRPLBridge",
						b"PaymentDelay",
					),
					None
				);
			});
		}

		#[test]
		fn migrate_with_no_payment_delay() {
			new_test_ext().execute_with(|| {
				// Setup storage
				StorageVersion::new(3).put::<XRPLBridge>();

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();
				assert_eq!(XRPLBridge::on_chain_storage_version(), 4);

				assert_eq!(PaymentDelay::<Runtime>::get(XrpAssetId::get()), None);
			});
		}
	}
}
