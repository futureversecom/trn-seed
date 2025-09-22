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

//! # Pallet Fee Proxy
//!
//! A utility pallet providing the possibility to call any runtime extrinsic with a specified gas
//! token and pay for fees in that token.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	pallet_prelude::*,
	traits::{IsSubType, IsType},
};
use frame_system::pallet_prelude::*;
use seed_pallet_common::{FeeConfig, MaintenanceCheckEVM};
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
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_transaction_payment::Config {
		/// The overarching call type.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin, PostInfo = PostDispatchInfo>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>
			+ IsSubType<Call<Self>>
			+ IsType<<Self as frame_system::Config>::RuntimeCall>;
		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The caller origin, overarching type of all pallets origins.
		type PalletsOrigin: Parameter
			+ Into<<Self as frame_system::Config>::RuntimeOrigin>
			+ IsType<<<Self as frame_system::Config>::RuntimeOrigin as frame_support::traits::OriginTrait>::PalletsOrigin>;
		/// The native token asset Id (managed by pallet-balances)
		#[pallet::constant]
		type FeeAssetId: Get<AssetId>;
		/// The OnChargeTransaction to route to after intercept
		type OnChargeTransaction: OnChargeTransaction<Self>;
		/// Convert EVM addresses into Runtime Id identifiers and vice versa
		type ErcIdConversion: ErcIdConversion<AssetId, EvmId = Address>;
		/// Base fee data provider for EVM transactions
		type EVMBaseFeeProvider: seed_pallet_common::FeeConfig;
		// Maintenance mode checker
		type MaintenanceChecker: MaintenanceCheckEVM<Self>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A call was made with specified payment asset
		CallWithFeePreferences { who: T::AccountId, payment_asset: AssetId, max_payment: Balance },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The inner call is a fee preference call
		NestedFeePreferenceCall,
		/// The selected fee token is equal to the native gas token
		FeeTokenIsGasToken,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Call an internal call with specified gas token
		/// payment_asset: The token to be used for paying gas fees. This is exchanged in
		///                OnChargeTransaction::withdraw_fee()
		/// max_payment: A CEILING (maximum) of how many tokens may be swapped to cover
		///              the final fee in the native gas token. The pallet will perform
		///              an exact-target swap for the required fee amount (including any
		///              additional EVM max fee component) provided it does not exceed
		///              this ceiling. Supplying an exact estimated fee here risks
		///              underpayment due to rounding, minimum deposit top-ups, or added
		///              max-fee scaling; providing a generous upper bound is safe—the
		///              pallet only spends what is required.
		/// call: The inner call to be performed after the exchange
		#[pallet::call_index(0)]
		#[pallet::weight({
			let dispatch_info = call.get_dispatch_info();
			(dispatch_info.weight.saturating_add(Weight::from_all(10_000u64)), dispatch_info.class)
		})]
		pub fn call_with_fee_preferences(
			origin: OriginFor<T>,
			payment_asset: AssetId,
			max_payment: Balance,
			call: Box<<T as Config>::RuntimeCall>,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			ensure!(payment_asset != T::FeeAssetId::get(), Error::<T>::FeeTokenIsGasToken);
			ensure!(
				!matches!(call.is_sub_type(), Some(Call::call_with_fee_preferences { .. })),
				Error::<T>::NestedFeePreferenceCall
			);
			let _ = call.dispatch(origin).map_err(|err| err.error)?;

			// Deposit runtime event
			Self::deposit_event(Event::CallWithFeePreferences { who, payment_asset, max_payment });

			Ok(())
		}
	}
}
