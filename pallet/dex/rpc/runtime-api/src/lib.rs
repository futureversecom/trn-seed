// Copyright 2020-2021 Parity Technologies (UK) Ltd. and Centrality Investments Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Runtime API definition required by DEX RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]

use pallet_dex::Config;
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
	}
}
