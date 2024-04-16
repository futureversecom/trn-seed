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

use crate::{Crowdsale, Runtime, Weight};
use frame_support::{
	dispatch::GetStorageVersion,
	traits::{OnRuntimeUpgrade, StorageVersion},
};
#[allow(unused_imports)]
use sp_std::vec::Vec;

pub struct Upgrade;

impl OnRuntimeUpgrade for Upgrade {
	fn on_runtime_upgrade() -> Weight {
		let current = Crowdsale::current_storage_version();
		let onchain = Crowdsale::on_chain_storage_version();
		log::info!(target: "Migration", "Crowdsale: Running migration with current storage version {current:?} / on-chain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(2);

		if onchain == 0 {
			log::info!(target: "Migration", "Crowdsale: Migrating from on-chain version 0 to on-chain version 1.");
			weight += v1::migrate::<Runtime>();

			StorageVersion::new(1).put::<Crowdsale>();

			log::info!(target: "Migration", "Crowdsale: Migration successfully completed.");
		} else {
			log::info!(target: "Migration", "Crowdsale: No migration was done, however migration code needs to be removed.");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		v1::pre_upgrade()?;
		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), &'static str> {
		v1::post_upgrade()?;
		Ok(())
	}
}

#[allow(dead_code)]
#[allow(unused_imports)]
pub mod v1 {
	use super::*;
	use crate::migrations::{Map, Value};
	use codec::{Decode, Encode, MaxEncodedLen};
	use frame_support::{
		sp_runtime::RuntimeDebug, storage_alias, weights::Weight, BoundedVec, StorageHasher,
		Twox64Concat,
	};
	use frame_system::pallet_prelude::BlockNumberFor;
	use pallet_crowdsale::{
		types::{SaleId, SaleInformation, SaleStatus},
		SaleInfo, SaleParticipation,
	};
	use scale_info::TypeInfo;
	use seed_primitives::{AssetId, Balance, CollectionUuid};

	type AccountId = <Runtime as frame_system::Config>::AccountId;
	type BlockNumber = BlockNumberFor<Runtime>;

	/// Information about a fixed price listing
	#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
	pub struct OldSaleInformation<AccountId, BlockNumber> {
		pub status: SaleStatus<BlockNumber>,
		pub admin: AccountId,
		pub vault: AccountId,
		pub payment_asset_id: AssetId,
		pub reward_collection_id: CollectionUuid,
		pub soft_cap_price: Balance,
		pub funds_raised: Balance,
		pub voucher_asset_id: AssetId,
		pub duration: BlockNumber,
	}

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Crowdsale: Upgrade to v1 Pre Upgrade.");
		let onchain = Crowdsale::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 1 {
			return Ok(())
		}
		assert_eq!(onchain, 0);

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Crowdsale: Upgrade to v1 Post Upgrade.");
		let current = Crowdsale::current_storage_version();
		let onchain = Crowdsale::on_chain_storage_version();
		assert_eq!(current, 1);
		assert_eq!(onchain, 1);
		Ok(())
	}

