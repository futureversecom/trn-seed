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

use crate as pallet_migration;
use crate::Config;
use frame_support::weights::constants::WEIGHT_REF_TIME_PER_MILLIS;
use seed_pallet_common::test_prelude::*;
use seed_primitives::migration::NoopMigration;
use sp_runtime::Perbill;

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Assets: pallet_assets,
		Balances: pallet_balances,
		Migration: pallet_migration,
	}
);

impl_frame_system_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_balance_config!(Test);

pub const WEIGHT_MILLISECS_PER_BLOCK: u64 = 1000;
pub const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_MILLISECS_PER_BLOCK * WEIGHT_REF_TIME_PER_MILLIS, u64::MAX);

parameter_types! {
	pub MaxMigrationWeight: Weight = Perbill::from_percent(20) * MAXIMUM_BLOCK_WEIGHT;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type CurrentMigration = NoopMigration<(CollectionUuid, SerialNumber)>;
	type MaxMigrationWeight = MaxMigrationWeight;
	type WeightInfo = ();
}
