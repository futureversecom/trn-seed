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
use crate::eth_types::EthereumEventInfo;
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use seed_primitives::validator::ChainId;

#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode, TypeInfo)]
pub enum SigningRequest {
	/// Request to sign an event for Ethereum
	Ethereum(EthereumEventInfo),
	/// Request to sign an XRPL tx (binary serialized in 'for signing' mode)
	XrplTx(Vec<u8>),
}

impl SigningRequest {
	/// Return the Chain Id associated with the signing request
	pub fn chain_id(&self) -> ChainId {
		match self {
			Self::Ethereum(_) => ChainId::Ethereum,
			Self::XrplTx { .. } => ChainId::Xrpl,
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
