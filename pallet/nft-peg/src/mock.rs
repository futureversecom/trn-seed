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
use frame_support::pallet_prelude::*;
use seed_pallet_common::test_prelude::*;

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		AssetsExt: pallet_assets_ext,
		Assets: pallet_assets,
		Nft: pallet_nft,
		NftPeg: pallet_nft_peg,
		Balances: pallet_balances
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);
impl_pallet_nft_config!(Test);

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
	type NFTMinter = Nft;
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
		if destination == &<pallet_nft_peg::Pallet<Test> as EthereumEventSubscriber>::address() {
			<pallet_nft_peg::Pallet<Test> as EthereumEventSubscriber>::process_event(source, data)
				.map_err(|(w, err)| (w, EventRouterError::FailedProcessing(err)))
		} else {
			Err((Weight::zero(), EventRouterError::NoReceiver))
		}
	}
}
