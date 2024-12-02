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

use crate::{Nfi, Runtime, Weight};
use frame_support::{
	dispatch::GetStorageVersion,
	traits::{OnRuntimeUpgrade, StorageVersion},
};

#[allow(unused_imports)]
use sp_runtime::DispatchError;
#[allow(unused_imports)]
use sp_std::vec::Vec;

pub struct Upgrade;

impl OnRuntimeUpgrade for Upgrade {
	fn on_runtime_upgrade() -> Weight {
		let current = Nfi::current_storage_version();
		let onchain = Nfi::on_chain_storage_version();
		log::info!(target: "Migration", "Nfi: Running migration with current storage version {current:?} / on-chain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(2);

		if onchain != 0 {
			log::info!(
				target: "Migration",
				"Nfi: No migration was done, This migration should be on top of storage version 0. Migration code needs to be removed."
			);
			return weight;
		}

		log::info!(target: "Migration", "Nfi: Migrating from on-chain version {onchain:?} to on-chain version {current:?}.");
		weight += v1::migrate::<Runtime>();
		StorageVersion::new(1).put::<Nfi>();
		log::info!(target: "Migration", "Nfi: Migration successfully completed.");
		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
		log::info!(target: "Migration", "Nfi: Upgrade to v1 Pre Upgrade.");
		let onchain = Nfi::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 1 {
			return Ok(Vec::new());
		}
		assert_eq!(onchain, 0);

		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), DispatchError> {
		log::info!(target: "Migration", "Nfi: Upgrade to v1 Post Upgrade.");
		let current = Nfi::current_storage_version();
		let onchain = Nfi::on_chain_storage_version();
		assert_eq!(current, 1);
		assert_eq!(onchain, 1);
		Ok(())
	}
}

#[allow(dead_code)]
#[allow(unused_imports)]
pub mod v1 {
	use super::*;
	use crate::migrations::Value;
	use frame_support::pallet_prelude::ValueQuery;
	use frame_support::Twox64Concat;

	use frame_support::weights::Weight;
	use pallet_xrpl_bridge::{
		DoorAddress, DoorTicketSequence, DoorTicketSequenceParams, DoorTicketSequenceParamsNext,
		DoorTxFee, PaymentDelay, TicketSequenceThresholdReachedEmitted,
	};

	use pallet_evm_chain_id::ChainId;
	use pallet_nfi::{
		types::{
			GenericCollectionId, GenericSerialNumber, MultiChainTokenId, NFIDataType, NFIMatrix,
			NFISubType,
		},
		Config, NfiData as NewNfiData, NfiEnabled as NewNfiEnabled,
	};
	use pallet_xrpl_bridge::types::{XRPLDoorAccount, XrplTicketSequenceParams};
	use seed_primitives::xrpl::{XrplAccountId, XrplTxTicketSequence};
	use seed_primitives::{Balance, CollectionUuid, TokenId};
	use sp_core::{Get, H160};

	#[frame_support::storage_alias]
	pub type NfiData<T: Config> = StorageDoubleMap<
		pallet_nfi::Pallet<T>,
		Twox64Concat,
		TokenId,
		Twox64Concat,
		NFISubType,
		NFIDataType<<T as pallet_nfi::Config>::MaxDataLength>,
	>;

	#[frame_support::storage_alias]
	pub type NfiEnabled<T: Config> = StorageDoubleMap<
		pallet_nfi::Pallet<T>,
		Twox64Concat,
		CollectionUuid,
		Twox64Concat,
		NFISubType,
		bool,
		ValueQuery,
	>;

	pub fn migrate<T: frame_system::Config + pallet_nfi::Config + pallet_evm_chain_id::Config>(
	) -> Weight {
		log::info!(target: "Migration", "Nfi: migrating multi door support");
		let mut weight: Weight = Weight::zero();

		// Chain Id
		weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().reads(1));
		let chain_id = ChainId::<T>::get();

		// Migrate NFI Data
		for (token_id, sub_type, nfi_data) in NfiData::<T>::iter() {
			weight = weight
				.saturating_add(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1));
			let multi_chain_token = MultiChainTokenId {
				chain_id,
				collection_id: GenericCollectionId::U32(token_id.0),
				serial_number: GenericSerialNumber::U32(token_id.1),
			};
			NewNfiData::<T>::insert(&multi_chain_token, sub_type, nfi_data);
		}

