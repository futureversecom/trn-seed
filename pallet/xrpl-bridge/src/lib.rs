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

use crate::types::{
	AssetWithdrawTransaction, DelayedPaymentId, DelayedWithdrawal, WithdrawTransaction,
	XRPLCurrency, XRPLCurrencyType, XRPLDoorAccount, XrpTransaction, XrpWithdrawTransaction,
	XrplTicketSequenceParams, XrplTxData,
};
use frame_support::{
	fail,
	pallet_prelude::*,
	traits::{
		fungibles::{metadata::Inspect as InspectMetadata, Inspect, Mutate},
		tokens::{Fortitude, Precision, Preservation},
		UnixTime,
	},
	transactional,
	weights::constants::RocksDbWeight as DbWeight,
	PalletId,
};
use frame_system::pallet_prelude::*;
use seed_pallet_common::{CreateExt, EthyToXrplBridgeAdapter, XrplBridgeToEthyAdapter};
use seed_primitives::{
	ethy::{crypto::AuthorityId, EventProofId},
	xrpl::{LedgerIndex, XrplAccountId, XrplTxHash, XrplTxTicketSequence},
	AssetId, Balance, Timestamp,
};
use sp_runtime::{
	traits::{AccountIdConversion, One, Zero},
	ArithmeticError, Percent, SaturatedConversion, Saturating,
};
use sp_std::{prelude::*, vec};
use xrpl_codec::{
	field::Amount,
	traits::BinarySerialize,
	transaction::{Payment, PaymentAltCurrency, PaymentWithDestinationTag, SignerListSet},
	types::{AccountIdType, AmountType, IssuedAmountType, IssuedValueType},
};

pub use pallet::*;

pub mod types;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_relayer;

