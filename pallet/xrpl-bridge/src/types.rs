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

use codec::{Decode, Encode};
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
use seed_primitives::{
	xrpl::{XrplAccountId, XrplTxHash, XrplTxNonce, XrplTxTicketSequence},
	AssetId, Balance,
};
use sp_core::H160;

/// Payment id used for distinguishing pending withdrawals/ deposit events
pub type DelayedPaymentId = u64;

#[derive(
	RuntimeDebugNoBound, Eq, CloneNoBound, PartialEqNoBound, Encode, Decode, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(XRPSymbolLimit))]
pub struct XrpTransaction<XRPSymbolLimit: Get<u32>> {
	pub transaction_hash: XrplTxHash,
	pub transaction: XrplTxData<XRPSymbolLimit>,
	pub timestamp: u64,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct DelayedWithdrawal<AccountId> {
	pub sender: AccountId,
	pub destination_tag: Option<u32>,
	pub withdraw_tx: XrpWithdrawTransaction,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Copy)]
pub struct XrpWithdrawTransaction {
	pub tx_fee: u64,
	pub tx_nonce: XrplTxNonce,
	pub tx_ticket_sequence: XrplTxTicketSequence,
	pub amount: Balance,
	pub destination: XrplAccountId,
}

#[derive(
	Eq, CloneNoBound, PartialEqNoBound, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(XRPSymbolLimit))]
pub enum XrplTxData<XRPSymbolLimit: Get<u32>> {
	Payment { amount: Balance, address: H160 },
	CurrencyPayment { amount: Balance, address: H160, currency: BoundedVec<u8, XRPSymbolLimit> },
	Xls20, // Nft
}

impl<XRPSymbolLimit: Get<u32>> Default for XrpTransaction<XRPSymbolLimit> {
	fn default() -> Self {
		XrpTransaction {
			transaction_hash: XrplTxHash::default(),
			transaction: XrplTxData::default(),
			timestamp: 0,
		}
	}
}

impl Default for XrpWithdrawTransaction {
	fn default() -> Self {
		XrpWithdrawTransaction {
			tx_fee: 0,
			tx_nonce: 0,
			tx_ticket_sequence: 0,
			amount: 0,
			destination: XrplAccountId::default(),
		}
	}
}

impl<XRPSymbolLimit: Get<u32>> Default for XrplTxData<XRPSymbolLimit> {
	fn default() -> Self {
		XrplTxData::Payment { amount: 0, address: H160::default() }
	}
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct XrplTicketSequenceParams {
	pub start_sequence: u32,
	pub bucket_size: u32,
}

impl Default for XrplTicketSequenceParams {
	fn default() -> Self {
		XrplTicketSequenceParams { start_sequence: 0_u32, bucket_size: 0_u32 }
	}
}

// Currency issued by issuer https://xrpl.org/docs/references/protocol/data-types/currency-formats#token-amounts
#[derive(
	Eq, CloneNoBound, PartialEqNoBound, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(XRPSymbolLimit))]
pub struct XRPLCurrency<XRPSymbolLimit: Get<u32>> {
	pub currency: BoundedVec<u8, XRPSymbolLimit>,
	pub issuer: XrplAccountId,
}

impl<XRPSymbolLimit: Get<u32>> Default for XRPLCurrency<XRPSymbolLimit> {
	fn default() -> Self {
		XRPLCurrency { currency: BoundedVec::default(), issuer: Default::default() }
	}
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct XRPLAsset {
	pub asset_id: AssetId,
	pub issuer: XrplAccountId,
}

impl Default for XRPLAsset {
	fn default() -> Self {
		XRPLAsset { asset_id: Default::default(), issuer: Default::default() }
	}
}
