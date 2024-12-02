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

use frame_support::pallet_prelude::Get;
use crate as pallet_nfi;
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
		Nfi: pallet_nfi,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);

parameter_types! {
	pub const NftPalletId: PalletId = PalletId(*b"nftokens");
	pub const MaxTokensPerCollection: u32 = 10_000;
	pub const Xls20PaymentAsset: AssetId = XRP_ASSET_ID;
	pub const MintLimit: u32 = 100;
	pub const StringLimit: u32 = 50;
	pub const FeePotId: PalletId = PalletId(*b"txfeepot");
	pub const MarketplaceNetworkFeePercentage: Permill = Permill::from_perthousand(5);
	pub const DefaultFeeTo: Option<PalletId> = None;
}

impl pallet_nft::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type MaxTokensPerCollection = MaxTokensPerCollection;
	type MintLimit = MintLimit;
	type OnTransferSubscription = ();
	type OnNewAssetSubscription = ();
	type MultiCurrency = AssetsExt;
	type PalletId = NftPalletId;
	type ParachainId = TestParachainId;
	type StringLimit = StringLimit;
	type WeightInfo = ();
	type Xls20MintRequest = ();
	type NFIRequest = Nfi;
}

parameter_types! {
	pub const SftPalletId: PalletId = PalletId(*b"sftokens");
	pub const MaxTokensPerSftCollection: u32 = 10_000;
	pub const MaxSerialsPerSftMint: u32 = 100;
	pub const MaxOwnersPerSftToken: u32 = 100;
}

impl pallet_sft::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = AssetsExt;
	type NFTExt = Nft;
	type OnTransferSubscription = ();
	type OnNewAssetSubscription = ();
	type PalletId = SftPalletId;
	type ParachainId = TestParachainId;
	type StringLimit = StringLimit;
	type WeightInfo = ();
	type MaxTokensPerSftCollection = MaxTokensPerSftCollection;
	type MaxSerialsPerMint = MaxSerialsPerSftMint;
	type MaxOwnersPerSftToken = MaxOwnersPerSftToken;
	type NFIRequest = Nfi;
}

parameter_types! {
	pub const MaxDataLength: u32 = 100;
	pub const MaxByteLength: u32 = 100;
	pub const NFINetworkFeePercentage: Permill = Permill::from_perthousand(5);
	pub const ChainId: u64 = 1234;
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = AssetsExt;
	type NFTExt = Nft;
	type SFTExt = Sft;
	type NetworkFeePercentage = NFINetworkFeePercentage;
	type MaxDataLength = MaxDataLength;
	type MaxByteLength = MaxByteLength;
	type WeightInfo = ();
	type ChainId = ChainId;
}
