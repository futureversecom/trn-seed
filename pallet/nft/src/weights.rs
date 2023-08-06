//! Autogenerated weights for pallet_nft
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-07-22, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `Justins-MacBook-Pro.local`, CPU: `<UNKNOWN>`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ../rust_builds/release/seed
// benchmark
// pallet
// --chain
// dev
// --steps
// 50
// --repeat
// 20
// --pallet
// pallet_nft
// --extrinsic=*
// --execution
// wasm
// --wasm-execution
// compiled
// --heap-pages
// 4096
// --output
// ./output
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_nft.
pub trait WeightInfo {
	fn claim_unowned_collection() -> Weight;
	fn set_owner() -> Weight;
	fn set_max_issuance() -> Weight;
    fn set_base_uri() -> Weight;
    fn set_name() -> Weight;
	fn register_marketplace() -> Weight;
	fn create_collection() -> Weight;
	fn mint() -> Weight;
	fn transfer() -> Weight;
	fn burn() -> Weight;
	fn sell() -> Weight;
	fn buy() -> Weight;
	fn auction() -> Weight;
	fn bid() -> Weight;
	fn cancel_sale() -> Weight;
	fn update_fixed_price() -> Weight;
	fn make_simple_offer() -> Weight;
	fn cancel_offer() -> Weight;
	fn accept_offer() -> Weight;
	fn set_fee_to() -> Weight;
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn claim_unowned_collection() -> Weight {
		(45_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn set_owner() -> Weight {
		(40_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn set_max_issuance() -> Weight {
		(40_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn set_base_uri() -> Weight {
		(45_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn set_name() -> Weight {
		(25_108_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: Nft NextMarketplaceId (r:1 w:1)
	// Storage: Nft RegisteredMarketplaces (r:0 w:1)
	fn register_marketplace() -> Weight {
		(48_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
	// Storage: Nft NextCollectionId (r:1 w:1)
	// Storage: EVM AccountCodes (r:1 w:1)
	// Storage: Futurepass DefaultProxy (r:1 w:0)
	// Storage: System Account (r:1 w:1)
	// Storage: Nft CollectionInfo (r:0 w:1)
	fn create_collection() -> Weight {
		(86_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(4 as Weight))
			.saturating_add(RocksDbWeight::get().writes(4 as Weight))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	fn mint() -> Weight {
		(53_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: Nft TokenLocks (r:1 w:0)
	// Storage: TokenApprovals ERC721Approvals (r:0 w:1)
	fn transfer() -> Weight {
		(66_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
	// Storage: Nft TokenLocks (r:1 w:0)
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: TokenApprovals ERC721Approvals (r:0 w:1)
	fn burn() -> Weight {
		(60_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: Nft NextListingId (r:1 w:1)
	// Storage: Nft TokenLocks (r:1 w:1)
	// Storage: Nft Listings (r:0 w:1)
	// Storage: Nft ListingEndSchedule (r:0 w:1)
	// Storage: Nft OpenCollectionListings (r:0 w:1)
	fn sell() -> Weight {
		(85_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(5 as Weight))
	}
	// Storage: Nft Listings (r:1 w:1)
	// Storage: Nft FeeTo (r:1 w:0)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:2)
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: TokenApprovals ERC721Approvals (r:0 w:1)
	// Storage: Nft TokenLocks (r:0 w:1)
	// Storage: Nft ListingEndSchedule (r:0 w:1)
	// Storage: Nft OpenCollectionListings (r:0 w:1)
	fn buy() -> Weight {
		(148_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(6 as Weight))
			.saturating_add(RocksDbWeight::get().writes(9 as Weight))
	}
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: Nft NextListingId (r:1 w:1)
	// Storage: Nft TokenLocks (r:1 w:1)
	// Storage: Nft Listings (r:0 w:1)
	// Storage: Nft ListingEndSchedule (r:0 w:1)
	// Storage: Nft OpenCollectionListings (r:0 w:1)
	fn auction() -> Weight {
		(93_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(5 as Weight))
	}
	// Storage: Nft Listings (r:1 w:1)
	// Storage: Nft ListingWinningBid (r:1 w:1)
	// Storage: AssetsExt Holds (r:1 w:1)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:2)
	// Storage: System Account (r:2 w:2)
	// Storage: Nft ListingEndSchedule (r:0 w:2)
	fn bid() -> Weight {
		(183_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(8 as Weight))
			.saturating_add(RocksDbWeight::get().writes(10 as Weight))
	}
	// Storage: Nft Listings (r:1 w:1)
	// Storage: Nft TokenLocks (r:0 w:1)
	// Storage: Nft ListingEndSchedule (r:0 w:1)
	// Storage: Nft OpenCollectionListings (r:0 w:1)
	fn cancel_sale() -> Weight {
		(57_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(4 as Weight))
	}
	// Storage: Nft Listings (r:1 w:1)
	fn update_fixed_price() -> Weight {
		(48_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: Nft CollectionInfo (r:1 w:0)
	// Storage: Nft NextOfferId (r:1 w:1)
	// Storage: Nft TokenLocks (r:1 w:0)
	// Storage: AssetsExt Holds (r:1 w:1)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:2)
	// Storage: System Account (r:1 w:1)
	// Storage: Nft TokenOffers (r:1 w:1)
	// Storage: Nft Offers (r:0 w:1)
	fn make_simple_offer() -> Weight {
		(172_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(9 as Weight))
			.saturating_add(RocksDbWeight::get().writes(8 as Weight))
	}
	// Storage: Nft Offers (r:1 w:1)
	// Storage: AssetsExt Holds (r:1 w:1)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:2)
	// Storage: System Account (r:1 w:1)
	// Storage: Nft TokenOffers (r:1 w:1)
	fn cancel_offer() -> Weight {
		(132_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(7 as Weight))
			.saturating_add(RocksDbWeight::get().writes(7 as Weight))
	}
	// Storage: Nft Offers (r:1 w:1)
	// Storage: Nft TokenLocks (r:1 w:0)
	// Storage: Nft CollectionInfo (r:1 w:1)
	// Storage: AssetsExt Holds (r:1 w:1)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:2 w:2)
	// Storage: System Account (r:1 w:1)
	// Storage: Nft TokenOffers (r:1 w:1)
	// Storage: TokenApprovals ERC721Approvals (r:0 w:1)
	fn accept_offer() -> Weight {
		(185_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(9 as Weight))
			.saturating_add(RocksDbWeight::get().writes(9 as Weight))
	}
	// Storage: Nft FeeTo (r:0 w:1)
	fn set_fee_to() -> Weight {
		(32_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
}
