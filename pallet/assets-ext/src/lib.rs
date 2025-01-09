// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
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
		fungibles::{self, roles::Inspect as InspectRole, Dust, Inspect, Mutate, Unbalanced},
		tokens::{
			DepositConsequence, Fortitude, Precision, Preservation, Provenance, WithdrawConsequence,
		},
		Currency, ReservableCurrency,
	},
	transactional, PalletId,
};
use frame_system::pallet_prelude::*;
use pallet_assets::WeightInfo as AssetsWeightInfo;
use precompile_utils::constants::ERC20_PRECOMPILE_ADDRESS_PREFIX;
use seed_pallet_common::{
	utils::next_asset_uuid, CreateExt, Hold, InspectExt, OnNewAssetSubscriber, TransferExt,
};
use seed_primitives::{AssetId, Balance, ParachainId};
use sp_runtime::traits::{AccountIdConversion, One, StaticLookup, Zero};
use sp_std::prelude::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod imbalances;
mod impls;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod weights;

pub use imbalances::*;
pub use impls::AssetCurrency;
pub use weights::WeightInfo;

/// The inner value of a `PalletId`, extracted for convenience as `PalletId` is missing trait
/// derivations e.g. `Ord`
pub type PalletIdValue = [u8; 8];

// Type used for AssetDeposit
pub type DepositBalanceOf<T, I = ()> = <<T as pallet_assets::Config<I>>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

