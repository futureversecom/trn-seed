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
use crate::Pallet as Migration;
use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::StorageHasher;
use frame_system::RawOrigin;
use seed_pallet_common::utils::{TokenBurnAuthority, TokenUtilityFlags as TokenFlags};
use seed_primitives::{
	CollectionUuid, CrossChainCompatibility, ListingId, MetadataScheme, OriginChain,
	RoyaltiesSchedule, SerialNumber, TokenCount, TokenLockReason,
};

benchmarks! {
	where_clause { where T: pallet_nft::Config }

	// This benchmarks the weight of dispatching migrate to execute 1 `NoopMigraton` step
	migrate {
		let weight_limit = T::MaxMigrationWeight::get();
		Status::<T>::put(MigrationStatus::InProgress { steps_done: 0 });
		MigrationEnabled::<T>::put(true);
		let block_number: BlockNumberFor<T> = 1_u32.into();
	}: {
		Migration::<T>::migrate(block_number, weight_limit)
	} verify {
		assert_eq!( Status::<T>::get(), MigrationStatus::Completed);
	}

	current_migration_step {
		let p in 1 .. (50);

		MigrationEnabled::<T>::put(true);

		///Old Collection Info
		#[derive(Encode,Decode,TypeInfo)]
		pub struct OldCollectionInformation<AccountId, MaxTokensPerCollection, StringLimit>
		{
			pub owner: AccountId,
			pub name: BoundedVec<u8, StringLimit>,
			pub metadata_scheme: MetadataScheme,
			pub royalties_schedule: Option<RoyaltiesSchedule<AccountId>>,
			pub max_issuance: Option<TokenCount>,
			pub origin_chain: OriginChain,
			pub next_serial_number: SerialNumber,
			pub collection_issuance: TokenCount,
			pub cross_chain_compatibility: CrossChainCompatibility,
			pub owned_tokens:
				BoundedVec<OldTokenOwnership<AccountId, MaxTokensPerCollection>, MaxTokensPerCollection>,
		}

		#[derive(Decode,Encode,TypeInfo)]
		pub struct OldTokenOwnership<AccountId, MaxTokensPerCollection>
		{
			pub owner: AccountId,
			pub owned_serials: BoundedVec<SerialNumber, MaxTokensPerCollection>,
		}
		let serials = (1..=p).collect::<Vec<SerialNumber>>();
		let key = Twox64Concat::hash(&(1 as CollectionUuid).encode());
		let collection_info = OldCollectionInformation {
			owner: T::AccountId::from(bench_account("test", 0, 0)),
			name: BoundedVec::truncate_from(vec![1, 2, 3]),
			metadata_scheme: MetadataScheme::try_from(b"metadata".as_slice()).unwrap(),
			royalties_schedule: None,
			max_issuance: Some(100),
			origin_chain: OriginChain::Root,
			next_serial_number: 1,
			collection_issuance: 1,
			cross_chain_compatibility: CrossChainCompatibility::default(),
			owned_tokens: BoundedVec::truncate_from(vec![OldTokenOwnership {
				owner: T::AccountId::from(bench_account("test", 0, 0)),
				owned_serials: BoundedVec::truncate_from(serials.clone()),
			}]),
		};
		frame_support::migration::put_storage_value::<OldCollectionInformation<T::AccountId,T::MaxTokensPerCollection,T::StringLimit>> (b"Nft", b"CollectionInfo", &key, collection_info);

		// Insert data into TokenLocks and TokenUtilityFlags to benchmark worst case scenario
		for serial in serials {
			let token_id = (1 as CollectionUuid, serial);
			let token_lock_reason = TokenLockReason::Listed(1 as ListingId);
			let key = Twox64Concat::hash(&token_id.encode());
			frame_support::migration::put_storage_value::<TokenLockReason> (b"Nft", b"TokenLocks", &key, token_lock_reason);
			let token_flags = TokenFlags {
				transferable: true,
				burn_authority: Some(TokenBurnAuthority::Both),
			};
			frame_support::migration::put_storage_value::<TokenFlags> (b"Nft", b"TokenUtilityFlags", &key, token_flags);
		}

		Status::<T>::put(MigrationStatus::InProgress { steps_done: 0 });
	}: {
		// Call a single step to benchmark.
		// Note we can't verify this step as there is different implementations of CurrentMigration
		// in the mock and the runtime
		T::CurrentMigration::step(None);
	}

	enable_migration {
		let enabled = true;
	}: _(RawOrigin::Root, enabled)
	verify {
		assert!(MigrationEnabled::<T>::get());
	}

	set_block_delay {
		let delay = Some(10);
	}: _(RawOrigin::Root, delay)
	verify {
		assert_eq!(BlockDelay::<T>::get(), Some(10));
	}

	set_block_limit {
		let limit = 1000;
	}: _(RawOrigin::Root, limit)
	verify {
		assert_eq!(BlockLimit::<T>::get(), 1000);
	}
}

impl_benchmark_test_suite!(
	Migration,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
