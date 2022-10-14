use codec::Encode;
use frame_support::{
    pallet_prelude::*, traits::ValidatorSet as ValidatorSetT,
    weights::constants::RocksDbWeight as DbWeight,
};
use frame_system::offchain::SubmitTransaction;
use sp_runtime::{transaction_validity::{
    InvalidTransaction, TransactionSource, TransactionValidity, ValidTransaction,
}, Percent, RuntimeAppPublic, DigestItem};
use sp_std::prelude::*;

use seed_pallet_common::log;
use seed_primitives::validator::{ConsensusLog, EventClaimId};

use crate::{xrpl_types::*, *};

impl<T: Config> Pallet<T> {
    /// Submit an event proof signing request in the block, for use by the ethy-gadget protocol
    pub(crate) fn do_request_event_proof(
        event_proof_id: EventProofId,
        request: EthySigningRequest,
    ) {
        // if bridge is paused (e.g transitioning authority set at the end of an era)
        // delay proofs until it is ready again
        if Self::bridge_paused() {
            PendingEventProofs::insert(event_proof_id, request);
            Self::deposit_event(Event::<T>::ProofDelayed(event_proof_id));
            return
        }

        let log: DigestItem = DigestItem::Consensus(
            ENGINE_ID,
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