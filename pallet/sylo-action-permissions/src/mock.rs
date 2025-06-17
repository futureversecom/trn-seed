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

use crate::{self as pallet_sylo_action_permissions};
use frame_support::traits::InstanceFilter;
use seed_pallet_common::test_prelude::*;
use sp_core::{ecdsa, Pair};

construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		Proxy: pallet_proxy,
		Futurepass: pallet_futurepass,
		Sudo: pallet_sudo,
		SyloActionPermissions: pallet_sylo_action_permissions,
	}
);

impl_frame_system_config!(Test);
impl_pallet_futurepass_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);
impl_pallet_proxy_config!(Test);

impl pallet_sudo::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = ();
}

parameter_types! {
	pub const MaxCallIds: u32 = 100;
	pub const StringLimit: u32 = 100;
	pub const XrplMaxMessageLength: u32 = 1000;
	pub const XrplMaxSignatureLength: u32 = 1000;
}

pub struct FuturepassIdentityLookup;
impl StaticLookup for FuturepassIdentityLookup {
	type Source = H160;
	type Target = H160;
	fn lookup(s: Self::Source) -> Result<Self::Target, LookupError> {
		pallet_futurepass::Holders::<Test>::get::<AccountId>(s.into())
			.map(|futurepass| futurepass.into())
			.ok_or(LookupError)
	}
	fn unlookup(t: Self::Target) -> Self::Source {
		t
	}
}

pub struct MockCallValidator;
impl seed_pallet_common::ExtrinsicChecker for MockCallValidator {
	type Call = RuntimeCall;
	type Extra = ();
	type Result = bool;
	fn check_extrinsic(_call: &Self::Call, _extra: &Self::Extra) -> Self::Result {
		false
	}
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type BlacklistedCallProvider = MockCallValidator;
	type FuturepassLookup = FuturepassIdentityLookup;
	type MaxCallIds = MaxCallIds;
	type StringLimit = StringLimit;
	type XrplMaxMessageLength = XrplMaxMessageLength;
	type XrplMaxSignatureLength = XrplMaxSignatureLength;
	type WeightInfo = ();
}

pub fn create_random_pair() -> (ecdsa::Pair, AccountId) {
	let (pair, _) = ecdsa::Pair::generate();
	let account: AccountId = pair.public().try_into().unwrap();
	(pair, account)
}
