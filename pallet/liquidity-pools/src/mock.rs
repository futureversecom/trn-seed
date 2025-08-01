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
use frame_support::weights::Weight;
use frame_system::EnsureRoot;
use seed_pallet_common::test_prelude::*;
use seed_primitives::AccountId;
use sp_runtime::testing::TestXt;

// Mock weight implementation for testing
//
// NOTE: This pallet uses a custom TestWeightInfo struct instead of importing from
// another pallet, which deviates from the pattern used in other pallets. This
// deviation is intentional to make the pallet's tests self-contained and independent
// of the benchmarking process, ensuring tests can run without external dependencies.
pub struct TestWeightInfo;
impl crate::WeightInfo for TestWeightInfo {
	fn create_pool() -> Weight {
		Weight::from_parts(1000, 0)
	}
	fn set_pool_succession() -> Weight {
		Weight::from_parts(1000, 0)
	}
	fn set_pool_rollover() -> Weight {
		Weight::from_parts(1000, 0)
	}
	fn close_pool() -> Weight {
		Weight::from_parts(1000, 0)
	}
	fn enter_pool() -> Weight {
		Weight::from_parts(1000, 0)
	}
	fn exit_pool() -> Weight {
		Weight::from_parts(1000, 0)
	}
	fn claim_reward() -> Weight {
		Weight::from_parts(1000, 0)
	}
	fn rollover_unsigned() -> Weight {
		Weight::from_parts(1000, 0)
	}
	fn emergency_recover_funds() -> Weight {
		Weight::from_parts(1000, 0)
	}
	fn trigger_pool_update() -> Weight {
		Weight::from_parts(1000, 0)
	}
	fn process_closing_pools() -> Weight {
		Weight::from_parts(1000, 0)
	}
	fn process_closure_batch() -> Weight {
		Weight::from_parts(1000, 0)
	}
	fn process_pool_status_updates() -> Weight {
		Weight::from_parts(1000, 0)
	}
}

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
	type WeightInfo = TestWeightInfo;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	RuntimeCall: From<C>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = TestXt<RuntimeCall, ()>;
}
