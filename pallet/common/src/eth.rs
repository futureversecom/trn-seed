//! shared pallet types and traits
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use ethabi::Token;
use frame_support::dispatch::TypeInfo;
use seed_primitives::{
	ethy::{EventProofId, ValidatorSetId},
	EthAddress,
};
use sp_std::vec::Vec;

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
