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

use crate as pallet_marketplace;
use frame_support::{
	dispatch::DispatchResult,
	parameter_types,
	traits::{FindAuthor, GenesisBuild},
	PalletId,
};
use frame_system::EnsureRoot;
use pallet_evm::{AddressMapping, BlockHashMapping, EnsureAddressNever};
use seed_pallet_common::*;
use seed_primitives::{
	AccountId, AssetId, Balance, CollectionUuid, MetadataScheme, SerialNumber, TokenId,
};
use sp_core::{H160, H256};
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
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		Nft: pallet_nft,
		EVM: pallet_evm,
		FeeControl: pallet_fee_control,
		TimestampPallet: pallet_timestamp,
		Marketplace: pallet_marketplace,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);
impl_pallet_evm_config!(Test);
impl_pallet_fee_control_config!(Test);
impl_pallet_timestamp_config!(Test);

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
	pub const MaxAttributeLength: u8 = 140;
	pub const MaxOffers: u32 = 10;
	pub const MaxTokensPerCollection: u32 = 10_000;
	pub const MintLimit: u32 = 5000;
	pub const Xls20PaymentAsset: AssetId = XRP_ASSET_ID;
	pub const StringLimit: u32 = 50;
}

impl pallet_nft::Config for Test {
	type DefaultListingDuration = DefaultListingDuration;
	type Event = Event;
	type MaxOffers = MaxOffers;
	type MaxTokensPerCollection = MaxTokensPerCollection;
	type MintLimit = MintLimit;
	type MultiCurrency = AssetsExt;
	type OnTransferSubscription = MockTransferSubscriber;
	type OnNewAssetSubscription = MockNewAssetSubscription;
	type PalletId = NftPalletId;
	type ParachainId = TestParachainId;
	type StringLimit = StringLimit;
	type WeightInfo = ();
	type Xls20MintRequest = MockXls20MintRequest;
}

impl crate::Config for Test {
	type Call = Call;
	type WeightInfo = ();
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
