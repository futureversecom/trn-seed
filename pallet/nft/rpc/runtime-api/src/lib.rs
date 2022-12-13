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

//! Runtime API definition required by NFT RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use pallet_nft::Config;
use seed_primitives::types::{CollectionUuid, SerialNumber, TokenId};
use sp_std::prelude::*;

sp_api::decl_runtime_apis! {
	/// The RPC API to interact with NFT module
	pub trait NftApi<AccountId, T> where
		AccountId: Codec,
		T: Config,
	{
		/// Find all the tokens owned by `who` in a given collection
		fn owned_tokens(
			collection_id: CollectionUuid,
			who: AccountId,
			cursor: SerialNumber,
			limit: u16
		) -> (SerialNumber, Vec<SerialNumber>);

		/// Return the token metadata URI for a given token
		fn token_uri(token_id: TokenId) -> Vec<u8>;
	}
}