pub mod weights;
pub use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(5);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type EthyAdapter: XrplBridgeToEthyAdapter<AuthorityId>;

		type MultiCurrency: CreateExt<AccountId = Self::AccountId>
			+ Inspect<Self::AccountId, AssetId = AssetId>
			+ Mutate<Self::AccountId, Balance = Balance>
			+ InspectMetadata<Self::AccountId>;

		/// Allowed origins to add/remove the relayers
		type ApproveOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Weight information
		type WeightInfo: WeightInfo;

		/// XRP Asset Id set at runtime
		#[pallet::constant]
		type XrpAssetId: Get<AssetId>;

		/// ROOT AssetId set at runtime
		#[pallet::constant]
		type NativeAssetId: Get<AssetId>;

		/// This pallet's Id, used for deriving a sovereign account ID
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Challenge Period to wait for a challenge before processing the transaction
		#[pallet::constant]
		type ChallengePeriod: Get<u32>;

		/// Maximum number of transactions that can be pruned in on_idle
		#[pallet::constant]
		type MaxPrunedTransactionsPerBlock: Get<u32>;

		/// Maximum number of delayed transactions that can be processed in a single block
		#[pallet::constant]
		type MaxDelayedPaymentsPerBlock: Get<u32>;

		/// Upper limit to the number of blocks we can check per block for delayed payments
		#[pallet::constant]
		type DelayedPaymentBlockLimit: Get<BlockNumberFor<Self>>;

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
		/// This asset is not supported by the bridge
		AssetNotSupported,
		/// Only the active relayer is permitted to perform this action
		NotPermitted,
		/// The paymentIds have been exhausted
		NoAvailablePaymentIds,
		/// The scheduled block cannot hold any more delayed payments
		DelayScheduleAtCapacity,
		/// There is no settledXRPTransactionDetails for this ledger index
		NoTransactionDetails,
		RelayerDoesNotExists,
		/// Withdraw amount must be non-zero and <= u64
		WithdrawInvalidAmount,
		/// The door address has not been configured
		DoorAddressNotSet,
		/// XRPL does not allow more than 8 signers for door address
		TooManySigners,
		/// The asset decimals are too high to bridge to XRPL
		InvalidAssetDecimals,
		/// The issued amount currency code is invalid
		InvalidCurrencyCode,
		/// Could not convert Balance to Mantissa Exponent
		InvalidMantissaExponentConversion,
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
		/// XRPL symbol to TRN asset id mapping is invalid
		InvalidSymbolMapping,
		/// The asset rounding due to saturation is too high, reduce the significant digits
		AssetRoundingTooHigh,

		TestErrorRemoveAfterUsing,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		TransactionAdded(LedgerIndex, XrplTxHash),
		TransactionChallenge(LedgerIndex, XrplTxHash),
		/// The payment delay was set
		PaymentDelaySet {
			asset_id: AssetId,
			payment_threshold: Balance,
			delay: BlockNumberFor<T>,
		},
		/// The payment delay was removed
		PaymentDelayRemoved {
			asset_id: AssetId,
		},
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
			asset_id: AssetId,
			amount: Balance,
			destination: XrplAccountId,
		},
		/// A withdrawal was delayed as it was above the min_payment threshold
		WithdrawDelayed {
			sender: T::AccountId,
			asset_id: AssetId,
			amount: Balance,
			destination: XrplAccountId,
			delayed_payment_id: DelayedPaymentId,
			payment_block: BlockNumberFor<T>,
		},
		RelayerAdded(T::AccountId),
		RelayerRemoved(T::AccountId),
		/// XRPL Door address set/reset
		DoorAddressSet {
			door_account: XRPLDoorAccount,
			address: Option<XrplAccountId>,
		},
		/// Next ticket sequence params set for the XRPL door account
		DoorNextTicketSequenceParamSet {
			door_account: XRPLDoorAccount,
			ticket_sequence_start_next: u32,
			ticket_bucket_size_next: u32,
		},
		/// ticket sequence params set for the XRPL door account
		DoorTicketSequenceParamSet {
			door_account: XRPLDoorAccount,
			ticket_sequence: u32,
			ticket_sequence_start: u32,
			ticket_bucket_size: u32,
		},
		LedgerIndexManualPrune {
			ledger_index: u32,
			total_cleared: u32,
		},
		/// ticket sequence threshold reached for the XRPL door account
		TicketSequenceThresholdReached {
			door_account: XRPLDoorAccount,
			current_ticket: u32,
		},
		XrplAssetMapSet {
			asset_id: AssetId,
			xrpl_currency: XRPLCurrency,
		},
		XrplAssetMapRemoved {
			asset_id: AssetId,
			xrpl_currency: XRPLCurrency,
		},
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<sp_core::H160>,
	{
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			Self::process_xrp_tx(n)
		}

		fn on_idle(now: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			let delay_weight = Self::process_delayed_payments(now, remaining_weight);
			let prune_weight = Self::clear_storages(remaining_weight.saturating_sub(delay_weight));
			delay_weight.saturating_add(prune_weight)
		}
	}

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::storage]
	#[pallet::getter(fn get_relayer)]
	/// List of all XRP transaction relayers
	pub type Relayer<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, bool>;

	#[pallet::storage]
	#[pallet::getter(fn process_xrp_transaction)]
	/// Temporary storage to set the transactions ready to be processed at specified block number
	pub type ProcessXRPTransaction<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<XrplTxHash, T::XRPTransactionLimit>,
	>;

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
	/// Map TRN asset Id to XRPL symbol, storage to keep mapping between TRN -> XRPL tokens/assets
	pub type AssetIdToXRPL<T: Config> = StorageMap<_, Twox64Concat, AssetId, XRPLCurrency>;

	#[pallet::storage]
	/// Map XRPL symbol to TRN asset Id, storage to keep mapping between XRPL -> TRN tokens/assets
	pub type XRPLToAssetId<T: Config> = StorageMap<_, Twox64Concat, XRPLCurrency, AssetId>;

	#[pallet::storage]
	/// Highest settled XRPL ledger index
	pub type HighestSettledLedgerIndex<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	/// Source tag to be used to indicate the transaction is happening from futureverse
	pub type SourceTag<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::type_value]
	pub fn DefaultHighestPrunedLedgerIndex() -> u32 {
		42_900_000_u32
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
	/// Payment delay for any withdraw over the specified Balance threshold
	pub type PaymentDelay<T: Config> =
		StorageMap<_, Twox64Concat, AssetId, (Balance, BlockNumberFor<T>)>;

	#[pallet::storage]
	/// Map from DelayedPaymentId to (sender, WithdrawTx)
	pub type DelayedPayments<T: Config> =
		StorageMap<_, Identity, DelayedPaymentId, DelayedWithdrawal<T::AccountId>>;

	#[pallet::storage]
	/// Map from block number to DelayedPaymentIds scheduled for that block
	pub type DelayedPaymentSchedule<T: Config> = StorageMap<
		_,
		Identity,
		BlockNumberFor<T>,
		BoundedVec<DelayedPaymentId, T::MaxDelayedPaymentsPerBlock>,
	>;

	#[pallet::storage]
	/// The highest block number that has had all delayed payments processed
	pub type NextDelayProcessBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::storage]
	/// The next available delayedPaymentId
	pub type NextDelayedPaymentId<T: Config> = StorageValue<_, DelayedPaymentId, ValueQuery>;

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
	/// The current ticket sequence of the XRPL door accounts
	pub type DoorTicketSequence<T: Config> = StorageMap<
		_,
		Twox64Concat,
		XRPLDoorAccount,
		XrplTxTicketSequence,
		ValueQuery,
		DefaultDoorTicketSequence,
	>;

	#[pallet::storage]
	#[pallet::getter(fn door_ticket_sequence_params)]
	/// The Ticket sequence params of the XRPL door accounts for the current allocation
	pub type DoorTicketSequenceParams<T: Config> =
		StorageMap<_, Twox64Concat, XRPLDoorAccount, XrplTicketSequenceParams, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn door_ticket_sequence_params_next)]
	/// The Ticket sequence params of the XRPL door accounts for the next allocation
	pub type DoorTicketSequenceParamsNext<T: Config> =
		StorageMap<_, Twox64Concat, XRPLDoorAccount, XrplTicketSequenceParams, ValueQuery>;

	#[pallet::type_value]
	pub fn DefaultTicketSequenceThresholdReachedEmitted() -> bool {
		false
	}
	#[pallet::storage]
	#[pallet::getter(fn ticket_sequence_threshold_reached_emitted)]
	/// Keeps track whether the TicketSequenceThresholdReached event is emitted for XRPL door accounts
	pub type TicketSequenceThresholdReachedEmitted<T: Config> = StorageMap<
		_,
		Twox64Concat,
		XRPLDoorAccount,
		bool,
		ValueQuery,
		DefaultTicketSequenceThresholdReachedEmitted,
	>;

	/// Default door tx fee 1 XRP
	#[pallet::type_value]
	pub fn DefaultDoorTxFee() -> u64 {
		1_000_000_u64
	}

	#[pallet::storage]
	#[pallet::getter(fn door_tx_fee)]
	/// The flat fee for XRPL door txs
	pub type DoorTxFee<T: Config> =
		StorageMap<_, Twox64Concat, XRPLDoorAccount, u64, ValueQuery, DefaultDoorTxFee>;

	#[pallet::storage]
	#[pallet::getter(fn door_address)]
	/// The door address on XRPL
	pub type DoorAddress<T: Config> = StorageMap<_, Twox64Concat, XRPLDoorAccount, XrplAccountId>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub xrp_relayers: Vec<T::AccountId>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { xrp_relayers: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::initialize_relayer(&self.xrp_relayers);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit xrp transaction
		#[pallet::call_index(0)]
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

			// Verify that the issuer supplied matches the issuer we have in our map,
			if let XrplTxData::CurrencyPayment { currency, .. } = &transaction {
				ensure!(XRPLToAssetId::<T>::contains_key(currency), Error::<T>::AssetNotSupported);
			}

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
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::submit_challenge())]
		#[transactional]
		pub fn submit_challenge(
			origin: OriginFor<T>,
			transaction_hash: XrplTxHash,
		) -> DispatchResult {
			let challenger = ensure_signed(origin)?;
			ChallengeXRPTransactionList::<T>::insert(&transaction_hash, challenger);
			Ok(())
		}

		/// Sets the payment delay
		/// payment_delay is a tuple of payment_threshold and delay in blocks
		#[pallet::call_index(2)]
		#[pallet::weight((T::WeightInfo::set_payment_delay(), DispatchClass::Operational))]
		pub fn set_payment_delay(
			origin: OriginFor<T>,
			asset_id: AssetId,
			payment_delay: Option<(Balance, BlockNumberFor<T>)>,
		) -> DispatchResult {
			ensure_root(origin)?;
			match payment_delay {
				Some((payment_threshold, delay)) => {
					PaymentDelay::<T>::insert(asset_id, (payment_threshold, delay));
					Self::deposit_event(Event::<T>::PaymentDelaySet {
						asset_id,
						payment_threshold,
						delay,
					});
				},
				None => {
					PaymentDelay::<T>::remove(asset_id);
					Self::deposit_event(Event::<T>::PaymentDelayRemoved { asset_id });
				},
			}
			Ok(())
		}

		/// Withdraw xrp transaction
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::withdraw_xrp())]
		#[transactional]
		pub fn withdraw_xrp(
			origin: OriginFor<T>,
			amount: Balance,
			destination: XrplAccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::add_to_withdraw(who, T::XrpAssetId::get(), amount, destination, None)
		}

		/// Withdraw xrp transaction
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::withdraw_xrp())]
		#[transactional]
		pub fn withdraw_xrp_with_destination_tag(
			origin: OriginFor<T>,
			amount: Balance,
			destination: XrplAccountId,
			destination_tag: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// println!("Decimals: {}", <T as pallet::Config>::MultiCurrency::decimals(asset_id));
			Self::add_to_withdraw(
				who,
				T::XrpAssetId::get(),
				amount,
				destination,
				Some(destination_tag),
			)
		}

		/// Withdraw any token to XRPL
		#[pallet::call_index(15)]
		#[pallet::weight(
			match asset_id {
				a if a == &T::XrpAssetId::get() => T::WeightInfo::withdraw_xrp(),
				_ => T::WeightInfo::withdraw_asset(),
			}
		)]
		#[transactional]
		pub fn withdraw(
			origin: OriginFor<T>,
			asset_id: AssetId,
			amount: Balance,
			destination: XrplAccountId,
			destination_tag: Option<u32>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::add_to_withdraw(who, asset_id, amount, destination, destination_tag)
		}

		/// add a relayer
		#[pallet::call_index(5)]
		#[pallet::weight((T::WeightInfo::add_relayer(), DispatchClass::Operational))]
		#[transactional]
		pub fn add_relayer(origin: OriginFor<T>, relayer: T::AccountId) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;
			Self::initialize_relayer(&vec![relayer.clone()]);
			Self::deposit_event(Event::<T>::RelayerAdded(relayer));
			Ok(())
		}

		/// remove a relayer
		#[pallet::call_index(6)]
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
		#[pallet::call_index(7)]
		#[pallet::weight((<T as Config>::WeightInfo::set_door_tx_fee(), DispatchClass::Operational))]
		pub fn set_door_tx_fee(
			origin: OriginFor<T>,
			door_type: XRPLDoorAccount,
			fee: u64,
		) -> DispatchResult {
			ensure_root(origin)?;
			DoorTxFee::<T>::set(door_type, fee);
			Ok(())
		}

		/// Set the xrp source tag
		#[pallet::call_index(8)]
		#[pallet::weight((<T as Config>::WeightInfo::set_xrp_source_tag(), DispatchClass::Operational))]
		pub fn set_xrp_source_tag(origin: OriginFor<T>, source_tag: u32) -> DispatchResult {
			ensure_root(origin)?;
			SourceTag::<T>::put(source_tag);
			Ok(())
		}

		/// Set/Reset XRPL door addresses managed by this pallet
		#[pallet::call_index(9)]
		#[pallet::weight((T::WeightInfo::set_door_address(), DispatchClass::Operational))]
		#[transactional]
		pub fn set_door_address(
			origin: OriginFor<T>,
			door_account: XRPLDoorAccount,
			door_address: Option<XrplAccountId>,
		) -> DispatchResult {
			T::ApproveOrigin::ensure_origin(origin)?;
			match door_address {
				Some(door_address) => {
					DoorAddress::<T>::insert(door_account, door_address);
				},
				None => {
					DoorAddress::<T>::remove(door_account);
				},
			}

			Self::deposit_event(Event::<T>::DoorAddressSet { door_account, address: door_address });
			Ok(())
		}

		/// Set the door account ticket sequence params for the next allocation
		#[pallet::call_index(10)]
		#[pallet::weight((T::WeightInfo::set_ticket_sequence_next_allocation(), DispatchClass::Operational))]
		pub fn set_ticket_sequence_next_allocation(
			origin: OriginFor<T>,
			door_account: XRPLDoorAccount,
			start_ticket_sequence: u32,
			ticket_bucket_size: u32,
		) -> DispatchResult {
			let relayer = ensure_signed(origin)?;
			let active_relayer = <Relayer<T>>::get(&relayer).unwrap_or(false);
			ensure!(active_relayer, Error::<T>::NotPermitted);

			let current_ticket_sequence = Self::door_ticket_sequence(door_account);
			let current_params = Self::door_ticket_sequence_params(door_account);

			if start_ticket_sequence < current_ticket_sequence
				|| start_ticket_sequence < current_params.start_sequence
				|| ticket_bucket_size == 0
			{
				fail!(Error::<T>::NextTicketSequenceParamsInvalid);
			}
			DoorTicketSequenceParamsNext::<T>::set(
				door_account,
				XrplTicketSequenceParams {
					start_sequence: start_ticket_sequence,
					bucket_size: ticket_bucket_size,
				},
			);
			Self::deposit_event(Event::<T>::DoorNextTicketSequenceParamSet {
				door_account,
				ticket_sequence_start_next: start_ticket_sequence,
				ticket_bucket_size_next: ticket_bucket_size,
			});
			Ok(())
		}

		/// Set the door account current ticket sequence params for current allocation - force set
		#[pallet::call_index(11)]
		#[pallet::weight((T::WeightInfo::set_ticket_sequence_current_allocation(), DispatchClass::Operational))]
		pub fn set_ticket_sequence_current_allocation(
			origin: OriginFor<T>,
			door_account: XRPLDoorAccount,
			ticket_sequence: u32,
			start_ticket_sequence: u32,
			ticket_bucket_size: u32,
		) -> DispatchResult {
			ensure_root(origin)?; // only the root will be able to do it
			let current_ticket_sequence = Self::door_ticket_sequence(door_account);
			let current_params = Self::door_ticket_sequence_params(door_account);

			if ticket_sequence < current_ticket_sequence
				|| start_ticket_sequence < current_params.start_sequence
				|| ticket_bucket_size == 0
			{
				fail!(Error::<T>::TicketSequenceParamsInvalid);
			}

			DoorTicketSequence::<T>::set(door_account, ticket_sequence);
			DoorTicketSequenceParams::<T>::set(
				door_account,
				XrplTicketSequenceParams {
					start_sequence: start_ticket_sequence,
					bucket_size: ticket_bucket_size,
				},
			);
			TicketSequenceThresholdReachedEmitted::<T>::remove(door_account);
			Self::deposit_event(Event::<T>::DoorTicketSequenceParamSet {
				door_account,
				ticket_sequence,
				ticket_sequence_start: start_ticket_sequence,
				ticket_bucket_size,
			});
			Ok(())
		}

		#[pallet::call_index(12)]
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
					highest_pruned_ledger_index
						<= highest_settled_ledger_index.saturating_sub(submission_window_width),
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

		#[pallet::call_index(13)]
		#[pallet::weight({
			let ledger_count = SettledXRPTransactionDetails::<T>::get(ledger_index).unwrap_or_default().len() as u32;
			(T::WeightInfo::prune_settled_ledger_index(ledger_count), DispatchClass::Operational)
		})]
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

		#[pallet::call_index(14)]
		#[pallet::weight(T::WeightInfo::set_xrpl_asset_map())]
		/// Sets the mapping for an asset to an xrpl symbol (requires governance)
		/// Sets both XRPLToAssetId and AssetIdToXRPL
		pub fn set_xrpl_asset_map(
			origin: OriginFor<T>,
			asset_id: AssetId,
			xrpl_currency: Option<XRPLCurrency>,
		) -> DispatchResult {
			ensure_root(origin)?;
			if let Some(xrpl_currency) = xrpl_currency {
				// Validate currency to prevent errors during withdrawal
				ensure!(xrpl_currency.symbol.is_valid(), Error::<T>::InvalidCurrencyCode);
				<AssetIdToXRPL<T>>::insert(asset_id, xrpl_currency);
				<XRPLToAssetId<T>>::insert(xrpl_currency, asset_id);
				Self::deposit_event(Event::XrplAssetMapSet { asset_id, xrpl_currency });
			} else {
				// Remove mapping if xrpl_currency is none
				let xrpl_currency_mapped =
					AssetIdToXRPL::<T>::get(asset_id).ok_or(Error::<T>::AssetNotSupported)?;
				<AssetIdToXRPL<T>>::remove(asset_id.clone());
				<XRPLToAssetId<T>>::remove(xrpl_currency_mapped);
				Self::deposit_event(Event::XrplAssetMapRemoved {
					asset_id: asset_id.clone(),
					xrpl_currency: xrpl_currency_mapped,
				});
			}
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

	pub fn process_xrp_tx(n: BlockNumberFor<T>) -> Weight
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
			match &tx.transaction {
				XrplTxData::Payment { amount, address } => {
					if let Err(e) = T::MultiCurrency::mint_into(
						T::XrpAssetId::get(),
						&(*address).into(),
						amount.clone(),
					) {
						Self::deposit_event(Event::ProcessingFailed(
							ledger_index,
							transaction_hash.clone(),
							e,
						));
					} else {
						Self::deposit_event(Event::ProcessingOk(
							ledger_index,
							transaction_hash.clone(),
						));
					}
				},
				XrplTxData::CurrencyPayment { amount, address, currency } => {
					reads += 1;
					let asset_id = match XRPLToAssetId::<T>::get(currency) {
						None => {
							Self::deposit_event(Event::ProcessingFailed(
								ledger_index,
								transaction_hash.clone(),
								Error::<T>::AssetNotSupported.into(),
							));
							continue;
						},
						Some(asset_id) => asset_id,
					};
					if asset_id == T::NativeAssetId::get() {
						let pallet_address: T::AccountId =
							T::PalletId::get().into_account_truncating();
						if let Err(e) = T::MultiCurrency::transfer(
							asset_id,
							&pallet_address,
							&(*address).into(),
							amount.clone(),
							Preservation::Expendable,
						) {
							Self::deposit_event(Event::ProcessingFailed(
								ledger_index,
								transaction_hash.clone(),
								e,
							));
							continue;
						}
					} else {
						if let Err(e) = T::MultiCurrency::mint_into(
							asset_id,
							&(*address).into(),
							amount.clone(),
						) {
							Self::deposit_event(Event::ProcessingFailed(
								ledger_index,
								transaction_hash.clone(),
								e,
							));
							continue;
						}
					}
					Self::deposit_event(Event::ProcessingOk(
						ledger_index,
						transaction_hash.clone(),
					));
				},
				_ => {
					Self::deposit_event(Event::NotSupportedTransaction);
					continue;
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
		}

		writes += 1;
		HighestSettledLedgerIndex::<T>::put(highest_settled_ledger_index);

		DbWeight::get().reads_writes(reads, writes)
	}

	/// Process any transactions that have been delayed due to the min_payment threshold
	pub fn process_delayed_payments(
		block_number: BlockNumberFor<T>,
		remaining_weight: Weight,
	) -> Weight {
		// Initial reads for the following:
		// Read: NextDelayProcessBlock, DoorAddress
		// Write: NextDelayProcessBlock
		let base_process_weight = DbWeight::get().reads_writes(2u64, 1u64);
		// Weight to process one withdraw tx
		// 1 read for DelayedPayments
		// 2 reads and 2 writes within submit_withdraw_request
		let weight_per_tx = DbWeight::get().reads_writes(3u64, 2u64);
		// The minimum weight required to clear at least one transaction.
		// This includes the weight_per_tx (To submit one withdrawal)
		// And the weight to update DelayedPaymentSchedule
		let min_weight_per_tx =
			weight_per_tx.saturating_add(DbWeight::get().reads_writes(2u64, 2u64));

		// Ensure we have enough weight to perform the initial reads + process at least one tx
		if remaining_weight.ref_time() <= (base_process_weight + min_weight_per_tx).ref_time() {
			return Weight::zero();
		}

		let mut used_weight = base_process_weight;
		let highest_processed_delay_block = <NextDelayProcessBlock<T>>::get();
		let mut new_highest = highest_processed_delay_block;
		// Limit the number of blocks to process to either the current block_number or the
		// DelayedPaymentBlockLimit + highest_processed_delay_block
		let block_limit = block_number
			.min(highest_processed_delay_block.saturating_add(T::DelayedPaymentBlockLimit::get()));

		// Get the current door address
		let Some(door_address) = DoorAddress::<T>::get(XRPLDoorAccount::Main) else {
			return used_weight;
		};

		// Loop through as many blocks as we can, checking each block to see if there are any
		// delayed payments to process
		while new_highest <= block_limit {
			// Check if we have enough remaining to mutate storage this block
			if remaining_weight.ref_time()
				<= used_weight
					.saturating_add(DbWeight::get().reads_writes(1, 2))
					.saturating_add(weight_per_tx)
					.ref_time()
			{
				break;
			}

			// Add weight for reading DelayedPaymentSchedule
			used_weight = used_weight.saturating_add(DbWeight::get().reads(1));
			let Some(delayed_payment_ids) = <DelayedPaymentSchedule<T>>::get(new_highest) else {
				// No delayed payments to process for this block
				new_highest = new_highest.saturating_add(BlockNumberFor::<T>::one());
				continue;
			};
			// Add weight for writing DelayedPaymentSchedule
			used_weight = used_weight.saturating_add(DbWeight::get().writes(1));

			// Check how many delayed payments we are able to process

			let max_to_clear =
				remaining_weight.saturating_sub(used_weight).div(weight_per_tx.ref_time());
			let max_to_clear =
				max_to_clear.ref_time().min(delayed_payment_ids.len() as u64) as usize;

			for i in 0..max_to_clear {
				let payment_id = delayed_payment_ids[i];
				if let Some(delayed_withdrawal) = <DelayedPayments<T>>::take(payment_id) {
					let asset_id = match delayed_withdrawal.withdraw_tx {
						WithdrawTransaction::XRP(_) => T::XrpAssetId::get(),
						WithdrawTransaction::Asset(tx) => tx.asset_id,
					};
					let _ = Self::submit_withdraw_request(
						delayed_withdrawal.sender,
						door_address.into(),
						delayed_withdrawal.withdraw_tx,
						asset_id,
						delayed_withdrawal.destination_tag,
					);
				};
			}
			// Add weight for the tx's we processed
			used_weight =
				used_weight.saturating_add(weight_per_tx.saturating_mul(max_to_clear as u64));

			// If we have cleared all txs in this block, remove them.
			// Otherwise, reinsert the remaining txs
			if max_to_clear < delayed_payment_ids.len() {
				let remaining_payment_ids = delayed_payment_ids[max_to_clear..].to_vec();
				let remaining_payment_ids = BoundedVec::truncate_from(remaining_payment_ids);
				<DelayedPaymentSchedule<T>>::insert(new_highest, remaining_payment_ids);
				break;
			} else {
				<DelayedPaymentSchedule<T>>::remove(new_highest);
				new_highest = new_highest.saturating_add(BlockNumberFor::<T>::one());
			}
		}

		// Update NextDelayProcessBlock with the last block we cleared
		if new_highest > highest_processed_delay_block {
			<NextDelayProcessBlock<T>>::put(new_highest);
		} else {
			// We didn't update the highest block, so remove the recorded weight
			used_weight = used_weight.saturating_sub(DbWeight::get().writes(1u64));
		}

		used_weight
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
		if remaining_weight.ref_time() <= (base_pruning_weight + min_weight_per_index).ref_time() {
			return Weight::zero();
		}

		// Add the cost of the initial reads and read the data
		let mut used_weight = base_pruning_weight;
		let current_end =
			HighestSettledLedgerIndex::<T>::get().saturating_sub(SubmissionWindowWidth::<T>::get());

		// Get range of indexes to clear
		let highest_pruned_index = <HighestPrunedLedgerIndex<T>>::get();
		let mut new_highest = highest_pruned_index;

		// Ensure we don't clear more than the specified max per block.
		// If this check is not in place, the settled_txs_to_clear could become very large
		// and cause memory issues
		let max_end = highest_pruned_index + T::MaxPrunedTransactionsPerBlock::get();
		let current_end = current_end.min(max_end);
		let settled_txs_to_clear = (highest_pruned_index..current_end).collect::<Vec<u32>>();

		if settled_txs_to_clear.len() == 0 {
			return used_weight;
		}

		// Add the write cost for HighestPrunedLedgerIndex if we have txs to clear
		// If we don't update this storage value we will remove this write later.
		used_weight = used_weight.saturating_add(DbWeight::get().writes(1u64));

		for ledger_index in settled_txs_to_clear {
			// Check if we have enough remaining to mutate storage this index
			if remaining_weight.ref_time()
				<= used_weight.saturating_add(DbWeight::get().reads_writes(1, 2)).ref_time()
			{
				break;
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
				let remaining_tx_hashes = tx_hashes[max_to_clear..].to_vec();
				let remaining_tx_hashes = BoundedVec::truncate_from(remaining_tx_hashes);
				<SettledXRPTransactionDetails<T>>::insert(ledger_index, remaining_tx_hashes);
				break;
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

	/// `who` the account requesting the withdraw
	/// `amount` the amount of XRP drops to withdraw (- the tx fee)
	///  `destination` the receiver classic `AccountID` on XRPL
	#[transactional]
	pub fn add_to_withdraw(
		who: AccountOf<T>,
		asset_id: AssetId,
		amount: Balance,
		destination: XrplAccountId,
		destination_tag: Option<u32>,
	) -> DispatchResult {
		// TODO: need a fee oracle, this is over estimating the fee
		// https://github.com/futureversecom/seed/issues/107
		ensure!(!amount.is_zero(), Error::<T>::WithdrawInvalidAmount);
		// Saturate the balance to be within the Mantissa range if the asset is not XRP
		let amount = Self::saturate_balance(amount, asset_id)?;
		let tx_fee = Self::door_tx_fee(XRPLDoorAccount::Main);
		let door_address =
			Self::door_address(XRPLDoorAccount::Main).ok_or(Error::<T>::DoorAddressNotSet)?;

		let tx_data = match asset_id {
			a if a == T::XrpAssetId::get() => {
				ensure!(
					amount.checked_add(tx_fee as Balance).is_some(),
					Error::<T>::WithdrawInvalidAmount
				); // xrp amounts are `u64`

				Self::process_xrp_withdrawal(destination, amount, tx_fee, who.clone())?
			},
			a if a == T::NativeAssetId::get() => {
				Self::process_root_withdrawal(destination, amount, tx_fee, who.clone())?
			},
			_ => {
				Self::process_asset_withdrawal(asset_id, destination, amount, tx_fee, who.clone())?
			},
		};

		// Check if there is a payment delay and delay the payment if necessary
		if let Some((payment_threshold, delay)) = PaymentDelay::<T>::get(asset_id) {
			if amount >= payment_threshold {
				Self::delay_payment(delay, who.clone(), asset_id, tx_data, destination_tag)?;
				return Ok(());
			}
		}

		Self::submit_withdraw_request(
			who,
			door_address.into(),
			tx_data,
			asset_id,
			destination_tag,
		)?;

		Ok(())
	}

	/// Handles burning of XRP tokens and creating the WithdrawTransaction to send to the bridge
	fn process_xrp_withdrawal(
		destination: XrplAccountId,
		amount: Balance,
		tx_fee: u64,
		who: AccountOf<T>,
	) -> Result<WithdrawTransaction, DispatchError> {
		// the door address pays the tx fee on XRPL. Therefore the withdrawn amount must include the
		// tx fee to maintain an accurate door balance
		// We burn on TRN as the tx_fee is burnt on XRPL side by the Door Address
		let _ = T::MultiCurrency::burn_from(
			T::XrpAssetId::get(),
			&who,
			amount + tx_fee as Balance,
			Precision::Exact,
			Fortitude::Polite,
		)?;

		let ticket_sequence = Self::get_door_ticket_sequence(XRPLDoorAccount::Main)?;
		Ok(WithdrawTransaction::XRP(XrpWithdrawTransaction {
			tx_nonce: 0_u32, // Sequence = 0 when using TicketSequence
			tx_fee,
			amount,
			destination,
			tx_ticket_sequence: ticket_sequence,
		}))
	}

	/// Handles transferring of Native tokens to the palletId and creating the WithdrawTransaction
	/// to send to the bridge
	/// We don't want to burn ROOT tokens as we need to maintain the total supply
	fn process_root_withdrawal(
		destination: XrplAccountId,
		amount: Balance,
		tx_fee: u64,
		who: AccountOf<T>,
	) -> Result<WithdrawTransaction, DispatchError> {
		let xrpl_currency = AssetIdToXRPL::<T>::get(T::NativeAssetId::get())
			.ok_or(Error::<T>::AssetNotSupported)?;

		// the door address pays the tx fee on XRPL. Therefore we must charge the user the tx fee
		// in XRP alongside their ROOT withdrawal
		// We burn on TRN as the tx_fee is burnt on XRPL side by the Door Address
		let _ = T::MultiCurrency::burn_from(
			T::XrpAssetId::get(),
			&who,
			tx_fee as Balance,
			Precision::Exact,
			Fortitude::Polite,
		)?;

		// Transfer ROOT tokens to the pallet address.
		let pallet_address: T::AccountId = T::PalletId::get().into_account_truncating();
		let _ = T::MultiCurrency::transfer(
			T::NativeAssetId::get(),
			&who,
			&pallet_address,
			amount,
			Preservation::Expendable,
		)?;

		let ticket_sequence = Self::get_door_ticket_sequence(XRPLDoorAccount::Main)?;
		Ok(WithdrawTransaction::Asset(AssetWithdrawTransaction {
			tx_nonce: 0_u32, // Sequence = 0 when using TicketSequence
			tx_fee,
			amount,
			destination,
			tx_ticket_sequence: ticket_sequence,
			asset_id: T::NativeAssetId::get(),
			currency: xrpl_currency.symbol,
			issuer: xrpl_currency.issuer,
		}))
	}

	/// Handles all other assets besides ROOT and XRP and creates the WithdrawTransaction to send to the bridge
	fn process_asset_withdrawal(
		asset_id: AssetId,
		destination: XrplAccountId,
		amount: Balance,
		tx_fee: u64,
		who: AccountOf<T>,
	) -> Result<WithdrawTransaction, DispatchError> {
		let xrpl_currency =
			AssetIdToXRPL::<T>::get(asset_id).ok_or(Error::<T>::AssetNotSupported)?;
		// the door address pays the tx fee on XRPL. Therefore we must charge the user the tx fee
		// in XRP alongside their asset withdrawal
		// We burn on TRN as the tx_fee is burnt on XRPL side by the Door Address
		let _ = T::MultiCurrency::burn_from(
			T::XrpAssetId::get(),
			&who,
			tx_fee as Balance,
			Precision::Exact,
			Fortitude::Polite,
		)?;

		// Burn their asset to send to the bridge
		let _ = T::MultiCurrency::burn_from(
			asset_id,
			&who,
			amount,
			Precision::Exact,
			Fortitude::Polite,
		)?;

		let ticket_sequence = Self::get_door_ticket_sequence(XRPLDoorAccount::Main)?;
		Ok(WithdrawTransaction::Asset(AssetWithdrawTransaction {
			tx_nonce: 0_u32, // Sequence = 0 when using TicketSequence
			tx_fee,
			amount,
			destination,
			tx_ticket_sequence: ticket_sequence,
			asset_id,
			currency: xrpl_currency.symbol,
			issuer: xrpl_currency.issuer,
		}))
	}

	/// Delay a withdrawal until a later block. Called if the withdrawal amount is over the
	/// PaymentDelay threshold
	fn delay_payment(
		delay: BlockNumberFor<T>,
		sender: T::AccountId,
		asset_id: AssetId,
		withdrawal: WithdrawTransaction,
		destination_tag: Option<u32>,
	) -> DispatchResult {
		// Get the next payment ID
		let delayed_payment_id = NextDelayedPaymentId::<T>::get();
		ensure!(
			delayed_payment_id.checked_add(One::one()).is_some(),
			Error::<T>::NoAvailablePaymentIds
		);

		let payment_block = <frame_system::Pallet<T>>::block_number().saturating_add(delay);
		DelayedPaymentSchedule::<T>::try_append(payment_block, delayed_payment_id)
			.map_err(|_| Error::<T>::DelayScheduleAtCapacity)?;
		DelayedPayments::<T>::insert(
			delayed_payment_id,
			DelayedWithdrawal {
				sender: sender.clone(),
				destination_tag,
				withdraw_tx: withdrawal.clone(),
			},
		);
		NextDelayedPaymentId::<T>::put(delayed_payment_id + 1);

		Self::deposit_event(Event::WithdrawDelayed {
			sender,
			asset_id,
			amount: withdrawal.amount(),
			destination: withdrawal.destination(),
			delayed_payment_id,
			payment_block,
		});
		return Ok(());
	}

	/// Construct an XRPL payment transaction and submit for signing
	/// Returns a (proof_id, tx_blob)
	fn submit_withdraw_request(
		sender: T::AccountId,
		door_address: [u8; 20],
		tx_data: WithdrawTransaction,
		asset_id: AssetId,
		destination_tag: Option<u32>,
	) -> DispatchResult {
		let amount = tx_data.amount();
		let destination = tx_data.destination();
		let tx_blob = match tx_data {
			WithdrawTransaction::XRP(tx) => {
				Self::serialize_xrp_tx(tx, door_address, destination_tag)
			},
			WithdrawTransaction::Asset(tx) => {
				Self::serialize_asset_tx(tx, door_address, destination_tag)?
			},
		};

		let proof_id = T::EthyAdapter::sign_xrpl_transaction(tx_blob.as_slice())?;
		Self::deposit_event(Event::WithdrawRequest {
			proof_id,
			sender,
			asset_id,
			amount,
			destination,
		});

		Ok(())
	}

	/// Serialise an XRP tx using the Payment type from the XRPL codec
	fn serialize_xrp_tx(
		tx_data: XrpWithdrawTransaction,
		door_address: [u8; 20],
		destination_tag: Option<u32>,
	) -> Vec<u8> {
		let XrpWithdrawTransaction { tx_fee, tx_nonce, tx_ticket_sequence, amount, destination } =
			tx_data;

		let tx_blob = if destination_tag.is_some() {
			let payment = PaymentWithDestinationTag::new(
				door_address,
				destination.into(),
				amount.saturated_into(),
				tx_nonce,
				tx_ticket_sequence,
				tx_fee,
				SourceTag::<T>::get(),
				destination_tag.unwrap(),
				// omit signer key since this is a 'MultiSigner' tx
				None,
			);
			payment.binary_serialize(true)
		} else {
			let payment = Payment::new(
				door_address,
				destination.into(),
				amount.saturated_into(),
				tx_nonce,
				tx_ticket_sequence,
				tx_fee,
				SourceTag::<T>::get(),
				// omit signer key since this is a 'MultiSigner' tx
				None,
			);
			payment.binary_serialize(true)
		};

		tx_blob
	}

	/// Serialise an asset tx using the CurrencyPayment type from the XRPL codec
	/// Called for any asset other than XRP
	/// This will convert the amount type into a mantissa exponent format which is
	/// the way balances are represented on XRPL
	fn serialize_asset_tx(
		tx_data: AssetWithdrawTransaction,
		door_address: [u8; 20],
		_destination_tag: Option<u32>,
	) -> Result<Vec<u8>, DispatchError> {
		let AssetWithdrawTransaction {
			tx_fee,
			tx_nonce,
			tx_ticket_sequence,
			amount,
			destination,
			currency,
			issuer,
			asset_id,
		} = tx_data;

		let issuer = AccountIdType(issuer.into());
		let decimals = T::MultiCurrency::decimals(asset_id);
		// Convert to mantissa and exponent
		let (mantissa, exponent) = Self::balance_to_mantissa_exponent(amount, decimals)?;
		// Create the necessary types from XRPL codec
		let value_type = IssuedValueType::from_mantissa_exponent(mantissa, exponent)
			.map_err(|_| Error::<T>::InvalidMantissaExponentConversion)?;
		let issued_amount =
			IssuedAmountType::from_issued_value(value_type, currency.into(), issuer)
				.map_err(|_| Error::<T>::InvalidCurrencyCode)?;
		let amount: Amount = Amount(AmountType::Issued(issued_amount));
		let payment = PaymentAltCurrency::new(
			door_address,
			destination.into(),
			amount,
			tx_nonce,
			tx_ticket_sequence,
			tx_fee,
			SourceTag::<T>::get(),
			// omit signer key since this is a 'MultiSigner' tx
			None,
		);
		Ok(payment.binary_serialize(true))
	}

	/// Saturate the balance so that we don't lose precision when we later convert it to
	/// mantissa and exponent
	/// If the asset is XRP, we don't need to do anything
	/// Will Error if the amount saturated is greater than 1.00 of the asset
	fn saturate_balance(amount: Balance, asset_id: AssetId) -> Result<Balance, DispatchError> {
		// XRP asset is stored as a u64, not as mantissa and exponent
		if asset_id == T::XrpAssetId::get() {
			return Ok(amount);
		}
		let decimals: i8 = T::MultiCurrency::decimals(asset_id)
			.try_into()
			.map_err(|_| Error::<T>::InvalidAssetDecimals)?;
		let (mantissa, exponent) = Self::balance_to_mantissa_exponent(amount, 0)?;
		let new_amount = mantissa as u128 * 10u128.pow(exponent as u32);
		// Return an error if we are saturating by more than 1.0 of the asset
		ensure!(
			(amount - new_amount) < 1 * 10u128.pow(decimals as u32),
			Error::<T>::AssetRoundingTooHigh
		);
		// Return mantissa and exponent back into Balance for our calculations
		Ok(new_amount)
	}

	/// Convert the balance to mantissa and exponent for sending to XRPL
	/// See: https://xrpl.org/docs/references/protocol/binary-format#token-amount-format
	fn balance_to_mantissa_exponent(
		amount: Balance,
		decimals: u8,
	) -> Result<(i64, i8), DispatchError> {
		const MANTISSA_MAX: u128 = 9_999_999_999_999_999;

		// Convert decimals to exponent and invert
		let mut exponent: i8 = decimals.try_into().map_err(|_| Error::<T>::InvalidAssetDecimals)?;
		exponent = -exponent;
		let mut mantissa: u128 = amount;

		// Scale down the u128 value to fit within i64 range
		while mantissa > MANTISSA_MAX {
			mantissa /= 10;
			exponent += 1;
		}

		// Convert to i64 (mantissa is now within i64 range)
		let mantissa = mantissa as i64;

		Ok((mantissa, exponent))
	}

	/// Return the current ticket sequence for the door account and increment it in storage
	pub fn get_door_ticket_sequence(
		door_account: XRPLDoorAccount,
	) -> Result<XrplTxTicketSequence, DispatchError> {
		let mut current_sequence = Self::door_ticket_sequence(door_account);
		let ticket_params = Self::door_ticket_sequence_params(door_account);

		// check if TicketSequenceThreshold reached. notify by emitting
		// TicketSequenceThresholdReached
		if ticket_params.bucket_size != 0
			&& Percent::from_rational(
				current_sequence - ticket_params.start_sequence + 1,
				ticket_params.bucket_size,
			) >= T::TicketSequenceThreshold::get()
			&& !Self::ticket_sequence_threshold_reached_emitted(door_account)
		{
			Self::deposit_event(Event::<T>::TicketSequenceThresholdReached {
				door_account,
				current_ticket: current_sequence,
			});
			TicketSequenceThresholdReachedEmitted::<T>::set(door_account, true);
		}

		let mut next_sequence =
			current_sequence.checked_add(One::one()).ok_or(ArithmeticError::Overflow)?;
		let last_sequence = ticket_params
			.start_sequence
			.checked_add(ticket_params.bucket_size)
			.ok_or(ArithmeticError::Overflow)?;
		if current_sequence >= last_sequence {
			// we ran out current bucket, check the next_start_sequence
			let next_ticket_params = Self::door_ticket_sequence_params_next(door_account);
			if next_ticket_params == XrplTicketSequenceParams::default()
				|| next_ticket_params.start_sequence == ticket_params.start_sequence
			{
				return Err(Error::<T>::NextTicketSequenceParamsNotSet.into());
			} else {
				// update next to current and clear next
				DoorTicketSequenceParams::<T>::set(door_account, next_ticket_params.clone());
				current_sequence = next_ticket_params.start_sequence;
				next_sequence =
					current_sequence.checked_add(One::one()).ok_or(ArithmeticError::Overflow)?;

				DoorTicketSequenceParamsNext::<T>::remove(door_account);
				TicketSequenceThresholdReachedEmitted::<T>::remove(door_account);
			}
		}
		DoorTicketSequence::<T>::set(door_account, next_sequence);

		Ok(current_sequence)
	}
}

impl<T: Config> EthyToXrplBridgeAdapter<XrplAccountId> for Pallet<T> {
	fn submit_signer_list_set_request(
		signer_entries: Vec<(XrplAccountId, u16)>,
	) -> Result<Vec<EventProofId>, DispatchError> {
		let mut results = vec![];
		let signer_quorum: u32 = signer_entries.len().saturating_sub(1) as u32;
		let signer_entries: Vec<([u8; 20], u16)> = signer_entries
			.into_iter()
			.map(|(account, weight)| (account.into(), weight))
			.collect();

		for door_account in XRPLDoorAccount::VALUES {
			let door_address =
				Self::door_address(door_account).ok_or(Error::<T>::DoorAddressNotSet)?;
			let ticket_sequence = Self::get_door_ticket_sequence(door_account)?;
			// TODO: need a fee oracle, this is over estimating the fee
			// https://github.com/futureversecom/seed/issues/107
			let tx_fee = Self::door_tx_fee(door_account);
			let signer_list_set = SignerListSet::new(
				door_address.into(),
				tx_fee,
				0_u32,
				ticket_sequence,
				signer_quorum,
				signer_entries.clone(),
				SourceTag::<T>::get(),
				// omit signer key since this is a 'MultiSigner' tx
				None,
			);
			let tx_blob = signer_list_set.binary_serialize(true);

			let proof_id = T::EthyAdapter::sign_xrpl_transaction(tx_blob.as_slice())?;
			results.push(proof_id);
		}
		Ok(results)
	}
}
