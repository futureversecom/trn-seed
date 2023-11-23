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

use crate::Pallet as Incentive;
use frame_benchmarking::{account as bench_account, *};
use frame_support::{
	assert_ok,
	traits::{fungibles::Mutate, Currency, ExistenceRequirement},
};
use frame_system::{Pallet as System, RawOrigin};
use seed_pallet_common::CreateExt;
use sp_core::H160;

// Helper function to get test account
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

fn mint_asset<T: Config>(acc: &T::AccountId) -> T::AssetId
where
	<T as frame_system::Config>::AccountId: From<H160>,
{
	// mint native token
	assert_ok!(T::Assets::mint_into(1u32.into(), &acc, 10_000_000u32.into()));

	let asset_id = T::Assets::create(&acc, None).unwrap();
	assert_ok!(T::Assets::mint_into(asset_id.into(), &acc, 10_000_000u32.into()));

	asset_id.into()
}

benchmarks! {
	create_pool {
		let asset_id = T::AssetId::default();
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
		let asset_id = T::AssetId::default();
		let interest_rate = 10;
		let max_tokens = 1000u32.into();
		let start_block = 1000u32.into();
		let end_block = 2000u32.into();
		let id = NextPoolId::<T>::get();
		Incentive::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block).unwrap();
	}: _(RawOrigin::Root, id)
	verify {
		let next_pool_id = T::PoolId::default();
		assert_eq!(Pools::<T>::get(id).is_none(), true);
	}

	set_incentive_pool_succession {
		// Insert test pools
		let asset_id = T::AssetId::default();
		let interest_rate = 2;
		let max_tokens = 3u32.into();
		let start_block = 4u32.into();
		let end_block = 5u32.into();

		let predecessor_id = NextPoolId::<T>::get();
		Incentive::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block).unwrap();

		let successor_id = NextPoolId::<T>::get();
		Incentive::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block).unwrap();
	}: _(RawOrigin::Root, predecessor_id, successor_id)
	verify {
		assert_eq!(PoolRelationships::<T>::get(predecessor_id).unwrap().successor_id, Some(successor_id));
	}

	// Update user rollover preference
	set_pool_rollover {
		let alith = account::<T>("Alith");
		let asset_id = mint_asset::<T>(&alith);

		let id = NextPoolId::<T>::get();

		let vault_account = Incentive::<T>::get_vault_account(id).unwrap();
		assert_ok!(T::Assets::mint_into(asset_id.into(), &vault_account, 10_000_000u32.into()));

		let user = account::<T>("user");
		assert_ok!(T::Assets::mint_into(asset_id.into(), &user, 10u32.into()));
		// Insert test pool user
		let interest_rate = 2;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();
		Incentive::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block).unwrap();
		System::<T>::set_block_number(start_block);
		Incentive::<T>::on_initialize(start_block);
		Incentive::<T>::join_pool(RawOrigin::Signed(user.clone()).into(), id, 10u32.into()).unwrap();
	}: _(RawOrigin::Signed(user), id, true)
	verify {
		assert_eq!(PoolUsers::<T>::get(id, user).unwrap().should_rollover, true);
	}

	// Join reward pool
	join_pool {
		let amount = 10u32.into();
		let alith = account::<T>("Alith");
		let asset_id = mint_asset::<T>(&alith);

		let id = NextPoolId::<T>::get();

		let vault_account = Incentive::<T>::get_vault_account(id).unwrap();
		assert_ok!(T::Assets::mint_into(asset_id.into(), &vault_account, 10_000_000u32.into()));

		let user = account::<T>("user");
		assert_ok!(T::Assets::mint_into(asset_id.into(), &user, 10u32.into()));
		// Insert test pool user
		let interest_rate = 2;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();
		Incentive::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block).unwrap();
		System::<T>::set_block_number(start_block);
		Incentive::<T>::on_initialize(start_block);
	}: _(RawOrigin::Signed(user.clone()), id, amount)
	verify {
		assert_eq!(PoolUsers::<T>::get(id, user).unwrap().amount, amount);
	}

	// Exit reward pool
	exit_pool {
		let alith = account::<T>("Alith");
		let asset_id = mint_asset::<T>(&alith);

		let id = NextPoolId::<T>::get();

		let vault_account = Incentive::<T>::get_vault_account(id).unwrap();
		assert_ok!(T::Assets::mint_into(asset_id.into(), &vault_account, 10_000_000u32.into()));

		let user = account::<T>("user");
		assert_ok!(T::Assets::mint_into(asset_id.into(), &user, 10u32.into()));
		// Insert test pool user
		let interest_rate = 2;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();
		Incentive::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block).unwrap();
		System::<T>::set_block_number(start_block);
		Incentive::<T>::on_initialize(start_block);
		Incentive::<T>::join_pool(RawOrigin::Signed(user.clone()).into(), id, 10u32.into()).unwrap();
	}: _(RawOrigin::Signed(user.clone()), id)
	verify {
		assert!(PoolUsers::<T>::get(id, user).is_none());
	}

	// Claim reward
	claim_reward {
		let alith = account::<T>("Alith");
		let asset_id = mint_asset::<T>(&alith);

		let id = NextPoolId::<T>::get();

		let vault_account = Incentive::<T>::get_vault_account(id).unwrap();
		assert_ok!(T::Assets::mint_into(asset_id.into(), &vault_account, 10_000_000u32.into()));
		assert_ok!(T::Currency::transfer(&alith, &vault_account, 10_000_000u32.into(), ExistenceRequirement::AllowDeath));

		let user = account::<T>("user");
		assert_ok!(T::Assets::mint_into(asset_id.into(), &user, 10u32.into()));
		// Insert test pool user
		let interest_rate = 1;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();
		Incentive::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block).unwrap();
		System::<T>::set_block_number(start_block);
		Incentive::<T>::on_initialize(start_block);
		Incentive::<T>::join_pool(RawOrigin::Signed(user.clone()).into(), id, 10u32.into()).unwrap();

		for i in 0..50+50 as u32 {
			System::<T>::set_block_number(i.into());
			Incentive::<T>::on_initialize(i.into());
		}
	}: _(RawOrigin::Signed(user.clone()), id)
	verify {
		// User reward debt should have increased
		let user_info = PoolUsers::<T>::get(id, user).unwrap();
		assert_eq!(user_info.reward_debt, 10u32.into());
	}

	// Unsigned rollover transaction
	rollover_unsigned {
		let alith = account::<T>("Alith");
		let asset_id = mint_asset::<T>(&alith);

		let id = NextPoolId::<T>::get();

		let vault_account = Incentive::<T>::get_vault_account(id).unwrap();
		assert_ok!(T::Assets::mint_into(asset_id.into(), &vault_account, 10_000_000u32.into()));
		assert_ok!(T::Currency::transfer(&alith, &vault_account, 10_000_000u32.into(), ExistenceRequirement::AllowDeath));

		let user = account::<T>("user");
		assert_ok!(T::Assets::mint_into(asset_id.into(), &user, 10u32.into()));
		// Insert test pool user
		let interest_rate = 1;
		let max_tokens = 100u32.into();
		let start_block = 10u32.into();
		let end_block = 50u32.into();
		Incentive::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block).unwrap();

		let successor_id = NextPoolId::<T>::get();
		Incentive::<T>::create_pool(RawOrigin::Root.into(), asset_id, interest_rate, max_tokens, start_block, end_block).unwrap();
		Incentive::<T>::set_incentive_pool_succession(RawOrigin::Root.into(), id, successor_id).unwrap();

		System::<T>::set_block_number(start_block);
		Incentive::<T>::on_initialize(start_block);
		Incentive::<T>::join_pool(RawOrigin::Signed(user.clone()).into(), id, 10u32.into()).unwrap();

		System::<T>::set_block_number(end_block);
		Incentive::<T>::on_initialize(end_block);
	}:_(RawOrigin::None, id, end_block)
}

impl_benchmark_test_suite!(Incentive, crate::mock::TestExt::default().build(), crate::mock::Test);
