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

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::{
	account as bench_account, benchmarks, impl_benchmark_test_suite, vec, Vec,
};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use seed_pallet_common::types::nft::{MetadataScheme, OriginChain};

use crate::Pallet as TokeApprovals;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

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
}

benchmarks! {
	erc721_approval {
		let BenchmarkData { coll_owner, token_id, .. } = setup_benchmark::<T>();
		let caller = coll_owner;
		let operator_account = account::<T>("Operator_Account");

	}: _(RawOrigin::None, caller, operator_account.clone(), token_id.clone())
	verify {
		assert_eq!(ERC721Approvals::<T>::get(token_id), Some(operator_account));
	}

	erc721_remove_approval {
		let BenchmarkData { coll_owner, token_id, .. } = setup_benchmark::<T>();
		let caller = coll_owner;
		let operator_account = account::<T>("Operator_Account");

		assert_ok!(TokeApprovals::<T>::erc721_approval(RawOrigin::None.into(), caller.clone(), operator_account.clone(), token_id.clone()));

		// Sanity check
		assert!(ERC721Approvals::<T>::get(token_id).is_some());

	}: _(origin::<T>(&caller), token_id.clone())
	verify {
		assert_eq!(ERC721Approvals::<T>::get(token_id), None);
	}

	erc20_approval {
		let BenchmarkData { coll_owner, token_id, .. } = setup_benchmark::<T>();
		let caller = coll_owner;
		let spender  = account::<T>("Spender");
		let asset_id = 100;
		let expected_amount: Balance = 10u32.into();

	}: _(RawOrigin::None, caller.clone(), spender.clone(), asset_id, expected_amount.clone())
	verify {
		let actual_balance = ERC20Approvals::<T>::get((caller, asset_id), spender);
		assert_eq!(actual_balance, Some(expected_amount));
	}

	erc20_update_approval {
		let BenchmarkData { coll_owner, token_id, .. } = setup_benchmark::<T>();
		let caller = coll_owner;
		let spender  = account::<T>("Spender");
		let asset_id = 100;
		let amount: Balance = 10u32.into();
		let decrease_by: Balance = 2u32.into();

		assert_ok!(TokeApprovals::<T>::erc20_approval(RawOrigin::None.into(), caller.clone(), spender.clone(), asset_id, amount.clone()));

	}: _(RawOrigin::None, caller.clone(), spender.clone(), asset_id, decrease_by.clone())
	verify {
		let expected_amount = amount - decrease_by;
		let actual_balance = ERC20Approvals::<T>::get((caller, asset_id), spender);
		assert_eq!(actual_balance, Some(expected_amount));
	}

	erc721_approval_for_all {
		let BenchmarkData { coll_owner, coll_id, .. } = setup_benchmark::<T>();
		let caller = coll_owner;
		let operator_account = account::<T>("Operator_Account");
		let expected_approved = true;

		// Sanity check
		let res = ERC721ApprovalsForAll::<T>::get(caller.clone(), (coll_id, operator_account.clone()));
		assert_eq!(res, None);


	}: _(RawOrigin::None, caller.clone(), operator_account.clone(), coll_id, expected_approved)
	verify {
		let actual_approved = ERC721ApprovalsForAll::<T>::get(caller, (coll_id, operator_account));
		assert_eq!(actual_approved, Some(expected_approved));
	}
}

impl_benchmark_test_suite!(TokeApprovals, crate::mock::new_test_ext(), crate::mock::Test,);
