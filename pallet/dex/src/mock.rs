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
use types::TradingPair;

pub(crate) use root_primitives::{AssetId, Balance, Index};

pub type MockAccountId = u64;

pub const ALICE: MockAccountId = 1;
pub const BOB: MockAccountId = 2;
pub const Mycl: AssetId = 1;
pub const pBTC: AssetId = 101;
pub const pETH: AssetId = 102;
pub const pUSDC: AssetId = 103;

parameter_types! {
	pub static pUSDCBTCPair: TradingPair = TradingPair::new(pUSDC, pBTC);
	pub static pUSDCETHPair: TradingPair = TradingPair::new(pUSDC, pETH);
	pub static pETHBTCPair: TradingPair = TradingPair::new(pETH, pBTC);
}

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
	type AccountId = MockAccountId;
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
	type ForceOrigin = EnsureRoot<MockAccountId>;
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
	pub const MaxHolds: u32 = 16;
	pub const MyclAssetId: AssetId = 1;
	pub const AssetsExtPalletId: PalletId = PalletId(*b"assetext");
	pub const TestParachainId: u32 = 100;
}
impl pallet_assets_ext::Config for Test {
	type Event = Event;
	type ParachainId = TestParachainId;
	type MaxHolds = MaxHolds;
	type MyclAssetId = MyclAssetId;
	type PalletId = AssetsExtPalletId;
}

parameter_types! {
	pub const GetExchangeFee: (u32, u32) = (1, 100);
	pub const TradingPathLimit: u32 = 3;
	pub const DEXPalletId: PalletId = PalletId(*b"mock/dex");
}
impl Config for Test {
	type Event = Event;
	type GetExchangeFee = GetExchangeFee;
	type TradingPathLimit = TradingPathLimit;
	type DEXPalletId = DEXPalletId;
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

pub struct ExtBuilder;

impl Default for ExtBuilder {
	fn default() -> Self {
		ExtBuilder
	}
}

/*
impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		t.into()
	}
}
*/

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		// ext.execute_with(|| GenericAsset::create(Origin::signed(1), pUSDC, 1, 1));
		// ext.execute_with(|| Assets::create(Origin::signed(1), pETH, 1, 1));
		// ext.execute_with(|| Assets::create(Origin::signed(1), pBTC, 1, 1));
		// ext.execute_with(|| {
		// 	Assets::create(
		// 		Origin::signed(1),
		// 		pUSDCBTCPair::get().get_dex_share_asset_id().unwrap(),
		// 		1,
		// 		1,
		// 	)
		// });
		// ext.execute_with(|| {
		// 	Assets::create(
		// 		Origin::signed(1),
		// 		pUSDCBTCPair::get().get_dex_share_asset_id().unwrap(),
		// 		1,
		// 		1,
		// 	)
		// });
		// ext.execute_with(|| {
		// 	Assets::create(
		// 		Origin::signed(1),
		// 		pUSDCBTCPair::get().get_dex_share_asset_id().unwrap(),
		// 		1,
		// 		1,
		// 	)
		// });
		ext
	}
}
