/* Copyright 2019-2021 Centrality Investments Limited
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
use seed_primitives::{
	validators::validator::{EventProofId, ValidatorSetId},
	AccountId,
};
use sp_runtime::KeyTypeId;

/// The session key type for bridge
pub const BRIDGE_KEY_TYPE: KeyTypeId = KeyTypeId(*b"brg-");

/// Crypto types for bridge protocol
pub mod crypto {
	mod app_crypto {
		use crate::helpers::BRIDGE_KEY_TYPE;
		use sp_application_crypto::{app_crypto, ecdsa};
		app_crypto!(ecdsa, BRIDGE_KEY_TYPE);
	}
	sp_application_crypto::with_pair! {
		/// bridge keypair using ecdsa as its crypto.
		pub type AuthorityPair = app_crypto::Pair;
	}
	/// bridge signature using ecdsa as its crypto.
	pub type AuthoritySignature = app_crypto::Signature;
	/// bridge identifier using ecdsa as its crypto.
	pub type AuthorityId = app_crypto::Public;
}

/// A set of authorities, a.k.a. validators.
#[derive(Decode, Encode, Debug, PartialEq, Clone)]
pub struct ValidatorSet<AuthorityId> {
	/// Public keys of the validator set elements
	pub validators: Vec<AuthorityId>,
	/// Identifier of the validator set
	pub id: ValidatorSetId,
	/// Minimum number of validator signatures required for a valid proof (i.e 'm' in 'm-of-n')
	pub proof_threshold: u32,
}

impl<AuthorityId> ValidatorSet<AuthorityId> {
	#[allow(dead_code)]
	/// Return an empty validator set with id of 0.
	pub fn empty() -> Self {
		Self { validators: Default::default(), id: Default::default(), proof_threshold: 0 }
	}
}

/// A consensus log item
#[derive(Decode, Encode)]
pub enum ConsensusLog<AuthorityId: Encode + Decode> {
	#[codec(index = 1)]
	/// Signal an `AuthoritiesChange` is scheduled for next session
	/// Generate a proof that the current validator set has witnessed the new authority set
	PendingAuthoritiesChange(PendingAuthorityChange<AuthorityId>),
}
/// Authority change data
#[derive(Decode, Encode)]
pub struct PendingAuthorityChange<AuthorityId: Encode + Decode> {
	/// The source of the change
	pub source: AccountId,
	/// The destination for the change
	pub destination: AccountId,
	/// The next validator set (ordered)
	pub next_validator_set: ValidatorSet<AuthorityId>,
	/// The event proof Id for this request
	pub event_proof_id: EventProofId,
}
