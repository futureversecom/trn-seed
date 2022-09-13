use crate as pallet_xrpl_bridge;
use frame_support::{
	construct_runtime, parameter_types,
	traits::{ConstU16, ConstU64},
};
use frame_system as system;
use seed_primitives::AssetId;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

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
		XRPLBridge: pallet_xrpl_bridge::{Pallet, Call, Storage, Event<T>},
		AssetsExt: pallet_assets_ext::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const AssetsExtPalletId: PalletId = PalletId(*b"assetext");
	pub const MaxHolds: u32 = 16;
	pub const XrpAssetId: AssetId = 2;
	pub const WorldId: seed_primitives::ParachainId = 100;
}

impl pallet_assets_ext::Config for Runtime {
	type Event = Event;
	type ParachainId = WorldId;
	type MaxHolds = MaxHolds;
	type NativeAssetId = XrpAssetId;
	type PalletId = AssetsExtPalletId;
}

parameter_types! {
	pub const ChallengePeriod: u32 = 3000u32;
}

impl pallet_xrpl_bridge::Config for Test {
	type Event = Event;
	type WeightInfo = ();
	type ChallengePeriod = ChallengePeriod;
	type MultiCurrency = ();
	type XrpAssetId = XrpAssetId;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}
