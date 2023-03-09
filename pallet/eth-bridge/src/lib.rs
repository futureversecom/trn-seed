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

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod eth_rpc_client;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod types;
mod weights;
pub use weights::WeightInfo;

use crate::types::{
	BridgeEthereumRpcApi, CheckedEthCallRequest, CheckedEthCallResult, EthBlock, EthCallId,
	EventClaim, EventClaimResult, EventClaimStatus, LatestOrNumber, NotarizationPayload,
};
use codec::Encode;
pub use eth_rpc_client::EthereumRpcClient;
use ethabi::{ParamType, Token};
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	pallet_prelude::*,
	traits::{
		fungibles::{Mutate, Transfer},
		Get, UnixTime, ValidatorSet as ValidatorSetT,
	},
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
use log::{debug, error, info, trace};
pub use pallet::*;
use seed_pallet_common::{
	eth::EthereumEventInfo,
	ethy::{BridgeAdapter, EthereumBridgeAdapter, EthyAdapter, EthySigningRequest, State::Paused},
	validator_set::ValidatorSetAdapter,
	CreateExt, EthCallFailure, EthCallOracle, EthCallOracleSubscriber, EthereumBridge,
	EthereumEventRouter, EventRouterError, Hold,
};
use seed_primitives::{
	ethy::{crypto::AuthorityId, EventClaimId, EventProofId},
	AccountId, AssetId, Balance, EthAddress,
};
use sp_core::{H160, H256};
use sp_runtime::{
	offchain as rt_offchain, traits::SaturatedConversion, DispatchError, Percent, RuntimeAppPublic,
};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

