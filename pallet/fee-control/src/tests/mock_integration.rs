/* Copyright 2021-2022 Centrality Investments Limited
 *
 * Licensed under the LGPL, Version 3.0 (the "License");
 * you may not use this file except in compliance with the License.
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * You may obtain a copy of the License at the root of this project source code,
 * or at:
 *     https://centrality.ai/licenses/gplv3.txt
 *     https://centrality.ai/licenses/lgplv3.txt
 */

use crate::{self as fee_control, types::DecimalBalance, *};

use core::marker::PhantomData;
use frame_system::{limits, EnsureRoot};
use pallet_evm::{AddressMapping, BlockHashMapping, EnsureAddressNever};
use pallet_transaction_payment::CurrencyAdapter;
pub use seed_primitives::types::Balance;

use frame_support::{
	parameter_types,
	traits::{Currency, FindAuthor, Imbalance, OnUnbalanced},
	weights::{DispatchClass, WeightToFee},
	ConsensusEngineId,
};
use frame_system::RawOrigin;
use sp_core::{H160, H256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	DispatchError, Perbill, Permill,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>},
		FeeControl: fee_control::{Pallet, Call, Storage, Event<T>},
		EVM: pallet_evm::{Pallet, Call, Storage, Event<T>},
		Ethereum: pallet_ethereum::{Pallet, Call, Storage, Event, Config, Origin},
		TimestampPallet: pallet_timestamp::{Pallet, Call, Storage, Inherent},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

pub type BlockNumber = u64;
pub type AccountId = u64;
pub const ALICE: AccountId = 0;
pub const BOB: AccountId = 1;
pub const TREASURY: AccountId = 420;

impl frame_system::Config for Test {
	type BlockWeights = ();
	type BlockLength = ();
	type BaseCallFilter = frame_support::traits::Everything;
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type BlockHashCount = BlockHashCount;
	type Event = Event;
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
	pub const OperationalFeeMultiplier: u8 = 5;
}

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
		if let Some(fees) = fees_then_tips.next() {
			let balance = Balances::free_balance(&TREASURY);
			let new_balance = balance.saturating_add(fees.peek());
			Balances::set_balance(root(), TREASURY, new_balance, Balance::from(0u32)).unwrap();
		}
	}
}

pub struct PaymentWeightToFee;
impl WeightToFee for PaymentWeightToFee {
	type Balance = Balance;
	fn weight_to_fee(weight: &Weight) -> Self::Balance {
		FeeControl::weight_to_fee(weight)
	}
}

pub struct PaymentLengthToFee;
impl WeightToFee for PaymentLengthToFee {
	type Balance = Balance;
	fn weight_to_fee(weight: &Weight) -> Self::Balance {
		FeeControl::length_to_fee(weight)
	}
}

