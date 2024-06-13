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
	let mint_amount = <T as pallet_staking::Config>::CurrencyBalance::one();
	assert_ok!(T::MultiCurrency::mint_into(asset_id, &asset_account, mint_amount.into()));
	assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get(), &asset_account, mint_amount));
	assert_ok!(T::MultiCurrency::mint_into(T::VtxAssetId::get(), &asset_account, mint_amount));
	asset_id
}

benchmarks! {
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

	start_vtx_dist {
		let vortex_dist_id = NextVortexId::<T>::get();
		let root_price = <T as pallet_staking::Config>::CurrencyBalance::one();
		let vortex_price = <T as pallet_staking::Config>::CurrencyBalance::one();
		let root_vault = account::<T>("root_vault");
		let fee_vault = account::<T>("fee_vault");
		let asset_id = mint_asset::<T>();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
		let start_era = 1u32;
		let end_era = 2u32;
		assert_ok!(VortexDistribution::<T>::set_vtx_dist_eras(RawOrigin::Root.into(), vortex_dist_id.clone(), start_era, end_era));
		let balance = <T as pallet_staking::Config>::CurrencyBalance::one();
		let asset_ids = BoundedVec::try_from(vec![asset_id]).unwrap();
		assert_ok!(VortexDistribution::<T>::set_assets_list(RawOrigin::Root.into(), asset_ids, vortex_dist_id.clone()));
		let root_price = balance;
		let asset_prices = BoundedVec::try_from(vec![(asset_id, balance.into())]).unwrap();
		assert_ok!(VortexDistribution::<T>::set_asset_prices(RawOrigin::Root.into(), root_price, asset_prices, vortex_dist_id));

		let vault_account = VortexDistribution::<T>::get_vtx_vault_account();
		let root_vault = VortexDistribution::<T>::get_root_vault_account();
		let fee_vault = VortexDistribution::<T>::get_fee_vault_account();

		let mint_amount = <T as pallet_staking::Config>::CurrencyBalance::one();
		assert_ok!(T::MultiCurrency::mint_into(asset_id, &fee_vault, mint_amount.into()));
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get(), &root_vault, mint_amount));
		assert_ok!(T::MultiCurrency::mint_into(T::VtxAssetId::get(), &vault_account, mint_amount));

		let balances = BoundedVec::try_from(vec![(account::<T>("test"), 100u32.into())]).unwrap();
		let points = BoundedVec::try_from(vec![(account::<T>("test"), 10u32.into())]).unwrap();
		let rates = BoundedVec::try_from(vec![(account::<T>("test"), 1u32.into())]).unwrap();

		for era in start_era..=end_era {
			assert_ok!(VortexDistribution::<T>::register_eff_bal_n_wk_pts(
				RawOrigin::Root.into(),
				vortex_dist_id.clone(),
				era,
				balances.clone(),
				points.clone(),
				rates.clone(),
			));
		}
		assert_ok!(VortexDistribution::<T>::trigger_vtx_distribution(RawOrigin::Root.into(), vortex_dist_id));
	}: _(RawOrigin::Root, vortex_dist_id)
	verify {
		assert_eq!(VtxDistStatuses::<T>::get(vortex_dist_id), VtxDistStatus::Paying);
	}

	set_vtx_dist_eras {
		let vortex_dist_id = NextVortexId::<T>::get();
		let start_era = 1u32;
		let end_era = 64u32;
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
	}: _(RawOrigin::Root, vortex_dist_id, start_era, end_era)
	verify {
		assert_eq!(VtxDistEras::<T>::get(vortex_dist_id), (start_era, end_era));
	}

	set_assets_list {
		let b in 1..500;

		let mut asset_ids_vec = vec![];
		for _ in 0..b {
			let asset_id = mint_asset::<T>();
			asset_ids_vec.push(asset_id);
		}
		let asset_ids = BoundedVec::try_from(asset_ids_vec).unwrap();
		let vortex_dist_id = NextVortexId::<T>::get();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
	}: _(RawOrigin::Root, asset_ids.clone(), vortex_dist_id)
	verify {
			assert_eq!(AssetsList::<T>::get(vortex_dist_id), asset_ids);
	}

	set_asset_prices {
		let b in 1..500;

		let balance =  <T as pallet_staking::Config>::CurrencyBalance::one();

		let mut asset_prices_vec = vec![];
		let mut asset_ids_vec = vec![];
		for i in 0..b {
			let asset_id = mint_asset::<T>();
			asset_prices_vec.push((asset_id, balance.into()));
			asset_ids_vec.push(asset_id);
		}

		let asset_prices = BoundedVec::try_from(asset_prices_vec).unwrap();
		let asset_ids = BoundedVec::try_from(asset_ids_vec).unwrap();
		let root_price = balance.into();
		let vortex_dist_id = NextVortexId::<T>::get();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
		assert_ok!(VortexDistribution::<T>::set_assets_list(RawOrigin::Root.into(), asset_ids, vortex_dist_id.clone()));
	}: _(RawOrigin::Root, root_price, asset_prices.clone(), vortex_dist_id)
	verify {
		for (asset_id, _) in asset_prices.into_iter() {
			assert_eq!(AssetPrices::<T>::get(vortex_dist_id, asset_id), balance);
		}
	}

	register_rewards {
		let vortex_dist_id = NextVortexId::<T>::get();
		let rewards = BoundedVec::try_from(vec![(account::<T>("test"), 100u32.into())]).unwrap();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
	}: _(RawOrigin::Root, vortex_dist_id, rewards)
	verify {
		let reward = VtxDistOrderbook::<T>::get(vortex_dist_id, account::<T>("test"));
		assert_eq!(reward, (100u32.into(), false));
	}

	register_eff_bal_n_wk_pts {
		let vortex_dist_id = NextVortexId::<T>::get();
		let asset_id = mint_asset::<T>();
		let start_era = 1u32;
		let end_era = 2u32;
		assert_ok!(VortexDistribution::<T>::set_vtx_dist_eras(RawOrigin::Root.into(), vortex_dist_id.clone(), start_era, end_era));
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
		let balance = <T as pallet_staking::Config>::CurrencyBalance::one();
		let asset_ids = BoundedVec::try_from(vec![asset_id]).unwrap();
		assert_ok!(VortexDistribution::<T>::set_assets_list(RawOrigin::Root.into(), asset_ids, vortex_dist_id.clone()));
		let root_price = balance;
		let asset_prices = BoundedVec::try_from(vec![(asset_id, balance.into())]).unwrap();
		let end_block: u32 = 500;
		assert_ok!(VortexDistribution::<T>::set_asset_prices(RawOrigin::Root.into(), root_price, asset_prices, vortex_dist_id));
		let vault_account = VortexDistribution::<T>::get_vtx_vault_account();
		let root_vault = VortexDistribution::<T>::get_root_vault_account();
		let fee_vault = VortexDistribution::<T>::get_fee_vault_account();
		let mint_amount = <T as pallet_staking::Config>::CurrencyBalance::one();
		assert_ok!(T::MultiCurrency::mint_into(asset_id, &fee_vault, mint_amount.into()));
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get(), &root_vault, mint_amount));
		assert_ok!(T::MultiCurrency::mint_into(T::VtxAssetId::get(), &vault_account, mint_amount));
		let balances = BoundedVec::try_from(vec![(account::<T>("test"), 100u32.into())]).unwrap();
		let points = BoundedVec::try_from(vec![(account::<T>("test"), 10u32.into())]).unwrap();
		let rates = BoundedVec::try_from(vec![(account::<T>("test"), 1u32.into())]).unwrap();
	}: _(RawOrigin::Root, vortex_dist_id, start_era, balances, points, rates)
	verify {
		let work_point = EffectiveBalancesWorkPoints::<T>::get((vortex_dist_id, start_era, account::<T>("test")));
		assert_eq!(work_point, (100u32.into(), 10u32.into(), 1u32.into()));
	}

	trigger_vtx_distribution {
		let vortex_dist_id = NextVortexId::<T>::get();
		let asset_id = mint_asset::<T>();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
		let balance = <T as pallet_staking::Config>::CurrencyBalance::one();
		let asset_ids = BoundedVec::try_from(vec![asset_id]).unwrap();
		assert_ok!(VortexDistribution::<T>::set_assets_list(RawOrigin::Root.into(), asset_ids, vortex_dist_id.clone()));
		let root_price = balance;
		let asset_prices = BoundedVec::try_from(vec![(asset_id, balance.into())]).unwrap();
		assert_ok!(VortexDistribution::<T>::set_asset_prices(RawOrigin::Root.into(), root_price, asset_prices, vortex_dist_id));

		let vault_account = VortexDistribution::<T>::get_vtx_vault_account();
		let root_vault = VortexDistribution::<T>::get_root_vault_account();
		let fee_vault = VortexDistribution::<T>::get_fee_vault_account();

		let mint_amount = <T as pallet_staking::Config>::CurrencyBalance::one();
		assert_ok!(T::MultiCurrency::mint_into(asset_id, &fee_vault, mint_amount.into()));
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get(), &root_vault, mint_amount));
		assert_ok!(T::MultiCurrency::mint_into(T::VtxAssetId::get(), &vault_account, mint_amount));
	}: _(RawOrigin::Root, vortex_dist_id)
	verify {
		let accum_vault = Vortex::<T>::get_vtx_vault_account();
		assert_eq!(T::MultiCurrency::balance(asset_id, &accum_vault), mint_amount);
		assert_eq!(T::MultiCurrency::balance(T::NativeAssetId::get(), &accum_vault), mint_amount);
	}

	redeem_tokens_from_vault {
		let vortex_dist_id = NextVortexId::<T>::get();
		let asset_id = mint_asset::<T>();
		let start_era = 1u32;
		let end_era = 2u32;
		assert_ok!(VortexDistribution::<T>::set_vtx_dist_eras(RawOrigin::Root.into(), vortex_dist_id.clone(), start_era, end_era));
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
		let balance = <T as pallet_staking::Config>::CurrencyBalance::one();
		let asset_ids = BoundedVec::try_from(vec![asset_id]).unwrap();
		assert_ok!(VortexDistribution::<T>::set_assets_list(RawOrigin::Root.into(), asset_ids, vortex_dist_id.clone()));
		let root_price = balance;
		let asset_prices = BoundedVec::try_from(vec![(asset_id, balance.into())]).unwrap();
		let end_block: u32 = 500;
		assert_ok!(VortexDistribution::<T>::set_asset_prices(RawOrigin::Root.into(), root_price, asset_prices, vortex_dist_id));
		let vault_account = VortexDistribution::<T>::get_vtx_vault_account();
		let root_vault = VortexDistribution::<T>::get_root_vault_account();
		let fee_vault = VortexDistribution::<T>::get_fee_vault_account();
		let mint_amount = <T as pallet_staking::Config>::CurrencyBalance::one();
		assert_ok!(T::MultiCurrency::mint_into(asset_id, &fee_vault, mint_amount.into()));
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get(), &root_vault, mint_amount));
		assert_ok!(T::MultiCurrency::mint_into(T::VtxAssetId::get(), &vault_account, mint_amount));
		let balances = BoundedVec::try_from(vec![(account::<T>("test"), 100u32.into())]).unwrap();
		let points = BoundedVec::try_from(vec![(account::<T>("test"), 10u32.into())]).unwrap();
		let rates = BoundedVec::try_from(vec![(account::<T>("test"), 1u32.into())]).unwrap();
		for era in start_era..=end_era {
			assert_ok!(VortexDistribution::<T>::register_eff_bal_n_wk_pts(
				RawOrigin::Root.into(),
				vortex_dist_id.clone(),
				era,
				balances.clone(),
				points.clone(),
				rates.clone(),
			));
		}
		assert_ok!(VortexDistribution::<T>::trigger_vtx_distribution(RawOrigin::Root.into(), vortex_dist_id));
		assert_ok!(VortexDistribution::<T>::start_vtx_dist(RawOrigin::Root.into(), vortex_dist_id));
		System::<T>::set_block_number(end_block.into());
		assert_ok!(VortexDistribution::<T>::pay_unsigned(RawOrigin::None.into(), vortex_dist_id, end_block.into()));
		VtxDistStatuses::<T>::mutate(vortex_dist_id, |status| {
			*status = VtxDistStatus::Done;
		});
	}: _(RawOrigin::Signed(account::<T>("test")), vortex_dist_id, balance)
	verify {
		assert_eq!(T::MultiCurrency::balance(T::VtxAssetId::get(), &account::<T>("test")), balance);

		let ratio = Perbill::from_rational(balance, 20u32.into());
		let mint_amount = <T as pallet_staking::Config>::CurrencyBalance::one();
		let expect_balance = ratio * mint_amount;
		assert_eq!(T::MultiCurrency::balance(asset_id, &account::<T>("test")), expect_balance);
		assert_eq!(T::MultiCurrency::balance(T::NativeAssetId::get(), &account::<T>("test")), expect_balance);
	}

	pay_unsigned {
		let vortex_dist_id = NextVortexId::<T>::get();
		let asset_id = mint_asset::<T>();
		assert_ok!(VortexDistribution::<T>::create_vtx_dist(RawOrigin::Root.into()));
		let start_era = 1u32;
		let end_era = 2u32;
		assert_ok!(VortexDistribution::<T>::set_vtx_dist_eras(RawOrigin::Root.into(), vortex_dist_id.clone(), start_era, end_era));
		let balance = <T as pallet_staking::Config>::CurrencyBalance::one();
		let asset_ids = BoundedVec::try_from(vec![asset_id]).unwrap();
		assert_ok!(VortexDistribution::<T>::set_assets_list(RawOrigin::Root.into(), asset_ids, vortex_dist_id.clone()));
		let root_price = balance;
		let asset_prices = BoundedVec::try_from(vec![(asset_id, balance.into())]).unwrap();
		let end_block: u32 = 500;
		assert_ok!(VortexDistribution::<T>::set_asset_prices(RawOrigin::Root.into(), root_price, asset_prices, vortex_dist_id));
		let vault_account = VortexDistribution::<T>::get_vtx_vault_account();
		let root_vault = VortexDistribution::<T>::get_root_vault_account();
		let fee_vault = VortexDistribution::<T>::get_fee_vault_account();
		let mint_amount = <T as pallet_staking::Config>::CurrencyBalance::one();
		assert_ok!(T::MultiCurrency::mint_into(asset_id, &fee_vault, mint_amount.into()));
		assert_ok!(T::MultiCurrency::mint_into(T::NativeAssetId::get(), &root_vault, mint_amount));
		assert_ok!(T::MultiCurrency::mint_into(T::VtxAssetId::get(), &vault_account, mint_amount));
		let balances = BoundedVec::try_from(vec![(account::<T>("test"), 100u32.into())]).unwrap();
		let points = BoundedVec::try_from(vec![(account::<T>("test"), 10u32.into())]).unwrap();
		let rates = BoundedVec::try_from(vec![(account::<T>("test"), 1u32.into())]).unwrap();
		for era in start_era..=end_era {
			assert_ok!(VortexDistribution::<T>::register_eff_bal_n_wk_pts(
				RawOrigin::Root.into(),
				vortex_dist_id.clone(),
				era,
				balances.clone(),
				points.clone(),
				rates.clone(),
			));
		}
		assert_ok!(VortexDistribution::<T>::trigger_vtx_distribution(RawOrigin::Root.into(), vortex_dist_id));
		assert_ok!(VortexDistribution::<T>::start_vtx_dist(RawOrigin::Root.into(), vortex_dist_id));
		System::<T>::set_block_number(end_block.into());
	}: _(RawOrigin::None, vortex_dist_id, end_block.into())
	verify {
		assert_eq!(T::MultiCurrency::balance(T::VtxAssetId::get(), &account::<T>("test")), 2u32.into());
	}
}
impl_benchmark_test_suite!(
	VortexDistribution,
	crate::mock::TestExt::benchmark().build(),
	crate::mock::Test,
);
