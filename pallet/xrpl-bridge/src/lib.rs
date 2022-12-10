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
	fail,
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
	ArithmeticError, Percent, SaturatedConversion,
};
use sp_std::{prelude::*, vec};
use xrpl_codec::{
	traits::BinarySerialize,
	transaction::{Payment, SignerListSet},
};

use seed_pallet_common::{CreateExt, EthyToXrplBridgeAdapter, XrplBridgeToEthyAdapter};
use seed_primitives::{
	ethy::crypto::AuthorityId,
	xrpl::{LedgerIndex, XrplAccountId, XrplTxHash, XrplTxNonce},
	AccountId, AssetId, Balance, Timestamp,
};

use crate::helpers::{
	XrpTransaction, XrpWithdrawTransaction, XrplTicketSequenceParams, XrplTxData,
};

pub use pallet::*;
use seed_primitives::{ethy::EventProofId, xrpl::XrplTxTicketSequence};

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
	use seed_primitives::xrpl::XrplTxTicketSequence;

	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId> {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type EthyAdapter: XrplBridgeToEthyAdapter<AuthorityId>;

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

		/// Threshold to emit event TicketSequenceThresholdReached
		type TicketSequenceThreshold: Get<Percent>;
	}

	#[pallet::error]
	pub enum Error<T> {
		NotPermitted,
		RelayerDoesNotExists,
		/// Withdraw amount must be non-zero and <= u64
		WithdrawInvalidAmount,
		/// The door address has not been configured
		DoorAddressNotSet,
		/// XRPL does not allow more than 8 signers for door address
		TooManySigners,
		/// The signers are not known by ethy
		InvalidSigners,
		/// Submitted a duplicate transaction hash
		TxReplay,
		/// The NextTicketSequenceParams has not been set
		NextTicketSequenceParamsNotSet,
		/// The NextTicketSequenceParams is invalid
		NextTicketSequenceParamsInvalid,
		/// The TicketSequenceParams is invalid
		TicketSequenceParamsInvalid,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		TransactionAdded(LedgerIndex, XrplTxHash),
		TransactionChallenge(LedgerIndex, XrplTxHash),
		Processed(LedgerIndex, XrplTxHash),
		/// Request to withdraw some XRP amount to XRPL
		WithdrawRequest {
			proof_id: u64,
			sender: T::AccountId,
			amount: Balance,
			destination: XrplAccountId,
		},
		RelayerAdded(T::AccountId),
		RelayerRemoved(T::AccountId),
		DoorAddressSet(XrplAccountId),
		DoorNextTicketSequenceParamSet {
			ticket_sequence_start_next: u32,
			ticket_bucket_size_next: u32,
		},
		DoorTicketSequenceParamSet {
			ticket_sequence: u32,
			ticket_sequence_start: u32,
			ticket_bucket_size: u32,
		},
		TicketSequenceThresholdReached(u32),
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(n: T::BlockNumber) -> Weight {
			let weights = Self::process_xrp_tx(n);
			weights + Self::clear_storages(n)
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
	#[pallet::getter(fn process_xrp_transaction)]
	/// Temporary storage to set the transactions ready to be processed at specified block number
	pub type ProcessXRPTransaction<T: Config> =
		StorageMap<_, Twox64Concat, T::BlockNumber, Vec<XrplTxHash>>;

	#[pallet::storage]
	#[pallet::getter(fn process_xrp_transaction_details)]
	/// Stores submitted transactions from XRPL waiting to be processed
	/// Transactions will be cleared `ClearTxPeriod` blocks after processing
	pub type ProcessXRPTransactionDetails<T: Config> =
		StorageMap<_, Identity, XrplTxHash, (LedgerIndex, XrpTransaction, T::AccountId)>;

	#[pallet::storage]
	#[pallet::getter(fn settled_xrp_transaction_details)]
	/// Settled xrp transactions stored as history for a specific period
	pub type SettledXRPTransactionDetails<T: Config> =
		StorageMap<_, Twox64Concat, T::BlockNumber, Vec<XrplTxHash>>;

	#[pallet::storage]
	#[pallet::getter(fn challenge_xrp_transaction_list)]
	/// Challenge received for a transaction mapped by hash, will be cleared when validator
	/// validates
	pub type ChallengeXRPTransactionList<T: Config> =
		StorageMap<_, Identity, XrplTxHash, T::AccountId>;

	#[pallet::type_value]
	pub fn DefaultDoorNonce() -> u32 {
		0_u32
	}
	#[pallet::storage]
	#[pallet::getter(fn door_nonce)]
	/// The nonce/sequence of the XRPL door account
	pub type DoorNonce<T: Config> = StorageValue<_, XrplTxNonce, ValueQuery, DefaultDoorNonce>;

	#[pallet::type_value]
	pub fn DefaultDoorTicketSequence() -> u32 {
		0_u32
	}
	#[pallet::storage]
	#[pallet::getter(fn door_ticket_sequence)]
	/// The current ticket sequence of the XRPL door account
	pub type DoorTicketSequence<T: Config> =
		StorageValue<_, XrplTxTicketSequence, ValueQuery, DefaultDoorTicketSequence>;

	#[pallet::storage]
	#[pallet::getter(fn door_ticket_sequence_params)]
	/// The Ticket sequence params of the XRPL door account for the current allocation
	pub type DoorTicketSequenceParams<T: Config> =
		StorageValue<_, XrplTicketSequenceParams, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn door_ticket_sequence_params_next)]
	/// The Ticket sequence params of the XRPL door account for the next allocation
	pub type DoorTicketSequenceParamsNext<T: Config> =
		StorageValue<_, XrplTicketSequenceParams, ValueQuery>;

	#[pallet::type_value]
	pub fn DefaultTicketSequenceThresholdReachedEmitted() -> bool {
		false
	}
	#[pallet::storage]
	#[pallet::getter(fn ticket_sequence_threshold_reached_emitted)]
	/// Keeps track whether the TicketSequenceThresholdReached event is emitted
	pub type TicketSequenceThresholdReachedEmitted<T: Config> =
		StorageValue<_, bool, ValueQuery, DefaultTicketSequenceThresholdReachedEmitted>;

	/// Default door tx fee 1 XRP
	#[pallet::type_value]
	pub fn DefaultDoorTxFee() -> u64 {
		1_000_000_u64
	}

	#[pallet::storage]
	#[pallet::getter(fn door_tx_fee)]
	/// The flat fee for XRPL door txs
	pub type DoorTxFee<T: Config> = StorageValue<_, u64, ValueQuery, DefaultDoorTxFee>;

	#[pallet::storage]
	#[pallet::getter(fn door_address)]
	/// The door address on XRPL
	pub type DoorAddress<T: Config> = StorageValue<_, XrplAccountId>;

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
			ensure!(
				Self::process_xrp_transaction_details(transaction_hash).is_none(),
				Error::<T>::TxReplay
			);

			Self::add_to_relay(relayer, ledger_index, transaction_hash, transaction, timestamp)
		}

		/// Submit xrp transaction challenge
		#[pallet::weight((<T as Config>::WeightInfo::submit_challenge(),
		DispatchClass::Operational))]
		#[transactional]
		pub fn submit_challenge(
			origin: OriginFor<T>,
			transaction_hash: XrplTxHash,
		) -> DispatchResult {
			// TODO(surangap): enable this again when the challenger is completed
			// let challenger = ensure_signed(origin)?;
			// ChallengeXRPTransactionList::<T>::insert(&transaction_hash, challenger);
			Ok(())
		}

		/// Withdraw xrp transaction
		#[pallet::weight((<T as Config>::WeightInfo::withdraw_xrp(), DispatchClass::Operational))]
		#[transactional]
		pub fn withdraw_xrp(
			origin: OriginFor<T>,
			amount: Balance,
			destination: XrplAccountId,
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

		/// Set XRPL door address managed by this pallet
		#[pallet::weight((<T as Config>::WeightInfo::set_xrpl_door_address(), DispatchClass::Operational))]
		#[transactional]
		pub fn set_door_address(
			origin: OriginFor<T>,
			door_address: XrplAccountId,
		) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;
			DoorAddress::<T>::put(door_address);
			Self::deposit_event(Event::<T>::DoorAddressSet(door_address));
			Ok(())
		}

		/// Set the door account ticket sequence params for the next allocation
		#[pallet::weight((<T as Config>::WeightInfo::set_ticket_sequence_next_allocation(), DispatchClass::Operational))]
		pub fn set_ticket_sequence_next_allocation(
			origin: OriginFor<T>,
			start_ticket_sequence: u32,
			ticket_bucket_size: u32,
		) -> DispatchResult {
			let relayer = ensure_signed(origin)?;
			let active_relayer = <Relayer<T>>::get(&relayer).unwrap_or(false);
			ensure!(active_relayer, Error::<T>::NotPermitted);

			let current_ticket_sequence = Self::door_ticket_sequence();
			let current_params = Self::door_ticket_sequence_params();

			if start_ticket_sequence < current_ticket_sequence ||
				start_ticket_sequence < current_params.start_sequence ||
				ticket_bucket_size == 0
			{
				fail!(Error::<T>::NextTicketSequenceParamsInvalid);
			}
			DoorTicketSequenceParamsNext::<T>::put(XrplTicketSequenceParams {
				start_sequence: start_ticket_sequence,
				bucket_size: ticket_bucket_size,
			});
			Self::deposit_event(Event::<T>::DoorNextTicketSequenceParamSet {
				ticket_sequence_start_next: start_ticket_sequence,
				ticket_bucket_size_next: ticket_bucket_size,
			});
			Ok(())
		}

		/// Set the door account current ticket sequence params for current allocation - force set
		#[pallet::weight((<T as Config>::WeightInfo::set_ticket_sequence_current_allocation(), DispatchClass::Operational))]
		pub fn set_ticket_sequence_current_allocation(
			origin: OriginFor<T>,
			ticket_sequence: u32,
			start_ticket_sequence: u32,
			ticket_bucket_size: u32,
		) -> DispatchResult {
			ensure_root(origin)?; // only the root will be able to do it
			let current_ticket_sequence = Self::door_ticket_sequence();
			let current_params = Self::door_ticket_sequence_params();

			if ticket_sequence < current_ticket_sequence ||
				start_ticket_sequence < current_params.start_sequence ||
				ticket_bucket_size == 0
			{
				fail!(Error::<T>::TicketSequenceParamsInvalid);
			}

			DoorTicketSequence::<T>::put(ticket_sequence);
			DoorTicketSequenceParams::<T>::put(XrplTicketSequenceParams {
				start_sequence: start_ticket_sequence,
				bucket_size: ticket_bucket_size,
			});
			TicketSequenceThresholdReachedEmitted::<T>::kill();
			Self::deposit_event(Event::<T>::DoorTicketSequenceParamSet {
				ticket_sequence,
				ticket_sequence_start: start_ticket_sequence,
				ticket_bucket_size,
			});
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
				let tx_details = <ProcessXRPTransactionDetails<T>>::get(transaction_hash);
				reads += 1;
				match tx_details {
					None => {},
					Some((ledger_index, ref tx, _relayer)) => {
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

	/// Prune settled transaction data from storage
	/// if it was scheduled to do so at block `n`
	pub fn clear_storages(n: T::BlockNumber) -> Weight {
		let mut reads: Weight = 0;
		let mut writes: Weight = 0;
		reads += 1;
		if <SettledXRPTransactionDetails<T>>::contains_key(n) {
			if let Some(tx_hashes) = <SettledXRPTransactionDetails<T>>::take(n) {
				writes += 1 + tx_hashes.len() as Weight;
				for tx_hash in tx_hashes {
					<ProcessXRPTransactionDetails<T>>::remove(tx_hash);
				}
			}
		}
		DbWeight::get().reads_writes(reads, writes)
	}

	pub fn add_to_relay(
		relayer: T::AccountId,
		ledger_index: LedgerIndex,
		transaction_hash: XrplTxHash,
		transaction: XrplTxData,
		timestamp: Timestamp,
	) -> DispatchResult {
		let val = XrpTransaction { transaction_hash, transaction, timestamp };
		<ProcessXRPTransactionDetails<T>>::insert(&transaction_hash, (ledger_index, val, relayer));

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
		destination: XrplAccountId,
	) -> DispatchResult {
		// TODO: need a fee oracle, this is over estimating the fee
		// https://github.com/futureversecom/seed/issues/107
		let tx_fee = Self::door_tx_fee();
		ensure!(!amount.is_zero(), Error::<T>::WithdrawInvalidAmount);
		ensure!(amount.checked_add(tx_fee as Balance).is_some(), Error::<T>::WithdrawInvalidAmount); // xrp amounts are `u64`
		let door_address = Self::door_address().ok_or(Error::<T>::DoorAddressNotSet)?;

		// the door address pays the tx fee on XRPL. Therefore the withdrawn amount must include the
		// tx fee to maintain an accurate door balance
		let _ =
			T::MultiCurrency::burn_from(T::XrpAssetId::get(), &who, amount + tx_fee as Balance)?;

		let ticket_sequence = Self::get_door_ticket_sequence()?;
		let tx_data = XrpWithdrawTransaction {
			tx_nonce: 0_u32, // Sequence = 0 when using TicketSequence
			tx_fee,
			amount,
			destination,
			tx_ticket_sequence: ticket_sequence,
		};

		let proof_id = Self::submit_withdraw_request(door_address.into(), tx_data)?;

		Self::deposit_event(Event::WithdrawRequest { proof_id, sender: who, amount, destination });

		Ok(())
	}

	/// Construct an XRPL payment transaction and submit for signing
	/// Returns a (proof_id, tx_blob)
	fn submit_withdraw_request(
		door_address: [u8; 20],
		tx_data: XrpWithdrawTransaction,
	) -> Result<u64, DispatchError> {
		let XrpWithdrawTransaction { tx_fee, tx_nonce, tx_ticket_sequence, amount, destination } =
			tx_data;

		let payment = Payment::new(
			door_address,
			destination.into(),
			amount.saturated_into(),
			tx_nonce,
			tx_ticket_sequence,
			tx_fee,
			// omit signer key since this is a 'MultiSigner' tx
			None,
		);
		let tx_blob = payment.binary_serialize(true);

		T::EthyAdapter::sign_xrpl_transaction(tx_blob.as_slice())
	}

	// Return the current door nonce and increment it in storage
	pub fn door_nonce_inc() -> Result<XrplTxNonce, DispatchError> {
		let nonce = Self::door_nonce();
		let next_nonce = nonce.checked_add(One::one()).ok_or(ArithmeticError::Overflow)?;
		DoorNonce::<T>::set(next_nonce);
		Ok(nonce)
	}

	// Return the current door ticket sequence and increment it in storage
	pub fn get_door_ticket_sequence() -> Result<XrplTxTicketSequence, DispatchError> {
		let mut current_sequence = Self::door_ticket_sequence();
		let ticket_params = Self::door_ticket_sequence_params();

		// check if TicketSequenceThreshold reached. notify by emitting
		// TicketSequenceThresholdReached
		if ticket_params.bucket_size != 0 &&
			Percent::from_rational(
				current_sequence - ticket_params.start_sequence + 1,
				ticket_params.bucket_size,
			) >= T::TicketSequenceThreshold::get() &&
			!Self::ticket_sequence_threshold_reached_emitted()
		{
			Self::deposit_event(Event::<T>::TicketSequenceThresholdReached(current_sequence));
			TicketSequenceThresholdReachedEmitted::<T>::put(true);
		}

		let mut next_sequence =
			current_sequence.checked_add(One::one()).ok_or(ArithmeticError::Overflow)?;
		let last_sequence = ticket_params
			.start_sequence
			.checked_add(ticket_params.bucket_size)
			.ok_or(ArithmeticError::Overflow)?;
		if current_sequence >= last_sequence {
			// we ran out current bucket, check the next_start_sequence
			let next_ticket_params = Self::door_ticket_sequence_params_next();
			if next_ticket_params == XrplTicketSequenceParams::default() ||
				next_ticket_params.start_sequence == ticket_params.start_sequence
			{
				return Err(Error::<T>::NextTicketSequenceParamsNotSet.into())
			} else {
				// update next to current and clear next
				DoorTicketSequenceParams::<T>::set(next_ticket_params.clone());
				current_sequence = next_ticket_params.start_sequence;
				next_sequence =
					current_sequence.checked_add(One::one()).ok_or(ArithmeticError::Overflow)?;

				DoorTicketSequenceParamsNext::<T>::kill();
				TicketSequenceThresholdReachedEmitted::<T>::kill();
			}
		}
		DoorTicketSequence::<T>::set(next_sequence);

		Ok(current_sequence)
	}
}

impl<T: Config> EthyToXrplBridgeAdapter<XrplAccountId> for Pallet<T> {
	fn submit_signer_list_set_request(
		signer_entries: Vec<(XrplAccountId, u16)>,
	) -> Result<EventProofId, DispatchError> {
		let door_address = Self::door_address().ok_or(Error::<T>::DoorAddressNotSet)?;
		// TODO: need a fee oracle, this is over estimating the fee
		// https://github.com/futureversecom/seed/issues/107
		let tx_fee = Self::door_tx_fee();
		let ticket_sequence = Self::get_door_ticket_sequence()?;
		let signer_quorum: u32 = T::EthyAdapter::xrp_validators().len().saturating_sub(1) as u32;
		let signer_entries = signer_entries
			.into_iter()
			.map(|(account, weight)| (account.into(), weight))
			.collect();

		let signer_list_set = SignerListSet::new(
			door_address.into(),
			tx_fee,
			0_u32,
			ticket_sequence,
			signer_quorum,
			signer_entries,
			// omit signer key since this is a 'MultiSigner' tx
			None,
		);
		let tx_blob = signer_list_set.binary_serialize(true);

		T::EthyAdapter::sign_xrpl_transaction(tx_blob.as_slice())
	}
}
