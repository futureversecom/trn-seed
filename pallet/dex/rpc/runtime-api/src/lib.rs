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

//! Runtime API definition required by DEX RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]

use pallet_dex::{Config, TradingPairStatus};
use seed_primitives::types::{AssetId, Balance};
use sp_runtime::DispatchError;
use sp_std::prelude::*;

sp_api::decl_runtime_apis! {
	/// The RPC API to interact with DEX module
	pub trait DexApi<T> where
		T: Config,
	{
		/// Returns amount of output token that can be obtained by swapping an amount of input token
		fn quote(
			amount_a: u128,
			reserve_a: u128,
			reserve_b: u128,
		) -> Result<u128, DispatchError>;

		/// Returns the amount of output tokens that you would receive if you sent an amount of input tokens
		fn get_amounts_out(
			amount_in: Balance,
			path: Vec<AssetId>,
		) -> Result<Vec<Balance>, DispatchError>;

		/// Returns the amount of input tokens that you would need to send to receive an amount of output tokens
		fn get_amounts_in(
			amount_out: Balance,
			path: Vec<AssetId>,
		) -> Result<Vec<Balance>, DispatchError>;

		/// Returns the LP token ID from the given trading pair
		fn get_lp_token_id(
		asset_id_a: AssetId,
		asset_id_b: AssetId,
		) -> Result<AssetId, DispatchError>;

		/// Returns the liquidity balances of the given trading pair
		fn get_liquidity(
		asset_id_a: AssetId,
		asset_id_b: AssetId,
		) -> (Balance, Balance);

		/// Returns the status of the given trading pairs
		fn get_trading_pair_status(
		asset_id_a: AssetId,
		asset_id_b: AssetId,
		) -> TradingPairStatus;
	}
}
