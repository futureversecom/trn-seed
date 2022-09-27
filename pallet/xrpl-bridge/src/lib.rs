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
use sp_runtime::{
	traits::{One, Zero},
	ArithmeticError, SaturatedConversion,
};
use sp_std::{prelude::*, vec};
use xrpl_codec::{traits::BinarySerialize, transaction::Payment};

use seed_pallet_common::{CreateExt, EthyXrplBridgeAdapter};
use seed_primitives::{
	xrpl::{XrplTxHash, XrplWithdrawAddress, XrplWithdrawTxNonce},
	AccountId, AssetId, Balance, LedgerIndex, Timestamp,
};

use crate::helpers::{XrpTransaction, XrpWithdrawTransaction, XrplTxData};

pub use pallet::*;

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

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type EthyAdapter: EthyXrplBridgeAdapter;

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

		/// Unix time
		type UnixTime: UnixTime;
	}

	#[pallet::error]
	pub enum Error<T> {
		NotPermitted,
		RelayerDoesNotExists,
		/// Withdraw amount must be non-zero and <= u64
		WithdrawInvalidAmount,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		TransactionAdded(LedgerIndex, XrplTxHash),
		TransactionChallenge(LedgerIndex, XrplTxHash),
		Processed(LedgerIndex, XrplTxHash),
		WithdrawRequest {
			tx_blob: Vec<u8>,
			proof_id: u64,
			sender: T::AccountId,
			amount: Balance,
			destination: XrplWithdrawAddress,
		},
		RelayerAdded(T::AccountId),
		RelayerRemoved(T::AccountId),
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(n: T::BlockNumber) -> Weight {
			if ProcessXRPTransaction::<T>::contains_key(n) {
				Self::process_xrp_tx(n)
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
		StorageMap<_, Blake2_128Concat, XrplTxHash, Timestamp>;

	#[pallet::storage]
	#[pallet::getter(fn challenge_xrp_transaction_list)]
	/// Challenge received for a transaction mapped by hash, will be cleared when validator
	/// validates
	pub type ChallengeXRPTransactionList<T: Config> =
		StorageMap<_, Blake2_128Concat, XrplTxHash, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn door_nonce)]
	/// The nonce/sequence of the XRPL door account
	pub type DoorNonce<T: Config> = StorageValue<_, XrplWithdrawTxNonce, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn door_signers)]
	/// Public keys of authorized door (multi) signers (subset of ethy session keys)
	pub type DoorSigners<T: Config> = StorageValue<_, Vec<[u8; 33]>, ValueQuery>;

	/// Default door tx fee 1 XRP
	#[pallet::type_value]
	pub fn DefaultDoorTxFee() -> u64 {
		1_000_000_u64
	}
	#[pallet::storage]
	#[pallet::getter(fn door_tx_fee)]
	/// The flat fee for XRPL door txs
	pub type DoorTxFee<T: Config> = StorageValue<_, u64, ValueQuery, DefaultDoorTxFee>;

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
		) -> DispatchResult {
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
		) -> DispatchResult {
			let challenger = ensure_signed(origin)?;
			ChallengeXRPTransactionList::<T>::insert(&transaction_hash, challenger);
			Ok(())
		}

		/// Withdraw xrp transaction
		#[pallet::weight((<T as Config>::WeightInfo::withdraw_xrp(), DispatchClass::Operational))]
		#[transactional]
		pub fn withdraw_xrp(
			origin: OriginFor<T>,
			amount: Balance,
			destination: XrplWithdrawAddress,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::add_to_withdraw(who, amount, destination)
		}

		/// add a relayer
		#[pallet::weight((<T as Config>::WeightInfo::add_relayer(), DispatchClass::Operational))]
		#[transactional]
		pub fn add_relayer(origin: OriginFor<T>, relayer: T::AccountId) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;
			Self::initialize_relayer(&vec![relayer]);
			Self::deposit_event(Event::<T>::RelayerAdded(relayer));
			Ok(())
		}

		/// remove a relayer
		#[pallet::weight((<T as Config>::WeightInfo::remove_relayer(), DispatchClass::Operational))]
		#[transactional]
		pub fn remove_relayer(origin: OriginFor<T>, relayer: T::AccountId) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;
			if <Relayer<T>>::contains_key(relayer) {
				<Relayer<T>>::remove(relayer);
				Self::deposit_event(Event::<T>::RelayerRemoved(relayer));
				Ok(())
			} else {
				Err(Error::<T>::RelayerDoesNotExists.into())
			}
		}

		/// Set the door account tx nonce
		#[pallet::weight((<T as Config>::WeightInfo::set_door_nonce(), DispatchClass::Operational))]
		pub fn set_door_nonce(origin: OriginFor<T>, nonce: u32) -> DispatchResult {
			ensure_root(origin)?;
			DoorNonce::<T>::set(nonce);
			Ok(())
		}

		/// Set the door tx fee amount
		#[pallet::weight((<T as Config>::WeightInfo::set_door_nonce(), DispatchClass::Operational))]
		pub fn set_door_tx_fee(origin: OriginFor<T>, fee: u64) -> DispatchResult {
			ensure_root(origin)?;
			DoorTxFee::<T>::set(fee);
			Ok(())
		}

		/// Set the door signers
		#[pallet::weight((<T as Config>::WeightInfo::set_door_nonce(), DispatchClass::Operational))]
		pub fn set_door_signers(
			origin: OriginFor<T>,
			new_signers: Vec<[u8; 33]>,
		) -> DispatchResult {
			ensure_root(origin)?;
			DoorSigners::<T>::set(new_signers);
			Ok(())
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
						<SettledXRPTransactionDetails<T>>::insert(
							&transaction_hash,
							T::UnixTime::now().as_secs(),
						);
						writes += 1;
						Self::deposit_event(Event::Processed(ledger_index, transaction_hash));
					},
				}
			}
		}
		DbWeight::get().reads_writes(reads, writes)
	}

	pub fn add_to_relay(
		relayer: AccountOf<T>,
		ledger_index: LedgerIndex,
		transaction_hash: XrplTxHash,
		transaction: XrplTxData,
		timestamp: Timestamp,
	) -> DispatchResult {
		let val = XrpTransaction { transaction_hash, transaction, timestamp };
		<RelayXRPTransaction<T>>::insert((&relayer, &ledger_index, &transaction_hash), val.clone());
		<ProcessXRPTransactionDetails<T>>::insert(&transaction_hash, (ledger_index, val));
		Self::add_to_xrp_process(transaction_hash)?;
		Self::deposit_event(Event::TransactionAdded(ledger_index, transaction_hash));
		Ok(())
	}

	pub fn add_to_xrp_process(transaction_hash: XrplTxHash) -> DispatchResult {
		let process_block_number =
			<frame_system::Pallet<T>>::block_number() + T::ChallengePeriod::get().into();
		ProcessXRPTransaction::<T>::append(&process_block_number, transaction_hash);
		Ok(())
	}

	///
	/// `who` the account requesting the withdraw
	/// `amount` the amount of XRP drops to withdraw (- the tx fee)
	///  `destination` the receiver classic `AccountID` on XRPL
	pub fn add_to_withdraw(
		who: AccountOf<T>,
		amount: Balance,
		destination: XrplWithdrawAddress,
	) -> DispatchResult {
		// TODO: need a fee oracle, this is over estimating the fee
		// https://github.com/futureversecom/seed/issues/107
		let tx_fee = Self::door_tx_fee();
		ensure!(!amount.is_zero(), Error::<T>::WithdrawInvalidAmount);
		ensure!(amount.checked_add(tx_fee as Balance).is_some(), Error::<T>::WithdrawInvalidAmount); // xrp amounts are `u64`

		// the door address pays the tx fee on XRPL. Therefore the withdrawn amount must include the
		// tx fee to maintain an accurate door balance
		let _ =
			T::MultiCurrency::burn_from(T::XrpAssetId::get(), &who, amount + tx_fee as Balance)?;
		let tx_nonce = Self::door_nonce_inc()?;
		let tx_data = XrpWithdrawTransaction { tx_nonce, tx_fee, amount, destination };

		let (proof_id, tx_blob) = Self::submit_withdraw_request(tx_data)?;

		Self::deposit_event(Event::WithdrawRequest {
			proof_id,
			tx_blob,
			sender: who,
			amount,
			destination,
		});

		Ok(())
	}

	/// Construct an XRPL payment transaction and submit for signing
	/// Returns a (proof_id, tx_blob)
	fn submit_withdraw_request(
		tx_data: XrpWithdrawTransaction,
	) -> Result<(u64, Vec<u8>), DispatchError> {
		let XrpWithdrawTransaction { tx_fee, tx_nonce, amount, destination } = tx_data;

		// TODO: use pallet config
		// rnZiKvrWFGi2JfHtLS8kxcqCqVhch6W5k5
		let door_address: [u8; 20] = hex_literal::hex!("3216fd40be8f9b0016253e5244085375d887a53e");

		let payment = Payment::new(
			door_address.into(),
			destination.into(),
			amount.saturated_into(),
			tx_nonce,
			tx_fee,
			// omit signer key since this is a 'MultiSigner' tx
			None,
		);
		let tx_blob = payment.binary_serialize(true);

		T::EthyAdapter::sign_xrpl_transaction(tx_blob.as_slice())
			.map(|event_proof_id| (event_proof_id, tx_blob))
	}

	// Return the current door nonce and increment it in storage
	pub fn door_nonce_inc() -> Result<XrplWithdrawTxNonce, DispatchError> {
		let nonce = Self::door_nonce();
		let next_nonce = nonce.checked_add(One::one()).ok_or(ArithmeticError::Overflow)?;
		DoorNonce::<T>::set(next_nonce);
		Ok(nonce)
	}
}