/// The maximum number of decimals allowed for an asset
pub const MAX_DECIMALS: u8 = 18;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		_phantom: sp_std::marker::PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { _phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
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
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
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
		BoundedVec<(PalletIdValue, Balance), <T as Config>::MaxHolds>,
		ValueQuery,
	>;

	/// The total units issued in the system.
	#[pallet::storage]
	pub(crate) type NextAssetId<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// The minimum deposit for creating an asset
	#[pallet::storage]
	pub type AssetDeposit<T: Config> = StorageValue<_, DepositBalanceOf<T>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
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
		/// The asset deposit was set
		AssetDepositSet { asset_deposit: DepositBalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Decimals cannot be higher than 18
		DecimalsTooHigh,
		/// No more Ids are available, they've been exhausted
		NoAvailableIds,
		/// The signer does not have permission to perform this action
		NoPermission,
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
		/// Sudo call to set the asset deposit for creating assets
		/// Note, this does not change the deposit when calling create within the assets pallet
		/// However that call is filtered
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::set_asset_deposit())]
		pub fn set_asset_deposit(
			origin: OriginFor<T>,
			asset_deposit: DepositBalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			<AssetDeposit<T>>::put(asset_deposit);
			Self::deposit_event(Event::AssetDepositSet { asset_deposit });
			Ok(())
		}

		/// Creates a new asset with unique ID according to the network asset id scheme.
		/// Decimals cannot be higher than 18 due to a restriction in the conversion function
		/// scale_wei_to_correct_decimals
		#[pallet::call_index(1)]
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
			let who = ensure_signed(origin)?;
			ensure!(decimals <= MAX_DECIMALS, Error::<T>::DecimalsTooHigh);
			// reserves some native currency from the user - as this should be a costly operation
			let deposit = <AssetDeposit<T>>::get();
			T::Currency::reserve(&who, deposit)?;
			let owner = owner.unwrap_or(who);
			Self::create_with_metadata(&owner, name, symbol, decimals, min_balance)?;
			Ok(())
		}

		/// Mints an asset to an account if the caller is the asset owner
		/// Attempting to mint ROOT token will throw an error
		#[pallet::call_index(2)]
		#[pallet::weight(<T as pallet_assets::Config>::WeightInfo::mint())]
		pub fn mint(
			origin: OriginFor<T>,
			asset_id: AssetId,
			beneficiary: AccountIdLookupOf<T>,
			#[pallet::compact] amount: Balance,
		) -> DispatchResult {
			if asset_id == T::NativeAssetId::get() {
				// We do not allow minting of the ROOT token
				Err(Error::<T>::NoPermission)?;
			} else {
				<pallet_assets::Pallet<T>>::mint(origin, asset_id.into(), beneficiary, amount)?;
			}
			Ok(())
		}

		/// Transfers either ROOT or an asset
		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet_assets::Config>::WeightInfo::transfer())]
		pub fn transfer(
			origin: OriginFor<T>,
			asset_id: AssetId,
			destination: T::AccountId,
			#[pallet::compact] amount: Balance,
			keep_alive: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let preservation = match keep_alive {
				true => Preservation::Preserve,
				false => Preservation::Expendable,
			};
			<Self as Mutate<T::AccountId>>::transfer(
				asset_id,
				&who,
				&destination,
				amount,
				preservation,
			)?;
			Ok(())
		}

		/// Burns an asset from an account. Caller must be the asset owner
		/// Attempting to burn ROOT token will throw an error
		#[pallet::call_index(4)]
		#[pallet::weight(<T as pallet_assets::Config>::WeightInfo::burn())]
		pub fn burn_from(
			origin: OriginFor<T>,
			asset_id: AssetId,
			who: AccountIdLookupOf<T>,
			#[pallet::compact] amount: Balance,
		) -> DispatchResult {
			if asset_id == T::NativeAssetId::get() {
				// We do not allow burning of the ROOT token
				Err(Error::<T>::NoPermission)?;
			} else {
				<pallet_assets::Pallet<T>>::burn(origin, asset_id.into(), who, amount)?;
			}
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Returns the AssetId unique across parachains
	pub fn next_asset_uuid() -> Result<AssetId, DispatchError> {
		let asset_id = <NextAssetId<T>>::get();
		match next_asset_uuid(asset_id, T::ParachainId::get()) {
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

	/// Called by the ERC20 precompile to verify whether an assetId exists or not
	/// Checked against whether the asset contains an owner
	pub fn asset_exists(asset_id: AssetId) -> bool {
		if asset_id == T::NativeAssetId::get() {
			true
		} else {
			<pallet_assets::Pallet<T>>::owner(asset_id).is_some()
		}
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

	fn total_balance(asset_id: Self::AssetId, who: &T::AccountId) -> Self::Balance {
		if asset_id == T::NativeAssetId::get() {
			<pallet_balances::Pallet<T, _> as fungible::Inspect<_>>::total_balance(who)
		} else {
			<pallet_assets::Pallet<T>>::total_balance(asset_id, who)
		}
	}

	fn balance(asset_id: AssetId, who: &T::AccountId) -> Balance {
		if asset_id == T::NativeAssetId::get() {
			<pallet_balances::Pallet<T, _>>::balance(who)
		} else {
			<pallet_assets::Pallet<T>>::balance(asset_id, who)
		}
	}

	fn reducible_balance(
		asset_id: AssetId,
		who: &T::AccountId,
		preservation: Preservation,
		force: Fortitude,
	) -> Balance {
		if asset_id == T::NativeAssetId::get() {
			<pallet_balances::Pallet<T, _> as fungible::Inspect<_>>::reducible_balance(
				who,
				preservation,
				force,
			)
		} else {
			<pallet_assets::Pallet<T> as fungibles::Inspect<_>>::reducible_balance(
				asset_id,
				who,
				preservation,
				force,
			)
		}
	}

	fn can_deposit(
		asset_id: AssetId,
		who: &T::AccountId,
		amount: Balance,
		provenance: Provenance,
	) -> DepositConsequence {
		if asset_id == T::NativeAssetId::get() {
			<pallet_balances::Pallet<T, _>>::can_deposit(who, amount, provenance)
		} else {
			<pallet_assets::Pallet<T>>::can_deposit(asset_id, who, amount, provenance)
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

	fn asset_exists(asset: Self::AssetId) -> bool {
		if asset == T::NativeAssetId::get() {
			// native token always exists
			true
		} else {
			<pallet_assets::Pallet<T>>::asset_exists(asset)
		}
	}
}

impl<T: Config> Unbalanced<T::AccountId> for Pallet<T> {
	fn handle_dust(dust: Dust<T::AccountId, Self>) {
		<pallet_assets::Pallet<T>>::handle_dust(Dust(dust.0, dust.1))
	}

	fn write_balance(
		asset: Self::AssetId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> Result<Option<Self::Balance>, DispatchError> {
		<pallet_assets::Pallet<T>>::write_balance(asset, who, amount)
	}

	fn set_total_issuance(asset: Self::AssetId, amount: Self::Balance) {
		<pallet_assets::Pallet<T>>::set_total_issuance(asset, amount)
	}
}

impl<T: Config> Mutate<T::AccountId> for Pallet<T> {
	fn mint_into(
		asset_id: AssetId,
		who: &T::AccountId,
		amount: Balance,
	) -> Result<Self::Balance, DispatchError> {
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
		precision: Precision,
		force: Fortitude,
	) -> Result<Balance, DispatchError> {
		if asset_id == T::NativeAssetId::get() {
			<pallet_balances::Pallet<T, _>>::burn_from(who, amount, precision, force)
		} else {
			<pallet_assets::Pallet<T>>::burn_from(asset_id, who, amount, precision, force)
		}
	}

	fn transfer(
		asset_id: AssetId,
		source: &T::AccountId,
		dest: &T::AccountId,
		amount: Balance,
		preservation: Preservation,
	) -> Result<Balance, DispatchError> {
		if asset_id == T::NativeAssetId::get() {
			<pallet_balances::Pallet<T, _> as fungible::Mutate<_>>::transfer(
				source,
				dest,
				amount,
				preservation,
			)
		} else {
			// Transfers with 0 amount will fail if the destination account does not exist
			// This is because the transfer value is less than the existential deposit
			<pallet_assets::Pallet<T> as fungibles::Mutate<T::AccountId>>::transfer(
				asset_id,
				source,
				dest,
				amount,
				preservation,
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
		ensure!(
			Self::reducible_balance(asset_id, who, Preservation::Expendable, Fortitude::Polite)
				>= total,
			Error::<T>::BalanceLow
		);

		// Skip zero transfers, these will error within the transfer function
		for (payee, amount) in transfers.iter().filter(|(_, b)| !b.is_zero()) {
			<Self as Mutate<T::AccountId>>::transfer(
				asset_id,
				who,
				payee,
				*amount,
				Preservation::Expendable,
			)?;
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

		let _ = <Self as Mutate<T::AccountId>>::transfer(
			asset_id,
			who,
			&T::PalletId::get().into_account_truncating(),
			amount,
			Preservation::Expendable,
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

			<Self as Mutate<T::AccountId>>::transfer(
				asset_id,
				&T::PalletId::get().into_account_truncating(),
				who,
				amount,
				Preservation::Expendable,
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

			Self::split_transfer(&T::PalletId::get().into_account_truncating(), asset_id, spends)?;

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

impl<T: Config> InspectExt for Pallet<T> {
	fn exists(asset_id: AssetId) -> bool {
		Self::asset_exists(asset_id)
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
		let new_asset_id = Self::create(owner, min_balance)?;

		// set metadata for new asset - as root origin
		<pallet_assets::Pallet<T>>::force_set_metadata(
			frame_system::RawOrigin::Root.into(),
			new_asset_id.into(),
			name,
			symbol,
			decimals,
			false,
		)?;
		Ok(new_asset_id)
	}
}

impl<T: Config> fungibles::metadata::Inspect<T::AccountId> for Pallet<T> {
	fn name(asset: Self::AssetId) -> Vec<u8> {
		<pallet_assets::Pallet<T> as fungibles::metadata::Inspect<T::AccountId>>::name(asset)
	}

	fn symbol(asset: Self::AssetId) -> Vec<u8> {
		<pallet_assets::Pallet<T> as fungibles::metadata::Inspect<T::AccountId>>::symbol(asset)
	}

	fn decimals(asset: Self::AssetId) -> u8 {
		<pallet_assets::Pallet<T> as fungibles::metadata::Inspect<T::AccountId>>::decimals(asset)
	}
}
