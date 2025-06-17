// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

use core::fmt::Debug;

use alloc::string::String;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{traits::Get, CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound};
use libsecp256k1::Message;
use pallet_xrpl::types::{encode_for_signing, sha512_first_half, XRPLTransaction, XrplPublicKey};
use scale_info::TypeInfo;
use seed_primitives::Balance;
use sp_core::{ed25519, hexdisplay::AsBytesRef, H160, U256};
use sp_io::hashing::keccak_256;
use sp_runtime::traits::Verify;
use sp_runtime::{BoundedBTreeSet, BoundedVec};
use xrpl_types::types::TransactionCommon;

/// The memo type data to be hex encoded in XRPL transaction for transact permission token signature.
pub const MEMO_TYPE_TOKEN: &str = "extrinsic";

#[derive(Encode, Decode, TypeInfo, Copy, MaxEncodedLen, Debug, Clone, PartialEq, Eq)]
pub enum Spender {
	GRANTOR,
	GRANTEE,
}

// Tuple of pallet name and extrinsic name which identifies a specific runtime call.
pub type CallId<StringLimit> = (BoundedVec<u8, StringLimit>, BoundedVec<u8, StringLimit>);

pub fn to_call_id<StringLimit>(pallet: &str, function: &str) -> CallId<StringLimit>
where
	StringLimit: Get<u32>,
{
	(
		BoundedVec::truncate_from(pallet.as_bytes().to_vec()),
		BoundedVec::truncate_from(function.as_bytes().to_vec()),
	)
}

#[derive(
	CloneNoBound, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebugNoBound, PartialEqNoBound, Eq,
)]
#[scale_info(skip_type_params(MaxCallIds, StringLimit))]
pub struct TransactPermission<BlockNumber, MaxCallIds, StringLimit>
where
	BlockNumber: Debug + PartialEq + Clone,
	MaxCallIds: Get<u32>,
	StringLimit: Get<u32>,
{
	// Whether the extrinsic will be paid from the grantor or grantee
	pub spender: Spender,

	// If grantor is spender, then allow grantor to set a limit on the
	// amount the grantee is allowed to spend
	pub spending_balance: Option<Balance>,

	// Optional set of calls (pallet, extrinsic name) that this dispatch
	// permission is valid for. If None, then all extrinsics are allowed.
	pub allowed_calls: BoundedBTreeSet<CallId<StringLimit>, MaxCallIds>,

	// The block number this permission was established
	pub block: BlockNumber,

	// An optional expiry for this permission
	pub expiry: Option<BlockNumber>,
}

#[derive(
	CloneNoBound, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebugNoBound, PartialEqNoBound, Eq,
)]
#[scale_info(skip_type_params(MaxCallIds, StringLimit))]
pub struct TransactPermissionToken<AccountId, BlockNumber, MaxCallIds, StringLimit>
where
	AccountId: Debug + PartialEq + Clone,
	BlockNumber: Debug + PartialEq + Clone,
	MaxCallIds: Get<u32>,
	StringLimit: Get<u32>,
{
	// Specifies the intended grantee of the permission that will be created
	pub grantee: AccountId,

	// Optional field that indicates that the futurepass account of the permission grantor
	// should be used, instead of the account recovered from the signature.
	pub futurepass: Option<AccountId>,

	// The spender of transact fee
	pub spender: Spender,

	// The spending balance if the spender is set as the grantor
	pub spending_balance: Option<Balance>,

	// Optional set of allowed calls
	pub allowed_calls: BoundedBTreeSet<CallId<StringLimit>, MaxCallIds>,

	// An optional expiry for this permission
	pub expiry: Option<BlockNumber>,

	// A randomly generated 32 byte nonce used to prevent replays
	pub nonce: U256,
}

#[derive(
	CloneNoBound, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebugNoBound, PartialEqNoBound, Eq,
)]
#[scale_info(skip_type_params(XrplMaxMessageLength, XrplMaxSignatureLength))]
pub struct XrplTokenSignature<XrplMaxMessageLength, XrplMaxSignatureLength>
where
	XrplMaxMessageLength: Get<u32>,
	XrplMaxSignatureLength: Get<u32>,
{
	pub encoded_msg: BoundedVec<u8, XrplMaxMessageLength>,
	pub signature: BoundedVec<u8, XrplMaxSignatureLength>,
}

#[derive(
	CloneNoBound, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebugNoBound, PartialEqNoBound, Eq,
)]
#[scale_info(skip_type_params(XrplMaxMessageLength, XrplMaxSignatureLength))]
pub enum TransactPermissionTokenSignature<XrplMaxMessageLength, XrplMaxSignatureLength>
where
	XrplMaxMessageLength: Get<u32>,
	XrplMaxSignatureLength: Get<u32>,
{
	EIP191([u8; 65]),
	XRPL(XrplTokenSignature<XrplMaxMessageLength, XrplMaxSignatureLength>),
}

impl<XrplMaxMessageLength, XrplMaxSignatureLength>
	TransactPermissionTokenSignature<XrplMaxMessageLength, XrplMaxSignatureLength>
