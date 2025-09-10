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

use crate as pallet_marketplace;
use seed_pallet_common::test_prelude::*;

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		Nft: pallet_nft,
		Sft: pallet_sft,
		Marketplace: pallet_marketplace,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);
impl_pallet_nft_config!(Test);
impl_pallet_sft_config!(Test);

parameter_types! {
	pub const MarketplacePalletId: PalletId = PalletId(*b"marketpl");
	pub const DefaultListingDuration: u64 = 100;
	pub const DefaultOfferDuration: u64 = 1000;
	pub const MaxOffers: u32 = 10;
	pub const MaxTokensPerListing: u32 = 100;
	pub const MaxListingsPerMultiBuy: u32 = 100;
	pub const DefaultFeeTo: Option<PalletId> = Some(FeePotId::get());
	pub const MarketplaceNetworkFeePercentage: Permill = Permill::from_perthousand(5);
}

impl crate::Config for Test {
	type RuntimeCall = RuntimeCall;
	type DefaultListingDuration = DefaultListingDuration;
	type DefaultOfferDuration = DefaultOfferDuration;
	type RuntimeEvent = RuntimeEvent;
	type DefaultFeeTo = DefaultFeeTo;
	type MultiCurrency = AssetsExt;
	type NFTExt = Nft;
	type SFTExt = Sft;
	type PalletId = MarketplacePalletId;
	type NetworkFeePercentage = MarketplaceNetworkFeePercentage;
	type WeightInfo = ();
	type MaxTokensPerListing = MaxTokensPerListing;
	type MaxListingsPerMultiBuy = MaxListingsPerMultiBuy;
	type MaxOffers = MaxOffers;
}
