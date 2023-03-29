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
use ethereum_types::{Bloom, U64};
use rustc_hex::ToHex;
use scale_info::TypeInfo;
use serde::{
	de::{Error, Visitor},
	Deserialize, Deserializer, Serialize, Serializer,
};
pub use sp_core::{H160, H256, U256};
use sp_runtime::RuntimeDebug;
use sp_std::{prelude::*, vec::Vec};

// following imports support serializing values to hex strings in no_std
#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::borrow::ToOwned;
#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(feature = "std")]
use std::string::String;

pub use seed_primitives::{
	ethy::{ConsensusLog, EthyChainId, EventClaimId, EventProofId, ValidatorSet, ETHY_ENGINE_ID},
	BlockNumber,
};

/// An EthCallOracle call Id
pub type EthCallId = u64;
/// An EthCallOracle request
#[derive(Encode, Decode, Default, PartialEq, Clone, TypeInfo)]
pub struct CheckedEthCallRequest {
	/// EVM input data for the call
	pub input: Vec<u8>,
	/// Ethereum address to receive the call
	pub target: EthAddress,
	/// CENNZnet timestamp when the original request was placed e.g by a contract/user (seconds)
	pub timestamp: u64,
	/// Informs the oldest acceptable block number that `try_block_number` can take (once the
	/// Ethereum latest block number is known) if `try_block_number` falls outside `(latest -
	/// max_block_look_behind) < try_block_number < latest` then it is considered invalid
	pub max_block_look_behind: u64,
	/// Hint at an Ethereum block # for the call (i.e. near `timestamp`)
	/// It is provided by an untrusted source and may or may not be used
	/// depending on its distance from the latest eth block i.e `(latest - max_block_look_behind) <
	/// try_block_number < latest`
	pub try_block_number: u64,
	/// CENNZnet timestamp when _this_ check request was queued (seconds)
	pub check_timestamp: u64,
}
#[derive(Encode, Decode, Debug, Eq, PartialOrd, Ord, PartialEq, Copy, Clone, TypeInfo)]
pub enum CheckedEthCallResult {
	/// returndata obtained, ethereum block number, ethereum timestamp
	Ok([u8; 32], u64, u64),
	/// returndata obtained, exceeds length limit
	ReturnDataExceedsLimit,
	/// returndata obtained, empty
	ReturnDataEmpty,
	/// Failed to retrieve all the required data from Ethereum
	DataProviderErr,
	/// Ethereum block number is invalid (0, max)
	InvalidEthBlock,
	/// Timestamps have desynced or are otherwise invalid
	InvalidTimestamp,
}

/// The ethereum block number data type
pub type EthBlockNumber = U64;
/// The ethereum address data type
pub type EthAddress = seed_primitives::EthAddress;
/// The ethereum transaction hash type
pub type EthHash = H256;

#[derive(Debug, Default, Clone, PartialEq, Eq, Decode, Encode, TypeInfo)]
/// Info required to claim an Ethereum event
pub struct EventClaim {
	/// The Ethereum transaction hash which caused the event
	pub tx_hash: EthHash,
	/// The source address (contract) which posted the event
	pub source: EthAddress,
	/// The destination address (contract) which should receive the event
	/// It may be symbolic, mapping to a pallet vs. a deployed contract
	pub destination: EthAddress,
	/// The Ethereum ABI encoded event data as logged on Ethereum
	pub data: Vec<u8>,
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
	/// The Tx Receipt was not present
	NoTxReceipt,
	/// The event source did not match the tx receipt `to` field
	UnexpectedSource,
}

/// Current status of a pending event claim
/// Invalid claims get removed from storage so no need to have an enum variant for ProvedInvalid
#[derive(Decode, Encode, Debug, PartialEq, Clone, TypeInfo)]
pub enum EventClaimStatus {
	/// The event is awaiting processing after the challenge period
	Pending,
	/// The event has been challenged and is awaiting notarization
	Challenged,
	/// The event has been challenged and has been proven to be valid
	/// This event will now be processed after the challenge period
	ProvenValid,
}

