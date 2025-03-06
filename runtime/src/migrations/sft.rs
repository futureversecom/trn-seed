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

use crate::{Runtime, Sft, Weight};
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
		let current = Sft::current_storage_version();
		let onchain = Sft::on_chain_storage_version();
		log::info!(target: "Migration", "Sft: Running migration with current storage version {current:?} / on-chain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(2);

		if onchain != 0 {
			log::info!(
				target: "Migration",
				"Sft: No migration was done, This migration should be on top of storage version 0. Migration code needs to be removed."
			);
			return weight;
		}

		log::info!(target: "Migration", "Sft: Migrating from on-chain version {onchain:?} to on-chain version {current:?}.");
		weight += v1::migrate::<Runtime>();
		StorageVersion::new(1).put::<Sft>();
		log::info!(target: "Migration", "Sft: Migration successfully completed.");
		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
		log::info!(target: "Migration", "Sft: Upgrade to v1 Pre Upgrade.");
		let onchain = Sft::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 1 {
			return Ok(Vec::new());
		}
		assert_eq!(onchain, 0);

		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), DispatchError> {
		log::info!(target: "Migration", "Sft: Upgrade to v1 Post Upgrade.");
		let current = Sft::current_storage_version();
		let onchain = Sft::on_chain_storage_version();
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
	use codec::{Decode, Encode, MaxEncodedLen};
	use core::fmt::Debug;
	use frame_support::pallet_prelude::ValueQuery;
	use frame_support::weights::Weight;
	use frame_support::{storage_alias, CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound};
	use frame_support::{BoundedVec, Twox64Concat};
	use scale_info::TypeInfo;
	use seed_primitives::{Balance, CollectionUuid, SerialNumber, TokenId};
	use sp_core::{Get, H160};

	#[frame_support::storage_alias]
	pub type PendingIssuances<T: pallet_sft::Config> = StorageMap<
		pallet_sft::Pallet<T>,
		Twox64Concat,
		CollectionUuid,
		SftCollectionPendingIssuances<
			<T as frame_system::Config>::AccountId,
			<T as pallet_sft::Config>::MaxSerialsPerMint,
			<T as pallet_sft::Config>::MaxSftPendingIssuances,
		>,
		ValueQuery,
	>;

	type IssuanceId = u32;

	#[derive(
		PartialEqNoBound, RuntimeDebugNoBound, CloneNoBound, Encode, Decode, TypeInfo, MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(MaxSerialsPerMint))]
	pub struct SftPendingIssuance<MaxSerialsPerMint: Get<u32>> {
		pub issuance_id: IssuanceId,
		pub serial_numbers: BoundedVec<(SerialNumber, Balance), MaxSerialsPerMint>,
	}

	/// The state of a sft collection's pending issuances
	#[derive(
		PartialEqNoBound, RuntimeDebugNoBound, CloneNoBound, Encode, Decode, TypeInfo, MaxEncodedLen,
	)]
	#[codec(mel_bound(AccountId: MaxEncodedLen))]
	#[scale_info(skip_type_params(MaxSerialsPerMint, MaxPendingIssuances))]
	pub struct SftCollectionPendingIssuances<AccountId, MaxSerialsPerMint, MaxPendingIssuances>
	where
		AccountId: Debug + PartialEq + Clone,
		MaxSerialsPerMint: Get<u32>,
		MaxPendingIssuances: Get<u32>,
	{
		pub next_issuance_id: IssuanceId,
		pub pending_issuances: BoundedVec<
			(AccountId, BoundedVec<SftPendingIssuance<MaxSerialsPerMint>, MaxPendingIssuances>),
			MaxPendingIssuances,
		>,
	}

	impl<AccountId, MaxSerialsPerMint, MaxPendingIssuances> Default
		for SftCollectionPendingIssuances<AccountId, MaxSerialsPerMint, MaxPendingIssuances>
	where
		AccountId: Debug + PartialEq + Clone,
		MaxSerialsPerMint: Get<u32>,
		MaxPendingIssuances: Get<u32>,
	{
		fn default() -> Self {
			SftCollectionPendingIssuances {
				next_issuance_id: 0,
				pending_issuances: BoundedVec::new(),
			}
		}
	}

	pub fn migrate<T: frame_system::Config + pallet_sft::Config>() -> Weight {
		log::info!(target: "Migration", "Sft: removing PendingIssuances");

		let results = PendingIssuances::<T>::clear(u32::MAX, None);
		let weight: Weight =
			<T as frame_system::Config>::DbWeight::get().reads(results.loops as u64);

		log::info!(target: "Migration", "Sft: removal of PendingIssuance successful");
		weight
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::create_account;
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
				StorageVersion::new(0).put::<Sft>();
				let item_count = 10_u32;
				for i in 0..item_count {
					let pending_issuance = SftCollectionPendingIssuances {
						next_issuance_id: 0,
						pending_issuances: BoundedVec::truncate_from(vec![(
							create_account(i as u64),
							BoundedVec::truncate_from(vec![SftPendingIssuance {
								issuance_id: 0,
								serial_numbers: BoundedVec::truncate_from(vec![(i, 100)]),
							}]),
						)]),
					};
					PendingIssuances::<Runtime>::insert(i, pending_issuance);
				}

				// Sanity check
				for i in 0..item_count {
					assert!(PendingIssuances::<Runtime>::contains_key(i));
				}

				// Do runtime upgrade
				let used_weight = Upgrade::on_runtime_upgrade();
				assert_eq!(Sft::on_chain_storage_version(), 1);

				// Check storage is removed
				for i in 0..item_count {
					assert!(!PendingIssuances::<Runtime>::contains_key(i));
				}
			});
		}
	}
}
