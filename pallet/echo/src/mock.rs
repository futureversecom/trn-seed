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

use crate::{self as pallet_echo, Config as EchoConfig, Weight, PING};
use ethabi::{ParamType, Token};
use seed_pallet_common::test_prelude::*;
use seed_primitives::ethy::EventProofId;
use sp_runtime::SaturatedConversion;

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Assets: pallet_assets,
		Balances: pallet_balances,
		Echo: pallet_echo,
	}
);

impl_frame_system_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_balance_config!(Test);

pub use test_storage::*;

// Pallet to provide the version, used to test runtime upgrade version changes
#[frame_support::pallet]
pub mod test_storage {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		//type RuntimeEvent: From<Event<Self>> + IsType<<Self as
		// frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub type TestVal<T: Config> = StorageValue<_, bool, ValueQuery>;

	// #[pallet::event]
	// #[pallet::generate_deposit(pub(super) fn deposit_event)]
	// pub enum Event<T: Config> {
	// 	// XCMP
	// 	/// Some XCM was executed OK.
	// 	VersionChanged,
	// }

	impl<T: Config> Pallet<T> {
		pub fn set_test_val(new_val: bool) {
			TestVal::<T>::put(new_val);
		}
	}
}

impl test_storage::Config for Test {
	//type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
	pub const MockEchoPalletId: PalletId = PalletId(*b"pingpong");
}
impl EchoConfig for Test {
	type RuntimeEvent = RuntimeEvent;
	type EthereumBridge = MockBridge;
	type PalletId = MockEchoPalletId;
	type WeightInfo = ();
}

pub struct MockBridge;
impl EthereumBridge for MockBridge {
	/// Mock sending an event to the bridge
	fn send_event(
		source: &H160,
		destination: &H160,
		event: &[u8],
	) -> Result<EventProofId, DispatchError> {
		let event_proof_id = 123;
		match ethabi::decode(&[ParamType::Uint(64), ParamType::Uint(64), ParamType::Address], event)
		{
			Ok(abi) => {
				// If coming from extrinsic, immediately process event
				if let [Token::Uint(ping_or_pong), Token::Uint(_session_id), Token::Address(_destination)] =
					abi.as_slice()
				{
					let ping_or_pong: u8 = (*ping_or_pong).saturated_into();
					if ping_or_pong == PING {
						let _ = MockEthereumEventRouter::route(source, destination, event);
					}
				}
			},
			Err(_) => return Ok(event_proof_id),
		};
		Ok(event_proof_id)
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
		if destination == &<pallet_echo::Pallet<Test> as EthereumEventSubscriber>::address() {
			<pallet_echo::Pallet<Test> as EthereumEventSubscriber>::process_event(source, data)
				.map_err(|(w, err)| (w, EventRouterError::FailedProcessing(err)))
		} else {
			Err((Weight::zero(), EventRouterError::NoReceiver))
		}
	}
}
