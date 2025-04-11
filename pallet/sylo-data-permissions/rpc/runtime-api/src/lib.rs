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

use alloc::string::String;
use codec::Codec;
use pallet_sylo_data_permissions::HasPermissionQueryResult;
use seed_pallet_common::sylo::DataPermission;
use sp_runtime::DispatchError;

sp_api::decl_runtime_apis! {
	/// The RPC API to interact with SyloDataPermissions module
	pub trait SyloDataPermissionsApi<AccountId> where
		AccountId: Codec,
	{
		/// Checks if an account has the given permission for the given
		/// data_id. This will query both the onchain Permission Records, and
		/// also the Tagged Permission Records.
		///
		/// This query will also return the offchain permission record if one
		/// exists, and the resolver endpoints for the offchain record.
		///
		/// Use this RPC call to avoid making multiple queries to onchain storage.
		fn has_permission_query(
			data_author: AccountId,
			grantee: AccountId,
			data_id: String,
			permission: DataPermission
		) -> Result<HasPermissionQueryResult, DispatchError>;
	}
}
