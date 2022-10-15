/* Copyright 2021-2022 Centrality Investments Limited
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

//! Eth Bridge Types

use codec::{Decode, Encode};
use core::fmt;
use ethabi::Token;
use scale_info::TypeInfo;
use seed_primitives::{
	validator::{EventProofId, ValidatorSetId},
	EthAddress,
};

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
