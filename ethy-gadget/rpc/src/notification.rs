// Copyright (C) 2020-2022 Parity Technologies (UK) Ltd. & Centrality Investments Ltd
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use serde::{Deserialize, Serialize};
use sp_core::{Bytes, H256};

use seed_primitives::{
	ethy::{EventProofId, ValidatorSetId},
	AccountId20,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct EventProofResponse {
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
pub struct XrplTxProofResponse {
	/// The event proof Id
	pub event_id: EventProofId,
	/// The signatures in the request
	pub signatures: Vec<Bytes>,
	/// The block hash of the event (finalized)
	pub block: H256,
}
