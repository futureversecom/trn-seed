
//! Autogenerated weights for `pallet_nft_peg`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-02-19, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-117-113`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-nft-peg
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./output

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_nft_peg`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_nft_peg::WeightInfo for WeightInfo<T> {
	// Storage: NftPeg ContractAddress (r:0 w:1)
	fn set_contract_address() -> Weight {
		(30_158_000 as Weight)
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: Nft TokenLocks (r:3 w:0)
	// Storage: NftPeg RootNftToErc721 (r:1 w:0)
	// Storage: NftPeg ContractAddress (r:1 w:0)
	// Storage: EthBridge NextEventProofId (r:1 w:1)
	// Storage: EthBridge NotaryKeys (r:1 w:0)
	// Storage: EthBridge NotarySetId (r:1 w:0)
	// Storage: EthBridge BridgePaused (r:1 w:0)
	// Storage: System Digest (r:1 w:1)
	// Storage: TokenApprovals ERC721Approvals (r:0 w:3)
	fn withdraw() -> Weight {
		(145_948_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(11 as Weight))
			.saturating_add(T::DbWeight::get().writes(6 as Weight))
	}
}
