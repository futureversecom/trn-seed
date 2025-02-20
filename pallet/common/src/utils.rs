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

//! shared pallet common utilities
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use seed_primitives::{AssetId, Balance};
use sp_core::U256;
// Maximum value that fits into 22 bits
const MAX_U22: u32 = (1 << 22) - 1;
// Maximum value that fits into 10 bits
const MAX_U10: u32 = (1 << 10) - 1;

/// Combines the incrementing next_id with the parachain_id
///
/// Useful for NFT collections and asset_id creation
///
/// The first 22 bits are dedicated to the unique ID
/// The last 10 bits are dedicated to the parachain_id
/// |    22 next_id bits   | 10 parachain_id bits |
/// |          1           |   100   |
/// 0b000000000000000000001_0001100100
pub fn next_asset_uuid(next_id: u32, parachain_id: u32) -> Option<u32> {
	// Check ids fit within limited bit sizes
	// next_id max 22 bits, parachain_id max 10 bits
	if next_id + 1_u32 > MAX_U22 || parachain_id > MAX_U10 {
		return None;
	}

	// next_id is the first 22 bits, parachain_id is the last 10 bits
	let next_global_uuid: u32 = (next_id << 10) | parachain_id;
	Some(next_global_uuid)
}

/// Convert 18dp wei values to correct dp equivalents
/// fractional amounts < `CPAY_UNIT_VALUE` are rounded up by adding 1 / 0.000001 cpay
pub fn scale_wei_to_correct_decimals(value: U256, decimals: u8) -> Balance {
	let unit_value = U256::from(10).pow(U256::from(18) - U256::from(decimals));
	let (quotient, remainder) = (value / unit_value, value % unit_value);
	if remainder == U256::from(0) {
		quotient.as_u128()
	} else {
		// if value has a fractional part < CPAY unit value
		// it is lost in this divide operation
		(quotient + 1).as_u128()
		// (quotient).as_u128() // <- validate this is correct
	}
}

/// convert X dp to 18dp (wei)
pub fn scale_decimals_to_wei(value: U256, decimals: u8) -> Balance {
	let unit_value = U256::from(10).pow(U256::from(18) - U256::from(decimals));
	(value * unit_value).as_u128()
}

#[derive(Debug, Default, Clone, Encode, Decode, PartialEq, TypeInfo, Copy, MaxEncodedLen)]
pub struct PublicMintInformation {
	/// Whether public minting is enabled for the collection
	pub enabled: bool,
	/// If pricing_details are set, the user will be charged this amount per token
	pub pricing_details: Option<(AssetId, Balance)>,
}

// Additional flags on a collection that determine whether tokens within the collection can be transferred, burned, or minted
#[derive(Debug, Clone, Encode, Decode, PartialEq, TypeInfo, Copy, MaxEncodedLen)]
pub struct CollectionUtilityFlags {
	pub transferable: bool,
	pub burnable: bool,
	pub mintable: bool,
}

impl Default for CollectionUtilityFlags {
	fn default() -> Self {
		Self { transferable: true, burnable: true, mintable: true }
	}
}

/// Permissions related to a token that determine whether it can be burned by the token_owner, collection_owner, or no one
/// Currently no-op, this will be used when implementing soulbound NFTs with ERC5484 support
/// Once set, the burn authority is immutable
#[derive(Debug, Clone, Encode, Decode, PartialEq, TypeInfo, Copy, MaxEncodedLen)]
pub enum TokenBurnAuthority {
	/// The token can be burned by the token_owner
	TokenOwner,
	/// The token can be burned by the collection_owner
	CollectionOwner,
	/// The token can be burned by either token or collection owner
	Both,
	/// The token cannot be burned by anyone
	Neither,
}

impl TryFrom<u8> for TokenBurnAuthority {
	type Error = &'static str;

	fn try_from(v: u8) -> Result<Self, Self::Error> {
		match v {
			0 => Ok(TokenBurnAuthority::TokenOwner),
			1 => Ok(TokenBurnAuthority::CollectionOwner),
			2 => Ok(TokenBurnAuthority::Both),
			3 => Ok(TokenBurnAuthority::Neither),
			_ => Err("Unrecognized burn authority"),
		}
	}
}

impl Into<u8> for TokenBurnAuthority {
	fn into(self) -> u8 {
		match self {
			TokenBurnAuthority::TokenOwner => 0,
			TokenBurnAuthority::CollectionOwner => 1,
			TokenBurnAuthority::Both => 2,
			TokenBurnAuthority::Neither => 3,
		}
	}
}

// Additional flags at a token level that determine whether that token can be transferred, or burned
#[derive(Debug, Clone, Encode, Decode, PartialEq, TypeInfo, Copy, MaxEncodedLen)]
pub struct TokenUtilityFlags {
	/// Whether the token can be transferred
	pub transferable: bool,
	/// What burn permissions the token has
	pub burn_authority: Option<TokenBurnAuthority>,
}

impl Default for TokenUtilityFlags {
	fn default() -> Self {
		Self { transferable: true, burn_authority: None }
	}
}
