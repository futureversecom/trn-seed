// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

#![cfg(test)]

use super::*;
use frame_support::{construct_runtime, parameter_types};
use frame_system::{limits, EnsureRoot};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Perbill,
};

use seed_primitives::AccountId;
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
	type Origin = Origin;
	type Index = Index;
	type BlockNumber = u64;
	type Call = Call;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
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
	type Event = Event;
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
	type Event = Event;
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
	type Event = Event;
	type ParachainId = TestParachainId;
	type MaxHolds = MaxHolds;
	type NativeAssetId = NativeAssetId;
	type OnNewAssetSubscription = ();
	type PalletId = AssetsExtPalletId;
	type WeightInfo = ();
}

parameter_types! {
	pub const GetExchangeFee: (u32, u32) = (3, 1000); // 0.3% fee
	pub const TradingPathLimit: u32 = 2;
	pub const DEXBurnPalletId: PalletId = PalletId(*b"burnaddr");
	pub const LPTokenName: [u8; 10] = *b"Uniswap V2";
	pub const LPTokenSymbol: [u8; 6] = *b"UNI-V2";
	pub const LPTokenDecimals: u8 = 6;
}
impl Config for Test {
	type Event = Event;
	type GetExchangeFee = GetExchangeFee;
	type TradingPathLimit = TradingPathLimit;
	type DEXBurnPalletId = DEXBurnPalletId;
	type LPTokenName = LPTokenName;
	type LPTokenSymbol = LPTokenSymbol;
	type LPTokenDecimals = LPTokenDecimals;
	type WeightInfo = ();
	type MultiCurrency = AssetsExt;
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
		Dex: crate::{Pallet, Call, Storage, Event<T>},
	}
);

#[derive(Default)]
pub struct TestExt;

impl TestExt {
	pub fn build(self) -> sp_io::TestExternalities {
		let storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		let mut ext: sp_io::TestExternalities = storage.into();
		ext.execute_with(|| System::initialize(&1, &[0u8; 32].into(), &Default::default()));
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
