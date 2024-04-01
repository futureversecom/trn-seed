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

use ethabi::{ParamType, Token};
use frame_support::{
	pallet_prelude::*,
	traits::{
		fungibles::Transfer,
		schedule::{Anon, DispatchTime},
		UnixTime, ValidatorSet as ValidatorSetT,
	},
	transactional,
	weights::constants::RocksDbWeight as DbWeight,
	PalletId, Parameter,
};
use frame_system::{offchain::CreateSignedTransaction, pallet_prelude::*};
use hex_literal::hex;
use sp_runtime::{
	offchain as rt_offchain,
	traits::{MaybeSerializeDeserialize, Member, SaturatedConversion},
	Percent, RuntimeAppPublic,
};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

pub use pallet::*;
use seed_pallet_common::{
	log, EthCallOracleSubscriber, EthereumEventRouter, EthyToXrplBridgeAdapter, EventRouterError,
	FinalSessionTracker as FinalSessionTrackerT, Hold,
};
use seed_primitives::{AssetId, Balance};

mod ethereum_http_cli;
pub use ethereum_http_cli::EthereumRpcClient;

mod impls;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod types;
use types::*;

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
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub xrp_door_signers: Vec<T::EthyId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { xrp_door_signers: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for new_signer in self.xrp_door_signers.iter() {
				XrplDoorSigners::<T>::insert(new_signer, true);
			}
			// set the NotaryXrplKeys as well
			let genesis_xrpl_keys = NotaryKeys::<T>::get()
				.into_iter()
				.filter(|validator| XrplDoorSigners::<T>::get(validator))
				.map(|validator| -> T::EthyId { validator.clone() })
				.take(T::MaxXrplKeys::get().into())
				.collect::<Vec<_>>();
			NotaryXrplKeys::<T>::put(genesis_xrpl_keys);
		}
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
		/// Length of time the bridge will be paused while the authority set changes
		type AuthorityChangeDelay: Get<Self::BlockNumber>;
		/// Knows the active authority set (validator stash addresses)
		type AuthoritySet: ValidatorSetT<Self::AccountId, ValidatorId = Self::AccountId>;
		/// The pallet bridge address (destination for incoming messages, source for outgoing)
		type BridgePalletId: Get<PalletId>;
		/// The runtime call type.
		type RuntimeCall: From<Call<Self>>;
		/// Bond required by challenger to make a challenge
		type ChallengeBond: Get<Balance>;
		// The duration in blocks of one epoch
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
			+ MaybeSerializeDeserialize;
		/// Reports the final session of na eras
		type FinalSessionTracker: FinalSessionTrackerT;
		/// Max amount of new signers that can be set an in extrinsic
		type MaxNewSigners: Get<u8>;
		/// Handles a multi-currency fungible asset system
		type MultiCurrency: Transfer<Self::AccountId> + Hold<AccountId = Self::AccountId>;
		/// The native token asset Id (managed by pallet-balances)
		type NativeAssetId: Get<AssetId>;
		/// The threshold of notarizations required to approve an Ethereum event
		type NotarizationThreshold: Get<Percent>;
		/// Bond required for an account to act as relayer
		type RelayerBond: Get<Balance>;
		/// The Scheduler.
		type Scheduler: Anon<Self::BlockNumber, <Self as Config>::RuntimeCall, Self::PalletsOrigin>;
		/// Overarching type of all pallets origins.
		type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>;
		/// Returns the block timestamp
		type UnixTime: UnixTime;
		/// Max Xrpl notary (validator) public keys
		type MaxXrplKeys: Get<u8>;
		/// Xrpl-bridge adapter
		type XrplBridgeAdapter: EthyToXrplBridgeAdapter<H160>;
	}

	/// Flag to indicate whether authorities have been changed during the current era
	#[pallet::storage]
	#[pallet::getter(fn authorities_changed_this_era)]
	pub type AuthoritiesChangedThisEra<T> = StorageValue<_, bool, ValueQuery>;

	/// Whether the bridge is paused (e.g. during validator transitions or by governance)
	#[pallet::storage]
	#[pallet::getter(fn bridge_paused)]
	pub type BridgePaused<T> = StorageValue<_, bool, ValueQuery>;

	/// Maps from event claim id to challenger and bond amount paid
	#[pallet::storage]
	#[pallet::getter(fn challenger_account)]
	pub type ChallengerAccount<T: Config> =
		StorageMap<_, Twox64Concat, EventClaimId, (T::AccountId, Balance), OptionQuery>;

	#[pallet::type_value]
	pub fn DefaultChallengePeriod<T: Config>() -> T::BlockNumber {
		T::BlockNumber::from(150_u32) // block time (4s) * 150 = 10 Minutes
	}

	/// The (optimistic) challenge period after which a submitted event is considered valid
	#[pallet::storage]
	#[pallet::getter(fn challenge_period)]
	pub type ChallengePeriod<T: Config> =
		StorageValue<_, T::BlockNumber, ValueQuery, DefaultChallengePeriod<T>>;

	/// The bridge contract address on Ethereum
	#[pallet::storage]
	#[pallet::getter(fn contract_address)]
	pub type ContractAddress<T> = StorageValue<_, EthAddress, ValueQuery>;

	#[pallet::type_value]
	pub fn DefaultEventBlockConfirmations() -> u64 {
		3u64
	}

	/// The minimum number of block confirmations needed to notarize an Ethereum event
	#[pallet::storage]
	#[pallet::getter(fn event_block_confirmations)]
	pub type EventBlockConfirmations<T> =
		StorageValue<_, u64, ValueQuery, DefaultEventBlockConfirmations>;

	/// Notarizations for queued events
	/// Either: None = no notarization exists OR Some(yay/nay)
	#[pallet::storage]
	#[pallet::getter(fn event_notarizations)]
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
	#[pallet::getter(fn delayed_event_proofs_per_block)]
	pub type DelayedEventProofsPerBlock<T> =
		StorageValue<_, u8, ValueQuery, DefaultDelayedEventProofsPerBlock>;

	/// Id of the next event proof
	#[pallet::storage]
	#[pallet::getter(fn next_event_proof_id)]
	pub type NextEventProofId<T> = StorageValue<_, EventProofId, ValueQuery>;

	/// Scheduled notary (validator) public keys for the next session
	#[pallet::storage]
	#[pallet::getter(fn next_notary_keys)]
	pub type NextNotaryKeys<T: Config> = StorageValue<_, Vec<T::EthyId>, ValueQuery>;

	/// Active notary (validator) public keys
	#[pallet::storage]
	#[pallet::getter(fn notary_keys)]
	pub type NotaryKeys<T: Config> = StorageValue<_, Vec<T::EthyId>, ValueQuery>;

	/// Active xrpl notary (validator) public keys
	#[pallet::storage]
	#[pallet::getter(fn notary_xrpl_keys)]
	pub type NotaryXrplKeys<T: Config> = StorageValue<_, Vec<T::EthyId>, ValueQuery>;

	/// Door Signers set by sudo (white list)
	#[pallet::storage]
	#[pallet::getter(fn xrpl_door_signers)]
	pub type XrplDoorSigners<T: Config> = StorageMap<_, Twox64Concat, T::EthyId, bool, ValueQuery>;

	/// The current validator set id
	#[pallet::storage]
	#[pallet::getter(fn notary_set_id)]
	pub type NotarySetId<T> = StorageValue<_, u64, ValueQuery>;

	/// The event proof Id generated by the previous validator set to notarize the current set.
	/// Useful for syncing the latest proof to Ethereum
	#[pallet::storage]
	#[pallet::getter(fn notary_set_proof_id)]
	pub type NotarySetProofId<T> = StorageValue<_, EventProofId, ValueQuery>;

	/// The event proof Id generated by the previous validator set to notarize the current set.
	/// Useful for syncing the latest proof to Xrpl
	#[pallet::storage]
	#[pallet::getter(fn xrpl_notary_set_proof_id)]
	pub type XrplNotarySetProofId<T> = StorageValue<_, EventProofId, ValueQuery>;

	/// Queued event claims, can be challenged within challenge period
	#[pallet::storage]
	#[pallet::getter(fn pending_event_claims)]
	pub type PendingEventClaims<T> =
		StorageMap<_, Twox64Concat, EventClaimId, EventClaim, OptionQuery>;

	/// Queued event proofs to be processed once bridge has been re-enabled
	#[pallet::storage]
	#[pallet::getter(fn pending_event_proofs)]
	pub type PendingEventProofs<T> =
		StorageMap<_, Twox64Concat, EventProofId, EthySigningRequest, OptionQuery>;

	/// List of all event ids that are currently being challenged
	#[pallet::storage]
	#[pallet::getter(fn pending_claim_challenges)]
	pub type PendingClaimChallenges<T> = StorageValue<_, Vec<EventClaimId>, ValueQuery>;

	/// Status of pending event claims
	#[pallet::storage]
	#[pallet::getter(fn pending_claim_status)]
	pub type PendingClaimStatus<T> =
		StorageMap<_, Twox64Concat, EventProofId, EventClaimStatus, OptionQuery>;

	/// Tracks processed message Ids (prevent replay)
	#[pallet::storage]
	#[pallet::getter(fn processed_message_ids)]
	pub type ProcessedMessageIds<T> = StorageValue<_, Vec<EventClaimId>, ValueQuery>;

	/// The block in which we process the next authority change
	#[pallet::storage]
	#[pallet::getter(fn next_authority_change)]
	pub type NextAuthorityChange<T: Config> = StorageValue<_, T::BlockNumber, OptionQuery>;

	/// Map from block number to list of EventClaims that will be considered valid and should be
	/// forwarded to handlers (i.e after the optimistic challenge period has passed without issue)
	#[pallet::storage]
	#[pallet::getter(fn messages_valid_at)]
	pub type MessagesValidAt<T: Config> =
		StorageMap<_, Twox64Concat, T::BlockNumber, Vec<EventClaimId>, ValueQuery>;

	// State Oracle
	/// Subscription Id for EthCall requests
	#[pallet::storage]
	pub type NextEthCallId<T> = StorageValue<_, EthCallId, ValueQuery>;

	/// The permissioned relayer
	#[pallet::storage]
	#[pallet::getter(fn relayer)]
	pub type Relayer<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// Maps from relayer account to their paid bond amount
	#[pallet::storage]
	#[pallet::getter(fn relayer_paid_bond)]
	pub type RelayerPaidBond<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Balance, ValueQuery>;

	/// Queue of pending EthCallOracle requests
	#[pallet::storage]
	#[pallet::getter(fn eth_call_requests)]
	pub type EthCallRequests<T> = StorageValue<_, Vec<EthCallId>, ValueQuery>;

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
	#[pallet::getter(fn eth_call_notarizations_aggregated)]
	pub type EthCallNotarizationsAggregated<T> =
		StorageMap<_, Twox64Concat, EthCallId, BTreeMap<CheckedEthCallResult, u32>, OptionQuery>;

	/// EthCallOracle request info
	#[pallet::storage]
	#[pallet::getter(fn eth_call_request_info)]
	pub type EthCallRequestInfo<T> =
		StorageMap<_, Twox64Concat, EthCallId, CheckedEthCallRequest, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Verifying an event succeeded
		Verified(EventClaimId),
		/// Verifying an event failed
		Invalid(EventClaimId),
		/// A notary (validator) set change is in motion (event_id, new_validator_set_id)
		/// A proof for the change will be generated with the given `event_id`
		AuthoritySetChange(EventProofId, u64),
		/// A notary (validator) set change for Xrpl is in motion (event_id, new_validator_set_id)
		/// A proof for the change will be generated with the given `event_id`
		XrplAuthoritySetChange(EventProofId, u64),
		/// Generating event proof delayed as bridge is paused
		ProofDelayed(EventProofId),
		/// Processing an event succeeded
		ProcessingOk(EventClaimId),
		/// Processing an event failed
		ProcessingFailed(EventClaimId, EventRouterError),
		/// An event has been challenged (claim_id, challenger)
		Challenged(EventClaimId, T::AccountId),
		/// The event is still awaiting consensus. Process block pushed out (claim_id, process_at)
		ProcessAtExtended(EventClaimId, T::BlockNumber),
		/// An event proof has been sent for signing by ethy-gadget
		EventSend { event_proof_id: EventProofId, signing_request: EthySigningRequest },
		/// An event has been submitted from Ethereum (event_claim_id, event_claim, process_at)
		EventSubmit(EventClaimId, EventClaim, T::BlockNumber),
		/// An account has deposited a relayer bond
		RelayerBondDeposit(T::AccountId, Balance),
		/// An account has withdrawn a relayer bond
		RelayerBondWithdraw(T::AccountId, Balance),
		/// A new relayer has been set
		RelayerSet(Option<T::AccountId>),
		/// Xrpl Door signers are set
		XrplDoorSignersSet,
		/// The schedule to unpause the bridge has failed (scheduled_block)
		FinaliseScheduleFail(T::BlockNumber),
		/// The bridge contract address has been set
		SetContractAddress(EthAddress),
		/// Xrpl authority set change request failed
		XrplAuthoritySetChangeRequestFailed,
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
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// This method schedules 3 different processes
		/// 1) Handle change in authorities 5 minutes before the end of an epoch
		/// 2) Process any newly valid event claims (incoming)
		/// 3) Process any deferred event proofs that were submitted while the bridge was paused
		/// (should only happen on the first few blocks in a new era) (outgoing)
		fn on_initialize(block_number: T::BlockNumber) -> Weight {
			let mut consumed_weight = Weight::zero();

			// 1) Handle authority change
			if Some(block_number) == Self::next_authority_change() {
				// Change authority keys, we are 5 minutes before the next epoch
				log!(trace, "💎 Epoch ends in 5 minutes, changing authorities");
				Self::handle_authorities_change();
			}

			// 2) Process validated messages
			// Removed message_id from MessagesValidAt and processes
			let mut processed_message_ids = Self::processed_message_ids();
			for message_id in MessagesValidAt::<T>::take(block_number) {
				if Self::pending_claim_status(message_id) == Some(EventClaimStatus::Challenged) {
					// We are still waiting on the challenge to be processed, push out by challenge
					// period
					let new_process_at = block_number + Self::challenge_period();
					<MessagesValidAt<T>>::append(new_process_at, message_id);
					Self::deposit_event(Event::<T>::ProcessAtExtended(message_id, new_process_at));
					continue
				}
				// Removed PendingEventClaim from storage and processes
				if let Some(EventClaim { source, destination, data, .. }) =
					PendingEventClaims::<T>::take(message_id)
				{
					// keep a runtime hardcoded list of destination <> palletId
					match T::EventRouter::route(&source, &destination, &data) {
						Ok(weight) => {
							consumed_weight += weight;
							Self::deposit_event(Event::<T>::ProcessingOk(message_id));
						},
						Err((weight, err)) => {
							consumed_weight += weight;
							Self::deposit_event(Event::<T>::ProcessingFailed(message_id, err));
						},
					}
				}
				// mark as processed
				if let Err(idx) = processed_message_ids.binary_search(&message_id) {
					processed_message_ids.insert(idx, message_id);
				}
				// Tidy up status check
				PendingClaimStatus::<T>::remove(message_id);
			}
			if !processed_message_ids.is_empty() {
				impls::prune_claim_ids(&mut processed_message_ids);
				ProcessedMessageIds::<T>::put(processed_message_ids);
			}

			// 3) Try process delayed proofs
			consumed_weight += DbWeight::get().reads(2u64);
			if PendingEventProofs::<T>::iter().next().is_some() && !Self::bridge_paused() {
				let max_delayed_events = Self::delayed_event_proofs_per_block();
				consumed_weight = consumed_weight.saturating_add(DbWeight::get().reads(1u64));
				consumed_weight = consumed_weight
					.saturating_add(DbWeight::get().writes(2u64).mul(max_delayed_events as u64));
				for (event_proof_id, signing_request) in
					PendingEventProofs::<T>::iter().take(max_delayed_events as usize)
				{
					Self::do_request_event_proof(event_proof_id, signing_request);
					PendingEventProofs::<T>::remove(event_proof_id);
				}
			}

			consumed_weight
		}

		fn offchain_worker(block_number: T::BlockNumber) {
			log!(trace, "💎 entering off-chain worker: {:?}", block_number);
			log!(trace, "💎 active notaries: {:?}", Self::notary_keys());

			// this passes if flag `--validator` set, not necessarily in the active set
			if !sp_io::offchain::is_validator() {
				log!(info, "💎 not a validator, exiting");
				return
			}

			// check a local key exists for a valid bridge notary
			if let Some((active_key, authority_index)) = Self::find_active_ethy_key() {
				// check enough validators have active notary keys
				let supports = NotaryKeys::<T>::decode_len().unwrap_or(0);
				let needed = T::NotarizationThreshold::get();
				// TODO: check every session change not block
				if Percent::from_rational(supports, T::AuthoritySet::validators().len()) < needed {
					log!(
						info,
						"💎 waiting for validator support to activate eth-bridge: {:?}/{:?}",
						supports,
						needed
					);
					return
				}
				// do some notarizing
				Self::do_event_notarization_ocw(&active_key, authority_index);
				Self::do_call_notarization_ocw(&active_key, authority_index);
			} else {
				log!(trace, "💎 not an active validator, exiting");
			}

			log!(trace, "💎 exiting off-chain worker");
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set new XRPL door signers
		#[pallet::weight(DbWeight::get().writes(new_signers.len() as u64).saturating_add(DbWeight::get().reads_writes(4, 3)))]
		pub fn set_xrpl_door_signers(
			origin: OriginFor<T>,
			new_signers: Vec<(T::EthyId, bool)>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(
				(new_signers.len() as u8) < T::MaxNewSigners::get(),
				Error::<T>::MaxNewSignersExceeded
			);

			for new_signer in new_signers {
				XrplDoorSigners::<T>::insert(new_signer.0, new_signer.1);
			}

			Self::update_xrpl_notary_keys(&Self::notary_keys());
			Self::deposit_event(Event::<T>::XrplDoorSignersSet);
			Ok(())
		}

		/// Set the relayer address
		#[pallet::weight(DbWeight::get().writes(1))]
		pub fn set_relayer(origin: OriginFor<T>, relayer: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;
			// Ensure relayer has bonded more than relayer bond amount
			ensure!(
				Self::relayer_paid_bond(relayer.clone()) >= T::RelayerBond::get(),
				Error::<T>::NoBondPaid
			);
			Relayer::<T>::put(relayer.clone());
			Self::deposit_event(Event::<T>::RelayerSet(Some(relayer)));
			Ok(())
		}

		/// Submit bond for relayer account
		#[pallet::weight(DbWeight::get().reads_writes(5, 6))]
		pub fn deposit_relayer_bond(origin: OriginFor<T>) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			// Ensure relayer doesn't already have a bond set
			ensure!(Self::relayer_paid_bond(origin.clone()) == 0, Error::<T>::CantBondRelayer);

			let relayer_bond = T::RelayerBond::get();
			// Attempt to place a hold from the relayer account
			T::MultiCurrency::place_hold(
				T::BridgePalletId::get(),
				&origin,
				T::NativeAssetId::get(),
				relayer_bond,
			)?;
			RelayerPaidBond::<T>::insert(origin.clone(), relayer_bond);
			Self::deposit_event(Event::<T>::RelayerBondDeposit(origin, relayer_bond));
			Ok(())
		}

		/// Withdraw relayer bond amount
		#[pallet::weight(DbWeight::get().reads_writes(3, 3))]
		pub fn withdraw_relayer_bond(origin: OriginFor<T>) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			// Ensure account is not the current relayer
			if Self::relayer() == Some(origin.clone()) {
				ensure!(Self::relayer() != Some(origin.clone()), Error::<T>::CantUnbondRelayer);
			};
			let relayer_paid_bond = Self::relayer_paid_bond(origin.clone());
			ensure!(relayer_paid_bond > 0, Error::<T>::CantUnbondRelayer);

			// Attempt to release the relayers hold
			T::MultiCurrency::release_hold(
				T::BridgePalletId::get(),
				&origin,
				T::NativeAssetId::get(),
				relayer_paid_bond,
			)?;
			RelayerPaidBond::<T>::remove(origin.clone());

			Self::deposit_event(Event::<T>::RelayerBondWithdraw(origin, relayer_paid_bond));
			Ok(())
		}

		/// Set event confirmations (blocks). Required block confirmations for an Ethereum event to
		/// be notarized by Seed
		#[pallet::weight(DbWeight::get().writes(1))]
		pub fn set_event_block_confirmations(
			origin: OriginFor<T>,
			confirmations: u64,
		) -> DispatchResult {
			ensure_root(origin)?;
			EventBlockConfirmations::<T>::put(confirmations);
			Ok(())
		}

		/// Set max number of delayed events that can be processed per block
		#[pallet::weight(DbWeight::get().writes(1))]
		pub fn set_delayed_event_proofs_per_block(
			origin: OriginFor<T>,
			count: u8,
		) -> DispatchResult {
			ensure_root(origin)?;
			DelayedEventProofsPerBlock::<T>::put(count);
			Ok(())
		}

		/// Set challenge period, this is the window in which an event can be challenged before
		/// processing
		#[pallet::weight(DbWeight::get().writes(1))]
		pub fn set_challenge_period(
			origin: OriginFor<T>,
			blocks: T::BlockNumber,
		) -> DispatchResult {
			ensure_root(origin)?;
			ChallengePeriod::<T>::put(blocks);
			Ok(())
		}

		/// Set the bridge contract address on Ethereum (requires governance)
		#[pallet::weight(DbWeight::get().writes(1))]
		pub fn set_contract_address(
			origin: OriginFor<T>,
			contract_address: EthAddress,
		) -> DispatchResult {
			ensure_root(origin)?;
			ContractAddress::<T>::put(contract_address);
			Self::deposit_event(<Event<T>>::SetContractAddress(contract_address));
			Ok(())
		}

		/// Pause or unpause the bridge (requires governance)
		#[pallet::weight(DbWeight::get().writes(1))]
		pub fn set_bridge_paused(origin: OriginFor<T>, paused: bool) -> DispatchResult {
			ensure_root(origin)?;
			match paused {
				true => BridgePaused::<T>::put(true),
				false => BridgePaused::<T>::kill(),
			};
			Ok(())
		}

		/// Finalise authority changes, unpauses bridge and sets new notary keys
		/// Called internally after force new era
		#[pallet::weight(DbWeight::get().writes(1))]
		pub fn finalise_authorities_change(
			origin: OriginFor<T>,
			next_notary_keys: Vec<T::EthyId>,
		) -> DispatchResult {
			ensure_none(origin)?;
			Self::do_finalise_authorities_change(next_notary_keys);
			Ok(())
		}

		/// Submit ABI encoded event data from the Ethereum bridge contract
		/// - tx_hash The Ethereum transaction hash which triggered the event
		/// - event ABI encoded bridge event
		#[pallet::weight(DbWeight::get().writes(1))]
		pub fn submit_event(origin: OriginFor<T>, tx_hash: H256, event: Vec<u8>) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			ensure!(Some(origin) == Self::relayer(), Error::<T>::NoPermission);

			// TODO: place some limit on `data` length (it should match on contract side)
			// event SendMessage(uint256 messageId, address source, address destination, bytes
			// message, uint256 fee);
			if let [Token::Uint(event_id), Token::Address(source), Token::Address(destination), Token::Bytes(data), Token::Uint(_fee)] =
				ethabi::decode(
					&[
						ParamType::Uint(64),
						ParamType::Address,
						ParamType::Address,
						ethabi::ParamType::Bytes,
						ParamType::Uint(64),
					],
					event.as_slice(),
				)
				.map_err(|_| Error::<T>::InvalidClaim)?
				.as_slice()
			{
				let event_id: EventClaimId = (*event_id).saturated_into();
				ensure!(
					!PendingEventClaims::<T>::contains_key(event_id),
					Error::<T>::EventReplayPending
				); // NOTE(surangap): prune PendingEventClaims also?
				if !Self::processed_message_ids().is_empty() {
					ensure!(
						event_id > Self::processed_message_ids()[0] &&
							Self::processed_message_ids().binary_search(&event_id).is_err(),
						Error::<T>::EventReplayProcessed
					);
				}
				let event_claim = EventClaim {
					tx_hash,
					source: *source,
					destination: *destination,
					data: data.clone(),
				};

				PendingEventClaims::<T>::insert(event_id, &event_claim);
				PendingClaimStatus::<T>::insert(event_id, EventClaimStatus::Pending);

				// TODO: there should be some limit per block
				let process_at: T::BlockNumber =
					<frame_system::Pallet<T>>::block_number() + Self::challenge_period();
				<MessagesValidAt<T>>::append(process_at, event_id);

				Self::deposit_event(Event::<T>::EventSubmit(event_id, event_claim, process_at));
			}
			Ok(())
		}

		/// Submit a challenge for an event
		/// Challenged events won't be processed until verified by validators
		/// An event can only be challenged once
		#[pallet::weight(DbWeight::get().writes(1) + DbWeight::get().reads(2))]
		pub fn submit_challenge(
			origin: OriginFor<T>,
			event_claim_id: EventClaimId,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			// Validate event_id existence
			ensure!(PendingEventClaims::<T>::contains_key(event_claim_id), Error::<T>::NoClaim);
			// Check that event isn't already being challenged
			ensure!(
				Self::pending_claim_status(event_claim_id) == Some(EventClaimStatus::Pending),
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
			PendingClaimChallenges::<T>::append(event_claim_id);
			<ChallengerAccount<T>>::insert(event_claim_id, (origin.clone(), challenger_bond));
			PendingClaimStatus::<T>::insert(event_claim_id, EventClaimStatus::Challenged);

			Self::deposit_event(Event::<T>::Challenged(event_claim_id, origin));
			Ok(())
		}

		/// Internal only
		/// Validators will submit inherents with their notarization vote for a given claim
		#[pallet::weight(1_000_000)]
		#[transactional]
		pub fn submit_notarization(
			origin: OriginFor<T>,
			payload: NotarizationPayload,
			_signature: <<T as Config>::EthyId as RuntimeAppPublic>::Signature,
		) -> DispatchResult {
			let _ = ensure_none(origin)?;

			// we don't need to verify the signature here because it has been verified in
			// `validate_unsigned` function when sending out the unsigned tx.
			let authority_index = payload.authority_index() as usize;
			let notary_keys = Self::notary_keys();
			let notary_public_key = match notary_keys.get(authority_index) {
				Some(id) => id,
				None => return Err(Error::<T>::InvalidNotarization.into()),
			};

			match payload {
				NotarizationPayload::Call { call_id, result, .. } =>
					Self::handle_call_notarization(call_id, result, notary_public_key),
				NotarizationPayload::Event { event_claim_id, result, .. } =>
					Self::handle_event_notarization(event_claim_id, result, notary_public_key),
			}
		}
	}
}
