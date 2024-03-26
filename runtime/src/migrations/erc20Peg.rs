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

use crate::{Erc20Peg, Runtime, Weight};
use frame_support::{
	dispatch::GetStorageVersion,
	traits::{OnRuntimeUpgrade, StorageVersion},
};
#[allow(unused_imports)]
use sp_std::vec::Vec;

pub struct Upgrade;

impl OnRuntimeUpgrade for Upgrade {
	fn on_runtime_upgrade() -> Weight {
		let current = Erc20Peg::current_storage_version();
		let onchain = Erc20Peg::on_chain_storage_version();
		log::info!(target: "Migration", "ERC20Peg: Running migration with current storage version {current:?} / on-chain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(2);

		if onchain == 0 {
			log::info!(target: "Migration", "Erc20Peg: Migrating from on-chain version 0 to on-chain version 1.");
			weight += v1::migrate::<Runtime>();

			StorageVersion::new(1).put::<Erc20Peg>();

			log::info!(target: "Migration", "Erc20Peg: Migration successfully completed.");
		} else {
			log::info!(target: "Migration", "Erc20Peg: No migration was done, however migration code needs to be removed.");
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
	use pallet_erc20_peg::{
		types::{Erc20DepositEvent, PendingPayment, WithdrawMessage},
		DelayedPayments,
	};
	use scale_info::TypeInfo;
	use seed_primitives::{AssetId, Balance, CollectionUuid};

	type AccountId = <Runtime as frame_system::Config>::AccountId;
	type BlockNumber = <Runtime as frame_system::Config>::BlockNumber;

	#[derive(Clone, Encode, Decode, RuntimeDebug, PartialEq, TypeInfo, MaxEncodedLen)]
	pub enum OldPendingPayment {
		/// A deposit event (deposit_event, tx_hash)
		Deposit(Erc20DepositEvent),
		/// A withdrawal (withdrawal_message)
		Withdrawal(WithdrawMessage),
	}

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Erc20Peg: Upgrade to v1 Pre Upgrade.");
		let onchain = Erc20Peg::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 1 {
			return Ok(())
		}
		assert_eq!(onchain, 0);

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Erc20Peg: Upgrade to v1 Post Upgrade.");
		let current = Erc20Peg::current_storage_version();
		let onchain = Erc20Peg::on_chain_storage_version();
		assert_eq!(current, 1);
		assert_eq!(onchain, 1);
		Ok(())
	}

	pub fn migrate<T: pallet_erc20_peg::Config>() -> Weight {
		log::info!(target: "Migration", "ERC20Peg: migrating listing tokens");
		let mut weight = Weight::zero();

		// Get total number of participants in SaleParticipation
		DelayedPayments::<Runtime>::translate::<OldPendingPayment, _>(
			|delayed_payment_id, pending_payment| {
				let default_account = AccountId::default();
				// Reads: SaleInfo + N * SaleParticipation
				// Writes: SaleInfo
				weight += <Runtime as frame_system::Config>::DbWeight::get().reads_writes(1, 1);
				return match pending_payment {
					PendingPayment::Deposit(deposit) => pending_payment.clone(),
					PendingPayment::Withdrawal(withdrawal_message) => {
						let newPendingPayment: PendingPayment =
							PendingPayment::Withdrawal((default_account, withdrawal_message));
						newPendingPayment
					},
				}
			},
		);

		log::info!(target: "Migration", "ERC20Peg: successfully migrated DelayedPayments");

		weight
	}

	// #[cfg(test)]
	// mod tests {
	// 	use super::*;
	// 	use crate::migrations::tests::new_test_ext;
	// 	use sp_core::H160;
	// 	use sp_runtime::Permill;
	//
	// 	fn create_account(seed: u64) -> AccountId {
	// 		AccountId::from(H160::from_low_u64_be(seed))
	// 	}
	//
	// 	#[test]
	// 	fn migration_test() {
	// 		new_test_ext().execute_with(|| {
	// 			// Setup storage
	// 			StorageVersion::new(0).put::<Crowdsale>();
	//
	// 			// SaleInfo with no participation's
	// 			let sale_id_key_1 = Twox64Concat::hash(&(1 as SaleId).encode());
	// 			let sale_info_1 = OldSaleInformation {
	// 				status: SaleStatus::Enabled(1),
	// 				admin: create_account(2),
	// 				vault: create_account(3),
	// 				payment_asset_id: 4,
	// 				reward_collection_id: 5,
	// 				soft_cap_price: 6,
	// 				funds_raised: 7,
	// 				voucher_asset_id: 8,
	// 				duration: 9,
	// 			};
	//
	// 			Map::unsafe_storage_put::<OldSaleInformation<AccountId, BlockNumber>>(
	// 				b"Crowdsale",
	// 				b"SaleInfo",
	// 				&sale_id_key_1,
	// 				sale_info_1.clone(),
	// 			);
	//
	// 			// SaleParticipation with 50 participation's
	// 			let sale_id_2: SaleId = 2;
	// 			let total_participants = 50;
	// 			for i in 0..total_participants {
	// 				let who = create_account(i as u64);
	// 				let participation: Balance = 10;
	// 				SaleParticipation::<Runtime>::insert(sale_id_2, who, participation);
	// 			}
	// 			let sale_id_key_2 = Twox64Concat::hash(&(sale_id_2).encode());
	// 			let sale_info_2 = OldSaleInformation {
	// 				status: SaleStatus::Enabled(1),
	// 				admin: create_account(2),
	// 				vault: create_account(3),
	// 				payment_asset_id: 4,
	// 				reward_collection_id: 5,
	// 				soft_cap_price: 6,
	// 				funds_raised: 7,
	// 				voucher_asset_id: 8,
	// 				duration: 9,
	// 			};
	//
	// 			Map::unsafe_storage_put::<OldSaleInformation<AccountId, BlockNumber>>(
	// 				b"Crowdsale",
	// 				b"SaleInfo",
	// 				&sale_id_key_2,
	// 				sale_info_2.clone(),
	// 			);
	//
	// 			// Do runtime upgrade
	// 			Upgrade::on_runtime_upgrade();
	// 			assert_eq!(Crowdsale::on_chain_storage_version(), 1);
	//
	// 			let expected_sale_info_1 = SaleInformation {
	// 				status: SaleStatus::Enabled(1),
	// 				admin: create_account(2),
	// 				vault: create_account(3),
	// 				payment_asset_id: 4,
	// 				reward_collection_id: 5,
	// 				soft_cap_price: 6,
	// 				funds_raised: 7,
	// 				participant_count: 0, // 0 as we have no participations
	// 				voucher_asset_id: 8,
	// 				duration: 9,
	// 			};
	// 			assert_eq!(
	// 				Map::unsafe_storage_get::<SaleInformation<AccountId, BlockNumber>>(
	// 					b"Crowdsale",
	// 					b"SaleInfo",
	// 					&sale_id_key_1,
	// 				),
	// 				Some(expected_sale_info_1)
	// 			);
	//
	// 			let expected_sale_info_2 = SaleInformation {
	// 				status: SaleStatus::Enabled(1),
	// 				admin: create_account(2),
	// 				vault: create_account(3),
	// 				payment_asset_id: 4,
	// 				reward_collection_id: 5,
	// 				soft_cap_price: 6,
	// 				funds_raised: 7,
	// 				participant_count: total_participants as u64, // From SaleParticipation map
	// 				voucher_asset_id: 8,
	// 				duration: 9,
	// 			};
	// 			assert_eq!(
	// 				Map::unsafe_storage_get::<SaleInformation<AccountId, BlockNumber>>(
	// 					b"Crowdsale",
	// 					b"SaleInfo",
	// 					&sale_id_key_2,
	// 				),
	// 				Some(expected_sale_info_2)
	// 			);
	// 		});
	// 	}
	// }
}
