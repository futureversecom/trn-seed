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

pub use precompile_addresses::*;

mod precompile_addresses {
	/// Calls to contracts starting with this prefix will be shim'd to the Seed NFT module
	/// via an ERC721 compliant interface (`Erc721PrecompileSet`)
	pub const ERC721_PRECOMPILE_ADDRESS_PREFIX: &[u8; 4] = &[0xAA; 4];
	/// Calls to contracts starting with this prefix will be shim'd to the Seed AssetsExt module
	/// via an ERC20 compliant interface (`Erc20PrecompileSet`)
	pub const ERC20_PRECOMPILE_ADDRESS_PREFIX: &[u8; 4] = &[0xCC; 4];
	/// Precompile address for NFT
	pub const NFT_PRECOMPILE: u64 = 1721; // 0x6B9
	/// Precompile address for peg precompile
	pub const PEG_PRECOMPILE: u64 = 1939; // 0x0793
	/// The decoded location for the fee proxy function selector
	/// 0x04BB = 00000100 10111011
	pub const FEE_PROXY_ADDRESS: u64 = 1211; // 0x04BB
	/// Function selector for call_with_fee_preferences
	/// bytes4(keccak256(bytes("callWithFeePreferences(address,uint128,address,bytes)")));
	pub const FEE_FUNCTION_SELECTOR: [u8; 4] = [0x25, 0x5a, 0x34, 0x32];
}
