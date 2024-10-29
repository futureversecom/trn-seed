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
use seed_primitives::{nft::OriginChain, CrossChainCompatibility, MetadataScheme};

pub fn build_collection<T: Config>(collection_owner: T::AccountId) -> CollectionUuid {
	T::NFTExt::do_create_collection(
		collection_owner.clone(),
		BoundedVec::truncate_from("Hello".encode()),
		0,
		Some(1000),
		None,
		MetadataScheme::try_from(b"https://google.com/".as_slice()).unwrap(),
		None,
		OriginChain::Root,
		CrossChainCompatibility::default(),
	)
	.unwrap()
}

fn initialize_crowdsale<T: Config>(owner: T::AccountId) -> (SaleId, AssetId, CollectionUuid) {
	let payment_asset_id = T::MultiCurrency::create(&owner, None).unwrap();
	let collection_id = build_collection::<T>(owner.clone());

	let soft_cap_price = 50_000_000;
	let sale_duration: BlockNumberFor<T> = 1000_u32.into();

	let sale_id = NextSaleId::<T>::get();
	let voucher_max_len_data =
		BoundedVec::try_from(vec![0u8; T::StringLimit::get() as usize]).unwrap();
	CrowdSale::<T>::initialize(
		RawOrigin::Signed(owner.clone()).into(),
		payment_asset_id,
		collection_id,
		soft_cap_price,
		sale_duration,
		Some(voucher_max_len_data.clone()),
		Some(voucher_max_len_data),
	)
	.unwrap();
	CrowdSale::<T>::enable(RawOrigin::Signed(owner.clone()).into(), sale_id).unwrap();
	(sale_id, payment_asset_id, collection_id)
}

