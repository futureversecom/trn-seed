/* Copyright 2019-2021 Centrality Investments Limited
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

use crate::helpers::XrpTransaction;
use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use seed_primitives::{BlockNumber, Timestamp};

pub use pallet::*;

use sp_std::prelude::*;

mod helpers;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

type AccountOf<T> = <T as frame_system::Config>::AccountId;

pub type RelayerId = u128;

pub use weights::WeightInfo;
pub type BoundedVecOfTransaction<T> = BoundedVec<u8, <T as Config>::TransactionLimit>;
pub type BoundedVecOfHash<T> = BoundedVec<u8, <T as Config>::HashLimit>;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Weight information
		type WeightInfo: WeightInfo;

		/// Hash Length
		#[pallet::constant]
		type HashLimit: Get<u32>;

		/// Transaction Length
		#[pallet::constant]
		type TransactionLimit: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		TransactionAdded(BlockNumber, BoundedVecOfHash<T>),
	}

	/// Global storage for relayers
	#[pallet::storage]
	#[pallet::getter(fn get_relayer)]
	pub type Relayer<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, Timestamp>;

	#[pallet::storage]
	#[pallet::getter(fn relay_xrp_transaction)]
	pub type RelayXRPTransaction<T: Config> = StorageNMap<
		_,
		(
			storage::Key<Blake2_128Concat, T::AccountId>,
			storage::Key<Blake2_128Concat, BlockNumber>,
			storage::Key<Blake2_128Concat, BoundedVecOfHash<T>>,
		),
		XrpTransaction<T>,
	>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set emergency price
		#[pallet::weight((<T as Config>::WeightInfo::submit_transaction(), DispatchClass::Operational))]
		#[transactional]
		pub fn submit_transaction(
			origin: OriginFor<T>,
			block_number: BlockNumber,
			hash: BoundedVecOfHash<T>,
			transaction: BoundedVecOfTransaction<T>,
			timestamp: Timestamp,
		) -> DispatchResultWithPostInfo {
			let relayer = ensure_signed(origin)?;
			Self::add_to_relay(relayer, block_number, hash, transaction, timestamp)
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn add_to_relay(
		relayer: AccountOf<T>,
		block_number: BlockNumber,
		hash: BoundedVecOfHash<T>,
		transaction: BoundedVecOfTransaction<T>,
		timestamp: Timestamp,
	) -> DispatchResultWithPostInfo {
		<Relayer<T>>::insert(relayer.clone(), timestamp);
		let val =
			XrpTransaction { hash: hash.clone(), transaction: transaction.clone(), timestamp };
		<RelayXRPTransaction<T>>::insert((&relayer, &block_number, &hash), val);
		Self::deposit_event(Event::TransactionAdded(block_number, hash));
		Ok(().into())
	}
}
