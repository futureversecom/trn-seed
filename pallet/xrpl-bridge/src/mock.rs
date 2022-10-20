use frame_support::{
	construct_runtime, parameter_types,
	traits::{ConstU16, ConstU64},
	PalletId,
};
use frame_system as system;
use frame_system::{limits, EnsureRoot};
use sp_core::{ByteArray, H256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	DispatchError,
};

use seed_pallet_common::XrplEthyBridgeAdapter;
use seed_primitives::{
	ethy::{crypto::AuthorityId, EventProofId},
	AccountId, AssetId, Balance, BlockNumber,
};

use crate as pallet_xrpl_bridge;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Assets: pallet_assets::{Pallet, Storage, Config<T>, Event<T>},
		XRPLBridge: pallet_xrpl_bridge::{Pallet, Call, Storage, Event<T>},
		AssetsExt: pallet_assets_ext::{Pallet, Call, Storage, Config<T>, Event<T>},
		TimestampPallet: pallet_timestamp::{Pallet, Call, Storage, Inherent},
	}
);

parameter_types! {
	pub BlockLength: limits::BlockLength = limits::BlockLength::max(2 * 1024);
}

impl frame_system::Config for Test {
	type BlockWeights = ();
	type BlockLength = BlockLength;
	type BaseCallFilter = frame_support::traits::Everything;
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Call = Call;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type BlockHashCount = ConstU64<250>;
	type Event = Event;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
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
pub type AssetsForceOrigin = EnsureRoot<AccountId>;

impl pallet_assets::Config for Test {
	type Event = Event;
	type Balance = Balance;
	type AssetId = AssetId;
	type Currency = Balances;
	type ForceOrigin = AssetsForceOrigin;
	type AssetDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = ();
	type AssetAccountDeposit = AssetAccountDeposit;
}

parameter_types! {
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ();
	type AccountStore = System;
	type MaxLocks = ();
	type WeightInfo = ();
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
}

parameter_types! {
	pub const AssetsExtPalletId: PalletId = PalletId(*b"assetext");
	pub const MaxHolds: u32 = 16;
	pub const XrpAssetId: AssetId = 2;
	pub const TestParachainId: u32 = 100;
}

impl pallet_assets_ext::Config for Test {
	type Event = Event;
	type ParachainId = TestParachainId;
	type MaxHolds = MaxHolds;
	type NativeAssetId = XrpAssetId;
	type PalletId = AssetsExtPalletId;
}

parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

// Time is measured by number of blocks.
pub const MILLISECS_PER_BLOCK: u64 = 4_000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

parameter_types! {
	pub const XrpTxChallengePeriod: u32 = 10 * MINUTES;
	pub const XrpClearTxPeriod: u32 = 10 * DAYS;
}

impl pallet_xrpl_bridge::Config for Test {
	type Event = Event;
	type WeightInfo = ();
	type ChallengePeriod = XrpTxChallengePeriod;
	type ClearTxPeriod = XrpClearTxPeriod;
	type MultiCurrency = AssetsExt;
	type XrpAssetId = XrpAssetId;
	type UnixTime = TimestampPallet;
	type ApproveOrigin = EnsureRoot<Self::AccountId>;
	type EthyAdapter = MockEthyAdapter;
}

pub struct MockEthyAdapter;

impl XrplEthyBridgeAdapter<AuthorityId> for MockEthyAdapter {
	/// Mock implementation of XrplEthyBridgeAdapter
	fn sign_xrpl_transaction(_tx_data: &[u8]) -> Result<EventProofId, DispatchError> {
		Ok(1)
	}
	fn validators() -> Vec<AuthorityId> {
		// some hard coded validators
		vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
		]
	}
	fn xrp_validators() -> Vec<AuthorityId> {
		// some hard coded validators
		vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
		]
	}
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}
