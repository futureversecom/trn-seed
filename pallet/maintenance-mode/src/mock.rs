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

use crate as pallet_maintenance_mode;
use crate::MaintenanceChecker;
use frame_support::{parameter_types, PalletId};
use frame_system::{limits, EnsureRoot};
use seed_pallet_common::*;
use seed_primitives::{AccountId, AssetId, Balance};
use sp_core::{H160, H256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type SystemError = frame_system::Error<Test>;

pub const XRP_ASSET_ID: AssetId = 2;

pub fn create_account(seed: u64) -> AccountId {
	AccountId::from(H160::from_low_u64_be(seed))
}

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		MaintenanceMode: pallet_maintenance_mode,
	}
);

impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);

/// Filters to prevent specific transactions from executing
pub enum CallFilter {}

// TODO Move to maintenance mode pallet
impl frame_support::traits::Contains<Call> for CallFilter {
	fn contains(call: &Call) -> bool {
		// Check whether this call has been paused by the maintenance_mode pallet
		if MaintenanceChecker::<Test>::call_paused(call) {
			return false
		}

		return true
	}
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockLength: limits::BlockLength = limits::BlockLength::max(2 * 1024);
}

impl frame_system::Config for Test {
	type BaseCallFilter = CallFilter;
	type Origin = Origin;
	type Index = u64;
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

impl pallet_maintenance_mode::Config for Test {
	type Call = Call;
	type Event = Event;
}

#[derive(Default)]
pub struct TestExt {}

impl TestExt {
	pub fn build(self) -> sp_io::TestExternalities {
		let ext = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		let mut ext: sp_io::TestExternalities = ext.into();
		ext.execute_with(|| {
			System::initialize(&1, &[0u8; 32].into(), &Default::default());
		});

		ext
	}
}
