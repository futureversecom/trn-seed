// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

use crate::{self as pallet_evm_chain_id, Config};
use frame_support::parameter_types;
use frame_system::EnsureRoot;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

pub type AccountId = u64;
pub const ALICE: AccountId = 10;

pub type BlockNumber = u64;
pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
pub type Block = frame_system::mocking::MockBlock<TestRuntime>;

frame_support::construct_runtime!(
	pub enum TestRuntime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		EVMChainId: pallet_evm_chain_id::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}
impl frame_system::Config for TestRuntime {
	type BlockWeights = ();
	type BlockLength = ();
	type BaseCallFilter = frame_support::traits::Everything;
	type RuntimeOrigin = RuntimeOrigin;
	type Index = u64;
	type BlockNumber = BlockNumber;
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
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const DefaultChainId: u64 = 7672;
}
impl Config for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
	type ApproveOrigin = EnsureRoot<Self::AccountId>;
	type DefaultChainId = DefaultChainId;
	type WeightInfo = ();
}

#[derive(Clone, Copy, Default)]
pub struct TestExt;
impl TestExt {
	pub fn build(self) -> sp_io::TestExternalities {
		let storage =
			frame_system::GenesisConfig::default().build_storage::<TestRuntime>().unwrap();
		let mut ext: sp_io::TestExternalities = storage.into();
		ext.execute_with(|| System::initialize(&1, &[0u8; 32].into(), &Default::default()));
		ext
	}
}
