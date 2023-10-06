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

use crate::{BlockedTokens, EthToRootNft, NextBlockedMintId, Pallet as NftPeg, RootNftToErc721};
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use pallet_nft::{CollectionInfo, CollectionInformation, CrossChainCompatibility, Pallet as Nft};
use seed_primitives::MetadataScheme;
use sp_std::vec;

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
		let p in 0..(50);

		let mut all_serials: Vec<BoundedVec<SerialNumber, T::MaxSerialsPerWithdraw>> = vec![];
		let mut collection_ids = vec![];
		for i in 0..p {
			let coll_id = Nft::<T>::next_collection_uuid().unwrap();
			let serial_numbers: Vec<SerialNumber> = vec![0u32, 1u32, 2u32];
			let token: H160 = H160::from_low_u64_be(i as u64);
			let token_1 = TokenInfo::<T>{token_address: token.clone().into(), token_ids: serial_numbers.clone().try_into().unwrap()};
			let token_info = GroupedTokenInfo::<T>{tokens: vec![token_1], destination: alice.clone()};

			// Deposit tokens to create collections
			assert_ok!(NftPeg::do_deposit(token_info, alice.clone().into()));

			let bounded: BoundedVec<SerialNumber, T::MaxSerialsPerWithdraw> = BoundedVec::truncate_from(serial_numbers);
			all_serials.push(bounded);
			collection_ids.push(coll_id);
		}
		let bounded_serial_numbers = BoundedVec::truncate_from(all_serials);
		let collection_ids = BoundedVec::truncate_from(collection_ids);

		// Sanity check
		for collection_id in &collection_ids {
			let collection_info: CollectionInformation<T::AccountId, T::MaxTokensPerCollection, T::StringLimit> = CollectionInfo::<T>::get(collection_id).expect("Collection exists");
			assert_eq!(collection_info.collection_issuance, 3);
		}
	}: _(origin::<T>(&alice), collection_ids.clone(), bounded_serial_numbers, alice.clone().into())
	verify {
		for collection_id in collection_ids {
			let collection_info: CollectionInformation<T::AccountId, T::MaxTokensPerCollection, T::StringLimit> = CollectionInfo::<T>::get(collection_id).expect("Collection exists");
			assert_eq!(collection_info.collection_issuance, 0);
		}
	}

	reclaim_blocked_nfts {
		let alice = account::<T>("Alice");
		let token = account::<T>("Token");

		let blocked_mint_id = NextBlockedMintId::<T>::get();

		let serial_numbers = vec![1_000_000_001_u32, 1_000_000_002_u32];
		let token_1 = TokenInfo::<T>{token_address: token.clone().into(), token_ids: serial_numbers.clone().try_into().unwrap()};
		let token_info = GroupedTokenInfo::<T>{tokens: vec![token_1], destination: alice.clone()};
		let collection_id = Nft::<T>::next_collection_uuid().unwrap();

		let collection_name = BoundedVec::truncate_from("test-collection".as_bytes().to_vec());
		let metadata_scheme = MetadataScheme::try_from(b"<CID>".as_slice()).unwrap();

		let collection_info = CollectionInformation {
			owner: alice.clone(),
			name: collection_name.clone(),
			metadata_scheme: metadata_scheme.clone(),
			royalties_schedule: None,
			max_issuance: None,
			origin_chain: OriginChain::Ethereum,
			next_serial_number: 1_000_000_001_u32,
			collection_issuance: 1_000_000_000_u32,
			cross_chain_compatibility: CrossChainCompatibility::default(),
			owned_tokens: BoundedVec::truncate_from(vec![]),
		};

		CollectionInfo::<T>::insert(collection_id, collection_info);
		EthToRootNft::<T>::insert(token.clone().into(), collection_id);
		RootNftToErc721::<T>::insert(collection_id, token.clone().into());

		let (_, err) =
			NftPeg::do_deposit(token_info, alice.clone().into()).unwrap_err();
		// Check tokens were blocked
		assert_eq!(err, pallet_nft::Error::<T>::BlockedMint.into());

	}: _(origin::<T>(&alice), blocked_mint_id, alice.clone().into())
	verify {
		let blocked_tokens = BlockedTokens::<T>::get(blocked_mint_id);
		assert!(blocked_tokens.is_none());
	}
}

impl_benchmark_test_suite!(NftPeg, crate::mock::new_test_ext(), crate::mock::Test,);
