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

use crate as pallet_nft_peg;

use seed_pallet_common::{
	EthereumBridge, EthereumEventRouter as EthereumEventRouterT, EthereumEventSubscriber,
	EventRouterError, EventRouterResult, Xls20MintRequest,
};
use seed_primitives::types::{AccountId, AssetId, Balance};

use frame_support::{pallet_prelude::*, parameter_types, PalletId};
use frame_system::EnsureRoot;
use sp_core::{H160, H256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Permill,
};

use seed_pallet_common::OnTransferSubscriber;
use seed_primitives::{CollectionUuid, MetadataScheme, SerialNumber, TokenId};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub const XRP_ASSET_ID: AssetId = 1;
pub const SPENDING_ASSET_ID: AssetId = XRP_ASSET_ID;

pub fn create_account(seed: u64) -> AccountId {
	AccountId::from(H160::from_low_u64_be(seed))
}

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		AssetsExt: pallet_assets_ext::{Pallet, Storage, Event<T>},
		Assets: pallet_assets::{Pallet, Storage, Config<T>, Event<T>},
		Nft: pallet_nft::{Pallet, Storage, Config<T>, Event<T>},
		NftPeg: pallet_nft_peg::{Pallet, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>}
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

type BlockNumber = u64;

impl frame_system::Config for Test {
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
}

parameter_types! {
	pub const MaxHolds: u32 = 16;
	pub const NativeAssetId: AssetId = 1;
	pub const AssetsExtPalletId: PalletId = PalletId(*b"assetext");
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

parameter_types! {
	pub const MaxReserves: u32 = 50;
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
	pub const PegPalletId: PalletId = PalletId(*b"  nftpeg");
}

pub struct MockTransferSubscriber;
impl OnTransferSubscriber for MockTransferSubscriber {
	fn on_nft_transfer(_token_id: &TokenId) {}
}

pub struct MockXls20MintRequest;
impl Xls20MintRequest for MockXls20MintRequest {
	type AccountId = AccountId;
	fn request_xls20_mint(
		_who: &Self::AccountId,
		_collection_id: CollectionUuid,
		_serial_numbers: Vec<SerialNumber>,
		_metadata_scheme: MetadataScheme,
	) -> DispatchResult {
		Ok(())
	}
}

parameter_types! {
	pub const NftPalletId: PalletId = PalletId(*b"nftokens");
	pub const TestParachainId: u32 = 100;
	pub const MaxTokensPerCollection: u32 = 10_000;
	pub const Xls20PaymentAsset: AssetId = XRP_ASSET_ID;
	pub const MintLimit: u32 = 1_000;
	pub const StringLimit: u32 = 50;
	pub const FeePotId: PalletId = PalletId(*b"txfeepot");
	pub const MarketplaceNetworkFeePercentage: Permill = Permill::from_perthousand(5);
	pub const DefaultFeeTo: Option<PalletId> = None;
}

impl pallet_nft::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MaxTokensPerCollection = MaxTokensPerCollection;
	type MintLimit = MintLimit;
	type OnTransferSubscription = MockTransferSubscriber;
	type OnNewAssetSubscription = ();
	type PalletId = NftPalletId;
	type ParachainId = TestParachainId;
	type StringLimit = StringLimit;
	type WeightInfo = ();
	type Xls20MintRequest = MockXls20MintRequest;
}

parameter_types! {
	pub const NftPegPalletId: PalletId = PalletId(*b"  nftpeg");
	pub const DelayLength: BlockNumber = 5;
	pub const MaxAddresses: u32 = 30;
	pub const MaxIdsPerMultipleMint: u32 = 50;
	pub const MaxCollectionsPerWithdraw: u32 = 10;
	pub const MaxSerialsPerWithdraw: u32 = 50;
}

impl pallet_nft_peg::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = NftPegPalletId;
	type DelayLength = DelayLength;
	type MaxAddresses = MaxAddresses;
	type MaxTokensPerMint = MaxIdsPerMultipleMint;
	type EthBridge = MockEthBridge;
	type NftPegWeightInfo = ();
	type MaxCollectionsPerWithdraw = MaxCollectionsPerWithdraw;
	type MaxSerialsPerWithdraw = MaxSerialsPerWithdraw;
}

/// Mock ethereum bridge
pub struct MockEthBridge;
impl EthereumBridge for MockEthBridge {
	fn send_event(
		_source: &H160,
		_destination: &H160,
		_message: &[u8],
	) -> Result<seed_primitives::ethy::EventProofId, DispatchError> {
		Ok(1)
	}
}

/// Handles routing verified bridge messages to other pallets
pub struct MockEthereumEventRouter;

impl EthereumEventRouterT for MockEthereumEventRouter {
	/// Route an event to a handler at `destination`
	/// - `source` the sender address on Ethereum
	/// - `destination` the intended handler (pseudo) address
	/// - `data` the Ethereum ABI encoded event data
	fn route(source: &H160, destination: &H160, data: &[u8]) -> EventRouterResult {
		// Route event to specific subscriber pallet
		if destination == &<pallet_nft_peg::Pallet<Test> as EthereumEventSubscriber>::address() {
			<pallet_nft_peg::Pallet<Test> as EthereumEventSubscriber>::process_event(source, data)
				.map_err(|(w, err)| (w, EventRouterError::FailedProcessing(err)))
		} else {
			Err((Weight::zero(), EventRouterError::NoReceiver))
		}
	}
}

/// Check the system event record contains `event`
pub(crate) fn has_event(event: crate::Event<Test>) -> bool {
	System::events()
		.into_iter()
		.map(|r| r.event)
		.find(|e| *e == RuntimeEvent::NftPeg(event.clone()))
		.is_some()
}

#[derive(Default)]
pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut ext: sp_io::TestExternalities =
			frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into();

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
