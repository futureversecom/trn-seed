/* Copyright 2021 Centrality Investments Limited
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
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{H160, H256, U256};
use sp_std::prelude::*;

/// Ethereum address type
pub type EthAddress = seed_primitives::EthAddress;

/// Claim id used for distinguishing pending withdrawals/ deposit claims
pub type ClaimId = u64;

/// States the origin of where the withdrawal call was made
pub enum WithdrawCallOrigin {
	// The withdrawal claim was called through the ERC20-Peg pallet
	Runtime,
	// The withdrawal claim was called through the EVM
	Evm,
}

/// A pending deposit or withdrawal
#[derive(Debug, Clone, Encode, Decode, PartialEq, TypeInfo, MaxEncodedLen)]
pub enum PendingClaim {
	/// A deposit claim (deposit_claim, tx_hash)
	Deposit(Erc20DepositEvent),
	/// A withdrawal (withdrawal_message)
	Withdrawal(WithdrawMessage),
}

/// A deposit event made by the ERC20 peg contract on Ethereum
#[derive(Debug, Default, Clone, PartialEq, Decode, Encode, TypeInfo, MaxEncodedLen)]
pub struct Erc20DepositEvent {
	/// The ERC20 token address / type deposited
	/// `0` indicates native Eth
	pub token_address: EthAddress,
	/// The amount (in 'wei') of the deposit
	pub amount: U256,
	/// The Seed beneficiary address
	pub beneficiary: H160,
}

/// A withdraw message to prove and submit to Ethereum
/// Allowing redemption of ERC20s
#[derive(Debug, Default, Clone, PartialEq, Decode, Encode, TypeInfo, MaxEncodedLen)]
pub struct WithdrawMessage {
	/// The ERC20 token address / type deposited
	pub token_address: EthAddress,
	/// The amount (in 'wei') of the deposit
	pub amount: U256,
	/// The Ethereum beneficiary address
	pub beneficiary: EthAddress,
}
