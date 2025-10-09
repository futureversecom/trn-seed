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

use crate as pallet_liquidity_pools;
use frame_system::EnsureRoot;
use seed_pallet_common::test_prelude::*;
use seed_primitives::AccountId;
use sp_runtime::testing::TestXt;

construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		LiquidityPools: pallet_liquidity_pools,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);

parameter_types! {
	pub const LiquidityPoolsPalletId: PalletId = PalletId(*b"lqdpools");
	pub const InterestRateBasePoint: u32 = 1_000_000;
	pub const UnsignedInterval: u32 =  5;
	pub const RolloverBatchSize: u32 = 10;
	pub const MaxStringLength: u32 = 1000;
	pub const MaxPoolsPerOnIdle: u32 = 5;
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PoolId = u32;
	type InterestRateBasePoint = InterestRateBasePoint;
	type MultiCurrency = AssetsExt;
	type PalletId = LiquidityPoolsPalletId;
	type UnsignedInterval = UnsignedInterval;
	type RolloverBatchSize = RolloverBatchSize;
	type MaxStringLength = MaxStringLength;
	type WeightInfo = ();
	type MaxPoolsPerOnIdle = MaxPoolsPerOnIdle;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	RuntimeCall: From<C>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = TestXt<RuntimeCall, ()>;
}
