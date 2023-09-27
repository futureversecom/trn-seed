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
use frame_support::{parameter_types, PalletId};
use frame_system::EnsureRoot;
use seed_pallet_common::*;
use seed_primitives::{AccountId, AssetId, Balance};
use sp_core::{H160, H256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

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
		Sudo: pallet_sudo,
		// Timestamp: pallet_timestamp,
		// ImOnline: pallet_im_online,
		// Ethy: pallet_ethy,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);
// impl_pallet_timestamp_config!(Test);
//
// parameter_types! {
// 	pub NposSolutionPriority: TransactionPriority =
// 		Perbill::from_percent(90) * TransactionPriority::max_value();
// 	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
// 	pub const MaxKeys: u32 = 10_000;
// 	pub const MaxPeerInHeartbeats: u32 = 10_000;
// 	pub const MaxPeerDataEncodingSize: u32 = 1_000;
// }
// impl pallet_im_online::Config for Test {
// 	type AuthorityId = ImOnlineId;
// 	type RuntimeEvent = RuntimeEvent;
// 	type ValidatorSet = ();
// 	type NextSessionRotation = Babe;
// 	type ReportUnresponsiveness = Offences;
// 	type UnsignedPriority = ImOnlineUnsignedPriority;
// 	type WeightInfo = weights::pallet_im_online::WeightInfo<Runtime>;
// 	type MaxKeys = MaxKeys;
// 	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
// 	type MaxPeerDataEncodingSize = MaxPeerDataEncodingSize;
// }
//
// parameter_types! {
// 	pub const NotarizationThreshold: Percent = Percent::from_parts(66_u8);
// 	pub const BridgePalletId: PalletId = PalletId(*b"ethybrdg");
// 	pub const EpochDuration: u64 = 1000_u64;
// 	pub const ChallengerBond: Balance = 100;
// 	pub const RelayerBond: Balance = 202;
// 	pub const XrpAssetId: AssetId = XRP_ASSET_ID;
// 	pub const MaxXrplKeys: u8 = 8;
// 	pub const MaxNewSigners: u8 = 20;
// 	pub const AuthorityChangeDelay: BlockNumber = 75;
// }
// impl pallet_ethy::Config for Test {
// 	type AuthorityChangeDelay = AuthorityChangeDelay;
// 	type AuthoritySet = MockValidatorSet;
// 	type BridgePalletId = BridgePalletId;
// 	type EthCallSubscribers = MockEthCallSubscriber;
// 	type EthereumRpcClient = MockEthereumRpcClient;
// 	type EthyId = AuthorityId;
// 	type EventRouter = MockEventRouter;
// 	type FinalSessionTracker = MockFinalSessionTracker;
// 	type NotarizationThreshold = NotarizationThreshold;
// 	type UnixTime = MockUnixTime;
// 	type RuntimeCall = RuntimeCall;
// 	type RuntimeEvent = RuntimeEvent;
// 	type EpochDuration = EpochDuration;
// 	type ChallengeBond = ChallengerBond;
// 	type MultiCurrency = AssetsExt;
// 	type NativeAssetId = XrpAssetId;
// 	type RelayerBond = RelayerBond;
// 	type MaxXrplKeys = MaxXrplKeys;
// 	type Scheduler = Scheduler;
// 	type PalletsOrigin = OriginCaller;
// 	type MaxNewSigners = MaxNewSigners;
// 	type XrplBridgeAdapter = MockXrplBridgeAdapter;
// }

// Implement the sudo module's `Config` on the Test runtime.
impl pallet_sudo::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
}

impl pallet_maintenance_mode::Config for Test {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type StringLimit = AssetsStringLimit;
	type WeightInfo = ();
	type SudoPallet = Sudo;
	type TimestampPallet = Sudo;
	type ImOnlinePallet = Sudo;
	type EthyPallet = Sudo;
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

#[allow(dead_code)]
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
