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

//! Runtime API definition required by ASSETS-EXT RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use pallet_assets_ext::Config;
use seed_primitives::AssetId;
extern crate alloc;
use alloc::string::String;

sp_api::decl_runtime_apis! {
	/// The RPC API to interact with AssetExt module
	pub trait AssetsExtApi<AccountId, T> where
		AccountId: Codec,
		T: Config,
	{
		/// Find asset balance owned by `who` for a given assetId
		fn asset_balance(
			asset_id: AssetId,
			who: AccountId,
		) -> String;

	}
}
