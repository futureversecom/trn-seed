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

//! NFT benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use seed_primitives::{nft::OriginChain, CrossChainCompatibility, MetadataScheme};

use crate::Pallet as TokenApprovals;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

// Create an NFT collection
// Returns the created `coll_id`
fn build_collection<T: Config>() -> (T::AccountId, CollectionUuid, TokenId) {
	let alice = account::<T>("Alice");
	let collection_name = BoundedVec::truncate_from("Hello".encode());
	let metadata_scheme = MetadataScheme::try_from(
		b"ethereum://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi/".as_slice(),
	)
	.unwrap();

	let collection_id = T::NFTExt::do_create_collection(
		alice.clone(),
		collection_name,
		0,
		None,
		None,
		metadata_scheme,
		None,
		OriginChain::Root,
		CrossChainCompatibility::default(),
	)
	.unwrap();

	assert_ok!(T::NFTExt::do_mint(alice.clone(), collection_id, 10, Some(alice.clone())));
	(alice, collection_id, TokenId::from((collection_id, 1)))
}

benchmarks! {
	erc721_approval {
		let ( alice, _, token_id ) = build_collection::<T>();
	}: _(RawOrigin::None, alice, account::<T>("Operator_Account"), token_id)

	erc721_remove_approval {
		let ( alice, _, token_id ) = build_collection::<T>();
		assert_ok!(TokenApprovals::<T>::erc721_approval(RawOrigin::None.into(), alice.clone(), account::<T>("Operator_Account"), token_id.clone()));
	}: _(origin::<T>(&alice), token_id.clone())

	erc20_approval {
		let ( alice, _, token_id ) = build_collection::<T>();
	}: _(RawOrigin::None, alice, account::<T>("Spender"), 100, Balance::from(10u32))

	erc20_update_approval {
		let ( alice, _, token_id ) = build_collection::<T>();
		let spender  = account::<T>("Spender");
		let asset_id = 100;

		assert_ok!(TokenApprovals::<T>::erc20_approval(RawOrigin::None.into(), alice.clone(), spender.clone(), asset_id, Balance::from(10u32)));
	}: _(RawOrigin::None, alice, spender, asset_id, Balance::from(2u32))

	erc721_approval_for_all {
		let ( alice, collection_id, _ ) = build_collection::<T>();
	}: _(RawOrigin::None, alice, account::<T>("Operator_Account"), collection_id, true)

	erc1155_approval_for_all {
		let ( alice, collection_id, _ ) = build_collection::<T>();
	}: _(RawOrigin::None, alice, account::<T>("Operator_Account"), collection_id, true)
}

impl_benchmark_test_suite!(
	TokenApprovals,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
