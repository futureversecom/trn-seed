//! shared pallet types and traits
#![cfg_attr(not(feature = "std"), no_std)]

use crate::eth::EthereumEventInfo;
use codec::{Decode, Encode};
use frame_support::{dispatch::TypeInfo, sp_io, PalletId};
use seed_primitives::{
	ethy::{EthyChainId, EventProofId},
	xrpl::XrplAccountId,
	EthAddress,
};
use sp_runtime::{DispatchError, Percent};
use sp_std::{fmt::Debug, vec::Vec};

/// Interface for pallet-ethy
pub trait EthyAdapter {
	/// Request ethy to request for an event proof from ethy-gadget
	/// If the event_proof_id is given, it will be used, or else next available will be used
	fn request_for_proof(
		request: EthySigningRequest,
		event_proof_id: Option<EventProofId>,
	) -> Result<EventProofId, DispatchError>;
	/// Get ethy state
	fn get_ethy_state() -> State;
	/// Get next event proof id
	/// This will increment the value at NextEventProofId storage item in ethy
	fn get_next_event_proof_id() -> EventProofId;
}

/// A request for ethy-gadget to sign something
#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode, TypeInfo)]
pub enum EthySigningRequest {
	/// Request to sign an event for Ethereum
	Ethereum(EthereumEventInfo),
	/// Request to sign an XRPL tx (binary serialized in 'for signing' mode)
	XrplTx(Vec<u8>),
}

impl EthySigningRequest {
	/// Return the Chain Id associated with the signing request
	pub fn chain_id(&self) -> EthyChainId {
		match self {
			Self::Ethereum(_) => EthyChainId::Ethereum,
			Self::XrplTx { .. } => EthyChainId::Xrpl,
		}
	}
	/// Return the data for signing by ethy
	pub fn data(&self) -> Vec<u8> {
		match self {
			// Ethereum event signing requires keccak hashing the event
			Self::Ethereum(event) =>
				sp_io::hashing::keccak_256(&event.abi_encode().as_slice()).to_vec(),
			// XRPL tx hashing must happen before signing to inject the public key
			Self::XrplTx(data) => data.clone(),
		}
	}
}

/// State of ethy module
#[derive(Decode, Encode, Debug, PartialEq, Clone, TypeInfo)]
pub enum State {
	Active,
	Paused,
}
impl Default for State {
	fn default() -> Self {
		Self::Active
	}
}

/// Common interface for all bridges
/// All bridges should implement this
pub trait BridgeAdapter {
	/// Return the pallet Id
	fn get_pallet_id() -> PalletId;
}

/// Interface for Ethereum bridge
pub trait EthereumBridgeAdapter: BridgeAdapter {
	/// Return ethereum contract address
	fn get_contract_address() -> EthAddress;
	/// Get notarizaton threshold for Eth bridge
	fn get_notarization_threshold() -> Percent;
}

/// Interface for pallet-xrpl-bridge
pub trait XRPLBridgeAdapter: BridgeAdapter {
	/// Return the formatted payload for signer list set message
	fn get_signer_list_set_payload(_: Vec<(XrplAccountId, u16)>) -> Result<Vec<u8>, DispatchError>;
}