	pub fn migrate<T: pallet_nft::Config + pallet_marketplace::Config>() -> Weight
	where
		AccountId: From<sp_core::H160>,
	{
		log::info!(target: "Migration", "Marketplace: migrating listing tokens");
		let mut weight = Weight::zero();

		SaleInfo::<Runtime>::translate::<OldSaleInformation<AccountId, BlockNumber>, _>(
			|sale_id, sale_info| {
				// Get total number of participants in SaleParticipation
				let participant_count =
					SaleParticipation::<Runtime>::iter_prefix(sale_id).count() as u64;

				// Reads: SaleInfo + N * SaleParticipation
				// Writes: SaleInfo
				weight += <Runtime as frame_system::Config>::DbWeight::get()
					.reads_writes(1 + participant_count, 1);

				let new_sale_info = SaleInformation {
					status: sale_info.status,
					admin: sale_info.admin,
					vault: sale_info.vault,
					payment_asset_id: sale_info.payment_asset_id,
					reward_collection_id: sale_info.reward_collection_id,
					soft_cap_price: sale_info.soft_cap_price,
					funds_raised: sale_info.funds_raised,
					participant_count,
					voucher_asset_id: sale_info.voucher_asset_id,
					duration: sale_info.duration,
				};

				Some(new_sale_info)
			},
		);

		log::info!(target: "Migration", "Crowdsale: successfully migrated SaleInfo");

		weight
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::new_test_ext;
		use sp_core::H160;
		use sp_runtime::Permill;

		fn create_account(seed: u64) -> AccountId {
			AccountId::from(H160::from_low_u64_be(seed))
		}

		#[test]
		fn migration_test() {
			new_test_ext().execute_with(|| {
				// Setup storage
				StorageVersion::new(0).put::<Crowdsale>();

				// SaleInfo with no participation's
				let sale_id_key_1 = Twox64Concat::hash(&(1 as SaleId).encode());
				let sale_info_1 = OldSaleInformation {
					status: SaleStatus::Enabled(1),
					admin: create_account(2),
					vault: create_account(3),
					payment_asset_id: 4,
					reward_collection_id: 5,
					soft_cap_price: 6,
					funds_raised: 7,
					voucher_asset_id: 8,
					duration: 9,
				};

				Map::unsafe_storage_put::<OldSaleInformation<AccountId, BlockNumber>>(
					b"Crowdsale",
					b"SaleInfo",
					&sale_id_key_1,
					sale_info_1.clone(),
				);

				// SaleParticipation with 50 participation's
				let sale_id_2: SaleId = 2;
				let total_participants = 50;
				for i in 0..total_participants {
					let who = create_account(i as u64);
					let participation: Balance = 10;
					SaleParticipation::<Runtime>::insert(sale_id_2, who, participation);
				}
				let sale_id_key_2 = Twox64Concat::hash(&(sale_id_2).encode());
				let sale_info_2 = OldSaleInformation {
					status: SaleStatus::Enabled(1),
					admin: create_account(2),
					vault: create_account(3),
					payment_asset_id: 4,
					reward_collection_id: 5,
					soft_cap_price: 6,
					funds_raised: 7,
					voucher_asset_id: 8,
					duration: 9,
				};

				Map::unsafe_storage_put::<OldSaleInformation<AccountId, BlockNumber>>(
					b"Crowdsale",
					b"SaleInfo",
					&sale_id_key_2,
					sale_info_2.clone(),
				);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();
				assert_eq!(Crowdsale::on_chain_storage_version(), 1);

				let expected_sale_info_1 = SaleInformation {
					status: SaleStatus::Enabled(1),
					admin: create_account(2),
					vault: create_account(3),
					payment_asset_id: 4,
					reward_collection_id: 5,
					soft_cap_price: 6,
					funds_raised: 7,
					participant_count: 0, // 0 as we have no participations
					voucher_asset_id: 8,
					duration: 9,
				};
				assert_eq!(
					Map::unsafe_storage_get::<SaleInformation<AccountId, BlockNumber>>(
						b"Crowdsale",
						b"SaleInfo",
						&sale_id_key_1,
					),
					Some(expected_sale_info_1)
				);

				let expected_sale_info_2 = SaleInformation {
					status: SaleStatus::Enabled(1),
					admin: create_account(2),
					vault: create_account(3),
					payment_asset_id: 4,
					reward_collection_id: 5,
					soft_cap_price: 6,
					funds_raised: 7,
					participant_count: total_participants as u64, // From SaleParticipation map
					voucher_asset_id: 8,
					duration: 9,
				};
				assert_eq!(
					Map::unsafe_storage_get::<SaleInformation<AccountId, BlockNumber>>(
						b"Crowdsale",
						b"SaleInfo",
						&sale_id_key_2,
					),
					Some(expected_sale_info_2)
				);
			});
		}
	}
}
