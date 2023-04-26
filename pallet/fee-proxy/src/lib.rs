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

//! # Pallet Fee Proxy
//!
//! A utility pallet providing the possibility to call any runtime extrinsic with a specified gas
//! token and pay for fees in that token.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{
	dispatch::Dispatchable,
	pallet_prelude::*,
	traits::IsSubType,
	weights::{GetDispatchInfo, PostDispatchInfo},
};
use frame_system::pallet_prelude::*;
use seed_primitives::{AssetId, Balance};
use sp_std::prelude::*;

mod impls;
#[cfg(test)]
mod mock;
mod runner;
#[cfg(test)]
mod tests;
pub use runner::{get_fee_preferences_data, FeePreferencesData, FeePreferencesRunner};

pub(crate) const LOG_TARGET: &str = "fee-preferences";

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use pallet_transaction_payment::OnChargeTransaction;
	use precompile_utils::{Address, ErcIdConversion};

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_transaction_payment::Config {
		/// Origin type to allow new whitelist entries
		type ApproveOrigin: EnsureOrigin<Self::Origin>;
		/// The overarching call type.
		type Call: Parameter
			+ Dispatchable<Origin = Self::Origin, PostInfo = PostDispatchInfo>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>
			+ IsSubType<Call<Self>>;
		/// The system event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The caller origin, overarching type of all pallets origins.
		type PalletsOrigin: Parameter
			+ Into<<Self as frame_system::Config>::Origin>
			+ IsType<<<Self as frame_system::Config>::Origin as frame_support::traits::OriginTrait>::PalletsOrigin>;
		/// The native token asset Id (managed by pallet-balances)
		#[pallet::constant]
		type FeeAssetId: Get<AssetId>;
		#[pallet::constant]
		type MaxWhiteListedAssets: Get<u32>;
		/// The OnChargeTransaction to route to after intercept
		type OnChargeTransaction: OnChargeTransaction<Self>;
		/// Convert EVM addresses into Runtime Id identifiers and vice versa
		type ErcIdConversion: ErcIdConversion<AssetId, EvmId = Address>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		AssetWhitelistSet {
			asset_id: AssetId,
			is_allowed: bool,
		},
		/// A call was made with specified payment asset
		CallWithFeePreferences {
			who: T::AccountId,
			payment_asset: AssetId,
			max_payment: Balance,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// All fee tokens must be in the map of known fee tokens
		FeeTokenNotWhitelisted,
		/// The inner call is a fee preference call
		NestedFeePreferenceCall,
		/// The selected fee token is equal to the native gas token
		FeeTokenIsGasToken,
	}

	// #[pallet::storage]
	// pub type AssetWhitelist<T: Config> = StorageMap<_, Twox64Concat, AssetId, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn asset_white_list)]
	pub type AssetWhitelist<T: Config> = StorageMap<_, Twox64Concat, AssetId, bool, ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Call an internal call with specified gas token
		/// payment_asset: The token to be used for paying gas fees. This is exchanged in
		///                OnChargeTransaction::withdraw_fee()
		/// max_payment: The limit of how many tokens will be used to perform the exchange
		/// call: The inner call to be performed after the exchange
		#[pallet::weight({
			let dispatch_info = call.get_dispatch_info();
			(dispatch_info.weight.saturating_add(10_000), dispatch_info.class)
		})]
		pub fn call_with_fee_preferences(
			origin: OriginFor<T>,
			payment_asset: AssetId,
			max_payment: Balance,
			call: Box<<T as Config>::Call>,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			ensure!(payment_asset != T::FeeAssetId::get(), Error::<T>::FeeTokenIsGasToken);
			// Token must be one of the known whitelisted ones in order to be used for fee proxy
			ensure!(AssetWhitelist::<T>::get(payment_asset), Error::<T>::FeeTokenNotWhitelisted);
			ensure!(
				!matches!(call.is_sub_type(), Some(Call::call_with_fee_preferences { .. })),
				Error::<T>::NestedFeePreferenceCall
			);
			let _ = call.dispatch(origin).map_err(|err| err.error)?;

			// Deposit runtime event
			Self::deposit_event(Event::CallWithFeePreferences { who, payment_asset, max_payment });

			Ok(())
		}

		#[pallet::weight(20_000_000)]
		pub fn set_fee_token(
			origin: OriginFor<T>,
			new_asset_setting: AssetId,
			is_allowed: bool,
		) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;

			AssetWhitelist::<T>::insert(new_asset_setting, is_allowed);

			Self::deposit_event(Event::<T>::AssetWhitelistSet {
				asset_id: new_asset_setting,
				is_allowed,
			});

			Ok(())
		}
	}
}
