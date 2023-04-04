use frame_support::parameter_types;
use pallet_evm_precompile_blake2::Blake2F;
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use pallet_evm_precompiles_erc20::Erc20PrecompileSet;
use pallet_evm_precompiles_erc721::Erc721PrecompileSet;
use pallet_evm_precompiles_futurepass::FuturePassPrecompile;
use pallet_evm_precompiles_nft::NftPrecompile;
use precompile_utils::{
	constants::{
		ERC20_PRECOMPILE_ADDRESS_PREFIX, ERC721_PRECOMPILE_ADDRESS_PREFIX, FUTUREPASS_PRECOMPILE,
		NFT_PRECOMPILE,
	},
	precompile_set::*,
};

parameter_types! {
	pub Erc721AssetPrefix: &'static [u8] = ERC721_PRECOMPILE_ADDRESS_PREFIX;
	pub Erc20AssetPrefix: &'static [u8] = ERC20_PRECOMPILE_ADDRESS_PREFIX;
}

/// The PrecompileSet installed in the Futureverse runtime.
/// We include six of the nine Istanbul precompiles
/// (https://github.com/ethereum/go-ethereum/blob/3c46f557/core/vm/contracts.go#L69)
/// as well as a special precompile for dispatching Substrate extrinsics
/// The following distribution has been decided for the precompiles
/// 0-1023: Ethereum Mainnet Precompiles
pub type FutureversePrecompiles<R> = PrecompileSetBuilder<
	R,
	(
		// Skip precompiles if out of range.
		PrecompilesInRangeInclusive<
			(AddressU64<1>, AddressU64<65535>),
			(
				// Ethereum precompiles:
				// We allow DELEGATECALL to stay compliant with Ethereum behavior.
				PrecompileAt<AddressU64<1>, ECRecover, ForbidRecursion, AllowDelegateCall>,
				PrecompileAt<AddressU64<2>, Sha256, ForbidRecursion, AllowDelegateCall>,
				PrecompileAt<AddressU64<3>, Ripemd160, ForbidRecursion, AllowDelegateCall>,
				PrecompileAt<AddressU64<4>, Identity, ForbidRecursion, AllowDelegateCall>,
				PrecompileAt<AddressU64<5>, Modexp, ForbidRecursion, AllowDelegateCall>,
				PrecompileAt<AddressU64<9>, Blake2F, ForbidRecursion, AllowDelegateCall>,
				// Non-Futureverse specific nor Ethereum precompiles :
				PrecompileAt<AddressU64<1024>, Sha3FIPS256>,
				PrecompileAt<AddressU64<1026>, ECRecoverPublicKey>,
				// Futureverse specific precompiles:
				PrecompileAt<AddressU64<NFT_PRECOMPILE>, NftPrecompile<R>>,
				PrecompileAt<
					AddressU64<FUTUREPASS_PRECOMPILE>,
					FuturePassPrecompile<R>,
					ForbidRecursion,
					AllowDelegateCall,
				>,
			),
		>,
		// Prefixed precompile sets (XC20)
		PrecompileSetStartingWith<Erc721AssetPrefix, Erc721PrecompileSet<R>>,
		PrecompileSetStartingWith<Erc20AssetPrefix, Erc20PrecompileSet<R>>,
	),
>;
