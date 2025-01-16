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

use crate as pallet_fee_proxy;
use crate::*;
use frame_support::{
	traits::{FindAuthor, InstanceFilter},
	weights::WeightToFee,
};
use pallet_evm::{AddressMapping, BlockHashMapping, EnsureAddressNever, GasWeightMapping};
use precompile_utils::{Address, ErcIdConversion};
use seed_pallet_common::test_prelude::*;
use sp_runtime::ConsensusEngineId;

pub type XrpCurrency = pallet_assets_ext::AssetCurrency<Test, XrpAssetId>;

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		FeeProxy: pallet_fee_proxy,
		Dex: pallet_dex,
		AssetsExt: pallet_assets_ext,
		Balances: pallet_balances,
		Assets: pallet_assets,
		TransactionPayment: pallet_transaction_payment,
		EVM: pallet_evm,
		Timestamp: pallet_timestamp,
		Futurepass: pallet_futurepass,
		FeeControl: pallet_fee_control,
		Sylo: pallet_sylo,
		Xrpl: pallet_xrpl,
		Utility: pallet_utility,
		Proxy: pallet_proxy,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);
impl_pallet_transaction_payment_config!(Test);
impl_pallet_dex_config!(Test);
impl_pallet_timestamp_config!(Test);
impl_pallet_evm_config!(Test);
impl_pallet_futurepass_config!(Test);
impl_pallet_fee_control_config!(Test);
impl_pallet_sylo_config!(Test);
impl_pallet_xrpl_config!(Test);
impl_pallet_proxy_config!(Test);
impl_pallet_utility_config!(Test);

// Mock ErcIdConversion for testing purposes
impl<RuntimeId> ErcIdConversion<RuntimeId> for Test
where
	RuntimeId: From<u32> + Into<u32>,
{
	type EvmId = Address;

	fn evm_id_to_runtime_id(
		evm_id: Self::EvmId,
		_precompile_address_prefix: &[u8; 4],
	) -> Option<RuntimeId> {
		if H160::from(evm_id) == H160::from_low_u64_be(16000) {
			// Our expected value for the test
			return Some(RuntimeId::from(16000));
		}
		None
	}
	fn runtime_id_to_evm_id(
		runtime_id: RuntimeId,
		_precompile_address_prefix: &[u8; 4],
	) -> Self::EvmId {
		let id: u32 = runtime_id.into();
		Self::EvmId::from(H160::from_low_u64_be(id as u64))
	}
}

parameter_types! {
	pub const XrpAssetId: AssetId = XRP_ASSET_ID;
}
impl Config for Test {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type PalletsOrigin = OriginCaller;
	type FeeAssetId = XrpAssetId;
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<XrpCurrency, ()>;
	type ErcIdConversion = Self;
	type EVMBaseFeeProvider = ();
	type MaintenanceChecker = ();
}

/// type alias for runtime configured FeePreferencesRunner
pub type Runner = FeePreferencesRunner<Test, Test, Futurepass>;
