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

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use crate::Pallet as Sylo;

use alloc::string::{String, ToString};
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, BoundedVec};
use frame_system::RawOrigin;
use sp_core::H160;

const MAX_SERVICE_ENDPOINTS: u32 = 10;
const STRING_LIMIT: u32 = 250;
const MAX_RESOLVERS: u32 = 10;
const MAX_TAGS: u32 = 10;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId
where
	T::AccountId: From<H160>,
{
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

pub fn bounded_string<T: Config>(name: &str) -> BoundedVec<u8, <T as Config>::StringLimit> {
	BoundedVec::truncate_from(name.as_bytes().to_vec())
}

pub fn max_bounded_string<T: Config>(bound: u32) -> BoundedVec<u8, <T as Config>::StringLimit> {
	let mut max_string = BoundedVec::new();
	for _ in 1..bound {
		max_string.force_push(b'a');
	}
	max_string
}

pub fn setup_resolver<T: Config>(
	caller: T::AccountId,
	identifier: BoundedVec<u8, <T as Config>::StringLimit>,
) -> BoundedVec<u8, <T as Config>::StringLimit> {
	let service_endpoints = BoundedVec::truncate_from(vec![bounded_string::<T>(
		"https://service-endpoint.one.two.three",
	)]);

	assert_ok!(Sylo::<T>::register_resolver(
		RawOrigin::Signed(caller).into(),
		identifier.clone(),
		service_endpoints,
	));

	return identifier;
}

pub fn setup_validation_record<T: Config>(
	caller: T::AccountId,
) -> BoundedVec<u8, <T as Config>::StringLimit> {
	let data_id = bounded_string::<T>("data-id");
	let resolvers = BoundedVec::new();
	let data_type = bounded_string::<T>("data-type");
	let tags = BoundedVec::new();
	let checksum = H256::from_low_u64_be(123);

	assert_ok!(Sylo::<T>::create_validation_record(
		RawOrigin::Signed(caller).into(),
		data_id.clone(),
		resolvers,
		data_type,
		tags,
		checksum,
	));

	return data_id;
}

benchmarks! {
	where_clause { where <T as frame_system::Config>::AccountId: From<sp_core::H160> + Into<sp_core::H160> }

	set_payment_asset {}: _(RawOrigin::Root, 24)

	set_sylo_resolver_method {}: _(RawOrigin::Root, bounded_string::<T>("sylo-resolver-method"))

	register_resolver {
		let p in 1 .. STRING_LIMIT;
		let q in 1 .. MAX_SERVICE_ENDPOINTS;

		let alice = account::<T>("Alice");

		let identifier = max_bounded_string::<T>(p);

		let mut service_endpoints = BoundedVec::new();
		for _ in 1..q {
			service_endpoints.force_push(max_bounded_string::<T>(p));
		}
	}: _(origin::<T>(&alice), identifier.clone(), service_endpoints.clone())
	verify {
		assert_eq!(Resolvers::<T>::get(identifier), Some(Resolver {
			controller: alice, service_endpoints
		}));
	}

	update_resolver {
		let p in 1 .. STRING_LIMIT;
		let q in 1 .. MAX_SERVICE_ENDPOINTS;

		let alice = account::<T>("Alice");

		let identifier = setup_resolver::<T>(alice.clone(), bounded_string::<T>("sylo-data-resolver"));

		let mut service_endpoints = BoundedVec::new();
		for _ in 1..q {
			service_endpoints.force_push(max_bounded_string::<T>(p));
		}
	}: _(origin::<T>(&alice), identifier.clone(), service_endpoints.clone())
	verify {
		assert_eq!(Resolvers::<T>::get(identifier), Some(Resolver {
			controller: alice, service_endpoints
		}));
	}

	unregister_resolver {
		let alice = account::<T>("Alice");

		let identifier = setup_resolver::<T>(alice.clone(), bounded_string::<T>("sylo-data-resolver"));
	}: _(origin::<T>(&alice), identifier.clone())
	verify {
		assert_eq!(Resolvers::<T>::get(identifier), None);
	}

	create_validation_record {
		let p in 1 .. STRING_LIMIT;
		let q in 1 .. MAX_RESOLVERS;
		let r in 1 .. MAX_TAGS;

		let alice = account::<T>("Alice");

		let data_id = bounded_string::<T>("data-id");

		let mut resolvers = BoundedVec::new();
		for i in 1 .. q {
			// create a maximum sized resolver id that is unique to each
			// resolver
			let mut resolver_id = String::from("sylo-resolver");
			resolver_id.push_str(i.to_string().as_str());
			let mut resolver_id = bounded_string::<T>(resolver_id.as_str());
			let id_len = <usize as TryInto<u32>>::try_into(resolver_id.len()).unwrap();
			if id_len < p {
				let max_affix = max_bounded_string::<T>(p - id_len);
				resolver_id.try_append(&mut max_affix.to_vec()).unwrap();
			};
			let resolver_id = setup_resolver::<T>(alice.clone(), resolver_id);
			resolvers.force_push(ResolverId {
				method: max_bounded_string::<T>(p),
				identifier: resolver_id,
			});
		}

		let data_type = max_bounded_string::<T>(p);

		let mut tags = BoundedVec::new();
		for _ in 1 .. r {
			tags.force_push(max_bounded_string::<T>(p));
		}

		let checksum = H256::from_low_u64_be(123);

		let block: BlockNumberFor<T> = 1_u32.into();
	}: _(origin::<T>(&alice), data_id.clone(), resolvers.clone(), data_type.clone(), tags.clone(), checksum.clone())
	verify {
		assert_eq!(ValidationRecords::<T>::get(&alice, &data_id), Some(ValidationRecord {
			author: alice,
			resolvers: resolvers,
			data_type: data_type,
			tags: tags,
			entries: BoundedVec::truncate_from(vec![ValidationEntry {
				checksum,
				block,
			}]),
		}));
	}

	add_validation_record_entry {
		let alice = account::<T>("Alice");

		let data_id = setup_validation_record::<T>(alice.clone());

		let checksum = H256::from_low_u64_be(123);
	}: _(origin::<T>(&alice), data_id.clone(), checksum.clone())
	verify {
		assert_eq!(ValidationRecords::<T>::get(&alice, &data_id), Some(ValidationRecord {
			author: alice,
			resolvers: BoundedVec::new(),
			data_type: bounded_string::<T>("data-type"),
			tags: BoundedVec::new(),
			entries: BoundedVec::truncate_from(vec![ValidationEntry {
				checksum,
				block: 0_u32.into(),
			}, ValidationEntry {
				checksum,
				block: 1_u32.into(),
			}]),
		}));
	}

	update_validation_record {
		let p in 1 .. STRING_LIMIT;
		let q in 1 .. MAX_RESOLVERS;
		let r in 1 .. MAX_TAGS;

		let alice = account::<T>("Alice");

		let data_id = setup_validation_record::<T>(alice.clone());

		let mut resolvers = BoundedVec::new();
		for i in 1 .. q {
			// create a maximum sized resolver id that is unique to each
			// resolver
			let mut resolver_id = String::from("sylo-resolver");
			resolver_id.push_str(i.to_string().as_str());
			let mut resolver_id = bounded_string::<T>(resolver_id.as_str());
			let id_len = <usize as TryInto<u32>>::try_into(resolver_id.len()).unwrap();
			if id_len < p {
				let max_affix = max_bounded_string::<T>(p - id_len);
				resolver_id.try_append(&mut max_affix.to_vec()).unwrap();
			};

			let resolver_id = setup_resolver::<T>(alice.clone(), resolver_id);
			resolvers.force_push(ResolverId {
				method: max_bounded_string::<T>(p),
				identifier: resolver_id,
			});
		}

		let data_type = max_bounded_string::<T>(p);

		let mut tags = BoundedVec::new();
		for _ in 1 .. r {
			tags.force_push(max_bounded_string::<T>(p));
		}

		let block: BlockNumberFor<T> = 1_u32.into();
	}: _(origin::<T>(&alice), data_id.clone(), Some(resolvers.clone()), Some(data_type.clone()), Some(tags.clone()))
	verify {
		assert_eq!(ValidationRecords::<T>::get(&alice, &data_id), Some(ValidationRecord {
			author: alice,
			resolvers: resolvers,
			data_type: data_type,
			tags: tags,
			entries: BoundedVec::truncate_from(vec![ValidationEntry {
				checksum: H256::from_low_u64_be(123),
				block,
			}]),
		}));
	}

	delete_validation_record {
		let alice = account::<T>("Alice");

		let data_id = setup_validation_record::<T>(alice.clone());
	}: _(origin::<T>(&alice), data_id.clone())
	verify {
		assert_eq!(ValidationRecords::<T>::get(&alice, &data_id), None);
	}
}

impl_benchmark_test_suite!(
	Sylo,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
