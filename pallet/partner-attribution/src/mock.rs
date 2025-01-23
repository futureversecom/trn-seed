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

use crate as pallet_partner_attribution;
use frame_support::traits::EnsureOrigin;
use seed_pallet_common::test_prelude::*;
use sp_core::H160;

construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		PartnerAttribution: pallet_partner_attribution,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);

pub struct EnsureAny;

impl EnsureOrigin<<Test as frame_system::Config>::RuntimeOrigin> for EnsureAny {
	type Success = H160;
	fn try_origin(
		o: <Test as frame_system::Config>::RuntimeOrigin,
	) -> Result<Self::Success, <Test as frame_system::Config>::RuntimeOrigin> {
		match o.clone().into() {
			Ok(RawOrigin::Signed(who)) => Ok(who.into()),
			_ => Err(o),
		}
	}
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ApproveOrigin = EnsureRoot<Self::AccountId>;
	type EnsureFuturepass = EnsureAny;
	type WeightInfo = ();
}
