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

//! A mock runtime for integration testing common runtime functionality
use crate::{self as pallet_assets_ext};
use frame_support::traits::FindAuthor;
use pallet_evm::{AddressMapping, BlockHashMapping, EnsureAddressNever, GasWeightMapping};
use seed_pallet_common::test_prelude::*;
use sp_runtime::ConsensusEngineId;
use std::marker::PhantomData;

construct_runtime!(
	pub enum Test where
		Block = Block<Test>,
		NodeBlock = Block<Test>,
		UncheckedExtrinsic = UncheckedExtrinsic<Test>,
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		EVM: pallet_evm,
		TimestampPallet: pallet_timestamp,
		FeeControl: pallet_fee_control,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_timestamp_config!(Test);
impl_pallet_evm_config!(Test);
impl_pallet_fee_control_config!(Test);

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

parameter_types! {
	pub const TestParachainId: seed_primitives::ParachainId = 100;
	pub const NativeAssetId: AssetId = 1;
	pub const AssetsExtPalletId: PalletId = PalletId(*b"assetext");
	pub const MaxHolds: u32 = 16;
}
impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ParachainId = TestParachainId;
	type MaxHolds = MaxHolds;
	type NativeAssetId = NativeAssetId;
	type OnNewAssetSubscription = MockNewAssetSubscription;
	type PalletId = AssetsExtPalletId;
	type WeightInfo = ();
}

#[derive(Default)]
pub struct TestExt {
	assets: Vec<AssetsFixture>,
	balances: Vec<(AccountId, Balance)>,
}

impl TestExt {
	/// Configure an asset with id, name and some endowments
	/// total supply = sum(endowments)
	pub fn with_asset(
		mut self,
		id: AssetId,
		name: &str,
		endowments: &[(AccountId, Balance)],
	) -> Self {
		self.assets.push(AssetsFixture::new(id, name.as_bytes(), endowments));
		self
	}
	/// Configure some native token balances
	pub fn with_balances(mut self, balances: &[(AccountId, Balance)]) -> Self {
		self.balances = balances.to_vec();
		self
	}
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		if !self.assets.is_empty() {
			let mut metadata = Vec::with_capacity(self.assets.len());
			let mut assets = Vec::with_capacity(self.assets.len());
			let mut accounts = Vec::<(AssetId, AccountId, Balance)>::default();

			let default_owner = create_account(100);
			for AssetsFixture { id, symbol, endowments } in self.assets {
				assets.push((id, default_owner, true, 1));
				metadata.push((id, symbol.clone(), symbol, 6));
				for (payee, balance) in endowments {
					accounts.push((id, payee, balance));
				}
			}

			pallet_assets::GenesisConfig::<Test> { assets, metadata, accounts }
				.assimilate_storage(&mut t)
				.unwrap();
		}

		if !self.balances.is_empty() {
			pallet_balances::GenesisConfig::<Test> { balances: self.balances }
				.assimilate_storage(&mut t)
				.unwrap();
		}

		let mut ext: sp_io::TestExternalities = t.into();
		ext.execute_with(|| crate::GenesisConfig::<Test>::default().build());

		ext
	}
}

/// Short helper
pub fn test_ext() -> TestExt {
	TestExt::default()
}

#[derive(Default)]
struct AssetsFixture {
	pub id: AssetId,
	pub symbol: Vec<u8>,
	pub endowments: Vec<(AccountId, Balance)>,
}

impl AssetsFixture {
	fn new(id: AssetId, symbol: &[u8], endowments: &[(AccountId, Balance)]) -> Self {
		Self { id, symbol: symbol.to_vec(), endowments: endowments.to_vec() }
	}
}

#[allow(dead_code)]
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