/// An independent notarization of a bridged value
/// This is signed and shared with the runtime after verification by a particular validator
#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo)]
#[repr(u8)]
pub enum NotarizationPayload {
	Call {
		/// The call Id being notarized
		call_id: EthCallId,
		/// The ordinal index of the signer in the notary set
		/// It may be used with chain storage to lookup the public key of the notary
		authority_index: u16,
		/// Result of the notarization check by this authority
		result: CheckedEthCallResult,
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

/// Provides request/responses according to a minimal subset of Ethereum RPC API
/// required for the bridge
pub trait BridgeEthereumRpcApi {
	/// Returns an ethereum block given a block height
	fn get_block_by_number(
		block_number: LatestOrNumber,
	) -> Result<Option<EthBlock>, BridgeRpcError>;
	/// Returns an ethereum transaction receipt given a tx hash
	fn get_transaction_receipt(hash: EthHash)
		-> Result<Option<TransactionReceipt>, BridgeRpcError>;
	/// Performs an `eth_call` request
	/// Returns the Ethereum abi encoded returndata as a Vec<u8>
	fn eth_call(
		target: EthAddress,
		input: &[u8],
		at_block: LatestOrNumber,
	) -> Result<Vec<u8>, BridgeRpcError>;
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

/// Log
#[derive(Debug, Deserialize, PartialEq, Eq, Hash, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Log {
	/// address
	pub address: EthAddress,
	/// Topics
	pub topics: Vec<H256>,
	/// Data
	#[serde(deserialize_with = "deserialize_hex")]
	pub data: Vec<u8>,
	/// Block Hash
	pub block_hash: H256,
	/// Block Number
	pub block_number: EthBlockNumber,
	/// Transaction Hash
	pub transaction_hash: Option<H256>,
	/// Transaction Index
	pub transaction_index: U64,
	/// Log Index in Block
	pub log_index: U256,
	/// Whether Log Type is Removed (Geth Compatibility Field)
	#[serde(default)]
	pub removed: bool,
}

// Copied from https://docs.rs/web3/0.14.0/src/web3/types/transaction.rs.html#40-73
// missing from/to fields
// remove after: https://github.com/tomusdrw/rust-web3/issues/513 is solved
/// "Receipt" of an executed transaction: details of its execution.
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionReceipt {
	/// Hash of the block this transaction was included within.
	pub block_hash: H256,
	/// Number of the block this transaction was included within.
	pub block_number: EthBlockNumber,
	/// Contract address created, or `None` if not a deployment.
	pub contract_address: Option<EthAddress>,
	/// Cumulative gas used within the block after this was executed.
	pub cumulative_gas_used: U256,
	pub effective_gas_price: Option<U256>,
	/// Address of the sender.
	pub from: EthAddress,
	/// Gas used by this transaction alone.
	///
	/// Gas used is `None` if the the client is running in light client mode.
	pub gas_used: Option<U256>,
	/// Logs generated within this transaction.
	pub logs: Vec<Log>,
	/// Status: either 1 (success) or 0 (failure).
	pub status: Option<U64>,
	/// Address of the receiver, or `None` if a contract deployment
	pub to: Option<EthAddress>,
	/// Transaction hash.
	pub transaction_hash: H256,
	/// Index within the block.
	pub transaction_index: U64,
	/// State root.
	pub root: Option<H256>,
	/// Logs bloom
	pub logs_bloom: Bloom,
	/// Transaction type, Some(1) for AccessList transaction, None for Legacy
	#[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
	pub transaction_type: Option<U64>,
	#[serde(default)]
	pub removed: bool,
}

/// Standard Eth block type
///
/// NB: for the bridge we only need the `timestamp` however the only RPCs available require fetching
/// the whole block
#[derive(Clone, Debug, PartialEq, Deserialize, Default)]
pub struct EthBlock {
	pub number: Option<U64>,
	pub hash: Option<H256>,
	pub timestamp: U256,
	// don't deserialize anything else
	#[serde(rename = "parentHash", skip_deserializing)]
	pub parent_hash: H256,
	#[serde(skip_deserializing)]
	pub nonce: Option<U64>,
	#[serde(rename = "sha3Uncles", skip_deserializing)]
	pub sha3_uncles: H256,
	#[serde(rename = "logsBloom", skip_deserializing)]
	pub logs_bloom: Option<Bloom>,
	#[serde(rename = "transactionsRoot", skip_deserializing)]
	pub transactions_root: H256,
	#[serde(rename = "stateRoot", skip_deserializing)]
	pub state_root: H256,
	#[serde(rename = "receiptsRoot", skip_deserializing)]
	pub receipts_root: H256,
	#[serde(skip_deserializing)]
	pub miner: EthAddress,
	#[serde(skip_deserializing)]
	pub difficulty: U256,
	#[serde(rename = "totalDifficulty", skip_deserializing)]
	pub total_difficulty: U256,
	#[serde(rename = "extraData", skip_deserializing)]
	pub extra_data: Vec<u8>,
	#[serde(skip_deserializing)]
	pub size: U256,
	#[serde(rename = "gasLimit", skip_deserializing)]
	pub gas_limit: U256,
	#[serde(rename = "gasUsed", skip_deserializing)]
	pub gas_used: U256,
	#[serde(skip_deserializing)]
	pub transactions: Vec<H256>,
	#[serde(skip_deserializing)]
	pub uncles: Vec<H256>,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, TypeInfo)]
pub struct EthResponse<'a, D> {
	jsonrpc: &'a str,
	id: u32,
	pub result: Option<D>,
}

