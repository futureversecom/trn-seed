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
	decl_error, decl_event, decl_module, decl_storage,
	pallet_prelude::*,
	traits::{fungibles::Transfer, UnixTime, ValidatorSet as ValidatorSetT},
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

use seed_pallet_common::{
	log, EthCallOracleSubscriber, EthereumEventRouter, EventRouterError,
	FinalSessionTracker as FinalSessionTrackerT, Hold,
};
use seed_primitives::{AccountId, AssetId, Balance};

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

/// This is the pallet's configuration trait
pub trait Config:
	frame_system::Config<AccountId = AccountId> + CreateSignedTransaction<Call<Self>>
{
	/// Knows the active authority set (validator stash addresses)
	type AuthoritySet: ValidatorSetT<Self::AccountId, ValidatorId = Self::AccountId>;
	/// The pallet bridge address (destination for incoming messages, source for outgoing)
	type BridgePalletId: Get<PalletId>;
	/// The runtime call type.
	type Call: From<Call<Self>>;
	/// Bond required by challenger to make a challenge
	type ChallengeBond: Get<Balance>;
	// The duration in blocks of one epoch
	type EpochDuration: Get<u64>;
	/// Pallet subscribing to of notarized eth calls
	type EthCallSubscribers: EthCallOracleSubscriber<CallId = EthCallId>;
	/// Provides an api for Ethereum JSON-RPC request/responses to the bridged ethereum network
	type EthereumRpcClient: BridgeEthereumRpcApi;
	/// The runtime event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
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
	/// Handles a multi-currency fungible asset system
	type MultiCurrency: Transfer<Self::AccountId> + Hold<AccountId = Self::AccountId>;
	/// The native token asset Id (managed by pallet-balances)
	type NativeAssetId: Get<AssetId>;
	/// The threshold of notarizations required to approve an Ethereum event
	type NotarizationThreshold: Get<Percent>;
	/// Bond required for an account to act as relayer
	type RelayerBond: Get<Balance>;
	/// Returns the block timestamp
	type UnixTime: UnixTime;
}

