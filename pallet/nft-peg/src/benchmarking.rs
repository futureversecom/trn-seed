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

use crate::Pallet as NftPeg;
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use pallet_nft::{CollectionInfo, CollectionInformation, Pallet as Nft};
use sp_std::vec;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

pub fn bound_serial_numbers<T: Config>(
	serial_numbers: Vec<SerialNumber>,
) -> BoundedVec<BoundedVec<SerialNumber, T::MaxSerialsPerWithdraw>, T::MaxCollectionsPerWithdraw> {
	let inner_serials = BoundedVec::truncate_from(serial_numbers);
	BoundedVec::truncate_from(vec![inner_serials])
}

benchmarks! {
	where_clause { where <T as frame_system::Config>::AccountId: From<sp_core::H160> + Into<sp_core::H160> }
	set_contract_address {
		let contract = account::<T>("Contract");
	}: _(RawOrigin::Root, contract.clone().into())
	verify {
		assert_eq!(ContractAddress::<T>::get(), contract.into());
	}

	withdraw {
		let alice = account::<T>("Alice");
		let token = account::<T>("Token");

		let serial_numbers = vec![0u32, 1u32, 2u32];
		let bounded_serial_numbers = bound_serial_numbers::<T>(serial_numbers.clone());
		let token_1 = TokenInfo::<T>{token_address: token.into(), token_ids: serial_numbers.clone().try_into().unwrap()};
		let token_info = GroupedTokenInfo::<T>{tokens: vec![token_1], destination: alice.clone()};
		let coll_id = Nft::<T>::next_collection_uuid().unwrap();
		let collection_ids = BoundedVec::truncate_from(vec![coll_id]);

		assert_ok!(NftPeg::do_deposit(token_info, alice.clone().into()));

		// Sanity Check
		let collection_info: CollectionInformation<T::AccountId, T::MaxTokensPerCollection, T::StringLimit> = CollectionInfo::<T>::get(coll_id).expect("Collection exists");
		for serial_id in &serial_numbers {
			assert!(collection_info.token_exists(*serial_id));
		}

	}: _(origin::<T>(&alice), collection_ids, bounded_serial_numbers, alice.clone().into())
	verify {
		for serial_id in serial_numbers {
			assert!(collection_info.token_exists(serial_id));
		}
	}

}

impl_benchmark_test_suite!(NftPeg, crate::mock::new_test_ext(), crate::mock::Test,);
