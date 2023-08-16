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
use crate::Config;
use frame_support::{dispatch::DispatchResult, parameter_types, PalletId};
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
	type Event = Event;
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
pub struct TestExt {}

impl TestExt {
	pub fn build(self) -> sp_io::TestExternalities {
		let ext = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		let mut ext: sp_io::TestExternalities = ext.into();
		ext.execute_with(|| {
			System::initialize(&1, &[0u8; 32].into(), &Default::default());
		});

		ext
	}
}

#[allow(dead_code)]
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
