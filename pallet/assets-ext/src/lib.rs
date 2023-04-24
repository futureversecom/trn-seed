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

//! # Pallet Assets Ext
//!
//! An extension pallet providing a hybrid asset system over pallet-balances & pallet-assets
//! The native asset Id (XRP) is used to proxy all requests to pallet-balances, while remaining
//! tokens are managed by pallet-assets
//!
//! It is intended for internal use by other pallets only
//!
//! It provides a minimal API for authorising holds on asset amounts e.g locking bidder funds of an
//! NFT auction This is similar to 'reserve' which is not implemented for pallet-assets within
//! substrate at this time

#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

use frame_support::{
	pallet_prelude::*,
	traits::{
		fungible::{self, Inspect as _, Mutate as _},
		fungibles::{self, Inspect, Mutate, Transfer},
		tokens::{DepositConsequence, WithdrawConsequence},
		ReservableCurrency,
	},
	transactional, PalletId,
};
use frame_system::pallet_prelude::*;
use precompile_utils::constants::ERC20_PRECOMPILE_ADDRESS_PREFIX;
use seed_pallet_common::{
	utils::next_asset_uuid, CreateExt, Hold, OnNewAssetSubscriber, TransferExt,
};
use seed_primitives::{AssetId, Balance, ParachainId};
use sp_runtime::traits::{AccountIdConversion, One, Zero};
use sp_std::prelude::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod imbalances;
mod impls;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;
mod weights;

pub use imbalances::*;
pub use impls::{AssetCurrency, DualStakingCurrency};
pub use weights::WeightInfo;

