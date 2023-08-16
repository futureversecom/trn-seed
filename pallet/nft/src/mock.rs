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

use crate as pallet_nft;
use frame_support::{
	dispatch::DispatchResult,
	parameter_types,
	traits::{FindAuthor, GenesisBuild},
	weights::Weight,
	PalletId,
};
use frame_system::{limits, EnsureRoot};
use pallet_evm::{AddressMapping, BlockHashMapping, EnsureAddressNever, FeeCalculator};
use seed_pallet_common::{OnNewAssetSubscriber, OnTransferSubscriber, Xls20MintRequest};
use seed_primitives::{
	AccountId, AssetId, Balance, CollectionUuid, MetadataScheme, SerialNumber, TokenId,
};
use sp_core::{H160, H256, U256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	ConsensusEngineId,
};
use std::marker::PhantomData;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

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
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Assets: pallet_assets::{Pallet, Storage, Config<T>, Event<T>},
		AssetsExt: pallet_assets_ext::{Pallet, Storage, Event<T>},
		Nft: pallet_nft::{Pallet, Call, Storage, Event<T>},
		EVM: pallet_evm::{Pallet, Config, Call, Storage, Event<T>},
		TimestampPallet: pallet_timestamp::{Pallet, Call, Storage, Inherent},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockLength: limits::BlockLength = limits::BlockLength::max(2 * 1024);
	pub const MaxReserves: u32 = 50;
}

impl frame_system::Config for Test {
	type BlockWeights = ();
	type BlockLength = BlockLength;
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
	pub const NativeAssetId: AssetId = 1;
	pub const AssetsExtPalletId: PalletId = PalletId(*b"assetext");
	pub const MaxHolds: u32 = 16;
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

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> (U256, Weight) {
		(1.into(), 0u64)
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

impl pallet_evm::Config for Test {
	type FeeCalculator = FixedGasPrice;
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

parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

pub struct MockTransferSubscriber;
impl OnTransferSubscriber for MockTransferSubscriber {
	fn on_nft_transfer(_token_id: &TokenId) {}
}

pub struct MockNewAssetSubscription;

impl<RuntimeId> OnNewAssetSubscriber<RuntimeId> for MockNewAssetSubscription
where
	RuntimeId: From<u32> + Into<u32>,
{
	fn on_asset_create(runtime_id: RuntimeId, _precompile_address_prefix: &[u8; 4]) {
		// Mock address without conversion
		let address = H160::from_low_u64_be(runtime_id.into().into());
		pallet_evm::Pallet::<Test>::create_account(
			address.into(),
			b"TRN Asset Precompile".to_vec(),
		);
	}
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
	pub const DefaultListingDuration: u64 = 5;
	pub const MaxOffers: u32 = 10;
	pub const TestParachainId: u32 = 100;
	pub const MaxTokensPerCollection: u32 = 10_000;
	pub const MintLimit: u32 = 5000;
	pub const Xls20PaymentAsset: AssetId = XRP_ASSET_ID;
	pub const StringLimit: u32 = 50;
}

impl crate::Config for Test {
	type Event = Event;
	type MaxTokensPerCollection = MaxTokensPerCollection;
	type MintLimit = MintLimit;
	type OnTransferSubscription = MockTransferSubscriber;
	type OnNewAssetSubscription = MockNewAssetSubscription;
	type PalletId = NftPalletId;
	type ParachainId = TestParachainId;
	type StringLimit = StringLimit;
	type WeightInfo = ();
	type Xls20MintRequest = MockXls20MintRequest;
}

#[derive(Default)]
pub struct TestExt {
	balances: Vec<(AccountId, Balance)>,
	xrp_balances: Vec<(AssetId, AccountId, Balance)>,
}

impl TestExt {
	/// Configure some native token balances
	pub fn with_balances(mut self, balances: &[(AccountId, Balance)]) -> Self {
		self.balances = balances.to_vec();
		self
	}
	/// Configure some XRP asset balances
	pub fn with_xrp_balances(mut self, balances: &[(AccountId, Balance)]) -> Self {
		self.xrp_balances = balances
			.to_vec()
			.into_iter()
			.map(|(who, balance)| (XRP_ASSET_ID, who, balance))
			.collect();
		self
	}
	pub fn build(self) -> sp_io::TestExternalities {
		let mut ext = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		if !self.balances.is_empty() {
			pallet_balances::GenesisConfig::<Test> { balances: self.balances }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

		if !self.xrp_balances.is_empty() {
			let assets = vec![(XRP_ASSET_ID, create_account(10), true, 1)];
			let metadata = vec![(XRP_ASSET_ID, b"XRP".to_vec(), b"XRP".to_vec(), 6_u8)];
			let accounts = self.xrp_balances;
			pallet_assets::GenesisConfig::<Test> { assets, metadata, accounts }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

		let mut ext: sp_io::TestExternalities = ext.into();
		ext.execute_with(|| {
			System::initialize(&1, &[0u8; 32].into(), &Default::default());
		});

		ext
	}
}

/// Check the system event record contains `event`
pub(crate) fn has_event(event: crate::Event<Test>) -> bool {
	System::events()
		.into_iter()
		.map(|r| r.event)
		// .filter_map(|e| if let Event::Nft(inner) = e { Some(inner) } else { None })
		.find(|e| *e == Event::Nft(event.clone()))
		.is_some()
}

#[allow(dead_code)]
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
