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
use frame_system::{limits, EnsureRoot};
use sp_core::{H160, H256};
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{BlakeTwo256, IdentityLookup},
	Perbill,
};

use seed_primitives::{AccountId, BlockNumber};
pub(crate) use seed_primitives::{AssetId, Balance, Index};

mod dex {
	pub use super::super::*;
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockLength: limits::BlockLength = limits::BlockLength::max(2 * 1024);
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type RuntimeOrigin = RuntimeOrigin;
	type Index = Index;
	type BlockNumber = u64;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type BlockLength = BlockLength;
	type BlockWeights = ();
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const AssetDeposit: Balance = 1_000_000;
	pub const AssetAccountDeposit: Balance = 16;
	pub const ApprovalDeposit: Balance = 1;
	pub const AssetsStringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 1 * 68;
	pub const MetadataDepositPerByte: Balance = 1;
}
impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = AssetId;

	type Currency = Balances;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = AssetAccountDeposit;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const MaxReserves: u32 = 50;
}
impl pallet_balances::Config for Test {
	type MaxLocks = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ();
	type AccountStore = System;
	type WeightInfo = ();
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
}

parameter_types! {
	pub const TestParachainId: u32 = 100;
	pub const MaxHolds: u32 = 16;
	pub const NativeAssetId: AssetId = 1;
	pub const AssetsExtPalletId: PalletId = PalletId(*b"assetext");
}
impl pallet_assets_ext::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ParachainId = TestParachainId;
	type MaxHolds = MaxHolds;
	type NativeAssetId = NativeAssetId;
	type OnNewAssetSubscription = ();
	type PalletId = AssetsExtPalletId;
	type WeightInfo = ();
}

parameter_types! {
	pub const IncentivePalletId: PalletId = PalletId(*b"incentiv");
	pub const UnsignedInterval: BlockNumber =  5;
	pub const RolloverBatchSize: u32 = 10;
}
impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AssetId = AssetId;
	type Balance = Balance;
	type PoolId = u32;
	type ApproveOrigin = EnsureRoot<AccountId>;
	type Currency = Balances;
	type Assets = AssetsExt;
	type PalletId = IncentivePalletId;
	type UnsignedInterval = UnsignedInterval;
	type RolloverBatchSize = RolloverBatchSize;
	type WeightInfo = ();
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	RuntimeCall: From<C>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = TestXt<RuntimeCall, ()>;
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Assets: pallet_assets::{Pallet, Storage, Config<T>, Event<T>},
		AssetsExt: pallet_assets_ext::{Pallet, Storage, Event<T>},
		Incentive: crate::{Pallet, Call, Storage, Event<T>},
	}
);

#[derive(Default)]
pub struct TestExt {
	balances: Vec<(AccountId, Balance)>,
	assets: Vec<(AssetId, AccountId, Balance)>,
}

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
			let assets = vec![(TEST_ASSET_ID, create_account(10), true, 1)];
			let metadata = vec![(TEST_ASSET_ID, b"FOO".to_vec(), b"FOO".to_vec(), 6_u8)];
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
