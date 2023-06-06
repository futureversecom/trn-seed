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

use crate::*;
pub use crate::{self as pallet_seed_utils};
use core::ops::Mul;

use codec::Encode;
use frame_support::{
	assert_ok, construct_runtime, parameter_types,
	traits::GenesisBuild,
	weights::{ConstantMultiplier, Weight, WeightToFee},
	PalletId,
};
use frame_system::{limits, EnsureRoot};
use pallet_transaction_payment::CurrencyAdapter;
use seed_primitives::{AccountId, AccountId20, RootOrGovernanceKeyGetter};
pub(crate) use seed_primitives::{AssetId, Balance, Index};
use sp_core::{traits::ReadRuntimeVersionExt, H256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Perbill, Permill,
};

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

parameter_types! {
	pub const GetExchangeFee: (u32, u32) = (3, 1000); // 0.3% fee
	pub const TradingPathLimit: u32 = 3;
	pub const DEXBurnPalletId: PalletId = PalletId(*b"burnaddr");
	pub const LPTokenDecimals: u8 = 6;
	pub const WithdrawAmount: u128 = 100;
	pub const XrpAssetId: AssetId = 2;
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

pub type XrpCurrency = pallet_assets_ext::AssetCurrency<Test, XrpAssetId>;

impl RootUpgrader for Test {
	fn set_code_cheap(code: Vec<u8>) -> Result<(), sp_runtime::DispatchError> {
		System::update_code_in_storage(code.as_ref())
	}
}

impl RootOrGovernanceKeyGetter<AccountId> for Test {
	fn get() -> AccountId {
		// TODO: Why doesn't this work after setting in genesis
		// Sudo::key().unwrap()
		AccountId20([1; 20])
	}
}

impl Config for Test {
	type Currency = XrpCurrency;
	type RootUpgrader = Test;
	type CallerKey = Test;
	type WithdrawAmount = WithdrawAmount;
}

impl pallet_sudo::Config for Test {
	type Event = Event;
	type Call = Call;
}

parameter_types! {
	pub const WeightToFeeReduction: Permill = Permill::from_parts(125);
	pub const TransactionByteFee: Balance = 2_500;
	pub const OperationalFeeMultiplier: u8 = 5;
}

/// `WeightToFee` implementation converts weight to fee using a fixed % deduction
pub struct PercentageOfWeight<M>(sp_std::marker::PhantomData<M>);

impl<M> WeightToFee for PercentageOfWeight<M>
where
	M: Get<Permill>,
{
	type Balance = Balance;

	fn weight_to_fee(weight: &Weight) -> Balance {
		M::get().mul(*weight as Balance)
	}
}

impl pallet_transaction_payment::Config for Test {
	type OnChargeTransaction = CurrencyAdapter<XrpCurrency, ()>;
	type Event = Event;
	type WeightToFee = PercentageOfWeight<WeightToFeeReduction>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = ();
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
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
		Sudo: pallet_sudo::{Pallet, Call, Storage, Config<T>, Event<T>},
		TransactionPayment: pallet_transaction_payment,
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Assets: pallet_assets::{Pallet, Storage, Config<T>, Event<T>},
		AssetsExt: pallet_assets_ext::{Pallet, Storage, Config<T>, Event<T>},
		SeedUtils: pallet_seed_utils,
	}
);

#[derive(Default)]
pub struct TestExt;

impl TestExt {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
		let mut ext: sp_io::TestExternalities = storage.clone().into();
		let alice = AccountId20([1; 20]);
		assert_ok!(pallet_sudo::GenesisConfig::<Test> { key: Some(alice) }
			.assimilate_storage(&mut storage));

		ext.execute_with(|| System::initialize(&1, &[0u8; 32].into(), &Default::default()));
		ext
	}
}

#[allow(dead_code)]
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let mut ext = sp_io::TestExternalities::new(t);

	struct ReadRuntimeVersion(Vec<u8>);
	impl sp_core::traits::ReadRuntimeVersion for ReadRuntimeVersion {
		fn read_runtime_version(
			&self,
			_wasm_code: &[u8],
			_ext: &mut dyn sp_externalities::Externalities,
		) -> Result<Vec<u8>, String> {
			Ok(self.0.clone())
		}
	}

	let version =
		sp_version::RuntimeVersion { spec_version: 2, impl_version: 1, ..Default::default() };
	let read_runtime_version = ReadRuntimeVersion(version.encode());

	ext.register_extension(ReadRuntimeVersionExt::new(read_runtime_version));
	ext.execute_with(|| System::set_block_number(1));
	ext
}
