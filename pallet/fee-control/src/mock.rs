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

use crate::{self as pallet_fee_control, *};

use frame_system::EnsureRoot;
use pallet_evm::{AddressMapping, BlockHashMapping, EnsureAddressNever};
pub use seed_primitives::types::{AccountId, Balance};
use seed_primitives::AssetId;

use frame_support::{
	parameter_types,
	traits::{FindAuthor, InstanceFilter},
	weights::WeightToFee,
	PalletId,
};
use precompile_utils::{Address, ErcIdConversion};
use seed_pallet_common::*;
use sp_core::{H160, H256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	ConsensusEngineId,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub const MOCK_PAYMENT_ASSET_ID: AssetId = 100;

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
		TransactionPayment: pallet_transaction_payment,
		FeeProxy: pallet_fee_proxy,
		Dex: pallet_dex,
		Evm: pallet_evm,
		Futurepass: pallet_futurepass,
		MockPallet: mock_pallet::pallet,
		FeeControl: pallet_fee_control,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);
impl_pallet_transaction_payment_config!(Test);
impl_pallet_fee_proxy_config!(Test);
impl_pallet_dex_config!(Test);
impl_pallet_timestamp_config!(Test);
impl_pallet_evm_config!(Test);
impl_pallet_futurepass_config!(Test);
impl_pallet_fee_control_config!(Test);

impl mock_pallet::pallet::Config for Test {}
// Mock pallet for testing extrinsics with a specific weight
pub mod mock_pallet {
	#[frame_support::pallet]
	pub mod pallet {
		use frame_support::pallet_prelude::*;
		use frame_system::pallet_prelude::*;
		#[pallet::pallet]
		#[pallet::generate_store(pub(super) trait Store)]
		pub struct Pallet<T>(_);

		#[pallet::config]
		pub trait Config: frame_system::Config {}

		#[pallet::genesis_config]
		pub struct GenesisConfig<T: Config> {
			_marker: PhantomData<T>,
		}

		#[cfg(feature = "std")]
		impl<T: Config> Default for GenesisConfig<T> {
			fn default() -> Self {
				GenesisConfig { _marker: Default::default() }
			}
		}

		#[pallet::genesis_build]
		impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
			fn build(&self) {
				unimplemented!()
			}
		}

		// Some expected weight, given by a balances transfer
		pub const WEIGHT: Weight = 0;

		#[pallet::call]
		impl<T: Config> Pallet<T> {
			// For tests. Charge some expected fee amount
			#[pallet::weight(WEIGHT)]
			pub fn mock_charge_fee(_origin: OriginFor<T>) -> DispatchResult {
				Ok(())
			}
		}
	}
}

#[derive(Default)]
pub struct TestExt;

impl TestExt {
	pub fn build(self) -> sp_io::TestExternalities {
		let storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
		let mut ext: sp_io::TestExternalities = storage.into();
		ext.execute_with(|| System::initialize(&1, &[0u8; 32].into(), &Default::default()));
		ext
	}
}
