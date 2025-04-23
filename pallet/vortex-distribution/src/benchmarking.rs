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

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use crate::Pallet as Vortex;
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, BoundedVec};
use frame_system::{Pallet as System, RawOrigin};
use sp_runtime::{traits::One, Perbill};

use crate::Pallet as VortexDistribution;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

fn mint_asset<T: Config>() -> AssetId {
	let asset_account = account::<T>("asset_vault");
	let asset_id = T::MultiCurrency::create(&asset_account, None).unwrap();
	let mint_amount = Balance::one();
	assert_ok!(T::MultiCurrency::mint_into(asset_id, &asset_account, mint_amount.into()));
	assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get(), &asset_account, mint_amount));
	assert_ok!(T::MultiCurrency::mint_into(T::VtxAssetId::get(), &asset_account, mint_amount));
	asset_id
}

benchmarks! {
	where_clause { where <T as frame_system::Config>::AccountId: From<sp_core::H160> }
	set_admin {
		let new_account = account::<T>("Admin");
		let account_lookup = T::Lookup::unlookup(new_account.clone());
		let vortex_dist_id = NextVortexId::<T>::get();
	}: _(RawOrigin::Root, account_lookup)
	verify {
		assert_eq!(AdminAccount::<T>::get().unwrap(), new_account);
	}

	create_vtx_dist {
		let vortex_dist_id = NextVortexId::<T>::get();
	}: _(RawOrigin::Root)
	verify {
		assert_eq!(VtxDistStatuses::<T>::get(vortex_dist_id), VtxDistStatus::Enabled);
	}

	disable_vtx_dist {
		let vortex_dist_id = NextVortexId::<T>::get();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
	}: _(RawOrigin::Root, vortex_dist_id)
	verify {
		assert_eq!(VtxDistStatuses::<T>::get(vortex_dist_id), VtxDistStatus::Disabled);
	}

	set_vtx_total_supply {
		let vortex_dist_id = NextVortexId::<T>::get();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
		let vtx_total_supply = Balance::one();
	}: _(RawOrigin::Root, vortex_dist_id, vtx_total_supply)
	verify {
		assert_eq!(VtxTotalSupply::<T>::get(vortex_dist_id), vtx_total_supply);
	}

	set_consider_current_balance {
		let vortex_dist_id = NextVortexId::<T>::get();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
	}: _(RawOrigin::Root, true)
	verify {
		assert_eq!(ConsiderCurrentBalance::<T>::get(), true);
	}

	set_disable_redeem {
		let vortex_dist_id = NextVortexId::<T>::get();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
	}: _(RawOrigin::Root, true)
	verify {
		assert_eq!(DisableRedeem::<T>::get(), true);
	}

	start_vtx_dist {
		System::<T>::set_block_number(1_u32.into());
		let vortex_dist_id = NextVortexId::<T>::get();
		let root_price = Balance::one().saturating_mul(10_u128.pow(6));
		let vortex_price = Balance::one().saturating_mul(10_u128.pow(6));
		let root_vault = account::<T>("root_vault");
		let fee_vault = account::<T>("fee_vault");
		let asset_id = mint_asset::<T>();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
		let balance = Balance::one().saturating_mul(10).saturating_mul(10_u128.pow(6));
		let asset_balances = BoundedVec::try_from(vec![(asset_id, balance), (T::NativeAssetId::get(), balance.into())]).unwrap();
		assert_ok!(VortexDistribution::<T>::set_vtx_vault_asset_balances(RawOrigin::Root.into(), vortex_dist_id.clone(), asset_balances.clone()));
		assert_ok!(VortexDistribution::<T>::set_fee_pot_asset_balances(RawOrigin::Root.into(), vortex_dist_id.clone(), asset_balances));
		let asset_price = Balance::one().saturating_mul(10_u128.pow(6));
		let asset_prices = BoundedVec::try_from(vec![(asset_id, asset_price), (T::NativeAssetId::get(), asset_price)]).unwrap();
		assert_ok!(VortexDistribution::<T>::set_asset_prices(RawOrigin::Root.into(), vortex_dist_id, asset_prices));

		let vtx_vault = VortexDistribution::<T>::get_vtx_vault_account();
		let root_vault = VortexDistribution::<T>::get_root_vault_account();
		let fee_vault = VortexDistribution::<T>::get_fee_vault_account();

		let mint_amount = Balance::one().saturating_mul(10).saturating_mul(10_u128.pow(6));
		assert_ok!(T::MultiCurrency::mint_into(asset_id, &fee_vault, mint_amount.into()));
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get(), &fee_vault, mint_amount));
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get(), &root_vault, mint_amount));
		assert_ok!(T::MultiCurrency::mint_into(T::VtxAssetId::get(), &vtx_vault, mint_amount));

		// set currrent vtx supply
		assert_ok!(VortexDistribution::<T>::set_vtx_total_supply(RawOrigin::Root.into(), vortex_dist_id.clone(), mint_amount));

		let reward_points = BoundedVec::try_from(vec![(account::<T>("test"), 100u32.into())]).unwrap();
		let work_points = BoundedVec::try_from(vec![(account::<T>("test"), 10u32.into())]).unwrap();
		assert_ok!(VortexDistribution::<T>::register_reward_points(
				RawOrigin::Root.into(),
				vortex_dist_id,
				reward_points
			));
		assert_ok!(VortexDistribution::<T>::register_work_points(RawOrigin::Root.into(), vortex_dist_id, work_points));

		assert_ok!(VortexDistribution::<T>::trigger_vtx_distribution(RawOrigin::Root.into(), vortex_dist_id));
		// run a few blocks to move the Vtx status from Triggering to Triggered
		for i in 2_u32..10 {
			System::<T>::set_block_number(i.into());
			VortexDistribution::<T>::on_idle(i.into(), Weight::from_all(1_000_000_000_000_u64));
		}
	}: _(RawOrigin::Root, vortex_dist_id)
	verify {
		assert_eq!(VtxDistStatuses::<T>::get(vortex_dist_id), VtxDistStatus::Paying);
	}

	set_fee_pot_asset_balances {
		let b in 1..500;

		let balance =  Balance::one();
		let mut asset_balances_vec = vec![];
		for _ in 0..b {
			let asset_id = mint_asset::<T>();
			asset_balances_vec.push((asset_id, balance));
		}
		let asset_balances = BoundedVec::try_from(asset_balances_vec).unwrap();
		let vortex_dist_id = NextVortexId::<T>::get();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
	}: _(RawOrigin::Root, vortex_dist_id, asset_balances.clone())
	verify {
		assert_eq!(FeePotAssetsList::<T>::get(vortex_dist_id), asset_balances);
	}

	set_vtx_vault_asset_balances {
		let b in 1..500;

		let balance =  Balance::one();
		let mut asset_balances_vec = vec![];
		for _ in 0..b {
			let asset_id = mint_asset::<T>();
			asset_balances_vec.push((asset_id, balance));
		}
		let asset_balances = BoundedVec::try_from(asset_balances_vec).unwrap();
		let vortex_dist_id = NextVortexId::<T>::get();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
	}: _(RawOrigin::Root, vortex_dist_id, asset_balances.clone())
	verify {
		assert_eq!(VtxVaultAssetsList::<T>::get(vortex_dist_id), asset_balances);
	}

	set_asset_prices {
		let b in 1..500;

		let balance =  Balance::one();
		let mut asset_prices_vec = vec![];
		let mut asset_balances_vec = vec![];
		for i in 0..b {
			let asset_id = mint_asset::<T>();
			asset_prices_vec.push((asset_id, balance.into()));
			asset_balances_vec.push((asset_id, balance.into()));
		}

		let asset_prices = BoundedVec::try_from(asset_prices_vec).unwrap();
		let asset_balances = BoundedVec::try_from(asset_balances_vec).unwrap();
		let vortex_dist_id = NextVortexId::<T>::get();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
		assert_ok!(VortexDistribution::<T>::set_fee_pot_asset_balances(RawOrigin::Root.into(), vortex_dist_id.clone(), asset_balances));
	}: _(RawOrigin::Root, vortex_dist_id, asset_prices.clone())
	verify {
		for (asset_id, _) in asset_prices.into_iter() {
			assert_eq!(AssetPrices::<T>::get(vortex_dist_id, asset_id), balance);
		}
	}

	register_reward_points {
		let b in 1..500;

		let balance =  Balance::one();
		let mut reward_points_vec = vec![];
		for i in 0..b {
			let account: T::AccountId = bench_account("account", i, 0);
			reward_points_vec.push((account, balance.into()));
		}

		let reward_points = BoundedVec::try_from(reward_points_vec).unwrap();
		let vortex_dist_id = NextVortexId::<T>::get();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
	}: _(RawOrigin::Root, vortex_dist_id, reward_points)
	verify {
		for i in 0..b {
			let account: T::AccountId = bench_account("account", i, 0);
			assert_eq!(RewardPoints::<T>::get(vortex_dist_id, account), balance);
		}
	}

	register_work_points {
		let b in 1..500;

		let balance =  Balance::one();
		let mut work_points_vec = vec![];
		for i in 0..b {
			let account: T::AccountId = bench_account("account", i, 0);
			work_points_vec.push((account, balance.into()));
		}

		let work_points = BoundedVec::try_from(work_points_vec).unwrap();
		let vortex_dist_id = NextVortexId::<T>::get();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
	}: _(RawOrigin::Root, vortex_dist_id, work_points)
	verify {
		for i in 0..b {
			let account: T::AccountId = bench_account("account", i, 0);
			assert_eq!(WorkPoints::<T>::get(vortex_dist_id, account), balance);
		}
	}

	trigger_vtx_distribution {
		let vortex_dist_id = NextVortexId::<T>::get();
		let root_price = Balance::one().saturating_mul(10_u128.pow(6));
		let vortex_price = Balance::one().saturating_mul(10_u128.pow(6));
		let root_vault = account::<T>("root_vault");
		let fee_vault = account::<T>("fee_vault");
		let asset_id = mint_asset::<T>();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
		let balance = Balance::one().saturating_mul(10).saturating_mul(10_u128.pow(6));
		let asset_balances = BoundedVec::try_from(vec![(asset_id, balance), (T::NativeAssetId::get(), balance.into())]).unwrap();
		assert_ok!(VortexDistribution::<T>::set_vtx_vault_asset_balances(RawOrigin::Root.into(), vortex_dist_id.clone(), asset_balances.clone()));
		assert_ok!(VortexDistribution::<T>::set_fee_pot_asset_balances(RawOrigin::Root.into(), vortex_dist_id.clone(), asset_balances));
		let asset_price = Balance::one().saturating_mul(10_u128.pow(6));
		let asset_prices = BoundedVec::try_from(vec![(asset_id, asset_price), (T::NativeAssetId::get(), asset_price)]).unwrap();
		assert_ok!(VortexDistribution::<T>::set_asset_prices(RawOrigin::Root.into(), vortex_dist_id, asset_prices));

		let vtx_vault = VortexDistribution::<T>::get_vtx_vault_account();
		let root_vault = VortexDistribution::<T>::get_root_vault_account();
		let fee_vault = VortexDistribution::<T>::get_fee_vault_account();

		let mint_amount = Balance::one().saturating_mul(10).saturating_mul(10_u128.pow(6));
		assert_ok!(T::MultiCurrency::mint_into(asset_id, &fee_vault, mint_amount.into()));
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get(), &fee_vault, mint_amount));
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get(), &root_vault, mint_amount));
		assert_ok!(T::MultiCurrency::mint_into(T::VtxAssetId::get(), &vtx_vault, mint_amount));

		// set currrent vtx supply
		assert_ok!(VortexDistribution::<T>::set_vtx_total_supply(RawOrigin::Root.into(), vortex_dist_id.clone(), mint_amount));

		let reward_points = BoundedVec::try_from(vec![(account::<T>("test"), 100u32.into())]).unwrap();
		let work_points = BoundedVec::try_from(vec![(account::<T>("test"), 10u32.into())]).unwrap();
		assert_ok!(VortexDistribution::<T>::register_reward_points(
				RawOrigin::Root.into(),
				vortex_dist_id,
				reward_points
			));
		assert_ok!(VortexDistribution::<T>::register_work_points(RawOrigin::Root.into(), vortex_dist_id, work_points));

	}: _(RawOrigin::Root, vortex_dist_id)
	verify {
		assert_eq!(VtxDistStatuses::<T>::get(vortex_dist_id), VtxDistStatus::Triggering);
	}

	redeem_tokens_from_vault {
		let vortex_dist_id = NextVortexId::<T>::get();
		let root_price = Balance::one().saturating_mul(10_u128.pow(6));
		let vortex_price = Balance::one().saturating_mul(10_u128.pow(6));
		let root_vault = account::<T>("root_vault");
		let fee_vault = account::<T>("fee_vault");
		let asset_id = mint_asset::<T>();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
		let balance = Balance::one().saturating_mul(10).saturating_mul(10_u128.pow(6));
		let asset_balances = BoundedVec::try_from(vec![(asset_id, balance), (T::NativeAssetId::get(), balance.into())]).unwrap();
		assert_ok!(VortexDistribution::<T>::set_vtx_vault_asset_balances(RawOrigin::Root.into(), vortex_dist_id.clone(), asset_balances.clone()));
		assert_ok!(VortexDistribution::<T>::set_fee_pot_asset_balances(RawOrigin::Root.into(), vortex_dist_id.clone(), asset_balances));
		let asset_price = Balance::one().saturating_mul(10_u128.pow(6));
		let asset_prices = BoundedVec::try_from(vec![(asset_id, asset_price), (T::NativeAssetId::get(), asset_price)]).unwrap();
		assert_ok!(VortexDistribution::<T>::set_asset_prices(RawOrigin::Root.into(), vortex_dist_id, asset_prices));

		let vtx_vault = VortexDistribution::<T>::get_vtx_vault_account();
		let root_vault = VortexDistribution::<T>::get_root_vault_account();
		let fee_vault = VortexDistribution::<T>::get_fee_vault_account();

		let mint_amount = Balance::one().saturating_mul(10).saturating_mul(10_u128.pow(6));
		assert_ok!(T::MultiCurrency::mint_into(asset_id, &fee_vault, mint_amount.into()));
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get(), &fee_vault, mint_amount));
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get(), &root_vault, mint_amount));
		assert_ok!(T::MultiCurrency::mint_into(T::VtxAssetId::get(), &vtx_vault, mint_amount));

		// set currrent vtx supply
		assert_ok!(VortexDistribution::<T>::set_vtx_total_supply(RawOrigin::Root.into(), vortex_dist_id.clone(), mint_amount));

		let reward_points = BoundedVec::try_from(vec![(account::<T>("test"), 100u32.into())]).unwrap();
		let work_points = BoundedVec::try_from(vec![(account::<T>("test"), 10u32.into())]).unwrap();
		assert_ok!(VortexDistribution::<T>::register_reward_points(
				RawOrigin::Root.into(),
				vortex_dist_id,
				reward_points
			));
		assert_ok!(VortexDistribution::<T>::register_work_points(RawOrigin::Root.into(), vortex_dist_id, work_points));
		assert_ok!(VortexDistribution::<T>::trigger_vtx_distribution(RawOrigin::Root.into(), vortex_dist_id));
		let vtx_redeem_asset_list =
				BoundedVec::try_from(vec![asset_id, T::NativeAssetId::get()]).unwrap();
		assert_ok!(VortexDistribution::<T>::set_vtx_vault_redeem_asset_list(
			RawOrigin::Root.into(),
			vtx_redeem_asset_list.clone()
		));
		// run a few blocks to move the Vtx status from Triggering to Triggered
		for i in 2_u32..4 {
			System::<T>::set_block_number(i.into());
			VortexDistribution::<T>::on_idle(i.into(), Weight::from_all(1_000_000_000_000_u64));
		}
		assert_ok!(VortexDistribution::<T>::start_vtx_dist(RawOrigin::Root.into(), vortex_dist_id));
		let end_block: u32 = 500;
		System::<T>::set_block_number(end_block.into());
		assert_ok!(VortexDistribution::<T>::pay_unsigned(RawOrigin::None.into(), vortex_dist_id, end_block.into()));
		VtxDistStatuses::<T>::mutate(vortex_dist_id, |status| {
			*status = VtxDistStatus::Done;
		});
		let vtx_balance = T::MultiCurrency::balance(T::VtxAssetId::get(), &account::<T>("test"));
		assert_ne!(vtx_balance, Balance::zero());
	}: _(RawOrigin::Signed(account::<T>("test")), vtx_balance)
	verify {
		// assert_eq!(T::MultiCurrency::balance(T::VtxAssetId::get(), &account::<T>("test")), 0);
		// let ratio = Perbill::from_rational(vtx_balance, TotalVortex::<T>::get(vortex_dist_id));
		// let mint_amount = Balance::one();
		// let expect_balance = ratio * mint_amount;
		// assert_eq!(T::MultiCurrency::balance(asset_id, &account::<T>("test")), expect_balance);
		// assert_eq!(T::MultiCurrency::balance(T::NativeAssetId::get(), &account::<T>("test")), expect_balance);
	}

	pay_unsigned {
		let vortex_dist_id = NextVortexId::<T>::get();
		let root_price = Balance::one().saturating_mul(10_u128.pow(6));
		let vortex_price = Balance::one().saturating_mul(10_u128.pow(6));
		let root_vault = account::<T>("root_vault");
		let fee_vault = account::<T>("fee_vault");
		let asset_id = mint_asset::<T>();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
		let balance = Balance::one().saturating_mul(10).saturating_mul(10_u128.pow(6));
		let asset_balances = BoundedVec::try_from(vec![(asset_id, balance), (T::NativeAssetId::get(), balance.into())]).unwrap();
		assert_ok!(VortexDistribution::<T>::set_vtx_vault_asset_balances(RawOrigin::Root.into(), vortex_dist_id.clone(), asset_balances.clone()));
		assert_ok!(VortexDistribution::<T>::set_fee_pot_asset_balances(RawOrigin::Root.into(), vortex_dist_id.clone(), asset_balances));
		let asset_price = Balance::one().saturating_mul(10_u128.pow(6));
		let asset_prices = BoundedVec::try_from(vec![(asset_id, asset_price), (T::NativeAssetId::get(), asset_price)]).unwrap();
		assert_ok!(VortexDistribution::<T>::set_asset_prices(RawOrigin::Root.into(), vortex_dist_id, asset_prices));

		let vtx_vault = VortexDistribution::<T>::get_vtx_vault_account();
		let root_vault = VortexDistribution::<T>::get_root_vault_account();
		let fee_vault = VortexDistribution::<T>::get_fee_vault_account();

		let mint_amount = Balance::one().saturating_mul(10).saturating_mul(10_u128.pow(6));
		assert_ok!(T::MultiCurrency::mint_into(asset_id, &fee_vault, mint_amount.into()));
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get(), &fee_vault, mint_amount));
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get(), &root_vault, mint_amount));
		assert_ok!(T::MultiCurrency::mint_into(T::VtxAssetId::get(), &vtx_vault, mint_amount));

		// set currrent vtx supply
		assert_ok!(VortexDistribution::<T>::set_vtx_total_supply(RawOrigin::Root.into(), vortex_dist_id.clone(), mint_amount));

		let reward_points = BoundedVec::try_from(vec![(account::<T>("test"), 100u32.into())]).unwrap();
		let work_points = BoundedVec::try_from(vec![(account::<T>("test"), 10u32.into())]).unwrap();
		assert_ok!(VortexDistribution::<T>::register_reward_points(
				RawOrigin::Root.into(),
				vortex_dist_id,
				reward_points
			));
		assert_ok!(VortexDistribution::<T>::register_work_points(RawOrigin::Root.into(), vortex_dist_id, work_points));
		assert_ok!(VortexDistribution::<T>::trigger_vtx_distribution(RawOrigin::Root.into(), vortex_dist_id));
		// run a few blocks to move the Vtx status from Triggering to Triggered
		for i in 2_u32..4 {
			System::<T>::set_block_number(i.into());
			VortexDistribution::<T>::on_idle(i.into(), Weight::from_all(1_000_000_000_000_u64));
		}
		assert_ok!(VortexDistribution::<T>::start_vtx_dist(RawOrigin::Root.into(), vortex_dist_id));
		let end_block: u32 = 500;
		System::<T>::set_block_number(end_block.into());
		assert_eq!(T::MultiCurrency::balance(T::VtxAssetId::get(), &account::<T>("test")), 0u32.into());
	}: _(RawOrigin::None, vortex_dist_id, end_block.into())
	verify {
		assert_eq!(T::MultiCurrency::balance(T::VtxAssetId::get(), &account::<T>("test")), 28u32.into());
	}

	set_vtx_vault_redeem_asset_list {
		let b in 1..500;

		let mut vtx_vault_redeem_asset_list_vec = vec![];
		for i in 0..b {
			vtx_vault_redeem_asset_list_vec.push(i);
		}

		let vtx_vault_redeem_asset_list = BoundedVec::try_from(vtx_vault_redeem_asset_list_vec).unwrap();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
	}: _(RawOrigin::Root, vtx_vault_redeem_asset_list.clone())
	verify {
			assert_eq!(VtxVaultRedeemAssetList::<T>::get(), vtx_vault_redeem_asset_list);
	}

	register_rewards {
		let b in 1..500;

		let balance =  Balance::one();
		let mut rewards_vec = vec![];
		for i in 0..b {
			let account: T::AccountId = bench_account("account", i, 0);
			rewards_vec.push((account, balance.into()));
		}

		let rewards = BoundedVec::try_from(rewards_vec).unwrap();
		let vortex_dist_id = NextVortexId::<T>::get();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
		assert_ok!(VortexDistribution::<T>::set_enable_manual_reward_input(RawOrigin::Root.into(), true));
	}: _(RawOrigin::Root, vortex_dist_id, rewards)
	verify {
		let first_account: T::AccountId = bench_account("account", 0, 0);
		let reward = VtxDistOrderbook::<T>::get(vortex_dist_id, first_account);
		assert_eq!(reward, (balance, false));
	}

	set_enable_manual_reward_input {
		let vortex_dist_id = NextVortexId::<T>::get();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
	}: _(RawOrigin::Root, true)
	verify {
		assert_eq!(EnableManualRewardInput::<T>::get(), true);
	}

}
impl_benchmark_test_suite!(
	VortexDistribution,
	crate::mock::TestExt::benchmark().build(),
	crate::mock::Test,
);
