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

//! Runtime API definition required by SFT RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]

use pallet_sft::Config;
use seed_primitives::types::TokenId;
use sp_std::prelude::*;

sp_api::decl_runtime_apis! {
	/// The RPC API to interact with SFT module
	pub trait SftApi<T> where
		T: Config,
	{
		/// Return the token metadata URI for a given token
		fn token_uri(token_id: TokenId) -> Vec<u8>;
	}
}