benchmarks! {
	initialize {
		let acc: T::AccountId = account("acc", 0, 0);
		let payment_asset_id = T::MultiCurrency::create(&acc, None).unwrap();
		let collection_id = build_collection::<T>(acc.clone());
		let soft_cap_price = 50_000_000;
		let sale_duration: BlockNumberFor<T> = 1000_u32.into();
		let voucher_max_len_data = BoundedVec::try_from(vec![0u8; T::StringLimit::get() as usize]).unwrap();
	}: _(RawOrigin::Signed(acc.clone()), payment_asset_id, collection_id, soft_cap_price, sale_duration, Some(voucher_max_len_data.clone()), Some(voucher_max_len_data))
	verify {
		// validate NextSaleId
		assert_eq!(NextSaleId::<T>::get(), 1);
		assert_eq!(SaleInfo::<T>::get(0).unwrap().status, SaleStatus::Pending(1_u32.into()));
  }

	enable {
		let acc: T::AccountId = account("acc", 0, 0);
		let payment_asset_id = T::MultiCurrency::create(&acc, None).unwrap();
		let collection_id = build_collection::<T>(acc.clone());

		let soft_cap_price = 50_000_000;
		let sale_duration: BlockNumberFor<T> = 1000_u32.into();
		CrowdSale::<T>::initialize(RawOrigin::Signed(acc.clone()).into(), payment_asset_id, collection_id, soft_cap_price, sale_duration, None, None).unwrap();

		let sale_id = 0;
	}: _(RawOrigin::Signed(acc.clone()), sale_id)
  verify {
		assert_eq!(SaleInfo::<T>::get(0).unwrap().status, SaleStatus::Enabled(1_u32.into()));
  }

	participate {
		let acc: T::AccountId = account("acc", 0, 0);
		let (sale_id, payment_asset_id, _) = initialize_crowdsale::<T>(acc.clone());

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
		let (sale_id, payment_asset_id, _) = initialize_crowdsale::<T>(acc.clone());

		// mint a participant some tokens; participate in the sale
		let participant = account("participant", 0, 0);
		let amount = 100_000_000;
		T::MultiCurrency::mint_into(payment_asset_id, &participant, amount).unwrap();
		CrowdSale::<T>::participate(RawOrigin::Signed(participant.clone()).into(), sale_id, amount).unwrap();

		// update block no. to end the sale
		let current_block = <frame_system::Pallet<T>>::block_number();
		let end_block = SaleInfo::<T>::get(sale_id).unwrap().duration.saturating_add(current_block);
		<frame_system::Pallet<T>>::set_block_number(end_block);

		// call hook to end the sale
		CrowdSale::<T>::on_initialize(end_block.into());
	}: _(RawOrigin::None)
	verify {
		let sale_info = SaleInfo::<T>::get(0).unwrap();
		assert_eq!(sale_info.status, SaleStatus::Ended(end_block));
	}

	claim_voucher {
		let acc: T::AccountId = account("acc", 0, 0);
		let (sale_id, payment_asset_id, _) = initialize_crowdsale::<T>(acc.clone());

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
		let sale_info = SaleInfo::<T>::get(sale_id).unwrap();

		assert_eq!(sale_info.status, SaleStatus::Ended(end_block));
		assert_eq!(T::MultiCurrency::balance(sale_info.voucher_asset_id, &sale_info.vault), 0);
		assert_eq!(T::MultiCurrency::balance(sale_info.voucher_asset_id, &acc), 998_000_000);
		assert_eq!(T::MultiCurrency::balance(sale_info.voucher_asset_id, &participant), 2_000_000);
	}

	redeem_voucher {
		let acc: T::AccountId = account("acc", 0, 0);
		let (sale_id, payment_asset_id, collection_id) = initialize_crowdsale::<T>(acc.clone());

		// mint a participant some tokens; participate in the sale
		let participant = account("participant", 0, 0);
		let amount = 100_000_000;
		T::MultiCurrency::mint_into(payment_asset_id, &participant, amount).unwrap();
		CrowdSale::<T>::participate(RawOrigin::Signed(participant.clone()).into(), sale_id, amount).unwrap();

		// update block no. to end the sale
		let current_block = <frame_system::Pallet<T>>::block_number();
		let end_block = SaleInfo::<T>::get(sale_id).unwrap().duration.saturating_add(current_block);
		<frame_system::Pallet<T>>::set_block_number(end_block);

		// call hook to end the sale
		CrowdSale::<T>::on_initialize(end_block.into());

		// claim the voucher
		CrowdSale::<T>::claim_voucher(RawOrigin::Signed(participant.clone()).into(), sale_id).unwrap();
	}: _(RawOrigin::Signed(participant.clone()), sale_id, 2)
	verify {
		let sale_info = SaleInfo::<T>::get(sale_id).unwrap();
		assert_eq!(T::MultiCurrency::balance(sale_info.voucher_asset_id, &participant), 0);
		assert_eq!(T::NFTExt::get_collection_issuance(collection_id).unwrap(), (2, Some(1000)));
	}

	proxy_vault_call {
		let acc: T::AccountId = account("acc", 0, 0);
		let (sale_id, payment_asset_id, collection_id) = initialize_crowdsale::<T>(acc.clone());
		let call: <T as Config>::RuntimeCall = frame_system::Call::<T>::remark { remark: b"Mischief Managed".to_vec() }.into();
	}: _(RawOrigin::Signed(acc), sale_id, Box::new(call))

	try_force_distribution {
		let acc: T::AccountId = account("acc", 0, 0);
		let (sale_id, payment_asset_id, collection_id) = initialize_crowdsale::<T>(acc.clone());

		// update block no. to end the sale
		let current_block = <frame_system::Pallet<T>>::block_number();
		let end_block = SaleInfo::<T>::get(0).unwrap().duration.saturating_add(current_block);
		<frame_system::Pallet<T>>::set_block_number(end_block);

		// manually change the status to DistributionFailed
		let mut sale_info = SaleInfo::<T>::get(sale_id).unwrap();
		sale_info.status = SaleStatus::DistributionFailed(end_block);
		SaleInfo::<T>::insert(0, sale_info);

	}: _(RawOrigin::Signed(acc.clone()), sale_id)
	verify {
		let sale_info = SaleInfo::<T>::get(0).unwrap();
		assert_eq!(sale_info.status, SaleStatus::Ended(end_block));
	}

	on_initialize {
		let p in 1 .. (T::MaxSalesPerBlock::get());

		let acc: T::AccountId = account("acc", 0, 0);
		for i in 0..p {
			let sale_id: SaleId = i as SaleId;
			let (sale_id, payment_asset_id, _) = initialize_crowdsale::<T>(acc.clone());

			// mint a participant some tokens; participate in the sale
			let participant = account("participant", 0, 0);
			let amount = 100_000_000;
			T::MultiCurrency::mint_into(payment_asset_id, &participant, amount).unwrap();
			CrowdSale::<T>::participate(RawOrigin::Signed(participant.clone()).into(), sale_id, amount).unwrap();
		}

		// update block no. to end the sale
		let current_block = <frame_system::Pallet<T>>::block_number();
		let end_block = SaleInfo::<T>::get(0).unwrap().duration.saturating_add(current_block);

		// Sanity check
		assert_eq!(SaleEndBlocks::<T>::get(end_block).unwrap().into_inner(), (0..p as u64).collect::<Vec<u64>>());

	}:  {CrowdSale::<T>::on_initialize(end_block.into());}
	verify {
		assert_eq!(SaleEndBlocks::<T>::get(end_block), None);
	}

	on_initialize_empty {
		let current_block = <frame_system::Pallet<T>>::block_number();
	}:  {CrowdSale::<T>::on_initialize(current_block.into());}
}

impl_benchmark_test_suite!(
	CrowdSale,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
