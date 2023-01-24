// /* Copyright 2019-2021 Centrality Investments Limited
// *
// * Licensed under the LGPL, Version 3.0 (the "License");
// * you may not use this file except in compliance with the License.
// * Unless required by applicable law or agreed to in writing, software
// * distributed under the License is distributed on an "AS IS" BASIS,
// * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// * See the License for the specific language governing permissions and
// * limitations under the License.
// * You may obtain a copy of the License at the root of this project source code,
// * or at:
// * https://centrality.ai/licenses/gplv3.txt
// * https://centrality.ai/licenses/lgplv3.txt
// */
//! NFT benchmarking.

use super::*;

use frame_benchmarking::{
	account as bench_account, benchmarks, impl_benchmark_test_suite, vec, Vec,
};
use frame_support::assert_ok;
use frame_system::RawOrigin;

use crate::Pallet as Dex;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

pub struct BenchmarkData<T: Config> {
	pub alice: T::AccountId,
	pub asset_id_1: AssetId,
	pub asset_id_2: AssetId,
}

// Create an NFT collection
// Returns the created `coll_id`
fn setup_benchmark<T: Config>() -> BenchmarkData<T> {
	let alice = account::<T>("Alice");
	let asset_id_1 = T::MultiCurrency::create(&alice).unwrap();
	let asset_id_2 = T::MultiCurrency::create(&alice).unwrap();
	let mint_amount = Balance::from(10_000_000u32);

	assert_ok!(T::MultiCurrency::mint_into(asset_id_1, &alice, mint_amount));
	assert_ok!(T::MultiCurrency::mint_into(asset_id_2, &alice, mint_amount));

	assert_ok!(Dex::<T>::add_liquidity(
		origin::<T>(&alice).into(),
		asset_id_1,
		asset_id_2,
		100000u32.into(),
		200000u32.into(),
		1000u32.into(),
		1000u32.into(),
		100u32.into()
	));

	BenchmarkData { alice, asset_id_1, asset_id_2 }
}

benchmarks! {
	swap_with_exact_supply {
		let BenchmarkData { alice, asset_id_1, asset_id_2, .. } = setup_benchmark::<T>();

		let path = vec![asset_id_1, asset_id_2];
		let amount_in = Balance::from(100u32);
		let min_amount_out = Balance::from(10u32);
		let before_balance = T::MultiCurrency::balance(asset_id_1, &alice);

	}: _(origin::<T>(&alice), amount_in, min_amount_out, path)
	verify {
		let after_balance = T::MultiCurrency::balance(asset_id_1, &alice);
		assert_eq!(after_balance, before_balance - amount_in);
	}

	swap_with_exact_target {
		let BenchmarkData { alice, asset_id_1, asset_id_2, .. } = setup_benchmark::<T>();

		let path = vec![asset_id_1, asset_id_2];
		let amount_out = Balance::from(100u32);
		let amount_in_max = Balance::from(120u32);
		let before_balance = T::MultiCurrency::balance(asset_id_1, &alice);

	}: _(origin::<T>(&alice), amount_out, amount_in_max, path)
	verify {
		let after_balance = T::MultiCurrency::balance(asset_id_1, &alice);
		assert!(after_balance < before_balance);
	}

	add_liquidity {
		let alice = account::<T>("Alice");
		let asset_id_1 = T::MultiCurrency::create(&alice).unwrap();
		let asset_id_2 = T::MultiCurrency::create(&alice).unwrap();
		let mint_amount = Balance::from(10_000_000u32);

		assert_ok!(T::MultiCurrency::mint_into(asset_id_1, &alice, mint_amount));
		assert_ok!(T::MultiCurrency::mint_into(asset_id_2, &alice, mint_amount));
		let trading_pair = TradingPair::new(asset_id_1, asset_id_2);

		// Sanity check
		assert_eq!(TradingPairStatuses::<T>::get(&trading_pair), TradingPairStatus::NotEnabled);

	}: _(origin::<T>(&alice), asset_id_1, asset_id_2, 100000u32.into(), 200000u32.into(), 1000u32.into(), 1000u32.into(), 100u32.into())
	verify {
		assert_eq!(TradingPairStatuses::<T>::get(&trading_pair), TradingPairStatus::Enabled);
	}

	remove_liquidity {
		let BenchmarkData { alice, asset_id_1, asset_id_2 } = setup_benchmark::<T>();
		let before_balance = T::MultiCurrency::balance(asset_id_1, &alice);

	}: _(origin::<T>(&alice), asset_id_1, asset_id_2, 100u32.into(), 10u32.into(), 10u32.into())
	verify {
		let after_balance = T::MultiCurrency::balance(asset_id_1, &alice);
		assert!(after_balance > before_balance);
	}

	reenable_trading_pair {
		let BenchmarkData { asset_id_1, asset_id_2, .. } = setup_benchmark::<T>();
		let trading_pair = TradingPair::new(asset_id_1, asset_id_2);

		assert_ok!(Dex::<T>::disable_trading_pair(RawOrigin::Root.into(), asset_id_1, asset_id_2));
		// Sanity check
		assert_eq!(TradingPairStatuses::<T>::get(&trading_pair), TradingPairStatus::NotEnabled);

	}: _(RawOrigin::Root, asset_id_1, asset_id_2)
	verify {
		assert_eq!(TradingPairStatuses::<T>::get(&trading_pair), TradingPairStatus::Enabled);
	}

	disable_trading_pair {
		let BenchmarkData { asset_id_1, asset_id_2, .. } = setup_benchmark::<T>();
		let trading_pair = TradingPair::new(asset_id_1, asset_id_2);

		// Sanity check
		assert_eq!(TradingPairStatuses::<T>::get(&trading_pair), TradingPairStatus::Enabled);

	}: _(RawOrigin::Root, asset_id_1, asset_id_2)
	verify {
		assert_eq!(TradingPairStatuses::<T>::get(&trading_pair), TradingPairStatus::NotEnabled);
	}
}

impl_benchmark_test_suite!(Dex, crate::mock::new_test_ext(), crate::mock::Test,);
