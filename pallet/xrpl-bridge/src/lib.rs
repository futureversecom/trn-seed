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
	xrpl::{LedgerIndex, XrplAccountId, XrplTxHash},
	AssetId, Balance, Timestamp,
};

use crate::helpers::{
	XrpTransaction, XrpWithdrawTransaction, XrplTicketSequenceParams, XrplTxData,
};

pub use pallet::*;
use seed_primitives::{ethy::EventProofId, xrpl::XrplTxTicketSequence};

mod helpers;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
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

	pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type EthyAdapter: XrplBridgeToEthyAdapter<AuthorityId>;

		type MultiCurrency: CreateExt<AccountId = Self::AccountId>
			+ Transfer<Self::AccountId, Balance = Balance>
			+ Inspect<Self::AccountId, AssetId = AssetId>
			+ Mutate<Self::AccountId>;

		/// Allowed origins to add/remove the relayers
		type ApproveOrigin: EnsureOrigin<Self::RuntimeOrigin>;

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

		/// Threshold to emit event TicketSequenceThresholdReached
		type TicketSequenceThreshold: Get<Percent>;

		/// Represents the maximum number of XRPL transactions that can be stored and processed in a
		/// single block in the temporary storage and the maximum number of XRPL transactions that
		/// can be stored in the settled transaction details storage for each block.
		type XRPTransactionLimit: Get<u32>;

		/// Maximum XRPL transactions within a single ledger
		type XRPLTransactionLimitPerLedger: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {
		NotPermitted,
		/// There is no settledXRPTransactionDetails for this ledger index
		NoTransactionDetails,
		RelayerDoesNotExists,
		/// Withdraw amount must be non-zero and <= u64
		WithdrawInvalidAmount,
		/// The door address has not been configured
		DoorAddressNotSet,
		/// XRPL does not allow more than 8 signers for door address
		TooManySigners,
		/// The signers are not known by ethy
		InvalidSigners,
		/// highest_pruned_ledger_index must be less than highest_settled_ledger_index -
		/// submission_window_width
		InvalidHighestPrunedIndex,
		/// Submitted a duplicate transaction hash
		TxReplay,
		/// The NextTicketSequenceParams has not been set
		NextTicketSequenceParamsNotSet,
		/// The NextTicketSequenceParams is invalid
		NextTicketSequenceParamsInvalid,
		/// The TicketSequenceParams is invalid
		TicketSequenceParamsInvalid,
		/// Cannot process more transactions at that block
		CannotProcessMoreTransactionsAtThatBlock,
		/// This ledger index is within the submission window and can't be pruned
		CannotPruneActiveLedgerIndex,
		/// Transaction submitted is outside the submission window
		OutSideSubmissionWindow,
		/// Too Many transactions per ledger
		TooManyTransactionsPerLedger,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		TransactionAdded(LedgerIndex, XrplTxHash),
		TransactionChallenge(LedgerIndex, XrplTxHash),
		/// Processing an event succeeded
		ProcessingOk(LedgerIndex, XrplTxHash),
		/// Processing an event failed
		ProcessingFailed(LedgerIndex, XrplTxHash, DispatchError),
		/// Transaction not supported
		NotSupportedTransaction,
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
		LedgerIndexManualPrune {
			ledger_index: u32,
			total_cleared: u32,
		},
		TicketSequenceThresholdReached(u32),
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<sp_core::H160>,
	{
		fn on_initialize(n: T::BlockNumber) -> Weight {
			Self::process_xrp_tx(n)
		}

		fn on_idle(_now: T::BlockNumber, remaining_weight: Weight) -> Weight {
			Self::clear_storages(remaining_weight)
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::storage]
	#[pallet::getter(fn get_relayer)]
	/// List of all XRP transaction relayers
	pub type Relayer<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, bool>;

	#[pallet::storage]
	#[pallet::getter(fn process_xrp_transaction)]
	/// Temporary storage to set the transactions ready to be processed at specified block number
	pub type ProcessXRPTransaction<T: Config> =
		StorageMap<_, Twox64Concat, T::BlockNumber, BoundedVec<XrplTxHash, T::XRPTransactionLimit>>;

	#[pallet::storage]
	#[pallet::getter(fn process_xrp_transaction_details)]
	/// Stores submitted transactions from XRPL waiting to be processed
	/// Transactions will be cleared according to the submission window after processing
	pub type ProcessXRPTransactionDetails<T: Config> =
		StorageMap<_, Identity, XrplTxHash, (LedgerIndex, XrpTransaction, T::AccountId)>;

	#[pallet::storage]
	#[pallet::getter(fn settled_xrp_transaction_details)]
	/// Settled xrp transactions stored against XRPL ledger index
	pub type SettledXRPTransactionDetails<T: Config> =
		StorageMap<_, Twox64Concat, u32, BoundedVec<XrplTxHash, T::XRPLTransactionLimitPerLedger>>;

	#[pallet::storage]
	/// Highest settled XRPL ledger index
	pub type HighestSettledLedgerIndex<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::type_value]
	pub fn DefaultHighestPrunedLedgerIndex() -> u32 {
		// TODO determine appropriate value
		8_267_856_u32
	}

	#[pallet::storage]
	/// Highest pruned XRPL ledger index
	pub type HighestPrunedLedgerIndex<T: Config> =
		StorageValue<_, u32, ValueQuery, DefaultHighestPrunedLedgerIndex>;

	#[pallet::type_value]
	/// XRPL ledger rate is between 3-5 seconds. let's take min 3 seconds and keep data for 10 days
	pub fn DefaultSubmissionWindowWidth() -> u32 {
		// 86400 is the number of seconds per day.
		86400_u32.saturating_div(3).saturating_mul(10)
	}
	#[pallet::storage]
	/// XRPL transactions submission window width in ledger indexes
	pub type SubmissionWindowWidth<T: Config> =
		StorageValue<_, u32, ValueQuery, DefaultSubmissionWindowWidth>;

	#[pallet::storage]
	#[pallet::getter(fn challenge_xrp_transaction_list)]
	/// Challenge received for a transaction mapped by hash, will be cleared when validator
	/// validates
	pub type ChallengeXRPTransactionList<T: Config> =
		StorageMap<_, Identity, XrplTxHash, T::AccountId>;

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
		#[pallet::weight((T::WeightInfo::submit_transaction(), DispatchClass::Operational))]
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
			// Check within the submission window
			let submission_window_end = HighestSettledLedgerIndex::<T>::get()
				.saturating_sub(SubmissionWindowWidth::<T>::get());
			ensure!(
				(ledger_index as u32).ge(&submission_window_end),
				Error::<T>::OutSideSubmissionWindow
			);
			// If within the submission window, check against ProcessXRPTransactionDetails
			ensure!(
				Self::process_xrp_transaction_details(transaction_hash).is_none(),
				Error::<T>::TxReplay
			);

			Self::add_to_relay(relayer, ledger_index, transaction_hash, transaction, timestamp)
		}

		/// Submit xrp transaction challenge
		#[pallet::weight((T::WeightInfo::submit_challenge(), DispatchClass::Operational))]
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
		#[pallet::weight((T::WeightInfo::withdraw_xrp(), DispatchClass::Operational))]
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
		#[pallet::weight((T::WeightInfo::add_relayer(), DispatchClass::Operational))]
		#[transactional]
		pub fn add_relayer(origin: OriginFor<T>, relayer: T::AccountId) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;
			Self::initialize_relayer(&vec![relayer.clone()]);
			Self::deposit_event(Event::<T>::RelayerAdded(relayer));
			Ok(())
		}

		/// remove a relayer
		#[pallet::weight((T::WeightInfo::remove_relayer(), DispatchClass::Operational))]
		#[transactional]
		pub fn remove_relayer(origin: OriginFor<T>, relayer: T::AccountId) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;
			if <Relayer<T>>::contains_key(relayer.clone()) {
				<Relayer<T>>::remove(relayer.clone());
				Self::deposit_event(Event::<T>::RelayerRemoved(relayer));
				Ok(())
			} else {
				Err(Error::<T>::RelayerDoesNotExists.into())
			}
		}

		/// Set the door tx fee amount
		#[pallet::weight((<T as Config>::WeightInfo::set_door_tx_fee(), DispatchClass::Operational))]
		pub fn set_door_tx_fee(origin: OriginFor<T>, fee: u64) -> DispatchResult {
			ensure_root(origin)?;
			DoorTxFee::<T>::set(fee);
			Ok(())
		}

		/// Set XRPL door address managed by this pallet
		#[pallet::weight((T::WeightInfo::set_door_address(), DispatchClass::Operational))]
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
		#[pallet::weight((T::WeightInfo::set_ticket_sequence_next_allocation(), DispatchClass::Operational))]
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
		#[pallet::weight((T::WeightInfo::set_ticket_sequence_current_allocation(), DispatchClass::Operational))]
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

		#[pallet::weight(T::WeightInfo::reset_settled_xrpl_tx_data(settled_tx_data.as_ref().unwrap_or(&vec![]).len() as u32))]
		#[transactional]
		pub fn reset_settled_xrpl_tx_data(
			origin: OriginFor<T>,
			highest_settled_ledger_index: u32,
			submission_window_width: u32,
			highest_pruned_ledger_index: Option<u32>,
			settled_tx_data: Option<Vec<(XrplTxHash, u32, XrpTransaction, T::AccountId)>>,
		) -> DispatchResult {
			ensure_root(origin)?;
			if let Some(highest_pruned_ledger_index) = highest_pruned_ledger_index {
				ensure!(
					highest_pruned_ledger_index <=
						highest_settled_ledger_index.saturating_sub(submission_window_width),
					Error::<T>::InvalidHighestPrunedIndex
				);
				HighestPrunedLedgerIndex::<T>::put(highest_pruned_ledger_index);
			}
			HighestSettledLedgerIndex::<T>::put(highest_settled_ledger_index);
			SubmissionWindowWidth::<T>::put(submission_window_width);

			if let Some(settled_txs) = settled_tx_data {
				for (xrpl_tx_hash, ledger_index, tx, account) in settled_txs {
					<ProcessXRPTransactionDetails<T>>::insert(
						xrpl_tx_hash,
						(ledger_index as LedgerIndex, tx, account),
					);

					<SettledXRPTransactionDetails<T>>::try_append(ledger_index, xrpl_tx_hash)
						.map_err(|_| Error::<T>::TooManyTransactionsPerLedger)?;
				}
			}

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn prune_settled_ledger_index(
			origin: OriginFor<T>,
			ledger_index: u32,
		) -> DispatchResult {
			ensure_root(origin)?;

			// Ensure the ledger index is not within the submission window
			let max_ledger_index = HighestSettledLedgerIndex::<T>::get()
				.saturating_sub(SubmissionWindowWidth::<T>::get());
			ensure!(ledger_index < max_ledger_index, Error::<T>::CannotPruneActiveLedgerIndex);

			// Clear the tx hashes for this ledger index
			let tx_hashes = <SettledXRPTransactionDetails<T>>::take(ledger_index)
				.ok_or(Error::<T>::NoTransactionDetails)?;
			let total_cleared = tx_hashes.len() as u32;

			for tx_hash in tx_hashes {
				<ProcessXRPTransactionDetails<T>>::remove(tx_hash);
			}

			Self::deposit_event(Event::LedgerIndexManualPrune { ledger_index, total_cleared });
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

	pub fn process_xrp_tx(n: T::BlockNumber) -> Weight
	where
		<T as frame_system::Config>::AccountId: From<sp_core::H160>,
	{
		let tx_items = match <ProcessXRPTransaction<T>>::take(n) {
			None => return DbWeight::get().reads(2u64),
			Some(v) => v,
		};
		let mut reads = 2u64;
		let mut writes = 0u64;

		let tx_details = tx_items
			.iter()
			.filter(|x| !<ChallengeXRPTransactionList<T>>::contains_key(x))
			.map(|x| (x, <ProcessXRPTransactionDetails<T>>::get(x)));

		reads += tx_items.len() as u64 * 2;
		let tx_details = tx_details.filter_map(|x| Some((x.0, x.1?)));

		reads += 1;
		let mut highest_settled_ledger_index = HighestSettledLedgerIndex::<T>::get();

		for (transaction_hash, (ledger_index, ref tx, _relayer)) in tx_details {
			match tx.transaction {
				XrplTxData::Payment { amount, address } => {
					if let Err(e) =
						T::MultiCurrency::mint_into(T::XrpAssetId::get(), &address.into(), amount)
					{
						Self::deposit_event(Event::ProcessingFailed(
							ledger_index,
							transaction_hash.clone(),
							e,
						));
					}
				},
				_ => {
					Self::deposit_event(Event::NotSupportedTransaction);
					continue
				},
			}

			// Add to SettledXRPTransactionDetails
			<SettledXRPTransactionDetails<T>>::try_append(
				ledger_index as u32,
				transaction_hash.clone(),
			)
			.expect("Should not happen since XRPLTransactionLimitPerLedger >= XRPTransactionLimit");

			// Update HighestSettledLedgerIndex
			if highest_settled_ledger_index < ledger_index as u32 {
				highest_settled_ledger_index = ledger_index as u32;
			}

			writes += 2;
			reads += 2;
			Self::deposit_event(Event::ProcessingOk(ledger_index, transaction_hash.clone()));
		}

		writes += 1;
		HighestSettledLedgerIndex::<T>::put(highest_settled_ledger_index);

		DbWeight::get().reads_writes(reads, writes)
	}

	/// Prune settled transaction data from storage
	pub fn clear_storages(remaining_weight: Weight) -> Weight {
		// Initial reads for the following:
		// Read: HighestSettledLedgerIndex, SubmissionWindowWidth, HighestPrunedLedgerIndex
		let base_pruning_weight = DbWeight::get().reads(3u64);
		// the weight per transaction is at least two writes
		// Reads: SettledXRPTransactionDetails
		// Writes: SettledXRPTransactionDetails, ProcessXRPTransactionDetails,
		// HighestPrunedLedgerIndex
		let min_weight_per_index = DbWeight::get().reads_writes(1, 3);

		// Ensure we have enough weight to perform the initial reads + at least one clear
		if remaining_weight.all_lte(base_pruning_weight + min_weight_per_index) {
			return Weight::zero()
		}

		// Add the cost of the initial reads and read the data
		let mut used_weight = base_pruning_weight;
		let current_end =
			HighestSettledLedgerIndex::<T>::get().saturating_sub(SubmissionWindowWidth::<T>::get());

		// Get range of indexes to clear
		let highest_pruned_index = <HighestPrunedLedgerIndex<T>>::get();
		let mut new_highest = highest_pruned_index;
		let settled_txs_to_clear = (highest_pruned_index..current_end).collect::<Vec<u32>>();

		if settled_txs_to_clear.len() == 0 {
			return used_weight
		}

		// Add the write cost for HighestPrunedLedgerIndex if we have txs to clear
		// If we don't update this storage value we will remove this write later.
		used_weight = used_weight.saturating_add(DbWeight::get().writes(1u64));

		for ledger_index in settled_txs_to_clear {
			// Check if we have enough remaining to mutate storage this index
			if remaining_weight
				.all_lte(used_weight.saturating_add(DbWeight::get().reads_writes(1, 2)))
			{
				break
			}

			// Add weight for reading SettledXRPTransactionDetails
			used_weight = used_weight.saturating_add(DbWeight::get().reads(1));
			let Some(tx_hashes) = <SettledXRPTransactionDetails<T>>::get(ledger_index) else {
				// No SettledXRPTransactionDetails for this index
				new_highest = new_highest.saturating_add(1);
				continue;
			};
			// Add weight for writing SettledXRPTransactionDetails
			used_weight = used_weight.saturating_add(DbWeight::get().writes(1));

			// Check how many tx_hashes we are able to clear with the remaining weight
			let weight_per_tx = DbWeight::get().writes(1u64);
			let max_to_clear =
				remaining_weight.saturating_sub(used_weight).div(weight_per_tx.ref_time());
			let max_to_clear = max_to_clear.ref_time().min(tx_hashes.len() as u64) as usize;

			// Remove as many tx_hashes as we can
			for i in 0..max_to_clear {
				let tx_hash = tx_hashes[i];
				<ProcessXRPTransactionDetails<T>>::remove(tx_hash);
			}
			// Add weight for the tx_hashes we cleared
			used_weight =
				used_weight.saturating_add(weight_per_tx.saturating_mul(max_to_clear as u64));

			// If we have tx_hashes left, reinsert them
			if max_to_clear < tx_hashes.len() {
				let tx_hashes_pruned = tx_hashes[max_to_clear..].to_vec();
				let tx_hashes_pruned = BoundedVec::truncate_from(tx_hashes_pruned);
				<SettledXRPTransactionDetails<T>>::insert(ledger_index, tx_hashes_pruned);
				break
			} else {
				new_highest = new_highest.saturating_add(1);
				<SettledXRPTransactionDetails<T>>::remove(ledger_index);
			}
		}

		// Update highest prunedLedgerIndex with the last ledger index we cleared
		if new_highest > highest_pruned_index {
			<HighestPrunedLedgerIndex<T>>::put(new_highest);
		} else {
			// We didn't actually update the highestPrunedLedgerIndex so remove the recorded weight
			used_weight = used_weight.saturating_sub(DbWeight::get().writes(1u64));
		}

		used_weight
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
		ProcessXRPTransaction::<T>::try_append(&process_block_number, transaction_hash)
			.map_err(|_| Error::<T>::CannotProcessMoreTransactionsAtThatBlock)?;

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
		let signer_quorum: u32 = signer_entries.len().saturating_sub(1) as u32;
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
