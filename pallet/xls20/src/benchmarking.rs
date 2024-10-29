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
use hex_literal::hex;
use pallet_nft::{CollectionInformation, TokenOwnership};
use seed_primitives::{nft::OriginChain, MetadataScheme, CrossChainCompatibility};
use sp_core::H160;

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
		CrossChainCompatibility::default()
	)
	.unwrap();
	assert_ok!(Xls20::<T>::enable_xls20_compatibility(origin::<T>(&caller).into(), collection_id));
	assert_ok!(Xls20::<T>::set_relayer(RawOrigin::Root.into(), relayer,));

	// Mint tokens
	if !initial_issuance.is_zero() {
		assert_ok!(T::NFTExt::do_mint(caller, collection_id, initial_issuance.into(), None,));
	}
	collection_id
}

benchmarks! {
	where_clause { where T: pallet_nft::Config }

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
		let serial_numbers = BoundedVec::truncate_from(vec![(0, hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66"))]);
	}: _(origin::<T>(&relayer), collection_id, serial_numbers)

	set_collection_mappings {
		let i in 0..256;
		let mut mappings: Vec<(CollectionUuid, Xls20Collection)> = Vec::new();
		for _ in 0..i {
			let collection_id = i as u32;
			let issuer = H160::from
			(hex!("95F14B0E44F78A264E41713C64B5F89242540EE2"));
			let xls20_collection = Xls20Collection::new(issuer, collection_id);
			mappings.push((collection_id, xls20_collection));
		}
	}: _(RawOrigin::Root, mappings.clone())
	verify {
		for (collection_id, xls20_collection) in mappings {
			assert_eq!(CollectionMapping::<T>::get(xls20_collection), Some(collection_id));
		}
	}

	deposit_token_transfer {
		let beneficiary = account::<T>("Beneficiary");
		let xls20_token_id = hex!("000B0C4495F14B0E44F78A264E41713C64B5F89242540EE2BC8B858E00000D65");
		let collection_id = 146_999_694;
		let serial_number = 3429;
		let issuer = H160::from(hex!("95F14B0E44F78A264E41713C64B5F89242540EE2"));
		let xls20_collection = Xls20Collection::new(issuer, collection_id);
		let pallet_address: T::AccountId = <T as pallet::Config>::PalletId::get().into_account_truncating();
		// Insert pallet address as the owner of the decoded token
		let owned_tokens = TokenOwnership::<T::AccountId, T::MaxTokensPerCollection>::new(
			pallet_address.clone(),
			BoundedVec::truncate_from(vec![serial_number])
		);
		let collection_info = CollectionInformation {
			owner: beneficiary.clone(),
			name: BoundedVec::truncate_from("New Collection".encode()),
			metadata_scheme: MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
			royalties_schedule: None,
			max_issuance: None,
			origin_chain: OriginChain::Root,
			next_serial_number: 3429,
			collection_issuance: 0,
			cross_chain_compatibility: CrossChainCompatibility::default(),
			owned_tokens: BoundedVec::truncate_from(vec![owned_tokens]),
		};
		// Insert collection data
		<pallet_nft::CollectionInfo<T>>::insert(collection_id, collection_info);
		CollectionMapping::<T>::insert(xls20_collection, collection_id);
		Xls20TokenMap::<T>::insert(collection_id, serial_number, xls20_token_id);

		// Sanity check
		let new_owner = T::NFTExt::get_token_owner(&(collection_id, serial_number)).unwrap();
		assert_eq!(new_owner, pallet_address);
	}: {Xls20::<T>::deposit_xls20_token(&beneficiary, xls20_token_id).expect("Failed to process asset deposit");}
	verify {
		// Token was transferred from pallet address to beneficiary
		let new_owner = T::NFTExt::get_token_owner(&(collection_id, serial_number)).unwrap();
		assert_eq!(new_owner, beneficiary);
	}

	deposit_token_mint {
		let beneficiary = account::<T>("Beneficiary");
		let xls20_token_id = hex!("000B0C4495F14B0E44F78A264E41713C64B5F89242540EE2BC8B858E00000D65");
		let collection_id = 146_999_694;
		let serial_number = 3429;
		let issuer = H160::from(hex!("95F14B0E44F78A264E41713C64B5F89242540EE2"));
		let xls20_collection = Xls20Collection::new(issuer, collection_id);
		let collection_info = CollectionInformation {
			owner: beneficiary.clone(),
			name: BoundedVec::truncate_from("New Collection".encode()),
			metadata_scheme: MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
			royalties_schedule: None,
			max_issuance: None,
			origin_chain: OriginChain::Root,
			next_serial_number: 3429,
			collection_issuance: 0,
			cross_chain_compatibility: CrossChainCompatibility::default(),
			owned_tokens: BoundedVec::truncate_from(vec![]), // No tokens
		};
		<pallet_nft::CollectionInfo<T>>::insert(collection_id, collection_info);
		CollectionMapping::<T>::insert(xls20_collection, collection_id);
	}: {Xls20::<T>::deposit_xls20_token(&beneficiary, xls20_token_id).expect("Failed to process asset deposit");}
	verify {
		// Token was minted
		let new_owner = T::NFTExt::get_token_owner(&(collection_id, serial_number)).unwrap();
		assert_eq!(new_owner, beneficiary);
	}

	deposit_token_create_collection {
		let beneficiary = account::<T>("Beneficiary");
		let xls20_token_id = hex!("000B0C4495F14B0E44F78A264E41713C64B5F89242540EE2BC8B858E00000D65");
		let collection_id = 146_999_694;
		let serial_number = 3429;
		let issuer = H160::from(hex!("95F14B0E44F78A264E41713C64B5F89242540EE2"));
		let next_collection_id = T::NFTExt::next_collection_uuid().expect("Failed to get next collection uuid");
	}: {Xls20::<T>::deposit_xls20_token(&beneficiary, xls20_token_id).expect("Failed to process asset deposit");}
	verify {
		// Token was minted
		let new_owner = T::NFTExt::get_token_owner(&(next_collection_id, serial_number)).unwrap();
		assert_eq!(new_owner, beneficiary);
		// Collection was created
		let collection_info = <pallet_nft::CollectionInfo<T>>::get(next_collection_id).expect("Failed to get collection info");
		let pallet_address: T::AccountId = <T as pallet::Config>::PalletId::get().into_account_truncating();
		assert_eq!(collection_info.owner, pallet_address);
	}
}

impl_benchmark_test_suite!(
	Xls20,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
