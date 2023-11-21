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
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

construct_runtime!(
	pub enum Test where
		Block = Block<Test>,
		NodeBlock = Block<Test>,
		UncheckedExtrinsic = UncheckedExtrinsic<Test>,
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
	type Xls20PaymentAsset = Xls20PaymentAsset;
}

#[derive(Default)]
pub struct TestExt {
	xrp_balances: Vec<(AssetId, AccountId, Balance)>,
}

impl TestExt {
	/// Configure some XRP asset balances
	pub fn with_xrp_balances(mut self, balances: &[(AccountId, Balance)]) -> Self {
		self.xrp_balances = balances
			.to_vec()
			.into_iter()
			.map(|(who, balance)| (XRP_ASSET_ID, who, balance))
			.collect();
		self
	}
	pub fn build(self) -> sp_io::TestExternalities {
		let mut ext = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		if !self.xrp_balances.is_empty() {
			let assets = vec![(XRP_ASSET_ID, create_account(10), true, 1)];
			let metadata = vec![(XRP_ASSET_ID, b"XRP".to_vec(), b"XRP".to_vec(), 6_u8)];
			let accounts = self.xrp_balances;
			pallet_assets::GenesisConfig::<Test> { assets, metadata, accounts }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

		let mut ext: sp_io::TestExternalities = ext.into();
		ext.execute_with(|| {
			System::initialize(&1, &[0u8; 32].into(), &Default::default());
		});

		ext
	}
}

/// Check the system event record contains `event`
pub(crate) fn has_event(event: crate::Event<Test>) -> bool {
	System::events()
		.into_iter()
		.map(|r| r.event)
		// .filter_map(|e| if let Event::Nft(inner) = e { Some(inner) } else { None })
		.find(|e| *e == RuntimeEvent::Xls20(event.clone()))
		.is_some()
}

#[allow(dead_code)]
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
