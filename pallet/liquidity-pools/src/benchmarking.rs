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

use crate::Pallet as LiquidityPools;
use frame_benchmarking::{account as bench_account, *};
use frame_support::{
	assert_ok,
	traits::{fungibles::Mutate, Currency, ExistenceRequirement},
};
use frame_system::RawOrigin;
use seed_pallet_common::CreateExt;
use seed_primitives::AssetId;
use sp_core::H160;

// Helper function to get test account
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

fn mint_asset<T: Config>(acc: &T::AccountId, asset_name: &str) -> AssetId
where
	<T as frame_system::Config>::AccountId: From<H160>,
{
	// mint native token
	assert_ok!(T::Assets::mint_into(1u32.into(), &acc, 10_000_000u32.into()));

	let asset_id =
		T::Assets::create_with_metadata(&acc, asset_name.into(), asset_name.into(), 6, None)
			.unwrap();
	assert_ok!(T::Assets::mint_into(asset_id.into(), &acc, 10_000_000u32.into()));

	asset_id.into()
}

benchmarks! {
	create_pool {
		let asset_id = AssetId::default();
		let interest_rate = 10;
		let max_tokens = 1000u32.into();
		let start_block = 1000u32.into();
		let end_block = 2000u32.into();
	}: _(RawOrigin::Root, asset_id, interest_rate, max_tokens, start_block, end_block)
	verify {
		let next_pool_id = T::PoolId::default();
		assert_eq!(Pools::<T>::get(next_pool_id).is_some(), true);
	}

	close_pool {
		let asset_id = AssetId::default();
		let interest_rate = 10;
		let max_tokens = 1000u32.into();
		let start_block = 1000u32.into();
		let end_block = 2000u32.into();
		let id = NextPoolId::<T>::get();
		LiquidityPools::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block).unwrap();
	}: _(RawOrigin::Root, id)
	verify {
		let next_pool_id = T::PoolId::default();
		assert_eq!(Pools::<T>::get(id).is_none(), true);
	}

	set_pool_succession {
		// Insert test pools
		let asset_id = AssetId::default();
		let interest_rate = 2;
		let max_tokens = 3u32.into();
		let start_block = 4u32.into();
		let end_block = 5u32.into();

		let predecessor_id = NextPoolId::<T>::get();
		LiquidityPools::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block).unwrap();

		let successor_id = NextPoolId::<T>::get();
		let start_block = 6u32.into();
		let end_block = 7u32.into();
		LiquidityPools::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block).unwrap();
	}: _(RawOrigin::Root, predecessor_id, successor_id)
	verify {
		assert_eq!(PoolRelationships::<T>::get(predecessor_id).unwrap().successor_id, Some(successor_id));
	}

	// Update user rollover preference
	set_pool_rollover {
		let remaining_weight: Weight =
			T::DbWeight::get().reads(100u64).saturating_add(T::DbWeight::get().writes(100u64));
		let alith = account::<T>("Alith");
		let asset_id = mint_asset::<T>(&alith, &"TEST");

		let id = NextPoolId::<T>::get();

		let vault_account = LiquidityPools::<T>::account_id();
		assert_ok!(T::Assets::mint_into(asset_id.into(), &vault_account, 10_000_000u32.into()));

		let user = account::<T>("user");
		assert_ok!(T::Assets::mint_into(asset_id.into(), &user, 10u32.into()));
		// Insert test pool user
		let interest_rate = 2;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();
		assert_ok!(LiquidityPools::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block));
		Pools::<T>::mutate(id, |pool| {
			*pool = Some(PoolInfo {
				pool_status: PoolStatus::Open,
				..pool.clone().unwrap()
			});
		});
		LiquidityPools::<T>::join_pool(RawOrigin::Signed(user.clone()).into(), id, 10u32.into()).unwrap();
	}: _(RawOrigin::Signed(user), id, true)
	verify {
		assert_eq!(PoolUsers::<T>::get(id, user).unwrap().should_rollover, true);
	}

	// Join reward pool
	join_pool {
		let remaining_weight: Weight =
			T::DbWeight::get().reads(100u64).saturating_add(T::DbWeight::get().writes(100u64));
		let amount = 10u32.into();
		let alith = account::<T>("Alith");
		let asset_id = mint_asset::<T>(&alith, &"TEST");

		let id = NextPoolId::<T>::get();

		let vault_account = LiquidityPools::<T>::account_id();
		assert_ok!(T::Assets::mint_into(asset_id.into(), &vault_account, 10_000_000u32.into()));

		let user = account::<T>("user");
		assert_ok!(T::Assets::mint_into(asset_id.into(), &user, 10u32.into()));
		// Insert test pool user
		let interest_rate = 2;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();
		LiquidityPools::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block).unwrap();
		Pools::<T>::mutate(id, |pool| {
			*pool = Some(PoolInfo {
				pool_status: PoolStatus::Open,
				..pool.clone().unwrap()
			});
		});
	}: _(RawOrigin::Signed(user.clone()), id, amount)
	verify {
		assert_eq!(PoolUsers::<T>::get(id, user).unwrap().amount, amount);
	}

	// Exit reward pool
	exit_pool {
		let remaining_weight: Weight =
			T::DbWeight::get().reads(100u64).saturating_add(T::DbWeight::get().writes(100u64));
		let alith = account::<T>("Alith");
		let asset_id = mint_asset::<T>(&alith, &"TEST");

		let id = NextPoolId::<T>::get();

		let vault_account = LiquidityPools::<T>::account_id();
		assert_ok!(T::Assets::mint_into(asset_id.into(), &vault_account, 10_000_000u32.into()));

		let user = account::<T>("user");
		assert_ok!(T::Assets::mint_into(asset_id.into(), &user, 10u32.into()));
		// Insert test pool user
		let interest_rate = 2;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();
		LiquidityPools::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block).unwrap();
		Pools::<T>::mutate(id, |pool| {
			*pool = Some(PoolInfo {
				pool_status: PoolStatus::Open,
				..pool.clone().unwrap()
			});
		});
		LiquidityPools::<T>::join_pool(RawOrigin::Signed(user.clone()).into(), id, 10u32.into()).unwrap();
	}: _(RawOrigin::Signed(user.clone()), id)
	verify {
		assert!(PoolUsers::<T>::get(id, user).is_none());
	}

	// Claim reward
	claim_reward {
		let remaining_weight: Weight =
			T::DbWeight::get().reads(100u64).saturating_add(T::DbWeight::get().writes(100u64));
		let alith = account::<T>("Alith");
		let asset_id = mint_asset::<T>(&alith, &"TEST");

		let id = NextPoolId::<T>::get();

		let vault_account = LiquidityPools::<T>::account_id();
		assert_ok!(T::Assets::mint_into(asset_id.into(), &vault_account, 10_000_000u32.into()));
		assert_ok!(T::Currency::transfer(&alith, &vault_account, 10_000_000u32.into(), ExistenceRequirement::AllowDeath));

		let user = account::<T>("user");
		assert_ok!(T::Assets::mint_into(asset_id.into(), &user, 10u32.into()));
		// Insert test pool user
		let interest_rate = 1_000_000;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();
		LiquidityPools::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block).unwrap();
		Pools::<T>::mutate(id, |pool| {
			*pool = Some(PoolInfo {
				pool_status: PoolStatus::Open,
				..pool.clone().unwrap()
			});
		});

		LiquidityPools::<T>::join_pool(RawOrigin::Signed(user.clone()).into(), id, 10u32.into()).unwrap();

		Pools::<T>::mutate(id, |pool| {
			*pool = Some(PoolInfo {
				pool_status: PoolStatus::Matured,
				..pool.clone().unwrap()
			});
		});
	}: _(RawOrigin::Signed(user.clone()), id)
	verify {
		// User reward debt should have increased
		let user_info = PoolUsers::<T>::get(id, user).unwrap();
		assert_eq!(user_info.amount, 10u32.into());
		assert_eq!(user_info.reward_debt, 10u32.into());
	}

	// Unsigned rollover transaction
	rollover_unsigned {
		let remaining_weight: Weight =
			T::DbWeight::get().reads(100u64).saturating_add(T::DbWeight::get().writes(100u64));
		let alith = account::<T>("Alith");
		let asset_id = mint_asset::<T>(&alith, &"TEST");

		let id = NextPoolId::<T>::get();

		let vault_account = LiquidityPools::<T>::account_id();
		assert_ok!(T::Assets::mint_into(asset_id.into(), &vault_account, 10_000_000u32.into()));
		assert_ok!(T::Currency::transfer(&alith, &vault_account, 10_000_000u32.into(), ExistenceRequirement::AllowDeath));

		let user = account::<T>("user");
		assert_ok!(T::Assets::mint_into(asset_id.into(), &user, 10u32.into()));
		// Insert test pool user
		let interest_rate = 1_000_000;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();
		LiquidityPools::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block).unwrap();

		let successor_id = NextPoolId::<T>::get();
		let start_block = 51u32.into();
		let end_block = 60u32.into();
		LiquidityPools::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block).unwrap();
		LiquidityPools::<T>::set_pool_succession(RawOrigin::Root.into(), id, successor_id).unwrap();

		Pools::<T>::mutate(id, |pool| {
			*pool = Some(PoolInfo {
				pool_status: PoolStatus::Open,
				..pool.clone().unwrap()
			});
		});

		LiquidityPools::<T>::join_pool(RawOrigin::Signed(user.clone()).into(), id, 10u32.into()).unwrap();

		Pools::<T>::mutate(id, |pool| {
			*pool = Some(PoolInfo {
				pool_status: PoolStatus::Matured,
				..pool.clone().unwrap()
			});
		});
	}:_(RawOrigin::None, id, end_block)
}

impl_benchmark_test_suite!(
	LiquidityPools,
	crate::mock::TestExt::default()
		.with_assets(&[(1, crate::mock::create_account(1), 100),])
		.build(),
	crate::mock::Test
);
