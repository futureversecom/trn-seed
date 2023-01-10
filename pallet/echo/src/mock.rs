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
use crate::{self as pallet_echo, Config, PING};
use ethabi::{ParamType, Token};
use frame_support::{parameter_types, storage::StorageValue, PalletId};
use seed_pallet_common::{
	EthereumBridge, EthereumEventRouter as EthereumEventRouterT, EthereumEventSubscriber,
	EventRouterError, EventRouterResult,
};

use seed_primitives::{ethy::EventProofId, AccountId};
use sp_core::{H160, H256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	DispatchError, SaturatedConversion,
};

pub type BlockNumber = u64;
pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
pub type Block = frame_system::mocking::MockBlock<TestRuntime>;

frame_support::construct_runtime!(
	pub enum TestRuntime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Echo: pallet_echo::{Pallet, Call, Storage, Event},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}
impl frame_system::Config for TestRuntime {
	type BlockWeights = ();
	type BlockLength = ();
	type BaseCallFilter = frame_support::traits::Everything;
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
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
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const MockEchoPalletId: PalletId = PalletId(*b"pingpong");
}
impl Config for TestRuntime {
	type Event = Event;
	type EthereumBridge = MockBridge;
	type PalletId = MockEchoPalletId;
}

pub(crate) mod test_storage {
	//! storage used by tests to store mock EthBlocks and TransactionReceipts
	use crate::Config;
	use frame_support::decl_storage;
	use seed_primitives::ethy::EventProofId;

	pub struct Module<T>(sp_std::marker::PhantomData<T>);
	decl_storage! {
		trait Store for Module<T: Config> as EthBridgeTest {
			pub NextEventProofId: EventProofId;
		}
	}
}

pub struct MockBridge;
impl EthereumBridge for MockBridge {
	/// Mock sending an event to the bridge
	fn send_event(
		source: &H160,
		destination: &H160,
		event: &[u8],
	) -> Result<EventProofId, DispatchError> {
		let event_proof_id = test_storage::NextEventProofId::get();
		test_storage::NextEventProofId::put(event_proof_id.wrapping_add(1));
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

impl EthereumEventRouterT for MockEthereumEventRouter {
	/// Route an event to a handler at `destination`
	/// - `source` the sender address on Ethereum
	/// - `destination` the intended handler (pseudo) address
	/// - `data` the Ethereum ABI encoded event data
	fn route(source: &H160, destination: &H160, data: &[u8]) -> EventRouterResult {
		// Route event to specific subscriber pallet
		if destination == &<pallet_echo::Pallet<TestRuntime> as EthereumEventSubscriber>::address()
		{
			<pallet_echo::Pallet<TestRuntime> as EthereumEventSubscriber>::process_event(
				source, data,
			)
			.map_err(|(w, err)| (w, EventRouterError::FailedProcessing(err)))
		} else {
			Err((0, EventRouterError::NoReceiver))
		}
	}
}

#[derive(Clone, Copy, Default)]
pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut ext: sp_io::TestExternalities = frame_system::GenesisConfig::default()
			.build_storage::<TestRuntime>()
			.unwrap()
			.into();

		ext.execute_with(|| frame_system::Pallet::<TestRuntime>::set_block_number(1));

		ext
	}
}

/// Check the system event record contains `event`
pub(crate) fn has_event(event: crate::Event) -> bool {
	System::events()
		.into_iter()
		.map(|r| r.event)
		// .filter_map(|e| if let Event::Nft(inner) = e { Some(inner) } else { None })
		.find(|e| *e == Event::Echo(event.clone()))
		.is_some()
}

#[allow(dead_code)]
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<TestRuntime>().unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
