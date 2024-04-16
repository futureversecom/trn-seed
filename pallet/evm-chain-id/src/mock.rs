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
use seed_pallet_common::test_prelude::*;

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		EVMChainId: pallet_evm_chain_id,
	}
);

impl_frame_system_config!(Test);

parameter_types! {
	pub const DefaultChainId: u64 = 7672;
}
impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ApproveOrigin = EnsureRoot<Self::AccountId>;
	type DefaultChainId = DefaultChainId;
	type WeightInfo = ();
}

#[derive(Clone, Copy, Default)]
pub struct TestExt;
impl TestExt {
	pub fn build(self) -> sp_io::TestExternalities {
		let storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
		let mut ext: sp_io::TestExternalities = storage.into();
		ext.execute_with(|| System::initialize(&1, &[0u8; 32].into(), &Default::default()));
		ext
	}
}
