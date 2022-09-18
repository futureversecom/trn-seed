use codec::Encode;
use ethabi::Token;
use frame_support::{
	pallet_prelude::*,
	traits::{OneSessionHandler, UnixTime, ValidatorSet as ValidatorSetT},
};
use frame_system::offchain::SubmitTransaction;
use sp_runtime::{
	generic::DigestItem,
	traits::{AccountIdConversion, SaturatedConversion},
	transaction_validity::{
		InvalidTransaction, TransactionSource, TransactionValidity, ValidTransaction,
	},
	Percent, RuntimeAppPublic,
};
use sp_std::prelude::*;

use seed_pallet_common::{
	log, EthCallFailure, EthCallOracle, EthCallOracleSubscriber, EthereumBridge,
	FinalSessionTracker as FinalSessionTrackerT,
};
use seed_primitives::ethy::PendingAuthorityChange;

use crate::{types::*, *};

impl<T: Config> EthereumBridge for Module<T> {
	/// Send an event via the bridge
	///  A proof of the event will be generated by notaries (async)
	///
	/// Returns an Id for the proof
	fn send_event(
		source: &H160,
		destination: &H160,
		event: &[u8],
	) -> Result<EventProofId, DispatchError> {
		let event_proof_id = Self::next_event_proof_id();
		NextEventProofId::put(event_proof_id.wrapping_add(1));

		let encoded_event = encode_event_for_proving(
			// event data
			*source,
			*destination,
			event,
			// proof metadata
			Self::validator_set().id,
			event_proof_id,
		);

		// if bridge is paused (e.g transitioning authority set at the end of an era)
		// delay proofs until it is ready again
		if Self::bridge_paused() {
			PendingEventProofs::insert(event_proof_id, encoded_event);
			Self::deposit_event(Event::<T>::ProofDelayed(event_proof_id));
		} else {
			Self::do_request_event_proof(event_proof_id, encoded_event);
		}

		Ok(event_proof_id)
	}
}

