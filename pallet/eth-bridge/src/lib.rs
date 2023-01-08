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

mod types;

use codec::Encode;
use frame_support::{ensure, fail, traits::Get, weights::Weight, BoundedVec, PalletId};
pub use pallet::*;
use seed_pallet_common::{EthereumBridge, EthereumEventSubscriber};
use seed_primitives::{CollectionUuid, SerialNumber};
use sp_core::{H160, U256};
use sp_runtime::{traits::AccountIdConversion, DispatchError, SaturatedConversion};
use sp_std::{boxed::Box, vec, vec::Vec};
use ethabi::{ParamType, Token};
use frame_support::dispatch::DispatchResult;
use seed_primitives::ethy::crypto::AuthorityId;
use seed_primitives::ethy::EventClaimId;
use crate::types::{CheckedEthCallResult, EthCallId, EventClaimResult};


pub(crate) const LOG_TARGET: &str = "eth-bridge";

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, transactional};
	use frame_support::traits::fungibles::Transfer;
	use frame_system::{ensure_signed, pallet_prelude::*};
	use log::info;
	use sp_core::H256;
	use sp_runtime::RuntimeAppPublic;
	use seed_pallet_common::ethy::EthySigningRequest;
	use seed_pallet_common::Hold;
	use seed_pallet_common::validator_set::ValidatorSetInterface;
	use seed_primitives::{AssetId, Balance, BlockNumber, EthAddress};
	use seed_primitives::ethy::{EventClaimId, EventProofId};
	use seed_primitives::ethy::crypto::AuthorityId;
	use crate::types::{EventClaim, EventClaimStatus, NotarizationPayload};

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
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
	}

	/// Map from relayer account to their paid bond amount
	#[pallet::storage]
	#[pallet::getter(fn relayer_paid_bond)]
	pub type RelayerPaidBond<T: Config> =  StorageMap<_, Twox64Concat, T::AccountId, Balance, ValueQuery>;

	/// The permissioned relayer
	#[pallet::storage]
	#[pallet::getter(fn relayer)]
	pub type Relayer<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// The minimum number of block confirmations needed to notarize an Ethereum event
	#[pallet::type_value]
	pub fn DefaultEventBlockConfirmations() -> u64 {
		3_u64
	}
	#[pallet::storage]
	#[pallet::getter(fn event_block_confirmations)]
	pub type EventBlockConfirmations<T> =  StorageValue<_, u64, ValueQuery, DefaultEventBlockConfirmations>;

	/// The (optimistic) challenge period after which a submitted event is considered valid
	#[pallet::type_value]
	pub fn DefaultChallengePeriod<T:Config>() -> T::BlockNumber {
		T::BlockNumber::from(150_u32) // 10 Minutes
	}
	#[pallet::storage]
	#[pallet::getter(fn challenge_period)]
	pub type ChallengePeriod<T:Config> = StorageValue<_, T::BlockNumber, ValueQuery, DefaultChallengePeriod<T>>;

	/// The bridge contract address on Ethereum
	#[pallet::storage]
	#[pallet::getter(fn contract_address)]
	pub type ContractAddress<T> =  StorageValue<_, EthAddress, ValueQuery>;

	/// Queued event claims, can be challenged within challenge period
	#[pallet::storage]
	#[pallet::getter(fn pending_event_claims)]
	pub type PendingEventClaims<T> = StorageMap<_, Twox64Concat, EventClaimId, EventClaim, OptionQuery>;

	/// Tracks processed message Ids (prevent replay)
	#[pallet::storage]
	#[pallet::getter(fn processed_message_ids)]
	pub type ProcessedMessageIds<T> = StorageValue<_, Vec<EventClaimId>, ValueQuery>;

	/// Status of pending event claims
	#[pallet::storage]
	#[pallet::getter(fn pending_claim_status)]
	pub type PendingClaimStatus<T> = StorageMap<_, Twox64Concat, EventClaimId, EventClaimStatus, OptionQuery>;

	/// Map from block number to list of EventClaims that will be considered valid and should be forwarded to handlers (i.e after the optimistic challenge period has passed without issue)
	#[pallet::storage]
	#[pallet::getter(fn messages_valid_at)]
	pub type MessagesValidAt<T: Config> = StorageMap<_, Twox64Concat, T::BlockNumber, Vec<EventClaimId>, ValueQuery>;

	/// List of all event ids that are currently being challenged
	#[pallet::storage]
	#[pallet::getter(fn pending_claim_challenges)]
	pub type PendingClaimChallenges<T> = StorageValue<_, Vec<EventClaimId>, ValueQuery>;

	/// Maps from event claim id to challenger and bond amount paid
	#[pallet::storage]
	#[pallet::getter(fn challenger_account)]
	pub type ChallengerAccount<T: Config> = StorageMap<_, Twox64Concat, EventClaimId, (T::AccountId, Balance), OptionQuery>;


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
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new relayer has been set
		RelayerSet { relayer_account: T::AccountId },
		/// An account has deposited a relayer bond
		RelayerBondDeposited { account_id: T::AccountId, amount: Balance},
		/// An account has withdrawn a relayer bond
		RelayerBondWithdrawn { account_id: T::AccountId, amount: Balance},
		/// The bridge contract address has been set
		ContractAddressSet { address: EthAddress },
		/// An event has been submitted from Ethereum (event_claim_id, event_claim, process_at)
		EventSubmit { event_claim_id: EventClaimId, event_claim: EventClaim, process_at: T::BlockNumber },
		/// An event has been challenged (claim_id, challenger)
		Challenged { claim_id: EventClaimId, challenger: T::AccountId },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> where
		<T as frame_system::Config>::AccountId: From<sp_core::H160> + Into<sp_core::H160>
	{
		/// Set the relayer address
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn set_relayer(origin: OriginFor<T>, relayer: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;
			// Ensure relayer has bonded more than relayer bond amount
			ensure!(Self::relayer_paid_bond(&relayer) >= T::RelayerBond::get(), Error::<T>::NoBondPaid);
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
			ensure!(Self::relayer_paid_bond(&origin) == 0, Error::<T>::CantBondRelayer);

			let relayer_bond = T::RelayerBond::get();
			// Attempt to place a hold from the relayer account
			T::MultiCurrency::place_hold(
				T::PalletId::get(),
				&origin,
				T::NativeAssetId::get(),
				relayer_bond,
			)?;
			RelayerPaidBond::<T>::insert(&origin, relayer_bond);
			Self::deposit_event(Event::<T>::RelayerBondDeposited { account_id: origin, amount: relayer_bond});
			Ok(())
		}

		/// Withdraw relayer bond amount
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn withdraw_relayer_bond(origin: OriginFor<T>) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			// Ensure account is not the current relayer
			if Self::relayer() == Some(origin.clone()) {
				// spk - check this logic
				ensure!(Self::relayer() != Some(origin.clone()), Error::<T>::CantUnbondRelayer);
			};
			let relayer_paid_bond = Self::relayer_paid_bond(&origin);
			ensure!(relayer_paid_bond > 0, Error::<T>::CantUnbondRelayer);

			// Attempt to release the relayers hold
			T::MultiCurrency::release_hold(
				T::PalletId::get(),
				&origin,
				T::NativeAssetId::get(),
				relayer_paid_bond,
			)?;
			RelayerPaidBond::<T>::remove(&origin);

			Self::deposit_event(Event::<T>::RelayerBondWithdrawn { account_id: origin, amount: relayer_paid_bond });
			Ok(())
		}

		/// Set event confirmations (blocks). Required block confirmations for an Ethereum event to be notarized by Seed
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn set_event_block_confirmations(origin: OriginFor<T>, confirmations: u64) -> DispatchResult {
			ensure_root(origin)?;
			EventBlockConfirmations::<T>::put(confirmations);
			Ok(())
		}

		/// Set challenge period, this is the window in which an event can be challenged before processing
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn set_challenge_period(origin: OriginFor<T>, blocks: T::BlockNumber) -> DispatchResult {
			ensure_root(origin)?;
			ChallengePeriod::<T>::put(blocks);
			Ok(())
		}

		/// Set the bridge contract address on Ethereum (requires governance)
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn set_contract_address(origin: OriginFor<T>, contract_address: EthAddress) -> DispatchResult {
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
			// event SendMessage(uint256 messageId, address source, address destination, bytes message, uint256 fee);
			if let [Token::Uint(event_id), Token::Address(source), Token::Address(destination), Token::Bytes(data), Token::Uint(_fee)] = ethabi::decode(&[
				ParamType::Uint(64),
				ParamType::Address,
				ParamType::Address,
				ethabi::ParamType::Bytes,
				ParamType::Uint(64),
			], event.as_slice()).map_err(|_| Error::<T>::InvalidClaim)?.as_slice() {
				let event_id: EventClaimId = (*event_id).saturated_into();
				ensure!(!PendingEventClaims::<T>::contains_key(event_id), Error::<T>::EventReplayPending);
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

				PendingEventClaims::<T>::insert(event_id, &event_claim);
				PendingClaimStatus::<T>::insert(event_id, EventClaimStatus::Pending);

				// TODO: there should be some limit per block
				let process_at: T::BlockNumber = <frame_system::Pallet<T>>::block_number() + Self::challenge_period();
				MessagesValidAt::<T>::append(process_at, event_id);
				Self::deposit_event(Event::<T>::EventSubmit { event_claim_id: event_id, event_claim, process_at });
			}
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		/// Submit a challenge for an event
		/// Challenged events won't be processed until verified by validators
		/// An event can only be challenged once
		pub fn submit_challenge(origin: OriginFor<T>, event_claim_id: EventClaimId) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			// Validate event_id existence
			ensure!(PendingEventClaims::<T>::contains_key(event_claim_id), Error::<T>::NoClaim);
			// Check that event isn't already being challenged
			ensure!(Self::pending_claim_status(event_claim_id) == Some(EventClaimStatus::Pending), Error::<T>::ClaimAlreadyChallenged);

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

			Self::deposit_event(Event::<T>::Challenged { claim_id: event_claim_id, challenger: origin });
			Ok(())
		}

		/// Internal only
		/// Validators will submit inherents with their notarization vote for a given claim
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		#[transactional]
		pub fn submit_notarization(origin: OriginFor<T>, payload: NotarizationPayload, _signature: <AuthorityId as RuntimeAppPublic>::Signature) -> DispatchResult {
			let _ = ensure_none(origin)?;

			// we don't need to verify the signature here because it has been verified in
			// `validate_unsigned` function when sending out the unsigned tx.
			let authority_index = payload.authority_index() as usize;
			let notary_keys = T::ValidatorSet::get_validator_set()?;
			let notary_public_key = match notary_keys.get(authority_index) {
				Some(id) => id,
				None => return Err(Error::<T>::InvalidNotarization.into()),
			};

			match payload {
				NotarizationPayload::Call{ call_id, result, .. } => Self::handle_call_notarization(call_id, result, notary_public_key),
				NotarizationPayload::Event{ event_claim_id, result, .. } => Self::handle_event_notarization(event_claim_id, result, notary_public_key),
			}
		}
	}
}

impl<T: Config> Pallet<T> where <T as frame_system::Config>::AccountId: From<sp_core::H160> {
	/// Handle a submitted call notarization
	pub(crate) fn handle_call_notarization(
		call_id: EthCallId,
		result: CheckedEthCallResult,
		notary_id: &AuthorityId,
	) -> Result<(), DispatchError> {
		Ok(())
	}

	/// Handle a submitted event notarization
	pub(crate) fn handle_event_notarization(
		event_claim_id: EventClaimId,
		result: EventClaimResult,
		notary_id: &AuthorityId,
	) -> Result<(), DispatchError> {
		Ok(())
	}
}
