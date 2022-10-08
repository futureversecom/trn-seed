use codec::Encode;
use frame_support::{
	pallet_prelude::*, traits::ValidatorSet as ValidatorSetT,
	weights::constants::RocksDbWeight as DbWeight,
};
use frame_system::offchain::SubmitTransaction;
use sp_runtime::{
	transaction_validity::{
		InvalidTransaction, TransactionSource, TransactionValidity, ValidTransaction,
	},
	Percent, RuntimeAppPublic,
};
use sp_std::prelude::*;

use seed_pallet_common::log;
use seed_primitives::validator::EventClaimId;

use crate::{xrpl_types::*, *};

impl<T: Config> Pallet<T> {
	/// Check the nodes local keystore for an active (staked) Validator session key
	/// Returns the public key and index of the key in the current notary set
	pub(crate) fn find_active_validator_key() -> Option<(T::ValidatorId, u16)> {
		// Get all signing keys for this protocol 'KeyTypeId'
		let local_keys = T::ValidatorId::all();
		if local_keys.is_empty() {
			log!(
				error,
				"💎 no signing keys for: {:?}, cannot participate in notarization!",
				T::ValidatorId::ID
			);
			return None
		};

		let mut maybe_active_key: Option<(T::ValidatorId, usize)> = None;
		for key in local_keys {
			if let Some(active_key_index) = Self::notary_keys().iter().position(|k| k == &key) {
				maybe_active_key = Some((key, active_key_index));
				break
			}
		}

		// check if locally known keys are in the active validator set
		if maybe_active_key.is_none() {
			log!(error, "💎 no active validator keys, exiting");
			return None
		}
		maybe_active_key.map(|(key, idx)| (key, idx as u16))
	}

	pub(crate) fn schedule_requests() -> Weight {
		let (list, w) = T::XrplBridgeCall::challenged_tx_list(CLAIMS_PER_BLOCK);
		let mut reads = 0 as Weight;
		let mut writes = 0 as Weight;
		for (ledger_index, xrp_transaction) in list {
			let call_id = <NextChainCallId<T>>::get();
			reads += 2;
			<ChainCallRequestInfo<T>>::insert(
				call_id,
				CheckedChainCallRequest { ledger_index, xrp_transaction },
			);
			<ChainCallRequests<T>>::append(call_id);
			<NextChainCallId<T>>::put(call_id + 1);
			writes += 3;
		}
		w + DbWeight::get().reads_writes(reads, writes)
	}

	pub(crate) fn do_call_validate_challenge_ocw(
		active_key: &T::ValidatorId,
		authority_index: u16,
	) {
		// we limit the total claims per invocation using `CALLS_PER_BLOCK` so we don't stall block
		// production
		for call_id in <ChainCallRequests<T>>::get().iter().take(CALLS_PER_BLOCK) {
			// skip if we've notarized it previously
			if <ChainCallNotarizations<T>>::contains_key::<ChainCallId, T::ValidatorId>(
				*call_id,
				active_key.clone(),
			) {
				log!(trace, "💎 already notarized call: {:?}, ignoring...", call_id);
				continue
			}

			if let Some(request) = Self::chain_call_request_info(call_id) {
				let _ = Self::offchain_try_transaction_entry_request(
					&request,
					&call_id,
					active_key,
					authority_index,
				);
			} else {
				// should not happen
				log!(error, "💎 empty call for: {:?}", call_id);
			}
		}
	}

	pub(crate) async fn offchain_try_transaction_entry_request(
		request: &CheckedChainCallRequest,
		call_id: &ChainCallId,
		active_key: &T::ValidatorId,
		authority_index: u16,
	) -> CheckedChainCallResult {
		let mut receiver = match T::ChainWebsocketClient::transaction_entry_request(
			request.xrp_transaction,
			request.ledger_index,
			call_id.clone(),
		)
		.await
		{
			Ok(receiver) => receiver,
			Err(e) => {
				log!(
					error,
					"💎 transaction_entry_request for call_id: {:?}, failed: {:?}",
					call_id,
					e
				);
				return CheckedChainCallResult::CallFailed
			},
		};
		let recv = match receiver.recv().await {
			Some(r) => r,
			None => return CheckedChainCallResult::CallFailed,
		};
		match recv {
			Ok(transaction_hash) => {
				let result = CheckedChainCallResult::Ok(transaction_hash);
				Self::offchain_send_notarization_payload(
					call_id,
					active_key,
					authority_index,
					result,
				);
			},
			Err(e) => {
				log!(trace, "💎 XRPL transaction_entry_request Error: {:?}", e);
				match e {
					BridgeRpcError::InvalidTransaction(_msg) => {
						let result =
							CheckedChainCallResult::NotOk(request.xrp_transaction.transaction_hash);
						Self::offchain_send_notarization_payload(
							call_id,
							active_key,
							authority_index,
							result,
						);
					},
					_ => {},
				}
			},
		}
		CheckedChainCallResult::CallFailed
	}

