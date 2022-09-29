use codec::Encode;
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
	log,
	xrpl::{XrplBridge, XrplCallOracle},
	FinalSessionTracker as FinalSessionTrackerT,
};
use seed_primitives::validator::{EventClaimId, EventProofId};

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
		// search all local ethy keys
		for key in local_keys {
			if let Some(active_key_index) = Self::validator_list().iter().position(|k| k == &key) {
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

	pub(crate) fn do_call_validate_challenge_ocw(
		active_key: &T::ValidatorId,
		authority_index: u16,
	) {
		// we limit the total claims per invocation using `CALLS_PER_BLOCK` so we don't stall block
		// production
		for tx_hash in T::XrplBridgeCall::challenged_tx_list(CALLS_PER_BLOCK) {
			// skip if we've notarized it previously
			if <ChainCallNotarizations<T>>::contains_key::<ChainCallId, T::ValidatorId>(
				*call_id,
				active_key.clone(),
			) {
				log!(trace, "💎 already notarized call: {:?}, ignoring...", call_id);
				continue
			}

			if let Some(request) = Self::chain_call_request_info(call_id) {
				let result = Self::offchain_try_xrp_call(&request);
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

	pub(crate) fn offchain_try_xrp_call(
		request: &CheckedChainCallRequest,
	) -> CheckedChainCallResult {

		/*let return_data = match T::ChainWebsocketClient::xrpl_call(
			request.target,
			&request.input,
			LatestOrNumber::Number(target_block_number),
		) {
			Ok(data) =>
				if data.is_empty() {
					return CheckedChainCallResult::ReturnDataEmpty
				} else {
					data
				},
			Err(err) => {
				log!(error, "💎 eth_call at: {:?}, failed: {:?}", target_block_number, err);
				return CheckedChainCallResult::DataProviderErr
			},
		};

		match return_data.try_into() {
			Ok(r) => CheckedChainCallResult::Ok(r, target_block_number, target_block_timestamp),
			Err(_) => CheckedChainCallResult::ReturnDataExceedsLimit,
		}*/
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
		let validator_keys = Self::validator_list();
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
		if !ChainCallRequestInfo::contains_key(call_id) {
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
			if let Some(cursor) = <ChainCallNotarizations<T>>::clear_prefix(
				call_id,
				ValidatorList::<T>::decode_len().unwrap_or(1_000) as u32,
				None,
			)
			.maybe_cursor
			{
				log!(error, "💎 cleaning storage entries failed: {:?}", cursor);
				return Err(Error::<T>::Internal.into())
			};
			ChainCallNotarizationsAggregated::remove(call_id);
			ChainCallRequestInfo::remove(call_id);
			ChainCallRequests::mutate(|requests| {
				requests.iter().position(|x| *x == call_id).map(|idx| requests.remove(idx));
			});

			Ok(())
		};

		let mut notarizations = ChainCallNotarizationsAggregated::get(call_id).unwrap_or_default();
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
		ChainCallNotarizationsAggregated::insert(call_id, notarizations);
		Ok(())
	}
}

impl<T: Config> frame_support::unsigned::ValidateUnsigned for Pallet<T> {
	type Call = Call<T>;

	fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
		if let Call::submit_notarization { ref payload, signature: ref signature } = call {
			// notarization must be from an active notary
			let validator_list = Self::validator_list();
			let notary_public_key = match validator_list.get(payload.authority_index() as usize) {
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

impl<T: Config> XrplCallOracle for Pallet<T> {
	type Address = XrplAddress;
	type CallId = ChainCallId;
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
		let call_id = NextChainCallId::get();
		ChainCallRequestInfo::insert(
			call_id,
			CheckedChainCallRequest {
				check_timestamp: T::UnixTime::now().as_secs(),
				input: input.to_vec(),
				target: *target,
				timestamp,
				try_block_number,
				max_block_look_behind,
			},
		);
		ChainCallRequests::append(call_id);
		NextChainCallId::put(call_id + 1);

		call_id
	}
}

/// Prunes claim ids that are less than the max contiguous claim id.
pub(crate) fn prune_claim_ids(claim_ids: &mut Vec<EventClaimId>) {
	// if < 1 element, nothing to do
	if let 0..=1 = claim_ids.len() {
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
	// drain the array from start to (first_noncontinuous_idx - 1) since we need the max contiguous
	// element in the pruned vector.
	match first_noncontinuous_idx {
		Some(idx) => claim_ids.drain(..idx - 1),
		None => claim_ids.drain(..claim_ids.len() - 1), // we need the last element to remain
	};
}
