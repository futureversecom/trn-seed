use super::*;
use crate::{self as pallet_sylo_action_permissions};
use seed_pallet_common::test_prelude::*;
use sp_core::{ecdsa, Pair};

construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		SyloActionPermissions: pallet_sylo_action_permissions,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);

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
		Ok(s)
	}
	fn unlookup(t: Self::Target) -> Self::Source {
		t
	}
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
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
