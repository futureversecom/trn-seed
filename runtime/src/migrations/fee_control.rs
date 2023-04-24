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

use crate::{FeeControl, Runtime, Weight};
use frame_support::{
	dispatch::GetStorageVersion,
	storage_alias,
	traits::{OnRuntimeUpgrade, StorageVersion},
};
use pallet::DefaultValues;
use pallet_fee_control as pallet;

#[allow(unused_imports)]
use super::Value as V;
#[allow(unused_imports)]
use frame_support::assert_ok;

pub struct Upgrade;
impl OnRuntimeUpgrade for Upgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		v3::pre_upgrade()?;

		Ok(())
	}

	fn on_runtime_upgrade() -> Weight {
		let current = FeeControl::current_storage_version();
		let onchain = FeeControl::on_chain_storage_version();
		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads_writes(2, 0);

		log::info!("FeeControl: Running migration with current storage version {current:?} / onchain {onchain:?}");

		if onchain == 2 {
			log::info!("FeeControl: Migrating from onchain version 2 to onchain version 3.");
			weight += v3::migrate::<Runtime>();

			log::info!("FeeControl: Migration successfully finished.");
			StorageVersion::new(3).put::<FeeControl>();
		} else {
			log::info!("FeeControl: No migration was done. If you are seeing this message, it means that you forgot to remove old existing migration code. Don't panic, it's not a big deal just don't forget it next time :)");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		v3::post_upgrade()?;

		Ok(())
	}
}

mod v3 {
	use super::*;
	use codec::{Decode, Encode, MaxEncodedLen};
	use frame_support::weights::Weight;
	use scale_info::TypeInfo;
	use seed_primitives::Balance;
	use sp_core::U256;
	use sp_runtime::Perbill;

	#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
	pub struct OldFeeConfig {
		pub evm_base_fee_per_gas: U256,
		pub weight_multiplier: Perbill,
		pub length_multiplier: Balance,
	}

	#[storage_alias]
	pub type Data<T: pallet::Config> = StorageValue<pallet::Pallet<T>, OldFeeConfig>;

	pub fn migrate<T: pallet::Config>() -> Weight {
		// Kill old storage
		Data::<T>::kill();

		// Transform
		let new_value = pallet::FeeConfig {
			evm_base_fee_per_gas: T::DefaultValues::evm_base_fee_per_gas(),
			weight_multiplier: T::DefaultValues::weight_multiplier(),
			length_multiplier: T::DefaultValues::length_multiplier(),
		};
		pallet::Data::<T>::put(new_value);

		log::info!("FeeControl: Removed Data");
		log::info!("FeeControl: Added Data");

		<Runtime as frame_system::Config>::DbWeight::get().reads_writes(0, 2)
	}

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade() -> Result<(), &'static str> {
		let onchain = FeeControl::on_chain_storage_version();
		if onchain == 3 {
			log::info!("Skipping FeeControl Upgrade to V3 Pre Upgrade checks.");
			return Ok(())
		}

		log::info!("FeeControl Upgrade to V3 Pre Upgrade.");
		assert_eq!(onchain, 2);
		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade() -> Result<(), &'static str> {
		log::info!("FeeControl Upgrade to V3 Post Upgrade.");
		let onchain = FeeControl::on_chain_storage_version();

		assert_eq!(onchain, 3);
		assert_ok!(V::storage_get::<pallet_fee_control::Data::<Runtime>, _>());

		Ok(())
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::new_test_ext;

		#[test]
		fn storage_version_is_incremented() {
			new_test_ext().execute_with(|| {
				// Preparation
				StorageVersion::new(2).put::<FeeControl>();

				// Action
				Upgrade::on_runtime_upgrade();

				// Check
				assert_eq!(FeeControl::on_chain_storage_version(), 3);
			});
		}

		#[test]
		fn storage_is_updated() {
			new_test_ext().execute_with(|| {
				// Preparation
				StorageVersion::new(2).put::<FeeControl>();

				// Insert storage
				let value = OldFeeConfig {
					evm_base_fee_per_gas: U256::from(100u32),
					weight_multiplier: Perbill::one(),
					length_multiplier: Balance::from(123u128),
				};
				Data::<Runtime>::put(value.clone());

				// Action
				Upgrade::on_runtime_upgrade();

				// Check
				let expected_value = pallet_fee_control::FeeConfig {
					evm_base_fee_per_gas:
						<Runtime as pallet::Config>::DefaultValues::evm_base_fee_per_gas(),
					weight_multiplier:
						<Runtime as pallet::Config>::DefaultValues::weight_multiplier(),
					length_multiplier:
						<Runtime as pallet::Config>::DefaultValues::length_multiplier(),
				};

				let actual_value = V::storage_get::<pallet::Data<Runtime>, _>();
				assert_eq!(actual_value, Ok(expected_value));
			});
		}
	}
}
