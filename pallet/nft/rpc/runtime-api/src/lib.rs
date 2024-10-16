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

//! Runtime API definition required by NFT RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use pallet_nft::CollectionDetail;
use pallet_nft::Config;
use seed_primitives::types::{CollectionUuid, SerialNumber, TokenCount, TokenId};
use sp_std::{fmt::Debug, prelude::*};

sp_api::decl_runtime_apis! {
	/// The RPC API to interact with NFT module
	pub trait NftApi<AccountId, T> where
		AccountId: Codec + Clone + Debug + PartialEq,
		T: Config,
	{
		/// Find all the tokens owned by `who` in a given collection
		fn owned_tokens(
			collection_id: CollectionUuid,
			who: AccountId,
			cursor: SerialNumber,
			limit: u16
		) -> (SerialNumber, TokenCount, Vec<SerialNumber>);

		/// Return the token metadata URI for a given token
		fn token_uri(token_id: TokenId) -> Vec<u8>;

		fn collection_details(collection_id: CollectionUuid) -> CollectionDetail<AccountId>;
	}
}
