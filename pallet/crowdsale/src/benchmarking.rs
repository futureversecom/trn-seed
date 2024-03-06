// Copyright 2024-2025 Futureverse Corporation Limited
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

use super::*;
#[allow(unused_imports)]
use crate::Pallet as CrowdSale;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::traits::fungibles::Inspect;
use frame_system::RawOrigin;
use seed_primitives::{nft::OriginChain, MetadataScheme};

benchmarks! {

	initialize {
		let acc: T::AccountId = account("acc", 0, 0);
		let payment_asset_id = T::MultiCurrency::create(&acc, None).unwrap();
		let collection_id = T::NFTExt::do_create_collection(
			acc.clone(),
			BoundedVec::truncate_from("Hello".encode()),
			0,
			Some(1000),
			None,
			MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
			None,
			OriginChain::Root,
		).unwrap();
		let soft_cap_price = 50_000_000;
		let sale_duration: T::BlockNumber = 1000_u32.into();
	}: _(RawOrigin::Signed(acc.clone()), payment_asset_id, collection_id, soft_cap_price, sale_duration)
  verify {
		// validate NextSaleId
		assert_eq!(NextSaleId::<T>::get(), 1);
		assert_eq!(SaleInfo::<T>::get(0).unwrap().status, SaleStatus::Pending(1_u32.into()));
  }

	enable {
		let acc: T::AccountId = account("acc", 0, 0);
		let payment_asset_id = T::MultiCurrency::create(&acc, None).unwrap();
		let collection_id = T::NFTExt::do_create_collection(
			acc.clone(),
			BoundedVec::truncate_from("Hello".encode()),
			0,
			Some(1000),
			None,
			MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
			None,
			OriginChain::Root,
		).unwrap();
		let soft_cap_price = 50_000_000;
		let sale_duration: T::BlockNumber = 1000_u32.into();
		CrowdSale::<T>::initialize(RawOrigin::Signed(acc.clone()).into(), payment_asset_id, collection_id, soft_cap_price, sale_duration).unwrap();

		let sale_id = 0;
	}: _(RawOrigin::Signed(acc.clone()), sale_id)
  verify {
		assert_eq!(SaleInfo::<T>::get(0).unwrap().status, SaleStatus::Enabled(1_u32.into()));
  }

	participate {
		let acc: T::AccountId = account("acc", 0, 0);
		let payment_asset_id = T::MultiCurrency::create(&acc, None).unwrap();
		let collection_id = T::NFTExt::do_create_collection(
			acc.clone(),
			BoundedVec::truncate_from("Hello".encode()),
			0,
			Some(1000),
			None,
			MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
			None,
			OriginChain::Root,
		).unwrap();
		let soft_cap_price = 50_000_000;
		let sale_duration: T::BlockNumber = 1000_u32.into();

		let sale_id = 0;
		CrowdSale::<T>::initialize(RawOrigin::Signed(acc.clone()).into(), payment_asset_id, collection_id, soft_cap_price, sale_duration).unwrap();
		CrowdSale::<T>::enable(RawOrigin::Signed(acc.clone()).into(), sale_id).unwrap();

		// mint a participant some tokens
		let participant = account("participant", 0, 0);
		let amount = 100_000_000;
		T::MultiCurrency::mint_into(payment_asset_id, &participant, amount).unwrap();

	}: _(RawOrigin::Signed(participant.clone()), sale_id, amount)
	verify {
		assert_eq!(SaleInfo::<T>::get(0).unwrap().funds_raised, 100_000_000);
	}

	distribute_crowdsale_rewards {
		let acc: T::AccountId = account("acc", 0, 0);
		let payment_asset_id = T::MultiCurrency::create(&acc, None).unwrap();
		let collection_id = T::NFTExt::do_create_collection(
			acc.clone(),
			BoundedVec::truncate_from("Hello".encode()),
			0,
			Some(1000),
			None,
			MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
			None,
			OriginChain::Root,
		).unwrap();
		let soft_cap_price = 50_000_000;
		let sale_duration: T::BlockNumber = 1000_u32.into();

		let sale_id = 0;
		CrowdSale::<T>::initialize(RawOrigin::Signed(acc.clone()).into(), payment_asset_id, collection_id, soft_cap_price, sale_duration).unwrap();
		CrowdSale::<T>::enable(RawOrigin::Signed(acc.clone()).into(), sale_id).unwrap();

		// mint a participant some tokens; participate in the sale
		let participant = account("participant", 0, 0);
		let amount = 100_000_000;
		T::MultiCurrency::mint_into(payment_asset_id, &participant, amount).unwrap();
		CrowdSale::<T>::participate(RawOrigin::Signed(participant.clone()).into(), sale_id, amount).unwrap();

		// update block no. to end the sale
		let current_block = <frame_system::Pallet<T>>::block_number();
		let end_block = SaleInfo::<T>::get(0).unwrap().duration.saturating_add(current_block);
		<frame_system::Pallet<T>>::set_block_number(end_block);

		// call hook to end the sale
		CrowdSale::<T>::on_initialize(end_block.into());
	}: _(RawOrigin::None)
	verify {
		let sale_info = SaleInfo::<T>::get(0).unwrap();
		assert_eq!(sale_info.status, SaleStatus::Ended(end_block, 2_000_000));
	}

	claim_voucher {
		let acc: T::AccountId = account("acc", 0, 0);
		let payment_asset_id = T::MultiCurrency::create(&acc, None).unwrap();
		let collection_id = T::NFTExt::do_create_collection(
			acc.clone(),
			BoundedVec::truncate_from("Hello".encode()),
			0,
			Some(1000),
			None,
			MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
			None,
			OriginChain::Root,
		).unwrap();
		let soft_cap_price = 50_000_000;
		let sale_duration: T::BlockNumber = 1000_u32.into();

		let sale_id = 0;
		CrowdSale::<T>::initialize(RawOrigin::Signed(acc.clone()).into(), payment_asset_id, collection_id, soft_cap_price, sale_duration).unwrap();
		CrowdSale::<T>::enable(RawOrigin::Signed(acc.clone()).into(), sale_id).unwrap();

		// mint a participant some tokens; participate in the sale
		let participant = account("participant", 0, 0);
		let amount = 100_000_000;
		T::MultiCurrency::mint_into(payment_asset_id, &participant, amount).unwrap();
		CrowdSale::<T>::participate(RawOrigin::Signed(participant.clone()).into(), sale_id, amount).unwrap();

		// update block no. to end the sale
		let current_block = <frame_system::Pallet<T>>::block_number();
		let end_block = SaleInfo::<T>::get(0).unwrap().duration.saturating_add(current_block);
		<frame_system::Pallet<T>>::set_block_number(end_block);

		// call hook to end the sale
		CrowdSale::<T>::on_initialize(end_block.into());

	}: _(RawOrigin::Signed(participant.clone()), sale_id)
	verify {
		let sale_info = SaleInfo::<T>::get(0).unwrap();

		assert_eq!(sale_info.status, SaleStatus::Ended(end_block, 2_000_000));
		assert_eq!(T::MultiCurrency::balance(sale_info.voucher_asset_id, &sale_info.vault), 0);
		assert_eq!(T::MultiCurrency::balance(sale_info.voucher_asset_id, &acc), 998_000_000);
		assert_eq!(T::MultiCurrency::balance(sale_info.voucher_asset_id, &participant), 2_000_000);
	}

	redeem_voucher {
		let acc: T::AccountId = account("acc", 0, 0);
		let payment_asset_id = T::MultiCurrency::create(&acc, None).unwrap();
		let collection_id = T::NFTExt::do_create_collection(
			acc.clone(),
			BoundedVec::truncate_from("Hello".encode()),
			0,
			Some(1000),
			None,
			MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
			None,
			OriginChain::Root,
		).unwrap();
		let soft_cap_price = 50_000_000;
		let sale_duration: T::BlockNumber = 1000_u32.into();

		let sale_id = 0;
		CrowdSale::<T>::initialize(RawOrigin::Signed(acc.clone()).into(), payment_asset_id, collection_id, soft_cap_price, sale_duration).unwrap();
		CrowdSale::<T>::enable(RawOrigin::Signed(acc.clone()).into(), sale_id).unwrap();

		// mint a participant some tokens; participate in the sale
		let participant = account("participant", 0, 0);
		let amount = 100_000_000;
		T::MultiCurrency::mint_into(payment_asset_id, &participant, amount).unwrap();
		CrowdSale::<T>::participate(RawOrigin::Signed(participant.clone()).into(), sale_id, amount).unwrap();

		// update block no. to end the sale
		let current_block = <frame_system::Pallet<T>>::block_number();
		let end_block = SaleInfo::<T>::get(0).unwrap().duration.saturating_add(current_block);
		<frame_system::Pallet<T>>::set_block_number(end_block);

		// call hook to end the sale
		CrowdSale::<T>::on_initialize(end_block.into());

		// claim the voucher
		CrowdSale::<T>::claim_voucher(RawOrigin::Signed(participant.clone()).into(), sale_id).unwrap();
	}: _(RawOrigin::Signed(participant.clone()), sale_id, 2)
	verify {
		let sale_info = SaleInfo::<T>::get(0).unwrap();
		assert_eq!(T::MultiCurrency::balance(sale_info.voucher_asset_id, &participant), 0);
		assert_eq!(T::NFTExt::get_collection_issuance(collection_id).unwrap(), (2, Some(1000)));
	}
}

impl_benchmark_test_suite!(
	CrowdSale,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
