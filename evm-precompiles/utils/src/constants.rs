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
}
