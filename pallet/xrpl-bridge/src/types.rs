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
use xrpl_codec::types::CurrencyCodeType;

/// Payment id used for distinguishing pending withdrawals/ deposit events
pub type DelayedPaymentId = u64;

#[derive(
	RuntimeDebugNoBound, Eq, CloneNoBound, PartialEqNoBound, Encode, Decode, TypeInfo, MaxEncodedLen,
)]
pub struct XrpTransaction {
	pub transaction_hash: XrplTxHash,
	pub transaction: XrplTxData,
	pub timestamp: u64,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct DelayedWithdrawal<AccountId> {
	pub sender: AccountId,
	pub destination_tag: Option<u32>,
	pub withdraw_tx: WithdrawTransaction,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum WithdrawTransaction {
	XRP(XrpWithdrawTransaction),
	Asset(AssetWithdrawTransaction),
}

impl WithdrawTransaction {
	pub fn amount(&self) -> Balance {
		match self {
			WithdrawTransaction::XRP(tx) => tx.amount,
			WithdrawTransaction::Asset(tx) => tx.amount,
		}
	}

	pub fn destination(&self) -> XrplAccountId {
		match self {
			WithdrawTransaction::XRP(tx) => tx.destination,
			WithdrawTransaction::Asset(tx) => tx.destination,
		}
	}
}

/// Withdrawal transaction for the XRP Currency
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Copy)]
pub struct XrpWithdrawTransaction {
	pub tx_fee: u64,
	pub tx_nonce: XrplTxNonce,
	pub tx_ticket_sequence: XrplTxTicketSequence,
	pub amount: Balance,
	pub destination: XrplAccountId,
}

/// Withdrawal transaction for all other assets
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Copy)]
pub struct AssetWithdrawTransaction {
	pub tx_fee: u64,
	pub tx_nonce: XrplTxNonce,
	pub tx_ticket_sequence: XrplTxTicketSequence,
	pub amount: Balance,
	pub destination: XrplAccountId,
	pub asset_id: AssetId,
	pub currency: XRPLCurrencyType,
	pub issuer: XrplAccountId,
}

#[derive(Eq, CloneNoBound, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum XrplTxData {
	Payment { amount: Balance, address: H160 },
	CurrencyPayment { amount: Balance, address: H160, currency: XRPLCurrency },
	Xls20, // Nft
}

impl Default for XrpTransaction {
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

impl Default for XrplTxData {
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

/// Currency issued by issuer https://xrpl.org/docs/references/protocol/data-types/currency-formats#token-amounts
#[derive(Eq, Copy, Clone, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct XRPLCurrency {
	pub symbol: XRPLCurrencyType,
	pub issuer: XrplAccountId,
}

impl Default for XRPLCurrency {
	fn default() -> Self {
		XRPLCurrency {
			symbol: XRPLCurrencyType::NonStandard([0; 20]),
			issuer: XrplAccountId::default(),
		}
	}
}

/// Currency type on TRN to match the CurrencyCodeType from XRPL codec
/// Supports both 3 and 20 byte currency codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum XRPLCurrencyType {
	Standard([u8; 3]),
	NonStandard([u8; 20]),
}

impl XRPLCurrencyType {
	pub fn is_valid(&self) -> bool {
		let currency: CurrencyCodeType = (*self).into();
		currency.is_valid()
	}
}

impl From<XRPLCurrencyType> for CurrencyCodeType {
	fn from(currency: XRPLCurrencyType) -> Self {
		match currency {
			XRPLCurrencyType::Standard(currency) => Self::Standard(currency),
			XRPLCurrencyType::NonStandard(currency) => Self::NonStandard(currency),
		}
	}
}
