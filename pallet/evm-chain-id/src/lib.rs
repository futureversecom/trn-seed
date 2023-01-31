// SPDX-License-Identifier: Apache-2.0
// This file is part of Frontier.
//
// Copyright (c) 2022 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, transactional};
	use frame_system::pallet_prelude::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The system event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	impl<T: Config> Get<u64> for Pallet<T> {
		fn get() -> u64 {
			Self::chain_id()
		}
	}

	/// The EVM chain ID.
	#[pallet::storage]
	#[pallet::getter(fn chain_id)]
	pub type ChainId<T> = StorageValue<_, u64, ValueQuery>;

	#[pallet::genesis_config]
	#[derive(Default)]
	pub struct GenesisConfig {
		pub chain_id: u64,
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			ChainId::<T>::put(self.chain_id);
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T> {
		ChainIdSet(u64),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight((T::DbWeight::get().reads_writes(1, 1), DispatchClass::Operational))]
		#[transactional]
		pub fn set_chain_id(
			origin: OriginFor<T>,
			#[pallet::compact] chain_id: u64,
		) -> DispatchResult {
			ensure_root(origin)?;
			ChainId::<T>::put(chain_id);
			Self::deposit_event(Event::<T>::ChainIdSet(chain_id));
			Ok(())
		}
	}
}