/// The inner value of a `PalletId`, extracted for convenience as `PalletId` is missing trait
/// derivations e.g. `Ord`
pub type PalletIdValue = [u8; 8];

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		_phantom: sp_std::marker::PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { _phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			NextAssetId::<T>::put(1_u32);
		}
	}

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config:
		pallet_assets::Config<AssetId = AssetId, Balance = Balance>
		+ pallet_balances::Config<Balance = Balance, ReserveIdentifier = PalletIdValue>
	{
		/// The overarching event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The parachain_id being used by this parachain
		type ParachainId: Get<ParachainId>;
		/// The maximum * of holds per asset & account
		#[pallet::constant]
		type MaxHolds: Get<u32>;
		/// The native token asset Id (managed by pallet-balances)
		#[pallet::constant]
		type NativeAssetId: Get<AssetId>;
		/// Handler for when a new asset has been created
		type OnNewAssetSubscription: OnNewAssetSubscriber<AssetId>;
		/// This pallet's Id, used for deriving a sovereign account ID
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// Interface to generate weights
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	/// The holdings of a specific account for a specific asset.
	pub(super) type Holds<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		AssetId,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<(PalletIdValue, Balance), T::MaxHolds>,
		ValueQuery,
	>;

	/// The total units issued in the system.
	#[pallet::storage]
	pub type NextAssetId<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Some assets have been placed on hold by a pallet
		PlaceHold {
			asset_id: AssetId,
			who: T::AccountId,
			amount: Balance,
			pallet_id: PalletIdValue,
		},
		/// Some held assets have been released by a pallet
		ReleaseHold {
			asset_id: AssetId,
			who: T::AccountId,
			amount: Balance,
			pallet_id: PalletIdValue,
		},
		/// Some held assets were spend by a pallet
		SpendHold {
			asset_id: AssetId,
			who: T::AccountId,
			spends: Vec<(T::AccountId, Balance)>,
			pallet_id: PalletIdValue,
		},
		/// Multi-part transfer of assets from who
		SplitTransfer {
			asset_id: AssetId,
			who: T::AccountId,
			transfers: Vec<(T::AccountId, Balance)>,
		},
		/// New asset has been created
		CreateAsset { asset_id: AssetId, creator: T::AccountId, initial_balance: Balance },
		/// Assets were withdrawn from this account by the system e.g. paying tx fees
		InternalWithdraw { asset_id: AssetId, who: T::AccountId, amount: Balance },
		/// Assets were deposited into this account by the system e.g. refunding gas
		InternalDeposit { asset_id: AssetId, who: T::AccountId, amount: Balance },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// No more Ids are available, they've been exhausted
		NoAvailableIds,
		/// Hold balance is less then the required amount
		BalanceLow,
		/// The account to alter does not exist
		NoAccount,
		/// Operation would overflow
		Overflow,
		/// Maximum holds placed on this asset/account pair
		MaxHolds,
		/// Failed to create a new asset
		CreateAssetFailed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates a new asset with unique ID according to the network asset id scheme.
		#[pallet::weight(<T as Config>::WeightInfo::create_asset())]
		#[transactional]
		pub fn create_asset(
			origin: OriginFor<T>,
			name: Vec<u8>,
			symbol: Vec<u8>,
			decimals: u8,
			min_balance: Option<Balance>,
			owner: Option<T::AccountId>,
		) -> DispatchResult {
			let who = frame_system::ensure_signed(origin)?;

			// reserves some native currency from the user - as this should be a costly operation
			let deposit = T::AssetDeposit::get();
			T::Currency::reserve(&who, deposit)?;
			let owner = owner.unwrap_or(who);
			Self::create_with_metadata(&owner, name, symbol, decimals, min_balance)?;
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Returns the AssetId unique across parachains
	pub fn next_asset_uuid() -> Result<AssetId, DispatchError> {
		let asset_id = <NextAssetId<T>>::get();
		match next_asset_uuid(asset_id, T::ParachainId::get().into()) {
			Some(next_asset_id) => Ok(next_asset_id),
			None => Err(Error::<T>::NoAvailableIds.into()),
		}
	}

	/// Returns the held balance of an account over an asset and pallet
	pub fn hold_balance(pallet_id: &PalletId, who: &T::AccountId, asset_id: &AssetId) -> Balance {
		Holds::<T>::get(asset_id, who)
			.iter()
			.find(|(pallet, _)| pallet == &pallet_id.0)
			.map(|(_, balance)| *balance)
			.unwrap_or_default()
	}
}

impl<T: Config> Inspect<T::AccountId> for Pallet<T> {
	type AssetId = AssetId;
	type Balance = Balance;

	fn total_issuance(asset_id: AssetId) -> Balance {
		if asset_id == T::NativeAssetId::get() {
			<pallet_balances::Pallet<T, _>>::total_issuance()
		} else {
			<pallet_assets::Pallet<T>>::total_issuance(asset_id)
		}
	}

	fn minimum_balance(asset_id: AssetId) -> Balance {
		if asset_id == T::NativeAssetId::get() {
			<pallet_balances::Pallet<T, _> as fungible::Inspect<_>>::minimum_balance()
		} else {
			<pallet_assets::Pallet<T>>::minimum_balance(asset_id)
		}
	}

	fn balance(asset_id: AssetId, who: &T::AccountId) -> Balance {
		if asset_id == T::NativeAssetId::get() {
			<pallet_balances::Pallet<T, _>>::balance(who)
		} else {
			<pallet_assets::Pallet<T>>::balance(asset_id, who)
		}
	}

	fn reducible_balance(asset_id: AssetId, who: &T::AccountId, keep_alive: bool) -> Balance {
		if asset_id == T::NativeAssetId::get() {
			<pallet_balances::Pallet<T, _> as fungible::Inspect<_>>::reducible_balance(
				who, keep_alive,
			)
		} else {
			<pallet_assets::Pallet<T> as fungibles::Inspect<_>>::reducible_balance(
				asset_id, who, keep_alive,
			)
		}
	}

	fn can_deposit(
		asset_id: AssetId,
		who: &T::AccountId,
		amount: Balance,
		mint: bool,
	) -> DepositConsequence {
		if asset_id == T::NativeAssetId::get() {
			<pallet_balances::Pallet<T, _>>::can_deposit(who, amount, mint)
		} else {
			<pallet_assets::Pallet<T>>::can_deposit(asset_id, who, amount, mint)
		}
	}

	fn can_withdraw(
		asset_id: AssetId,
		who: &T::AccountId,
		amount: Balance,
	) -> WithdrawConsequence<Balance> {
		if asset_id == T::NativeAssetId::get() {
			<pallet_balances::Pallet<T, _>>::can_withdraw(who, amount)
		} else {
			<pallet_assets::Pallet<T>>::can_withdraw(asset_id, who, amount)
		}
	}
}

impl<T: Config> Mutate<T::AccountId> for Pallet<T> {
	fn mint_into(asset_id: AssetId, who: &T::AccountId, amount: Balance) -> DispatchResult {
		if asset_id == T::NativeAssetId::get() {
			<pallet_balances::Pallet<T, _>>::mint_into(who, amount)
		} else {
			<pallet_assets::Pallet<T>>::mint_into(asset_id, who, amount)
		}
	}

	fn burn_from(
		asset_id: AssetId,
		who: &T::AccountId,
		amount: Balance,
	) -> Result<Balance, DispatchError> {
		if asset_id == T::NativeAssetId::get() {
			<pallet_balances::Pallet<T, _>>::burn_from(who, amount)
		} else {
			<pallet_assets::Pallet<T>>::burn_from(asset_id, who, amount)
		}
	}
}

impl<T: Config> Transfer<T::AccountId> for Pallet<T> {
	fn transfer(
		asset_id: AssetId,
		source: &T::AccountId,
		dest: &T::AccountId,
		amount: Balance,
		keep_alive: bool,
	) -> Result<Balance, DispatchError> {
		if asset_id == T::NativeAssetId::get() {
			<pallet_balances::Pallet<T, _> as fungible::Transfer<_>>::transfer(
				source, dest, amount, keep_alive,
			)
		} else {
			<pallet_assets::Pallet<T> as fungibles::Transfer<T::AccountId>>::transfer(
				asset_id, source, dest, amount, keep_alive,
			)
		}
	}
}

impl<T: Config> TransferExt for Pallet<T> {
	type AccountId = <T as frame_system::Config>::AccountId;

	fn split_transfer(
		who: &Self::AccountId,
		asset_id: AssetId,
		transfers: &[(Self::AccountId, Balance)],
	) -> DispatchResult {
		let total = transfers.iter().map(|x| x.1).sum::<Balance>();
		// This check will fail before making any transfers to restrict partial transfers
		ensure!(Self::reducible_balance(asset_id, who, false) >= total, Error::<T>::BalanceLow);

		for (payee, amount) in transfers.into_iter() {
			Self::transfer(asset_id, who, payee, *amount, false)?;
		}

		Self::deposit_event(Event::SplitTransfer {
			asset_id,
			who: who.clone(),
			transfers: transfers.to_vec(),
		});

		Ok(())
	}
}

impl<T: Config> Hold for Pallet<T> {
	type AccountId = <T as frame_system::Config>::AccountId;

	/// Create a hold on some amount of asset from who
	///
	/// If a hold already exists, it will be increased by `amount`
	fn place_hold(
		pallet_id: PalletId,
		who: &Self::AccountId,
		asset_id: AssetId,
		amount: Balance,
	) -> DispatchResult {
		let mut holds = Holds::<T>::get(asset_id, who);
		match holds.binary_search_by_key(&pallet_id.0, |(p, _)| *p) {
			Ok(index) => {
				let (_, existing_hold) = holds[index];
				let increased_hold =
					existing_hold.checked_add(amount).ok_or(Error::<T>::Overflow)?;
				holds[index].1 = increased_hold;
			},
			Err(index) => {
				holds
					.try_insert(index, (pallet_id.0, amount))
					.map_err(|_| Error::<T>::MaxHolds)?;
			},
		}

		let _ = Self::transfer(
			asset_id,
			who,
			&T::PalletId::get().into_account_truncating(),
			amount,
			false,
		)?;
		Holds::<T>::insert(asset_id, who.clone(), holds);

		Self::deposit_event(Event::PlaceHold {
			asset_id,
			who: who.clone(),
			amount,
			pallet_id: pallet_id.0,
		});

		Ok(())
	}

	/// Release a previously held amount of asset from who
	fn release_hold(
		pallet_id: PalletId,
		who: &Self::AccountId,
		asset_id: AssetId,
		amount: Balance,
	) -> DispatchResult {
		let mut holds = Holds::<T>::get(asset_id, who);
		if let Ok(index) = holds.binary_search_by_key(&pallet_id.0, |(p, _)| *p) {
			let (_, existing_hold) = holds[index];
			let decreased_hold = existing_hold.checked_sub(amount).ok_or(Error::<T>::BalanceLow)?;
			if decreased_hold.is_zero() {
				holds.remove(index);
			} else {
				holds[index].1 = decreased_hold;
			}

			let _ = Self::transfer(
				asset_id,
				&T::PalletId::get().into_account_truncating(),
				who,
				amount,
				false,
			)
			.map(|_| ())?;

			if holds.is_empty() {
				Holds::<T>::take(asset_id, who.clone());
			} else {
				Holds::<T>::insert(asset_id, who.clone(), holds);
			}

			Self::deposit_event(Event::ReleaseHold {
				asset_id,
				who: who.clone(),
				amount,
				pallet_id: pallet_id.0,
			});

			Ok(())
		} else {
			Err(Error::<T>::BalanceLow.into())
		}
	}

	/// Spend held assets
	fn spend_hold(
		pallet_id: PalletId,
		who: &Self::AccountId,
		asset_id: AssetId,
		spends: &[(Self::AccountId, Balance)],
	) -> DispatchResult {
		let total_spend = spends.iter().map(|x| x.1).sum::<Balance>();
		let mut holds = Holds::<T>::get(asset_id, who);
		if let Ok(index) = holds.binary_search_by_key(&pallet_id.0, |(p, _)| *p) {
			let (_, existing_hold) = holds[index];
			let decreased_hold =
				existing_hold.checked_sub(total_spend).ok_or(Error::<T>::BalanceLow)?;
			if decreased_hold.is_zero() {
				holds.remove(index);
			} else {
				holds[index].1 = decreased_hold;
			}

			let _ = Self::split_transfer(
				&T::PalletId::get().into_account_truncating(),
				asset_id,
				spends,
			)?;

			if holds.is_empty() {
				Holds::<T>::take(asset_id, who.clone());
			} else {
				Holds::<T>::insert(asset_id, who.clone(), holds);
			}

			Self::deposit_event(Event::SpendHold {
				asset_id,
				who: who.clone(),
				spends: spends.to_vec(),
				pallet_id: pallet_id.0,
			});

			Ok(())
		} else {
			Err(Error::<T>::BalanceLow.into())
		}
	}
}

impl<T: Config> CreateExt for Pallet<T> {
	type AccountId = <T as frame_system::Config>::AccountId;

	fn create(
		owner: &Self::AccountId,
		min_balance: Option<Balance>,
	) -> Result<AssetId, DispatchError> {
		// Default to 1, errors in pallet_assets if set to 0
		let min_balance = min_balance.unwrap_or(1);

		let next_asset_id = Self::next_asset_uuid()?;

		// create the asset
		<pallet_assets::Pallet<T> as fungibles::Create<_>>::create(
			next_asset_id,
			owner.clone(),
			true,
			min_balance,
		)?;

		// update the next id, will not overflow, asserted prior qed.
		<NextAssetId<T>>::mutate(|i| *i += u32::one());

		// Add asset code to evm pallet
		T::OnNewAssetSubscription::on_asset_create(next_asset_id, ERC20_PRECOMPILE_ADDRESS_PREFIX);

		Self::deposit_event(Event::CreateAsset {
			asset_id: next_asset_id,
			creator: owner.clone(),
			initial_balance: min_balance,
		});

		Ok(next_asset_id)
	}

	fn create_with_metadata(
		owner: &Self::AccountId,
		name: Vec<u8>,
		symbol: Vec<u8>,
		decimals: u8,
		min_balance: Option<Balance>,
	) -> Result<AssetId, DispatchError> {
		let new_asset_id = Self::create(&owner, min_balance)?;

		// set metadata for new asset - as root origin
		<pallet_assets::Pallet<T>>::force_set_metadata(
			frame_system::RawOrigin::Root.into(),
			new_asset_id,
			name,
			symbol,
			decimals,
			false,
		)?;
		Ok(new_asset_id)
	}
}

impl<T: Config> fungibles::InspectMetadata<T::AccountId> for Pallet<T> {
	fn name(asset: &Self::AssetId) -> Vec<u8> {
		<pallet_assets::Pallet<T> as fungibles::InspectMetadata<T::AccountId>>::name(asset)
	}

	fn symbol(asset: &Self::AssetId) -> Vec<u8> {
		<pallet_assets::Pallet<T> as fungibles::InspectMetadata<T::AccountId>>::symbol(asset)
	}

	fn decimals(asset: &Self::AssetId) -> u8 {
		<pallet_assets::Pallet<T> as fungibles::InspectMetadata<T::AccountId>>::decimals(asset)
	}
}