	fn offchain_send_notarization_payload(
		call_id: &ChainCallId,
		active_key: &T::ValidatorId,
		authority_index: u16,
		result: CheckedChainCallResult,
	) {
		let payload = NotarizationPayload::Call { call_id: *call_id, authority_index, result };
		let _ = Self::offchain_send_notarization(active_key, payload)
			.map_err(|err| {
				log!(error, "💎 sending notarization failed 🙈, {:?}", err);
			})
			.map(|_| {
				log!(info, "💎 sent notarization: '{:?}' for call: {:?}", result, call_id,);
			});
	}
	/// Send a notarization for the given claim
	fn offchain_send_notarization(
		key: &T::ValidatorId,
		payload: NotarizationPayload,
	) -> Result<(), Error<T>> {
		let signature =
			key.sign(&payload.encode()).ok_or(<Error<T>>::OffchainUnsignedTxSignedPayload)?;

		let call = Call::submit_notarization { payload, signature };

		// Retrieve the signer to sign the payload
		SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
			.map_err(|_| <Error<T>>::OffchainUnsignedTxSignedPayload)
	}

	/// Return the active Ethy validator set.
	pub fn validator_set() -> ValidatorSet<T::ValidatorId> {
		let validator_keys = Self::notary_keys();
		ValidatorSet::<T::ValidatorId> {
			proof_threshold: T::NotarizationThreshold::get().mul_ceil(validator_keys.len() as u32),
			validators: validator_keys,
			id: Self::notary_set_id(),
		}
	}

	/// Handle a submitted call notarization
	pub(crate) fn handle_call_notarization(
		call_id: ChainCallId,
		result: CheckedChainCallResult,
		notary_id: &T::ValidatorId,
	) -> DispatchResult {
		if !<ChainCallRequestInfo<T>>::contains_key(call_id) {
			// there's no claim active
			return Err(Error::<T>::InvalidClaim.into())
		}

		// Record the notarization (ensures the validator won't resubmit it)
		<ChainCallNotarizations<T>>::insert::<EventClaimId, T::ValidatorId, CheckedChainCallResult>(
			call_id,
			notary_id.clone(),
			result,
		);

		// notify subscribers of a notarized eth_call outcome and clean upstate
		let do_callback_and_clean_up = |result: CheckedChainCallResult| {
			match result {
				CheckedChainCallResult::Ok(transaction_hash) =>
					T::XrplBridgeCall::process_transaction(transaction_hash),
				CheckedChainCallResult::NotOk(transaction_hash) =>
					T::XrplBridgeCall::do_not_process_transaction(transaction_hash),
				CheckedChainCallResult::CallFailed => {},
			}
			if let Some(cursor) = <ChainCallNotarizations<T>>::clear_prefix(
				call_id,
				NotaryKeys::<T>::decode_len().unwrap_or(1_000) as u32,
				None,
			)
			.maybe_cursor
			{
				log!(error, "💎 cleaning storage entries failed: {:?}", cursor);
				return Err(Error::<T>::Internal.into())
			};
			<ChainCallNotarizationsAggregated<T>>::remove(call_id);
			<ChainCallRequestInfo<T>>::remove(call_id);
			<ChainCallRequests<T>>::mutate(|requests| {
				requests.iter().position(|x| *x == call_id).map(|idx| requests.remove(idx));
			});

			Ok(())
		};

		let mut notarizations =
			<ChainCallNotarizationsAggregated<T>>::get(call_id).unwrap_or_default();
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
		<ChainCallNotarizationsAggregated<T>>::insert(call_id, notarizations);
		Ok(())
	}
}

impl<T: Config> frame_support::unsigned::ValidateUnsigned for Pallet<T> {
	type Call = Call<T>;

	fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
		if let Call::submit_notarization { ref payload, ref signature } = call {
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
			ValidTransaction::with_tag_prefix("xrp-bridge")
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
