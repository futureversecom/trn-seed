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
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_core::{H160, H256};

use seed_primitives::{
	xrpl::{XrplTxHash, XrplWithdrawAddress, XrplWithdrawTxNonce},
	Balance,
};

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct XrpTransaction {
	pub transaction_hash: XrplTxHash,
	pub transaction: XrplTxData,
	pub timestamp: u64,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct XrpWithdrawTransaction {
	pub tx_nonce: XrplWithdrawTxNonce,
	pub amount: Balance,
	pub destination: XrplWithdrawAddress,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub enum XrplTxData {
	Payment { amount: Balance, address: H160 },
	CurrencyPayment { amount: Balance, address: H160, currency_id: H256 },
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
			tx_nonce: 0,
			amount: 0,
			destination: XrplWithdrawAddress::default(),
		}
	}
}

impl Default for XrplTxData {
	fn default() -> Self {
		XrplTxData::Payment { amount: 0, address: H160::default() }
	}
}
