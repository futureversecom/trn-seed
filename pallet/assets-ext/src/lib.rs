//! # Pallet Assets Ext
//!
//! An extension pallet providing a hybrid asset system over pallet-balances & pallet-assets
//! The mycelium asset Id is used to proxy all requests to pallet-balances, while remaining tokens
//! are managed by pallet-assets
//!
//! It is intended for internal use by other pallets only
//!
//! It provides a minimal API for authorising holds on asset amounts e.g locking bidder funds of an
//! NFT auction This is similar to 'reserve' which is not implemented for pallet-assets within
//! substrate at this time

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	pallet_prelude::*,
	traits::{
		fungible::{self, Inspect as _, Mutate as _, Unbalanced as _},
		fungibles::{self, Inspect, Mutate, Transfer, Unbalanced},
		tokens::{DepositConsequence, WithdrawConsequence},
		NamedReservableCurrency,
	},
	PalletId,
};
use seed_pallet_common::{utils::next_asset_uuid, CreateExt, Hold, TransferExt};
use seed_primitives::{AssetId, Balance, ParachainId};
use sp_runtime::traits::{AccountIdConversion, One, Zero};
use sp_std::prelude::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;

pub use pallet::*;

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
		/// The mycelium asset Id (managed by pallet-balances)
		#[pallet::constant]
		type MyclAssetId: Get<AssetId>;
		/// This pallet's Id, used for deriving a sovereign account ID
		#[pallet::constant]
		type PalletId: Get<PalletId>;
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

	/// Create a hold on some amount of asset from who
	///
	/// If a hold already exists, it will be increased by `amount`
	pub(crate) fn place_hold(
		pallet_id: PalletId,
		asset_id: AssetId,
		who: &T::AccountId,
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
		let _ = <pallet_assets::Pallet<T> as fungibles::Transfer<T::AccountId>>::transfer(
			asset_id,
			who,
			&T::PalletId::get().into_account_truncating(),
			amount,
			false,
		)?;
		Holds::<T>::insert(asset_id, who.clone(), holds);
		Ok(())
	}

	/// Release a previously held amount of asset from who
	pub(crate) fn release_hold(
		pallet_id: PalletId,
		asset_id: AssetId,
		who: &T::AccountId,
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

			let _ = <pallet_assets::Pallet<T> as fungibles::Transfer<T::AccountId>>::transfer(
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
			Ok(())
		} else {
			Err(Error::<T>::BalanceLow.into())
		}
	}

	/// Spend held assets
	pub(crate) fn spend_hold(
		pallet_id: PalletId,
		asset_id: AssetId,
		who: &T::AccountId,
		spends: &[(T::AccountId, Balance)],
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

			Self::split_transfer(asset_id, &T::PalletId::get().into_account_truncating(), spends)?;

			if holds.is_empty() {
				Holds::<T>::take(asset_id, who.clone());
			} else {
				Holds::<T>::insert(asset_id, who.clone(), holds);
			}
			Ok(())
		} else {
			Err(Error::<T>::BalanceLow.into())
		}
	}

	/// Efficiently make multiple transfers of asset Id from source
	pub fn split_transfer(
		asset_id: AssetId,
		source: &T::AccountId,
		transfers: &[(T::AccountId, Balance)],
	) -> DispatchResult {
		let total_transfer = transfers.iter().map(|x| x.1).sum::<Balance>();
		let _ = <pallet_assets::Pallet<T>>::decrease_balance(asset_id, source, total_transfer)?;

		for (payee, amount) in transfers.into_iter() {
			<pallet_assets::Pallet<T>>::increase_balance(asset_id, payee, *amount)?;
		}

		Ok(())
	}
}

impl<T: Config> Inspect<T::AccountId> for Pallet<T> {
	type AssetId = AssetId;
	type Balance = Balance;

	fn total_issuance(asset_id: AssetId) -> Balance {
		if asset_id == T::MyclAssetId::get() {
			<pallet_balances::Pallet<T, _>>::total_issuance()
		} else {
			<pallet_assets::Pallet<T>>::total_issuance(asset_id)
		}
	}

	fn minimum_balance(asset_id: AssetId) -> Balance {
		if asset_id == T::MyclAssetId::get() {
			<pallet_balances::Pallet<T, _> as fungible::Inspect<_>>::minimum_balance()
		} else {
			<pallet_assets::Pallet<T>>::minimum_balance(asset_id)
		}
	}

