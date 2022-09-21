/* Copyright 2019-2021 Centrality Investments Limited
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

use crate as crml_erc20_peg;
use seed_primitives::types::{AssetId, Balance};

use frame_support::{pallet_prelude::*, parameter_types, PalletId};
use frame_system::EnsureRoot;
use sp_core::{H256, H160};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use seed_pallet_common::{EthAbiCodec, EventClaimVerifier};

pub const CENNZ_ASSET_ID: AssetId = 16000;
pub const CPAY_ASSET_ID: AssetId = 16001;
pub const NEXT_ASSET_ID: AssetId = 17000;

pub const STAKING_ASSET_ID: AssetId = CENNZ_ASSET_ID;
pub const SPENDING_ASSET_ID: AssetId = CPAY_ASSET_ID;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = u64;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		AssetsExt: pallet_assets_ext::{Pallet, Storage, Event<T>},
		Assets: pallet_assets::{Pallet, Storage, Config<T>, Event<T>},
		Erc20Peg: crml_erc20_peg::{Pallet, Call, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>}
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Test {
	type BlockWeights = ();
	type BlockLength = ();
	type BaseCallFilter = frame_support::traits::Everything;
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
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
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
}

parameter_types! {
	pub const AssetDeposit: Balance = 1_000_000;
	pub const AssetAccountDeposit: Balance = 16;
	pub const ApprovalDeposit: Balance = 1;
	pub const AssetsStringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 1 * 68;
	pub const MetadataDepositPerByte: Balance = 1;
}
pub type AssetsForceOrigin = EnsureRoot<AccountId>;

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

parameter_types! {
	pub const TestParachainId: u32 = 100;
	pub const MaxHolds: u32 = 16;
	pub const NativeAssetId: AssetId = 1;
	pub const AssetsExtPalletId: PalletId = PalletId(*b"assetext");
}

impl pallet_assets_ext::Config for Test {
	type Event = Event;
	type ParachainId = TestParachainId;
	type MaxHolds = MaxHolds;
	type NativeAssetId = NativeAssetId;
	type PalletId = AssetsExtPalletId;
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
	pub const DefaultListingDuration: u64 = 5;
	pub const MaxAttributeLength: u8 = 140;
	pub const PegPalletId: PalletId = PalletId(*b"py/erc20");
	pub const DepositEventSignature: [u8; 32] = hex_literal::hex!("76bb911c362d5b1feb3058bc7dc9354703e4b6eb9c61cc845f73da880cf62f61");
	pub const MaxLengthErc20Meta: u32 = 300;
	pub const MaxClaimsPerBlock: u32 = 300;
	pub const MaxReadyBlocks: u32 = 300;
	pub const MaxInitialErcMetas: u8 = 50;
}

impl crate::Config for Test {
	type DepositEventSignature = DepositEventSignature;
	type Event = Event;
	type EthBridge = MockEthBridge;
	type PegPalletId = PegPalletId;
	type MultiCurrency = AssetsExt;
	type MaxLengthErc20Meta = MaxLengthErc20Meta;
	type MaxClaimsPerBlock = MaxClaimsPerBlock;
	type MaxReadyBlocks = MaxReadyBlocks;
	type MaxInitialErcMetas = MaxInitialErcMetas;
}

/// Mock ethereum bridge
pub struct MockEthBridge;

impl EventClaimVerifier for MockEthBridge {
	/// Submit an event claim
	fn submit_event_claim(
		_contract_address: &H160,
		_event_signature: &H256,
		_tx_hash: &H256,
		_event_data: &[u8],
	) -> Result<u64, DispatchError> {
		Ok(1)
	}

	/// Generate proof of the given message
	/// Returns a unique proof Id on success
	fn generate_event_proof<M: EthAbiCodec>(_message: &M) -> Result<u64, DispatchError> {
		Ok(2)
	}
}

#[derive(Default)]
pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| {
			System::initialize(&1, &[0u8; 32].into(), &Default::default());
		});

		ext
	}
}