impl pallet_transaction_payment::Config for Test {
	type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees>;
	type Event = Event;
	type WeightToFee = PaymentWeightToFee;
	type LengthToFee = PaymentLengthToFee;
	type FeeMultiplierUpdate = ();
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
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

pub struct MockBlockHashMapping<Test>(PhantomData<Test>);
impl<Test> BlockHashMapping for MockBlockHashMapping<Test> {
	fn block_hash(_number: u32) -> H256 {
		H256::default()
	}
}

pub struct MockAddressMapping;
impl AddressMapping<AccountId> for MockAddressMapping {
	fn into_account_id(_address: H160) -> AccountId {
		ALICE
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

impl pallet_evm::Config for Test {
	type FeeCalculator = FeeControl;
	type GasWeightMapping = ();
	type BlockHashMapping = MockBlockHashMapping<Test>;
	type CallOrigin = EnsureAddressNever<AccountId>;
	type WithdrawOrigin = EnsureAddressNever<AccountId>;
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
	type HandleTxValidation = ();
}

impl pallet_ethereum::Config for Test {
	type Event = Event;
	type StateRoot = pallet_ethereum::IntermediateStateRoot<Test>;
	type HandleTxValidation = ();
}

parameter_types! {
	pub const OneXRP: Balance = 1_000_000;
	pub const DefaultWeightMultiplier: Perbill = Perbill::from_parts(100);
	pub const DefaultLengthMultiplier: Balance = 1_000;
	pub const DefaultOutputTxPrice: Balance = 100_000;
	pub const DefaultOutputLenPrice: Balance = 10;
	/// Floor network base fee per gas
	/// 0.000015 XRP per gas, 15000 GWEI
	pub const DefaultEvmBaseFeePerGas: u128 = 15_000_000_000_000u128;
	pub const EvmXRPScaleFactor: Balance = 1_000_000_000_000;
	pub const FeeControlThreshold: Permill = Permill::from_parts(350000);
	pub const FeeControlElasticity: Permill = Permill::from_parts(5000);
	pub const FeeControlMaxBlockWeightThreshold: Permill = Permill::from_parts(750000);
}

pub struct InputGasLimit;
impl Get<U256> for InputGasLimit {
	fn get() -> U256 {
		21_000u32.into()
	}
}

pub struct InputTxWeight;
impl Get<Weight> for InputTxWeight {
	fn get() -> Weight {
		let tx_weight = <pallet_balances::weights::SubstrateWeight<Test> as pallet_balances::WeightInfo>::transfer_keep_alive();

		tx_weight + BaseWeight::get()
	}
}

pub struct BaseWeight;
impl BaseWeight {
	pub fn get() -> Weight {
		let base_block_weight: limits::BlockWeights =
			<Test as frame_system::Config>::BlockWeights::get();
		base_block_weight.get(DispatchClass::Normal).base_extrinsic
	}
}

impl Config for Test {
	type Event = Event;
	type WeightInfo = ();
	type CallOrigin = EnsureRoot<AccountId>;
	type OneXRP = OneXRP;
	type WeightMultiplier = DefaultWeightMultiplier;
	type LengthMultiplier = DefaultLengthMultiplier;
	type EvmBaseFeePerGas = DefaultEvmBaseFeePerGas;
	type OutputTxPrice = DefaultOutputTxPrice;
	type OutputLenPrice = DefaultOutputLenPrice;
	type InputTxWeight = InputTxWeight;
	type InputGasLimit = InputGasLimit;
	type EvmXRPScaleFactor = EvmXRPScaleFactor;
	type Threshold = FeeControlThreshold;
	type Elasticity = FeeControlElasticity;
	type MaxBlockWeightThreshold = FeeControlMaxBlockWeightThreshold;
}

#[derive(Default)]
pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build() -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		pallet_balances::GenesisConfig::<Test> {
			balances: vec![(ALICE, Balance::from(1_000_000_000_000_000_000u128))],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

#[allow(dead_code)]
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	t.into()
}

#[allow(dead_code)]
pub fn origin(account: AccountId) -> Origin {
	RawOrigin::Signed(account).into()
}

pub fn root() -> Origin {
	RawOrigin::Root.into()
}

pub struct SettingsBuilder(FeeControlData);

#[allow(dead_code)]
impl SettingsBuilder {
	pub fn new() -> Self {
		let mut data = SettingsAndMultipliers::<Test>::get();
		data.refresh_data = false;
		Self(data)
	}

	pub fn tx_weight(mut self, value: Weight) -> Self {
		self.0.input_tx_weight = value;
		self
	}

	pub fn gas_limit(mut self, value: U256) -> Self {
		self.0.input_gas_limit = value;
		self
	}

	pub fn tx_fee(mut self, value: Balance) -> Self {
		self.0.output_tx_fee = value;
		self
	}

	pub fn len_fee(mut self, value: Balance) -> Self {
		self.0.output_len_fee = value;
		self
	}

	pub fn weight_multiplier(mut self, value: Perbill) -> Self {
		self.0.weight_multiplier = value;
		self
	}

	pub fn length_multiplier(mut self, value: DecimalBalance) -> Self {
		self.0.length_multiplier = value;
		self
	}

	pub fn done(self) -> Result<(), DispatchError> {
		FeeControl::set_fee_control_config(
			root(),
			self.0.weight_multiplier.into(),
			self.0.length_multiplier.into(),
			ConfigOp::Noop,
			ConfigOp::Noop,
			self.0.input_tx_weight.into(),
			self.0.input_gas_limit.into(),
			self.0.output_tx_fee.into(),
			self.0.output_len_fee.into(),
			ConfigOp::Noop,
			self.0.refresh_data.into(),
		)
	}
}
