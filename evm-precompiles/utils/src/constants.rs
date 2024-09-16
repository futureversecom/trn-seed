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

pub use precompile_addresses::*;

mod precompile_addresses {
	/// Calls to contracts starting with this prefix will be shim'd to the Seed NFT module
	/// via an IERC721 compliant interface (`Erc721PrecompileSet`)
	pub const ERC721_PRECOMPILE_ADDRESS_PREFIX: &[u8; 4] = &[0xAA; 4];
	/// Calls to contracts starting with this prefix will be shim'd to the Seed SFT module
	/// via an IERC1155 compliant interface (`Erc1155PrecompileSet`)
	pub const ERC1155_PRECOMPILE_ADDRESS_PREFIX: &[u8; 4] = &[0xBB; 4];
	/// Calls to contracts starting with this prefix will be shim'd to the Seed AssetsExt module
	/// via an IERC20 compliant interface (`Erc20PrecompileSet`)
	pub const ERC20_PRECOMPILE_ADDRESS_PREFIX: &[u8; 4] = &[0xCC; 4];
	/// Calls to contracts starting with this prefix will be shim'd to the Futurepass module
	pub const FUTUREPASS_PRECOMPILE_ADDRESS_PREFIX: &[u8; 4] = &[0xFF; 4];
	/// Precompile address for NFT
	pub const NFT_PRECOMPILE: u64 = 1721; // 0x06B9
	/// Precompile address for SFT
	pub const SFT_PRECOMPILE: u64 = 1731; // 0x06C3
	/// Precompile address for peg precompile
	pub const PEG_PRECOMPILE: u64 = 1939; // 0x0793
	/// Precompile address for dex precompile; IUniswapV2Router01 compliant interface for
	/// (`DexPrecompile`)
	pub const DEX_PRECOMPILE: u64 = 56797; // 0xDDDD
	/// The decoded location for the fee proxy function selector
	/// 0x04BB = 00000100 10111011
	pub const FEE_PROXY_ADDRESS: u64 = 1211; // 0x04BB
	/// Function selector for call_with_fee_preferences (deprecated)
	/// bytes4(keccak256(bytes("callWithFeePreferences(address,uint128,address,bytes)")));
	#[deprecated(note = "Use `callWithFeePreferences(address,address,bytes)` instead")]
	pub const FEE_FUNCTION_SELECTOR_DEPRECATED: [u8; 4] = [0x25, 0x5a, 0x34, 0x32];
	/// Function selector for call_with_fee_preferences
	/// bytes4(keccak256(bytes("callWithFeePreferences(address,address,bytes)")));
	pub const FEE_FUNCTION_SELECTOR: [u8; 4] = [0xf6, 0x09, 0x82, 0x86];
	/// Precompile address for futurepass registar
	pub const FUTUREPASS_REGISTRAR_PRECOMPILE: u64 = 65_535; // 0xFFFF
	/// Precompile address for marketplace
	pub const MARKETPLACE_PRECOMPILE: u64 = 1741; // 0x06CD
}
