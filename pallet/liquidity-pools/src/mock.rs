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

#![cfg(test)]

use super::*;
use frame_support::{construct_runtime, parameter_types, PalletId};
use frame_system::EnsureRoot;
use seed_pallet_common::{
	impl_frame_system_config, impl_pallet_assets_config, impl_pallet_assets_ext_config,
	impl_pallet_balance_config,
};
use sp_core::{H160, H256};
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{BlakeTwo256, IdentityLookup},
};

use seed_primitives::AccountId;
pub(crate) use seed_primitives::{AssetId, Balance};

mod dex {
	pub use super::super::*;
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		LiquidityPools: crate,
	}
);

impl_frame_system_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_ext_config!(Test);

parameter_types! {
	pub const LiquidityPoolsPalletId: PalletId = PalletId(*b"lqdpools");
	pub const UnsignedInterval: u32 =  5;
	pub const RolloverBatchSize: u32 = 10;
	pub const MaxStringLength: u32 = 1000;
	pub const RootAssetId: AssetId = 1;
	pub const InterestRateBasePoint: u32 = 1_000_000;
}
impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PoolId = u32;
	type ApproveOrigin = EnsureRoot<AccountId>;
	type InterestRateBasePoint = InterestRateBasePoint;
	type Currency = Balances;
	type Assets = AssetsExt;
	type NativeAssetId = RootAssetId;
	type PalletId = LiquidityPoolsPalletId;
	type UnsignedInterval = UnsignedInterval;
	type RolloverBatchSize = RolloverBatchSize;
	type MaxStringLength = MaxStringLength;
	type WeightInfo = ();
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	RuntimeCall: From<C>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = TestXt<RuntimeCall, ()>;
}
#[derive(Default)]
pub struct TestExt {
	balances: Vec<(AccountId, Balance)>,
	assets: Vec<(AssetId, AccountId, Balance)>,
}

pub const ROOT_ASSET_ID: AssetId = 1;
pub const TEST_ASSET_ID: AssetId = 2;

pub fn create_account(seed: u64) -> AccountId {
	AccountId::from(H160::from_low_u64_be(seed))
}

impl TestExt {
	pub fn with_balances(mut self, balances: &[(AccountId, Balance)]) -> Self {
		self.balances = balances.to_vec();
		self
	}
	pub fn with_assets(mut self, assets: &[(AssetId, AccountId, Balance)]) -> Self {
		self.assets = assets.to_vec();
		self
	}
	pub fn build(self) -> sp_io::TestExternalities {
		let mut ext = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		if !self.balances.is_empty() {
			pallet_balances::GenesisConfig::<Test> { balances: self.balances }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

		if !self.assets.is_empty() {
			let assets = vec![
				(ROOT_ASSET_ID, create_account(10), true, 1),
				(TEST_ASSET_ID, create_account(10), true, 1),
			];
			let metadata = vec![
				(ROOT_ASSET_ID, b"ROOT".to_vec(), b"ROOT".to_vec(), 6_u8),
				(TEST_ASSET_ID, b"FOO".to_vec(), b"FOO".to_vec(), 6_u8),
			];
			pallet_assets::GenesisConfig::<Test> { assets, metadata, accounts: self.assets }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

		let mut ext: sp_io::TestExternalities = ext.into();
		ext.execute_with(|| {
			System::initialize(&1, &[0u8; 32].into(), &Default::default());
		});
		ext.execute_with(|| pallet_assets_ext::GenesisConfig::<Test>::default().build());
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