decl_storage! {
	trait Store for Module<T: Config> as EthBridge {
		/// Whether the bridge is paused (e.g. during validator transitions or by governance)
		BridgePaused get(fn bridge_paused): bool;
		/// Maps from event claim id to challenger and bond amount paid
		ChallengerAccount get(fn challenger_account): map hasher(twox_64_concat) EventClaimId => Option<(T::AccountId, Balance)>;
		/// The (optimistic) challenge period after which a submitted event is considered valid
		ChallengePeriod get(fn challenge_period): T::BlockNumber = T::BlockNumber::from(150_u32); // 10 Minutes
		/// The bridge contract address on Ethereum
		pub ContractAddress get(fn contract_address): EthAddress;
		/// The minimum number of block confirmations needed to notarize an Ethereum event
		EventBlockConfirmations get(fn event_block_confirmations): u64 = 3;
		/// Notarizations for queued events
		/// Either: None = no notarization exists OR Some(yay/nay)
		EventNotarizations get(fn event_notarizations): double_map hasher(twox_64_concat) EventClaimId, hasher(twox_64_concat) T::EthyId => Option<EventClaimResult>;
		/// The maximum number of delayed events that can be processed in on_initialize()
		DelayedEventProofsPerBlock get(fn delayed_event_proofs_per_block): u8 = 5;
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
		/// Queued event claims, can be challenged within challenge period
		PendingEventClaims get(fn pending_event_claims): map hasher(twox_64_concat) EventClaimId => Option<EventClaim>;
		/// Queued event proofs to be processed once bridge has been re-enabled
		PendingEventProofs get(fn pending_event_proofs): map hasher(twox_64_concat) EventProofId => Option<EthySigningRequest>;
		/// List of all event ids that are currently being challenged
		PendingClaimChallenges get(fn pending_claim_challenges): Vec<EventClaimId>;
		/// Status of pending event claims
		PendingClaimStatus get(fn pending_claim_status): map hasher(twox_64_concat) EventProofId => Option<EventClaimStatus>;
		/// Tracks processed message Ids (prevent replay)
		ProcessedMessageIds get(fn processed_message_ids): Vec<EventClaimId>;
		/// The block in which we process the next authority change
		NextAuthorityChange get(fn next_authority_change): Option<T::BlockNumber>;
		/// Map from block number to list of EventClaims that will be considered valid and should be forwarded to handlers (i.e after the optimistic challenge period has passed without issue)
		MessagesValidAt get(fn messages_valid_at): map hasher(twox_64_concat) T::BlockNumber => Vec<EventClaimId>;
		// State Oracle
		/// Subscription Id for EthCall requests
		NextEthCallId: EthCallId;
		/// The permissioned relayer
		Relayer get(fn relayer): Option<T::AccountId>;
		/// Maps from relayer account to their paid bond amount
		RelayerPaidBond get(fn relayer_paid_bond): map hasher(twox_64_concat) T::AccountId => Balance;
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
	pub enum Event<T> where
		AccountId = <T as frame_system::Config>::AccountId,
		BlockNumber = <T as frame_system::Config>::BlockNumber,
	{
		/// Verifying an event succeeded
		Verified(EventClaimId),
		/// Verifying an event failed
		Invalid(EventClaimId),
		/// A notary (validator) set change is in motion (event_id, new_validator_set_id)
		/// A proof for the change will be generated with the given `event_id`
		AuthoritySetChange(EventProofId, u64),
		/// Generating event proof delayed as bridge is paused
		ProofDelayed(EventProofId),
		/// Processing an event succeeded
		ProcessingOk(EventClaimId),
		/// Processing an event failed
		ProcessingFailed(EventClaimId, EventRouterError),
		/// An event has been challenged (claim_id, challenger)
		Challenged(EventClaimId, AccountId),
		/// The event is still awaiting consensus. Process block pushed out (claim_id, process_at)
		ProcessAtExtended(EventClaimId, BlockNumber),
		/// An event proof has been sent for signing by ethy-gadget
		EventSend { event_proof_id: EventProofId, signing_request: EthySigningRequest },
		/// An event has been submitted from Ethereum (event_claim_id, event_claim, process_at)
		EventSubmit(EventClaimId, EventClaim, BlockNumber),
		/// An account has deposited a relayer bond
		RelayerBondDeposit(AccountId, Balance),
		/// An account has withdrawn a relayer bond
		RelayerBondWithdraw(AccountId, Balance),
		/// A new relayer has been set
		RelayerSet(Option<AccountId>),
		/// The bridge contract address has been set
		SetContractAddress(EthAddress),
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
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// This method schedules 3 different processes
		/// 1) Handle change in authorities 5 minutes before the end of an epoch
		/// 2) Process any newly valid event claims (incoming)
		/// 3) Process any deferred event proofs that were submitted while the bridge was paused (should only happen on the first few blocks in a new era) (outgoing)
		fn on_initialize(block_number: T::BlockNumber) -> Weight {
			let mut consumed_weight = 0 as Weight;

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
					// We are still waiting on the challenge to be processed, push out by challenge period
					let new_process_at = block_number + Self::challenge_period();
					<MessagesValidAt<T>>::append(
						new_process_at,
						message_id,
					);
					Self::deposit_event(Event::<T>::ProcessAtExtended(message_id, new_process_at));
					continue
				}
				// Removed PendingEventClaim from storage and processes
				if let Some(EventClaim { source, destination, data, .. } ) = PendingEventClaims::take(message_id) {
					// keep a runtime hardcoded list of destination <> palletId
					match T::EventRouter::route(&source, &destination, &data) {
						Ok(weight) => {
							consumed_weight += weight;
							Self::deposit_event(Event::<T>::ProcessingOk(message_id));
						},
						Err((weight, err)) => {
							consumed_weight += weight;
							Self::deposit_event(Event::<T>::ProcessingFailed(message_id, err));
						}
					}
				}
				// mark as processed
				if let Err(idx) = processed_message_ids.binary_search(&message_id) {
					processed_message_ids.insert(idx, message_id);
				}
				// Tidy up status check
				PendingClaimStatus::remove(message_id);
			}
			if !processed_message_ids.is_empty() {
				impls::prune_claim_ids(&mut processed_message_ids);
				ProcessedMessageIds::put(processed_message_ids);
			}

			// 3) Try process delayed proofs
			consumed_weight += DbWeight::get().reads(2 as Weight);
			if PendingEventProofs::iter().next().is_some() && !Self::bridge_paused() {
				let max_delayed_events = Self::delayed_event_proofs_per_block();
				consumed_weight = consumed_weight.saturating_add(DbWeight::get().reads(1 as Weight) + max_delayed_events as Weight * DbWeight::get().writes(2 as Weight));
				for (event_proof_id, signing_request) in PendingEventProofs::iter().take(max_delayed_events as usize) {
					Self::do_request_event_proof(event_proof_id, signing_request);
					PendingEventProofs::remove(event_proof_id);
				}
			}

			consumed_weight
		}

		#[weight = DbWeight::get().writes(1)]
		/// Set the relayer address
		pub fn set_relayer(origin, relayer: T::AccountId) {
			ensure_root(origin)?;
			// Ensure relayer has bonded more than relayer bond amount
			ensure!(Self::relayer_paid_bond(relayer) >= T::RelayerBond::get(), Error::<T>::NoBondPaid);
			<Relayer<T>>::put(relayer);
			Self::deposit_event(Event::<T>::RelayerSet(Some(relayer)));
		}

		#[weight = DbWeight::get().writes(1)]
		/// Submit bond for relayer account
		pub fn deposit_relayer_bond(origin) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			// Ensure relayer doesn't already have a bond set
			ensure!(Self::relayer_paid_bond(origin) == 0, Error::<T>::CantBondRelayer);

			let relayer_bond = T::RelayerBond::get();
			// Attempt to place a hold from the relayer account
			T::MultiCurrency::place_hold(
				T::BridgePalletId::get(),
				&origin,
				T::NativeAssetId::get(),
				relayer_bond,
			)?;
			<RelayerPaidBond<T>>::insert(origin, relayer_bond);
			Self::deposit_event(Event::<T>::RelayerBondDeposit(origin, relayer_bond));
			Ok(())
		}

		#[weight = DbWeight::get().writes(1)]
		/// Withdraw relayer bond amount
		pub fn withdraw_relayer_bond(origin) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			// Ensure account is not the current relayer
			if Self::relayer() == Some(origin) {
				ensure!(Self::relayer() != Some(origin), Error::<T>::CantUnbondRelayer);
			};
			let relayer_paid_bond = Self::relayer_paid_bond(origin);
			ensure!(relayer_paid_bond > 0, Error::<T>::CantUnbondRelayer);

			// Attempt to release the relayers hold
			T::MultiCurrency::release_hold(
				T::BridgePalletId::get(),
				&origin,
				T::NativeAssetId::get(),
				relayer_paid_bond,
			)?;
			<RelayerPaidBond<T>>::remove(origin);

			Self::deposit_event(Event::<T>::RelayerBondWithdraw(origin, relayer_paid_bond));
			Ok(())
		}

		#[weight = DbWeight::get().writes(1)]
		/// Set event confirmations (blocks). Required block confirmations for an Ethereum event to be notarized by Seed
		pub fn set_event_block_confirmations(origin, confirmations: u64) {
			ensure_root(origin)?;
			EventBlockConfirmations::put(confirmations)
		}

		#[weight = DbWeight::get().writes(1)]
		/// Set max number of delayed events that can be processed per block
		pub fn set_delayed_event_proofs_per_block(origin, count: u8) {
			ensure_root(origin)?;
			DelayedEventProofsPerBlock::put(count);
		}

		#[weight = DbWeight::get().writes(1)]
		/// Set challenge period, this is the window in which an event can be challenged before processing
		pub fn set_challenge_period(origin, blocks: T::BlockNumber) {
			ensure_root(origin)?;
			<ChallengePeriod<T>>::put(blocks);
		}

		#[weight = DbWeight::get().writes(1)]
		/// Set the bridge contract address on Ethereum (requires governance)
		pub fn set_contract_address(origin, contract_address: EthAddress) {
			ensure_root(origin)?;
			ContractAddress::put(contract_address);
			Self::deposit_event(<Event<T>>::SetContractAddress(contract_address));
		}

		#[weight = DbWeight::get().writes(1)]
		/// Submit ABI encoded event data from the Ethereum bridge contract
		/// - tx_hash The Ethereum transaction hash which triggered the event
		/// - event ABI encoded bridge event
		pub fn submit_event(origin, tx_hash: H256, event: Vec<u8>) {
			let origin = ensure_signed(origin)?;

			ensure!(Some(origin) == Self::relayer(), Error::<T>::NoPermission);

			// TODO: place some limit on `data` length (it should match on contract side)
			// Message(event_id, msg.caller, destination, data);
			if let [Token::Uint(event_id), Token::Address(source), Token::Address(destination), Token::Bytes(data), Token::Uint(fee)] = ethabi::decode(&[
				ParamType::Uint(64),
				ParamType::Address,
				ParamType::Address,
				ethabi::ParamType::Bytes,
				ParamType::Uint(64),
			], event.as_slice()).map_err(|_| Error::<T>::InvalidClaim)?.as_slice() {
				let event_id: EventClaimId = (*event_id).saturated_into();
				ensure!(!PendingEventClaims::contains_key(event_id), Error::<T>::EventReplayPending); // NOTE(surangap): prune PendingEventClaims also?
				if !Self::processed_message_ids().is_empty() {
					ensure!( event_id > Self::processed_message_ids()[0] &&
						Self::processed_message_ids().binary_search(&event_id).is_err() , Error::<T>::EventReplayProcessed);
				}
				let event_claim = EventClaim {
					tx_hash,
					source: *source,
					destination: *destination,
					data: data.clone(),
				};

				PendingEventClaims::insert(event_id, &event_claim);
				PendingClaimStatus::insert(event_id, EventClaimStatus::Pending);

				// TODO: there should be some limit per block
				let process_at: T::BlockNumber = <frame_system::Pallet<T>>::block_number() + Self::challenge_period();
				<MessagesValidAt<T>>::append(process_at, event_id);

				Self::deposit_event(Event::<T>::EventSubmit(event_id, event_claim, process_at));
			}
		}

		#[weight = DbWeight::get().writes(1) + DbWeight::get().reads(2)]
		/// Submit a challenge for an event
		/// Challenged events won't be processed until verified by validators
		/// An event can only be challenged once
		pub fn submit_challenge(origin, event_claim_id: EventClaimId) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			// Validate event_id existence
			ensure!(PendingEventClaims::contains_key(event_claim_id), Error::<T>::NoClaim);
			// Check that event isn't already being challenged
			ensure!(Self::pending_claim_status(event_claim_id) == Some(EventClaimStatus::Pending), Error::<T>::ClaimAlreadyChallenged);

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
			PendingClaimChallenges::append(event_claim_id);
			<ChallengerAccount<T>>::insert(event_claim_id, (origin, challenger_bond));
			PendingClaimStatus::insert(event_claim_id, EventClaimStatus::Challenged);

			Self::deposit_event(Event::<T>::Challenged(event_claim_id, origin));
			Ok(())
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
