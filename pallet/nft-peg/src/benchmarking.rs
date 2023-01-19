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

use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;

use crate::Pallet as NftPeg;
use pallet_nft::{Pallet as Nft, TokenOwner};

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
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

		let token_ids = vec![0u32, 1u32, 2u32];
		let token_1 = TokenInfo::<T>{token_address: token.into(), token_ids: token_ids.clone().try_into().unwrap()};
		let token_info = GroupedTokenInfo::<T>{tokens: vec![token_1], destination: alice.clone()};
		let coll_id = Nft::<T>::next_collection_uuid().unwrap();

		assert_ok!(NftPeg::do_deposit(token_info, alice.clone().into()));

		// Sanity Check
		for serial_id in &token_ids {
			assert!(TokenOwner::<T>::get(coll_id, *serial_id).is_some());
		}

	}: _(origin::<T>(&alice), vec![coll_id], vec![token_ids.clone()], alice.clone().into())
	verify {
		for serial_id in token_ids {
			assert!(TokenOwner::<T>::get(coll_id, serial_id).is_none());
		}
	}

}

impl_benchmark_test_suite!(NftPeg, crate::mock::new_test_ext(), crate::mock::Test,);
