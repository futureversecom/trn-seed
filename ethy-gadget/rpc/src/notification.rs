// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

use serde::{Deserialize, Serialize};
use sp_core::{Bytes, H256};

use seed_primitives::{
	ethy::{EventProofId, ValidatorSetId},
	AccountId20,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct EthEventProofResponse {
	/// The event proof Id
	pub event_id: EventProofId,
	/// The signatures in the request
	pub signatures: Vec<Bytes>,
	/// The validators that signed the request
	pub validators: Vec<AccountId20>,
	/// The validators set Id that signed the proof
	pub validator_set_id: ValidatorSetId,
	/// THe block hash of the event (finalized)
	pub block: H256,
	/// Metadata tag
	pub tag: Option<Bytes>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct XrplEventProofResponse {
	/// The event proof Id
	pub event_id: EventProofId,
	/// The Xrpl validator signatures in the request
	pub signatures: Vec<Bytes>,
	/// The Xrpl validators that signed the request
	pub validators: Vec<Bytes>,
	/// The validators set Id that signed the proof
	pub validator_set_id: ValidatorSetId,
	/// THe block hash of the event (finalized)
	pub block: H256,
	/// Metadata tag
	pub tag: Option<Bytes>,
}
