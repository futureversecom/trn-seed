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

use crate::{Runtime, Weight, Xls20};
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
		let current = Xls20::current_storage_version();
		let onchain = Xls20::on_chain_storage_version();
		log::info!(target: "Migration", "Xls20: Running migration with current storage version {current:?} / on-chain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(2);

		if onchain == 0 {
			log::info!(target: "Migration", "Xls20: Migrating from on-chain version 0 to on-chain version 1.");
			weight += v1::migrate::<Runtime>();

			StorageVersion::new(3).put::<Xls20>();

			log::info!(target: "Migration", "Xls20: Migration successfully completed.");
		} else {
			log::info!(target: "Migration", "Xls20: No migration was done, however migration code needs to be removed.");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
		log::info!(target: "Migration", "Xls20: Upgrade to v1 Pre Upgrade.");
		let onchain = Xls20::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 1 {
			return Ok(Vec::new());
		}
		assert_eq!(onchain, 0);

		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), DispatchError> {
		log::info!(target: "Migration", "Xls20: Upgrade to v1 Post Upgrade.");
		let current = Xls20::current_storage_version();
		let onchain = Xls20::on_chain_storage_version();
		assert_eq!(current, 1);
		assert_eq!(onchain, 1);
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
		sp_runtime::RuntimeDebug,
		storage_alias,
		weights::{constants::RocksDbWeight, Weight},
		BoundedVec, StorageHasher, Twox64Concat,
	};
	use pallet_xls20::{Xls20TokenId, Xls20TokenMap};
	use pallet_xrpl_bridge::{
		types::{DelayedPaymentId, DelayedWithdrawal, WithdrawTransaction, XrpWithdrawTransaction},
		DelayedPayments,
	};
	use scale_info::TypeInfo;
	use sp_core::{Get, H160};

	type AccountId = <Runtime as frame_system::Config>::AccountId;

	pub fn migrate<T: frame_system::Config + pallet_xls20::Config>() -> Weight
	where
		AccountId: From<H160>,
	{
		log::info!(target: "Migration", "Xls20: migrating Xls20TokenMap");
		let mut weight: Weight = Weight::zero();
		// Xls20TokenMap::<T>::remove_all(None);
		Xls20TokenMap::<T>::translate::<[u8; 64], _>(|_, _, token_id| {
			weight = weight
				.saturating_add(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1));
			let mut new_token_id = [0; 32];
			new_token_id.copy_from_slice(&token_id[..32]);
			Some(new_token_id)
		});

		log::info!(target: "Migration", "Xls20: successfully migrated Xls20TokenMap");

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
				StorageVersion::new(2).put::<Xls20>();

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
					b"Xls20",
					b"DelayedPayments",
					&key,
					old_delayed_withdrawal,
				);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();
				assert_eq!(Xls20::on_chain_storage_version(), 3);

				let delayed_withdrawal = Map::unsafe_storage_get::<DelayedWithdrawal<AccountId>>(
					b"Xls20",
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

		// #[test]
		// fn migration_test_2() {
		//     new_test_ext().execute_with(|| {
		//         // Setup storage
		//         StorageVersion::new(2).put::<Xls20>();
		//
		//         // token locks with no listings
		//         let old_delayed_withdrawal = OldDelayedWithdrawal::<AccountId> {
		//             sender: create_account::<AccountId>(345),
		//             destination_tag: None,
		//             withdraw_tx: XrpWithdrawTransaction {
		//                 tx_fee: 11,
		//                 tx_nonce: 22,
		//                 tx_ticket_sequence: 33,
		//                 amount: 44,
		//                 destination: Default::default(),
		//             },
		//         };
		//         let payment_id: DelayedPaymentId = 2;
		//         let key = Twox64Concat::hash(&(payment_id).encode());
		//         Map::unsafe_storage_put::<OldDelayedWithdrawal<AccountId>>(
		//             b"Xls20",
		//             b"DelayedPayments",
		//             &key,
		//             old_delayed_withdrawal,
		//         );
		//
		//         // Do runtime upgrade
		//         Upgrade::on_runtime_upgrade();
		//         assert_eq!(Xls20::on_chain_storage_version(), 3);
		//
		//         let delayed_withdrawal = Map::unsafe_storage_get::<DelayedWithdrawal<AccountId>>(
		//             b"Xls20",
		//             b"DelayedPayments",
		//             &key,
		//         );
		//         let expected_withdrawal = DelayedWithdrawal::<AccountId> {
		//             sender: create_account::<AccountId>(345),
		//             destination_tag: None,
		//             withdraw_tx: WithdrawTransaction::XRP(XrpWithdrawTransaction {
		//                 tx_fee: 11,
		//                 tx_nonce: 22,
		//                 tx_ticket_sequence: 33,
		//                 amount: 44,
		//                 destination: Default::default(),
		//             }),
		//         };
		//         assert_eq!(delayed_withdrawal, Some(expected_withdrawal));
		//     });
		// }
	}
}
