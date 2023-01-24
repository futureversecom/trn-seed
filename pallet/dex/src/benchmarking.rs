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
/*
pub struct BenchmarkData<T: Config> {
	pub coll_owner: T::AccountId,
	pub coll_id: CollectionUuid,
	pub coll_tokens: Vec<TokenId>,
	pub token_id: TokenId,
}

// Create an NFT collection
// Returns the created `coll_id`
fn setup_benchmark<T: Config>() -> BenchmarkData<T> {
	let alice = account::<T>("Alice");
	let coll_owner = alice.clone();
	let collection_name = "Hello".into();
	let metadata_scheme = MetadataScheme::IpfsDir(
		b"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi".to_vec(),
	);

	let coll_id = T::NFTExt::do_create_collection(
		coll_owner.clone(),
		collection_name,
		0,
		None,
		None,
		metadata_scheme,
		None,
		OriginChain::Root,
	)
	.unwrap();
	assert_ok!(T::NFTExt::do_mint(&coll_owner, coll_id, vec![1, 2]));
	let coll_tokens: Vec<TokenId> = vec![(coll_id, 1), (coll_id, 2)];

	let token_id = coll_tokens[0].clone();

	BenchmarkData { coll_owner, coll_id, coll_tokens, token_id }
} */

benchmarks! {
	swap_with_exact_supply {
		let alice = account::<T>("Alice");
		let asset_id_1 = T::MultiCurrency::create(&alice).unwrap();
		let asset_id_2 = T::MultiCurrency::create(&alice).unwrap();
		let mint_amount: Balance = 10000000u32.into();

		assert_ok!(T::MultiCurrency::mint_into(asset_id_1, &alice, mint_amount));
		assert_ok!(T::MultiCurrency::mint_into(asset_id_2, &alice, mint_amount));

		let amount_desired_1: Balance = 100000u32.into();
		let amount_desired_2: Balance = 200000u32.into();
		let amount_min_1: Balance = 1000u32.into();
		let amount_min_2: Balance = 1000u32.into();
		let min_share_increment: Balance = 100u32.into();

		assert_ok!(Dex::<T>::add_liquidity(origin::<T>(&alice).into(), asset_id_1, asset_id_2, amount_desired_1, amount_desired_2, amount_min_1, amount_min_2, min_share_increment));

		let path = vec![asset_id_1, asset_id_2];
		let amount_in: Balance = 100u32.into();
		let min_amount_out = 10u32.into();
		let before_balance = T::MultiCurrency::balance(asset_id_1, &alice);

	}: _(origin::<T>(&alice), amount_in, min_amount_out, path)
	verify {
		let after_balance = T::MultiCurrency::balance(asset_id_1, &alice);
		assert_eq!(after_balance, before_balance - amount_in);
	}


	impl_benchmark_test_suite!(Dex, crate::mock::new_test_ext(), crate::mock::Test,);
}
