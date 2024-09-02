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

		if onchain == 2 {
			log::info!(target: "Migration", "XRPLBridge: Migrating from on-chain version 2 to on-chain version 3.");
			weight += v3::migrate::<Runtime>();

			StorageVersion::new(3).put::<XRPLBridge>();

			log::info!(target: "Migration", "XRPLBridge: Migration successfully completed.");
		} else {
			log::info!(target: "Migration", "XRPLBridge: No migration was done, however migration code needs to be removed.");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
		log::info!(target: "Migration", "XRPLBridge: Upgrade to v3 Pre Upgrade.");
		let onchain = XRPLBridge::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 3 {
			return Ok(Vec::new());
		}
		assert_eq!(onchain, 2);

		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), DispatchError> {
		log::info!(target: "Migration", "XRPLBridge: Upgrade to v3 Post Upgrade.");
		let current = XRPLBridge::current_storage_version();
		let onchain = XRPLBridge::on_chain_storage_version();
		assert_eq!(current, 3);
		assert_eq!(onchain, 3);
		Ok(())
	}
}

#[allow(dead_code)]
#[allow(unused_imports)]
pub mod v3 {
	use super::*;
	use crate::migrations::{Map, Value};
	use codec::{Decode, Encode, MaxEncodedLen};
	use frame_support::{
		sp_runtime::RuntimeDebug,
		storage_alias,
		weights::{constants::RocksDbWeight, Weight},
		BoundedVec, StorageHasher, Twox64Concat,
	};
	use pallet_xrpl_bridge::{
		types::{DelayedPaymentId, DelayedWithdrawal, WithdrawTransaction, XrpWithdrawTransaction},
		DelayedPayments,
	};
	use scale_info::TypeInfo;
	use sp_core::{Get, H160};

	type AccountId = <Runtime as frame_system::Config>::AccountId;

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct OldDelayedWithdrawal<AccountId> {
		pub sender: AccountId,
		pub destination_tag: Option<u32>,
		pub withdraw_tx: XrpWithdrawTransaction,
	}

	pub fn migrate<T: frame_system::Config + pallet_xrpl_bridge::Config>() -> Weight
	where
		AccountId: From<H160>,
	{
		log::info!(target: "Migration", "XRPLBridge: migrating DelayedPayments");
		let mut weight: Weight = Weight::zero();

		DelayedPayments::<T>::translate::<OldDelayedWithdrawal<T::AccountId>, _>(
			|_, delayed_withdrawal| {
				weight = weight.saturating_add(
					<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1),
				);
				let new_withdrawal = DelayedWithdrawal::<T::AccountId> {
					sender: delayed_withdrawal.sender,
					destination_tag: delayed_withdrawal.destination_tag,
					withdraw_tx: WithdrawTransaction::XRP(delayed_withdrawal.withdraw_tx),
				};

				Some(new_withdrawal)
			},
		);

		log::info!(target: "Migration", "XRPLBridge: successfully migrated DelayedPayments");

		weight
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::{create_account, new_test_ext};

		#[test]
		fn migration_test_1() {
			new_test_ext().execute_with(|| {
				// Setup storage
				StorageVersion::new(2).put::<XRPLBridge>();

				// token locks with no listings
				let old_delayed_withdrawal = OldDelayedWithdrawal::<AccountId> {
					sender: create_account::<AccountId>(123),
					destination_tag: Some(1),
					withdraw_tx: XrpWithdrawTransaction {
						tx_fee: 1,
						tx_nonce: 2,
						tx_ticket_sequence: 3,
						amount: 4,
						destination: Default::default(),
					},
				};
				let payment_id: DelayedPaymentId = 1;
				let key = Twox64Concat::hash(&(payment_id).encode());
				Map::unsafe_storage_put::<OldDelayedWithdrawal<AccountId>>(
					b"XRPLBridge",
					b"DelayedPayments",
					&key,
					old_delayed_withdrawal,
				);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();
				assert_eq!(XRPLBridge::on_chain_storage_version(), 3);

				let delayed_withdrawal = Map::unsafe_storage_get::<DelayedWithdrawal<AccountId>>(
					b"XRPLBridge",
					b"DelayedPayments",
					&key,
				);
				let expected_withdrawal = DelayedWithdrawal::<AccountId> {
					sender: create_account::<AccountId>(123),
					destination_tag: Some(1),
					withdraw_tx: WithdrawTransaction::XRP(XrpWithdrawTransaction {
						tx_fee: 1,
						tx_nonce: 2,
						tx_ticket_sequence: 3,
						amount: 4,
						destination: Default::default(),
					}),
				};
				assert_eq!(delayed_withdrawal, Some(expected_withdrawal));
			});
		}

		#[test]
		fn migration_test_2() {
			new_test_ext().execute_with(|| {
				// Setup storage
				StorageVersion::new(2).put::<XRPLBridge>();

				// token locks with no listings
				let old_delayed_withdrawal = OldDelayedWithdrawal::<AccountId> {
					sender: create_account::<AccountId>(345),
					destination_tag: None,
					withdraw_tx: XrpWithdrawTransaction {
						tx_fee: 11,
						tx_nonce: 22,
						tx_ticket_sequence: 33,
						amount: 44,
						destination: Default::default(),
					},
				};
				let payment_id: DelayedPaymentId = 2;
				let key = Twox64Concat::hash(&(payment_id).encode());
				Map::unsafe_storage_put::<OldDelayedWithdrawal<AccountId>>(
					b"XRPLBridge",
					b"DelayedPayments",
					&key,
					old_delayed_withdrawal,
				);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();
				assert_eq!(XRPLBridge::on_chain_storage_version(), 3);

				let delayed_withdrawal = Map::unsafe_storage_get::<DelayedWithdrawal<AccountId>>(
					b"XRPLBridge",
					b"DelayedPayments",
					&key,
				);
				let expected_withdrawal = DelayedWithdrawal::<AccountId> {
					sender: create_account::<AccountId>(345),
					destination_tag: None,
					withdraw_tx: WithdrawTransaction::XRP(XrpWithdrawTransaction {
						tx_fee: 11,
						tx_nonce: 22,
						tx_ticket_sequence: 33,
						amount: 44,
						destination: Default::default(),
					}),
				};
				assert_eq!(delayed_withdrawal, Some(expected_withdrawal));
			});
		}
	}
}
