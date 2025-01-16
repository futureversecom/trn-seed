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

use frame_support::{
	traits::{FindAuthor, InstanceFilter},
	weights::WeightToFee,
};
use pallet_evm::{AddressMapping, BlockHashMapping, EnsureAddressNever, GasWeightMapping};
use precompile_utils::{Address, ErcIdConversion};
use seed_pallet_common::test_prelude::*;
use seed_pallet_common::ExtrinsicChecker;
use sp_runtime::traits::{LookupError, StaticLookup};
use sp_runtime::ConsensusEngineId;

pub const MOCK_PAYMENT_ASSET_ID: AssetId = 100;

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		TransactionPayment: pallet_transaction_payment,
		FeeProxy: pallet_fee_proxy,
		Dex: pallet_dex,
		Evm: pallet_evm,
		Timestamp: pallet_timestamp,
		Futurepass: pallet_futurepass,
		Sylo: pallet_sylo_data_verification,
		MockPallet: mock_pallet::pallet,
		Xrpl: pallet_xrpl,
		Utility: pallet_utility,
		Proxy: pallet_proxy,
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
impl_pallet_sylo_data_verification_config!(Test);
impl_pallet_xrpl_config!(Test);
impl_pallet_proxy_config!(Test);
impl_pallet_utility_config!(Test);
impl_pallet_fee_control_config!(Test);

impl mock_pallet::pallet::Config for Test {}
// Mock pallet for testing extrinsics with a specific weight
pub mod mock_pallet {
	#[frame_support::pallet]
	pub mod pallet {
		use frame_support::pallet_prelude::*;
		use frame_system::pallet_prelude::*;
		#[pallet::pallet]
		pub struct Pallet<T>(_);

		#[pallet::config]
		pub trait Config: frame_system::Config {}

		#[pallet::genesis_config]
		pub struct GenesisConfig<T: Config> {
			_marker: PhantomData<T>,
		}

		impl<T: Config> Default for GenesisConfig<T> {
			fn default() -> Self {
				GenesisConfig { _marker: Default::default() }
			}
		}

		#[pallet::genesis_build]
		impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
			fn build(&self) {
				unimplemented!()
			}
		}

		// Some expected weight, given by a balances transfer
		pub const WEIGHT: Weight = Weight::zero();

		#[pallet::call]
		impl<T: Config> Pallet<T> {
			// For tests. Charge some expected fee amount
			#[pallet::call_index(0)]
			#[pallet::weight(WEIGHT)]
			pub fn mock_charge_fee(_origin: OriginFor<T>) -> DispatchResult {
				Ok(())
			}
		}
	}
}
