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

//! Ethy Pallet 🌉
//!
//! This pallet defines m-of-n protocols for validators to agree on values from a bridged
//! Ethereum chain (Ethereum JSON-RPC compliant), and conversely, generate proofs of events that
//! have occurred on the network
//!
//! The proofs are a collection of signatures which can be verified by a bridged contract on
//! Ethereum with awareness of the current validator set.
//!
//! There are 2 types of Ethereum values the bridge can verify:
//! 1) verify a transaction hash exists that executed a specific contract producing a specific event
//! log 2) verify the `returndata` of executing a contract at some time _t_ with input `i`
//!
//! Ethy validators use an offchain worker and Ethereum full node connections to independently
//! verify and observe events happened on Ethereum.
//! Once a threshold of validators sign a notarization having witnessed the event it is considered
//! verified.
//!
//! Events are opaque to this pallet, other pallets are forwarded incoming events and can submit
//! outgoing event for signing

#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

use ethabi::ParamType;
use frame_support::{
	pallet_prelude::*,
	traits::{
		fungibles::Mutate,
		schedule::{Anon, DispatchTime},
		UnixTime, ValidatorSet as ValidatorSetT,
	},
	transactional,
	weights::constants::RocksDbWeight as DbWeight,
	PalletId, Parameter,
};
use frame_system::{offchain::CreateSignedTransaction, pallet_prelude::*};
use hex_literal::hex;
use seed_pallet_common::{
	log, EthCallOracleSubscriber, EthereumEventRouter, EthyToXrplBridgeAdapter, EventRouterError,
	FinalSessionTracker as FinalSessionTrackerT, Hold,
};
use seed_primitives::{AssetId, Balance};
use sp_core::bounded::WeakBoundedVec;
use sp_runtime::{
	offchain as rt_offchain,
	traits::{MaybeSerializeDeserialize, Member},
	Percent, RuntimeAppPublic,
};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

mod ethereum_http_cli;
pub use ethereum_http_cli::EthereumRpcClient;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod impls;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod types;
pub use types::*;

pub mod weights;
pub use weights::WeightInfo;

/// Strategy for integrating staged EVM activities with Frontier storages.
pub trait FrontierLogMerge<T: pallet::Config> {
	fn on_initialize() -> Weight;
	fn on_finalize();
}

/// No-op strategy (use in tests or when Frontier integration is not desired).
pub struct NoFrontierMerge;
impl<T: pallet::Config> FrontierLogMerge<T> for NoFrontierMerge {
	fn on_initialize() -> Weight {
		Weight::zero()
	}
	fn on_finalize() {}
}

/// Frontier-enabled strategy. Only compiled when `frontier-logs` is enabled.
#[cfg(feature = "frontier-logs")]
pub struct FrontierMerge;
#[cfg(feature = "frontier-logs")]
impl<T> FrontierLogMerge<T> for FrontierMerge
where
	T: pallet::Config + pallet_ethereum::Config,
{
	fn on_initialize() -> Weight {
		// Clear any leftover staged EVM activities to avoid leaks.
		StagedEvmActivities::<T>::kill();
		DbWeight::get().writes(1)
	}
	fn on_finalize() {
		Pallet::<T>::on_finalize_frontier();
	}
}

/// The type to sign and send transactions.
const UNSIGNED_TXS_PRIORITY: u64 = 100;
/// Max notarization claims to attempt per block/OCW invocation
const CLAIMS_PER_BLOCK: usize = 1;
/// Max eth_call checks to attempt per block/OCW invocation
const CALLS_PER_BLOCK: usize = 1;

/// The logging target for this pallet
pub(crate) const LOG_TARGET: &str = "ethy";