		// Migrate NFIEnabled
		for (collection_id, sub_type, enabled) in NfiEnabled::<T>::iter() {
			weight = weight
				.saturating_add(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1));
			let collection_id = GenericCollectionId::U32(collection_id);
			NewNfiEnabled::<T>::insert((chain_id, collection_id), sub_type, enabled);
		}

		log::info!(target: "Migration", "Nfi: migrating to cross chain support successful");
		weight
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::new_test_ext;
		use crate::migrations::Map;
		use codec::{Decode, Encode};
		use frame_support::{BoundedVec, StorageHasher};
		use scale_info::TypeInfo;
		use sp_core::H256;

		#[test]
		fn migrate_with_data() {
			new_test_ext().execute_with(|| {
				// Setup storage
				StorageVersion::new(0).put::<Nfi>();

				let token_id: TokenId = (1124, 123);
				let mut key_1 = Twox64Concat::hash(&token_id.encode());
				let sub_type = NFISubType::NFI;
				let key_2 = Twox64Concat::hash(&(sub_type).encode());
				key_1.extend_from_slice(&key_2);
				let data = NFIDataType::NFI(NFIMatrix {
					metadata_link: BoundedVec::truncate_from(vec![1, 2, 3]),
					verification_hash: H256::from_low_u64_be(123),
				});
				Map::unsafe_storage_put::<
					NFIDataType<<Runtime as pallet_nfi::Config>::MaxDataLength>,
				>(b"Nfi", b"NfiData", &key_1, data.clone());

				let token_id_2: TokenId = (1124, 556);
				let mut key_1 = Twox64Concat::hash(&token_id_2.encode());
				let sub_type = NFISubType::NFI;
				let key_2 = Twox64Concat::hash(&(sub_type).encode());
				key_1.extend_from_slice(&key_2);
				let data_2 = NFIDataType::NFI(NFIMatrix {
					metadata_link: BoundedVec::truncate_from(vec![2, 4, 6]),
					verification_hash: H256::from_low_u64_be(246),
				});
				Map::unsafe_storage_put::<
					NFIDataType<<Runtime as pallet_nfi::Config>::MaxDataLength>,
				>(b"Nfi", b"NfiData", &key_1, data_2.clone());

				let collection_id: CollectionUuid = 123;
				let mut key_1 = Twox64Concat::hash(&collection_id.encode());
				let sub_type = NFISubType::NFI;
				let key_2 = Twox64Concat::hash(&(sub_type).encode());
				key_1.extend_from_slice(&key_2);
				Map::unsafe_storage_put::<bool>(b"Nfi", b"NfiEnabled", &key_1, true);

				let collection_id_2: CollectionUuid = 556;
				let mut key_1 = Twox64Concat::hash(&collection_id_2.encode());
				let sub_type = NFISubType::NFI;
				let key_2 = Twox64Concat::hash(&(sub_type).encode());
				key_1.extend_from_slice(&key_2);
				Map::unsafe_storage_put::<bool>(b"Nfi", b"NfiEnabled", &key_1, false);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();
				assert_eq!(Nfi::on_chain_storage_version(), 1);

				// Verify NfiData
				let chain_id = ChainId::<Runtime>::get();
				let multi_chain_token = MultiChainTokenId {
					chain_id,
					collection_id: GenericCollectionId::U32(token_id.0),
					serial_number: GenericSerialNumber::U32(token_id.1),
				};
				let migrated_data = NewNfiData::<Runtime>::get(&multi_chain_token, sub_type);
				assert_eq!(migrated_data, Some(data));

				let multi_chain_token_2 = MultiChainTokenId {
					chain_id,
					collection_id: GenericCollectionId::U32(token_id_2.0),
					serial_number: GenericSerialNumber::U32(token_id_2.1),
				};
				let migrated_data_2 = NewNfiData::<Runtime>::get(&multi_chain_token_2, sub_type);
				assert_eq!(migrated_data_2, Some(data_2));

				// Verify NfiEnabled
				let collection_id = GenericCollectionId::U32(collection_id);
				let migrated_enabled =
					NewNfiEnabled::<Runtime>::get((chain_id, collection_id), sub_type);
				assert_eq!(migrated_enabled, true);

				let collection_id_2 = GenericCollectionId::U32(collection_id_2);
				let migrated_enabled_2 =
					NewNfiEnabled::<Runtime>::get((chain_id, collection_id_2), sub_type);
				assert_eq!(migrated_enabled_2, false);
			});
		}
	}
}
