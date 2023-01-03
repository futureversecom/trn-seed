/* Copyright 2021 Centrality Investments Limited
 *
 * Licensed under the LGPL, Version 3.0 (the "License");
 * you may not use this file except in compliance with the License.
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * You may obtain a copy of the License at the root of this project source code,
 * or at:
 *     https://centrality.ai/licenses/gplv3.txt
 *     https://centrality.ai/licenses/lgplv3.txt
 */
#![cfg_attr(not(feature = "std"), no_std)]

use codec::Encode;
use frame_support::{ensure, fail, traits::Get, weights::Weight, BoundedVec, PalletId};
pub use pallet::*;
use seed_pallet_common::{EthereumBridge, EthereumEventSubscriber};
use seed_primitives::{CollectionUuid, SerialNumber};
use sp_core::{H160, U256};
use sp_runtime::{traits::AccountIdConversion, DispatchError, SaturatedConversion};
use sp_std::{boxed::Box, vec, vec::Vec};
use seed_primitives::ethy::EventProofId;

pub mod types;
use types::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, transactional};
	use frame_system::{ensure_signed, pallet_prelude::*};
	use seed_pallet_common::ethy::State;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type PalletId: Get<PalletId>;
	}

	#[pallet::storage]
	#[pallet::getter(fn bridge_state)]
	/// Bridge state. whether it's active or paused
	pub type EthyState<T> = StorageValue<_, State, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> where
		<T as frame_system::Config>::AccountId: From<sp_core::H160> + Into<sp_core::H160>
	{
	}
}

impl<T: Config> Pallet<T> where <T as frame_system::Config>::AccountId: From<sp_core::H160> {}
