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

use crate::Pallet as Nft;
use codec::Encode;
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, BoundedVec};
use frame_system::RawOrigin;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

pub fn build_collection<T: Config>(caller: Option<T::AccountId>) -> CollectionUuid {
	let id = Nft::<T>::next_collection_uuid().unwrap();
	let caller = caller.unwrap_or_else(|| account::<T>("Alice"));
	let metadata_scheme = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
	let cross_chain_compatibility = CrossChainCompatibility::default();

	assert_ok!(Nft::<T>::create_collection(
		origin::<T>(&caller).into(),
		BoundedVec::truncate_from("New Collection".encode()),
		1000,
		None,
		None,
		metadata_scheme,
		None,
		cross_chain_compatibility,
	));

	id
}

benchmarks! {
	claim_unowned_collection {
		let collection_id = build_collection::<T>(Some(Nft::<T>::account_id()));
	}: _(RawOrigin::Root, collection_id, account::<T>("Alice"))

	set_owner {
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, account::<T>("Bob"))

	set_max_issuance {
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, 100001)

	set_base_uri {
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, "https://example.com/tokens/".into())

	set_name {
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, BoundedVec::truncate_from("New Name".encode()))

	create_collection {
		let p in 1 .. (500);
		let metadata = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
		let ccc = CrossChainCompatibility { xrpl: false };
	}: _(origin::<T>(&account::<T>("Alice")), BoundedVec::truncate_from("Collection".encode()), p, None, None, metadata, None, ccc)

	mint {
		let p in 1 .. (500);
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, p, None)

	transfer {
		let collection_id = build_collection::<T>(None);
		let p in 1 .. (500);
		let serial_numbers: Vec<SerialNumber> = (0..p).collect();
		let serial_numbers = BoundedVec::try_from(serial_numbers).unwrap();
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, serial_numbers, account::<T>("Bob"))

	burn {
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Alice")), TokenId::from((collection_id, 0)))
}

impl_benchmark_test_suite!(Nft, crate::mock::new_test_ext(), crate::mock::Test,);
