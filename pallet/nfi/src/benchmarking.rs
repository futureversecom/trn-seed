// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

//! Marketplace benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as Nfi;
use codec::Encode;
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, BoundedVec};
use frame_system::RawOrigin;
use pallet_nft::Pallet as Nft;
use seed_primitives::{CrossChainCompatibility, MetadataScheme};
use sp_std::vec;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

pub fn build_collection<T: Config + pallet_nft::Config>(
	caller: Option<T::AccountId>,
) -> CollectionUuid {
	let collection_id = Nft::<T>::next_collection_uuid().unwrap();
	let caller = caller.unwrap_or_else(|| account::<T>("Alice"));
	let metadata_scheme = MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap();
	let cross_chain_compatibility = CrossChainCompatibility::default();

	assert_ok!(Nft::<T>::create_collection(
		origin::<T>(&caller).into(),
		BoundedVec::truncate_from("New Collection".encode()),
		100,
		None,
		None,
		metadata_scheme,
		None,
		cross_chain_compatibility,
	));

	collection_id
}

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent)
where
	<T as frame_system::Config>::AccountId: From<sp_core::H160>,
{
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

benchmarks! {
	where_clause { where T: pallet_nft::Config + pallet_sft::Config, <T as frame_system::Config>::AccountId: From<sp_core::H160> }

	set_relayer {
		let relayer = account::<T>("Bob");
	}: _(RawOrigin::Root, relayer.clone())
	verify {
		assert_eq!(Relayer::<T>::get(), Some(relayer));
	}

	set_fee_to {
		let fee_account = account::<T>("Alice");
	}: _(RawOrigin::Root, Some(fee_account.clone()))
	verify {
		assert_eq!(FeeTo::<T>::get().unwrap(), fee_account);
	}

	set_fee_details {
		let fee_details = FeeDetails {
			asset_id: 1,
			amount: 100,
			receiver: account::<T>("Alice"),
		};
		let sub_type = NFISubType::NFI;
	}: _(RawOrigin::Root, sub_type, Some(fee_details.clone()))
	verify {
		assert_eq!(MintFee::<T>::get(sub_type), Some(fee_details));
	}

	enable_nfi {
		let caller = account::<T>("Alice");
		let collection_id = build_collection::<T>(Some(caller.clone()));
		let sub_type = NFISubType::NFI;
	}: _(origin::<T>(&caller), collection_id, sub_type)
	verify {
		assert!(NfiEnabled::<T>::get(collection_id, sub_type));
	}

	manual_data_request {
		let caller = account::<T>("Alice");
		let collection_id = build_collection::<T>(Some(caller.clone()));
		let sub_type = NFISubType::NFI;
		let token_id = (collection_id, 0);
		assert_ok!(Nfi::<T>::enable_nfi(origin::<T>(&caller).into(), collection_id, sub_type));
	}: _(origin::<T>(&caller), token_id, sub_type)
	verify {
		assert_last_event::<T>(Event::DataRequest {
			caller,
			sub_type,
			collection_id,
			serial_numbers: vec![0]
		}.into())
	}

	submit_nfi_data {
		let caller = account::<T>("Alice");
		let collection_id = build_collection::<T>(Some(caller.clone()));
		let sub_type = NFISubType::NFI;
		let token_id = (collection_id, 0);
		let data_item = NFIDataType::NFI(NFIMatrix {
			metadata_link: BoundedVec::truncate_from(b"https://google.com/".to_vec()),
			verification_hash: Default::default(),
		});
		assert_ok!(Nfi::<T>::enable_nfi(origin::<T>(&caller).into(), collection_id, sub_type));
		assert_ok!(Nfi::<T>::set_relayer(RawOrigin::Root.into(), caller.clone()));
	}: _(origin::<T>(&caller), token_id, data_item.clone())
	verify {
		assert_last_event::<T>(Event::DataSet {
			sub_type,
			token_id,
			data_item
		}.into())
	}
}

impl_benchmark_test_suite!(
	Nfi,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
