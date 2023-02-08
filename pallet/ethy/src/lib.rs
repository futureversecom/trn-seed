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

use codec::Encode;
use ethabi::Token;
use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight as DbWeight, Weight},
};
use log::{debug, error, info, trace};
pub use pallet::*;
use seed_pallet_common::{
	eth::EthereumEventInfo,
	ethy::{
		BridgeAdapter, EthereumBridgeAdapter, EthyAdapter, EthySigningRequest, State,
		State::{Active, Paused},
		XRPLBridgeAdapter,
	},
	validator_set::{ValidatorSetChangeHandler, ValidatorSetChangeInfo, ValidatorSetInterface},
};
use seed_primitives::{
	ethy::{crypto::AuthorityId, EventProofId},
	EthyEcdsaToEthereum, EthyEcdsaToXRPLAccountId,
};
use sp_runtime::{
	traits::{AccountIdConversion, Convert},
	DigestItem, DispatchError,
};
use sp_std::vec::Vec;

pub mod types;
use types::*;

pub(crate) const LOG_TARGET: &str = "ethy";

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use seed_pallet_common::{
		ethy::{EthereumBridgeAdapter, State},
		validator_set::ValidatorSetInterface,
	};
	use seed_primitives::ethy::ValidatorSetId;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Ethereum Bridge Adapter
		type EthereumBridgeAdapter: EthereumBridgeAdapter;
		/// Validator set Adapter
		type ValidatorSetAdapter: ValidatorSetInterface<AuthorityId>;
		/// XRPL Bridge Adapter
		type XrplBridgeAdapter: XRPLBridgeAdapter;
	}

	#[pallet::storage]
	#[pallet::getter(fn ethy_state)]
	/// Ethy state. whether it's active or paused
	pub type EthyState<T> = StorageValue<_, State, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn notary_set_proof_id)]
	/// The event proof Id generated by the previous validator set to notarize the current set.
	/// Useful for syncing the latest proof to Ethereum
	pub type NotarySetProofId<T> = StorageValue<_, EventProofId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn xrpl_notary_set_proof_id)]
	/// The event proof Id generated by the previous validator set to notarize the current set.
	/// Useful for syncing the latest proof to Xrpl
	pub type XrplNotarySetProofId<T> = StorageValue<_, EventProofId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pending_proof_requests)]
	/// Queued proof requests to be processed once the ethy is active again
	pub type PendingProofRequests<T> =
		StorageMap<_, Twox64Concat, EventProofId, EthySigningRequest>;

	#[pallet::storage]
	#[pallet::getter(fn next_event_proof_id)]
	/// Id of the next event proof
	pub type NextEventProofId<T> = StorageValue<_, EventProofId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn delayed_proof_requests_per_block)]
	/// The maximum number of delayed proof requests that can be processed in on_initialize()
	pub type DelayedProofRequestsPerBlock<T> = StorageValue<_, u8, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A notary (validator) set change is in motion (event_proof_id, new_validator_set_id)
		/// A proof for the change will be generated with the given `event_proof_id`
		AuthoritySetChanged { event_proof_id: EventProofId, new_validator_set_id: ValidatorSetId },
		/// notary set change failed
		AuthoritySetChangedFailed {
			current_validator_set: ValidatorSetId,
			new_validator_set_id: ValidatorSetId,
		},
		/// A notary (validator) set change for Xrpl is in motion (event_proof_id,
		/// new_validator_set_id) A proof for the change will be generated with the given
		/// `event_proof_id`
		XrplAuthoritySetChanged {
			event_proof_id: EventProofId,
			new_validator_set_id: ValidatorSetId,
		},
		/// Proof delayed since ethy is in paused state
		ProofDelayed { event_proof_id: EventProofId },
		/// An event proof has been sent for signing by ethy-gadget
		EventSend { event_proof_id: EventProofId, signing_request: EthySigningRequest },
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_block_number: T::BlockNumber) -> Weight {
			// process delayed proof requests
			let mut weight = 0 as Weight;
			if PendingProofRequests::<T>::iter().next().is_some() && Self::ethy_state() == Active {
				let max_delayed_requests = Self::delayed_proof_requests_per_block();
				weight = weight.saturating_add(
					max_delayed_requests as Weight *
						(DbWeight::get().reads(1 as Weight) +
							DbWeight::get().writes(1 as Weight)),
				);
				for (event_proof_id, signing_request) in
					PendingProofRequests::<T>::iter().take(max_delayed_requests as usize)
				{
					Self::request_for_event_proof(event_proof_id, signing_request);
					PendingProofRequests::<T>::remove(event_proof_id);
				}
			}
			weight += DbWeight::get().reads(1 as Weight);
			weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<sp_core::H160> + Into<sp_core::H160>,
	{
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		/// Pause or unpause ethy (requires governance)
		pub fn set_ethy_state(origin: OriginFor<T>, state: State) -> DispatchResult {
			ensure_root(origin)?;
			EthyState::<T>::put(state);
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn request_for_proof_validator_set_change_ethereum(
		info: ValidatorSetChangeInfo<AuthorityId>,
	) -> Result<EventProofId, DispatchError> {
		trace!(target: LOG_TARGET, "💎 request validator set change proof for ethereum");
		let next_validator_addresses: Vec<Token> = info
			.next_validator_set
			.to_vec()
			.into_iter()
			.map(|k| EthyEcdsaToEthereum::convert(k.as_ref()))
			.map(|k| Token::Address(k.into()))
			.collect();
		debug!(
			target: LOG_TARGET,
			"💎 ethereum new signer addresses: {:?}", next_validator_addresses
		);

		let validator_set_message = ethabi::encode(&[
			Token::Array(next_validator_addresses),
			Token::Uint(info.next_validator_set_id.into()),
		]);

		let next_event_proof_id = Self::get_next_event_proof_id();
		let event_proof_info = EthereumEventInfo {
			source: T::EthereumBridgeAdapter::get_pallet_id().into_account_truncating(),
			destination: T::EthereumBridgeAdapter::get_contract_address(),
			message: validator_set_message.to_vec(),
			validator_set_id: info.current_validator_set_id,
			event_proof_id: next_event_proof_id,
		};

		Self::request_for_event_proof(
			next_event_proof_id,
			EthySigningRequest::Ethereum(event_proof_info),
		);
		Ok(next_event_proof_id)
	}

	fn request_for_proof_validator_set_change_xrpl(
		info: ValidatorSetChangeInfo<AuthorityId>,
	) -> Result<EventProofId, DispatchError> {
		trace!(target: LOG_TARGET, "💎 request validator set change proof for xrpl");
		let mut next_notary_xrpl_keys =
			T::ValidatorSetAdapter::get_xrpl_notary_keys(&info.next_validator_set);
		let mut notary_xrpl_keys = T::ValidatorSetAdapter::get_xrpl_validator_set();

		// sort to avoid same key set shuffles.
		next_notary_xrpl_keys.sort();
		notary_xrpl_keys.sort();

		if notary_xrpl_keys == next_notary_xrpl_keys {
			info!(
				target: LOG_TARGET,
				"💎 notary xrpl keys unchanged. next validator set id: {:?}",
				info.next_validator_set_id
			);
			return Ok(0) // return EventProofId = 0
		}

		let signer_entries = next_notary_xrpl_keys
			.into_iter()
			.map(|k| EthyEcdsaToXRPLAccountId::convert(k.as_ref()))
			// TODO(surangap): Add a proper way to store XRPL weights if we intend to allow
			// having different weights
			.map(|entry| (entry.into(), 1_u16))
			.collect::<Vec<_>>();
		debug!(target: LOG_TARGET, "💎 xrpl new signer entries: {:?}", signer_entries);

		let xrpl_payload = T::XrplBridgeAdapter::get_signer_list_set_payload(signer_entries)?;
		let next_event_proof_id = Self::get_next_event_proof_id();
		Self::request_for_event_proof(
			next_event_proof_id,
			EthySigningRequest::XrplTx(xrpl_payload),
		);
		Ok(next_event_proof_id)
	}

	/// Gives the next event proof Id, increment by one and stores it
	fn get_next_event_proof_id() -> EventProofId {
		let event_proof_id = Self::next_event_proof_id();
		NextEventProofId::<T>::put(event_proof_id.wrapping_add(1));
		event_proof_id
	}

	/// Request for an event proof signing request, to be received by the ethy-gadget
	pub(crate) fn request_for_event_proof(
		event_proof_id: EventProofId,
		request: EthySigningRequest,
	) {
		// if the ethy is paused (e.g transitioning authority set at the end of an era)
		// delay the proofs until it is active again
		if Self::ethy_state() == Paused {
			PendingProofRequests::<T>::insert(event_proof_id, request);
			Self::deposit_event(Event::<T>::ProofDelayed { event_proof_id });
			return
		}

		let log: DigestItem = DigestItem::Consensus(
			ETHY_ENGINE_ID,
			ConsensusLog::<T::AccountId>::OpaqueSigningRequest {
				chain_id: request.chain_id(),
				data: request.data(),
				event_proof_id,
			}
			.encode(),
		);
		<frame_system::Pallet<T>>::deposit_log(log);
		Self::deposit_event(Event::<T>::EventSend { event_proof_id, signing_request: request });
	}
}

impl<T: Config> ValidatorSetChangeHandler<AuthorityId> for Pallet<T> {
	fn validator_set_change_in_progress(info: ValidatorSetChangeInfo<AuthorityId>) {
		// request for proof ethereum
		match Self::request_for_proof_validator_set_change_ethereum(info.clone()) {
			Ok(event_proof_id) => {
				// Signal the event id that will be used for the proof of validator set change.
				// Any observer can subscribe to this event and submit the resulting proof to keep
				// the validator set on the Ethereum bridge contract updated.
				Self::deposit_event(Event::<T>::AuthoritySetChanged {
					event_proof_id,
					new_validator_set_id: info.next_validator_set_id,
				});
				// store the notary set change event proof id
				NotarySetProofId::<T>::put(event_proof_id);
				info!(target: LOG_TARGET, "💎 authority set change proof request success. event proof If: {:?}, new validator set Id: {:?}", event_proof_id, info.next_validator_set_id);
			},
			Err(e) => {
				Self::deposit_event(Event::<T>::AuthoritySetChangedFailed {
					current_validator_set: info.current_validator_set_id,
					new_validator_set_id: info.next_validator_set_id,
				});
				error!(target: LOG_TARGET, "💎 authority set change proof request failed. next validator set Id: {:?}, error: {:?}", info.next_validator_set_id, Into::<&str>::into(e));
			},
		}

		// request for proof xrpl
		match Self::request_for_proof_validator_set_change_xrpl(info.clone()) {
			Ok(event_proof_id) => {
				// event_proof_id == 0 special case - xrpl notary keys remains the same
				if event_proof_id != 0 {
					// Signal the event id that will be used for the proof of xrpl notary set
					// change. Any observer can subscribe to this event and submit the resulting
					// proof to keep the door account signer set on the xrpl updated.
					Self::deposit_event(Event::<T>::XrplAuthoritySetChanged {
						event_proof_id,
						new_validator_set_id: info.next_validator_set_id,
					});
					// store the xrpl notary set change event proof id
					XrplNotarySetProofId::<T>::put(event_proof_id);
					info!(target: LOG_TARGET, "💎 xrpl notary set change proof request success. event proof If: {:?}, new validator set Id: {:?}", event_proof_id, info.next_validator_set_id);
				}
			},
			Err(e) => {
				Self::deposit_event(Event::<T>::AuthoritySetChangedFailed {
					current_validator_set: info.current_validator_set_id,
					new_validator_set_id: info.next_validator_set_id,
				});
				error!(target: LOG_TARGET, "💎 xrpl notary set change proof request failed. next validator set Id: {:?}, error: {:?}", info.next_validator_set_id, Into::<&str>::into(e));
			},
		}

		//pause the ethy to pause all bridge activities
		EthyState::<T>::put(Paused);
	}

	fn validator_set_change_finalized(info: ValidatorSetChangeInfo<AuthorityId>) {
		info!(
			target: LOG_TARGET,
			"💎 validator set change finalized received. new validator set id: {:?}",
			info.current_validator_set_id
		);
		// send notification to ethy-gadget
		let log = DigestItem::Consensus(
			ETHY_ENGINE_ID,
			ConsensusLog::AuthoritiesChange(ValidatorSet {
				validators: info.current_validator_set.clone(),
				id: info.current_validator_set_id,
				proof_threshold: T::EthereumBridgeAdapter::get_notarization_threshold()
					.mul_ceil(info.current_validator_set.len() as u32),
			})
			.encode(),
		);
		<frame_system::Pallet<T>>::deposit_log(log);

		// set ethy to Active
		EthyState::<T>::put(Active);
	}
}

impl<T: Config> EthyAdapter for Pallet<T> {
	fn request_for_proof(
		request: EthySigningRequest,
		event_proof_id: Option<EventProofId>,
	) -> Result<EventProofId, DispatchError> {
		match event_proof_id {
			Some(event_proof_id) => {
				Self::request_for_event_proof(event_proof_id, request);
				return Ok(event_proof_id)
			},
			None => {
				let event_proof_id = Self::get_next_event_proof_id();
				Self::request_for_event_proof(event_proof_id, request);
				return Ok(event_proof_id)
			},
		}
	}

	fn get_ethy_state() -> State {
		EthyState::<T>::get()
	}

	fn get_next_event_proof_id() -> EventProofId {
		Self::get_next_event_proof_id()
	}
}
