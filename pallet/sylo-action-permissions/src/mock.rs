use super::*;
use crate::{self as pallet_sylo_action_permissions};
use seed_pallet_common::test_prelude::*;

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
	pub const ModuleLimit: u32 = 100;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type ModuleLimit = ModuleLimit;
	// type WeightInfo = ();
}