impl<T: Config> Module<T> {
	/// Check the nodes local keystore for an active (staked) Ethy session key
	/// Returns the public key and index of the key in the current notary set
	pub(crate) fn find_active_ethy_key() -> Option<(T::EthyId, u16)> {
		// Get all signing keys for this protocol 'KeyTypeId'
		let local_keys = T::EthyId::all();
		if local_keys.is_empty() {
			log!(
				error,
				"💎 no signing keys for: {:?}, cannot participate in notarization!",
				T::EthyId::ID
			);
			return None
		};

		let mut maybe_active_key: Option<(T::EthyId, usize)> = None;
		// search all local ethy keys
		for key in local_keys {
			if let Some(active_key_index) = Self::notary_keys().iter().position(|k| k == &key) {
				maybe_active_key = Some((key, active_key_index));
				break
			}
		}

		// check if locally known keys are in the active validator set
		if maybe_active_key.is_none() {
			log!(error, "💎 no active ethy keys, exiting");
			return None
		}
		maybe_active_key.map(|(key, idx)| (key, idx as u16))
	}
	/// Handle OCW event notarization protocol for validators
	/// Receives the node's local notary session key and index in the set
	pub(crate) fn do_event_notarization_ocw(active_key: &T::EthyId, authority_index: u16) {
		// do not try to notarize events while the bridge is paused
		if Self::bridge_paused() {
			return
		}

		// check all pending claims we have _yet_ to notarize and try to notarize them
		// this will be invoked once every block
		// we limit the total claims per invocation using `CLAIMS_PER_BLOCK` so we don't stall block
		// production.
		for event_claim_id in PendingClaimChallenges::get().iter().take(CLAIMS_PER_BLOCK) {
			let event_claim = Self::pending_event_claims(event_claim_id);
			if event_claim.is_none() {
				// This shouldn't happen
				log!(error, "💎 notarization failed, event claim: {:?} not found", event_claim_id);
				continue
			};

			// skip if we've notarized it previously
			if <EventNotarizations<T>>::contains_key::<EventClaimId, T::EthyId>(
				*event_claim_id,
				active_key.clone(),
			) {
				log!(trace, "💎 already notarized claim: {:?}, ignoring...", event_claim_id);
				continue
			}

			let result = Self::offchain_try_notarize_event(*event_claim_id, event_claim.unwrap());
			log!(trace, "💎 claim verification status: {:?}", &result);
			let payload = NotarizationPayload::Event {
				event_claim_id: *event_claim_id,
				authority_index,
				result: result.clone(),
			};
			let _ = Self::offchain_send_notarization(active_key, payload)
				.map_err(|err| {
					log!(error, "💎 sending notarization failed 🙈, {:?}", err);
				})
				.map(|_| {
					log!(
						info,
						"💎 sent notarization: '{:?}' for claim: {:?}",
						result,
						event_claim_id
					);
				});
		}
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
	/// - message has not expired older than `T::EventDeadline`
	///
	/// Returns result of the validation
	pub(crate) fn offchain_try_notarize_event(
		event_claim_id: EventClaimId,
		event_claim: EventClaim,
	) -> EventClaimResult {
		let EventClaim { tx_hash, data, source, destination } = event_claim;
		let result = T::EthereumRpcClient::get_transaction_receipt(tx_hash);
		if let Err(err) = result {
			log!(error, "💎 eth_getTransactionReceipt({:?}) failed: {:?}", tx_hash, err);
			return EventClaimResult::DataProviderErr
		}

		let maybe_tx_receipt = result.unwrap(); // error handled above qed.
		let tx_receipt = match maybe_tx_receipt {
			Some(t) => t,
			None => return EventClaimResult::NoTxReceipt,
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
		if let Some(log) = matching_log {
			// check if the Ethereum event data matches what was reported
			// in the original claim
			if log.data != submitted_event_data {
				log!(
					trace,
					"💎 mismatch in provided data vs. observed data. provided: {:?} observed: {:?}",
					submitted_event_data,
					log.data,
				);
				return EventClaimResult::UnexpectedData
			}
			if log.address != T::BridgeContractAddress::get() {
				return EventClaimResult::UnexpectedContractAddress
			}
		} else {
			return EventClaimResult::NoTxLogs
		}

		//  have we got enough block confirmations to be re-org safe?
		let observed_block_number: u64 = tx_receipt.block_number.saturated_into();

		let latest_block: EthBlock =
			match T::EthereumRpcClient::get_block_by_number(LatestOrNumber::Latest) {
				Ok(None) => return EventClaimResult::DataProviderErr,
				Ok(Some(block)) => block,
				Err(err) => {
					log!(error, "💎 eth_getBlockByNumber latest failed: {:?}", err);
					return EventClaimResult::DataProviderErr
				},
			};

		let latest_block_number = latest_block.number.unwrap_or_default().as_u64();
		let block_confirmations = latest_block_number.saturating_sub(observed_block_number);
		if block_confirmations < Self::event_block_confirmations() {
			return EventClaimResult::NotEnoughConfirmations
		}

		// calculate if the block is expired w some high degree of confidence without making
		// a query. time since the event = block_confirmations * ~12 seconds avg
		if block_confirmations * 12 > Self::event_deadline_seconds() {
			return EventClaimResult::Expired
		}

		//  check the block this tx is in if the timestamp > deadline
		let observed_block: EthBlock = match T::EthereumRpcClient::get_block_by_number(
			LatestOrNumber::Number(observed_block_number),
		) {
			Ok(None) => return EventClaimResult::DataProviderErr,
			Ok(Some(block)) => block,
			Err(err) => {
				log!(error, "💎 eth_getBlockByNumber observed failed: {:?}", err);
				return EventClaimResult::DataProviderErr
			},
		};

		// claim is past the expiration deadline
		// eth. block timestamp (seconds)
		// deadline (seconds)
		if T::UnixTime::now()
			.as_secs()
			.saturated_into::<u64>()
			.saturating_sub(observed_block.timestamp.saturated_into::<u64>()) >
			Self::event_deadline_seconds()
		{
			return EventClaimResult::Expired
		}

		EventClaimResult::Valid
	}

	/// Handle OCW eth call checking protocol for validators
	/// Receives the node's local notary session key and index in the set
	pub(crate) fn do_call_notarization_ocw(active_key: &T::EthyId, authority_index: u16) {
		// we limit the total claims per invocation using `CALLS_PER_BLOCK` so we don't stall block
		// production
		for call_id in EthCallRequests::get().iter().take(CALLS_PER_BLOCK) {
			// skip if we've notarized it previously
			if <EthCallNotarizations<T>>::contains_key::<EthCallId, T::EthyId>(
				*call_id,
				active_key.clone(),
			) {
				log!(trace, "💎 already notarized call: {:?}, ignoring...", call_id);
				continue
			}

			if let Some(request) = Self::eth_call_request_info(call_id) {
				let result = Self::offchain_try_eth_call(&request);
				log!(trace, "💎 checked call status: {:?}", &result);
				let payload =
					NotarizationPayload::Call { call_id: *call_id, authority_index, result };
				let _ = Self::offchain_send_notarization(active_key, payload)
					.map_err(|err| {
						log!(error, "💎 sending notarization failed 🙈, {:?}", err);
					})
					.map(|_| {
						log!(info, "💎 sent notarization: '{:?}' for call: {:?}", result, call_id,);
					});
			} else {
				// should not happen
				log!(error, "💎 empty call for: {:?}", call_id);
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
		let latest_block: EthBlock =
			match T::EthereumRpcClient::get_block_by_number(LatestOrNumber::Latest) {
				Ok(None) => return CheckedEthCallResult::DataProviderErr,
				Ok(Some(block)) => block,
				Err(err) => {
					log!(error, "💎 eth_getBlockByNumber latest failed: {:?}", err);
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
			let target_block: EthBlock = match T::EthereumRpcClient::get_block_by_number(
				LatestOrNumber::Number(request.try_block_number),
			) {
				Ok(None) => return CheckedEthCallResult::DataProviderErr,
				Ok(Some(block)) => block,
				Err(err) => {
					log!(error, "💎 eth_getBlockByNumber latest failed: {:?}", err);
					return CheckedEthCallResult::DataProviderErr
				},
			};
			target_block_number = request.try_block_number;
			target_block_timestamp = target_block.timestamp.saturated_into();
		}

		let return_data = match T::EthereumRpcClient::eth_call(
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
				log!(error, "💎 eth_call at: {:?}, failed: {:?}", target_block_number, err);
				return CheckedEthCallResult::DataProviderErr
			},
		};

		// valid returndata is ethereum abi encoded and therefore always >= 32 bytes
		match return_data.try_into() {
			Ok(r) => CheckedEthCallResult::Ok(r, target_block_number, target_block_timestamp),
			Err(_) => CheckedEthCallResult::ReturnDataExceedsLimit,
		}
	}

	/// Send a notarization for the given claim
	fn offchain_send_notarization(
		key: &T::EthyId,
		payload: NotarizationPayload,
	) -> Result<(), Error<T>> {
		let signature =
			key.sign(&payload.encode()).ok_or(<Error<T>>::OffchainUnsignedTxSignedPayload)?;

		let call = Call::submit_notarization { payload, _signature: signature };

		// Retrieve the signer to sign the payload
		SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
			.map_err(|_| <Error<T>>::OffchainUnsignedTxSignedPayload)
	}

	/// Return the active Ethy validator set.
	pub fn validator_set() -> ValidatorSet<T::EthyId> {
		let validator_keys = Self::notary_keys();
		ValidatorSet::<T::EthyId> {
			proof_threshold: T::NotarizationThreshold::get().mul_ceil(validator_keys.len() as u32),
			validators: validator_keys,
			id: Self::notary_set_id(),
		}
	}

	/// Handle a submitted event notarization
	pub(crate) fn handle_event_notarization(
		event_claim_id: EventClaimId,
		result: EventClaimResult,
		notary_id: &T::EthyId,
	) -> DispatchResult {
		if !PendingEventClaims::contains_key(event_claim_id) {
			// there's no claim active
			return Err(Error::<T>::InvalidClaim.into())
		}

		// Store the new notarization
		<EventNotarizations<T>>::insert::<EventClaimId, T::EthyId, EventClaimResult>(
			event_claim_id,
			notary_id.clone(),
			result,
		);

		// Count notarization votes
		let notary_count = T::AuthoritySet::validators().len() as u32;
		let mut yay_count = 0_u32;
		let mut nay_count = 0_u32;
		// TODO: store the count
		for (_id, result) in <EventNotarizations<T>>::iter_prefix(event_claim_id) {
			match result {
				EventClaimResult::Valid => yay_count += 1,
				_ => nay_count += 1,
			}
		}

		// Claim is invalid (nays > (100% - NotarizationThreshold))
		if Percent::from_rational(nay_count, notary_count) >
			(Percent::from_parts(100_u8 - T::NotarizationThreshold::get().deconstruct()))
		{
			if let Some(cursor) = <EventNotarizations<T>>::clear_prefix(
				event_claim_id,
				NotaryKeys::<T>::decode_len().unwrap_or(1_000) as u32,
				None,
			)
			.maybe_cursor
			{
				log!(error, "💎 cleaning storage entries failed: {:?}", cursor);
				return Err(Error::<T>::Internal.into())
			}
			PendingClaimChallenges::mutate(|event_ids| {
				event_ids
					.iter()
					.position(|x| *x == event_claim_id)
					.map(|idx| event_ids.remove(idx));
			});

			if let Some(_event_claim) = PendingEventClaims::take(event_claim_id) {
				// TODO: voting is complete, the event is invalid
				// handle slashing, ensure event is requeued for execution
				Self::deposit_event(Event::<T>::Invalid(event_claim_id));
				return Ok(())
			} else {
				log!(error, "💎 unexpected empty claim");
				return Err(Error::<T>::InvalidClaim.into())
			}
		}

		// Claim is valid
		if Percent::from_rational(yay_count, notary_count) >= T::NotarizationThreshold::get() {
			// no need to track info on this claim any more since it's approved
			if let Some(cursor) = <EventNotarizations<T>>::clear_prefix(
				event_claim_id,
				NotaryKeys::<T>::decode_len().unwrap_or(1_000) as u32,
				None,
			)
			.maybe_cursor
			{
				log!(error, "💎 cleaning storage entries failed: {:?}", cursor);
				return Err(Error::<T>::Internal.into())
			}
			PendingClaimChallenges::mutate(|event_ids| {
				event_ids
					.iter()
					.position(|x| *x == event_claim_id)
					.map(|idx| event_ids.remove(idx));
			});

			if let Some(_event_claim) = PendingEventClaims::take(event_claim_id) {
				// TODO: voting is complete, the event is valid
				// handle slashing, ensure event is requeued for execution
				Self::deposit_event(Event::<T>::Verified(event_claim_id));
			} else {
				log!(error, "💎 unexpected empty claim");
				return Err(Error::<T>::InvalidClaim.into())
			}
		}

		Ok(())
	}

	/// Handle a submitted call notarization
	pub(crate) fn handle_call_notarization(
		call_id: EthCallId,
		result: CheckedEthCallResult,
		notary_id: &T::EthyId,
	) -> DispatchResult {
		if !EthCallRequestInfo::contains_key(call_id) {
			// there's no claim active
			return Err(Error::<T>::InvalidClaim.into())
		}

		// Record the notarization (ensures the validator won't resubmit it)
		<EthCallNotarizations<T>>::insert::<EventClaimId, T::EthyId, CheckedEthCallResult>(
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
				NotaryKeys::<T>::decode_len().unwrap_or(1_000) as u32,
				None,
			)
			.maybe_cursor
			{
				log!(error, "💎 cleaning storage entries failed: {:?}", cursor);
				return Err(Error::<T>::Internal.into())
			};
			EthCallNotarizationsAggregated::remove(call_id);
			EthCallRequestInfo::remove(call_id);
			EthCallRequests::mutate(|requests| {
				requests.iter().position(|x| *x == call_id).map(|idx| requests.remove(idx));
			});

			Ok(())
		};

		let mut notarizations = EthCallNotarizationsAggregated::get(call_id).unwrap_or_default();
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
		EthCallNotarizationsAggregated::insert(call_id, notarizations);
		Ok(())
	}

	/// Handle changes to the authority set
	/// This could be called when validators rotate their keys, we don't want to
	/// change this until the era has changed to avoid generating proofs for small set changes or
	/// too frequently
	/// - `new`: The validator set that is active right now
	/// - `queued`: The validator set that will activate next session
	pub(crate) fn handle_authorities_change(new: Vec<T::EthyId>, queued: Vec<T::EthyId>) {
		// ### Session life cycle
		// block on_initialize if ShouldEndSession(n)
		//  rotate_session
		//    before_end_session
		//    end_session (end just been)
		//    start_session (start now)
		//    new_session (start now + 1)
		//   -> on_new_session <- this function is CALLED here

		let log_notary_change = |next_keys: &[T::EthyId]| {
			// Store the keys for usage next session
			<NextNotaryKeys<T>>::put(next_keys);
			// Signal the Event Id that will be used for the proof of validator set change.
			// Any observer can subscribe to this event and submit the resulting proof to keep the
			// validator set on the Ethereum bridge contract updated.
			let event_proof_id = NextEventProofId::get();
			let next_validator_set_id = Self::notary_set_id().wrapping_add(1);
			Self::deposit_event(Event::<T>::AuthoritySetChange(
				event_proof_id,
				next_validator_set_id,
			));
			NotarySetProofId::put(event_proof_id);
			NextEventProofId::put(event_proof_id.wrapping_add(1));
			let log: DigestItem = DigestItem::Consensus(
				ETHY_ENGINE_ID,
				ConsensusLog::PendingAuthoritiesChange(PendingAuthorityChange {
					source: T::BridgePalletId::get().into_account_truncating(),
					destination: T::BridgeContractAddress::get().into(),
					next_validator_set: ValidatorSet {
						validators: next_keys.to_vec(),
						id: next_validator_set_id,
						proof_threshold: T::NotarizationThreshold::get()
							.mul_ceil(next_keys.len() as u32),
					},
					event_proof_id,
				})
				.encode(),
			);
			<frame_system::Pallet<T>>::deposit_log(log);
		};

		// signal 1 session early about the `queued` validator set change for the next era so
		// there's time to generate a proof
		if T::FinalSessionTracker::is_next_session_final() {
			log!(trace, "💎 next session final");
			log_notary_change(queued.as_ref());
		} else if T::FinalSessionTracker::is_active_session_final() {
			// Pause bridge claim/proofs
			// Prevents claims/proofs being partially processed and failing if the validator set
			// changes significantly
			// Note: the bridge will be reactivated at the end of the session
			log!(trace, "💎 active session final");
			BridgePaused::put(true);

			if Self::next_notary_keys().is_empty() {
				// if we're here the era was forced, we need to generate a proof asap
				log!(warn, "💎 urgent notary key rotation");
				log_notary_change(new.as_ref());
			}
		}
	}

	/// Submits an Ethereum event proof request in the block, for use by the ethy-gadget protocol
	pub(crate) fn do_request_event_proof(
		event_proof_id: EventClaimId,
		packed_event_with_id: Message,
	) {
		let log: DigestItem = DigestItem::Consensus(
			ETHY_ENGINE_ID,
			ConsensusLog::<T::AccountId>::OpaqueSigningRequest((
				packed_event_with_id,
				event_proof_id,
			))
			.encode(),
		);
		<frame_system::Pallet<T>>::deposit_log(log);
	}
}

impl<T: Config> frame_support::unsigned::ValidateUnsigned for Module<T> {
	type Call = Call<T>;

	fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
		if let Call::submit_notarization { ref payload, _signature: ref signature } = call {
			// notarization must be from an active notary
			let notary_keys = Self::notary_keys();
			let notary_public_key = match notary_keys.get(payload.authority_index() as usize) {
				Some(id) => id,
				None => return InvalidTransaction::BadProof.into(),
			};
			// notarization must not be a duplicate/equivocation
			if <EventNotarizations<T>>::contains_key(payload.payload_id(), &notary_public_key) {
				log!(
					error,
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
		} else {
			InvalidTransaction::Call.into()
		}
	}
}

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Module<T> {
	type Public = T::EthyId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Module<T> {
	type Key = T::EthyId;

	fn on_genesis_session<'a, I: 'a>(validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::EthyId)>,
	{
		let keys = validators.map(|x| x.1).collect::<Vec<_>>();
		if !keys.is_empty() {
			assert!(NotaryKeys::<T>::decode_len().is_none(), "NotaryKeys are already initialized!");
			NotaryKeys::<T>::put(keys);
		}
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::EthyId)>,
	{
		// Only run change process at the end of an era
		if T::FinalSessionTracker::is_next_session_final() ||
			T::FinalSessionTracker::is_active_session_final()
		{
			// Record authorities for the new session.
			let next_authorities = validators.map(|(_, k)| k).collect::<Vec<_>>();
			let next_queued_authorities = queued_validators.map(|(_, k)| k).collect::<Vec<_>>();

			Self::handle_authorities_change(next_authorities, next_queued_authorities);
		}
	}

	/// A notification for end of the session.
	///
	/// Note it is triggered before any [`SessionManager::end_session`] handlers,
	/// so we can still affect the validator set.
	fn on_before_session_ending() {
		// Re-activate the bridge, allowing claims & proofs again
		if T::FinalSessionTracker::is_active_session_final() {
			log!(trace, "💎 session & era ending, set new validator keys");
			// A proof should've been generated now so we can reactivate the bridge with the new
			// validator set
			BridgePaused::kill();
			// Time to update the bridge validator keys.
			let next_notary_keys = NextNotaryKeys::<T>::take();
			// Store the new keys and increment the validator set id
			// Next notary keys should be unset, until populated by new session logic
			<NotaryKeys<T>>::put(&next_notary_keys);
			NotarySetId::mutate(|next_set_id| *next_set_id = next_set_id.wrapping_add(1));
		}
	}

	fn on_disabled(_i: u32) {}
}

impl<T: Config> EthCallOracle for Module<T> {
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
		let call_id = NextEthCallId::get();
		EthCallRequestInfo::insert(
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
		EthCallRequests::append(call_id);
		NextEthCallId::put(call_id + 1);

		call_id
	}
}

/// Ethereum ABI encode an event/message for proving (and later submission to Ethereum)
/// `source` the pallet pseudo address sending the event
/// `destination` the contract address to receive the event
/// `message` The message data
/// `validator_set_id` The id of the current validator set
/// `event_proof_id` The id of this outgoing event/proof
pub fn encode_event_for_proving(
	source: H160,
	destination: H160,
	message: &[u8],
	validator_set_id: u64,
	event_proof_id: EventProofId,
) -> Vec<u8> {
	ethabi::encode(&[
		Token::Address(source),
		Token::Address(destination),
		Token::Bytes(message.to_vec()),
		Token::Uint(validator_set_id.into()),
		Token::Uint(event_proof_id.into()),
	])
}
