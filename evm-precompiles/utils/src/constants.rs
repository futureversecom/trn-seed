pub use precompile_addresses::*;

mod precompile_addresses {
	/// Calls to contracts starting with this prefix will be shim'd to the Seed NFT module
	/// via an ERC721 compliant interface (`Erc721PrecompileSet`)
	pub const ERC721_PRECOMPILE_ADDRESS_PREFIX: &[u8; 4] = &[0xAA; 4];
	/// Calls to contracts starting with this prefix will be shim'd to the Seed AssetsExt module
	/// via an ERC20 compliant interface (`Erc20PrecompileSet`)
	pub const ERC20_PRECOMPILE_ADDRESS_PREFIX: &[u8; 4] = &[0xCC; 4];
	/// Precompile address for NFT
	pub const NFT_PRECOMPILE: u64 = 1721;
	/// The decoded location for the fee proxy function selector
	/// 0x04BB = 00000100 10111011
	pub const FEE_PROXY_ADDRESS: u64 = 1211;
	/// Function selector for call_with_fee_preferences
	/// bytes4(keccak256(bytes("callWithFeePreferences(address,uint128,address,bytes)")));
	pub const FEE_FUNCTION_SELECTOR: [u8; 4] = [0x25, 0x5a, 0x34, 0x32];
	/// Precompile address for futurepass
	pub const FUTUREPASS_PRECOMPILE: u64 = 1722;
}
