#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{
	dispatch::Dispatchable,
	pallet_prelude::*,
	traits::IsSubType,
	weights::{DispatchInfo, GetDispatchInfo, PostDispatchInfo},
};
use frame_system::pallet_prelude::*;
use seed_primitives::{AccountId, AssetId, Balance};
use sp_std::prelude::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::weights::extract_actual_weight;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		/// The system event type
		type Event: From<Event> + IsType<<Self as frame_system::Config>::Event>;
		/// The overarching call type.
		type Call: Parameter
			+ Dispatchable<Origin = Self::Origin, PostInfo = PostDispatchInfo>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>
			+ IsSubType<Call<Self>>;
		/// The caller origin, overarching type of all pallets origins.
		type PalletsOrigin: Parameter
			+ Into<<Self as frame_system::Config>::Origin>
			+ IsType<<<Self as frame_system::Config>::Origin as frame_support::traits::OriginTrait>::PalletsOrigin>;
		/// The fee asset that will be exchanged
		type NativeAssetId: Get<AssetId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event {
		/// A call was made with specified payment asset
		CallWithFeePreferences {
			payment_asset: AssetId,
			predicted_weight: Weight,
			used_weight: Weight,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The call failed to dispatch
		DispatchFailed,
		/// The inner call is a fee preference call
		NestedFeePreferenceCall,
		/// The selected fee token is equal to the native gas token
		FeeTokenIsGasToken,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// call an internal call with specified gas token
		/// TODO Better weight estimate
		#[pallet::weight({
			let dispatch_info = call.get_dispatch_info();
			(dispatch_info.weight.saturating_add(10_000), dispatch_info.class)
		})]
		pub fn call_with_fee_preferences(
			origin: OriginFor<T>,
			payment_asset: AssetId,
			max_payment: Balance,
			evm_estimate: Balance,
			call: Box<<T as Config>::Call>,
		) -> DispatchResult {
			let _ = ensure_signed(origin.clone())?;

			ensure!(payment_asset != T::NativeAssetId::get(), Error::<T>::FeeTokenIsGasToken);
			ensure!(
				!matches!(call.is_sub_type(), Some(Call::call_with_fee_preferences { .. })),
				Error::<T>::NestedFeePreferenceCall
			);

			let dispatch_info: DispatchInfo = call.get_dispatch_info();
			let predicted_weight = dispatch_info.weight;

			// TODO Potential fix:
			// TODO Check users XRP balance before the call
			let post_dispatch_info = call.dispatch(origin).map_err(|err| err.error)?;

			// TODO Check users XRP balance after the call and exchange back to fee token

			let used_weight = post_dispatch_info.calc_actual_weight(&dispatch_info);

			// Deposit runtime event
			Self::deposit_event(Event::CallWithFeePreferences {
				payment_asset,
				predicted_weight,
				used_weight,
			});

			Ok(())
		}
	}
}
