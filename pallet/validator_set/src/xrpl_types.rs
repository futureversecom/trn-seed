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

use async_trait::async_trait;
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use seed_primitives::{
	validator::{EventClaimId, EventProofId, ValidatorSetId},
	xrpl::{LedgerIndex, XrpTransaction},
};
pub use sp_core::{H160, H256, U256};
use sp_runtime::RuntimeDebug;
use sp_std::{prelude::*, vec::Vec};

pub type XrplTxHash = seed_primitives::xrpl::XrplTxHash;

pub type XrplAddress = seed_primitives::xrpl::XrplAddress;
/// An Chain CallOracle call Id
pub type ChainCallId = u64;

#[derive(Debug, Default, Clone, PartialEq, Eq, Decode, Encode, TypeInfo)]
/// Info required to claim an Ethereum event
pub struct EventClaim {
	/// The Ethereum transaction hash which caused the event
	pub tx_hash: XrplTxHash,
	/// The source address (contract) which posted the event
	pub source: XrplAddress,
	/// The destination address (contract) which should receive the event
	/// It may be symbolic, mapping to a pallet vs. a deployed contract
	pub destination: XrplAddress,
	/// The Ethereum ABI encoded event data as logged on Ethereum
	pub data: Vec<u8>,
}
#[derive(Debug, Default, Clone, PartialEq, Eq, Decode, Encode, TypeInfo)]
/// Info related to an event proof
pub struct EventProofInfo {
	/// The source address (contract) which posted the event
	pub source: XrplAddress,
	/// The destination address (contract) which should receive the event
	/// It may be symbolic, mapping to a pallet vs. a deployed contract
	pub destination: XrplAddress,
	/// The Ethereum ABI encoded event data as logged on Ethereum
	pub message: Vec<u8>,
	/// The validator set id for the proof
	pub validator_set_id: ValidatorSetId,
	/// The events proof id
	pub event_proof_id: EventProofId,
}
/// An EthCallOracle request
#[derive(Encode, Decode, Default, PartialEq, Clone, TypeInfo)]
pub struct CheckedChainCallRequest {
	pub tx_hash: XrplTxHash,
	pub ledger_index: LedgerIndex,
}
#[derive(Encode, Decode, Debug, Eq, PartialOrd, Ord, PartialEq, Copy, Clone, TypeInfo)]
pub enum CheckedChainCallResult {
	Ok(XrpTransaction, XrplTxHash, LedgerIndex),
	CallFailed,
}

/// Possible outcomes from attempting to verify an Ethereum event claim
#[derive(Decode, Encode, Debug, PartialEq, Clone, TypeInfo)]
pub enum EventClaimResult {
	/// It's valid
	Valid,
	/// Couldn't request data from the Eth client
	DataProviderErr,
	/// The eth tx is marked failed
	TxStatusFailed,
	/// The transaction recipient was not the expected contract
	UnexpectedContractAddress,
	/// The expected tx logs were not present
	NoTxLogs,
	/// Not enough block confirmations yet
	NotEnoughConfirmations,
	/// Tx event logs indicated this claim does not match the event
	UnexpectedData,
	/// The deposit tx is past the expiration deadline
	Expired,
	/// The Tx Receipt was not present
	NoTxReceipt,
	/// The event source did not match the tx receipt `to` field
	UnexpectedSource,
}

/// An independent notarization of a bridged value
/// This is signed and shared with the runtime after verification by a particular validator
#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo)]
#[repr(u8)]
pub enum NotarizationPayload {
	Call {
		/// The call Id being notarized
		call_id: ChainCallId,
		/// The ordinal index of the signer in the notary set
		/// It may be used with chain storage to lookup the public key of the notary
		authority_index: u16,
		/// Result of the notarization check by this authority
		result: CheckedChainCallResult,
	},
	Event {
		/// The message Id being notarized
		event_claim_id: EventClaimId,
		/// The ordinal index of the signer in the notary set
		/// It may be used with chain storage to lookup the public key of the notary
		authority_index: u16,
		/// Result of the notarization check by this authority
		result: EventClaimResult,
	},
}

impl NotarizationPayload {
	/// Return enum type Id
	pub fn type_id(&self) -> u64 {
		match self {
			Self::Call { .. } => 0_u64,
			Self::Event { .. } => 1_u64,
		}
	}
	/// Get the authority index
	pub fn authority_index(&self) -> u16 {
		match self {
			Self::Call { authority_index, .. } => *authority_index,
			Self::Event { authority_index, .. } => *authority_index,
		}
	}
	/// Get the payload id
	pub fn payload_id(&self) -> u64 {
		match self {
			Self::Call { call_id, .. } => *call_id,
			Self::Event { event_claim_id, .. } => *event_claim_id,
		}
	}
}

#[derive(Debug)]
pub enum LatestOrNumber {
	Latest,
	Number(u64),
}

#[derive(Debug, Clone, PartialEq, TypeInfo)]
/// Error type for BridgeEthereumRpcApi
pub enum BridgeRpcError {
	/// HTTP network request failed
	HttpFetch,
	/// Unable to decode response payload as JSON
	InvalidJSON,
	/// offchain worker not configured properly
	OcwConfig,
}

#[async_trait]
/// Provides request/responses according to a minimal subset of Xrpl RPC API
/// required for the bridge
pub trait BridgeXrplWebsocketApi {
	async fn transaction_entry_request(
		hash: XrplTxHash,
		ledger_index: Option<u32>,
		call_id: ChainCallId,
	) -> Result<(), BridgeRpcError>;
}
