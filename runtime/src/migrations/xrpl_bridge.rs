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
	storage_alias,
	traits::{OnRuntimeUpgrade, StorageVersion},
};

#[allow(unused_imports)]
use super::Value as V;
#[allow(unused_imports)]
use frame_support::assert_ok;
use frame_support::{BoundedVec, Twox64Concat};
use pallet_xrpl_bridge as pallet;
use sp_std::vec::Vec;

pub struct Upgrade;
impl OnRuntimeUpgrade for Upgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		let onchain = XRPLBridge::on_chain_storage_version();
		if onchain == 1 {
			v2::pre_upgrade()?;
		}

		Ok(())
	}

	fn on_runtime_upgrade() -> Weight {
		let current = XRPLBridge::current_storage_version();
		let onchain = XRPLBridge::on_chain_storage_version();
		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads_writes(2, 0);

		log::info!("XrplBridge: Running migration with current storage version {current:?} / onchain {onchain:?}");

		if onchain == 1 {
			log::info!("XrplBridge: Migrating from onchain version 1 to onchain version 2.");
			weight += v2::migrate::<Runtime>();

			log::info!("XrplBridge: Migration successfully finished.");
			StorageVersion::new(2).put::<XRPLBridge>();
		} else {
			log::info!("XrplBridge: No migration was done. If you are seeing this message, it means that you forgot to remove old existing migration code. Don't panic, it's not a big deal just don't forget it next time :)");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		let onchain = XRPLBridge::on_chain_storage_version();
		if onchain == 2 {
			v2::post_upgrade()?;
		}

		Ok(())
	}
}

mod v2 {
	use super::*;
	use frame_support::weights::Weight;
	use seed_primitives::xrpl::XrplTxHash;
	type BlockNumber<T> = <T as frame_system::Config>::BlockNumber;

	#[storage_alias]
	pub type ProcessXRPTransaction<T: pallet::Config> =
		StorageMap<pallet::Pallet<T>, Twox64Concat, BlockNumber<T>, Vec<XrplTxHash>>;

	#[storage_alias]
	pub type SettledXRPTransactionDetails<T: pallet::Config> =
		StorageMap<pallet::Pallet<T>, Twox64Concat, BlockNumber<T>, Vec<XrplTxHash>>;

	pub fn migrate<T: pallet::Config>() -> Weight {
		let xrp_transaction_old: Vec<(BlockNumber<T>, Vec<XrplTxHash>)> =
			ProcessXRPTransaction::<T>::iter().collect();
		let xrp_transaction_details_old: Vec<(BlockNumber<T>, Vec<XrplTxHash>)> =
			SettledXRPTransactionDetails::<T>::iter().collect();

		_ = ProcessXRPTransaction::<T>::clear(u32::max_value(), None);
		_ = SettledXRPTransactionDetails::<T>::clear(u32::max_value(), None);

		log::info!("XrplBridge: Removed [ProcessXRPTransaction, SettledXRPTransactionDetails]");

		let read = xrp_transaction_old.len() + xrp_transaction_details_old.len();
		let write = read;

		for (key, value) in xrp_transaction_old {
			let Ok(val) = BoundedVec::try_from(value.clone()) else {
				log::warn!("Failed to add key:{:?}, value {:?} for ProcessXRPTransaction", key, value);
				continue;
			};

			pallet::ProcessXRPTransaction::<T>::insert(key, val);
		}

		for (key, value) in xrp_transaction_details_old {
			let Ok(val) = BoundedVec::try_from(value.clone()) else {
				log::warn!("Failed to add key:{:?}, value {:?} for SettledXRPTransactionDetails", key, value);
				continue;
			};

			pallet::SettledXRPTransactionDetails::<T>::insert(key, val);
		}

		log::info!("XrplBridge: ReAdded [ProcessXRPTransaction, SettledXRPTransactionDetails]");

		<Runtime as frame_system::Config>::DbWeight::get().reads_writes(read as u64, write as u64)
	}

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade() -> Result<(), &'static str> {
		log::info!("XrplBridge: Upgrade to V2 Pre Upgrade.");

		assert_eq!(XRPLBridge::on_chain_storage_version(), 1);

