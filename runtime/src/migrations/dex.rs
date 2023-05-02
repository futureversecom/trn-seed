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

use crate::{Dex, Runtime, Weight};
use frame_support::{
	dispatch::GetStorageVersion,
	traits::{OnRuntimeUpgrade, StorageVersion},
};

pub struct Upgrade;
impl OnRuntimeUpgrade for Upgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		v1::pre_upgrade()?;

		Ok(())
	}

	fn on_runtime_upgrade() -> Weight {
		let current = Dex::current_storage_version();
		let onchain = Dex::on_chain_storage_version();
		log::info!(target: "Migration", "Dex: Running migration with current storage version {current:?} / onchain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads_writes(2, 0);

		if onchain == 0 {
			log::info!(target: "Migration", "Dex: Migrating from onchain version 0 to onchain version 1.");
			weight += v1::migrate::<Runtime>();

			log::info!(target: "Migration", "Dex: Migration successfully finished.");
			StorageVersion::new(1).put::<Dex>();
		} else {
			log::info!(target: "Migration", "Dex: No migration was done. If you are seeing this message, it means that you forgot to remove old existing migration code. Don't panic, it's not a big deal just don't forget it next time :)");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		v1::post_upgrade()?;

		Ok(())
	}
}

#[allow(dead_code)]
pub mod v1 {
	use super::*;

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Dex: Upgrade to v1 Pre Upgrade.");
		let onchain = Dex::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 1 {
			return Ok(())
		}
		assert_eq!(onchain, 0);

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Dex: Upgrade to v1 Post Upgrade.");

		let current = Dex::current_storage_version();
		let onchain = Dex::on_chain_storage_version();
		assert_eq!(current, 1);
		assert_eq!(onchain, 1);

		Ok(())
	}

	pub fn migrate<T: pallet_dex::Config>() -> Weight {
		log::info!(target: "Migration", "Dex: Cleaning up dex related storages...");

		// Kill Dex Storage
		_ = pallet_dex::TradingPairLPToken::<T>::clear(u32::MAX, None);
		_ = pallet_dex::LiquidityPool::<T>::clear(u32::MAX, None);
		_ = pallet_dex::TradingPairStatuses::<T>::clear(u32::MAX, None);

		log::info!(target: "Migration", "Dex: ...Successfully cleaned up dex related storages");

		<Runtime as frame_system::Config>::DbWeight::get().reads_writes(3, 0)
	}
	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::new_test_ext;
		use pallet_dex::{TradingPair, TradingPairStatus};

		#[test]
		fn migration_test() {
			new_test_ext().execute_with(|| {
				StorageVersion::new(0).put::<Dex>();

				let tp = TradingPair::new(1, 2);

				// Insert TradingPair data
				pallet_dex::TradingPairLPToken::<Runtime>::insert(tp, Some(12));
				pallet_dex::LiquidityPool::<Runtime>::insert(tp, (1000000, 3000000));
				pallet_dex::TradingPairStatuses::<Runtime>::insert(tp, TradingPairStatus::Enabled);

				// Make sure TradingPair data are correctly stored
				assert_eq!(pallet_dex::TradingPairLPToken::<Runtime>::get(tp).unwrap(), 12);
				assert_eq!(pallet_dex::LiquidityPool::<Runtime>::get(tp), (1000000, 3000000));
				assert_eq!(
					pallet_dex::TradingPairStatuses::<Runtime>::get(tp),
					TradingPairStatus::Enabled
				);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();

				// Check if inserted data are cleaned
				assert_eq!(pallet_dex::TradingPairLPToken::<Runtime>::get(tp), None);
				assert_eq!(pallet_dex::LiquidityPool::<Runtime>::get(tp), (0, 0));
				assert_eq!(
					pallet_dex::TradingPairStatuses::<Runtime>::get(tp),
					TradingPairStatus::NotEnabled
				);

				// Check if version has been set correctly
				let onchain = Dex::on_chain_storage_version();
				assert_eq!(onchain, 1);
			});
		}
	}
}
