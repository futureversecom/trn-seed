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
use ethereum_types::{Bloom, U64};
use rustc_hex::ToHex;
use scale_info::TypeInfo;
use serde::{
	de::{Error, Visitor},
	Deserialize, Deserializer, Serialize, Serializer,
};
pub use sp_core::{H160, H256, U256};
use sp_runtime::{DispatchError, RuntimeDebug};
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

use seed_primitives::ethy::ValidatorSetId;
pub use seed_primitives::{
	ethy::{ConsensusLog, EthyChainId, EventClaimId, EventProofId, ValidatorSet, ETHY_ENGINE_ID},
	BlockNumber,
};

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