	fn balance(asset_id: AssetId, who: &T::AccountId) -> Balance {
		if asset_id == T::MyclAssetId::get() {
			<pallet_balances::Pallet<T, _>>::balance(who)
		} else {
			<pallet_assets::Pallet<T>>::balance(asset_id, who)
		}
	}

	fn reducible_balance(asset_id: AssetId, who: &T::AccountId, keep_alive: bool) -> Balance {
		if asset_id == T::MyclAssetId::get() {
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
		if asset_id == T::MyclAssetId::get() {
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
		if asset_id == T::MyclAssetId::get() {
			<pallet_balances::Pallet<T, _>>::can_withdraw(who, amount)
		} else {
			<pallet_assets::Pallet<T>>::can_withdraw(asset_id, who, amount)
		}
	}
}

impl<T: Config> Mutate<T::AccountId> for Pallet<T> {
	fn mint_into(asset_id: AssetId, who: &T::AccountId, amount: Balance) -> DispatchResult {
		if asset_id == T::MyclAssetId::get() {
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
		if asset_id == T::MyclAssetId::get() {
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
		if asset_id == T::MyclAssetId::get() {
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
		if asset_id == T::MyclAssetId::get() {
			let total = transfers.iter().map(|x| x.1).sum::<Balance>();
			<pallet_balances::Pallet<T, _>>::decrease_balance(who, total)?;
			for (payee, amount) in transfers.into_iter() {
				<pallet_balances::Pallet<T, _>>::increase_balance(payee, *amount)?;
			}
		} else {
			Self::split_transfer(asset_id, who, transfers)?;
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

	fn place_hold(
		pallet_id: PalletId,
		who: &Self::AccountId,
		asset_id: AssetId,
		amount: Balance,
	) -> DispatchResult {
		if asset_id == T::MyclAssetId::get() {
			<pallet_balances::Pallet<T, _>>::reserve_named(&pallet_id.0, who, amount)?;
		} else {
			Self::place_hold(pallet_id, asset_id, who, amount)?;
		}

		Self::deposit_event(Event::PlaceHold {
			asset_id,
			who: who.clone(),
			amount,
			pallet_id: pallet_id.0,
		});

		Ok(())
	}
	fn release_hold(
		pallet_id: PalletId,
		who: &Self::AccountId,
		asset_id: AssetId,
		amount: Balance,
	) -> DispatchResult {
		if asset_id == T::MyclAssetId::get() {
			ensure!(
				<pallet_balances::Pallet<T, _>>::unreserve_named(&pallet_id.0, who, amount)
					.is_zero(),
				Error::<T>::BalanceLow
			);
		} else {
			Self::release_hold(pallet_id, asset_id, who, amount)?;
		}

		Self::deposit_event(Event::ReleaseHold {
			asset_id,
			who: who.clone(),
			amount,
			pallet_id: pallet_id.0,
		});

		Ok(())
	}
	fn spend_hold(
		pallet_id: PalletId,
		who: &Self::AccountId,
		asset_id: AssetId,
		spends: &[(Self::AccountId, Balance)],
	) -> DispatchResult {
		if asset_id == T::MyclAssetId::get() {
			let total = spends.iter().map(|x| x.1).sum::<Balance>();
			ensure!(
				<pallet_balances::Pallet<T, _>>::unreserve_named(&pallet_id.0, who, total)
					.is_zero(),
				Error::<T>::BalanceLow
			);
			<pallet_balances::Pallet<T, _>>::decrease_balance(who, total)?;
			for (payee, amount) in spends.into_iter() {
				<pallet_balances::Pallet<T, _>>::increase_balance(payee, *amount)?;
			}
		} else {
			Self::spend_hold(pallet_id, asset_id, who, spends)?;
		}

		Self::deposit_event(Event::SpendHold {
			asset_id,
			who: who.clone(),
			spends: spends.to_vec(),
			pallet_id: pallet_id.0,
		});

		Ok(())
	}
}

impl<T: Config> CreateExt for Pallet<T> {
	type AccountId = <T as frame_system::Config>::AccountId;

	fn create(owner: Self::AccountId) -> Result<u32, DispatchError> {
		let min_balance = 1;

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

		Self::deposit_event(Event::CreateAsset {
			asset_id: next_asset_id,
			creator: owner,
			initial_balance: min_balance,
		});

		Ok(next_asset_id)
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
