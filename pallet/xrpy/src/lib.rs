/* Copyright 2021-2022 Centrality Investments Limited
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
//! CENNZnet validators use an offchain worker and Ethereum full node connections to independently
//! verify and observe events happened on Ethereum.
//! Once a threshold of validators sign a notarization having witnessed the event it is considered
//! verified.
//!
//! Events are opaque to this pallet, other pallet handle submitting "event claims" and "callbacks"
//! to handle success

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	pallet_prelude::*,
	traits::{UnixTime, ValidatorSet as ValidatorSetT},
	transactional,
	weights::constants::RocksDbWeight as DbWeight,
	Parameter,
};
use frame_system::{offchain::CreateSignedTransaction, pallet_prelude::*};
use sp_runtime::{
	offchain as rt_offchain,
	traits::{MaybeSerializeDeserialize, Member, SaturatedConversion, Zero},
	Percent, RuntimeAppPublic,
};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use seed_pallet_common::{
	log, EthCallOracleSubscriber, EthereumEventClaimSubscriber,
	FinalSessionTracker as FinalSessionTrackerT,
};

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
/// Bucket claims in intervals of this factor (seconds)
const BUCKET_FACTOR_S: u64 = 3_600; // 1 hour
/// Number of blocks between claim pruning
const CLAIM_PRUNING_INTERVAL: BlockNumber = BUCKET_FACTOR_S as u32 / 5_u32;

/// The logging target for this pallet
pub(crate) const LOG_TARGET: &str = "ethy";

/// This is the pallet's configuration trait
pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
	/// The runtime call type.
	type Call: From<Call<Self>>;
	/// Knows the active authority set (validator stash addresses)
	type AuthoritySet: ValidatorSetT<Self::AccountId, ValidatorId = Self::AccountId>;
	/// Pallet subscribing to of notarized eth calls
	type EthCallSubscribers: EthCallOracleSubscriber<CallId = EthCallId>;
	/// Provides an api for Ethereum JSON-RPC request/responses to the bridged ethereum network
	type EthereumRpcClient: BridgeEthereumRpcApi;
	/// The runtime event type.
	type Event: From<Event> + Into<<Self as frame_system::Config>::Event>;
	/// Pallets subscribing to notarized event claims
	type EventClaimSubscribers: EthereumEventClaimSubscriber;
	/// The identifier type for an authority in this module (i.e. active validator session key)
	/// 33 byte ECDSA public key
	type EthyId: Member
		+ Parameter
		+ AsRef<[u8]>
		+ RuntimeAppPublic
		+ Ord
		+ MaybeSerializeDeserialize;
	/// Reports the final session of na eras
	type FinalSessionTracker: FinalSessionTrackerT;
	/// The threshold of notarizations required to approve an Ethereum event
	type NotarizationThreshold: Get<Percent>;
	/// Returns the block timestamp
	type UnixTime: UnixTime;
}

decl_storage! {
	trait Store for Module<T: Config> as EthBridge {
		/// Whether the bridge is paused (e.g. during validator transitions or by governance)
		BridgePaused get(fn bridge_paused): bool;
		/// The minimum number of block confirmations needed to notarize an Ethereum event
		EventBlockConfirmations get(fn event_block_confirmations): u64 = 3;
		/// Events cannot be claimed after this time (seconds)
		EventDeadlineSeconds get(fn event_deadline_seconds): u64 = 604_800; // 1 week
		/// Notarizations for queued events
		/// Either: None = no notarization exists OR Some(yay/nay)
		EventNotarizations get(fn event_notarizations): double_map hasher(twox_64_concat) EventClaimId, hasher(twox_64_concat) T::EthyId => Option<EventClaimResult>;
		/// The maximum number of delayed events that can be processed in on_initialize()
		DelayedEventProofsPerBlock get(fn delayed_event_proofs_per_block): u8 = 5;
		/// Id of the next event claim
		NextEventClaimId get(fn next_event_claim_id): EventClaimId;
		/// Id of the next event proof
		NextEventProofId get(fn next_event_proof_id): EventProofId;
		/// Scheduled notary (validator) public keys for the next session
		NextNotaryKeys get(fn next_notary_keys): Vec<T::EthyId>;
		/// Active notary (validator) public keys
		NotaryKeys get(fn notary_keys): Vec<T::EthyId>;
		/// The current validator set id
		NotarySetId get(fn notary_set_id): u64;
		/// The event proof Id generated by the previous validator set to notarize the current set.
		/// Useful for syncing the latest proof to Ethereum
		NotarySetProofId get(fn notary_set_proof_id): EventProofId;
		/// Queued event claims, awaiting notarization
		PendingEventClaims get(fn pending_event_claims): map hasher(twox_64_concat) EventClaimId => Option<EventClaim>;
		/// Queued event proofs to be processed once bridge has been re-enabled (Ethereum ABI encoded `EventClaim`)
		PendingEventProofs get (fn pending_event_proofs): map hasher(twox_64_concat) EventProofId => Option<Message>;
		/// Map of pending tx hashes to claim Id
		PendingTxHashes get(fn pending_tx_hashes): map hasher(twox_64_concat) EthHash => EventClaimId;
		/// Processed tx hashes bucketed by unix timestamp (`BUCKET_FACTOR_S`)
		// Used in conjunction with `EventDeadlineSeconds` to prevent "double spends".
		// After a bucket is older than the deadline, any events prior are considered expired.
		// This allows the record of processed events to be pruned from state regularly
		ProcessedTxBuckets get(fn processed_tx_buckets): double_map hasher(twox_64_concat) u64, hasher(identity) EthHash => ();
		/// Set of processed tx hashes
		/// Periodically cleared after `EventDeadlineSeconds` expires
		ProcessedTxHashes get(fn processed_tx_hashes): map hasher(twox_64_concat) EthHash => ();
		/// Subscription Id for EthCall requests
		NextEthCallId: EthCallId;
		/// Queue of pending EthCallOracle requests
		EthCallRequests get(fn eth_call_requests): Vec<EthCallId>;
		/// EthCallOracle notarizations keyed by (Id, Notary)
		EthCallNotarizations: double_map hasher(twox_64_concat) EthCallId, hasher(twox_64_concat) T::EthyId => Option<CheckedEthCallResult>;
		/// map from EthCallOracle notarizations to an aggregated count
		EthCallNotarizationsAggregated get(fn eth_call_notarizations_aggregated): map hasher(twox_64_concat) EthCallId => Option<BTreeMap<CheckedEthCallResult, u32>>;
		/// EthCallOracle request info
		EthCallRequestInfo get(fn eth_call_request_info): map hasher(twox_64_concat) EthCallId => Option<CheckedEthCallRequest>;
	}
}

decl_event! {
	pub enum Event {
		/// Verifying an event succeeded
		Verified(EventClaimId),
		/// Verifying an event failed
		Invalid(EventClaimId),
		/// A notary (validator) set change is in motion (event_id, new_validator_set_id)
		/// A proof for the change will be generated with the given `event_id`
		AuthoritySetChange(EventProofId, u64),
		/// Generating event proof delayed as bridge is paused
		ProofDelayed(EventProofId),
	}
}

decl_error! {
	pub enum Error for Module<T: Config> {
		// Error returned when making signed transactions in off-chain worker
		NoLocalSigningAccount,
		// Error returned when making unsigned transactions with signed payloads in off-chain worker
		OffchainUnsignedTxSignedPayload,
		/// A notarization was invalid
		InvalidNotarization,
		// Error returned when fetching github info
		HttpFetch,
		/// Claim was invalid
		InvalidClaim,
		/// offchain worker not configured properly
		OcwConfig,
		/// This message has already been notarized
		AlreadyNotarized,
		/// Claim in progress
		DuplicateClaim,
		/// The bridge is paused pending validator set changes (once every era / 24 hours)
		/// It will reactive after ~10 minutes
		BridgePaused,
		/// Some internal operation failed
		Internal,
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// This method schedules 2 different processes
		/// 1) pruning expired transactions hashes from state every `CLAIM_PRUNING_INTERVAL` blocks
		/// 2) processing any deferred event proofs that were submitted while the bridge was paused (should only happen on the first few blocks in a new era)
		fn on_initialize(block_number: T::BlockNumber) -> Weight {
			let mut consumed_weight = 0 as Weight;

			// 1) Prune claim storage every hour on CENNZnet (BUCKET_FACTOR_S / 5 seconds = 720 blocks)
			if (block_number % T::BlockNumber::from(CLAIM_PRUNING_INTERVAL)).is_zero() {
				// Find the bucket to expire
				let now = T::UnixTime::now().as_secs().saturated_into::<u64>();
				consumed_weight += DbWeight::get().reads(1 as Weight);
				let expired_bucket_index = (now % Self::event_deadline_seconds()) / BUCKET_FACTOR_S;
				let mut removed_count = 0;
				for (expired_tx_hash, _empty_value) in ProcessedTxBuckets::iter_prefix(expired_bucket_index) {
					ProcessedTxHashes::remove(expired_tx_hash);
					removed_count += 1;
				}
				if let Some(cursor) = ProcessedTxBuckets::clear_prefix(expired_bucket_index, removed_count, None).maybe_cursor {
					log!(error, "💎 cleaning storage entries failed: {:?}", cursor);
				}
				consumed_weight += DbWeight::get().writes(2 * removed_count as Weight);
			}

			// 2) Try process delayed proofs
			consumed_weight += DbWeight::get().reads(2 as Weight);
			if PendingEventProofs::iter().next().is_some() && !Self::bridge_paused() {
				let max_delayed_events = Self::delayed_event_proofs_per_block();
				consumed_weight = consumed_weight.saturating_add(DbWeight::get().reads(1 as Weight) + max_delayed_events as Weight * DbWeight::get().writes(2 as Weight));
				for (event_proof_id, packed_event_with_id) in PendingEventProofs::iter().take(max_delayed_events as usize) {
					Self::do_request_event_proof(event_proof_id, packed_event_with_id);
					PendingEventProofs::remove(event_proof_id);
				}
			}

			consumed_weight
		}

		#[weight = DbWeight::get().writes(1)]
		/// Set event confirmations (blocks). Required block confirmations for an Ethereum event to be notarized by CENNZnet
		pub fn set_event_block_confirmations(origin, confirmations: u64) {
			ensure_root(origin)?;
			EventBlockConfirmations::put(confirmations)
		}

		#[weight = DbWeight::get().writes(1)]
		/// Set event deadline (seconds). Events cannot be notarized after this time has elapsed
		pub fn set_event_deadline(origin, seconds: u64) {
			ensure_root(origin)?;
			EventDeadlineSeconds::put(seconds);
		}

		#[weight = DbWeight::get().writes(1)]
		/// Set max number of delayed events that can be processed per block
		pub fn set_delayed_event_proofs_per_block(origin, count: u8) {
			ensure_root(origin)?;
			DelayedEventProofsPerBlock::put(count);
		}

		#[weight = 1_000_000]
		#[transactional]
		/// Internal only
		/// Validators will submit inherents with their notarization vote for a given claim
		pub fn submit_notarization(origin, payload: NotarizationPayload, _signature: <<T as Config>::EthyId as RuntimeAppPublic>::Signature) -> DispatchResult {
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
				NotarizationPayload::Call{ call_id, result, .. } => Self::handle_call_notarization(call_id, result, notary_public_key),
				NotarizationPayload::Event{ event_claim_id, result, .. } => Self::handle_event_notarization(event_claim_id, result, notary_public_key),
			}
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
					log!(info, "💎 waiting for validator support to activate eth-bridge: {:?}/{:?}", supports, needed);
					return;
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
}
