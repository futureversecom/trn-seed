#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

use frame_support::{
	pallet_prelude::*,
	PalletId,
};
use frame_system::pallet_prelude::*;
use seed_pallet_common::{
	 OnNewAssetSubscriber, TransferExt,
};
use seed_primitives::{AssetId, Balance, ParachainId};
use sp_runtime::traits::{AccountIdConversion, One, Zero};
use sp_std::prelude::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;


/// The inner value of a `PalletId`, extracted for convenience as `PalletId` is missing trait
/// derivations e.g. `Ord`
pub type PalletIdValue = [u8; 8];

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}


	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Something,
	}

	#[pallet::error]
	pub enum Error<T> {
		Something
	}



	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight((16_000_000 as Weight).saturating_add(T::DbWeight::get().reads_writes(1, 2)))]
		pub fn something(origin: OriginFor<T>) -> DispatchResult {
			Ok(())
		}
	}
}

