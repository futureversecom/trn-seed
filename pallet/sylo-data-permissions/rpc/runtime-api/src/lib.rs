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

//! Runtime API definition required by SYLO DATA PERMISSIONS RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use alloc::{string::String, vec::Vec};
use codec::Codec;
use pallet_sylo_data_permissions::GetPermissionsResult;
use sp_runtime::DispatchError;

sp_api::decl_runtime_apis! {
	/// The RPC API to interact with SyloDataPermissions module
	pub trait SyloDataPermissionsApi<AccountId> where
		AccountId: Codec,
	{
		/// Queries the pallet's storage for all available permissions
		/// for the given data ids that have been granted to a specific account.
		///
		/// The response value will include a mapping from each data id to a
		/// vector of the onchain permissions that have been granted to that
		/// account.It will also include the off-chain permission reference if it
		/// exists.
		///
		/// Clients can use this RPC call to avoid making multiple separate queries
		/// to onchain storage.
		fn get_permissions(
			data_author: AccountId,
			grantee: AccountId,
			data_ids: Vec<String>,
		) -> Result<GetPermissionsResult, DispatchError>;
	}
}
