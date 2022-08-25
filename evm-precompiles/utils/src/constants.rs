pub use precompile_addresses::*;

mod precompile_addresses {
	/// Calls to contracts starting with this prefix will be shim'd to the CENNZnet NFT module
	/// via an ERC721 compliant interface (`Erc721PrecompileSet`)
	pub const ERC721_PRECOMPILE_ADDRESS_PREFIX: &[u8; 4] = &[0xAA; 4];
}