/// The solidity selector of bridge events
/// i.e. output of `keccak256('SubmitEvent(address,address,bytes)')` /
/// `0f8885c9654c5901d61d2eae1fa5d11a67f9b8fca77146d5109bc7be00f4472a`
const SUBMIT_BRIDGE_EVENT_SELECTOR: [u8; 32] =
	hex!("0f8885c9654c5901d61d2eae1fa5d11a67f9b8fca77146d5109bc7be00f4472a");

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub xrp_door_signers: Vec<T::EthyId>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { xrp_door_signers: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			for new_signer in self.xrp_door_signers.iter() {
				XrplDoorSigners::<T>::insert(new_signer, true);
			}
			// set the NotaryXrplKeys as well
			let genesis_xrpl_keys = NotaryKeys::<T>::get()
				.into_iter()
				.filter(|validator| XrplDoorSigners::<T>::get(validator))
				.take(T::MaxXrplKeys::get().into())
				.collect::<Vec<_>>();
			NotaryXrplKeys::<T>::put(WeakBoundedVec::force_from(genesis_xrpl_keys, None));
		}
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
		/// Length of time the bridge will be paused while the authority set changes
		#[pallet::constant]
		type AuthorityChangeDelay: Get<BlockNumberFor<Self>>;
		/// Knows the active authority set (validator stash addresses)
		type AuthoritySet: ValidatorSetT<Self::AccountId, ValidatorId = Self::AccountId>;
		/// The pallet bridge address (destination for incoming messages, source for outgoing)
		#[pallet::constant]
		type BridgePalletId: Get<PalletId>;
		/// The runtime call type.
		type RuntimeCall: From<Call<Self>>;
		/// Bond required by challenger to make a challenge
		#[pallet::constant]
		type ChallengeBond: Get<Balance>;
		// The duration in blocks of one epoch
		#[pallet::constant]
		type EpochDuration: Get<u64>;
		/// Pallet subscribing to of notarized eth calls
		type EthCallSubscribers: EthCallOracleSubscriber<CallId = EthCallId>;
		/// Provides an api for Ethereum JSON-RPC request/responses to the bridged ethereum network
		type EthereumRpcClient: BridgeEthereumRpcApi;
		/// The runtime event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Handles routing received Ethereum events upon verification
		type EventRouter: EthereumEventRouter;
		/// The identifier type for an authority in this module (i.e. active validator session key)
		/// 33 byte secp256k1 public key
		type EthyId: Member
			+ Parameter
			+ AsRef<[u8]>
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;
		/// Reports the final session of na eras
		type FinalSessionTracker: FinalSessionTrackerT;
		/// Max amount of new signers that can be set an in extrinsic
		#[pallet::constant]
		type MaxNewSigners: Get<u8>;
		/// Handles a multi-currency fungible asset system
		type MultiCurrency: Mutate<Self::AccountId, Balance = Balance, AssetId = AssetId>
			+ Hold<AccountId = Self::AccountId>;
		/// The native token asset Id (managed by pallet-balances)
		#[pallet::constant]
		type NativeAssetId: Get<AssetId>;
		/// The threshold of notarizations required to approve an Ethereum event
		#[pallet::constant]
		type NotarizationThreshold: Get<Percent>;
		/// Bond required for an account to act as relayer
		#[pallet::constant]
		type RelayerBond: Get<Balance>;
		/// The Scheduler.
		type Scheduler: Anon<
			BlockNumberFor<Self>,
			<Self as Config>::RuntimeCall,
			Self::PalletsOrigin,
		>;
		/// Overarching type of all pallets origins.
		type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>;
		/// Maximum number of processed message Ids that will we keep as a buffer to prevent
		/// replays.
		#[pallet::constant]
		type MaxProcessedMessageIds: Get<u32>;
		/// Returns the block timestamp
		type UnixTime: UnixTime;
		/// Max Xrpl notary (validator) public keys
		#[pallet::constant]
		type MaxXrplKeys: Get<u8>;
		/// Xrpl-bridge adapter
		type XrplBridgeAdapter: EthyToXrplBridgeAdapter<H160>;
		/// Maximum count of notary keys
		#[pallet::constant]
		type MaxAuthorities: Get<u32>;
		/// Maximum size of eth abi and message data
		#[pallet::constant]
		type MaxEthData: Get<u32>;
		/// Maximum number of pending challenges
		#[pallet::constant]
		type MaxChallenges: Get<u32>;
		/// Maximum number of valid messages per block
		#[pallet::constant]
		type MaxMessagesPerBlock: Get<u32>;
		/// Maximum number of Eth Call Requests
		#[pallet::constant]
		type MaxCallRequests: Get<u32>;
		/// Provides the public call to weight mapping
		type WeightInfo: WeightInfo;
		/// Strategy for integrating pallet-origin EVM logs with Frontier storages.
		/// Use `NoFrontierMerge` for no-op (tests), or `FrontierMerge` to enable merging.
		type FrontierLogMerge: crate::FrontierLogMerge<Self>;
	}
	/// Flag to indicate whether authorities have been changed during the current era

	#[pallet::storage]
	pub type AuthoritiesChangedThisEra<T> = StorageValue<_, bool, ValueQuery>;

	/// Whether the bridge is paused (e.g. during validator transitions or by governance)
	#[pallet::storage]
	pub type BridgePaused<T> = StorageValue<_, BridgePauseStatus, ValueQuery>;

	/// Maps from event claim id to challenger and bond amount paid
	#[pallet::storage]
	pub type ChallengerAccount<T: Config> =
		StorageMap<_, Twox64Concat, EventClaimId, (T::AccountId, Balance), OptionQuery>;

	#[pallet::type_value]
	pub fn DefaultChallengePeriod<T: Config>() -> BlockNumberFor<T> {
		BlockNumberFor::<T>::from(150_u32) // block time (4s) * 150 = 10 Minutes
	}

	/// The (optimistic) challenge period after which a submitted event is considered valid
	#[pallet::storage]
	pub type ChallengePeriod<T: Config> =
		StorageValue<_, BlockNumberFor<T>, ValueQuery, DefaultChallengePeriod<T>>;

	/// The bridge contract address on Ethereum
	#[pallet::storage]
	pub type ContractAddress<T> = StorageValue<_, EthAddress, ValueQuery>;

	#[pallet::type_value]
	pub fn DefaultEventBlockConfirmations() -> u64 {
		3u64
	}

	/// The minimum number of block confirmations needed to notarize an Ethereum event
	#[pallet::storage]
	pub type EventBlockConfirmations<T> =
		StorageValue<_, u64, ValueQuery, DefaultEventBlockConfirmations>;

	/// Notarizations for queued events
	/// Either: None = no notarization exists OR Some(yay/nay)
	#[pallet::storage]
	pub type EventNotarizations<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EventClaimId,
		Twox64Concat,
		T::EthyId,
		EventClaimResult,
		OptionQuery,
	>;

	#[pallet::type_value]
	pub fn DefaultDelayedEventProofsPerBlock() -> u8 {
		5u8
	}

	/// The maximum number of delayed events that can be processed in on_initialize()
	#[pallet::storage]
	pub type DelayedEventProofsPerBlock<T> =
		StorageValue<_, u8, ValueQuery, DefaultDelayedEventProofsPerBlock>;

	/// Id of the next event proof
	#[pallet::storage]
	pub type NextEventProofId<T> = StorageValue<_, EventProofId, ValueQuery>;

	/// Scheduled notary (validator) public keys for the next session
	#[pallet::storage]
	pub type NextNotaryKeys<T: Config> =
		StorageValue<_, WeakBoundedVec<T::EthyId, T::MaxAuthorities>, ValueQuery>;

	/// Active notary (validator) public keys
	#[pallet::storage]
	pub type NotaryKeys<T: Config> =
		StorageValue<_, WeakBoundedVec<T::EthyId, T::MaxAuthorities>, ValueQuery>;

	/// Active xrpl notary (validator) public keys
	#[pallet::storage]
	pub type NotaryXrplKeys<T: Config> =
		StorageValue<_, WeakBoundedVec<T::EthyId, T::MaxAuthorities>, ValueQuery>;

	/// Door Signers set by sudo (white list)
	#[pallet::storage]
	pub type XrplDoorSigners<T: Config> = StorageMap<_, Twox64Concat, T::EthyId, bool, ValueQuery>;

	/// The current validator set id
	#[pallet::storage]
	pub type NotarySetId<T> = StorageValue<_, u64, ValueQuery>;

	/// The event proof Id generated by the previous validator set to notarize the current set.
	/// Useful for syncing the latest proof to Ethereum
	#[pallet::storage]
	pub type NotarySetProofId<T> = StorageValue<_, EventProofId, ValueQuery>;

	/// The event proof Id generated by the previous validator set to notarize the current set.
	/// Useful for syncing the latest proof to Xrpl
	#[pallet::storage]
	pub type XrplNotarySetProofId<T> = StorageValue<_, EventProofId, ValueQuery>;

	/// Queued event claims, can be challenged within challenge period
	#[pallet::storage]
	pub type PendingEventClaims<T: Config> =
		StorageMap<_, Twox64Concat, EventClaimId, EventClaim<T::MaxEthData>, OptionQuery>;

	/// Queued event proofs to be processed once bridge has been re-enabled
	#[pallet::storage]
	pub type PendingEventProofs<T: Config> =
		StorageMap<_, Twox64Concat, EventProofId, EthySigningRequest<T::MaxEthData>, OptionQuery>;

	/// List of all event ids that are currently being challenged
	#[pallet::storage]
	pub type PendingClaimChallenges<T: Config> =
		StorageValue<_, BoundedVec<EventClaimId, T::MaxChallenges>, ValueQuery>;

	/// Status of pending event claims
	#[pallet::storage]
	pub type PendingClaimStatus<T> =
		StorageMap<_, Twox64Concat, EventProofId, EventClaimStatus, OptionQuery>;

	/// Tracks processed message Ids (prevent replay)
	/// Must remain unbounded as this list will grow indefinitely
	#[pallet::storage]
	pub type ProcessedMessageIds<T: Config> =
		StorageValue<_, BoundedVec<EventClaimId, T::MaxProcessedMessageIds>, ValueQuery>;

	/// Tracks message Ids that are outside of the MessageId buffer and were not processed
	/// These message Ids can be either processed or cleared by the relayer
	#[pallet::storage]
	pub type MissedMessageIds<T> = StorageValue<_, Vec<EventClaimId>, ValueQuery>;

	/// The block in which we process the next authority change
	#[pallet::storage]
	pub type NextAuthorityChange<T: Config> = StorageValue<_, BlockNumberFor<T>, OptionQuery>;

	/// Map from block number to list of EventClaims that will be considered valid and should be
	/// forwarded to handlers (i.e after the optimistic challenge period has passed without issue)
	#[pallet::storage]
	pub type MessagesValidAt<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		WeakBoundedVec<EventClaimId, T::MaxMessagesPerBlock>,
		ValueQuery,
	>;

	// State Oracle
	/// Subscription Id for EthCall requests
	#[pallet::storage]
	pub type NextEthCallId<T> = StorageValue<_, EthCallId, ValueQuery>;

	/// The permissioned relayer
	#[pallet::storage]
	pub type Relayer<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// Maps from relayer account to their paid bond amount
	#[pallet::storage]
	pub type RelayerPaidBond<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Balance, ValueQuery>;

	/// Queue of pending EthCallOracle requests
	#[pallet::storage]
	pub type EthCallRequests<T: Config> =
		StorageValue<_, WeakBoundedVec<EthCallId, T::MaxCallRequests>, ValueQuery>;

	/// EthCallOracle notarizations keyed by (Id, Notary)
	#[pallet::storage]
	pub type EthCallNotarizations<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EthCallId,
		Twox64Concat,
		T::EthyId,
		CheckedEthCallResult,
		OptionQuery,
	>;

	/// map from EthCallOracle notarizations to an aggregated count
	#[pallet::storage]
	pub type EthCallNotarizationsAggregated<T> =
		StorageMap<_, Twox64Concat, EthCallId, BTreeMap<CheckedEthCallResult, u32>, OptionQuery>;

	/// EthCallOracle request info
	#[pallet::storage]
	pub type EthCallRequestInfo<T: Config> =
		StorageMap<_, Twox64Concat, EthCallId, CheckedEthCallRequest<T::MaxEthData>, OptionQuery>;

	/// Staging area for EVM call activities originating from pallet logic in the current block.
	/// These are merged into Frontier's canonical storages (CurrentTransactionStatuses/CurrentBlock)
	/// during on_finalize to ensure Ethereum tooling (eth_getLogs) sees them with correct ordering.
	#[pallet::storage]
	pub type StagedEvmActivities<T: Config> = StorageValue<_, Vec<EvmCallActivity>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Verifying an event succeeded
		Verified { event_claim_id: EventClaimId },
		/// Verifying an event failed
		Invalid { event_claim_id: EventClaimId },
		/// A notary (validator) set change is in motion
		/// A proof for the change will be generated with the given `event_id`
		AuthoritySetChange { event_proof_id: EventProofId, validator_set_id: u64 },
		/// A notary (validator) set change for Xrpl is in motion
		/// A set of proofs for the change will be generated with the given `event_proof_ids`
		XrplAuthoritySetChange { event_proof_ids: Vec<EventProofId>, validator_set_id: u64 },
		/// Generating event proof delayed as bridge is paused
		ProofDelayed { event_proof_id: EventProofId },
		/// Processing an event succeeded
		ProcessingOk { event_claim_id: EventClaimId },
		/// Processing an event failed
		ProcessingFailed { event_claim_id: EventClaimId, router_error: EventRouterError },
		/// An event has been challenged
		Challenged { event_claim_id: EventClaimId, challenger: T::AccountId },
		/// The event is still awaiting consensus. Process block pushed out
		ProcessAtExtended { event_claim_id: EventClaimId, process_at: BlockNumberFor<T> },
		/// An event proof has been sent for signing by ethy-gadget
		EventSend {
			event_proof_id: EventProofId,
			signing_request: EthySigningRequest<T::MaxEthData>,
		},
		/// An event has been submitted from Ethereum
		EventSubmit {
			event_claim_id: EventClaimId,
			event_claim: EventClaim<T::MaxEthData>,
			process_at: BlockNumberFor<T>,
		},
		/// An account has deposited a relayer bond
		RelayerBondDeposit { relayer: T::AccountId, bond: Balance },
		/// An account has withdrawn a relayer bond
		RelayerBondWithdraw { relayer: T::AccountId, bond: Balance },
		/// A new relayer has been set
		RelayerSet { relayer: Option<T::AccountId> },
		/// Xrpl Door signers are set
		XrplDoorSignersSet { new_signers: Vec<(T::EthyId, bool)> },
		/// The schedule to unpause the bridge has failed
		FinaliseScheduleFail { scheduled_block: BlockNumberFor<T> },
		/// The bridge contract address has been set
		SetContractAddress { address: EthAddress },
		/// Xrpl authority set change request failed
		XrplAuthoritySetChangeRequestFailed { error: DispatchError },
		/// Ethereum event confirmations were set
		EventBlockConfirmationsSet { confirmations: u64 },
		/// DelayedEventProofsPerBlock was set
		DelayedEventProofsPerBlockSet { count: u8 },
		/// A new challenge period was set
		ChallengePeriodSet { period: BlockNumberFor<T> },
		/// The bridge has been manually paused or unpaused
		BridgeManualPause { paused: bool },
		/// A range of missing event Ids were removed
		MissingEventIdsRemoved { range: (EventClaimId, EventClaimId) },
	}

	#[pallet::error]
	pub enum Error<T> {
		// Error returned when making signed transactions in off-chain worker
		NoLocalSigningAccount,
		// Error returned when making unsigned transactions with signed payloads in off-chain
		// worker
		OffchainUnsignedTxSignedPayload,
		/// A notarization was invalid
		InvalidNotarization,
		// Error returned when fetching github info
		HttpFetch,
		/// Claim was invalid e.g. not properly ABI encoded
		InvalidClaim,
		/// offchain worker not configured properly
		OcwConfig,
		/// Event was already submitted and is pending
		EventReplayPending,
		/// Event was already submitted and is complete
		EventReplayProcessed,
		/// The bridge is paused pending validator set changes (once every era / 24 hours)
		/// It will reactive after ~10 minutes
		BridgePaused,
		/// Some internal operation failed
		Internal,
		/// Caller does not have permission for that action
		NoPermission,
		/// There is no event claim associated with the supplied claim_id
		NoClaim,
		/// There is already a challenge for this claim
		ClaimAlreadyChallenged,
		/// The relayer is active and cant unbond the specified amount
		CantUnbondRelayer,
		/// The relayer already has a bonded amount
		CantBondRelayer,
		/// The relayer hasn't paid the relayer bond so can't be set as the active relayer
		NoBondPaid,
		/// Someone tried to set a greater amount of validators than allowed
		MaxNewSignersExceeded,
		/// No more challenges are allowed for this claim_id
		MaxChallengesExceeded,
		/// The supplied message length is above the specified bounds
		MessageTooLarge,
	}

	// Unified hooks; Frontier-specific behavior is called conditionally.
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// This method schedules 3 different processes
		/// 1) Handle change in authorities 5 minutes before the end of an epoch
		/// 2) Process any newly valid event claims (incoming)
		/// 3) Process any deferred event proofs that were submitted while the bridge was paused
		/// (should only happen on the first few blocks in a new era) (outgoing)
		fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
			// Reads: NextAuthorityChange, MessagesValidAt, ProcessedMessageIds
			let mut consumed_weight = DbWeight::get().reads(3u64);
			// Delegate optional Frontier cleanup to the strategy; adds its weight if any.
			consumed_weight = consumed_weight.saturating_add(
				<T as pallet::Config>::FrontierLogMerge::on_initialize(),
			);

			// 1) Handle authority change
			if Some(block_number) == NextAuthorityChange::<T>::get() {
				// Change authority keys, we are 5 minutes before the next epoch
				log!(trace, "💎 Epoch ends in 5 minutes, changing authorities");
				Self::handle_authorities_change();
				consumed_weight = consumed_weight.saturating_add(
					<T as crate::pallet::Config>::WeightInfo::handle_authorities_change(),
				);
			}

			// 2) Process validated messages
			// Removed message_id from MessagesValidAt and processes
			let mut processed_message_ids = ProcessedMessageIds::<T>::get().into_inner();
			let mut message_processed: bool = false;
			for message_id in MessagesValidAt::<T>::take(block_number) {
				// reads: PendingClaimStatus, PendingEventClaims
				// writes: PendingClaimStatus, PendingEventClaims
				consumed_weight =
					consumed_weight.saturating_add(DbWeight::get().reads_writes(2_u64, 2_u64));
				if PendingClaimStatus::<T>::get(message_id) == Some(EventClaimStatus::Challenged) {
					// We are still waiting on the challenge to be processed, push out by challenge
					// period
					let new_process_at = block_number + ChallengePeriod::<T>::get();
					// read + write: MessagesValidAt
					consumed_weight =
						consumed_weight.saturating_add(DbWeight::get().reads_writes(1_u64, 1_u64));
					MessagesValidAt::<T>::mutate(new_process_at, |v| {
						let mut message_ids = v.clone().into_inner();
						message_ids.push(message_id);
						let message_ids_bounded = WeakBoundedVec::force_from(
							message_ids,
							Some(
								"Warning: There are more MessagesValidAt than expected. \
								A runtime configuration adjustment may be needed.",
							),
						);
						*v = message_ids_bounded;
					});
					Self::deposit_event(Event::<T>::ProcessAtExtended {
						event_claim_id: message_id,
						process_at: new_process_at,
					});
					continue;
				}
				// Removed PendingEventClaim from storage and processes
				if let Some(EventClaim { source, destination, data, .. }) =
					PendingEventClaims::<T>::take(message_id)
				{
					// keep a runtime hardcoded list of destination <> palletId
					match T::EventRouter::route(&source, &destination, &data) {
						Ok(weight) => {
							consumed_weight += weight;
							Self::deposit_event(Event::<T>::ProcessingOk {
								event_claim_id: message_id,
							});
						},
						Err((weight, err)) => {
							consumed_weight += weight;
							Self::deposit_event(Event::<T>::ProcessingFailed {
								event_claim_id: message_id,
								router_error: err,
							});
						},
					}
				}

				let first_processed = processed_message_ids.first().cloned().unwrap_or_default();
				// Is this message_id within the MaxProcessedMessageIds?
				if message_id >= first_processed {
					// mark as processed
					if let Err(idx) = processed_message_ids.binary_search(&message_id) {
						processed_message_ids.insert(idx, message_id);
					}
				} else {
					// REMOVE event_id to MissedMessageIds
					// read + write: MissedMessageIds
					consumed_weight =
						consumed_weight.saturating_add(DbWeight::get().reads_writes(1_u64, 1_u64));
					MissedMessageIds::<T>::mutate(|missed_message_ids| {
						if let Ok(idx) = missed_message_ids.binary_search(&message_id) {
							missed_message_ids.remove(idx);
						}
					});
				}

				// Tidy up status check
				PendingClaimStatus::<T>::remove(message_id);
				message_processed = true;
			}

			if message_processed && !processed_message_ids.is_empty() {
				// write: ProcessedMessageIds
				consumed_weight = consumed_weight.saturating_add(DbWeight::get().writes(1_u64));
				let prune_weight = Self::prune_claim_ids(&mut processed_message_ids);
				consumed_weight = consumed_weight.saturating_add(prune_weight);
				// Truncate is safe as the length is asserted within prune_claim_ids
				let processed_message_ids = BoundedVec::truncate_from(processed_message_ids);
				ProcessedMessageIds::<T>::put(processed_message_ids);
			}

			consumed_weight
		}

		fn on_finalize(_n: BlockNumberFor<T>) {
			// Delegate optional Frontier finalize to the strategy
			<T as pallet::Config>::FrontierLogMerge::on_finalize();
		}

		fn on_idle(_n: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			// Minimum weight to read the initial values:
			// - BridgePaused, PendingEventProofs, DelayedEventProofsPerBlock
			let base_weight = DbWeight::get().reads(3u64);

			// do_request_proof weight:
			// reads - BridgePaused
			// writes - PendingEventProofs || frame_system::Digest
			// loop weight:
			// reads_writes - PendingEventProofs
			let weight_per_proof = DbWeight::get().reads_writes(2, 2);

			// Do we have enough weight to process one proof?
			if remaining_weight.ref_time() <= (base_weight + weight_per_proof).ref_time() {
				return Weight::zero();
			}

			// Don't do anything if the bridge is paused
			if Self::bridge_paused() {
				return DbWeight::get().reads(1u64);
			}

			let mut consumed_weight = base_weight;
			let mut pending_event_proofs = PendingEventProofs::<T>::drain();
			let max_delayed_events = DelayedEventProofsPerBlock::<T>::get();

			for _ in 0..max_delayed_events {
				// Check if we have enough weight to process this iteration
				let new_consumed_weight = consumed_weight.saturating_add(weight_per_proof);
				if remaining_weight.ref_time() <= new_consumed_weight.ref_time() {
					break;
				}
				let Some((event_proof_id, signing_request)) = pending_event_proofs.next() else {
					break;
				};
				consumed_weight = new_consumed_weight;
				Self::do_request_event_proof(event_proof_id, signing_request);
			}

			consumed_weight
		}

	fn offchain_worker(block_number: BlockNumberFor<T>) {
			let active_notaries = NotaryKeys::<T>::get().into_inner();
			log!(debug, "💎 entering off-chain worker: {:?}", block_number);
			log!(debug, "💎 active notaries: {:?}", active_notaries);

			// this passes if flag `--validator` set, not necessarily in the active set
			if !sp_io::offchain::is_validator() {
				log!(info, "💎 not a validator, exiting");
				return;
			}

			// check a local key exists for a valid bridge notary
			if let Some((active_key, authority_index)) = Self::find_active_ethy_key() {
				// check enough validators have active notary keys
				let supports = active_notaries.len();
				let needed = T::NotarizationThreshold::get();
				// TODO: check every session change not block
				if Percent::from_rational(
					supports as u32,
					T::AuthoritySet::validators().len() as u32,
				) < needed
				{
					log!(
						info,
						"💎 waiting for validator support to activate eth-bridge: {:?}/{:?}",
						supports,
						needed
					);
					return;
				}
				// do some notarizing
				Self::do_event_notarization_ocw(&active_key, authority_index);
				Self::do_call_notarization_ocw(&active_key, authority_index);
			} else {
				log!(debug, "💎 not an active validator, exiting");
			}

			log!(debug, "💎 exiting off-chain worker");
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> frame_support::unsigned::ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			match call {
				Call::submit_notarization { ref payload, ref signature } => {
					// notarization must be from an active notary
					let notary_keys = NotaryKeys::<T>::get();
					let notary_public_key =
						match notary_keys.get(payload.authority_index() as usize) {
							Some(id) => id,
							None => return InvalidTransaction::BadProof.into(),
						};

					// notarization must not be a duplicate/equivocation
					match payload {
						NotarizationPayload::Call { .. } => {
							if <EthCallNotarizations<T>>::contains_key(
								payload.payload_id(),
								&notary_public_key,
							) {
								log!(
									error,
									"💎 received equivocation from: {:?} on {:?}",
									notary_public_key,
									payload.payload_id()
								);
								return InvalidTransaction::BadProof.into();
							}
						},
						NotarizationPayload::Event { .. } => {
							if <EventNotarizations<T>>::contains_key(
								payload.payload_id(),
								&notary_public_key,
							) {
								log!(
									error,
									"💎 received equivocation from: {:?} on {:?}",
									notary_public_key,
									payload.payload_id()
								);
								return InvalidTransaction::BadProof.into();
							}
						},
					}

					// notarization is signed correctly
					if !(notary_public_key.verify(&payload.encode(), signature)) {
						return InvalidTransaction::BadProof.into();
					}

					ValidTransaction::with_tag_prefix("eth-bridge")
						.priority(UNSIGNED_TXS_PRIORITY)
						// 'provides' must be unique for each submission on the network (i.e. unique for
						// each claim id and validator)
						.and_provides([
							b"notarize",
							&payload.type_id().to_be_bytes(),
							&payload.payload_id().to_be_bytes(),
							&(payload.authority_index() as u64).to_be_bytes(),
						])
						.longevity(3)
						.propagate(true)
						.build()
				},
				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	/// Helper to stage EVM call activity from pallet-origin EVM executions (e.g., FeeProxy).
	/// These are merged into Frontier canonical storage at on_finalize.
	impl<T: Config> Pallet<T> {
		#[cfg(feature = "frontier-logs")]
	pub fn on_finalize_frontier()
		where
			T: pallet_ethereum::Config,
		{
			// Merge any staged EVM activities into Frontier's canonical Current* storages
			let mut staged = StagedEvmActivities::<T>::take();
			if staged.is_empty() {
				return;
			}

			// Ensure deterministic ordering to keep logIndex and transactionIndex stable
			staged.sort_by(|a, b| match a.extrinsic_index.cmp(&b.extrinsic_index) {
				core::cmp::Ordering::Equal => a.ordinal.cmp(&b.ordinal),
				other => other,
			});

			// If Frontier hasn't built a block (e.g. no Ethereum tx in block), we still have
			// Current* populated by pallet-ethereum::store_block at on_finalize; append to them.
			let mut statuses =
				pallet_ethereum::CurrentTransactionStatuses::<T>::get().unwrap_or_default();
			let block = pallet_ethereum::CurrentBlock::<T>::get();

			let mut block_logs_bloom = block.as_ref().map(|b| b.header.logs_bloom).unwrap_or_default();

			for activity in staged.into_iter() {
				// Build a TransactionStatus entry
				let transaction_index = statuses.len() as u32;
				let mut status = fp_rpc::TransactionStatus {
					transaction_hash: activity.tx_hash,
					transaction_index,
					from: activity.from,
					to: activity.to,
					contract_address: None,
					logs: activity.logs.clone(),
					logs_bloom: Default::default(),
				};

				// Compute logs bloom like Frontier does
				let mut bloom = ethereum_types::Bloom::default();
				for log in &activity.logs {
					bloom.accrue(ethereum_types::BloomInput::Raw(&log.address[..]));
					for topic in &log.topics {
						bloom.accrue(ethereum_types::BloomInput::Raw(&topic[..]));
					}
				}
				status.logs_bloom = bloom;

				// Append status
				statuses.push(status.clone());

				// Accrue into block bloom
				for status_log in &status.logs {
					block_logs_bloom.accrue(ethereum_types::BloomInput::Raw(&status_log.address[..]));
					for topic in &status_log.topics {
						block_logs_bloom.accrue(ethereum_types::BloomInput::Raw(&topic[..]));
					}
				}
			}

			// If we have a current block, update its header bloom/gas_used
			if let Some(mut b) = block.clone() {
				b.header.logs_bloom = block_logs_bloom;
				b.transactions = b.transactions; // unchanged
				pallet_ethereum::CurrentBlock::<T>::put(b);
			}

			pallet_ethereum::CurrentTransactionStatuses::<T>::put(statuses);
		}

		pub fn log_evm_call_activity(
			from: H160,
			to: Option<H160>,
			logs: Vec<ethereum::Log>,
			success: bool,
			tx_hash: H256,
		) -> DispatchResult {
			// Determine ordering information
			let extrinsic_index = frame_system::Pallet::<T>::extrinsic_index().unwrap_or(0);
			let mut staged = StagedEvmActivities::<T>::get();
			let ordinal = staged.len() as u32;
			// Synthesize a unique tx hash using provided seed and ordinal to avoid duplicates
			let mut seed = [0u8; 36];
			seed[..32].copy_from_slice(tx_hash.as_bytes());
			seed[32..].copy_from_slice(&ordinal.to_le_bytes());
			let synthetic = H256::from(sp_io::hashing::blake2_256(&seed));

			staged.push(EvmCallActivity {
				from,
				to,
				logs,
				success,
				extrinsic_index,
				ordinal,
				tx_hash: synthetic,
			});
			StagedEvmActivities::<T>::put(staged);
			Ok(())
		}
	}

	// Note: Frontier finalize helper is available but not invoked from pallet hooks.

	/// Minimal struct to stage pallet-origin EVM call results inside the block.
	#[derive(Clone, PartialEq, Eq, codec::Encode, codec::Decode, scale_info::TypeInfo)]
	pub struct EvmCallActivity {
		pub from: sp_core::H160,
		pub to: Option<sp_core::H160>,
		pub logs: sp_std::vec::Vec<ethereum::Log>,
		pub success: bool,
		// Ordering within block
		pub extrinsic_index: u32,
		pub ordinal: u32,
		// Synthetic transaction hash used for TransactionStatus
		pub tx_hash: sp_core::H256,
	}
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set new XRPL door signers
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_xrpl_door_signers(new_signers.len() as u32))]
		pub fn set_xrpl_door_signers(
			origin: OriginFor<T>,
			new_signers: Vec<(T::EthyId, bool)>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(
				(new_signers.len() as u8) < T::MaxNewSigners::get(),
				Error::<T>::MaxNewSignersExceeded
			);

			for new_signer in new_signers.clone() {
				XrplDoorSigners::<T>::insert(new_signer.0, new_signer.1);
			}

			Self::update_xrpl_notary_keys(&NotaryKeys::<T>::get());
			Self::deposit_event(Event::<T>::XrplDoorSignersSet { new_signers });
			Ok(())
		}

		/// Set the relayer address
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_relayer())]
		pub fn set_relayer(origin: OriginFor<T>, relayer: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;
			// Ensure relayer has bonded more than relayer bond amount
			ensure!(
				RelayerPaidBond::<T>::get(relayer.clone()) >= T::RelayerBond::get(),
				Error::<T>::NoBondPaid
			);
			Relayer::<T>::put(relayer.clone());
			Self::deposit_event(Event::<T>::RelayerSet { relayer: Some(relayer) });
			Ok(())
		}

		/// Submit bond for relayer account
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::deposit_relayer_bond())]
		pub fn deposit_relayer_bond(origin: OriginFor<T>) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			// Ensure relayer doesn't already have a bond set
			ensure!(RelayerPaidBond::<T>::get(origin.clone()) == 0, Error::<T>::CantBondRelayer);

			let relayer_bond = T::RelayerBond::get();
			// Attempt to place a hold from the relayer account
			T::MultiCurrency::place_hold(
				T::BridgePalletId::get(),
				&origin,
				T::NativeAssetId::get(),
				relayer_bond,
			)?;
			RelayerPaidBond::<T>::insert(origin.clone(), relayer_bond);
			Self::deposit_event(Event::<T>::RelayerBondDeposit {
				relayer: origin,
				bond: relayer_bond,
			});
			Ok(())
		}

		/// Withdraw relayer bond amount
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::withdraw_relayer_bond())]
		pub fn withdraw_relayer_bond(origin: OriginFor<T>) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			// Ensure account is not the current relayer
			if Relayer::<T>::get() == Some(origin.clone()) {
				ensure!(Relayer::<T>::get() != Some(origin.clone()), Error::<T>::CantUnbondRelayer);
			};
			let relayer_paid_bond = RelayerPaidBond::<T>::get(origin.clone());
			ensure!(relayer_paid_bond > 0, Error::<T>::CantUnbondRelayer);

			// Attempt to release the relayers hold
			T::MultiCurrency::release_hold(
				T::BridgePalletId::get(),
				&origin,
				T::NativeAssetId::get(),
				relayer_paid_bond,
			)?;
			RelayerPaidBond::<T>::remove(origin.clone());

			Self::deposit_event(Event::<T>::RelayerBondWithdraw {
				relayer: origin,
				bond: relayer_paid_bond,
			});
			Ok(())
		}

		/// Set event confirmations (blocks). Required block confirmations for an Ethereum event to
		/// be notarized by Seed
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::set_event_block_confirmations())]
		pub fn set_event_block_confirmations(
			origin: OriginFor<T>,
			confirmations: u64,
		) -> DispatchResult {
			ensure_root(origin)?;
			EventBlockConfirmations::<T>::put(confirmations);
			Self::deposit_event(Event::<T>::EventBlockConfirmationsSet { confirmations });
			Ok(())
		}

		/// Set max number of delayed events that can be processed per block
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::set_delayed_event_proofs_per_block())]
		pub fn set_delayed_event_proofs_per_block(
			origin: OriginFor<T>,
			count: u8,
		) -> DispatchResult {
			ensure_root(origin)?;
			DelayedEventProofsPerBlock::<T>::put(count);
			Self::deposit_event(Event::<T>::DelayedEventProofsPerBlockSet { count });
			Ok(())
		}

		/// Set challenge period, this is the window in which an event can be challenged before
		/// processing
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::set_challenge_period())]
		pub fn set_challenge_period(
			origin: OriginFor<T>,
			blocks: BlockNumberFor<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ChallengePeriod::<T>::put(blocks);
			Self::deposit_event(Event::<T>::ChallengePeriodSet { period: blocks });
			Ok(())
		}

		/// Set the bridge contract address on Ethereum (requires governance)
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::set_contract_address())]
		pub fn set_contract_address(
			origin: OriginFor<T>,
			contract_address: EthAddress,
		) -> DispatchResult {
			ensure_root(origin)?;
			ContractAddress::<T>::put(contract_address);
			Self::deposit_event(Event::<T>::SetContractAddress { address: contract_address });
			Ok(())
		}

		/// Pause or unpause the bridge (requires governance)
		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::set_bridge_paused())]
		pub fn set_bridge_paused(origin: OriginFor<T>, paused: bool) -> DispatchResult {
			ensure_root(origin)?;
			BridgePaused::<T>::mutate(|p| p.manual_pause = paused);
			Self::deposit_event(Event::<T>::BridgeManualPause { paused });
			Ok(())
		}

		/// Finalise authority changes, unpauses bridge and sets new notary keys
		/// Called internally after force new era
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::finalise_authorities_change())]
		pub fn finalise_authorities_change(
			origin: OriginFor<T>,
			next_notary_keys: WeakBoundedVec<T::EthyId, T::MaxAuthorities>,
		) -> DispatchResult {
			ensure_none(origin)?;
			Self::do_finalise_authorities_change(next_notary_keys);
			Ok(())
		}

		/// Admin function to manually remove an event_id from MissedMessageIds
		/// This should only be used if the event_id is confirmed to be invalid
		/// event_id_range is the lower and upper event_ids to clear (Both Inclusive)
		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::remove_missing_event_id())]
		pub fn remove_missing_event_id(
			origin: OriginFor<T>,
			event_id_range: (EventClaimId, EventClaimId),
		) -> DispatchResult {
			ensure_root(origin)?;
			let mut missed_message_ids = MissedMessageIds::<T>::get();
			missed_message_ids = missed_message_ids
				.into_iter()
				.filter(|id| *id < event_id_range.0 || *id > event_id_range.1)
				.collect::<Vec<_>>();
			MissedMessageIds::<T>::put(missed_message_ids);
			Self::deposit_event(Event::<T>::MissingEventIdsRemoved { range: event_id_range });
			Ok(())
		}

		/// Submit ABI encoded event data from the Ethereum bridge contract
		/// Used to recover events that were pruned but not handled by the pruning algorithn,
		/// Only events contained within MissedMessageIds can be processed here
		/// - tx_hash The Ethereum transaction hash which triggered the event
		/// - event ABI encoded bridge event
		#[pallet::call_index(11)]
		#[pallet::weight(T::WeightInfo::submit_missing_event())]
		#[transactional]
		pub fn submit_missing_event(
			origin: OriginFor<T>,
			tx_hash: H256,
			event: Vec<u8>,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			ensure!(Some(origin) == Relayer::<T>::get(), Error::<T>::NoPermission);

			let (event_id, event_claim) = Self::decode_event_data(tx_hash, event)?;

			// Ensure the message Id is contained within missed message ids
			// This is to handle the case where a message ID was pruned but not processed
			let missed_message_ids: Vec<EventClaimId> = MissedMessageIds::<T>::get();
			ensure!(
				missed_message_ids.binary_search(&event_id).is_ok(),
				Error::<T>::EventReplayProcessed
			);

			Self::do_submit_event(event_id, event_claim)?;
			Ok(())
		}

		/// Submit ABI encoded event data from the Ethereum bridge contract
		/// - tx_hash The Ethereum transaction hash which triggered the event
		/// - event ABI encoded bridge event
		#[pallet::call_index(12)]
		#[pallet::weight(T::WeightInfo::submit_event())]
		pub fn submit_event(origin: OriginFor<T>, tx_hash: H256, event: Vec<u8>) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			ensure!(Some(origin) == Relayer::<T>::get(), Error::<T>::NoPermission);

			let (event_id, event_claim) = Self::decode_event_data(tx_hash, event)?;

			// Verify that the event_id is not contained within ProcessedMessageIds
			// to prevent replay
			let processed_message_ids: Vec<EventClaimId> =
				ProcessedMessageIds::<T>::get().into_inner();
			if !processed_message_ids.is_empty() {
				ensure!(
					event_id > processed_message_ids[0]
						&& processed_message_ids.binary_search(&event_id).is_err(),
					Error::<T>::EventReplayProcessed
				);
			}
			Self::do_submit_event(event_id, event_claim)?;
			Ok(())
		}

		/// Submit a challenge for an event
		/// Challenged events won't be processed until verified by validators
		/// An event can only be challenged once
		#[pallet::call_index(13)]
		#[pallet::weight(T::WeightInfo::submit_challenge())]
		#[transactional]
		pub fn submit_challenge(
			origin: OriginFor<T>,
			event_claim_id: EventClaimId,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			// Validate event_id existence
			ensure!(PendingEventClaims::<T>::contains_key(event_claim_id), Error::<T>::NoClaim);
			// Check that event isn't already being challenged
			ensure!(
				PendingClaimStatus::<T>::get(event_claim_id) == Some(EventClaimStatus::Pending),
				Error::<T>::ClaimAlreadyChallenged
			);

			let challenger_bond = T::ChallengeBond::get();
			// try lock challenger bond
			T::MultiCurrency::place_hold(
				T::BridgePalletId::get(),
				&origin,
				T::NativeAssetId::get(),
				challenger_bond,
			)?;

			// Add event to challenged event storage
			// Not sorted so we can check using FIFO
			// Include challenger account for releasing funds in case claim is invalid
			PendingClaimChallenges::<T>::try_append(event_claim_id)
				.map_err(|_| Error::<T>::MaxChallengesExceeded)?;
			ChallengerAccount::<T>::insert(event_claim_id, (origin.clone(), challenger_bond));
			PendingClaimStatus::<T>::insert(event_claim_id, EventClaimStatus::Challenged);

			Self::deposit_event(Event::<T>::Challenged { event_claim_id, challenger: origin });
			Ok(())
		}

		/// Internal only
		/// Validators will submit inherents with their notarization vote for a given claim
		#[pallet::call_index(14)]
		#[pallet::weight(T::WeightInfo::submit_notarization())]
		#[transactional]
		pub fn submit_notarization(
			origin: OriginFor<T>,
			payload: NotarizationPayload,
			_signature: <<T as Config>::EthyId as RuntimeAppPublic>::Signature,
		) -> DispatchResult {
			ensure_none(origin)?;

			// we don't need to verify the signature here because it has been verified in
			// `validate_unsigned` function when sending out the unsigned tx.
			let authority_index = payload.authority_index() as usize;
			let notary_keys = NotaryKeys::<T>::get();
			let notary_public_key = match notary_keys.get(authority_index) {
				Some(id) => id,
				None => return Err(Error::<T>::InvalidNotarization.into()),
			};

			match payload {
				NotarizationPayload::Call { call_id, result, .. } => {
					Self::handle_call_notarization(call_id, result, notary_public_key)
				},
				NotarizationPayload::Event { event_claim_id, result, .. } => {
					Self::handle_event_notarization(event_claim_id, result, notary_public_key)
				},
			}
		}
	}
}
