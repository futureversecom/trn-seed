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

use crate as pallet_maintenance_mode;
use seed_pallet_common::test_prelude::*;

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		MaintenanceMode: pallet_maintenance_mode,
		Sudo: pallet_sudo,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);

// Implement the sudo module's `Config` on the Test runtime.
impl pallet_sudo::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = ();
}

impl pallet_maintenance_mode::Config for Test {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type StringLimit = AssetsStringLimit;
	type WeightInfo = ();
	type SudoPallet = Sudo;
	// Use Sudo for easy mock setup, tested in integration tests
	type TimestampPallet = Sudo;
	// Use Sudo for easy mock setup, tested in integration tests
	type ImOnlinePallet = Sudo;
	// Use Sudo for easy mock setup, tested in integration tests
	type EthyPallet = Sudo;
	type DemocracyPallet = Sudo;
	type PreimagePallet = Sudo;
	type CouncilPallet = Sudo;
	type SchedulerPallet = Sudo;
}
