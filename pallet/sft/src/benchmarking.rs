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
	let collection_name = bounded_string::<T>("Collection");

	assert_ok!(Sft::<T>::create_collection(
		origin::<T>(&caller).into(),
		collection_name.clone(),
		None,
		metadata_scheme.clone(),
		None
	));

	id
}

/// Helper function to create a token
/// Returns the TokenId (CollectionId, SerialNumber)
pub fn build_token<T: Config>(caller: Option<T::AccountId>, initial_issuance: Balance) -> TokenId {
	let caller = caller.unwrap_or_else(|| account::<T>("Alice"));
	let collection_id = build_collection::<T>(Some(caller.clone()));
	let token_name = bounded_string::<T>("test-token");

	assert_ok!(Sft::<T>::create_token(
		origin::<T>(&caller).into(),
		collection_id,
		token_name,
		initial_issuance,
		None,
		None,
	));

	(collection_id, 0)
}

/// Helper function for creating the bounded (SerialNumbers, Balance) type
pub fn bounded_combined<T: Config>(
	serial_numbers: Vec<SerialNumber>,
	quantities: Vec<Balance>,
) -> BoundedVec<(SerialNumber, Balance), <T as Config>::MaxSerialsPerMint> {
	let combined: Vec<(SerialNumber, Balance)> =
		serial_numbers.into_iter().zip(quantities).collect();
	BoundedVec::truncate_from(combined)
}

/// Helper function for creating the collection name type
pub fn bounded_string<T: Config>(name: &str) -> BoundedVec<u8, <T as Config>::StringLimit> {
	BoundedVec::truncate_from(name.as_bytes().to_vec())
}

benchmarks! {
	create_collection {
		let metadata = MetadataScheme::Https("google.com".into());
	}: _(origin::<T>(&account::<T>("Alice")), bounded_string::<T>("Collection"), None, metadata, None)

	create_token {
		let id = build_collection::<T>(None);
		let initial_issuance = u128::MAX;
	}: _(origin::<T>(&account::<T>("Alice")), id, bounded_string::<T>("Token"), initial_issuance, None, None)

	mint {
		let owner = account::<T>("Alice");
		let (collection_id, serial_number) = build_token::<T>(Some(owner.clone()), 0);
		let serial_numbers = bounded_combined::<T>(vec![serial_number], vec![u128::MAX]);
	}: _(origin::<T>(&owner), collection_id, serial_numbers, None)

	transfer {
		let owner = account::<T>("Alice");
		let (collection_id, serial_number) = build_token::<T>(Some(owner.clone()), u128::MAX);
		let serial_numbers = bounded_combined::<T>(vec![serial_number], vec![u128::MAX]);
	}: _(origin::<T>(&owner), collection_id, serial_numbers, account::<T>("Bob"))

	burn {
		let owner = account::<T>("Alice");
		let initial_issuance = 1000;
		let (collection_id, serial_number) = build_token::<T>(Some(owner.clone()), initial_issuance);
		let serial_numbers = bounded_combined::<T>(vec![serial_number], vec![initial_issuance]);
	}: _(origin::<T>(&owner), collection_id, serial_numbers)

	set_owner {
		let owner = account::<T>("Alice");
		let collection_id = build_collection::<T>(Some(owner.clone()));
	}: _(origin::<T>(&owner), collection_id, account::<T>("Bob"))

	set_max_issuance {
		let owner = account::<T>("Alice");
		let token_id = build_token::<T>(Some(owner.clone()), 0);
	}: _(origin::<T>(&owner), token_id, 32)

	set_base_uri {
		let owner = account::<T>("Alice");
		let id = build_collection::<T>(Some(owner.clone()));
		let metadata_scheme = MetadataScheme::Https(b"example.com/changed".to_vec());
	}: _(origin::<T>(&owner), id, metadata_scheme)
}

impl_benchmark_test_suite!(Sft, crate::mock::new_test_ext(), crate::mock::Test,);