/// The type to sign and send transactions.
const UNSIGNED_TXS_PRIORITY: u64 = 100;
/// Max notarization claims to attempt per block/OCW invocation
const CLAIMS_PER_BLOCK: usize = 1;
/// Max eth_call checks to attempt per block/OCW invocation
const CALLS_PER_BLOCK: usize = 1;
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
		type MultiCurrency: Transfer<Self::AccountId>
			+ Hold<AccountId = Self::AccountId>
			+ CreateExt<AccountId = Self::AccountId>
			+ Mutate<Self::AccountId, AssetId = AssetId>;
		/// Bond required by challenger to make a challenge
		type ChallengeBond: Get<Balance>;
		/// Validator set Adapter
		type ValidatorSet: ValidatorSetAdapter<AuthorityId>;
		/// Ethy Adapter
		type EthyAdapter: EthyAdapter;
		/// The threshold of notarizations required to approve an Ethereum event
		type NotarizationThreshold: Get<Percent>;
		/// Knows the active authority set (validator stash addresses)
		type AuthoritySet: ValidatorSetT<Self::AccountId, ValidatorId = Self::AccountId>;
		/// Handles routing received Ethereum events upon verification
		type EventRouter: EthereumEventRouter;
		/// Pallet subscribing to of notarized eth calls
		type EthCallSubscribers: EthCallOracleSubscriber<CallId = EthCallId>;
		/// Provides an api for Ethereum JSON-RPC request/responses to the bridged ethereum network
		type RpcClient: BridgeEthereumRpcApi;
		/// The runtime call type.
		type Call: From<Call<Self>>;
		/// Returns the block timestamp
		type UnixTime: UnixTime;
		type WeightInfo: WeightInfo;
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
		T::BlockNumber::from(150_u32) // block time (4s) * 150 = 10 Minutes
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

	/// Subscription Id for EthCall requests
	#[pallet::storage]
	pub type NextEthCallId<T> = StorageValue<_, EthCallId, ValueQuery>;

	/// Queue of pending EthCallOracle requests
	#[pallet::storage]
	#[pallet::getter(fn eth_call_requests)]
	pub type EthCallRequests<T> = StorageValue<_, Vec<EthCallId>, ValueQuery>;

	/// EthCallOracle notarizations keyed by (Id, Notary)
	#[pallet::storage]
	pub type EthCallNotarizations<T> = StorageDoubleMap<
		_,
		Twox64Concat,
		EthCallId,
		Twox64Concat,
		AuthorityId,
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
			Self::do_call_notarization_ocw(&active_key, authority_index);
			debug!(target: LOG_TARGET, "Exiting off-chain worker");
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<sp_core::H160> + Into<sp_core::H160>,
	{
		/// Set the relayer address
		#[pallet::weight(T::WeightInfo::set_relayer())]
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
		#[pallet::weight(T::WeightInfo::deposit_relayer_bond())]
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
		#[pallet::weight(T::WeightInfo::withdraw_relayer_bond())]
		pub fn withdraw_relayer_bond(origin: OriginFor<T>) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			// Ensure account is not the current relayer
			ensure!(Self::relayer() != Some(origin.clone()), Error::<T>::CantUnbondRelayer);
			// relayer_bond should be > 0
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
		#[pallet::weight(T::WeightInfo::set_event_block_confirmations())]
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
		#[pallet::weight(T::WeightInfo::set_challenge_period())]
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
		#[pallet::weight(T::WeightInfo::set_contract_address())]
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
		#[pallet::weight(T::WeightInfo::submit_event())]
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

		#[pallet::weight(T::WeightInfo::submit_challenge())]
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

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			let Call::submit_notarization { payload, signature } = call else {
				return InvalidTransaction::Call.into()
			};
			// notarization must be from an active notary
			let notary_keys = T::ValidatorSet::get_validator_set();
			let Some(notary_public_key) =  notary_keys.get(payload.authority_index() as usize) else  {
				return InvalidTransaction::BadProof.into();
			};
			// notarization must not be a duplicate/equivocation
			if <EventNotarizations<T>>::contains_key(payload.payload_id(), &notary_public_key) {
				error!(
					target: LOG_TARGET,
					"💎 received equivocation from: {:?} on {:?}",
					notary_public_key,
					payload.payload_id()
				);
				return InvalidTransaction::BadProof.into()
			}
			// notarization is signed correctly
			if !(notary_public_key.verify(&payload.encode(), signature)) {
				return InvalidTransaction::BadProof.into()
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
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Handle a submitted call notarization
	pub(crate) fn handle_call_notarization(
		call_id: EthCallId,
		result: CheckedEthCallResult,
		notary_id: &AuthorityId,
	) -> Result<(), DispatchError> {
		if !EthCallRequestInfo::<T>::contains_key(call_id) {
			// there's no claim active
			return Err(Error::<T>::InvalidClaim.into())
		}

		// Record the notarization (ensures the validator won't resubmit it)
		<EthCallNotarizations<T>>::insert::<EventClaimId, AuthorityId, CheckedEthCallResult>(
			call_id,
			notary_id.clone(),
			result,
		);

		// notify subscribers of a notarized eth_call outcome and clean upstate
		let do_callback_and_clean_up = |result: CheckedEthCallResult| {
			match result {
				CheckedEthCallResult::Ok(return_data, block, timestamp) =>
					T::EthCallSubscribers::on_eth_call_complete(
						call_id,
						&return_data,
						block,
						timestamp,
					),
				CheckedEthCallResult::ReturnDataEmpty => T::EthCallSubscribers::on_eth_call_failed(
					call_id,
					EthCallFailure::ReturnDataEmpty,
				),
				CheckedEthCallResult::ReturnDataExceedsLimit =>
					T::EthCallSubscribers::on_eth_call_failed(
						call_id,
						EthCallFailure::ReturnDataExceedsLimit,
					),
				_ => T::EthCallSubscribers::on_eth_call_failed(call_id, EthCallFailure::Internal),
			}
			if let Some(cursor) = <EthCallNotarizations<T>>::clear_prefix(
				call_id,
				T::ValidatorSet::get_validator_set().len() as u32,
				None,
			)
			.maybe_cursor
			{
				error!(target: LOG_TARGET, "cleaning storage entries failed: {:?}", cursor);
				return Err(Error::<T>::Internal.into())
			};
			EthCallNotarizationsAggregated::<T>::remove(call_id);
			EthCallRequestInfo::<T>::remove(call_id);
			EthCallRequests::<T>::mutate(|requests| {
				requests.iter().position(|x| *x == call_id).map(|idx| requests.remove(idx));
			});

			Ok(())
		};

		let mut notarizations =
			EthCallNotarizationsAggregated::<T>::get(call_id).unwrap_or_default();
		// increment notarization count for this result
		*notarizations.entry(result).or_insert(0) += 1;

		let notary_count = T::AuthoritySet::validators().len() as u32;
		let notarization_threshold = T::NotarizationThreshold::get();
		let mut total_count = 0;
		for (result, count) in notarizations.iter() {
			// is there consensus on `result`?
			if Percent::from_rational(*count, notary_count) >= notarization_threshold {
				return do_callback_and_clean_up(*result)
			}
			total_count += count;
		}

		let outstanding_count = notary_count.saturating_sub(total_count);
		let can_reach_consensus = notarizations.iter().any(|(_, count)| {
			Percent::from_rational(count + outstanding_count, notary_count) >=
				notarization_threshold
		});
		// cannot or will not reach consensus based on current notarizations
		if total_count == notary_count || !can_reach_consensus {
			return do_callback_and_clean_up(result)
		}

		// update counts
		EthCallNotarizationsAggregated::<T>::insert(call_id, notarizations);
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

	/// Handle OCW eth call checking protocol for validators
	/// Receives the node's local notary session key and index in the set
	pub(crate) fn do_call_notarization_ocw(active_key: &AuthorityId, authority_index: u16) {
		// we limit the total claims per invocation using `CALLS_PER_BLOCK` so we don't stall block
		// production
		for call_id in EthCallRequests::<T>::get().iter().take(CALLS_PER_BLOCK) {
			// skip if we've notarized it previously
			if EthCallNotarizations::<T>::contains_key::<EthCallId, AuthorityId>(
				*call_id,
				active_key.clone(),
			) {
				trace!(target: LOG_TARGET, "already notarized call: {:?}, ignoring...", call_id);
				continue
			}

			if let Some(request) = Self::eth_call_request_info(call_id) {
				let result = Self::offchain_try_eth_call(&request);
				trace!(target: LOG_TARGET, "checked call status: {:?}", &result);
				let payload =
					NotarizationPayload::Call { call_id: *call_id, authority_index, result };
				let _ = Self::offchain_send_notarization(active_key, payload)
					.map_err(|err| {
						error!(target: LOG_TARGET, "sending notarization failed 🙈, {:?}", err);
					})
					.map(|_| {
						info!(
							target: LOG_TARGET,
							"sent notarization: '{:?}' for call: {:?}", result, call_id,
						);
					});
			} else {
				// should not happen
				error!(target: LOG_TARGET, "empty call for: {:?}", call_id);
			}
		}
	}

	/// Performs an `eth_call` request to the bridged ethereum network
	///
	/// The call will be executed at `try_block_number` if it is within `max_block_look_behind`
	/// blocks of the latest ethereum block, otherwise the call is executed at the latest ethereum
	/// block.
	///
	/// `request` - details of the `eth_call` request to perform
	/// `try_block_number` - a block number to try the call at `latest - max_block_look_behind <= t
	/// < latest` `max_block_look_behind` - max ethereum blocks to look back from head
	pub(crate) fn offchain_try_eth_call(request: &CheckedEthCallRequest) -> CheckedEthCallResult {
		// OCW has 1 block to do all its stuff, so needs to be kept light
		//
		// basic flow of this function:
		// 1) get latest ethereum block
		// 2) check relayed block # and timestamp is within acceptable range (based on
		// `max_block_look_behind`) 3a) within range: do an eth_call at the relayed block
		// 3b) out of range: do an eth_call at block number latest
		let latest_block: EthBlock = match T::RpcClient::get_block_by_number(LatestOrNumber::Latest)
		{
			Ok(None) => return CheckedEthCallResult::DataProviderErr,
			Ok(Some(block)) => block,
			Err(err) => {
				error!(target: LOG_TARGET, "eth_getBlockByNumber latest failed: {:?}", err);
				return CheckedEthCallResult::DataProviderErr
			},
		};
		// some future proofing/protections if timestamps or block numbers are de-synced, stuck, or
		// missing this protocol should vote to abort
		let latest_eth_block_timestamp: u64 = latest_block.timestamp.saturated_into();
		if latest_eth_block_timestamp == u64::max_value() {
			return CheckedEthCallResult::InvalidTimestamp
		}
		// latest ethereum block timestamp should be after the request
		if latest_eth_block_timestamp < request.timestamp {
			return CheckedEthCallResult::InvalidTimestamp
		}
		let latest_eth_block_number = match latest_block.number {
			Some(number) => {
				if number.is_zero() || number.low_u64() == u64::max_value() {
					return CheckedEthCallResult::InvalidEthBlock
				}
				number.low_u64()
			},
			None => return CheckedEthCallResult::InvalidEthBlock,
		};

		// check relayed block # and timestamp is within acceptable range
		let mut target_block_number = latest_eth_block_number;
		let mut target_block_timestamp = latest_eth_block_timestamp;

		// there can be delay between challenge submission and execution
		// this should be factored into the acceptable block window, in normal conditions is should
		// be < 5s
		let check_delay = T::UnixTime::now().as_secs().saturating_sub(request.check_timestamp);
		let extra_look_behind = check_delay / 12_u64; // lenient here, any delay >= 12s gets an extra block

		let oldest_acceptable_eth_block = latest_eth_block_number
			.saturating_sub(request.max_block_look_behind)
			.saturating_sub(extra_look_behind);

		if request.try_block_number >= oldest_acceptable_eth_block &&
			request.try_block_number < latest_eth_block_number
		{
			let target_block: EthBlock = match T::RpcClient::get_block_by_number(
				LatestOrNumber::Number(request.try_block_number),
			) {
				Ok(None) => return CheckedEthCallResult::DataProviderErr,
				Ok(Some(block)) => block,
				Err(err) => {
					error!(target: LOG_TARGET, "eth_getBlockByNumber latest failed: {:?}", err);
					return CheckedEthCallResult::DataProviderErr
				},
			};
			target_block_number = request.try_block_number;
			target_block_timestamp = target_block.timestamp.saturated_into();
		}

		let return_data = match T::RpcClient::eth_call(
			request.target,
			&request.input,
			LatestOrNumber::Number(target_block_number),
		) {
			Ok(data) =>
				if data.is_empty() {
					return CheckedEthCallResult::ReturnDataEmpty
				} else {
					data
				},
			Err(err) => {
				error!(
					target: LOG_TARGET,
					"eth_call at: {:?}, failed: {:?}", target_block_number, err
				);
				return CheckedEthCallResult::DataProviderErr
			},
		};

		// valid returndata is ethereum abi encoded and therefore always >= 32 bytes
		match return_data.try_into() {
			Ok(r) => CheckedEthCallResult::Ok(r, target_block_number, target_block_timestamp),
			Err(_) => CheckedEthCallResult::ReturnDataExceedsLimit,
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
			error!(target: LOG_TARGET, "💎 cleaning storage entries failed: {:?}", cursor);
			return Err(Error::<T>::Internal.into())
		}
		PendingClaimChallenges::<T>::mutate(|event_ids| {
			event_ids
				.iter()
				.position(|x| *x == event_claim_id)
				.map(|idx| event_ids.remove(idx));
		});

		let Some(_) = PendingEventClaims::<T>::take(event_claim_id) else {
			error!(target: LOG_TARGET, "💎 unexpected empty claim");
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
			error!(target: LOG_TARGET, "💎 unexpected missing challenger account");
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
			error!(target: LOG_TARGET, "💎 cleaning storage entries failed: {:?}", cursor);
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
			error!(target: LOG_TARGET, "💎 unexpected empty claim");
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
				error!(target: LOG_TARGET, "💎 unexpected missing challenger account");
			}

			PendingClaimStatus::<T>::insert(event_claim_id, EventClaimStatus::ProvenValid);
			Self::deposit_event(Event::<T>::EventVerified { claim_id: event_claim_id });
		} else {
			error!(target: LOG_TARGET, "💎 No relayer set");
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

impl<T: Config> EthCallOracle for Pallet<T> {
	type Address = EthAddress;
	type CallId = EthCallId;
	/// Request an eth_call on some `target` contract with `input` on the bridged ethereum network
	/// Pre-checks are performed based on `max_block_look_behind` and `try_block_number`
	/// `timestamp` - cennznet timestamp of the request
	/// `try_block_number` - ethereum block number hint
	///
	/// Returns a call Id for subscribers
	fn checked_eth_call(
		target: &Self::Address,
		input: &[u8],
		timestamp: u64,
		try_block_number: u64,
		max_block_look_behind: u64,
	) -> Self::CallId {
		// store the job for validators to process async
		let call_id = NextEthCallId::<T>::get();
		EthCallRequestInfo::<T>::insert(
			call_id,
			CheckedEthCallRequest {
				check_timestamp: T::UnixTime::now().as_secs(),
				input: input.to_vec(),
				target: *target,
				timestamp,
				try_block_number,
				max_block_look_behind,
			},
		);
		EthCallRequests::<T>::append(call_id);
		NextEthCallId::<T>::put(call_id + 1);

		call_id
	}
}
