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

//! DEX benchmarking.

use super::*;

use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite, vec};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_core::H160;

use crate::Pallet as Dex;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId
where
	T::AccountId: From<H160>,
{
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId>
where
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
{
	RawOrigin::Signed(acc.clone())
}

fn mint_asset<T: Config>() -> AssetId
where
	<T as frame_system::Config>::AccountId: From<H160>,
{
	let alice = account::<T>("Alice");
	let asset_id = T::MultiCurrency::create(&alice, None).unwrap();
	let mint_amount = Balance::from(10_000_000u32);
	assert_ok!(T::MultiCurrency::mint_into(asset_id, &alice, mint_amount));

	asset_id
}

fn build_liquidity<T: Config>() -> (AssetId, AssetId)
where
	<T as frame_system::Config>::AccountId: From<H160>,
{
	let (asset_id_1, asset_id_2) = (mint_asset::<T>(), mint_asset::<T>());

	assert_ok!(Dex::<T>::add_liquidity(
		origin::<T>(&account::<T>("Alice")).into(),
		asset_id_1,
		asset_id_2,
		Balance::from(100_000u32),
		Balance::from(200_000u32),
		Balance::from(1_000u32),
		Balance::from(1_000u32),
		None,
		None,
	));

	(asset_id_1, asset_id_2)
}

benchmarks! {
	where_clause { where <T as frame_system::Config>::AccountId: From<sp_core::H160> + Into<sp_core::H160> }
	swap_with_exact_supply {
		let alice = account::<T>("Alice");
		let (asset_id_1, asset_id_2) = build_liquidity::<T>();
	}: _(origin::<T>(&alice), Balance::from(100u32), Balance::from(10u32), vec![asset_id_1, asset_id_2], None, None)

	swap_with_exact_target {
		let (asset_id_1, asset_id_2) = build_liquidity::<T>();
	}: _(origin::<T>(&account::<T>("Alice")), Balance::from(100u32), Balance::from(120u32), vec![asset_id_1, asset_id_2], None, None)

	add_liquidity {
		let (asset_id_1, asset_id_2) = (mint_asset::<T>(), mint_asset::<T>());
	}: _(origin::<T>(&account::<T>("Alice")), asset_id_1, asset_id_2, Balance::from(100000u32), Balance::from(200000u32), Balance::from(1000u32), Balance::from(1000u32), None, None)

	remove_liquidity {
		let (asset_id_1, asset_id_2) = build_liquidity::<T>();
	}: _(origin::<T>(&account::<T>("Alice")), asset_id_1, asset_id_2, Balance::from(100u32), Balance::from(10u32), Balance::from(10u32), None, None)

	reenable_trading_pair {
		let (asset_id_1, asset_id_2) = build_liquidity::<T>();
		assert_ok!(Dex::<T>::disable_trading_pair(RawOrigin::Root.into(), asset_id_1, asset_id_2));
	}: _(RawOrigin::Root, asset_id_1, asset_id_2)

	disable_trading_pair {
		let (asset_id_1, asset_id_2) = build_liquidity::<T>();
	}: _(RawOrigin::Root, asset_id_1, asset_id_2)

	set_fee_to {
		let fee_account = account::<T>("Alice");
	}: _(RawOrigin::Root, Some(fee_account))

	set_admin {
		let new_admin = account::<T>("Admin");
	}: _(RawOrigin::Root, new_admin.clone())
	verify {
		assert_eq!(crate::AdminAccount::<T>::get(), Some(new_admin));
	}
}

impl_benchmark_test_suite!(
	Dex,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