/// JSON-RPC protocol version header
const JSONRPC: &str = "2.0";

/// Request for 'eth_getTransactionReceipt'
#[derive(Serialize, Debug)]
pub struct GetTxReceiptRequest {
	#[serde(rename = "jsonrpc")]
	/// The version of the JSON RPC spec
	pub json_rpc: &'static str,
	/// The method which is called
	pub method: &'static str,
	/// Arguments supplied to the method. Can be an empty Vec.
	pub params: [H256; 1],
	/// The id for the request
	pub id: usize,
}

/// JSON-RPC method name for the request
const METHOD_TX: &str = "eth_getTransactionReceipt";
impl GetTxReceiptRequest {
	pub fn new(tx_hash: H256, id: usize) -> Self {
		Self { json_rpc: JSONRPC, method: METHOD_TX, params: [tx_hash], id }
	}
}

const METHOD_GET_BLOCK_BY_NUMBER: &str = "eth_getBlockByNumber";
/// Request for 'eth_blockNumber'
#[derive(Serialize, Debug)]
pub struct GetBlockRequest {
	#[serde(rename = "jsonrpc")]
	/// The version of the JSON RPC spec
	pub json_rpc: &'static str,
	/// The method which is called
	pub method: &'static str,
	/// Arguments supplied to the method. (blockNumber, fullTxData?)
	#[serde(serialize_with = "serialize_params")]
	pub params: (LatestOrNumber, bool),
	/// The id for the request
	pub id: usize,
}

#[derive(Debug)]
pub enum LatestOrNumber {
	Latest,
	Number(u64),
}

const METHOD_ETH_CALL: &str = "eth_call";
/// Request for 'eth_call'
#[derive(Serialize, Debug)]
pub struct EthCallRpcRequest {
	#[serde(rename = "jsonrpc")]
	/// The version of the JSON RPC spec
	pub json_rpc: &'static str,
	/// The method which is called
	pub method: &'static str,
	/// Arguments supplied to the method. (blockNumber, fullTxData?)
	#[serde(serialize_with = "serialize_params_eth_call")]
	pub params: (EthCallRpcParams, LatestOrNumber),
	/// The id for the request
	pub id: usize,
}
#[derive(Serialize, Debug)]
pub struct EthCallRpcParams {
	/// The contract to call
	pub to: EthAddress,
	/// The input buffer to pass to `to`
	pub data: Bytes,
}
impl EthCallRpcRequest {
	pub fn new(target: EthAddress, input: &[u8], id: usize, block: LatestOrNumber) -> Self {
		Self {
			json_rpc: JSONRPC,
			method: METHOD_ETH_CALL,
			params: (EthCallRpcParams { to: target, data: Bytes::new(input.to_vec()) }, block),
			id,
		}
	}
}

