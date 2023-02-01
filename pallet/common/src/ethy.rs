//! shared pallet types and traits
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use ethabi::Token;
use frame_support::{dispatch::TypeInfo, sp_io, PalletId};
use seed_primitives::{
	ethy::{EthyChainId, EventProofId, ValidatorSetId},
	xrpl::XrplAccountId,
	EthAddress,
};
use sp_runtime::{DispatchError, Percent};
use sp_std::{fmt::Debug, vec::Vec};

/// Interface for pallet-ethy
pub trait EthyAdapter {
	/// request ethy to request for an event proof from ethy-gadget
	/// if the event_proof_id is given, it will be used, or else next available will be used
	fn request_for_proof(
		request: EthySigningRequest,
		event_proof_id: Option<EventProofId>,
	) -> Result<EventProofId, DispatchError>;
	/// get ethy state
	fn get_ethy_state() -> State;
	/// get next event proof id
	/// This will increment the value at NextEventProofId storage item in ethy
	fn get_next_event_proof_id() -> EventProofId;
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Decode, Encode, TypeInfo)]
/// Info related to an Ethereum event proof (outgoing)
pub struct EthereumEventInfo {
	/// The source address (contract) which posted the event
	pub source: EthAddress,
	/// The destination address (contract) which should receive the event
	/// It may be symbolic, mapping to a pallet vs. a deployed contract
	pub destination: EthAddress,
	/// The Ethereum ABI encoded event data as logged on Ethereum
	pub message: Vec<u8>,
	/// The validator set id for the proof
	pub validator_set_id: ValidatorSetId,
	/// The event's proof id
	pub event_proof_id: EventProofId,
}

impl EthereumEventInfo {
	/// Ethereum ABI encode an event/message for proving (and later submission to Ethereum)
	/// `source` the pallet pseudo address sending the event
	/// `destination` the contract address to receive the event
	/// `message` The message data
	/// `validator_set_id` The id of the current validator set
	/// `event_proof_id` The id of this outgoing event/proof
	pub fn abi_encode(&self) -> Vec<u8> {
		ethabi::encode(&[
			Token::Address(self.source),
			Token::Address(self.destination),
			Token::Bytes(self.message.clone()),
			Token::Uint(self.validator_set_id.into()),
			Token::Uint(self.event_proof_id.into()),
		])
	}
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

/// state of ethy module
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
/// all bridges should implement it
pub trait BridgeAdapter {
	/// returns the pallet Id
	fn get_pallet_id() -> Result<PalletId, DispatchError>;
}

/// Interface for Ethereum bridge
pub trait EthereumBridgeAdapter: BridgeAdapter {
	fn get_contract_address() -> Result<EthAddress, DispatchError>;
	fn get_notarization_threshold() -> Result<Percent, DispatchError>;
}

/// Interface for pallet-xrpl-bridge
pub trait XRPLBridgeAdapter<EthyId>: BridgeAdapter {
	fn get_door_signers() -> Result<Vec<EthyId>, DispatchError>;
	fn get_signer_list_set_payload(_: Vec<(XrplAccountId, u16)>) -> Result<Vec<u8>, DispatchError>;
}