		let keys: Vec<BlockNumber<Runtime>> =
			ProcessXRPTransaction::<Runtime>::iter_keys().collect();
		for key in keys.iter() {
			ProcessXRPTransaction::<Runtime>::try_get(key)
				.expect("Should not happen. Old ProcessXRPTransaction is corrupted");
		}

		log::info!(
			"XrplBridge: Checked {} value from old ProcessXRPTransaction and they are all valid",
			keys.len()
		);

		let keys: Vec<BlockNumber<Runtime>> =
			SettledXRPTransactionDetails::<Runtime>::iter_keys().collect();
		for key in keys.iter() {
			SettledXRPTransactionDetails::<Runtime>::try_get(key)
				.expect("Should not happen. Old SettledXRPTransactionDetails is corrupted");
		}

		log::info!(
			"XrplBridge: Checked {} value from old SettledXRPTransactionDetails and they are all valid",
			keys.len()
		);

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade() -> Result<(), &'static str> {
		log::info!("XrplBridge: Upgrade to V2 Post Upgrade.");

		assert_eq!(XRPLBridge::on_chain_storage_version(), 2);

		let keys: Vec<BlockNumber<Runtime>> =
			pallet::ProcessXRPTransaction::<Runtime>::iter_keys().collect();
		for key in keys.iter() {
			pallet::ProcessXRPTransaction::<Runtime>::try_get(key)
				.expect("Should not happen. New ProcessXRPTransaction is corrupted");
		}

		log::info!(
			"XrplBridge: Checked {} value from new ProcessXRPTransaction and they are all valid",
			keys.len()
		);

		let keys: Vec<BlockNumber<Runtime>> =
			pallet::SettledXRPTransactionDetails::<Runtime>::iter_keys().collect();
		for key in keys.iter() {
			pallet::SettledXRPTransactionDetails::<Runtime>::try_get(key)
				.expect("Should not happen. New SettledXRPTransactionDetails is corrupted");
		}

		log::info!(
			"XrplBridge: Checked {} value from new SettledXRPTransactionDetails and they are all valid",
			keys.len()
		);

		Ok(())
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::new_test_ext;

		/* 		#[test]
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
				assert_eq!(V::exists::<EvmBaseFeePerGas::<Runtime>, _>(), true);
				assert_eq!(V::exists::<ExtrinsicWeightToFee::<Runtime>, _>(), true);

				// Action
				Upgrade::on_runtime_upgrade();

				// Check
				assert_eq!(V::exists::<EvmBaseFeePerGas::<Runtime>, _>(), false);
				assert_eq!(V::exists::<ExtrinsicWeightToFee::<Runtime>, _>(), false);
			});
		}

		#[test]
		fn new_storage_is_created_with_defaults() {
			new_test_ext().execute_with(|| {
				// Preparation
				StorageVersion::new(1).put::<FeeControl>();
				assert_eq!(V::exists::<EvmBaseFeePerGas::<Runtime>, _>(), false);
				assert_eq!(V::exists::<ExtrinsicWeightToFee::<Runtime>, _>(), false);

				// Action
				Upgrade::on_runtime_upgrade();

				// Check
				let expected_value = pallet_fee_control::FeeConfig {
					evm_base_fee_per_gas:
						<Runtime as pallet_fee_control::Config>::DefaultValues::evm_base_fee_per_gas(
						),
					weight_multiplier:
						<Runtime as pallet_fee_control::Config>::DefaultValues::weight_multiplier(),
					length_multiplier:
						<Runtime as pallet_fee_control::Config>::DefaultValues::length_multiplier(),
				};

				let actual_value = V::storage_get::<pallet_fee_control::Data<Runtime>, _>();
				assert_eq!(actual_value, Ok(expected_value));
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
				let expected_value = pallet_fee_control::FeeConfig {
					evm_base_fee_per_gas,
					weight_multiplier,
					length_multiplier:
						<Runtime as pallet_fee_control::Config>::DefaultValues::length_multiplier(),
				};

				let actual_value = V::storage_get::<pallet_fee_control::Data<Runtime>, _>();
				assert_eq!(actual_value, Ok(expected_value));
			});
		} */
	}
}
