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

use crate::{self as fee_control, *};

use frame_system::EnsureRoot;
pub use seed_primitives::types::Balance;

use frame_support::parameter_types;
use frame_system::RawOrigin;
use sp_core::H256;
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
		FeeControl: fee_control::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

pub type BlockNumber = u64;
pub type AccountId = u64;

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
	pub const OneXRP: Balance = 1_000_000;
	pub const DefaultWeightMultiplier: Perbill = Perbill::from_parts(100);
	pub const DefaultLengthMultiplier: Balance = 1_000;
	pub const DefaultOutputTxPrice: Balance = 100_000;
	pub const DefaultOutputLenPrice: Balance = 10;
	/// Floor network base fee per gas
	/// 0.000015 XRP per gas, 15000 GWEI
	pub const DefaultEvmBaseFeePerGas: u64 = 15_000_000_000_000;
	pub const InputTxWeight: Weight = 100_000_000;
	pub const EvmXRPScaleFactor: Balance = 1_000_000_000_000;
	pub const FeeControlThreshold: Permill = Permill::from_parts(100000);
	pub const FeeControlElasticity: Permill = Permill::from_parts(5000);
	pub const FeeControlMaxBlockWeightThreshold: Permill = Permill::from_parts(750000);
}

pub struct InputGasLimit;
impl Get<U256> for InputGasLimit {
	fn get() -> U256 {
		21_000u32.into()
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
		let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

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

pub fn origin(account: AccountId) -> Origin {
	RawOrigin::Signed(account).into()
}

pub fn root() -> Origin {
	RawOrigin::Root.into()
}

pub struct SettingsBuilder(FeeControlData);

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

	pub fn done(self) -> Result<(), DispatchError> {
		FeeControl::set_fee_control_config(
			root(),
			ConfigOp::Noop,
			ConfigOp::Noop,
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