where
	XrplMaxMessageLength: Get<u32>,
	XrplMaxSignatureLength: Get<u32>,
{
	pub fn verify_signature<
		AccountId: Debug + PartialEq + Clone + Decode + Encode + From<H160>,
		BlockNumber: Debug + PartialEq + Clone + Decode + Encode,
		MaxCallIds: Get<u32>,
		StringLimit: Get<u32>,
	>(
		&self,
		token: &TransactPermissionToken<AccountId, BlockNumber, MaxCallIds, StringLimit>,
	) -> Result<AccountId, &'static str> {
		match self {
			Self::EIP191(signature) => {
				let encoded_token = Encode::encode(token);
				let eth_signed_msg = keccak_256(
					seed_primitives::ethereum_signed_message(encoded_token.as_bytes_ref()).as_ref(),
				);
				let recovered_pubkey =
					sp_io::crypto::secp256k1_ecdsa_recover(signature, &eth_signed_msg)
						.map_err(|_| "Failed to recover public key")?;
				let account = H160::from_slice(&keccak_256(&recovered_pubkey)[12..]);
				Ok(account.into())
			},
			Self::XRPL(xrpl_signature) => {
				let transaction = XRPLTokenTransaction::try_from(&xrpl_signature.encoded_msg[..])
					.map_err(|_| "Failed to create XRPLTokenTransaction")?;
				transaction.verify_token_signature(token, &xrpl_signature.signature)
			},
		}
	}
}

pub struct XRPLTokenTransaction {
	pub xrpl_transaction: XRPLTransaction,
}

impl XRPLTokenTransaction {
	pub fn extract_token<
		AccountId: Debug + PartialEq + Clone + Decode,
		BlockNumber: Debug + PartialEq + Clone + Decode,
		MaxCallIds: Get<u32>,
		StringLimit: Get<u32>,
	>(
		&self,
	) -> Result<
		TransactPermissionToken<AccountId, BlockNumber, MaxCallIds, StringLimit>,
		&'static str,
	> {
		for memo_elm in &self.xrpl_transaction.memos {
			let hex_decoded_type = hex::decode(&memo_elm.memo.memo_type)
				.map_err(|_| "failed to decode memo_type as hex")?;
			let memo_type = String::from_utf8(hex_decoded_type)
				.map_err(|_| "failed to convert memo_type to utf8")?;
			if memo_type.eq_ignore_ascii_case(MEMO_TYPE_TOKEN) {
				let hex_decoded_data = hex::decode(&memo_elm.memo.memo_data)
					.map_err(|_| "failed to decode memo_data as hex")?;

				let token: TransactPermissionToken<
					AccountId,
					BlockNumber,
					MaxCallIds,
					StringLimit,
				> = Decode::decode(&mut &hex_decoded_data[..])
					.map_err(|_| "failed to decode memo_data into TransactPermissionToken")?;

				return Ok(token);
			}
		}

		Err("XRPL token signature not found in transaction memos")
	}

	pub fn verify_token_signature<
		AccountId: Debug + PartialEq + Clone + Decode + Encode + From<H160>,
		BlockNumber: Debug + PartialEq + Clone + Decode + Encode,
		MaxCallIds: Get<u32>,
		StringLimit: Get<u32>,
	>(
		&self,
		token: &TransactPermissionToken<AccountId, BlockNumber, MaxCallIds, StringLimit>,
		signature: &[u8],
	) -> Result<AccountId, &'static str> {
		// Extract the token from the XRPL transaction
		let extracted_token: TransactPermissionToken<
			AccountId,
			BlockNumber,
			MaxCallIds,
			StringLimit,
		> = self.extract_token()?;

		// Ensure the passed-in token matches the extracted token
		if token != &extracted_token {
			return Err(
				"Provided token does not match the token extracted from the XRPL transaction",
			);
		}

		let tx_common: TransactionCommon = (&self.xrpl_transaction).try_into()?;
		let encoded_message = encode_for_signing(&tx_common)?;

		let public_key = self.xrpl_transaction.get_public_key()?;

		match public_key {
			XrplPublicKey::ED25519(public) => {
				let signature = ed25519::Signature::from_raw(
					signature.try_into().map_err(|_| "Invalid signature length")?,
				);
				if signature.verify(&*encoded_message, &public) {
					let account = self.xrpl_transaction.get_account()?;
					Ok(account.into())
				} else {
					Err("Invalid ED25519 signature")
				}
			},
			XrplPublicKey::ECDSA(public) => {
				let hashed_msg: Message =
					libsecp256k1::Message::parse(&sha512_first_half(&encoded_message));
				let pub_key = libsecp256k1::PublicKey::parse_compressed(&public.0)
					.map_err(|_| "Failed to parse public key")?;
				let signature = libsecp256k1::Signature::parse_der(signature)
					.map_err(|_| "Failed to parse signature")?;
				if libsecp256k1::verify(&hashed_msg, &signature, &pub_key) {
					let account = self.xrpl_transaction.get_account()?;
					Ok(account.into())
				} else {
					Err("Invalid ECDSA signature")
				}
			},
		}
	}
}

impl TryFrom<&[u8]> for XRPLTokenTransaction {
	type Error = &'static str;

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		let xrpl_transaction = XRPLTransaction::try_from(value)?;
		Ok(Self { xrpl_transaction })
	}
}
