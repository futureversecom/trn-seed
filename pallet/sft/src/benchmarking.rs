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
//! SFT benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, BoundedVec};
use frame_system::RawOrigin;
use sp_runtime::Permill;

use crate::Pallet as Sft;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

pub fn build_collection<T: Config>(caller: Option<T::AccountId>) -> CollectionUuid {
	let id = T::NFTExt::next_collection_uuid().expect("Failed to get next collection uuid");
	let caller = caller.unwrap_or_else(|| account::<T>("Alice"));
	let metadata_scheme = MetadataScheme::Https(b"example.com/metadata/".to_vec());
	let collection_name = BoundedVec::truncate_from("Collection".as_bytes().to_vec());

	assert_ok!(Sft::create_collection(
		Some(caller).into(),
		collection_name.clone(),
		None,
		metadata_scheme.clone(),
		None
	));

	id
}

/// Helper function for creating the bounded (SerialNumbers, Balance) type
pub fn bounded_combined(
	serial_numbers: Vec<SerialNumber>,
	quantities: Vec<Balance>,
) -> BoundedVec<(SerialNumber, Balance), <T as Config>::MaxSerialsPerMint> {
	let combined: Vec<(SerialNumber, Balance)> =
		serial_numbers.into_iter().zip(quantities).collect();
	BoundedVec::truncate_from(combined)
}

/// Helper function for creating the collection name type
pub fn bounded_string(name: &str) -> BoundedVec<u8, <T as Config>::StringLimit> {
	BoundedVec::truncate_from(name.as_bytes().to_vec())
}

benchmarks! {
	create_collection {
		let metadata = MetadataScheme::Https("google.com".into());
	}: _(origin::<T>(&account::<T>("Alice")), bounded_string("Collection"), None, metadata, None)

	create_token {
		let id = build_collection::<T>(None);
		let initial_issuance = u128::MAX;
	}: _(origin::<T>(&account::<T>("Alice")), id, bounded_string("Token"), initial_issuance, None, None)

	mint {
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, 1, None)

	transfer {
		let collection_id = build_collection::<T>(None);
		let serial_numbers = BoundedVec::try_from(vec![0]).unwrap();
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, serial_numbers, account::<T>("Bob"))

	burn {
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Alice")), TokenId::from((collection_id, 0)))

	set_owner {

	}: _(RawOrigin::Root, collection_id, account::<T>("Alice"))

	set_max_issuance {
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, 32)

	set_base_uri {
		let collection_id = build_collection::<T>(None);
	}: _(origin::<T>(&account::<T>("Alice")), collection_id, "https://example.com/tokens/".into())
}

impl_benchmark_test_suite!(Sft, crate::mock::new_test_ext(), crate::mock::Test,);
