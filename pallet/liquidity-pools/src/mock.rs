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
	pub const ClosureBatchSize: u32 = 5; // FRN-68: For testing bounded closure
	pub const MaxPoolsPerBlock: u32 = 3; // Small number for testing
	pub const TransactionMaxAge: u32 = 64; // Transaction max age in blocks
	pub const MaxStringLength: u32 = 1000;
	pub const MaxUrgentUpdates: u32 = 10; // FRN-71: Max urgent updates in queue
	pub const MaxClosingPoolsPerBlock: u32 = 10; // new config for closing pools per block
	pub const MaxPoolsPerOffchainCall: u32 = 50; // new config for OCW batching
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PoolId = u32;
	type InterestRateBasePoint = InterestRateBasePoint;
	type MultiCurrency = AssetsExt;
	type PalletId = LiquidityPoolsPalletId;
	type UnsignedInterval = UnsignedInterval;
	type RolloverBatchSize = RolloverBatchSize;
	type ClosureBatchSize = ClosureBatchSize;
	type MaxPoolsPerBlock = MaxPoolsPerBlock;
	type TransactionMaxAge = TransactionMaxAge;
	type MaxStringLength = MaxStringLength;
	type MaxUrgentUpdates = MaxUrgentUpdates;
	type MaxClosingPoolsPerBlock = MaxClosingPoolsPerBlock;
	type MaxPoolsPerOffchainCall = MaxPoolsPerOffchainCall;
	type WeightInfo = ();
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	RuntimeCall: From<C>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = TestXt<RuntimeCall, ()>;
}
