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

use crate::*;
use frame_support::sp_io;
use sp_core::H160;
use sp_runtime::BuildStorage;

struct AssetsFixture<T: frame_system::Config + pallet_assets::Config> {
	pub id: <T as pallet_assets::Config>::AssetIdParameter,
	pub symbol: Vec<u8>,
	pub endowments: Vec<(T::AccountId, <T as pallet_assets::Config>::Balance)>,
}

impl<T: frame_system::Config + pallet_assets::Config> AssetsFixture<T> {
	fn new(
		id: <T as pallet_assets::Config>::AssetIdParameter,
		symbol: &[u8],
		endowments: &[(T::AccountId, <T as pallet_assets::Config>::Balance)],
	) -> Self {
		Self { id, symbol: symbol.to_vec(), endowments: endowments.to_vec() }
	}
}

/// Generec TestExternalities builder to be used across all pallets
pub struct TestExt<T: frame_system::Config + pallet_balances::Config + pallet_assets::Config> {
	balances: Vec<(T::AccountId, <T as pallet_balances::Config>::Balance)>,
	xrp_balances: Vec<(
		<T as pallet_assets::Config>::AssetIdParameter,
		T::AccountId,
		<T as pallet_assets::Config>::Balance,
	)>,
	assets: Vec<AssetsFixture<T>>,
	block_number: BlockNumber,
}

impl<T> Default for TestExt<T>
where
	T: frame_system::Config + pallet_balances::Config + pallet_assets::Config,
	<T as pallet_balances::Config>::Balance: From<Balance>,
{
	/// Create new TestExt with default values
	fn default() -> Self {
		Self { balances: vec![], xrp_balances: vec![], assets: vec![], block_number: 1 }
	}
}

impl<T> TestExt<T>
where
	T: frame_system::Config + pallet_balances::Config + pallet_assets::Config,
	<T as pallet_balances::Config>::Balance: From<Balance>,
	<T as pallet_assets::Config>::Balance: From<Balance>,
	T::AccountId: From<H160>,
	<T as pallet_assets::Config>::AssetIdParameter: From<AssetId>,
	<T as pallet_assets::Config>::AssetId: From<u32>,
	<T as frame_system::Config>::Hash: From<[u8; 32]>,
{
	/// Configure some native token balances
	pub fn with_balances(
		mut self,
		balances: &[(T::AccountId, <T as pallet_balances::Config>::Balance)],
	) -> Self {
		self.balances = balances.to_vec();
		self
	}

	/// Configures the root native asset to be created in the genesis config.
	/// We need to do this because the native asset is not created by default.
	/// This also ensures correct decimals are set for the native asset.
	pub fn configure_root(mut self) -> Self {
		self.assets.push(AssetsFixture::new(1.into(), "ROOT".as_bytes(), &[]));
		self
	}

	/// Configure an asset with id, name and some endowments
	/// total supply = sum(endowments)
	pub fn with_asset(
		mut self,
		id: <T as pallet_assets::Config>::AssetIdParameter,
		name: &str,
		endowments: &[(T::AccountId, <T as pallet_assets::Config>::Balance)],
	) -> Self {
		self.assets.push(AssetsFixture::new(id, name.as_bytes(), endowments));
		self
	}

	/// Configure some XRP asset balances
	pub fn with_xrp_balances(
		mut self,
		balances: &[(T::AccountId, <T as pallet_assets::Config>::Balance)],
	) -> Self {
		self.xrp_balances = balances
			.to_vec()
			.into_iter()
			.map(|(who, balance)| (2.into(), who, balance))
			.collect();
		self
	}

	/// Configure starting block number
	pub fn with_block_number(mut self, block_number: BlockNumber) -> Self {
		self.block_number = block_number;
		self
	}

	/// Build the Text Externalities for general use across all pallets
	pub fn build(self) -> sp_io::TestExternalities {
		let mut ext = frame_system::GenesisConfig::<T>::default().build_storage().unwrap();
		let mut assets = Vec::default();
		let mut metadata = Vec::default();
		let mut accounts = Vec::default();
		let default_owner = T::AccountId::from(H160::from_low_u64_be(100));

		// add assets
		if !self.assets.is_empty() {
			for AssetsFixture { id, symbol, endowments } in self.assets {
				assets.push((id.into(), default_owner.clone(), true, 1.into()));
				metadata.push((id.into(), symbol.clone(), symbol, 6));
				for (payee, balance) in endowments {
					accounts.push((id.into(), payee, balance));
				}
			}
		}

		// add xrp balances
		if !self.xrp_balances.is_empty() {
			assets.push((2.into(), default_owner, true, 1.into()));
			metadata.push((2.into(), b"XRP".to_vec(), b"XRP".to_vec(), 6_u8));
			for (_, payee, balance) in self.xrp_balances {
				accounts.push((2.into(), payee, balance));
			}
		}

		// Configure pallet_assets Genesis Config with assets
		if !assets.is_empty() {
			pallet_assets::GenesisConfig::<T> { assets, metadata, accounts }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

		// add initial balances to Genesis Config
		if !self.balances.is_empty() {
			pallet_balances::GenesisConfig::<T> { balances: self.balances }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

		let mut ext: sp_io::TestExternalities = ext.into();
		ext.execute_with(|| {
			frame_system::Pallet::<T>::initialize(
				&self.block_number.into(),
				&[0u8; 32].into(),
				&Default::default(),
			)
		});

		ext
	}
}
