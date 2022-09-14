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

use crate::helpers::{XrpTransaction, XrplTxData};
use frame_support::{
	pallet_prelude::*,
	traits::fungibles::{Inspect, Mutate, Transfer},
	transactional,
};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use seed_pallet_common::CreateExt;
use seed_primitives::{AccountId, AssetId, Balance, LedgerIndex, Timestamp};
use sp_core::H512;
use sp_std::vec;

pub use pallet::*;

use sp_std::prelude::*;

mod helpers;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

type AccountOf<T> = <T as frame_system::Config>::AccountId;

pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type MultiCurrency: CreateExt<AccountId = Self::AccountId>
			+ Transfer<Self::AccountId, Balance = Balance>
			+ Inspect<Self::AccountId, AssetId = AssetId>
			+ Mutate<Self::AccountId>;

		/// Weight information
		type WeightInfo: WeightInfo;

		/// Transaction Length
		#[pallet::constant]
		type XrpAssetId: Get<AssetId>;

		/// Transaction Length
		#[pallet::constant]
		type ChallengePeriod: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {
		NotPermitted,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		TransactionAdded(LedgerIndex, H512),
		TransactionChallenge(LedgerIndex, H512),
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(n: T::BlockNumber) -> Weight {
			if ProcessXRPTransaction::<T>::contains_key(n) {
				Self::process_xrp_tx(n)
			} else {
				10_000
			}
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	/// Global storage for relayers
	#[pallet::storage]
	#[pallet::getter(fn get_relayer)]
	pub type Relayer<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, bool>;

	#[pallet::storage]
	#[pallet::getter(fn relay_xrp_transaction)]
	pub type RelayXRPTransaction<T: Config> = StorageNMap<
		_,
		(
			storage::Key<Blake2_128Concat, T::AccountId>,
			storage::Key<Blake2_128Concat, LedgerIndex>,
			storage::Key<Blake2_128Concat, H512>,
		),
		XrpTransaction,
	>;

	#[pallet::storage]
	#[pallet::getter(fn process_xrp_transaction)]
	pub type ProcessXRPTransaction<T: Config> =
		StorageMap<_, Blake2_128Concat, T::BlockNumber, Vec<H512>>;

	#[pallet::storage]
	#[pallet::getter(fn process_xrp_transaction_details)]
	pub type ProcessXRPTransactionDetails<T: Config> =
		StorageMap<_, Blake2_128Concat, H512, (LedgerIndex, XrpTransaction)>;

	#[pallet::storage]
	#[pallet::getter(fn settled_xrp_transaction_details)]
	pub type SettledXRPTransactionDetails<T: Config> =
		StorageMap<_, Blake2_128Concat, H512, (LedgerIndex, XrpTransaction)>;

	#[pallet::storage]
	#[pallet::getter(fn challenge_xrp_transaction_list)]
	pub type ChallengeXRPTransactionList<T: Config> =
		StorageMap<_, Blake2_128Concat, H512, Vec<T::AccountId>>;

	#[pallet::storage]
	#[pallet::getter(fn challenge_xrp_transaction_details)]
	pub type ChallengeXRPTransactionDetails<T: Config> = StorageNMap<
		_,
		(storage::Key<Blake2_128Concat, T::AccountId>, storage::Key<Blake2_128Concat, H512>),
		(LedgerIndex, XrpTransaction),
	>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub xrp_relayers: Vec<T::AccountId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { xrp_relayers: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::initialize_relayer(&self.xrp_relayers);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit xrp transaction
		#[pallet::weight((<T as Config>::WeightInfo::submit_transaction(), DispatchClass::Operational))]
		#[transactional]
		pub fn submit_transaction(
			origin: OriginFor<T>,
			ledger_index: LedgerIndex,
			transaction_hash: H512,
			transaction: XrplTxData,
			timestamp: Timestamp,
		) -> DispatchResultWithPostInfo {
			let relayer = ensure_signed(origin)?;
			let active_relayer = <Relayer<T>>::get(&relayer).unwrap_or(false);
			ensure!(active_relayer, Error::<T>::NotPermitted);
			Self::add_to_relay(relayer, ledger_index, transaction_hash, transaction, timestamp)
		}

		/// Submit xrp transaction challenge
		#[pallet::weight((<T as Config>::WeightInfo::submit_challenge(), DispatchClass::Operational))]
		#[transactional]
		pub fn submit_challenge(
			origin: OriginFor<T>,
			ledger_index: LedgerIndex,
			transaction_hash: H512,
			transaction: XrplTxData,
			timestamp: Timestamp,
		) -> DispatchResultWithPostInfo {
			let challenger = ensure_signed(origin)?;
			Self::add_to_challenge(
				challenger,
				ledger_index,
				transaction_hash,
				transaction,
				timestamp,
			)
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn initialize_relayer(xrp_relayers: &Vec<T::AccountId>) {
		for relayer in xrp_relayers {
			<Relayer<T>>::insert(relayer, true);
		}
	}

	pub fn process_xrp_tx(n: T::BlockNumber) -> Weight {
		let tx_items: Vec<H512> = match <ProcessXRPTransaction<T>>::get(n) {
			None => return 10_000,
			Some(v) => v,
		};
		for transaction_hash in tx_items {
			if !<ChallengeXRPTransactionList<T>>::contains_key(transaction_hash) {
				let tx_details = <ProcessXRPTransactionDetails<T>>::get(transaction_hash);
				match tx_details {
					None => {},
					Some((ledger_index, tx)) => {
						match tx.transaction {
							XrplTxData::Payment { amount, address } => {
								let _ = T::MultiCurrency::mint_into(
									T::XrpAssetId::get(),
									&address.into(),
									amount,
								);
							},
							XrplTxData::CurrencyPayment {
								amount: _,
								address: _,
								currency_id: _,
							} => {},
							XrplTxData::Xls20 => {},
						}
						<SettledXRPTransactionDetails<T>>::insert(
							&transaction_hash,
							(ledger_index, tx),
						);
						<ProcessXRPTransactionDetails<T>>::remove(&transaction_hash);
					},
				}
			}
		}
		<ProcessXRPTransaction<T>>::remove(n);
		10_000
	}

	pub fn add_to_relay(
		relayer: AccountOf<T>,
		ledger_index: LedgerIndex,
		transaction_hash: H512,
		transaction: XrplTxData,
		timestamp: Timestamp,
	) -> DispatchResultWithPostInfo {
		let val = XrpTransaction { transaction_hash, transaction, timestamp };
		<RelayXRPTransaction<T>>::insert((&relayer, &ledger_index, &transaction_hash), val.clone());
		<ProcessXRPTransactionDetails<T>>::insert(&transaction_hash, (ledger_index, val));
		Self::add_to_xrp_process(transaction_hash).expect("Failed to add to challenger list");
		Self::deposit_event(Event::TransactionAdded(ledger_index, transaction_hash));
		Ok(().into())
	}

	pub fn add_to_xrp_process(transaction_hash: H512) -> DispatchResultWithPostInfo {
		let process_block_number =
			<frame_system::Pallet<T>>::block_number() + T::ChallengePeriod::get().into();
		let value = ProcessXRPTransaction::<T>::get(&process_block_number);

		match value {
			None =>
				ProcessXRPTransaction::<T>::insert(&process_block_number, vec![transaction_hash]),
			Some(mut list) => match list.binary_search(&transaction_hash) {
				Ok(_) => {},
				Err(pos) => {
					list.insert(pos, transaction_hash);
					ProcessXRPTransaction::<T>::insert(&process_block_number, list);
				},
			},
		}
		Ok(().into())
	}

	pub fn add_to_challenge(
		challenger: AccountOf<T>,
		ledger_index: LedgerIndex,
		transaction_hash: H512,
		transaction: XrplTxData,
		timestamp: Timestamp,
	) -> DispatchResultWithPostInfo {
		let val = XrpTransaction { transaction_hash, transaction, timestamp };
		<ChallengeXRPTransactionDetails<T>>::insert(
			(&challenger, &transaction_hash),
			(ledger_index, val),
		);
		Self::add_to_challenge_list(challenger, transaction_hash)
			.expect("Failed to add to challenger list");
		Self::deposit_event(Event::TransactionChallenge(ledger_index, transaction_hash));
		Ok(().into())
	}

	pub fn add_to_challenge_list(
		challenger: AccountOf<T>,
		transaction_hash: H512,
	) -> DispatchResultWithPostInfo {
		let value = ChallengeXRPTransactionList::<T>::get(&transaction_hash);

		match value {
			None => ChallengeXRPTransactionList::<T>::insert(&transaction_hash, vec![challenger]),
			Some(mut list) => match list.binary_search(&challenger) {
				Ok(_) => {},
				Err(pos) => {
					list.insert(pos, challenger);
					ChallengeXRPTransactionList::<T>::insert(&transaction_hash, list);
				},
			},
		}
		Ok(().into())
	}
}
