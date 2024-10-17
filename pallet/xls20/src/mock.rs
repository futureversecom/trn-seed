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

use crate as pallet_xls20;
use seed_pallet_common::test_prelude::*;

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		Nft: pallet_nft,
		Xls20: pallet_xls20
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);

pub struct MockTransferSubscriber;
impl OnTransferSubscriber for MockTransferSubscriber {
	fn on_nft_transfer(_token_id: &TokenId) {}
}

pub struct MockNewAssetSubscription;

impl<RuntimeId> OnNewAssetSubscriber<RuntimeId> for MockNewAssetSubscription
where
	RuntimeId: From<u32> + Into<u32>,
{
	fn on_asset_create(_runtime_id: RuntimeId, _precompile_address_prefix: &[u8; 4]) {}
}

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
	type OnTransferSubscription = MockTransferSubscriber;
	type OnNewAssetSubscription = MockNewAssetSubscription;
	type MultiCurrency = AssetsExt;
	type PalletId = NftPalletId;
	type ParachainId = TestParachainId;
	type StringLimit = StringLimit;
	type WeightInfo = ();
	type Xls20MintRequest = Xls20;
	type NFIRequest = ();
}

parameter_types! {
	pub const MaxTokensPerXls20Mint: u32 = 1000;
}
impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MaxTokensPerXls20Mint = MaxTokensPerXls20Mint;
	type MultiCurrency = AssetsExt;
	type WeightInfo = ();
	type NFTExt = Nft;
	type NFTCollectionInfo = Nft;
	type Xls20PaymentAsset = Xls20PaymentAsset;
	type Migrator = ();
}
