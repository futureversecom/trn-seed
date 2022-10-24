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
use serde::{Deserialize, Serialize};
pub use sp_core::{H160, H256, U256};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;
use tokio::sync::mpsc::Receiver;

pub type XrplTxHash = seed_primitives::xrpl::XrplTxHash;

pub type XrplAddress = seed_primitives::xrpl::XrplAddress;
/// An Chain CallOracle call Id
pub type ChainCallId = u64;

/// An EthCallOracle request
#[derive(Encode, Decode, Default, PartialEq, Clone, TypeInfo, Debug)]
pub struct CheckedChainCallRequest {
	pub ledger_index: LedgerIndex,
	pub xrp_transaction: XrpTransaction,
}
#[derive(Encode, Decode, Debug, Eq, PartialOrd, Ord, PartialEq, Copy, Clone, TypeInfo)]
pub enum CheckedChainCallResult {
	Ok(XrplTxHash),
	NotOk(XrplTxHash),
	CallFailed,
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
}

impl NotarizationPayload {
	/// Return enum type Id
	pub fn type_id(&self) -> u64 {
		match self {
			Self::Call { .. } => 0_u64,
		}
	}
	/// Get the authority index
	pub fn authority_index(&self) -> u16 {
		match self {
			Self::Call { authority_index, .. } => *authority_index,
		}
	}
	/// Get the payload id
	pub fn payload_id(&self) -> u64 {
		match self {
			Self::Call { call_id, .. } => *call_id,
		}
	}
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
	/// Transaction is invalid
	InvalidTransaction(String),
}

#[async_trait]
/// Provides request/responses according to a minimal subset of Xrpl RPC API
/// required for the bridge
pub trait BridgeXrplWebsocketApi {
	async fn transaction_entry_request(
		xrp_transaction: XrpTransaction,
		ledger_index: LedgerIndex,
		call_id: ChainCallId,
	) -> Result<Receiver<Result<XrplTxHash, BridgeRpcError>>, BridgeRpcError>;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionEntryResponse {
	pub result: Option<TransactionEntryResponseResult>,
	pub status: String,
	pub r#type: String,
	pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionEntryResponseResult {
	pub ledger_hash: String,
	pub ledger_index: u64,
	pub tx_json: Payment,
	pub validated: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Payment {
	#[serde(rename(deserialize = "Account"))]
	pub account: String,
	#[serde(rename(deserialize = "Amount"))]
	pub amount: String, // https://xrpl.org/basic-data-types.html#specifying-currency-amounts
	pub hash: String,
	#[serde(rename(deserialize = "Memos"))]
	pub memos: Option<Vec<Memo>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all(deserialize = "PascalCase"))]
pub struct Memo {
	pub memo_type: String,
	pub memo_data: String,
}
