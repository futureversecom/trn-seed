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

use codec::{Decode, Encode};
use core::fmt;
use rustc_hex::ToHex;
use scale_info::TypeInfo;
use serde::{
	de::{Error, Visitor},
	Deserialize, Deserializer, Serialize, Serializer,
};
pub use sp_core::{H160, H256, U256};
use sp_runtime::RuntimeDebug;
use sp_std::{prelude::*, vec::Vec};
use seed_primitives::validators::validator::{EventProofId, ValidatorSetId};

pub type XrplAddress = seed_primitives::XrplWithdrawAddress;

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