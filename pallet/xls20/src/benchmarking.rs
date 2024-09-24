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

//! XLS-20 benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use crate::Pallet as Xls20;
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, BoundedVec};
use frame_system::RawOrigin;
use seed_primitives::{nft::OriginChain, MetadataScheme, xrpl::Xls20TokenId};

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

pub fn build_xls20_collection<T: Config>(
	caller: Option<T::AccountId>,
	relayer: Option<T::AccountId>,
	initial_issuance: u32,
) -> CollectionUuid {
	let caller = caller.unwrap_or_else(|| account::<T>("Alice"));
	let relayer = relayer.unwrap_or_else(|| account::<T>("Bob"));
	let metadata_scheme = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
	let collection_id = T::NFTExt::do_create_collection(
		caller.clone(),
		BoundedVec::truncate_from("New Collection".encode()),
		0,
		None,
		None,
		metadata_scheme,
		None,
		OriginChain::Root,
	)
	.unwrap();

	assert_ok!(Xls20::<T>::enable_xls20_compatibility(origin::<T>(&caller).into(), collection_id));

	// Setup relayer
	assert_ok!(Xls20::<T>::set_relayer(RawOrigin::Root.into(), relayer,));

	// Mint tokens
	if !initial_issuance.is_zero() {
		assert_ok!(T::NFTExt::do_mint(caller, collection_id, initial_issuance.into(), None,));
	}

	collection_id
}

fn setup_token_mappings<T: Config>(
	input: Vec<(SerialNumber, &str)>,
) -> BoundedVec<(SerialNumber, Xls20TokenId), T::MaxTokensPerXls20Mint> {
	let input: Vec<(SerialNumber, Xls20TokenId)> = input
		.into_iter()
		.map(|(s, token)| (s, Xls20TokenId::try_from(token.as_bytes()).unwrap()))
		.collect();

	BoundedVec::try_from(input).unwrap()
}

benchmarks! {
	set_relayer {
	}: _(RawOrigin::Root, account::<T>("Bob"))

	set_xls20_fee {
	}: _(RawOrigin::Root, 100_u32.into())

	enable_xls20_compatibility {
		let caller = account::<T>("Alice");
		let collection_id = build_xls20_collection::<T>(Some(caller.clone()), None, 0);
	}: _(origin::<T>(&caller), collection_id)

	re_request_xls20_mint {
		let caller = account::<T>("Alice");
		let collection_id = build_xls20_collection::<T>(Some(caller.clone()), None, 1);
		let serial_numbers = BoundedVec::try_from(vec![0]).unwrap();
	}: _(origin::<T>(&caller), collection_id, serial_numbers)

	fulfill_xls20_mint {
		let caller = account::<T>("Alice");
		let relayer = account::<T>("Bob");
		let collection_id = build_xls20_collection::<T>(Some(caller), Some(relayer.clone()), 1);
		let serial_numbers = setup_token_mappings::<T>(vec![(0, "000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66")]);
	}: _(origin::<T>(&relayer), collection_id, serial_numbers)
}

impl_benchmark_test_suite!(
	Xls20,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
