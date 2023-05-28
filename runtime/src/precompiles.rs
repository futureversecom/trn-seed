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

use frame_support::parameter_types;
use pallet_evm_precompile_blake2::Blake2F;
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use pallet_evm_precompiles_erc1155::Erc1155PrecompileSet;
use pallet_evm_precompiles_erc20::Erc20PrecompileSet;
use pallet_evm_precompiles_erc721::Erc721PrecompileSet;
use pallet_evm_precompiles_futurepass::FuturePassPrecompileSet;
use pallet_evm_precompiles_futurepass_registrar::FuturePassRegistrarPrecompile;
use pallet_evm_precompiles_nft::NftPrecompile;
use pallet_evm_precompiles_peg::PegPrecompile;
use precompile_utils::{
	constants::{
		ERC1155_PRECOMPILE_ADDRESS_PREFIX, ERC20_PRECOMPILE_ADDRESS_PREFIX,
		ERC721_PRECOMPILE_ADDRESS_PREFIX, FUTUREPASS_PRECOMPILE_ADDRESS_PREFIX,
		FUTUREPASS_REGISTRAR_PRECOMPILE, NFT_PRECOMPILE, PEG_PRECOMPILE,
	},
	precompile_set::*,
};

parameter_types! {
	pub Erc721AssetPrefix: &'static [u8] = ERC721_PRECOMPILE_ADDRESS_PREFIX;
	pub Erc1155AssetPrefix: &'static [u8] = ERC1155_PRECOMPILE_ADDRESS_PREFIX;
	pub Erc20AssetPrefix: &'static [u8] = ERC20_PRECOMPILE_ADDRESS_PREFIX;
	pub FuturepassPrefix: &'static [u8] = FUTUREPASS_PRECOMPILE_ADDRESS_PREFIX;
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
				PrecompileAt<AddressU64<PEG_PRECOMPILE>, PegPrecompile<R>>,
				PrecompileAt<
					AddressU64<FUTUREPASS_REGISTRAR_PRECOMPILE>,
					FuturePassRegistrarPrecompile<R>,
				>,
			),
		>,
		// Prefixed precompile sets (XC20)
		PrecompileSetStartingWith<Erc721AssetPrefix, Erc721PrecompileSet<R>>,
		PrecompileSetStartingWith<Erc1155AssetPrefix, Erc1155PrecompileSet<R>>,
		PrecompileSetStartingWith<Erc20AssetPrefix, Erc20PrecompileSet<R>>,
		PrecompileSetStartingWith<
			FuturepassPrefix,
			FuturePassPrecompileSet<R>,
			LimitRecursionTo<1>,
			AllowDelegateCall,
		>,
	),
>;
