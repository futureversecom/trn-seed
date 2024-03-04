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

use super::*;
#[allow(unused_imports)]
use crate::Pallet as CrowdSale;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
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

		// validate event emitted
		frame_system::Pallet::<T>::assert_last_event(
			<T as pallet::Config>::RuntimeEvent::from(Event::CrowdsaleCreated {
				sale_id: 0,
				info: SaleInfo::<T>::get(0).unwrap(),
			}).into()
		);
  }

}

impl_benchmark_test_suite!(
	CrowdSale,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
