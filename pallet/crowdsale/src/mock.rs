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

use crate as pallet_crowdsale;
use seed_pallet_common::test_prelude::*;
use sp_runtime::testing::TestXt;

pub type Extrinsic = TestXt<RuntimeCall, ()>;

construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		Nft: pallet_nft,
		Crowdsale: pallet_crowdsale
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);
impl_pallet_nft_config!(Test);

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	RuntimeCall: From<C>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = Extrinsic;
}

pub struct ValidatedCall;
impl seed_pallet_common::ExtrinsicChecker for ValidatedCall {
	type Call = RuntimeCall;
	type Result = DispatchResult;
	fn check_extrinsic(_call: &Self::Call, _extra: &Self::Extra) -> Self::Result {
		Ok(())
	}
}

parameter_types! {
	pub const CrowdSalePalletId: PalletId = PalletId(*b"crowdsal");
	pub const MaxSalesPerBlock: u32 = 5;
	pub const MaxConsecutiveSales: u32 = 1000;
	pub const MaxPaymentsPerBlock: u32 = 5;
	pub const MaxSaleDuration: u64 = 1000;
	pub const UnsignedInterval: u32 = 10;
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletId = CrowdSalePalletId;
	type StringLimit = StringLimit;
	type ProxyCallValidator = ValidatedCall;
	type MultiCurrency = AssetsExt;
	type NFTExt = Nft;
	type MaxSalesPerBlock = MaxSalesPerBlock;
	type MaxConsecutiveSales = MaxConsecutiveSales;
	type MaxPaymentsPerBlock = MaxPaymentsPerBlock;
	type MaxSaleDuration = MaxSaleDuration;
	type UnsignedInterval = UnsignedInterval;
	type WeightInfo = ();
}
