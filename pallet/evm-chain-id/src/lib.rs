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

//! # EVM chain ID pallet
//!
//! The pallet that stores the numeric Ethereum-style chain id in the runtime.
//! It can simplify setting up multiple networks with different chain ID by configuring the
//! chain spec without requiring changes to the runtime config.
//!
//! **NOTE**: we recommend that the production chains still use the const parameter type, as
//! this extra storage access would imply some performance penalty.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod weights;

pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The system event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Allowed origins to ease transition to council givernance
		type ApproveOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		/// The default chain ID to use if not set in the chain spec
		type DefaultChainId: Get<u64>;
		/// Interface to access weight values
		type WeightInfo: WeightInfo;
	}

	impl<T: Config> Get<u64> for Pallet<T> {
		fn get() -> u64 {
			ChainId::<T>::get()
		}
	}

	#[pallet::type_value]
	pub fn DefaultChainId<T: Config>() -> u64 {
		T::DefaultChainId::get()
	}

	/// The EVM chain ID.
	#[pallet::storage]
	pub type ChainId<T> = StorageValue<_, u64, ValueQuery, DefaultChainId<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T> {
		ChainIdSet(u64),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_chain_id())]
		pub fn set_chain_id(
			origin: OriginFor<T>,
			#[pallet::compact] chain_id: u64,
		) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;
			ChainId::<T>::put(chain_id);
			Self::deposit_event(Event::<T>::ChainIdSet(chain_id));
			Ok(())
		}
	}
}
