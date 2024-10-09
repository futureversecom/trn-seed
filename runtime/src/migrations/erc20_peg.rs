// Copyright 2022-2024 Futureverse Corporation Limited
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
use sp_runtime::DispatchError;
#[allow(unused_imports)]
use sp_std::vec::Vec;

pub struct Upgrade;

impl OnRuntimeUpgrade for Upgrade {
	fn on_runtime_upgrade() -> Weight {
		let current = Erc20Peg::current_storage_version();
		let onchain = Erc20Peg::on_chain_storage_version();
		log::info!(target: "Migration", "Erc20Peg: Running migration with current storage version {current:?} / on-chain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(2);

		if onchain < 1 {
			log::info!(target: "Migration", "XRPLBridge: Migrating from on-chain version {onchain:?} to on-chain version {current:?}.");
			weight += v1::migrate::<Runtime>();
			weight += v1::migrate_k::<Runtime>();

			StorageVersion::new(1).put::<Erc20Peg>();

			log::info!(target: "Migration", "Erc20Peg: Migration successfully completed.");
		} else {
			log::info!(target: "Migration", "Erc20Peg: No migration was done, however migration code needs to be removed.");
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
		log::info!(target: "Migration", "Erc20Peg: Upgrade to v1 Pre Upgrade.");
		let onchain = Erc20Peg::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 1 {
			return Ok(Vec::new());
		}
		assert_eq!(onchain, 0);

		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), DispatchError> {
		log::info!(target: "Migration", "Erc20Peg: Upgrade to v1 Post Upgrade.");
		let current = Erc20Peg::current_storage_version();
		let onchain = Erc20Peg::on_chain_storage_version();
		assert_eq!(current, 1);
		assert_eq!(onchain, 1);
		Ok(())
	}
}

#[allow(dead_code)]
#[allow(unused_imports)]
pub mod v1 {
	use super::*;
	use crate::{
		migrations::{Map, Value},
		sp_api_hidden_includes_construct_runtime::hidden_include::IterableStorageMap,
	};
	use codec::{Decode, Encode, MaxEncodedLen};
	use frame_support::{
		sp_runtime::RuntimeDebug, storage_alias, weights::Weight, BoundedVec, StorageHasher,
		Twox64Concat, WeakBoundedVec,
	};
	use pallet_erc20_peg::{
		types::{
			DelayedPaymentId, DelayedPaymentSchedule, Erc20DepositEvent, PendingPayment,
			WithdrawMessage,
		},
		DelayedPayments,
	};
	use scale_info::TypeInfo;
	use seed_primitives::{AssetId, Balance, CollectionUuid};
	use sp_core::{Get, H160};

	use frame_system::pallet_prelude::BlockNumberFor;

	type AccountId = <Runtime as frame_system::Config>::AccountId;

	#[derive(Clone, Encode, Decode, RuntimeDebug, PartialEq, TypeInfo, MaxEncodedLen)]
	pub enum OldPendingPayment {
		/// A deposit event (deposit_event, tx_hash)
		Deposit(Erc20DepositEvent),
		/// A withdrawal (withdrawal_message)
		Withdrawal(WithdrawMessage),
	}

	pub fn migrate<T: frame_system::Config + pallet_erc20_peg::Config>() -> Weight
	where
		AccountId: From<H160>,
	{
		log::info!(target: "Migration", "ERC20Peg:[DelayedPayments] Migrating from on-chain version 0 to on-chain version 1");
		let mut weight: Weight = Weight::zero();
		let default_account = AccountId::default();

		// Get total number of participants in SaleParticipation
		DelayedPayments::<T>::translate::<OldPendingPayment, _>(|_, pending_payment| {
			// Reads: DelayedPayments
			// Writes: DelayedPayments
			weight += <Runtime as frame_system::Config>::DbWeight::get().reads_writes(1, 1);
			let new_pending_payment = match pending_payment {
				OldPendingPayment::Deposit(deposit) => PendingPayment::Deposit(deposit),
				OldPendingPayment::Withdrawal(withdrawal_message) => {
					PendingPayment::Withdrawal((default_account, withdrawal_message))
				},
			};
			Some(new_pending_payment)
		});

		log::info!(target: "Migration", "ERC20Peg: successfully migrated DelayedPayments");

		weight
	}

	type OldDelayedPaymentSchedule =
		BoundedVec<DelayedPaymentId, <Runtime as pallet_erc20_peg::Config>::MaxDelaysPerBlock>;
	type NewDelayedPaymentSchedule =
		WeakBoundedVec<DelayedPaymentId, <Runtime as pallet_erc20_peg::Config>::MaxDelaysPerBlock>;

	pub fn migrate_k<T: frame_system::Config + pallet_erc20_peg::Config>() -> Weight {
		log::info!(target: "Migration", "ERC20Peg:[DelayedPaymentSchedule] Migrating from on-chain version 0 to on-chain version 1");

		let mut weight: Weight = Weight::zero();

		DelayedPaymentSchedule::<T>::translate_values::<
			BoundedVec<DelayedPaymentId, <Runtime as pallet_erc20_peg::Config>::MaxDelaysPerBlock>,
			_,
		>(|old| {
			weight += <Runtime as frame_system::Config>::DbWeight::get().reads_writes(1, 1);
			Some(WeakBoundedVec::force_from(old.into_inner(), None))
		});

		log::info!(target: "Migration", "ERC20Peg: successfully migrated DelayedPaymentSchedule");

		weight
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::new_test_ext;
		use pallet_erc20_peg::types::DelayedPaymentId;
		use sp_core::{H160, U256};
		use sp_runtime::Permill;

		fn create_account(seed: u64) -> AccountId {
			AccountId::from(H160::from_low_u64_be(seed))
		}

		#[test]
		fn migration_test() {
			new_test_ext().execute_with(|| {
				// Setup storage
				StorageVersion::new(0).put::<Erc20Peg>();

				// Deposit pending payment
				let payment_id_key_1 = Twox64Concat::hash(&(1 as DelayedPaymentId).encode());
				let pending_payment_1 = OldPendingPayment::Deposit(Erc20DepositEvent {
					token_address: create_account(12).into(),
					amount: U256::from(13),
					beneficiary: create_account(14).into(),
				});

				Map::unsafe_storage_put::<OldPendingPayment>(
					b"Erc20Peg",
					b"DelayedPayments",
					&payment_id_key_1,
					pending_payment_1.clone(),
				);

				// Withdrawal pending payment
				let payment_id_key_2 = Twox64Concat::hash(&(2 as DelayedPaymentId).encode());
				let pending_payment_2 = OldPendingPayment::Withdrawal(WithdrawMessage {
					token_address: create_account(15).into(),
					amount: U256::from(16),
					beneficiary: create_account(17).into(),
				});

				Map::unsafe_storage_put::<OldPendingPayment>(
					b"Erc20Peg",
					b"DelayedPayments",
					&payment_id_key_2,
					pending_payment_2.clone(),
				);

				// Do runtime upgrade
				Upgrade::on_runtime_upgrade();
				assert_eq!(Erc20Peg::on_chain_storage_version(), 1);

				// Verify storage
				let expected_pending_payment_1 = PendingPayment::Deposit(Erc20DepositEvent {
					token_address: create_account(12).into(),
					amount: U256::from(13),
					beneficiary: create_account(14).into(),
				});
				assert_eq!(
					Map::unsafe_storage_get::<PendingPayment<AccountId>>(
						b"Erc20Peg",
						b"DelayedPayments",
						&payment_id_key_1,
					),
					Some(expected_pending_payment_1)
				);

				// Withdrawal should now be a tuple with default AccountId
				let expected_pending_payment_2 = PendingPayment::Withdrawal((
					AccountId::default(),
					WithdrawMessage {
						token_address: create_account(15).into(),
						amount: U256::from(16),
						beneficiary: create_account(17).into(),
					},
				));
				assert_eq!(
					Map::unsafe_storage_get::<PendingPayment<AccountId>>(
						b"Erc20Peg",
						b"DelayedPayments",
						&payment_id_key_2,
					),
					Some(expected_pending_payment_2)
				);
			});
		}
	}

	#[cfg(test)]
	mod tests {
		use super::*;

		use codec::Encode;
		use pallet_erc20_peg::DelayedPaymentSchedule;

		use crate::migrations::{tests::new_test_ext, Map};

		#[test]
		fn migration_test() {
			new_test_ext().execute_with(|| {
				StorageVersion::new(0).put::<Erc20Peg>();

				let one: BlockNumberFor<Runtime> = 1;
				let two: BlockNumberFor<Runtime> = 2;

				let block_key_1 = Twox64Concat::hash(&one.encode());
				let block_key_2 = Twox64Concat::hash(&two.encode());

				let delayed_payment_1: OldDelayedPaymentSchedule =
					BoundedVec::truncate_from(vec![1, 2, 3, 4, 5]);
				let delayed_payment_2: OldDelayedPaymentSchedule =
					BoundedVec::truncate_from(vec![6, 7, 8, 9, 10]);

				Map::unsafe_storage_put::<OldDelayedPaymentSchedule>(
					b"Erc20Peg",
					b"DelayedPaymentSchedule",
					&block_key_1,
					delayed_payment_1.clone(),
				);
				Map::unsafe_storage_put::<OldDelayedPaymentSchedule>(
					b"Erc20Peg",
					b"DelayedPaymentSchedule",
					&block_key_2,
					delayed_payment_2.clone(),
				);

				Upgrade::on_runtime_upgrade();
				assert_eq!(Erc20Peg::on_chain_storage_version(), 1);

				let expected_delayed_payment_1: NewDelayedPaymentSchedule =
					WeakBoundedVec::force_from(delayed_payment_1.into_inner(), None);
				let expected_delayed_payment_2: NewDelayedPaymentSchedule =
					WeakBoundedVec::force_from(delayed_payment_2.into_inner(), None);

				assert_eq!(
					Map::unsafe_storage_get::<NewDelayedPaymentSchedule>(
						b"Erc20Peg",
						b"DelayedPaymentSchedule",
						&block_key_1
					),
					Some(expected_delayed_payment_1)
				);
				assert_eq!(
					Map::unsafe_storage_get::<NewDelayedPaymentSchedule>(
						b"Erc20Peg",
						b"DelayedPaymentSchedule",
						&block_key_2
					),
					Some(expected_delayed_payment_2)
				);
			})
		}
	}
}
