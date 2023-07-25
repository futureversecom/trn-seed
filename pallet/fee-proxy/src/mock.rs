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

use crate as pallet_fee_proxy;
use crate::*;
use frame_support::{
	parameter_types,
	traits::{AsEnsureOriginWithArg, FindAuthor, InstanceFilter},
	weights::{ConstantMultiplier, WeightToFee},
	PalletId,
};
use frame_system::{limits, EnsureNever, EnsureRoot};
use pallet_evm::{AddressMapping, BlockHashMapping, EnsureAddressNever, FeeCalculator};
use precompile_utils::{Address, ErcIdConversion};
use seed_pallet_common::*;
use seed_primitives::{AccountId, AssetId};
use sp_core::{H160, H256, U256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	ConsensusEngineId, Permill,
};
use std::ops::Mul;

pub const XRP_ASSET_ID: AssetId = 1;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		FeeProxy: pallet_fee_proxy::{Pallet, Call, Storage, Event<T>},
		Dex: pallet_dex::{Pallet, Call, Storage, Event<T>},
		AssetsExt: pallet_assets_ext::{Pallet, Call, Storage, Config<T>, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		RootTesting: pallet_root_testing::{Pallet, Call, Storage},
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>, Config<T>},
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>},
		EVM: pallet_evm::{Pallet, Config, Call, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Futurepass: pallet_futurepass,
	}
);

impl_pallet_futurepass_config!(Test);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockLength: limits::BlockLength = limits::BlockLength::max(2 * 1024);
	pub const MaxReserves: u32 = 50;
}

impl frame_system::Config for Test {
	type BlockWeights = ();
	type BlockLength = BlockLength;
	type BaseCallFilter = frame_support::traits::Everything;
	type RuntimeOrigin = RuntimeOrigin;
	type Index = u64;
	type BlockNumber = u64;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type BlockHashCount = BlockHashCount;
	type RuntimeEvent = RuntimeEvent;
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

// Mock ErcIdConversion for testing purposes
impl<RuntimeId> ErcIdConversion<RuntimeId> for Test
where
	RuntimeId: From<u32> + Into<u32>,
{
	type EvmId = Address;

	fn evm_id_to_runtime_id(
		evm_id: Self::EvmId,
		_precompile_address_prefix: &[u8; 4],
	) -> Option<RuntimeId> {
		if H160::from(evm_id) == H160::from_low_u64_be(16000) {
			// Our expected value for the test
			return Some(RuntimeId::from(16000))
		}
		None
	}
	fn runtime_id_to_evm_id(
		runtime_id: RuntimeId,
		_precompile_address_prefix: &[u8; 4],
	) -> Self::EvmId {
		let id: u32 = runtime_id.into();
		Self::EvmId::from(H160::from_low_u64_be(id as u64))
	}
}

pub type XrpCurrency = pallet_assets_ext::AssetCurrency<Test, XrpAssetId>;

parameter_types! {
		pub const XrpAssetId: AssetId = XRP_ASSET_ID;
}

impl Config for Test {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type PalletsOrigin = OriginCaller;
	type FeeAssetId = XrpAssetId;
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<XrpCurrency, ()>;
	type ErcIdConversion = Self;
}

parameter_types! {
	pub const GetExchangeFee: (u32, u32) = (3, 1000); // 0.3% fee
	pub const TradingPathLimit: u32 = 3;
	pub const DEXBurnPalletId: PalletId = PalletId(*b"burnaddr");
	pub const LPTokenDecimals: u8 = 6;
}
impl pallet_dex::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type GetExchangeFee = GetExchangeFee;
	type TradingPathLimit = TradingPathLimit;
	type DEXBurnPalletId = DEXBurnPalletId;
	type LPTokenDecimals = LPTokenDecimals;
	type WeightInfo = ();
	type MultiCurrency = AssetsExt;
}

parameter_types! {
	pub const AssetDeposit: Balance = 1_000_000;
	pub const AssetAccountDeposit: Balance = 16;
	pub const ApprovalDeposit: Balance = 1;
	pub const AssetsStringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 1 * 68;
	pub const MetadataDepositPerByte: Balance = 1;
	pub const RemoveItemsLimit: u32 = 656;
}
pub type AssetsForceOrigin = EnsureRoot<AccountId>;

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
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
	type RemoveItemsLimit = RemoveItemsLimit;
	type AssetIdParameter = AssetId;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureNever<AccountId>>;
	type CallbackHandle = ();
}

parameter_types! {
	pub const NativeAssetId: AssetId = 1;
	pub const AssetsExtPalletId: PalletId = PalletId(*b"assetext");
	pub const MaxHolds: u32 = 16;
	pub const TestParachainId: u32 = 100;
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

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ();
	type AccountStore = System;
	type MaxLocks = ();
	type WeightInfo = ();
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
}

parameter_types! {
	pub const TransactionByteFee: Balance = 2_500;
	pub const OperationalFeeMultiplier: u8 = 5;
	pub const WeightToFeeReduction: Permill = Permill::from_parts(125);
}

/// `WeightToFee` implementation converts weight to fee using a fixed % deduction
pub struct PercentageOfWeight<M>(sp_std::marker::PhantomData<M>);

impl<M> WeightToFee for PercentageOfWeight<M>
where
	M: Get<Permill>,
{
	type Balance = Balance;

	fn weight_to_fee(weight: &Weight) -> Balance {
		M::get().mul(weight.ref_time() as Balance)
	}
}

impl pallet_transaction_payment::Config for Test {
	type OnChargeTransaction = FeeProxy;
	type RuntimeEvent = RuntimeEvent;
	type WeightToFee = PercentageOfWeight<WeightToFeeReduction>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = ();
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
}

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> (U256, Weight) {
		(1.into(), 0u64.into())
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
impl AddressMapping<AccountId> for MockAddressMapping {
	fn into_account_id(address: H160) -> AccountId {
		address.into()
	}
}

pub struct MockBlockHashMapping<Test>(PhantomData<Test>);
impl<Test> BlockHashMapping for MockBlockHashMapping<Test> {
	fn block_hash(_number: u32) -> H256 {
		H256::default()
	}
}

parameter_types! {
	  pub WeightPerGas: Weight = Weight::from_parts(1, 0);
}

impl pallet_evm::Config for Test {
	type FeeCalculator = FixedGasPrice;
	type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
	type WeightPerGas = WeightPerGas;
	type BlockHashMapping = MockBlockHashMapping<Test>;
	type CallOrigin = EnsureAddressNever<AccountId>;
	type WithdrawOrigin = EnsureAddressNever<AccountId>;
	type AddressMapping = MockAddressMapping;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type PrecompilesType = ();
	type PrecompilesValue = ();
	type ChainId = ();
	type BlockGasLimit = ();
	type Runner = pallet_evm::runner::stack::Runner<Self>;
	type OnChargeTransaction = ();
	type OnCreate = ();
	type FindAuthor = FindAuthorTruncated;
	type Timestamp = Timestamp;
	type WeightInfo = ();
}

impl pallet_root_testing::Config for Test {}

parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

/// type alias for runtime configured FeePreferencesRunner
pub type Runner = FeePreferencesRunner<Test, Test, Futurepass>;

#[derive(Default)]
pub struct TestExt;

impl TestExt {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut ext: sp_io::TestExternalities =
			frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into();
		ext.execute_with(|| {
			System::initialize(&1, &[0u8; 32].into(), &Default::default());
		});
		ext
	}
}
