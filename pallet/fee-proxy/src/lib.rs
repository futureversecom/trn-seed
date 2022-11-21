#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use codec::Codec;
use frame_support::{
	dispatch::Dispatchable,
	pallet_prelude::*,
	traits::OriginTrait,
	weights::{DispatchInfo, GetDispatchInfo, PostDispatchInfo},
};
use frame_system::pallet_prelude::*;
use seed_primitives::{AccountId, AssetId};
use sp_std::prelude::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::dispatch::UnfilteredDispatchable;
	use frame_support::traits::IsSubType;
	use seed_primitives::Balance;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		/// The system event type
		type Event: From<Event> + IsType<<Self as frame_system::Config>::Event>;
		/// The aggregated origin which the dispatch will take.
		// type Origin: OriginTrait<PalletsOrigin = Self::PalletsOrigin>
		// 	+ From<Self::PalletsOrigin>
		// 	+ IsType<<Self as frame_system::Config>::Origin>;
		/// The caller origin, overarching type of all pallets origins.
		// type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>
		// 	+ Codec
		// 	+ Clone
		// 	+ Eq
		// 	+ TypeInfo;
		/// The runtime call type.
		// type Call: From<Call<Self>>;
		/// The overarching call type.
		type Call: Parameter
			+ Dispatchable<Origin = Self::Origin, PostInfo = PostDispatchInfo>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>
			+ IsSubType<Call<Self>>;
		// + IsSubType<Call<Self>>
		// + IsType<<Self as frame_system::Config>::Call>;

		/// The caller origin, overarching type of all pallets origins.
		type PalletsOrigin: Parameter
			+ Into<<Self as frame_system::Config>::Origin>
			+ IsType<<<Self as frame_system::Config>::Origin as frame_support::traits::OriginTrait>::PalletsOrigin>;

		type NativeAssetId: Get<AssetId>;

		type MaxExchangeBalance: Get<Balance>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event {
		/// A call was made with specified payment asset
		CallWithFeePreferences {
			payment_asset: AssetId,
			predicted_weight: Weight,
			used_weight: Option<Weight>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The call failed to dispatch
		DispatchFailed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// call an internal call with specified gas token
		/// TODO Better weight estimate
		#[pallet::weight(100000)]
		pub fn call_with_fee_preferences(
			origin: OriginFor<T>,
			call: Box<<T as Config>::Call>,
			payment_asset: AssetId,
		) -> DispatchResult {
			let _ = ensure_signed(origin.clone())?;

			// let call = <T as Config>::Call::from(call);
			let dispatch_info: DispatchInfo = call.get_dispatch_info();
			let predicted_weight = dispatch_info.weight;
			// let origin = <T as Config>::Origin::from(origin);

			// TODO Some limit on nested fee preferences calls
			// TODO better errors
			let post_dispatch_info = call.dispatch(origin).map_err(|err| err.error)?;

			let used_weight = post_dispatch_info.actual_weight;
			// Deposit runtime event
			Self::deposit_event(Event::CallWithFeePreferences {
				payment_asset,
				predicted_weight,
				used_weight,
			});

			// match post_dispatch_info {
			// 	Ok(dispatch_info) => {
			// 		let used_weight = dispatch_info.actual_weight;
			// 		// Deposit runtime event
			// 		Self::deposit_event(Event::CallWithFeePreferences {
			// 			payment_asset,
			// 			predicted_weight,
			// 			used_weight,
			// 		});
			// 	},
			// 	Err(e) => return Err(Error::<T>::DispatchFailed.into()),
			// }

			Ok(())
		}
	}
}
