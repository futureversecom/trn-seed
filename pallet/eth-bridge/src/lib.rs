/* Copyright 2021 Centrality Investments Limited
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

mod ethereum_http_cli;
mod types;
use crate::types::{
	BridgeEthereumRpcApi, CheckedEthCallResult, EthBlock, EthCallId, EventClaim, EventClaimResult,
	EventClaimStatus, LatestOrNumber, NotarizationPayload,
};
use codec::Encode;
use ethabi::{ParamType, Token};
pub use ethereum_http_cli::EthereumRpcClient;
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	pallet_prelude::*,
	traits::{fungibles::Transfer, Get, ValidatorSet as ValidatorSetT},
	transactional,
	weights::Weight,
	PalletId,
};
use frame_system::{
	ensure_signed,
	offchain::{CreateSignedTransaction, SubmitTransaction},
	pallet_prelude::*,
};
use hex_literal::hex;
use log::{debug, error, info};
pub use pallet::*;
use seed_pallet_common::{
	eth::EthereumEventInfo,
	ethy::{BridgeAdapter, EthereumBridgeAdapter, EthyAdapter, EthySigningRequest, State::Paused},
	log,
	validator_set::ValidatorSetInterface,
	EthereumBridge, EthereumEventRouter, EventRouterError, Hold,
};
use seed_primitives::{
	ethy::{crypto::AuthorityId, EventClaimId, EventProofId},
	AccountId, AssetId, Balance, EthAddress,
};
use sp_core::{H160, H256};
use sp_runtime::{
	offchain as rt_offchain, traits::SaturatedConversion, DispatchError, Percent, RuntimeAppPublic,
};
use sp_std::vec::Vec;

/// Max notarization claims to attempt per block/OCW invocation
const CLAIMS_PER_BLOCK: usize = 1;
/// The logging target for this pallet
pub(crate) const LOG_TARGET: &str = "eth-bridge";
/// The solidity selector of bridge events
/// i.e. output of `keccak256('SubmitEvent(address,address,bytes)')` /
/// `0f8885c9654c5901d61d2eae1fa5d11a67f9b8fca77146d5109bc7be00f4472a`
const SUBMIT_BRIDGE_EVENT_SELECTOR: [u8; 32] =
	hex!("0f8885c9654c5901d61d2eae1fa5d11a67f9b8fca77146d5109bc7be00f4472a");

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config<AccountId = AccountId> + CreateSignedTransaction<Call<Self>>
	{
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// Bond required for an account to act as relayer
		type RelayerBond: Get<Balance>;
		/// The native token asset Id (managed by pallet-balances)
		type NativeAssetId: Get<AssetId>;
		/// Handles a multi-currency fungible asset system
		type MultiCurrency: Transfer<Self::AccountId> + Hold<AccountId = Self::AccountId>;
		/// Bond required by challenger to make a challenge
		type ChallengeBond: Get<Balance>;
		/// Validator set Adapter
		type ValidatorSet: ValidatorSetInterface<AuthorityId>;
		/// Ethy Adapter
		type EthyAdapter: EthyAdapter;
		/// The threshold of notarizations required to approve an Ethereum event
		type NotarizationThreshold: Get<Percent>;
		/// Knows the active authority set (validator stash addresses)
		type AuthoritySet: ValidatorSetT<Self::AccountId, ValidatorId = Self::AccountId>;
		/// Handles routing received Ethereum events upon verification
		type EventRouter: EthereumEventRouter;
		/// Provides an api for Ethereum JSON-RPC request/responses to the bridged ethereum network
		type RpcClient: BridgeEthereumRpcApi;
		/// The runtime call type.
		type Call: From<Call<Self>>;
	}

	/// Map from relayer account to their paid bond amount
	#[pallet::storage]
	#[pallet::getter(fn relayer_bond)]
	pub type RelayerBond<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Balance, ValueQuery>;

	/// The permissioned relayer
	#[pallet::storage]
	#[pallet::getter(fn relayer)]
	pub type Relayer<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// The minimum number of block confirmations needed to notarize an Ethereum event
	#[pallet::type_value]
	pub fn DefaultEventBlockConfirmations() -> u64 {
		3u64
	}
	#[pallet::storage]
	#[pallet::getter(fn event_block_confirmations)]
	pub type EventBlockConfirmations<T> =
		StorageValue<_, u64, ValueQuery, DefaultEventBlockConfirmations>;

	/// The (optimistic) challenge period after which a submitted event is considered valid
	#[pallet::type_value]
	pub fn DefaultChallengePeriod<T: Config>() -> T::BlockNumber {
		T::BlockNumber::from(150u32) // block time (4s) * 150 = 10 Minutes
	}
	#[pallet::storage]
	#[pallet::getter(fn challenge_period)]
	pub type ChallengePeriod<T: Config> =
		StorageValue<_, T::BlockNumber, ValueQuery, DefaultChallengePeriod<T>>;

	/// The bridge contract address on Ethereum
	#[pallet::storage]
	#[pallet::getter(fn contract_address)]
	pub type ContractAddress<T> = StorageValue<_, EthAddress, ValueQuery>;

	/// Queued event claims, can be challenged within challenge period
	#[pallet::storage]
	#[pallet::getter(fn pending_event_claims)]
	pub type PendingEventClaims<T> =
		StorageMap<_, Twox64Concat, EventClaimId, EventClaim, OptionQuery>;

	/// Tracks processed message Ids (prevent replay)
	#[pallet::storage]
	#[pallet::getter(fn processed_message_ids)]
	pub type ProcessedMessageIds<T> = StorageValue<_, Vec<EventClaimId>, ValueQuery>;

	/// Status of pending event claims
	#[pallet::storage]
	#[pallet::getter(fn pending_claim_status)]
	pub type PendingClaimStatus<T> =
		StorageMap<_, Twox64Concat, EventClaimId, EventClaimStatus, OptionQuery>;

	/// Map from block number to list of EventClaims that will be considered valid and should be
	/// forwarded to handlers (i.e after the optimistic challenge period has passed without issue)
	#[pallet::storage]
	#[pallet::getter(fn messages_valid_at)]
	pub type MessagesValidAt<T: Config> =
		StorageMap<_, Twox64Concat, T::BlockNumber, Vec<EventClaimId>, ValueQuery>;

	/// List of all event ids that are currently being challenged
	#[pallet::storage]
	#[pallet::getter(fn pending_claim_challenges)]
	pub type PendingClaimChallenges<T> = StorageValue<_, Vec<EventClaimId>, ValueQuery>;

	/// Maps from event claim id to challenger and bond amount paid
	#[pallet::storage]
	#[pallet::getter(fn challenger_account)]
	pub type ChallengerAccount<T: Config> =
		StorageMap<_, Twox64Concat, EventClaimId, (T::AccountId, Balance), OptionQuery>;

	/// Notarizations for queued events
	/// Either: None = no notarization exists OR Some(yay/nay)
	#[pallet::storage]
	#[pallet::getter(fn event_notarizations)]
	pub type EventNotarizations<T> = StorageDoubleMap<
		_,
		Twox64Concat,
		EventClaimId,
		Twox64Concat,
		AuthorityId,
		EventClaimResult,
		OptionQuery,
	>;

	#[pallet::error]
	pub enum Error<T> {
		/// The relayer hasn't paid the relayer bond
		NoBondPaid,
		/// The relayer already has a bonded amount
		CantBondRelayer,
		/// The relayer is active and cant unbond the specified amount
		CantUnbondRelayer,
		/// Claim was invalid e.g. not properly ABI encoded
		InvalidClaim,
		/// Event was already submitted and is pending
		EventReplayPending,
		/// Event was already submitted and is complete
		EventReplayProcessed,
		/// Caller does not have permission for that action
		NoPermission,
		/// There is already a challenge for this claim
		ClaimAlreadyChallenged,
		/// There is no event claim associated with the supplied claim_id
		NoClaim,
		/// A notarization was invalid
		InvalidNotarization,
		/// Error returned when making unsigned transactions with signed payloads in off-chain
		/// worker
		OffchainUnsignedTxSignedPayload,
		/// Some internal operation failed
		Internal,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new relayer has been set
		RelayerSet { relayer_account: T::AccountId },
		/// A relayer has been removed
		RelayerRemoved { relayer_account: T::AccountId },
		/// An account has deposited a relayer bond
		RelayerBondDeposited { account_id: T::AccountId, amount: Balance },
		/// An account has withdrawn a relayer bond
		RelayerBondWithdrawn { account_id: T::AccountId, amount: Balance },
		/// The bridge contract address has been set
		ContractAddressSet { address: EthAddress },
		/// The block confirmations for an Ethereum event to be notarized by Seed has been set
		EventBlockConfirmationsSet { blocks: T::BlockNumber },
		/// Challenge period, the window in which an event can be challenged before processing has
		/// been set
		ChallengePeriodSet { blocks: T::BlockNumber },
		/// An event has been submitted from Ethereum (event_claim_id, event_claim, process_at)
		EventSubmit {
			event_claim_id: EventClaimId,
			event_claim: EventClaim,
			process_at: T::BlockNumber,
		},
		/// An event has been challenged (claim_id, challenger)
		Challenged { claim_id: EventClaimId, challenger: T::AccountId },
		/// The event is still awaiting consensus. Process block pushed out (claim_id, process_at)
		ProcessAtExtended { claim_id: EventClaimId, process_at: T::BlockNumber },
		/// Processing an event succeeded
		ProcessingOk { claim_id: EventClaimId },
		/// Processing an event failed
		ProcessingFailed { claim_id: EventClaimId, error: EventRouterError },
		/// Verifying an event succeeded
		EventVerified { claim_id: EventClaimId },
		/// Verifying an event failed
		EventInvalid { claim_id: EventClaimId },
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_number: T::BlockNumber) -> Weight {
			let mut consumed_weight = 0 as Weight;
			// process submitted eth events
			let mut processed_message_ids = Self::processed_message_ids();
			for message_id in MessagesValidAt::<T>::take(block_number) {
				if Self::pending_claim_status(message_id) == Some(EventClaimStatus::Challenged) {
					// We are still waiting on the challenge to be processed, push out by challenge
					// period
					let new_process_at = block_number + Self::challenge_period();
					MessagesValidAt::<T>::append(&new_process_at, message_id);
					Self::deposit_event(Event::<T>::ProcessAtExtended {
						claim_id: message_id,
						process_at: new_process_at,
					});
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
							Self::deposit_event(Event::<T>::ProcessingOk { claim_id: message_id });
						},
						Err((weight, err)) => {
							consumed_weight += weight;
							Self::deposit_event(Event::<T>::ProcessingFailed {
								claim_id: message_id,
								error: err,
							});
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
				Self::prune_claim_ids(&mut processed_message_ids);
				ProcessedMessageIds::<T>::put(processed_message_ids);
			}

			consumed_weight
		}

		fn offchain_worker(_block_number: T::BlockNumber) {
			let validator_set = T::ValidatorSet::get_validator_set();
			// this passes if flag `--validator` set, not necessarily in the active set
			if !sp_io::offchain::is_validator() {
				debug!(target: LOG_TARGET, "Not a validator, exiting");
				return
			}

			// check a local key exists for a valid bridge notary
			let Some((active_key, authority_index)) = Self::find_active_ethy_key(&validator_set) else {
				debug!(target: LOG_TARGET, "Not an active validator, exiting");
				return
			};

			// check enough validators have active notary keys
			let supports = validator_set.len();
			let needed = T::NotarizationThreshold::get();
			// TODO: check every session change not block
			if Percent::from_rational(supports, T::AuthoritySet::validators().len()) < needed {
				info!(
					target: LOG_TARGET,
					"Waiting for validator support to activate eth-bridge: {:?}/{:?}",
					supports,
					needed
				);
				return
			}
			// do some notarizing
			Self::do_event_notarization_ocw(&active_key, authority_index);
			// TODO(surangap) - check if we need this, seems it's not being used
			// Self::do_call_notarization_ocw(&active_key, authority_index);
			debug!(target: LOG_TARGET, "Exiting off-chain worker");
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<sp_core::H160> + Into<sp_core::H160>,
	{
		/// Set the relayer address
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn set_relayer(origin: OriginFor<T>, relayer: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;
			// Ensure relayer has bonded more than relayer bond amount
			ensure!(Self::relayer_bond(&relayer) >= T::RelayerBond::get(), Error::<T>::NoBondPaid);
			Relayer::<T>::put(&relayer);
			info!(target: LOG_TARGET, "relayer set. Account Id: {:?}", relayer);
			Self::deposit_event(Event::<T>::RelayerSet { relayer_account: relayer });
			Ok(())
		}

		/// Submit bond for relayer account
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn deposit_relayer_bond(origin: OriginFor<T>) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			// Ensure relayer doesn't already have a bond set
			ensure!(Self::relayer_bond(&origin) == 0, Error::<T>::CantBondRelayer);

			let relayer_bond = T::RelayerBond::get();
			// Attempt to place a hold from the relayer account
			T::MultiCurrency::place_hold(
				T::PalletId::get(),
				&origin,
				T::NativeAssetId::get(),
				relayer_bond,
			)?;
			RelayerBond::<T>::insert(&origin, relayer_bond);
			Self::deposit_event(Event::<T>::RelayerBondDeposited {
				account_id: origin,
				amount: relayer_bond,
			});
			Ok(())
		}

		/// Withdraw relayer bond amount
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn withdraw_relayer_bond(origin: OriginFor<T>) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			// Ensure account is not the current relayer
			if Self::relayer() == Some(origin.clone()) {
				// spk - check this logic, can be simplified?
				ensure!(Self::relayer() != Some(origin.clone()), Error::<T>::CantUnbondRelayer);
			};
			let relayer_bond = Self::relayer_bond(&origin);
			ensure!(relayer_bond > 0, Error::<T>::CantUnbondRelayer);

			// Attempt to release the relayers hold
			T::MultiCurrency::release_hold(
				T::PalletId::get(),
				&origin,
				T::NativeAssetId::get(),
				relayer_bond,
			)?;
			RelayerBond::<T>::remove(&origin);

			Self::deposit_event(Event::<T>::RelayerBondWithdrawn {
				account_id: origin,
				amount: relayer_bond,
			});
			Ok(())
		}

		/// Set event confirmations (blocks). Required block confirmations for an Ethereum event to
		/// be notarized by Seed
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn set_event_block_confirmations(
			origin: OriginFor<T>,
			confirmations: u64,
		) -> DispatchResult {
			ensure_root(origin)?;
			EventBlockConfirmations::<T>::put(confirmations);
			Self::deposit_event(Event::<T>::EventBlockConfirmationsSet {
				blocks: T::BlockNumber::from(confirmations as u32),
			});
			Ok(())
		}

		/// Set challenge period, this is the window in which an event can be challenged before
		/// processing
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn set_challenge_period(
			origin: OriginFor<T>,
			blocks: T::BlockNumber,
		) -> DispatchResult {
			ensure_root(origin)?;
			ChallengePeriod::<T>::put(blocks);
			Self::deposit_event(Event::<T>::ChallengePeriodSet { blocks });
			Ok(())
		}

		/// Set the bridge contract address on Ethereum (requires governance)
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn set_contract_address(
			origin: OriginFor<T>,
			contract_address: EthAddress,
		) -> DispatchResult {
			ensure_root(origin)?;
			ContractAddress::<T>::put(contract_address);
			Self::deposit_event(Event::<T>::ContractAddressSet { address: contract_address });
			Ok(())
		}

		/// Submit ABI encoded event data from the Ethereum bridge contract
		/// - tx_hash The Ethereum transaction hash which triggered the event
		/// - event ABI encoded bridge event
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn submit_event(origin: OriginFor<T>, tx_hash: H256, event: Vec<u8>) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			ensure!(Some(origin) == Self::relayer(), Error::<T>::NoPermission);

			// TODO: place some limit on `data` length (it should match on contract side)
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
				);
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
				MessagesValidAt::<T>::append(process_at, event_id);
				Self::deposit_event(Event::<T>::EventSubmit {
					event_claim_id: event_id,
					event_claim,
					process_at,
				});
			}
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		/// Submit a challenge for an event
		/// Challenged events won't be processed until verified by validators
		/// An event can only be challenged once
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
				T::PalletId::get(),
				&origin,
				T::NativeAssetId::get(),
				challenger_bond,
			)?;

			// Add event to challenged event storage
			// Not sorted so we can check using FIFO
			// Include challenger account for releasing funds in case claim is valid
			PendingClaimChallenges::<T>::append(event_claim_id);
			ChallengerAccount::<T>::insert(event_claim_id, (&origin, challenger_bond));
			PendingClaimStatus::<T>::insert(event_claim_id, EventClaimStatus::Challenged);

			Self::deposit_event(Event::<T>::Challenged {
				claim_id: event_claim_id,
				challenger: origin,
			});
			Ok(())
		}

		/// Internal only
		/// Validators will submit inherents with their notarization vote for a given claim
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		#[transactional]
		pub fn submit_notarization(
			origin: OriginFor<T>,
			payload: NotarizationPayload,
			_signature: <AuthorityId as RuntimeAppPublic>::Signature,
		) -> DispatchResult {
			let _ = ensure_none(origin)?;

			// we don't need to verify the signature here because it has been verified in
			// `validate_unsigned` function when sending out the unsigned tx.
			let authority_index = payload.authority_index() as usize;
			let notary_keys = T::ValidatorSet::get_validator_set();
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

impl<T: Config> Pallet<T> {
	/// Handle a submitted call notarization
	pub(crate) fn handle_call_notarization(
		_call_id: EthCallId,
		_result: CheckedEthCallResult,
		_notary_id: &AuthorityId,
	) -> Result<(), DispatchError> {
		Ok(())
	}

	/// Handle a submitted event notarization
	pub(crate) fn handle_event_notarization(
		event_claim_id: EventClaimId,
		result: EventClaimResult,
		notary_id: &AuthorityId,
	) -> Result<(), DispatchError> {
		ensure!(
			Self::pending_claim_status(event_claim_id) == Some(EventClaimStatus::Challenged),
			Error::<T>::InvalidClaim
		);

		// Store the new notarization
		<EventNotarizations<T>>::insert::<EventClaimId, AuthorityId, EventClaimResult>(
			event_claim_id,
			notary_id.clone(),
			result,
		);

		// Count notarization votes
		let notary_count = T::AuthoritySet::validators().len() as u32;
		let mut yay_count = 0u32;
		let mut nay_count = 0u32;
		// TODO: store the count
		for (_id, result) in <EventNotarizations<T>>::iter_prefix(event_claim_id) {
			match result {
				EventClaimResult::Valid => yay_count += 1,
				_ => nay_count += 1,
			}
		}

		// Claim is invalid (nays > (100% - NotarizationThreshold))
		if Percent::from_rational(nay_count, notary_count) >
			(Percent::from_parts(100u8 - T::NotarizationThreshold::get().deconstruct()))
		{
			Self::handle_invalid_claim(event_claim_id)?;
		}
		// Claim is valid
		if Percent::from_rational(yay_count, notary_count) >= T::NotarizationThreshold::get() {
			Self::handle_valid_claim(event_claim_id)?;
		}

		Ok(())
	}

	/// Check the nodes local keystore for an active (staked) Ethy session key
	/// Returns the public key and index of the key in the current notary set
	pub(crate) fn find_active_ethy_key(
		validator_set: &Vec<AuthorityId>,
	) -> Option<(AuthorityId, u16)> {
		// Get all signing keys for this protocol 'KeyTypeId'
		let local_keys = AuthorityId::all();
		if local_keys.is_empty() {
			error!(
				target: LOG_TARGET,
				"No signing keys for: {:?}, cannot participate in notarization!",
				AuthorityId::ID
			);
			return None
		};

		let mut maybe_active_key: Option<(AuthorityId, usize)> = None;
		// search all local ethy keys
		for key in local_keys {
			if let Some(active_key_index) = validator_set.iter().position(|k| k == &key) {
				maybe_active_key = Some((key, active_key_index));
				break
			}
		}

		// check if locally known keys are in the active validator set
		if maybe_active_key.is_none() {
			error!(target: LOG_TARGET, "No active ethy keys, exiting");
			return None
		}
		maybe_active_key.map(|(key, idx)| (key, idx as u16))
	}

	/// Handle OCW event notarization protocol for validators
	/// Receives the node's local notary session key and index in the set
	pub(crate) fn do_event_notarization_ocw(active_key: &AuthorityId, authority_index: u16) {
		// do not try to notarize events while the ethy is paused
		if T::EthyAdapter::get_ethy_state() == Paused {
			return
		}

		// check all pending claims we have yet to notarize and try to notarize them
		// this will be invoked once every block
		// we limit the total claims per invocation using `CLAIMS_PER_BLOCK` so we don't stall block
		// production.
		for event_claim_id in PendingClaimChallenges::<T>::get().iter().take(CLAIMS_PER_BLOCK) {
			let event_claim = Self::pending_event_claims(event_claim_id);
			if event_claim.is_none() {
				// This shouldn't happen
				error!(
					target: LOG_TARGET,
					"Notarization failed, event claim: {:?} not found", event_claim_id
				);
				continue
			};

			// skip if we've notarized it previously
			if EventNotarizations::<T>::contains_key::<EventClaimId, AuthorityId>(
				*event_claim_id,
				active_key.clone(),
			) {
				debug!(
					target: LOG_TARGET,
					"Already notarized claim: {:?}, ignoring...", event_claim_id
				);
				continue
			}

			let result = Self::offchain_try_notarize_event(*event_claim_id, event_claim.unwrap());
			debug!(target: LOG_TARGET, "Claim verification status: {:?}", &result);
			let payload = NotarizationPayload::Event {
				event_claim_id: *event_claim_id,
				authority_index,
				result: result.clone(),
			};
			let _ = Self::offchain_send_notarization(active_key, payload)
				.map_err(|err| {
					error!(target: LOG_TARGET, "Sending notarization failed 🙈, {:?}", err);
				})
				.map(|_| {
					info!(
						target: LOG_TARGET,
						"Sent notarization: '{:?}' for claim: {:?}", result, event_claim_id
					);
				});
		}
	}

	/// Prunes claim ids that are less than the max contiguous claim id.
	pub(crate) fn prune_claim_ids(claim_ids: &mut Vec<EventClaimId>) {
		// if < 1 element, nothing to do
		if claim_ids.len() <= 1 {
			return
		}
		// sort first
		claim_ids.sort();
		// get the index of the fist element that's non contiguous.
		let first_noncontinuous_idx = claim_ids.iter().enumerate().position(|(i, &x)| {
			if i > 0 {
				x != claim_ids[i - 1] + 1
			} else {
				false
			}
		});
		// drain the array from start to (first_noncontinuous_idx - 1) since we need the max
		// contiguous element in the pruned vector.
		match first_noncontinuous_idx {
			Some(idx) => claim_ids.drain(..idx - 1),
			None => claim_ids.drain(..claim_ids.len() - 1), // we need the last element to remain
		};
	}

	/// Verify a bridge message
	///
	/// `event_claim_id` - The event claim Id
	/// `event_claim` - The event claim info
	/// Checks:
	/// - check Eth full node for transaction status
	/// - tx success
	/// - tx sent to source contract address
	/// - check for exact log data match
	/// - check log source == bridge contract address
	/// - confirmations `>= T::EventConfirmations`
	///
	/// Returns result of the validation
	pub(crate) fn offchain_try_notarize_event(
		event_claim_id: EventClaimId,
		event_claim: EventClaim,
	) -> EventClaimResult {
		let EventClaim { tx_hash, data, source, destination } = event_claim;
		let result = T::RpcClient::get_transaction_receipt(tx_hash);
		let Ok(maybe_tx_receipt) = result else {
			error!(
				target: LOG_TARGET,
				"Eth getTransactionReceipt({:?}) failed: {:?}", tx_hash, result
			);
			return EventClaimResult::DataProviderErr
		};
		let Some(tx_receipt) = maybe_tx_receipt else {
			return EventClaimResult::NoTxReceipt;
		};
		let status = tx_receipt.status.unwrap_or_default();
		if status.is_zero() {
			return EventClaimResult::TxStatusFailed
		}

		// this may be overly restrictive
		// requires the transaction calls the source contract as the entrypoint or fails.
		// example 1: contract A -> bridge contract, ok
		// example 2: contract A -> contract B -> bridge contract, fails
		if tx_receipt.to != Some(source) {
			return EventClaimResult::UnexpectedSource
		}

		// search for a bridge deposit event in this tx receipt
		let matching_log = tx_receipt.logs.iter().find(|log| {
			log.transaction_hash == Some(tx_hash) &&
				log.topics.contains(&SUBMIT_BRIDGE_EVENT_SELECTOR.into())
		});

		let submitted_event_data = ethabi::encode(&[
			Token::Uint(event_claim_id.into()),
			Token::Address(source),
			Token::Address(destination),
			Token::Bytes(data),
		]);
		let Some(log) = matching_log else {
			return EventClaimResult::NoTxLogs
		};

		// check if the Ethereum event data matches what was reported
		// in the original claim
		if log.data != submitted_event_data {
			error!(
				target: LOG_TARGET,
				"Mismatch in provided data vs. observed data. provided: {:?} observed: {:?}",
				submitted_event_data,
				log.data,
			);
			return EventClaimResult::UnexpectedData
		}
		if log.address != Self::contract_address() {
			return EventClaimResult::UnexpectedContractAddress
		}

		//  have we got enough block confirmations to be re-org safe?
		let observed_block_number: u64 = tx_receipt.block_number.saturated_into();

		let latest_block: EthBlock = match T::RpcClient::get_block_by_number(LatestOrNumber::Latest)
		{
			Ok(None) => return EventClaimResult::DataProviderErr,
			Ok(Some(block)) => block,
			Err(err) => {
				error!(target: LOG_TARGET, "Eth getBlockByNumber latest failed: {:?}", err);
				return EventClaimResult::DataProviderErr
			},
		};

		let latest_block_number = latest_block.number.unwrap_or_default().as_u64();
		let block_confirmations = latest_block_number.saturating_sub(observed_block_number);
		if block_confirmations < Self::event_block_confirmations() {
			return EventClaimResult::NotEnoughConfirmations
		}

		EventClaimResult::Valid
	}

	/// Send a notarization for the given claim
	pub(crate) fn offchain_send_notarization(
		key: &AuthorityId,
		payload: NotarizationPayload,
	) -> Result<(), Error<T>> {
		let signature =
			key.sign(&payload.encode()).ok_or(<Error<T>>::OffchainUnsignedTxSignedPayload)?;

		let call = Call::submit_notarization { payload, signature };

		// Retrieve the signer to sign the payload
		SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
			.map_err(|_| <Error<T>>::OffchainUnsignedTxSignedPayload)
	}

	/// Handle claim after challenge has proven claim to be invalid
	/// Slash relayer and pay slashed amount to challenger
	/// repay challenger bond to challenger
	/// Remove the active relayer
	pub(crate) fn handle_invalid_claim(event_claim_id: EventClaimId) -> DispatchResult {
		if let Some(cursor) = <EventNotarizations<T>>::clear_prefix(
			event_claim_id,
			T::ValidatorSet::get_validator_set().len() as u32,
			None,
		)
		.maybe_cursor
		{
			log!(error, "💎 cleaning storage entries failed: {:?}", cursor);
			return Err(Error::<T>::Internal.into())
		}
		PendingClaimChallenges::<T>::mutate(|event_ids| {
			event_ids
				.iter()
				.position(|x| *x == event_claim_id)
				.map(|idx| event_ids.remove(idx));
		});

		let Some(_) = PendingEventClaims::<T>::take(event_claim_id) else {
			log!(error, "💎 unexpected empty claim");
			return Err(Error::<T>::InvalidClaim.into())
		};
		if let Some((challenger, bond_amount)) = <ChallengerAccount<T>>::take(event_claim_id) {
			// Challenger is correct, the event is invalid.
			// Return challenger bond to challenger and reward challenger with relayer bond
			T::MultiCurrency::release_hold(
				T::PalletId::get(),
				&challenger,
				T::NativeAssetId::get(),
				bond_amount,
			)?;

			if let Some(relayer) = Self::relayer() {
				// Relayer bond goes to challenger
				let relayer_bond = <RelayerBond<T>>::take(relayer);
				T::MultiCurrency::spend_hold(
					T::PalletId::get(),
					&relayer,
					T::NativeAssetId::get(),
					&[(challenger, relayer_bond)],
				)?;
				// Relayer has been slashed, remove their stored bond amount and set relayer to
				// None
				Self::deposit_event(Event::<T>::RelayerRemoved { relayer_account: relayer });
				<Relayer<T>>::kill();
			};

			PendingClaimStatus::<T>::remove(event_claim_id);
		} else {
			// This shouldn't happen
			log!(error, "💎 unexpected missing challenger account");
		}
		Self::deposit_event(Event::<T>::EventInvalid { claim_id: event_claim_id });
		return Ok(())
	}

	/// Handle claim after challenge has proven claim to be valid
	/// Pay challenger bond to relayer
	pub(crate) fn handle_valid_claim(event_claim_id: EventClaimId) -> DispatchResult {
		// no need to track info on this claim any more since it's approved
		if let Some(cursor) = <EventNotarizations<T>>::clear_prefix(
			event_claim_id,
			T::ValidatorSet::get_validator_set().len() as u32,
			None,
		)
		.maybe_cursor
		{
			log!(error, "💎 cleaning storage entries failed: {:?}", cursor);
			return Err(Error::<T>::Internal.into())
		}
		// Remove the claim from pending_claim_challenges
		PendingClaimChallenges::<T>::mutate(|event_ids| {
			event_ids
				.iter()
				.position(|x| *x == event_claim_id)
				.map(|idx| event_ids.remove(idx));
		});

		if !PendingEventClaims::<T>::contains_key(event_claim_id) {
			log!(error, "💎 unexpected empty claim");
			return Err(Error::<T>::InvalidClaim.into())
		}
		if let Some(relayer) = Self::relayer() {
			if let Some((challenger, bond_amount)) = <ChallengerAccount<T>>::take(event_claim_id) {
				// Challenger is incorrect, the event is valid. Send funds to relayer
				T::MultiCurrency::spend_hold(
					T::PalletId::get(),
					&challenger,
					T::NativeAssetId::get(),
					&[(relayer, bond_amount)],
				)?;
			} else {
				// This shouldn't happen
				log!(error, "💎 unexpected missing challenger account");
			}

			PendingClaimStatus::<T>::insert(event_claim_id, EventClaimStatus::ProvenValid);
			Self::deposit_event(Event::<T>::EventVerified { claim_id: event_claim_id });
		} else {
			log!(error, "💎 No relayer set");
		}

		Ok(())
	}
}

impl<T: Config> BridgeAdapter for Pallet<T> {
	fn get_pallet_id() -> PalletId {
		T::PalletId::get()
	}
}

impl<T: Config> EthereumBridgeAdapter for Pallet<T> {
	fn get_contract_address() -> EthAddress {
		ContractAddress::<T>::get()
	}

	fn get_notarization_threshold() -> Percent {
		T::NotarizationThreshold::get()
	}
}

impl<T: Config> EthereumBridge for Pallet<T> {
	/// Send an event via the bridge
	///  A proof of the event will be generated by notaries (async)
	///
	/// Returns an Id for the proof
	fn send_event(
		source: &H160,
		destination: &H160,
		app_event: &[u8],
	) -> Result<EventProofId, DispatchError> {
		let event_proof_id = T::EthyAdapter::get_next_event_proof_id();

		let event_proof_info = EthereumEventInfo {
			source: *source,
			destination: *destination,
			message: app_event.to_vec(),
			validator_set_id: T::ValidatorSet::get_validator_set_id(),
			event_proof_id,
		};

		T::EthyAdapter::request_for_proof(
			EthySigningRequest::Ethereum(event_proof_info),
			Some(event_proof_id),
		)?;
		Ok(event_proof_id)
	}
}
