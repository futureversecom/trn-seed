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

use crate as pallet_erc20_peg;
use frame_support::pallet_prelude::*;
use seed_pallet_common::test_prelude::*;

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		AssetsExt: pallet_assets_ext,
		Assets: pallet_assets,
		Erc20Peg: pallet_erc20_peg::{Pallet, Call, Storage, Event<T>},
		Balances: pallet_balances
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);

parameter_types! {
	pub const PegPalletId: PalletId = PalletId(*b"py/erc20");
	pub const MaxDelaysPerBlock: u32 = 10;
	pub const MaxReadyBlocks: u32 = 10;
	pub const MaxNormalDispatchables: u32 = 100;
	pub const StringLimit: u32 = 50;
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type EthBridge = MockEthBridge;
	type PegPalletId = PegPalletId;
	type MultiCurrency = AssetsExt;
	type WeightInfo = ();
	type MaxNormalDispatchables = MaxNormalDispatchables;
	type NativeAssetId = NativeAssetId;
	type StringLimit = StringLimit;
	type MaxDelaysPerBlock = MaxDelaysPerBlock;
	type MaxReadyBlocks = MaxReadyBlocks;
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

impl EthereumEventRouter for MockEthereumEventRouter {
	/// Route an event to a handler at `destination`
	/// - `source` the sender address on Ethereum
	/// - `destination` the intended handler (pseudo) address
	/// - `data` the Ethereum ABI encoded event data
	fn route(source: &H160, destination: &H160, data: &[u8]) -> EventRouterResult {
		// Route event to specific subscriber pallet
		if destination == &<pallet_erc20_peg::Pallet<Test> as EthereumEventSubscriber>::address() {
			<pallet_erc20_peg::Pallet<Test> as EthereumEventSubscriber>::process_event(source, data)
				.map_err(|(w, err)| (w, EventRouterError::FailedProcessing(err)))
		} else {
			Err((Weight::zero(), EventRouterError::NoReceiver))
		}
	}
}

#[derive(Default)]
pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

		// Setup XRP asset
		let metadata = vec![(XRP_ASSET_ID, b"XRP".to_vec(), b"XRP".to_vec(), 6)];
		let default_account = create_account(100_u64);
		let assets = vec![(XRP_ASSET_ID, default_account, true, 1)];
		pallet_assets::GenesisConfig::<Test> { assets, metadata, accounts: vec![] }
			.assimilate_storage(&mut t)
			.unwrap();

		let mut ext: sp_io::TestExternalities = t.into();
		ext.execute_with(|| {
			System::initialize(&1, &[0u8; 32].into(), &Default::default());
		});

		ext
	}
}
