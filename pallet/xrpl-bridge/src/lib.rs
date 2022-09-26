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

use crate::helpers::{XrpRequestLog, XrpTransaction, XrpWithdrawTransaction, XrplTxData};
use frame_support::{
	pallet_prelude::*,
	traits::{
		fungibles::{Inspect, Mutate, Transfer},
		UnixTime,
	},
	transactional,
	weights::constants::RocksDbWeight as DbWeight,
};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use seed_pallet_common::CreateExt;
use seed_primitives::{
	AccountId, AssetId, Balance, LedgerIndex, Timestamp, XrplTxHash, XrplWithdrawAddress,
	XrplWithdrawTxNonce,
};
use sp_runtime::{traits::One, ArithmeticError, DigestItem};
use sp_std::vec;

pub use pallet::*;

use sp_std::prelude::*;

mod helpers;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_relayer;

pub mod weights;

type AccountOf<T> = <T as frame_system::Config>::AccountId;

pub use weights::WeightInfo;

pub const XRPL_ENGINE_ID: sp_runtime::ConsensusEngineId = *b"XRP-";

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

		/// Allowed origins to add/remove the relayers
		type ApproveOrigin: EnsureOrigin<Self::Origin>;

		/// Weight information
		type WeightInfo: WeightInfo;

		/// XRP Asset Id set at runtime
		#[pallet::constant]
		type XrpAssetId: Get<AssetId>;

		/// Challenge Period to wait for a challenge before processing the transaction
		#[pallet::constant]
		type ChallengePeriod: Get<u32>;

		/// Clear Period to wait for a transaction to be cleared from settled storages
		#[pallet::constant]
		type ClearTxPeriod: Get<u32>;

		/// Unix time
		type UnixTime: UnixTime;
	}

	#[pallet::error]
	pub enum Error<T> {
		NotPermitted,
		RelayerDoesNotExists,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		TransactionAdded(LedgerIndex, XrplTxHash),
		TransactionChallenge(LedgerIndex, XrplTxHash),
		Processed(LedgerIndex, XrplTxHash),
		WithdrawRequested(XrplWithdrawTxNonce),
		WithdrawSettled(XrplWithdrawTxNonce),
		RelayerAdded(T::AccountId),
		RelayerRemoved(T::AccountId),
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(n: T::BlockNumber) -> Weight {
			if ProcessXRPTransaction::<T>::contains_key(n) {
				let weights = Self::process_xrp_tx(n);
				weights + Self::clear_storages(n)
			} else {
				DbWeight::get().reads(1 as Weight)
			}
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::storage]
	#[pallet::getter(fn get_relayer)]
	/// List of all XRP transaction relayers
	pub type Relayer<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, bool>;

	#[pallet::storage]
	#[pallet::getter(fn relay_xrp_transaction)]
	/// Transaction submitted by relayers
	pub type RelayXRPTransaction<T: Config> = StorageNMap<
		_,
		(
			storage::Key<Blake2_128Concat, T::AccountId>,
			storage::Key<Blake2_128Concat, LedgerIndex>,
			storage::Key<Blake2_128Concat, XrplTxHash>,
		),
		XrpTransaction,
	>;

	#[pallet::storage]
	#[pallet::getter(fn process_xrp_transaction)]
	/// Temporary storage to set the transactions ready to be processed at specified block number
	pub type ProcessXRPTransaction<T: Config> =
		StorageMap<_, Blake2_128Concat, T::BlockNumber, Vec<XrplTxHash>>;

	#[pallet::storage]
	#[pallet::getter(fn process_xrp_transaction_details)]
	/// Temporary storage to store transaction details to be processed, it will be cleared after
	/// transaction is processed
	pub type ProcessXRPTransactionDetails<T: Config> =
		StorageMap<_, Blake2_128Concat, XrplTxHash, (LedgerIndex, XrpTransaction)>;

	#[pallet::storage]
	#[pallet::getter(fn settled_xrp_transaction_details)]
	/// Settled xrp transactions stored as history for a specific period
	pub type SettledXRPTransactionDetails<T: Config> =
		StorageMap<_, Blake2_128Concat, T::BlockNumber, Vec<XrplTxHash>>;

	#[pallet::storage]
	#[pallet::getter(fn challenge_xrp_transaction_list)]
	/// Challenge received for a transaction mapped by hash, will be cleared when validator
	/// validates
	pub type ChallengeXRPTransactionList<T: Config> =
		StorageMap<_, Blake2_128Concat, XrplTxHash, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn pending_withdraw_xrp_transaction)]
	/// The list of pending transaction nonce id to be processed by xrp gadget to settle transaction
	/// on ripple network
	pub type PendingWithdrawXRPTransaction<T: Config> =
		StorageValue<_, Vec<XrplWithdrawTxNonce>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn settled_withdraw_xrp_transaction)]
	/// Settled xrp withdraw transactions on ripple network
	pub type SettledWithdrawXRPTransaction<T: Config> =
		StorageMap<_, Blake2_128Concat, T::BlockNumber, Vec<XrplWithdrawTxNonce>>;

	#[pallet::storage]
	#[pallet::getter(fn withdraw_xrp_transaction_details)]
	/// Xrp withdraw transaction details
	pub type WithdrawXRPTransactionDetails<T: Config> =
		StorageMap<_, Blake2_128Concat, XrplWithdrawTxNonce, XrpWithdrawTransaction>;

	#[pallet::storage]
	#[pallet::getter(fn get_withdraw_tx_nonce)]
	/// Stores and increments Withdraw Tx Nonce id
	pub type CurrentWithdrawTxNonce<T: Config> = StorageValue<_, XrplWithdrawTxNonce>;

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
			transaction_hash: XrplTxHash,
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
			transaction_hash: XrplTxHash,
		) -> DispatchResultWithPostInfo {
			let challenger = ensure_signed(origin)?;
			ChallengeXRPTransactionList::<T>::insert(&transaction_hash, challenger);
			Ok(().into())
		}

		/// Withdraw xrp transaction
		#[pallet::weight((<T as Config>::WeightInfo::withdraw_xrp(), DispatchClass::Operational))]
		#[transactional]
		pub fn withdraw_xrp(
			origin: OriginFor<T>,
			amount: Balance,
			destination: XrplWithdrawAddress,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::add_to_withdraw(who, amount, destination)
		}

		/// Withdraw xrp transaction
		#[pallet::weight((<T as Config>::WeightInfo::withdraw_xrp_settled(), DispatchClass::Operational))]
		#[transactional]
		pub fn withdraw_xrp_settled(
			origin: OriginFor<T>,
			tx_nonce_list: Vec<XrplWithdrawTxNonce>,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			// Todo check required for authorized access
			for tx_nonce in tx_nonce_list {
				let mut tx_list = <PendingWithdrawXRPTransaction<T>>::get();
				match tx_list.binary_search(&tx_nonce) {
					Ok(pos) => {
						tx_list.remove(pos);
						<PendingWithdrawXRPTransaction<T>>::put(tx_list);
						let clear_block_number = <frame_system::Pallet<T>>::block_number() +
							T::ClearTxPeriod::get().into();
						<SettledWithdrawXRPTransaction<T>>::append(&clear_block_number, &tx_nonce);
						Self::deposit_event(Event::WithdrawSettled(tx_nonce));
					},
					Err(_) => {},
				}
			}
			Ok(())
		}

		/// add a relayer
		#[pallet::weight((<T as Config>::WeightInfo::add_relayer(), DispatchClass::Operational))]
		#[transactional]
		pub fn add_relayer(
			origin: OriginFor<T>,
			relayer: T::AccountId,
		) -> DispatchResultWithPostInfo {
			T::ApproveOrigin::ensure_origin(origin)?;
			Self::initialize_relayer(&vec![relayer]);
			Self::deposit_event(Event::<T>::RelayerAdded(relayer));
			Ok(().into())
		}

		/// remove a relayer
		#[pallet::weight((<T as Config>::WeightInfo::remove_relayer(), DispatchClass::Operational))]
		#[transactional]
		pub fn remove_relayer(
			origin: OriginFor<T>,
			relayer: T::AccountId,
		) -> DispatchResultWithPostInfo {
			T::ApproveOrigin::ensure_origin(origin)?;
			if <Relayer<T>>::contains_key(relayer) {
				<Relayer<T>>::remove(relayer);
				Self::deposit_event(Event::<T>::RelayerRemoved(relayer));
				Ok(().into())
			} else {
				Err(Error::<T>::RelayerDoesNotExists.into())
			}
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
		let tx_items: Vec<XrplTxHash> = match <ProcessXRPTransaction<T>>::take(n) {
			None => return DbWeight::get().reads(2 as Weight),
			Some(v) => v,
		};
		let mut reads = 2 as Weight;
		let mut writes = 0 as Weight;
		for transaction_hash in tx_items {
			if !<ChallengeXRPTransactionList<T>>::contains_key(transaction_hash) {
				let tx_details = <ProcessXRPTransactionDetails<T>>::take(transaction_hash);
				reads += 1;
				match tx_details {
					None => {},
					Some((ledger_index, ref tx)) => {
						match tx.transaction {
							XrplTxData::Payment { amount, address } => {
								let _ = T::MultiCurrency::mint_into(
									T::XrpAssetId::get(),
									&address.into(),
									amount,
								);
								writes += 1;
							},
							XrplTxData::CurrencyPayment {
								amount: _,
								address: _,
								currency_id: _,
							} => {},
							XrplTxData::Xls20 => {},
						}
						let clear_block_number = <frame_system::Pallet<T>>::block_number() +
							T::ClearTxPeriod::get().into();
						<SettledXRPTransactionDetails<T>>::append(
							&clear_block_number,
							transaction_hash.clone(),
						);
						writes += 1;
						Self::deposit_event(Event::Processed(ledger_index, transaction_hash));
					},
				}
			}
		}
		DbWeight::get().reads_writes(reads, writes)
	}

	pub fn clear_storages(n: T::BlockNumber) -> Weight {
		let mut reads: Weight = 0;
		let mut writes: Weight = 0;
		reads += 1;
		if <SettledXRPTransactionDetails<T>>::contains_key(n) {
			<SettledXRPTransactionDetails<T>>::remove(n);
			writes += 1;
		}
		reads += 1;
		if <SettledWithdrawXRPTransaction<T>>::contains_key(n) {
			<SettledWithdrawXRPTransaction<T>>::remove(n);
			writes += 1;
		}
		DbWeight::get().reads_writes(reads, writes)
	}

	pub fn add_to_relay(
		relayer: AccountOf<T>,
		ledger_index: LedgerIndex,
		transaction_hash: XrplTxHash,
		transaction: XrplTxData,
		timestamp: Timestamp,
	) -> DispatchResultWithPostInfo {
		let val = XrpTransaction { transaction_hash, transaction, timestamp };
		<RelayXRPTransaction<T>>::insert((&relayer, &ledger_index, &transaction_hash), val.clone());
		<ProcessXRPTransactionDetails<T>>::insert(&transaction_hash, (ledger_index, val));
		Self::add_to_xrp_process(transaction_hash)?;
		Self::deposit_event(Event::TransactionAdded(ledger_index, transaction_hash));
		Ok(().into())
	}

	pub fn add_to_xrp_process(transaction_hash: XrplTxHash) -> DispatchResultWithPostInfo {
		let process_block_number =
			<frame_system::Pallet<T>>::block_number() + T::ChallengePeriod::get().into();
		ProcessXRPTransaction::<T>::append(&process_block_number, transaction_hash);
		Ok(().into())
	}

	pub fn add_to_withdraw(
		who: AccountOf<T>,
		amount: Balance,
		destination: XrplWithdrawAddress,
	) -> DispatchResultWithPostInfo {
		let _ = T::MultiCurrency::burn_from(T::XrpAssetId::get(), &who, amount)?;
		let tx_nonce = Self::withdraw_tx_nonce_inc()?;
		let tx_data = XrpWithdrawTransaction { tx_nonce, amount, destination };
		<PendingWithdrawXRPTransaction<T>>::append(&tx_nonce);
		<WithdrawXRPTransactionDetails<T>>::insert(&tx_nonce, &tx_data);
		Self::withdraw_request_deposit_log(tx_nonce, tx_data);
		Self::deposit_event(Event::WithdrawRequested(tx_nonce));
		Ok(().into())
	}

	pub fn withdraw_request_deposit_log(
		tx_nonce: XrplWithdrawTxNonce,
		tx_data: XrpWithdrawTransaction,
	) {
		let log: DigestItem = DigestItem::Consensus(
			XRPL_ENGINE_ID,
			XrpRequestLog::XrpWithdrawRequest(tx_nonce, tx_data).encode(),
		);
		<frame_system::Pallet<T>>::deposit_log(log);
	}

	pub fn withdraw_tx_nonce_inc() -> Result<XrplWithdrawTxNonce, DispatchError> {
		let tx_nonce = CurrentWithdrawTxNonce::<T>::get().unwrap_or(0);
		let next_tx_nonce = tx_nonce.checked_add(One::one()).ok_or(ArithmeticError::Overflow)?;
		CurrentWithdrawTxNonce::<T>::set(Some(next_tx_nonce));
		Ok(next_tx_nonce)
	}
}
