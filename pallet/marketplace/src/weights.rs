// This file is part of Substrate.

// Copyright (C) 2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Autogenerated weights for pallet_marketplace
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-05-23, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `ip-172-31-102-147`, CPU: `Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz`
//! EXECUTION: , WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/seed
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet-marketplace
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./pallet/marketplace/src/weights.rs
// --template
// ./scripts/pallet_template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_marketplace.
pub trait WeightInfo {
	fn register_marketplace() -> Weight;
	fn sell_nft(p: u32, ) -> Weight;
	fn sell_sft(p: u32, ) -> Weight;
	fn buy() -> Weight;
	fn buy_multi(p: u32, ) -> Weight;
	fn auction_nft(p: u32, ) -> Weight;
	fn auction_sft(p: u32, ) -> Weight;
	fn bid() -> Weight;
	fn cancel_sale() -> Weight;
	fn update_fixed_price() -> Weight;
	fn make_simple_offer() -> Weight;
	fn cancel_offer() -> Weight;
	fn accept_offer() -> Weight;
	fn set_fee_to() -> Weight;
}

/// Weights for pallet_marketplace using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: `Marketplace::NextMarketplaceId` (r:1 w:1)
	// Proof: `Marketplace::NextMarketplaceId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::RegisteredMarketplaces` (r:0 w:1)
	// Proof: `Marketplace::RegisteredMarketplaces` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	fn register_marketplace() -> Weight {
		Weight::from_all(48_124_000_u64)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	// Storage: `Nft::CollectionInfo` (r:1 w:0)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::FeeTo` (r:1 w:0)
	// Proof: `Marketplace::FeeTo` (`max_values`: Some(1), `max_size`: Some(21), added: 516, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::NextListingId` (r:1 w:1)
	// Proof: `Marketplace::NextListingId` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	// Storage: `Nft::TokenLocks` (r:50 w:50)
	// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::Listings` (r:0 w:1)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingEndSchedule` (r:0 w:1)
	// Proof: `Marketplace::ListingEndSchedule` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::OpenCollectionListings` (r:0 w:1)
	// Proof: `Marketplace::OpenCollectionListings` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 50]`.
	fn sell_nft(p: u32, ) -> Weight {
		Weight::from_all(95_762_950_u64)
			// Standard Error: 12_435
			.saturating_add(Weight::from_all(13_939_641_u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(p as u64)))
			.saturating_add(T::DbWeight::get().writes(4_u64))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(p as u64)))
	}
	// Storage: `Sft::SftCollectionInfo` (r:1 w:0)
	// Proof: `Sft::SftCollectionInfo` (`max_values`: None, `max_size`: Some(484), added: 2959, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::FeeTo` (r:1 w:0)
	// Proof: `Marketplace::FeeTo` (`max_values`: Some(1), `max_size`: Some(21), added: 516, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::NextListingId` (r:1 w:1)
	// Proof: `Marketplace::NextListingId` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	// Storage: `Sft::TokenInfo` (r:50 w:50)
	// Proof: `Sft::TokenInfo` (`max_values`: None, `max_size`: Some(52000104), added: 52002579, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::Listings` (r:0 w:1)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingEndSchedule` (r:0 w:1)
	// Proof: `Marketplace::ListingEndSchedule` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::OpenCollectionListings` (r:0 w:1)
	// Proof: `Marketplace::OpenCollectionListings` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 50]`.
	fn sell_sft(p: u32, ) -> Weight {
		Weight::from_all(91_324_086_u64)
			// Standard Error: 28_637
			.saturating_add(Weight::from_all(10_702_400_u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(p as u64)))
			.saturating_add(T::DbWeight::get().writes(4_u64))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(p as u64)))
	}
	// Storage: `Marketplace::Listings` (r:1 w:1)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:0)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:1 w:0)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `Nft::CollectionInfo` (r:1 w:1)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `TokenApprovals::ERC721Approvals` (r:0 w:1)
	// Proof: `TokenApprovals::ERC721Approvals` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingEndSchedule` (r:0 w:1)
	// Proof: `Marketplace::ListingEndSchedule` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::OpenCollectionListings` (r:0 w:1)
	// Proof: `Marketplace::OpenCollectionListings` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Nft::TokenLocks` (r:0 w:1)
	// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	fn buy() -> Weight {
		Weight::from_all(165_370_000_u64)
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().writes(6_u64))
	}
	// Storage: `Marketplace::Listings` (r:50 w:50)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:50 w:0)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:50 w:0)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `Nft::CollectionInfo` (r:50 w:50)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `TokenApprovals::ERC721Approvals` (r:0 w:50)
	// Proof: `TokenApprovals::ERC721Approvals` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingEndSchedule` (r:0 w:50)
	// Proof: `Marketplace::ListingEndSchedule` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::OpenCollectionListings` (r:0 w:50)
	// Proof: `Marketplace::OpenCollectionListings` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Nft::TokenLocks` (r:0 w:50)
	// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 50]`.
	fn buy_multi(p: u32, ) -> Weight {
		Weight::from_all(29_771_213_u64)
			// Standard Error: 137_227
			.saturating_add(Weight::from_all(120_268_894_u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads((4_u64).saturating_mul(p as u64)))
			.saturating_add(T::DbWeight::get().writes((6_u64).saturating_mul(p as u64)))
	}
	// Storage: `Nft::CollectionInfo` (r:1 w:0)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::FeeTo` (r:1 w:0)
	// Proof: `Marketplace::FeeTo` (`max_values`: Some(1), `max_size`: Some(21), added: 516, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::NextListingId` (r:1 w:1)
	// Proof: `Marketplace::NextListingId` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	// Storage: `Nft::TokenLocks` (r:50 w:50)
	// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::Listings` (r:0 w:1)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingEndSchedule` (r:0 w:1)
	// Proof: `Marketplace::ListingEndSchedule` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::OpenCollectionListings` (r:0 w:1)
	// Proof: `Marketplace::OpenCollectionListings` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 50]`.
	fn auction_nft(p: u32, ) -> Weight {
		Weight::from_all(98_821_301_u64)
			// Standard Error: 54_582
			.saturating_add(Weight::from_all(14_043_585_u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(p as u64)))
			.saturating_add(T::DbWeight::get().writes(4_u64))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(p as u64)))
	}
	// Storage: `Sft::SftCollectionInfo` (r:1 w:0)
	// Proof: `Sft::SftCollectionInfo` (`max_values`: None, `max_size`: Some(484), added: 2959, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::FeeTo` (r:1 w:0)
	// Proof: `Marketplace::FeeTo` (`max_values`: Some(1), `max_size`: Some(21), added: 516, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::NextListingId` (r:1 w:1)
	// Proof: `Marketplace::NextListingId` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	// Storage: `Sft::TokenInfo` (r:50 w:50)
	// Proof: `Sft::TokenInfo` (`max_values`: None, `max_size`: Some(52000104), added: 52002579, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::Listings` (r:0 w:1)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingEndSchedule` (r:0 w:1)
	// Proof: `Marketplace::ListingEndSchedule` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::OpenCollectionListings` (r:0 w:1)
	// Proof: `Marketplace::OpenCollectionListings` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 50]`.
	fn auction_sft(p: u32, ) -> Weight {
		Weight::from_all(92_904_042_u64)
			// Standard Error: 27_291
			.saturating_add(Weight::from_all(10_468_680_u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(p as u64)))
			.saturating_add(T::DbWeight::get().writes(4_u64))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(p as u64)))
	}
	// Storage: `Marketplace::Listings` (r:1 w:1)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingWinningBid` (r:1 w:1)
	// Proof: `Marketplace::ListingWinningBid` (`max_values`: None, `max_size`: Some(60), added: 2535, mode: `MaxEncodedLen`)
	// Storage: `AssetsExt::Holds` (r:1 w:1)
	// Proof: `AssetsExt::Holds` (`max_values`: None, `max_size`: Some(433), added: 2908, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:2 w:2)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:1 w:1)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingEndSchedule` (r:0 w:2)
	// Proof: `Marketplace::ListingEndSchedule` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	fn bid() -> Weight {
		Weight::from_all(209_405_000_u64)
			.saturating_add(T::DbWeight::get().reads(7_u64))
			.saturating_add(T::DbWeight::get().writes(9_u64))
	}
	// Storage: `Marketplace::Listings` (r:1 w:1)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingEndSchedule` (r:0 w:1)
	// Proof: `Marketplace::ListingEndSchedule` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::OpenCollectionListings` (r:0 w:1)
	// Proof: `Marketplace::OpenCollectionListings` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Nft::TokenLocks` (r:0 w:1)
	// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	fn cancel_sale() -> Weight {
		Weight::from_all(65_696_000_u64)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}
	// Storage: `Marketplace::Listings` (r:1 w:1)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	fn update_fixed_price() -> Weight {
		Weight::from_all(49_654_000_u64)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	// Storage: `Nft::CollectionInfo` (r:1 w:0)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::NextOfferId` (r:1 w:1)
	// Proof: `Marketplace::NextOfferId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `Nft::TokenLocks` (r:1 w:0)
	// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	// Storage: `AssetsExt::Holds` (r:1 w:1)
	// Proof: `AssetsExt::Holds` (`max_values`: None, `max_size`: Some(433), added: 2908, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:2 w:2)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:1 w:1)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::TokenOffers` (r:1 w:1)
	// Proof: `Marketplace::TokenOffers` (`max_values`: None, `max_size`: Some(818), added: 3293, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::Offers` (r:0 w:1)
	// Proof: `Marketplace::Offers` (`max_values`: None, `max_size`: Some(70), added: 2545, mode: `MaxEncodedLen`)
	fn make_simple_offer() -> Weight {
		Weight::from_all(223_298_000_u64)
			.saturating_add(T::DbWeight::get().reads(9_u64))
			.saturating_add(T::DbWeight::get().writes(8_u64))
	}
	// Storage: `Marketplace::Offers` (r:1 w:1)
	// Proof: `Marketplace::Offers` (`max_values`: None, `max_size`: Some(70), added: 2545, mode: `MaxEncodedLen`)
	// Storage: `AssetsExt::Holds` (r:1 w:1)
	// Proof: `AssetsExt::Holds` (`max_values`: None, `max_size`: Some(433), added: 2908, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:2 w:2)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:1 w:1)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::TokenOffers` (r:1 w:1)
	// Proof: `Marketplace::TokenOffers` (`max_values`: None, `max_size`: Some(818), added: 3293, mode: `MaxEncodedLen`)
	fn cancel_offer() -> Weight {
		Weight::from_all(204_413_000_u64)
			.saturating_add(T::DbWeight::get().reads(7_u64))
			.saturating_add(T::DbWeight::get().writes(7_u64))
	}
	// Storage: `Marketplace::Offers` (r:1 w:1)
	// Proof: `Marketplace::Offers` (`max_values`: None, `max_size`: Some(70), added: 2545, mode: `MaxEncodedLen`)
	// Storage: `Nft::CollectionInfo` (r:1 w:1)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `Nft::TokenLocks` (r:1 w:1)
	// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::FeeTo` (r:1 w:0)
	// Proof: `Marketplace::FeeTo` (`max_values`: Some(1), `max_size`: Some(21), added: 516, mode: `MaxEncodedLen`)
	// Storage: `AssetsExt::Holds` (r:1 w:1)
	// Proof: `AssetsExt::Holds` (`max_values`: None, `max_size`: Some(433), added: 2908, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:2 w:2)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:1 w:1)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::TokenOffers` (r:1 w:1)
	// Proof: `Marketplace::TokenOffers` (`max_values`: None, `max_size`: Some(818), added: 3293, mode: `MaxEncodedLen`)
	// Storage: `TokenApprovals::ERC721Approvals` (r:0 w:1)
	// Proof: `TokenApprovals::ERC721Approvals` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	fn accept_offer() -> Weight {
		Weight::from_all(325_816_000_u64)
			.saturating_add(T::DbWeight::get().reads(10_u64))
			.saturating_add(T::DbWeight::get().writes(10_u64))
	}
	// Storage: `Marketplace::FeeTo` (r:0 w:1)
	// Proof: `Marketplace::FeeTo` (`max_values`: Some(1), `max_size`: Some(21), added: 516, mode: `MaxEncodedLen`)
	fn set_fee_to() -> Weight {
		Weight::from_all(24_729_000_u64)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: `Marketplace::NextMarketplaceId` (r:1 w:1)
	// Proof: `Marketplace::NextMarketplaceId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::RegisteredMarketplaces` (r:0 w:1)
	// Proof: `Marketplace::RegisteredMarketplaces` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	fn register_marketplace() -> Weight {
		Weight::from_all(48_124_000_u64)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	// Storage: `Nft::CollectionInfo` (r:1 w:0)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::FeeTo` (r:1 w:0)
	// Proof: `Marketplace::FeeTo` (`max_values`: Some(1), `max_size`: Some(21), added: 516, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::NextListingId` (r:1 w:1)
	// Proof: `Marketplace::NextListingId` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	// Storage: `Nft::TokenLocks` (r:50 w:50)
	// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::Listings` (r:0 w:1)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingEndSchedule` (r:0 w:1)
	// Proof: `Marketplace::ListingEndSchedule` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::OpenCollectionListings` (r:0 w:1)
	// Proof: `Marketplace::OpenCollectionListings` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 50]`.
	fn sell_nft(p: u32, ) -> Weight {
		Weight::from_all(95_762_950_u64)
			// Standard Error: 12_435
			.saturating_add(Weight::from_all(13_939_641_u64).saturating_mul(p as u64))
			.saturating_add(RocksDbWeight::get().reads(3_u64))
			.saturating_add(RocksDbWeight::get().reads((1_u64).saturating_mul(p as u64)))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
			.saturating_add(RocksDbWeight::get().writes((1_u64).saturating_mul(p as u64)))
	}
	// Storage: `Sft::SftCollectionInfo` (r:1 w:0)
	// Proof: `Sft::SftCollectionInfo` (`max_values`: None, `max_size`: Some(484), added: 2959, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::FeeTo` (r:1 w:0)
	// Proof: `Marketplace::FeeTo` (`max_values`: Some(1), `max_size`: Some(21), added: 516, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::NextListingId` (r:1 w:1)
	// Proof: `Marketplace::NextListingId` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	// Storage: `Sft::TokenInfo` (r:50 w:50)
	// Proof: `Sft::TokenInfo` (`max_values`: None, `max_size`: Some(52000104), added: 52002579, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::Listings` (r:0 w:1)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingEndSchedule` (r:0 w:1)
	// Proof: `Marketplace::ListingEndSchedule` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::OpenCollectionListings` (r:0 w:1)
	// Proof: `Marketplace::OpenCollectionListings` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 50]`.
	fn sell_sft(p: u32, ) -> Weight {
		Weight::from_all(91_324_086_u64)
			// Standard Error: 28_637
			.saturating_add(Weight::from_all(10_702_400_u64).saturating_mul(p as u64))
			.saturating_add(RocksDbWeight::get().reads(3_u64))
			.saturating_add(RocksDbWeight::get().reads((1_u64).saturating_mul(p as u64)))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
			.saturating_add(RocksDbWeight::get().writes((1_u64).saturating_mul(p as u64)))
	}
	// Storage: `Marketplace::Listings` (r:1 w:1)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:0)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:1 w:0)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `Nft::CollectionInfo` (r:1 w:1)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `TokenApprovals::ERC721Approvals` (r:0 w:1)
	// Proof: `TokenApprovals::ERC721Approvals` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingEndSchedule` (r:0 w:1)
	// Proof: `Marketplace::ListingEndSchedule` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::OpenCollectionListings` (r:0 w:1)
	// Proof: `Marketplace::OpenCollectionListings` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Nft::TokenLocks` (r:0 w:1)
	// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	fn buy() -> Weight {
		Weight::from_all(165_370_000_u64)
			.saturating_add(RocksDbWeight::get().reads(4_u64))
			.saturating_add(RocksDbWeight::get().writes(6_u64))
	}
	// Storage: `Marketplace::Listings` (r:50 w:50)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:50 w:0)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:50 w:0)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `Nft::CollectionInfo` (r:50 w:50)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `TokenApprovals::ERC721Approvals` (r:0 w:50)
	// Proof: `TokenApprovals::ERC721Approvals` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingEndSchedule` (r:0 w:50)
	// Proof: `Marketplace::ListingEndSchedule` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::OpenCollectionListings` (r:0 w:50)
	// Proof: `Marketplace::OpenCollectionListings` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Nft::TokenLocks` (r:0 w:50)
	// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 50]`.
	fn buy_multi(p: u32, ) -> Weight {
		Weight::from_all(29_771_213_u64)
			// Standard Error: 137_227
			.saturating_add(Weight::from_all(120_268_894_u64).saturating_mul(p as u64))
			.saturating_add(RocksDbWeight::get().reads((4_u64).saturating_mul(p as u64)))
			.saturating_add(RocksDbWeight::get().writes((6_u64).saturating_mul(p as u64)))
	}
	// Storage: `Nft::CollectionInfo` (r:1 w:0)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::FeeTo` (r:1 w:0)
	// Proof: `Marketplace::FeeTo` (`max_values`: Some(1), `max_size`: Some(21), added: 516, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::NextListingId` (r:1 w:1)
	// Proof: `Marketplace::NextListingId` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	// Storage: `Nft::TokenLocks` (r:50 w:50)
	// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::Listings` (r:0 w:1)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingEndSchedule` (r:0 w:1)
	// Proof: `Marketplace::ListingEndSchedule` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::OpenCollectionListings` (r:0 w:1)
	// Proof: `Marketplace::OpenCollectionListings` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 50]`.
	fn auction_nft(p: u32, ) -> Weight {
		Weight::from_all(98_821_301_u64)
			// Standard Error: 54_582
			.saturating_add(Weight::from_all(14_043_585_u64).saturating_mul(p as u64))
			.saturating_add(RocksDbWeight::get().reads(3_u64))
			.saturating_add(RocksDbWeight::get().reads((1_u64).saturating_mul(p as u64)))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
			.saturating_add(RocksDbWeight::get().writes((1_u64).saturating_mul(p as u64)))
	}
	// Storage: `Sft::SftCollectionInfo` (r:1 w:0)
	// Proof: `Sft::SftCollectionInfo` (`max_values`: None, `max_size`: Some(484), added: 2959, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::FeeTo` (r:1 w:0)
	// Proof: `Marketplace::FeeTo` (`max_values`: Some(1), `max_size`: Some(21), added: 516, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::NextListingId` (r:1 w:1)
	// Proof: `Marketplace::NextListingId` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	// Storage: `Sft::TokenInfo` (r:50 w:50)
	// Proof: `Sft::TokenInfo` (`max_values`: None, `max_size`: Some(52000104), added: 52002579, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::Listings` (r:0 w:1)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingEndSchedule` (r:0 w:1)
	// Proof: `Marketplace::ListingEndSchedule` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::OpenCollectionListings` (r:0 w:1)
	// Proof: `Marketplace::OpenCollectionListings` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 50]`.
	fn auction_sft(p: u32, ) -> Weight {
		Weight::from_all(92_904_042_u64)
			// Standard Error: 27_291
			.saturating_add(Weight::from_all(10_468_680_u64).saturating_mul(p as u64))
			.saturating_add(RocksDbWeight::get().reads(3_u64))
			.saturating_add(RocksDbWeight::get().reads((1_u64).saturating_mul(p as u64)))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
			.saturating_add(RocksDbWeight::get().writes((1_u64).saturating_mul(p as u64)))
	}
	// Storage: `Marketplace::Listings` (r:1 w:1)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingWinningBid` (r:1 w:1)
	// Proof: `Marketplace::ListingWinningBid` (`max_values`: None, `max_size`: Some(60), added: 2535, mode: `MaxEncodedLen`)
	// Storage: `AssetsExt::Holds` (r:1 w:1)
	// Proof: `AssetsExt::Holds` (`max_values`: None, `max_size`: Some(433), added: 2908, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:2 w:2)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:1 w:1)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingEndSchedule` (r:0 w:2)
	// Proof: `Marketplace::ListingEndSchedule` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	fn bid() -> Weight {
		Weight::from_all(209_405_000_u64)
			.saturating_add(RocksDbWeight::get().reads(7_u64))
			.saturating_add(RocksDbWeight::get().writes(9_u64))
	}
	// Storage: `Marketplace::Listings` (r:1 w:1)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::ListingEndSchedule` (r:0 w:1)
	// Proof: `Marketplace::ListingEndSchedule` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::OpenCollectionListings` (r:0 w:1)
	// Proof: `Marketplace::OpenCollectionListings` (`max_values`: None, `max_size`: Some(37), added: 2512, mode: `MaxEncodedLen`)
	// Storage: `Nft::TokenLocks` (r:0 w:1)
	// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	fn cancel_sale() -> Weight {
		Weight::from_all(65_696_000_u64)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
	}
	// Storage: `Marketplace::Listings` (r:1 w:1)
	// Proof: `Marketplace::Listings` (`max_values`: None, `max_size`: Some(20295), added: 22770, mode: `MaxEncodedLen`)
	fn update_fixed_price() -> Weight {
		Weight::from_all(49_654_000_u64)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	// Storage: `Nft::CollectionInfo` (r:1 w:0)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::NextOfferId` (r:1 w:1)
	// Proof: `Marketplace::NextOfferId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	// Storage: `Nft::TokenLocks` (r:1 w:0)
	// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	// Storage: `AssetsExt::Holds` (r:1 w:1)
	// Proof: `AssetsExt::Holds` (`max_values`: None, `max_size`: Some(433), added: 2908, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:2 w:2)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:1 w:1)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::TokenOffers` (r:1 w:1)
	// Proof: `Marketplace::TokenOffers` (`max_values`: None, `max_size`: Some(818), added: 3293, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::Offers` (r:0 w:1)
	// Proof: `Marketplace::Offers` (`max_values`: None, `max_size`: Some(70), added: 2545, mode: `MaxEncodedLen`)
	fn make_simple_offer() -> Weight {
		Weight::from_all(223_298_000_u64)
			.saturating_add(RocksDbWeight::get().reads(9_u64))
			.saturating_add(RocksDbWeight::get().writes(8_u64))
	}
	// Storage: `Marketplace::Offers` (r:1 w:1)
	// Proof: `Marketplace::Offers` (`max_values`: None, `max_size`: Some(70), added: 2545, mode: `MaxEncodedLen`)
	// Storage: `AssetsExt::Holds` (r:1 w:1)
	// Proof: `AssetsExt::Holds` (`max_values`: None, `max_size`: Some(433), added: 2908, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:2 w:2)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:1 w:1)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::TokenOffers` (r:1 w:1)
	// Proof: `Marketplace::TokenOffers` (`max_values`: None, `max_size`: Some(818), added: 3293, mode: `MaxEncodedLen`)
	fn cancel_offer() -> Weight {
		Weight::from_all(204_413_000_u64)
			.saturating_add(RocksDbWeight::get().reads(7_u64))
			.saturating_add(RocksDbWeight::get().writes(7_u64))
	}
	// Storage: `Marketplace::Offers` (r:1 w:1)
	// Proof: `Marketplace::Offers` (`max_values`: None, `max_size`: Some(70), added: 2545, mode: `MaxEncodedLen`)
	// Storage: `Nft::CollectionInfo` (r:1 w:1)
	// Proof: `Nft::CollectionInfo` (`max_values`: None, `max_size`: Some(4294967295), added: 2474, mode: `MaxEncodedLen`)
	// Storage: `Nft::TokenLocks` (r:1 w:1)
	// Proof: `Nft::TokenLocks` (`max_values`: None, `max_size`: Some(33), added: 2508, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::FeeTo` (r:1 w:0)
	// Proof: `Marketplace::FeeTo` (`max_values`: Some(1), `max_size`: Some(21), added: 516, mode: `MaxEncodedLen`)
	// Storage: `AssetsExt::Holds` (r:1 w:1)
	// Proof: `AssetsExt::Holds` (`max_values`: None, `max_size`: Some(433), added: 2908, mode: `MaxEncodedLen`)
	// Storage: `Assets::Asset` (r:1 w:1)
	// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(162), added: 2637, mode: `MaxEncodedLen`)
	// Storage: `Assets::Account` (r:2 w:2)
	// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(110), added: 2585, mode: `MaxEncodedLen`)
	// Storage: `System::Account` (r:1 w:1)
	// Proof: `System::Account` (`max_values`: None, `max_size`: Some(116), added: 2591, mode: `MaxEncodedLen`)
	// Storage: `Marketplace::TokenOffers` (r:1 w:1)
	// Proof: `Marketplace::TokenOffers` (`max_values`: None, `max_size`: Some(818), added: 3293, mode: `MaxEncodedLen`)
	// Storage: `TokenApprovals::ERC721Approvals` (r:0 w:1)
	// Proof: `TokenApprovals::ERC721Approvals` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	fn accept_offer() -> Weight {
		Weight::from_all(325_816_000_u64)
			.saturating_add(RocksDbWeight::get().reads(10_u64))
			.saturating_add(RocksDbWeight::get().writes(10_u64))
	}
	// Storage: `Marketplace::FeeTo` (r:0 w:1)
	// Proof: `Marketplace::FeeTo` (`max_values`: Some(1), `max_size`: Some(21), added: 516, mode: `MaxEncodedLen`)
	fn set_fee_to() -> Weight {
		Weight::from_all(24_729_000_u64)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
}

