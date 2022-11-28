//! # Pallet Fee Proxy
//!
//! A utility pallet providing the possibility to call any runtime extrinsic with a specified gas token
//! and pay for fees in that token.

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
	use seed_primitives::AccountId;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config<AccountId = AccountId> + pallet_transaction_payment::Config
	{
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
		/// The OnChargeTransaction to route to after intercept
		type OnChargeTransaction: OnChargeTransaction<Self>;
		/// Convert EVM addresses into Runtime Id identifiers and vice versa
		type ErcIdConversion: ErcIdConversion<AssetId, EvmId = Address>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
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
