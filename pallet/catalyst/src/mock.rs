use super::*;
use crate as pallet_catalyst;

use frame_support::{
	construct_runtime, ord_parameter_types, parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU32, ConstU64, SortedMembers},
};
use frame_system::EnsureSignedBy;
use pallet_catalyst_reward::StakingIdType;
use sp_core::H256;
use sp_runtime::{
	testing::TestXt,
	traits::{BlakeTwo256, IdentityLookup},
};

pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u64;

type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test {
		System: frame_system,
		Timestamp: pallet_timestamp,
		Balances: pallet_balances,
		Utils: plug_utils,
		Assets: pallet_assets,
		Staking: pallet_plugstaking,
		CatalystReward: pallet_catalyst_reward,
		Catalyst: pallet_catalyst,
	}
);

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type Nonce = u32;
	type BlockHashCount = ConstU64<250>;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<2>;
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

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type MaxLocks = ();
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type MaxHolds = ();
	type RuntimeHoldReason = ();
}

parameter_types! {
	pub const AssetDeposit: u64 = 1;
	pub const ApprovalDeposit: u64 = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: u64 = 1;
	pub const MetadataDepositPerByte: u64 = 1;
	pub const AssetAccountDeposit: Balance = 10;
}

type AssetId = u64;

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type AssetId = AssetId;
	type AssetIdParameter = parity_scale_codec::Compact<AssetId>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = AssetAccountDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = ();
	type RemoveItemsLimit = ConstU32<1000>;
	pallet_assets::runtime_benchmarks_enabled! {
		type BenchmarkHelper = ();
	}
}

ord_parameter_types! {
	pub const AdminAccountId: AccountId = TEST_ACCOUNT;
}

impl plug_utils::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Assets = Assets;
	type Currency = Balances;
	type AdminOrigin = EnsureSignedBy<AdminAccountId, AccountId>;
	type WeightInfo = ();
}

parameter_types! {
	pub const StakingPalletId: PalletId = PalletId(*b"plug/stk");
	pub const BaseMultiplier: u128 = 10u128.pow(18);
}

impl pallet_plugstaking::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type StakingAdminOrigin = EnsureSignedBy<AdminAccountId, AccountId>;
	type StakingPalletId = StakingPalletId;
	type BaseMultiplier = BaseMultiplier;
	type VerifiedUserOrigin = EnsureSignedBy<AdminAccountId, AccountId>;
}

parameter_types! {
	pub const CatalystRewardPalletId: PalletId = PalletId(*b"cata/cre");
	pub const CATStakingId: StakingIdType = 1111;
}

impl pallet_catalyst_reward::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type CATRewardAdminOrigin = EnsureSignedBy<AdminAccountId, AccountId>;
	type CATStakingId = CATStakingId;
	type YieldFarmingPalletId = CatalystRewardPalletId;
	type VerifiedUserOrigin = EnsureSignedBy<AdminAccountId, AccountId>;
	type WeightInfo = ();
}

parameter_types! {
	pub const CATPalletId: PalletId = PalletId(*b"catl/cat");
	pub const CatalystAssetId: AssetId = 2;
	pub const PLUGAssetId: AssetId = 3;
	pub const CatalystVoucherAssetId: AssetId = 4;
	pub const UnsignedInterval: BlockNumber =  30;
	pub const PayoutBatchSize: u32 = 4;
	pub const TimeDiminishingNo: u128 = 50000;
	pub TimeDiminishingBase: FixedU128 = FixedU128::saturating_from_rational(1461u32, 2u32);
	pub TimeDiminishingFactor: FixedU128 = FixedU128::from_inner(1000006540618180000); //1.00000654061818
}

pub struct OneToMany;
impl SortedMembers<u64> for OneToMany {
	fn sorted_members() -> Vec<u64> {
		vec![TEST_ACCOUNT, 1, 2, 3]
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn add(_m: &u128) {}
}

impl pallet_catalyst::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type CATPalletId = CATPalletId;
	type PLUGAssetId = PLUGAssetId;
	type CatalystAssetId = CatalystAssetId;
	type CatalystVoucherAssetId = CatalystVoucherAssetId;
	type CATIdentifier = u32;
	type CATAdminOrigin = EnsureSignedBy<AdminAccountId, AccountId>;
	type VerifiedUserOrigin = EnsureSignedBy<OneToMany, AccountId>;
	type TimeDiminishingNo = TimeDiminishingNo;
	type TimeDiminishingBase = TimeDiminishingBase;
	type TimeDiminishingFactor = TimeDiminishingFactor;
	type UnsignedInterval = UnsignedInterval;
	type PayoutBatchSize = PayoutBatchSize;
	type WeightInfo = ();
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	RuntimeCall: From<C>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = TestXt<RuntimeCall, ()>;
}

pub(crate) const TEST_ACCOUNT: <Test as frame_system::Config>::AccountId = 0;
