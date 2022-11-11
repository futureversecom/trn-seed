//! A mock runtime for integration testing common runtime functionality
use frame_support::{
	parameter_types,
	traits::{FindAuthor, GenesisBuild},
	weights::Weight,
	PalletId,
};
use frame_system::{limits, EnsureRoot};
use pallet_evm::{AddressMapping, BlockHashMapping, EnsureAddressNever, FeeCalculator};
use seed_pallet_common::OnNewAssetSubscriber;
use sp_core::{H160, H256, U256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	ConsensusEngineId,
};
use std::marker::PhantomData;

pub(crate) use seed_primitives::{AssetId, Balance, Index};

use crate::{self as pallet_assets_ext};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub type MockAccountId = u64;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Assets: pallet_assets::{Pallet, Storage, Config<T>, Event<T>},
		AssetsExt: pallet_assets_ext::{Pallet, Storage, Event<T>},
		EVM: pallet_evm::{Pallet, Config, Call, Storage, Event<T>},
		TimestampPallet: pallet_timestamp::{Pallet, Call, Storage, Inherent},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockLength: limits::BlockLength = limits::BlockLength::max(2 * 1024);
	pub const MaxReserves: u32 = 50;
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
pub type AssetsForceOrigin = EnsureRoot<MockAccountId>;

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

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> (U256, Weight) {
		(1.into(), 0u64)
	}
}

pub struct FindAuthorTruncated;
impl FindAuthor<H160> for FindAuthorTruncated {
	fn find_author<'a, I>(_digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		None
	}
}

pub struct MockAddressMapping;
impl AddressMapping<MockAccountId> for MockAddressMapping {
	fn into_account_id(_address: H160) -> MockAccountId {
		0_u64
	}
}

pub struct MockBlockHashMapping<Test>(PhantomData<Test>);
impl<Test> BlockHashMapping for MockBlockHashMapping<Test> {
	fn block_hash(_number: u32) -> H256 {
		H256::default()
	}
}

impl pallet_evm::Config for Test {
	type FeeCalculator = FixedGasPrice;
	type GasWeightMapping = ();
	type BlockHashMapping = MockBlockHashMapping<Test>;
	type CallOrigin = EnsureAddressNever<MockAccountId>;
	type WithdrawOrigin = EnsureAddressNever<MockAccountId>;
	type AddressMapping = MockAddressMapping;
	type Currency = Balances;
	type Event = Event;
	type Runner = pallet_evm::runner::stack::Runner<Self>;
	type PrecompilesType = ();
	type PrecompilesValue = ();
	type ChainId = ();
	type BlockGasLimit = ();
	type OnChargeTransaction = ();
	type FindAuthor = FindAuthorTruncated;
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

pub struct MockNewAssetSubscription;

impl<RuntimeId> OnNewAssetSubscriber<RuntimeId> for MockNewAssetSubscription
where
	RuntimeId: From<u32> + Into<u32>,
{
	fn on_asset_create(runtime_id: RuntimeId, _precompile_address_prefix: &[u8; 4]) {
		// Mock address without conversion
		let address = H160::from_low_u64_be(runtime_id.into().into());
		pallet_evm::Pallet::<Test>::create_account(
			address.into(),
			b"TRN Asset Precompile".to_vec(),
		);
	}
}

parameter_types! {
	pub const TestParachainId: seed_primitives::ParachainId = 100;
	pub const NativeAssetId: AssetId = 1;
	pub const AssetsExtPalletId: PalletId = PalletId(*b"assetext");
	pub const MaxHolds: u32 = 16;
}
impl crate::Config for Test {
	type Event = Event;
	type ParachainId = TestParachainId;
	type MaxHolds = MaxHolds;
	type NativeAssetId = NativeAssetId;
	type OnNewAssetSubscription = MockNewAssetSubscription;
	type PalletId = AssetsExtPalletId;
}

#[derive(Default)]
pub struct TestExt {
	assets: Vec<AssetsFixture>,
	balances: Vec<(MockAccountId, Balance)>,
}

impl TestExt {
	/// Configure an asset with id, name and some endowments
	/// total supply = sum(endowments)
	pub fn with_asset(
		mut self,
		id: AssetId,
		name: &str,
		endowments: &[(MockAccountId, Balance)],
	) -> Self {
		self.assets.push(AssetsFixture::new(id, name.as_bytes(), endowments));
		self
	}
	/// Configure some native token balances
	pub fn with_balances(mut self, balances: &[(MockAccountId, Balance)]) -> Self {
		self.balances = balances.to_vec();
		self
	}
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		if !self.assets.is_empty() {
			let mut metadata = Vec::with_capacity(self.assets.len());
			let mut assets = Vec::with_capacity(self.assets.len());
			let mut accounts = Vec::<(AssetId, MockAccountId, Balance)>::default();

			let default_owner = 100_u64;
			for AssetsFixture { id, symbol, endowments } in self.assets {
				assets.push((id, default_owner, true, 1));
				metadata.push((id, symbol.clone(), symbol, 6));
				for (payee, balance) in endowments {
					accounts.push((id, payee, balance));
				}
			}

			pallet_assets::GenesisConfig::<Test> { assets, metadata, accounts }
				.assimilate_storage(&mut t)
				.unwrap();
		}

		if !self.balances.is_empty() {
			pallet_balances::GenesisConfig::<Test> { balances: self.balances }
				.assimilate_storage(&mut t)
				.unwrap();
		}

		let mut ext: sp_io::TestExternalities = t.into();
		ext.execute_with(|| crate::GenesisConfig::<Test>::default().build());

		ext
	}
}

/// Short helper
pub fn test_ext() -> TestExt {
	TestExt::default()
}

#[derive(Default)]
struct AssetsFixture {
	pub id: AssetId,
	pub symbol: Vec<u8>,
	pub endowments: Vec<(MockAccountId, Balance)>,
}

impl AssetsFixture {
	fn new(id: AssetId, symbol: &[u8], endowments: &[(MockAccountId, Balance)]) -> Self {
		Self { id, symbol: symbol.to_vec(), endowments: endowments.to_vec() }
	}
}
