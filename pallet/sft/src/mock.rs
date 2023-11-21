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

use crate as pallet_sft;
use crate::{tests::create_account, Config};
use frame_support::{
	dispatch::DispatchResult, pallet_prelude::GenesisBuild, parameter_types, PalletId,
};
use frame_system::EnsureRoot;
use seed_pallet_common::*;
use seed_primitives::{
	AccountId, AssetId, Balance, CollectionUuid, MetadataScheme, SerialNumber, TokenId,
};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub const XRP_ASSET_ID: AssetId = 2;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		Nft: pallet_nft,
		Sft: pallet_sft,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);
impl_pallet_nft_config!(Test);

pub struct MockNewAssetSubscription;

impl<RuntimeId> OnNewAssetSubscriber<RuntimeId> for MockNewAssetSubscription
where
	RuntimeId: From<u32> + Into<u32>,
{
	fn on_asset_create(_runtime_id: RuntimeId, _precompile_address_prefix: &[u8; 4]) {}
}

parameter_types! {
	pub const SftPalletId: PalletId = PalletId(*b"sftokens");
	pub const MaxTokensPerSftCollection: u32 = 10_000;
	pub const MaxSerialsPerMint: u32 = 10;
	pub const MaxOwnersPerSftToken: u32 = 100;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = AssetsExt;
	type NFTExt = Nft;
	type OnTransferSubscription = MockTransferSubscriber;
	type OnNewAssetSubscription = MockNewAssetSubscription;
	type PalletId = SftPalletId;
	type ParachainId = TestParachainId;
	type StringLimit = StringLimit;
	type WeightInfo = ();
	type MaxTokensPerSftCollection = MaxTokensPerSftCollection;
	type MaxSerialsPerMint = MaxSerialsPerMint;
	type MaxOwnersPerSftToken = MaxOwnersPerSftToken;
}

#[derive(Default)]
pub struct TestExt {
	balances: Vec<(AccountId, Balance)>,
	xrp_balances: Vec<(AssetId, AccountId, Balance)>,
}

impl TestExt {
	/// Configure some native token balances
	pub fn with_balances(mut self, balances: &[(AccountId, Balance)]) -> Self {
		self.balances = balances.to_vec();
		self
	}
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

		if !self.balances.is_empty() {
			pallet_balances::GenesisConfig::<Test> { balances: self.balances }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

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
		.find(|e| *e == RuntimeEvent::Sft(event.clone()))
		.is_some()
}

#[allow(dead_code)]
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
