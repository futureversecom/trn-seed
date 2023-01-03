//! shared pallet types and traits
#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::DispatchError;

/// Interface for pallet-xrpl-bridge
pub trait XRPLBridgeAdapter<EthyId> {
    fn get_door_signers() -> Result<Vec<EthyId>, DispatchError>;
    fn get_signer_list_set_payload(
        _: Vec<(EthyId, u16)>,
    ) -> Result<Vec<u8>, DispatchError>;
}