/// Serializes the parameters for `GetBlockRequest`
pub fn serialize_params<S: serde::Serializer>(
	v: &(LatestOrNumber, bool),
	s: S,
) -> Result<S::Ok, S::Error> {
	use core::fmt::Write;
	use serde::ser::SerializeTuple;

	let mut tup = s.serialize_tuple(2)?;
	match v.0 {
		LatestOrNumber::Latest => tup.serialize_element(&"latest")?,
		LatestOrNumber::Number(n) => {
			// Ethereum JSON RPC API expects the block number as a hex string
			let mut hex_block_number = sp_std::Writer::default();
			write!(&mut hex_block_number, "{:#x}", n).expect("valid bytes");
			// this should always be valid utf8
			tup.serialize_element(
				&core::str::from_utf8(hex_block_number.inner()).expect("valid bytes"),
			)?;
		},
	}
	tup.serialize_element(&v.1)?;
	tup.end()
}

/// Serializes the parameters for `EthCallRequest`
pub fn serialize_params_eth_call<S: serde::Serializer>(
	v: &(EthCallRpcParams, LatestOrNumber),
	s: S,
) -> Result<S::Ok, S::Error> {
	use core::fmt::Write;
	use serde::ser::SerializeTuple;

	let mut tup = s.serialize_tuple(2)?;
	tup.serialize_element(&v.0)?;
	match v.1 {
		LatestOrNumber::Latest => tup.serialize_element(&"latest")?,
		LatestOrNumber::Number(n) => {
			// Ethereum JSON RPC API expects the block number as a hex string
			let mut hex_block_number = sp_std::Writer::default();
			write!(&mut hex_block_number, "{:#x}", n).expect("valid bytes");
			// this should always be valid utf8
			tup.serialize_element(
				&core::str::from_utf8(hex_block_number.inner()).expect("valid bytes"),
			)?;
		},
	}
	tup.end()
}

/// JSON-RPC method name for the request
impl GetBlockRequest {
	pub fn for_number(id: usize, block_number: u64) -> Self {
		Self {
			json_rpc: JSONRPC,
			method: METHOD_GET_BLOCK_BY_NUMBER,
			params: (LatestOrNumber::Number(block_number), false), /* `false` = return tx hashes
			                                                        * not full tx data */
			id,
		}
	}
	pub fn latest(id: usize) -> Self {
		Self {
			json_rpc: JSONRPC,
			method: METHOD_GET_BLOCK_BY_NUMBER,
			params: (LatestOrNumber::Latest, false), // `false` = return tx hashes not full tx data
			id,
		}
	}
}

// Serde deserialize hex string, expects prefix '0x'
pub fn deserialize_hex<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
	deserializer.deserialize_str(BytesVisitor)
}

/// Deserializes "0x" prefixed hex strings into Vec<u8>s
pub(crate) struct BytesVisitor;
impl<'a> Visitor<'a> for BytesVisitor {
	type Value = Vec<u8>;

	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		write!(formatter, "a 0x-prefixed, hex-encoded vector of bytes")
	}

	fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
	where
		E: Error,
	{
		if value.len() >= 2 && value.starts_with("0x") && value.len() & 1 == 0 {
			Ok(decode_hex(&value[2..]).expect("it is hex"))
		} else {
			Err(Error::custom(
				"Invalid bytes format. Expected a 0x-prefixed hex string with even length",
			))
		}
	}
}

/// Wrapper structure around vector of bytes.
#[derive(Debug, PartialEq, Eq, Default, Hash, Clone)]
pub struct Bytes(pub Vec<u8>);

impl Bytes {
	/// Simple constructor.
	pub fn new(bytes: Vec<u8>) -> Bytes {
		Bytes(bytes)
	}
}

impl Serialize for Bytes {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut serialized = "0x".to_owned();
		serialized.push_str(self.0.to_hex::<String>().as_ref());
		serializer.serialize_str(serialized.as_ref())
	}
}

impl<'a> Deserialize<'a> for Bytes {
	fn deserialize<D>(deserializer: D) -> Result<Bytes, D::Error>
	where
		D: Deserializer<'a>,
	{
		deserializer.deserialize_any(BytesVisitor).map(Bytes::new)
	}
}

// decode a non-0x prefixed hex string into a `Vec<u8>`
fn decode_hex(s: &str) -> Result<Vec<u8>, core::num::ParseIntError> {
	(0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16)).collect()
}
