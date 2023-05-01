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

use crate::{Assets, Runtime, Weight};
use frame_support::{
	dispatch::GetStorageVersion,
	storage_alias,
	traits::{Get, OnRuntimeUpgrade, StorageVersion},
};
use sp_std::{fmt::Debug, vec::Vec};

pub struct Upgrade;
impl OnRuntimeUpgrade for Upgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		v1::pre_upgrade()?;

		Ok(())
	}

	fn on_runtime_upgrade() -> Weight {
		let current = Assets::current_storage_version();
		let onchain = Assets::on_chain_storage_version();
		log::info!(target: "Migration", "Assets: Running migration with current storage version {current:?} / onchain {onchain:?}");

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads_writes(2, 0);

		if onchain == 0 {
			log::info!(target: "Migration", "Assets: Migrating from onchain version 0 to onchain version 1.");
			weight += v1::migrate::<Runtime>();

			log::info!(target: "Migration", "Assets: Migration successfully finished.");
			StorageVersion::new(1).put::<Assets>();
		} else {
			log::info!(target: "Migration", "Assets: No migration was done. If you are seeing this message, it means that you forgot to remove old existing migration code. Don't panic, it's not a big deal just don't forget it next time :)");
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
	use codec::{Decode, Encode};
	use core::fmt::Write;
	use frame_support::{
		pallet_prelude::{NMapKey, ValueQuery},
		Blake2_128Concat, BoundedVec, CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound,
		Twox64Concat,
	};
	use pallet_assets::{Approval, AssetAccount, AssetDetails, AssetMetadata};
	use scale_info::TypeInfo;
	use seed_primitives::{CollectionUuid, MetadataScheme, SerialNumber, TokenCount};
	use sp_core::H160;
	use sp_std::vec::Vec;

	type Balance<T> = <T as pallet_balances::Config>::Balance;
	type AccountId<T> = <T as frame_system::Config>::AccountId;
	type AssetId<T, I> = <T as pallet_assets::Config<I>>::AssetId;
	type DepositBalanceOf<T, I = ()> = <<T as pallet_assets::Config<I>>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;
	type AssetAccountOf<T, I> = AssetAccount<
		<T as pallet_assets::Config<I>>::Balance,
		DepositBalanceOf<T, I>,
		<T as pallet_assets::Config<I>>::Extra,
	>;

	#[derive(
		Clone, Encode, Decode, Eq, PartialEq, Default, RuntimeDebug, MaxEncodedLen, TypeInfo,
	)]
	pub struct AssetMetadataClone<DepositBalance, BoundedString> {
		/// The balance deposited for this metadata.
		///
		/// This pays for the data stored in this struct.
		pub(super) deposit: DepositBalance,
		/// The user friendly name of this asset. Limited in length by `StringLimit`.
		pub(super) name: BoundedString,
		/// The ticker symbol for this asset. Limited in length by `StringLimit`.
		pub(super) symbol: BoundedString,
		/// The number of decimals this asset uses to represent one unit.
		pub(super) decimals: u8,
		/// Whether the asset metadata may be changed by a non Force origin.
		pub(super) is_frozen: bool,
	}

	#[storage_alias]
	/// Details of an asset.
	type Asset<T: pallet_assets::Config<I>, I: 'static = ()> = StorageMap<
		pallet_assets::Pallet<T>,
		Blake2_128Concat,
		AssetId<T>,
		AssetDetails<Balance<T>, AccountId<T>, DepositBalanceOf<T, I>>,
	>;

	#[storage_alias]
	/// The holdings of a specific account for a specific asset.
	type Account<T: pallet_assets::Config<I>, I: 'static = ()> = StorageDoubleMap<
		pallet_assets::Pallet<T>,
		Blake2_128Concat,
		AssetId<T>,
		Blake2_128Concat,
		AccountId<T>,
		AssetAccountOf<T, I>,
	>;

	#[storage_alias]
	/// Approved balance transfers. First balance is the amount approved for transfer. Second
	/// is the amount of `T::Currency` reserved for storing this.
	/// First key is the asset ID, second key is the owner and third key is the delegate.
	type Approvals<T: pallet_assets::Config<I>, I: 'static = ()> = StorageNMap<
		pallet_assets::Pallet<T>,
		(
			NMapKey<Blake2_128Concat, AssetId<T>>,
			NMapKey<Blake2_128Concat, AccountId<T>>, // owner
			NMapKey<Blake2_128Concat, AccountId<T>>, // delegate
		),
		Approval<Balance<T>, DepositBalanceOf<T, I>>,
	>;

	#[storage_alias]
	/// Metadata of an asset.
	type Metadata<T: pallet_assets::Config<I>, I: 'static = ()> = StorageMap<
		pallet_assets::Pallet<T>,
		Blake2_128Concat,
		AssetId<T>,
		AssetMetadataClone<
			DepositBalanceOf<T, I>,
			BoundedVec<u8, <T as pallet_assets::Config<I>>::StringLimit>,
		>,
		ValueQuery,
	>;

	#[cfg(feature = "try-runtime")]
	pub fn pre_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Assets: Upgrade to v1 Pre Upgrade.");
		let onchain = Assets::on_chain_storage_version();
		// Return OK(()) if upgrade has already been done
		if onchain == 1 {
			return Ok(())
		}
		assert_eq!(onchain, 0);

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_upgrade() -> Result<(), &'static str> {
		log::info!(target: "Migration", "Assets: Upgrade to v1 Post Upgrade.");

		let current = Assets::current_storage_version();
		let onchain = Assets::on_chain_storage_version();
		assert_eq!(current, 1);
		assert_eq!(onchain, 1);

		Ok(())
	}

	pub fn migrate<T: pallet_assets::Config<I>, I: 'static = ()>() -> Weight {
		log::info!(target: "Migration", "Assets: Cleaning up dex related storages...");

		// Get all LP asset keys
		const LPTokenName: Vec<u8> = Vec::from(*b"Uniswap V2");
		let mut lp_asset_keys: Vec<T::AssetId> = Vec::new();
		Metadata::<T, I>::iter().for_each(|(k, v)| {
			if v.name == LPTokenName {
				lp_asset_keys.push(k);
			}
		});

		let key_count = lp_asset_keys.len();

		// Remove LP assets related storages
		for key in lp_asset_keys {
			pallet_assets::Asset::<T, I>::remove(key);
			pallet_assets::Metadata::<T, I>::remove(key);
			_ = pallet_assets::Account::<T, I>::clear_prefix(key, u32::MAX, None);
			_ = pallet_assets::Approvals::<T, I>::clear_prefix(key, u32::MAX, None);
		}

		log::info!(target: "Migration", "Assets: ...Successfully cleaned up dex related storages");

		<Runtime as frame_system::Config>::DbWeight::get().writes((key_count * 4) as u64)
	}

}
