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
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, traits::fungibles::Mutate};
use frame_system::RawOrigin;
use seed_primitives::AssetId;

/// Global incrementing ID for generating unique accounts
static mut ACCOUNT_ID: u32 = 0;

/// Helper function to get a unique account by incrementing a global counter
pub fn account<T: Config>() -> T::AccountId {
	unsafe {
		let id = ACCOUNT_ID;
		ACCOUNT_ID += 1;
		bench_account("", id, 0)
	}
}

fn mint_asset<T: Config>() -> AssetId {
	T::MultiCurrency::create(&bench_account("", 0, 0), None).unwrap()
}

benchmarks! {

	create_pool {
		let reward_asset_id = mint_asset::<T>();
		let staked_asset_id = mint_asset::<T>();

		let creator = account::<T>();
		assert_ok!(T::MultiCurrency::mint_into(reward_asset_id, &creator, 100_000_000u32.into()));

		let next_pool_id = NextPoolId::<T>::get();
		let interest_rate = 1_000_000;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();
	}: _(RawOrigin::Signed(creator), reward_asset_id, staked_asset_id, interest_rate, max_tokens, start_block, end_block)
	verify {
		assert!(Pools::<T>::get(next_pool_id).is_some());
	}

	close_pool {
		let reward_asset_id = mint_asset::<T>();
		let staked_asset_id = mint_asset::<T>();

		let creator = account::<T>();
		assert_ok!(T::MultiCurrency::mint_into(reward_asset_id, &creator, 100_000_000u32.into()));

		let next_pool_id = NextPoolId::<T>::get();

		let interest_rate = 1_000_000;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();
		assert_ok!(LiquidityPools::<T>::create_pool(RawOrigin::Signed(creator).into(), reward_asset_id, staked_asset_id, interest_rate, max_tokens, start_block, end_block));

		// create pool user; enter pool as a user
		let user = account::<T>();
		assert_ok!(T::MultiCurrency::mint_into(staked_asset_id.into(), &user, 10u32.into()));

		// Open pool
		Pools::<T>::mutate(next_pool_id, |pool| {
			*pool = Some(PoolInfo {
				pool_status: PoolStatus::Open,
				..pool.clone().unwrap()
			});
		});

		// Enter pool
		assert_ok!(LiquidityPools::<T>::enter_pool(RawOrigin::Signed(user.clone()).into(), next_pool_id, 10u32.into()));
	}: _(RawOrigin::Signed(creator), next_pool_id)
	verify {
		assert_eq!(Pools::<T>::get(next_pool_id).unwrap().pool_status, PoolStatus::Closing);
	}

	set_pool_succession {
		let reward_asset_id = mint_asset::<T>();
		let staked_asset_id = mint_asset::<T>();

		let creator = account::<T>();
		assert_ok!(T::MultiCurrency::mint_into(reward_asset_id, &creator, 300_000_000u32.into()));

		// Insert test pools
		let interest_rate = 1_000_000;
		let max_tokens = 100u32.into();
		let start_block = 4u32.into();
		let end_block = 5u32.into();

		let predecessor_id = NextPoolId::<T>::get();
		assert_ok!(LiquidityPools::<T>::create_pool(RawOrigin::Signed(creator).into(), reward_asset_id, staked_asset_id, interest_rate, max_tokens, start_block, end_block));

		let successor_id = NextPoolId::<T>::get();
		let start_block = 6u32.into();
		let end_block = 7u32.into();
		assert_ok!(LiquidityPools::<T>::create_pool(RawOrigin::Signed(creator).into(), reward_asset_id, staked_asset_id, interest_rate, max_tokens, start_block, end_block));
	}: _(RawOrigin::Signed(creator), predecessor_id, successor_id)
	verify {
		assert_eq!(PoolRelationships::<T>::get(predecessor_id).unwrap().successor_id, Some(successor_id));
	}

	// Update user rollover preference
	set_pool_rollover {
		let reward_asset_id = mint_asset::<T>();
		let staked_asset_id = mint_asset::<T>();

		let creator = account::<T>();
		assert_ok!(T::MultiCurrency::mint_into(reward_asset_id, &creator, 100_000_000u32.into()));

		// Mint asset to user
		let user = account::<T>();
		assert_ok!(T::MultiCurrency::mint_into(staked_asset_id.into(), &user, 10u32.into()));

		// Insert test pool user
		let id = NextPoolId::<T>::get();
		let interest_rate = 1_000_000;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();
		assert_ok!(LiquidityPools::<T>::create_pool(RawOrigin::Signed(creator).into(), reward_asset_id, staked_asset_id, interest_rate, max_tokens, start_block, end_block));

		// Open pool
		Pools::<T>::mutate(id, |pool| {
			*pool = Some(PoolInfo {
				pool_status: PoolStatus::Open,
				..pool.clone().unwrap()
			});
		});

		// Enter pool
		assert_ok!(LiquidityPools::<T>::enter_pool(RawOrigin::Signed(user.clone()).into(), id, 10u32.into()));
	}: _(RawOrigin::Signed(user), id, true)
	verify {
		assert!(PoolUsers::<T>::get(id, user).unwrap().should_rollover);
	}

	enter_pool {
		let reward_asset_id = mint_asset::<T>();
		let staked_asset_id = mint_asset::<T>();

		// Mint asset to creator
		let creator = account::<T>();
		assert_ok!(T::MultiCurrency::mint_into(reward_asset_id, &creator, 100_000_000u32.into()));

		// Mint asset to user
		let user = account::<T>();
		assert_ok!(T::MultiCurrency::mint_into(staked_asset_id.into(), &user, 10u32.into()));

		// Create pool
		let id = NextPoolId::<T>::get();
		let interest_rate = 1_000_000;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();
		assert_ok!(LiquidityPools::<T>::create_pool(RawOrigin::Signed(creator).into(), reward_asset_id, staked_asset_id, interest_rate, max_tokens, start_block, end_block));

		// Manually open pool
		Pools::<T>::mutate(id, |pool| {
			*pool = Some(PoolInfo {
				pool_status: PoolStatus::Open,
				..pool.clone().unwrap()
			});
		});

		let enter_amount = 10u32.into();
	}: _(RawOrigin::Signed(user.clone()), id, enter_amount)
	verify {
		assert_eq!(PoolUsers::<T>::get(id, user).unwrap().amount, enter_amount);
	}

	exit_pool {
		let reward_asset_id = mint_asset::<T>();
		let staked_asset_id = mint_asset::<T>();

		// Mint asset to creator
		let creator = account::<T>();
		assert_ok!(T::MultiCurrency::mint_into(reward_asset_id, &creator, 100_000_000u32.into()));

		// Mint asset to user
		let user = account::<T>();
		assert_ok!(T::MultiCurrency::mint_into(staked_asset_id.into(), &user, 10u32.into()));

		// Create pool
		let id = NextPoolId::<T>::get();
		let interest_rate = 1_000_000;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();
		assert_ok!(LiquidityPools::<T>::create_pool(RawOrigin::Signed(creator).into(), reward_asset_id, staked_asset_id, interest_rate, max_tokens, start_block, end_block));

		// Manually open pool
		Pools::<T>::mutate(id, |pool| {
			*pool = Some(PoolInfo {
				pool_status: PoolStatus::Open,
				..pool.clone().unwrap()
			});
		});

		// Enter pool
		assert_ok!(LiquidityPools::<T>::enter_pool(RawOrigin::Signed(user.clone()).into(), id, 10u32.into()));
	}: _(RawOrigin::Signed(user.clone()), id)
	verify {
		assert!(PoolUsers::<T>::get(id, user).is_none());
	}

	claim_reward {
		let reward_asset_id = mint_asset::<T>();
		let staked_asset_id = mint_asset::<T>();

		// Mint asset to creator
		let creator = account::<T>();
		assert_ok!(T::MultiCurrency::mint_into(reward_asset_id, &creator, 100_000_000u32.into()));

		// Mint asset to user
		let user = account::<T>();
		assert_ok!(T::MultiCurrency::mint_into(staked_asset_id.into(), &user, 10u32.into()));

		// Create pool
		let id = NextPoolId::<T>::get();
		let interest_rate = 1_000_000;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();
		assert_ok!(LiquidityPools::<T>::create_pool(RawOrigin::Signed(creator).into(), reward_asset_id, staked_asset_id, interest_rate, max_tokens, start_block, end_block));

		// Manually open pool
		Pools::<T>::mutate(id, |pool| {
			*pool = Some(PoolInfo {
				pool_status: PoolStatus::Open,
				..pool.clone().unwrap()
			});
		});

		// Enter pool
		assert_ok!(LiquidityPools::<T>::enter_pool(RawOrigin::Signed(user.clone()).into(), id, 10u32.into()));

		// Manually mature pool
		Pools::<T>::mutate(id, |pool| {
			*pool = Some(PoolInfo {
				pool_status: PoolStatus::Matured,
				..pool.clone().unwrap()
			});
		});
	}: _(RawOrigin::Signed(user.clone()), id)
	verify {
		assert_eq!(PoolUsers::<T>::get(id, user), None);
	}

	// Unsigned rollover transaction
	rollover_unsigned {
		let reward_asset_id = mint_asset::<T>();
		let staked_asset_id = mint_asset::<T>();

		let creator = account::<T>();
		assert_ok!(T::MultiCurrency::mint_into(reward_asset_id, &creator, 200_000_000u32.into()));

		let user = account::<T>();
		assert_ok!(T::MultiCurrency::mint_into(staked_asset_id.into(), &user, 10u32.into()));

		// Insert test pool user
		let id = NextPoolId::<T>::get();
		let interest_rate = 1_000_000;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();
		assert_ok!(LiquidityPools::<T>::create_pool(RawOrigin::Signed(creator).into(), reward_asset_id, staked_asset_id, interest_rate, max_tokens, start_block, end_block));

		let successor_id = NextPoolId::<T>::get();
		let start_block = 51u32.into();
		let end_block = 60u32.into();
		assert_ok!(LiquidityPools::<T>::create_pool(RawOrigin::Signed(creator).into(), reward_asset_id, staked_asset_id, interest_rate, max_tokens, start_block, end_block));
		assert_ok!(LiquidityPools::<T>::set_pool_succession(RawOrigin::Signed(creator).into(), id, successor_id));

		Pools::<T>::mutate(id, |pool| {
			*pool = Some(PoolInfo {
				pool_status: PoolStatus::Open,
				..pool.clone().unwrap()
			});
		});

		assert_ok!(LiquidityPools::<T>::enter_pool(RawOrigin::Signed(user.clone()).into(), id, 10u32.into()));

		Pools::<T>::mutate(id, |pool| {
			*pool = Some(PoolInfo {
				pool_status: PoolStatus::Matured,
				..pool.clone().unwrap()
			});
		});
	}:_(RawOrigin::None, id, end_block)

	emergency_recover_funds {
		let reward_asset_id = mint_asset::<T>();
		let staked_asset_id = mint_asset::<T>();

		let creator = account::<T>();
		assert_ok!(T::MultiCurrency::mint_into(reward_asset_id, &creator, 100_000_000u32.into()));

		let user = account::<T>();
		let stake_amount = 10u32.into();
		assert_ok!(T::MultiCurrency::mint_into(staked_asset_id.into(), &user, stake_amount));

		let pool_id = NextPoolId::<T>::get();
		let interest_rate = 1_000_000;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();

		assert_ok!(LiquidityPools::<T>::create_pool(
			RawOrigin::Signed(creator).into(),
			reward_asset_id,
			staked_asset_id,
			interest_rate,
			max_tokens,
			start_block,
			end_block
		));

		// Open pool and user enters
		Pools::<T>::mutate(pool_id, |pool| {
			*pool = Some(PoolInfo {
				pool_status: PoolStatus::Open,
				..pool.clone().unwrap()
			});
		});

		assert_ok!(LiquidityPools::<T>::enter_pool(
			RawOrigin::Signed(user.clone()).into(),
			pool_id,
			stake_amount
		));
	}: _(RawOrigin::Signed(user.clone()), pool_id)
	verify {
		assert!(PoolUsers::<T>::get(pool_id, user).is_none());
	}

	trigger_pool_update {
		let reward_asset_id = mint_asset::<T>();
		let staked_asset_id = mint_asset::<T>();

		let creator = account::<T>();
		assert_ok!(T::MultiCurrency::mint_into(reward_asset_id, &creator, 100_000_000u32.into()));

		let pool_id = NextPoolId::<T>::get();
		let interest_rate = 1_000_000;
		let max_tokens = 100u32.into();
		let start_block = 1u32.into(); // Past block to make eligible for urgent processing
		let end_block = 50u32.into();

		assert_ok!(LiquidityPools::<T>::create_pool(
			RawOrigin::Signed(creator.clone()).into(),
			reward_asset_id,
			staked_asset_id,
			interest_rate,
			max_tokens,
			start_block,
			end_block
		));

		// Set current block to make pool eligible for urgent processing
		frame_system::Pallet::<T>::set_block_number(10u32.into());

		let caller = account::<T>();
	}: _(RawOrigin::Signed(caller), pool_id)
	verify {
		assert!(UrgentPoolUpdates::<T>::contains_key(pool_id));
	}
}

impl_benchmark_test_suite!(
	LiquidityPools,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test,
);
