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

use super::*;
use crate::{self as pallet_sylo};
use seed_pallet_common::test_prelude::*;

construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		Sylo: pallet_sylo,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);

parameter_types! {
	pub const MaxResolvers: u32 = 10;
	pub const MaxTags: u32 = 10;
	pub const MaxEntries: u32 = 100;
	pub const MaxServiceEndpoints: u32 = 10;
	pub const StringLimit: u32 = 100;
}
impl Config for Test {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type ApproveOrigin = EnsureRoot<AccountId>;
	type MaxResolvers = MaxResolvers;
	type MaxTags = MaxTags;
	type MaxEntries = MaxEntries;
	type MaxServiceEndpoints = MaxServiceEndpoints;
	type StringLimit = StringLimit;
	type WeightInfo = ();
}
