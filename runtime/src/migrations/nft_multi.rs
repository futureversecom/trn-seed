use crate::*;
use codec::{Decode, Encode, MaxEncodedLen};
use core::fmt::Debug;
use frame_support::traits::IsType;
use frame_support::{dispatch::GetStorageVersion, traits::StorageVersion, DefaultNoBound};
use frame_support::{
	storage_alias, CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound, Twox64Concat,
};
use pallet_migration::WeightInfo;
use pallet_nft::{CollectionInformation, TokenInformation};
use scale_info::TypeInfo;
use seed_pallet_common::utils::TokenUtilityFlags as TokenFlags;
use seed_primitives::migration::{MigrationStep, MigrationStepResult};
use seed_primitives::{
	CollectionUuid, CrossChainCompatibility, MetadataScheme, OriginChain, RoyaltiesSchedule,
	SerialNumber, TokenCount,
};
use sp_runtime::{BoundedVec, DispatchResult};

use sp_std::marker::PhantomData;

#[allow(dead_code)]
pub(crate) const LOG_TARGET: &str = "migration";

/// How many tokens max can we migrate per step?
const MAX_TOKENS_PER_STEP: u32 = 50;

mod old {
	use super::*;
	use frame_support::pallet_prelude::ValueQuery;
	use seed_primitives::TokenLockReason;

	#[storage_alias]
	pub type CollectionInfo<T: pallet_nft::Config> = StorageMap<
		pallet_nft::Pallet<T>,
		Twox64Concat,
		CollectionUuid,
		CollectionInformation<
			<T as frame_system::Config>::AccountId,
			<T as pallet_nft::Config>::MaxTokensPerCollection,
			<T as pallet_nft::Config>::StringLimit,
		>,
	>;

	#[storage_alias]
	pub type TokenLocks<T: pallet_nft::Config> =
		StorageMap<pallet_nft::Pallet<T>, Twox64Concat, TokenId, TokenLockReason>;

	/// Map from a token_id to transferable and burn authority flags
	#[storage_alias]
	pub type TokenUtilityFlags<T: pallet_nft::Config> =
		StorageMap<pallet_nft::Pallet<T>, Twox64Concat, TokenId, TokenFlags, ValueQuery>;

	/// Information related to a specific collection
	#[derive(
		PartialEqNoBound, RuntimeDebugNoBound, CloneNoBound, Encode, Decode, TypeInfo, MaxEncodedLen,
	)]
	#[codec(mel_bound(AccountId: MaxEncodedLen))]
	#[scale_info(skip_type_params(MaxTokensPerCollection, StringLimit))]
	pub struct CollectionInformation<AccountId, MaxTokensPerCollection, StringLimit>
	where
		AccountId: Debug + PartialEq + Clone,
		MaxTokensPerCollection: Get<u32>,
		StringLimit: Get<u32>,
	{
		/// The owner of the collection
		pub owner: AccountId,
		/// A human friendly name
		pub name: BoundedVec<u8, StringLimit>,
		/// Collection metadata reference scheme
		pub metadata_scheme: MetadataScheme,
		/// configured royalties schedule
		pub royalties_schedule: Option<RoyaltiesSchedule<AccountId>>,
		/// Maximum number of tokens allowed in a collection
		pub max_issuance: Option<TokenCount>,
		/// The chain in which the collection was minted originally
		pub origin_chain: OriginChain,
		/// The next available serial_number
		pub next_serial_number: SerialNumber,
		/// the total count of tokens in this collection
		pub collection_issuance: TokenCount,
		/// This collections compatibility with other chains
		pub cross_chain_compatibility: CrossChainCompatibility,
		/// All serial numbers owned by an account in a collection
		pub owned_tokens: BoundedVec<
			old::TokenOwnership<AccountId, MaxTokensPerCollection>,
			MaxTokensPerCollection,
		>,
	}

	#[derive(
		PartialEqNoBound, RuntimeDebugNoBound, Decode, Encode, CloneNoBound, TypeInfo, MaxEncodedLen,
	)]
	#[codec(mel_bound(AccountId: MaxEncodedLen))]
	#[scale_info(skip_type_params(MaxTokensPerCollection))]
	pub struct TokenOwnership<AccountId, MaxTokensPerCollection>
	where
		AccountId: Debug + PartialEq + Clone,
		MaxTokensPerCollection: Get<u32>,
	{
		pub owner: AccountId,
		pub owned_serials: BoundedVec<SerialNumber, MaxTokensPerCollection>,
	}
}

/// Convert from old CollectionInfo type to new type
/// Returns a bool if the CollectionInfo has been completely migrated, if not, this function
/// will re-insert the old CollectionInfo into the storage
fn convert<T: pallet_nft::Config>(
	collection_id: CollectionUuid,
	old: &mut old::CollectionInformation<T::AccountId, T::MaxTokensPerCollection, T::StringLimit>,
) -> (bool, TokenCount) {
	log::debug!(target: LOG_TARGET, "🦆 Migrating collection_id {}", collection_id);
	// Is the collection info fully migrated
	let mut completed = false;
	let mut migrated_token_count: TokenCount = 0;

	// For simplicity, we will only migrate max 50 tokens from 1 account at a time
	if let Some(mut ownership) = old.owned_tokens.pop() {
		let mut serial_numbers = ownership.owned_serials.clone();
		// take at max 10 from serial_numbers
		migrated_token_count = serial_numbers.len().min(MAX_TOKENS_PER_STEP as usize) as TokenCount;
		let serials_to_migrate = serial_numbers.drain(..migrated_token_count as usize).collect();
		log::debug!(target: LOG_TARGET, "🦆 Migrating {:?} tokens for owner: {:?}", serials_to_migrate, ownership.owner);

		for serial_number in &serials_to_migrate {
			let token_locks = old::TokenLocks::<T>::take((collection_id, serial_number));
			let token_utility_flags =
				old::TokenUtilityFlags::<T>::take((collection_id, serial_number));

			let token_info = TokenInformation {
				owner: ownership.owner.clone(),
				lock_status: token_locks,
				utility_flags: token_utility_flags,
			};
			pallet_nft::TokenInfo::<T>::insert(collection_id, serial_number, token_info);
		}

		// Update OwnedTokens with the migrated serials
		let _ = pallet_nft::OwnedTokens::<T>::try_mutate(
			&ownership.owner,
			collection_id,
			|owned_serials| -> DispatchResult {
				match owned_serials.as_mut() {
					Some(owned_serials) => {
						for serial_number in serials_to_migrate {
							// Force push is safe as the existing bound is the same as the new bound
							owned_serials.force_push(serial_number);
						}
					},
					None => {
						*owned_serials = Some(BoundedVec::truncate_from(serials_to_migrate));
					},
				}
				Ok(())
			},
		);

		// If migration of this user's serial numbers is not complete,
		// reinstate remaining serials into ownership
		if serial_numbers.is_empty() {
			log::debug!(target: LOG_TARGET, "🦆 Migration complete for owner: {:?}", ownership.owner);
			if old.owned_tokens.is_empty() {
				completed = true;
			}
		} else {
			log::debug!(target: LOG_TARGET, "🦆 Migration on-going for owner: {:?}", ownership.owner);
			ownership.owned_serials = serial_numbers;
			old.owned_tokens.force_push(ownership);
		}
	} else {
		// No serials to migrate, therefore complete
		completed = true;
	}

	if completed {
		log::debug!(target: LOG_TARGET, "🦆 Migration complete for collection: {}", collection_id);
		// Migration for this collection_info is complete.
		// Delete the old collection_info and insert the new one
		old::CollectionInfo::<T>::remove(collection_id);
		let new_collection_info = CollectionInformation {
			owner: old.owner.clone(),
			name: old.name.clone(),
			metadata_scheme: old.metadata_scheme.clone(),
			royalties_schedule: old.royalties_schedule.clone(),
			max_issuance: old.max_issuance.clone(),
			origin_chain: old.origin_chain.clone(),
			next_serial_number: old.next_serial_number.clone(),
			collection_issuance: old.collection_issuance.clone(),
			cross_chain_compatibility: old.cross_chain_compatibility.clone(),
		};
		pallet_nft::CollectionInfo::<T>::insert(collection_id, new_collection_info);
	} else {
		log::debug!(target: LOG_TARGET, "🦆 Migration on-going for collection: {}", collection_id);
		// Re-insert the old collection_info
		old::CollectionInfo::<T>::insert(collection_id, old);
	}

	(completed, migrated_token_count)
}

#[derive(Encode, Decode, MaxEncodedLen, DefaultNoBound)]
pub struct NftMigration<T: pallet_nft::Config> {
	phantom: PhantomData<T>,
}

impl<T: pallet_nft::Config + pallet_migration::Config> MigrationStep for NftMigration<T> {
	const TARGET_VERSION: u16 = 9;

	fn version_check() -> bool {
		Nft::on_chain_storage_version() == Self::TARGET_VERSION
	}

	fn on_complete() {
		StorageVersion::new(Self::TARGET_VERSION).put::<Nft>();
	}

	fn max_step_weight() -> Weight {
		<T as pallet_migration::Config>::WeightInfo::current_migration_step(MAX_TOKENS_PER_STEP)
	}

	/// Migrate one token
	fn step(last_key: Option<Vec<u8>>) -> MigrationStepResult {
		let mut iter = if let Some(last_key) = last_key.clone() {
			old::CollectionInfo::<T>::iter_from(last_key)
		} else {
			old::CollectionInfo::<T>::iter()
		};

		if let Some((key, mut old)) = iter.next() {
			let (completed, migrated_count) = convert::<T>(key, old.into_mut());

			// If we have completed the migration for this collection, we can move on to the next one
			let last_key = match completed {
				true => Some(old::CollectionInfo::<T>::hashed_key_for(key)),
				false => last_key,
			};
			MigrationStepResult::continue_step(
				<T as pallet_migration::Config>::WeightInfo::current_migration_step(migrated_count),
				last_key,
			)
		} else {
			log::debug!(target: LOG_TARGET, "🦆 No more tokens to migrate");
			MigrationStepResult::finish_step(Self::max_step_weight())
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::migrations::{tests::new_test_ext, Map};
	use frame_support::{StorageHasher, Twox64Concat};
	use seed_pallet_common::test_prelude::create_account;
	use seed_pallet_common::utils::TokenBurnAuthority;
	use seed_primitives::{ListingId, TokenLockReason};
	type AccountId = <Runtime as frame_system::Config>::AccountId;

	/// Helper function to manually insert fake data into storage map
	fn insert_old_data(
		collection_id: CollectionUuid,
		old_value: old::CollectionInformation<
			AccountId,
			MaxTokensPerCollection,
			CollectionNameStringLimit,
		>,
	) {
		let key = Twox64Concat::hash(&(collection_id).encode());
		Map::unsafe_storage_put::<
			old::CollectionInformation<
				AccountId,
				MaxTokensPerCollection,
				CollectionNameStringLimit,
			>,
		>(b"Nft", b"CollectionInfo", &key, old_value);
	}

	fn insert_old_token_flags(token_id: TokenId, token_flags: TokenFlags) {
		let key = Twox64Concat::hash(&(token_id).encode());
		Map::unsafe_storage_put::<TokenFlags>(b"Nft", b"TokenUtilityFlags", &key, token_flags);
	}

	fn insert_old_token_locks(token_id: TokenId, token_lock_reason: TokenLockReason) {
		let key = Twox64Concat::hash(&(token_id).encode());
		Map::unsafe_storage_put::<TokenLockReason>(b"Nft", b"TokenLocks", &key, token_lock_reason);
	}

	#[test]
	fn migrate_single_step() {
		new_test_ext().execute_with(|| {
			let account = create_account(123);
			let serials: Vec<SerialNumber> = (1..=MAX_TOKENS_PER_STEP).collect();
			let old_token_ownership = old::TokenOwnership::<AccountId, MaxTokensPerCollection> {
				owner: account,
				owned_serials: BoundedVec::<SerialNumber, MaxTokensPerCollection>::truncate_from(
					serials.clone(),
				),
			};
			// Add token locks and TokenFlags for each serial
			for serial in serials.clone() {
				let token_lock_reason = TokenLockReason::Listed(serial as ListingId);
				insert_old_token_locks((123, serial), token_lock_reason);
				let token_flags = TokenFlags {
					transferable: false,
					burn_authority: Some(TokenBurnAuthority::Both),
				};
				insert_old_token_flags((123, serial), token_flags);
				old::TokenUtilityFlags::<Runtime>::insert((123, serial), token_flags);
			}

			let old = old::CollectionInformation {
				owner: create_account(126),
				name: BoundedVec::<u8, CollectionNameStringLimit>::truncate_from(vec![1, 2, 3, 4]),
				metadata_scheme: MetadataScheme::try_from(b"metadata".as_slice()).unwrap(),
				royalties_schedule: None,
				max_issuance: Some(500),
				origin_chain: OriginChain::Root,
				next_serial_number: 2,
				collection_issuance: 5,
				cross_chain_compatibility: CrossChainCompatibility::default(),
				owned_tokens: BoundedVec::<
					old::TokenOwnership<AccountId, MaxTokensPerCollection>,
					MaxTokensPerCollection,
				>::truncate_from(vec![old_token_ownership.clone()]),
			};
			let collection_id = 123;
			insert_old_data(collection_id, old.clone());

			// Perform 1 step to migrate the user's tokens
			let result = NftMigration::<Runtime>::step(None);
			assert!(!result.is_finished());

			// Check TokenInfo and OwnedTokens
			for serial_number in serials.clone() {
				let token_lock_reason = TokenLockReason::Listed(serial_number as ListingId);
				let token_flags = TokenFlags {
					transferable: false,
					burn_authority: Some(TokenBurnAuthority::Both),
				};
				let expected_token_info = TokenInformation {
					owner: account,
					lock_status: Some(token_lock_reason),
					utility_flags: token_flags,
				};
				// assert!(!old::TokenLocks::<Runtime>::contains_key((123, serial_number)));
				// assert!(!old::TokenUtilityFlags::<Runtime>::contains_key((123, serial_number)));
				let token_info =
					pallet_nft::TokenInfo::<Runtime>::get(collection_id, serial_number).unwrap();
				assert_eq!(expected_token_info, token_info);
			}
			let owned_tokens =
				pallet_nft::OwnedTokens::<Runtime>::get(account, collection_id).unwrap();
			assert_eq!(owned_tokens.into_inner(), serials);

			// Collection info should now be the new format
			let expected_collection_info = CollectionInformation {
				owner: create_account(126),
				name: BoundedVec::<u8, CollectionNameStringLimit>::truncate_from(vec![1, 2, 3, 4]),
				metadata_scheme: MetadataScheme::try_from(b"metadata".as_slice()).unwrap(),
				royalties_schedule: None,
				max_issuance: Some(500),
				origin_chain: OriginChain::Root,
				next_serial_number: 2,
				collection_issuance: 5,
				cross_chain_compatibility: CrossChainCompatibility::default(),
			};
			let new_collection_info =
				pallet_nft::CollectionInfo::<Runtime>::get(collection_id).unwrap();
			assert_eq!(new_collection_info, expected_collection_info);

			// Attempting to perform one more step should return Finished
			let last_key = result.last_key;
			let result = NftMigration::<Runtime>::step(last_key.clone());
			assert!(result.is_finished());
		});
	}

	#[test]
	fn migrate_two_owners() {
		new_test_ext().execute_with(|| {
			let account_1 = create_account(123);
			let serials_1 = vec![1, 2, 3];
			let old_token_ownership_1 = old::TokenOwnership::<AccountId, MaxTokensPerCollection> {
				owner: account_1,
				owned_serials: BoundedVec::<SerialNumber, MaxTokensPerCollection>::truncate_from(
					serials_1.clone(),
				),
			};

			let account_2 = create_account(126);
			let serials_2 = vec![6, 7, 8, 9];
			let old_token_ownership_2 = old::TokenOwnership::<AccountId, MaxTokensPerCollection> {
				owner: account_2,
				owned_serials: BoundedVec::<SerialNumber, MaxTokensPerCollection>::truncate_from(
					serials_2.clone(),
				),
			};

			let old = old::CollectionInformation {
				owner: create_account(126),
				name: BoundedVec::<u8, CollectionNameStringLimit>::truncate_from(vec![1, 2, 3, 4]),
				metadata_scheme: MetadataScheme::try_from(b"metadata".as_slice()).unwrap(),
				royalties_schedule: None,
				max_issuance: Some(500),
				origin_chain: OriginChain::Root,
				next_serial_number: 2,
				collection_issuance: 5,
				cross_chain_compatibility: CrossChainCompatibility::default(),
				owned_tokens: BoundedVec::<
					old::TokenOwnership<AccountId, MaxTokensPerCollection>,
					MaxTokensPerCollection,
				>::truncate_from(vec![
					old_token_ownership_1.clone(),
					old_token_ownership_2,
				]),
			};
			let collection_id = 123;
			insert_old_data(collection_id, old.clone());

			// Perform 1 step to migrate the last user (user_2)
			let result = NftMigration::<Runtime>::step(None);
			assert!(!result.is_finished());

			// Check user 2 tokens
			for serial_number in serials_2.clone() {
				let token_info =
					pallet_nft::TokenInfo::<Runtime>::get(collection_id, serial_number).unwrap();
				assert_eq!(token_info.owner, account_2);
			}
			let owned_tokens =
				pallet_nft::OwnedTokens::<Runtime>::get(account_2, collection_id).unwrap();
			assert_eq!(owned_tokens.into_inner(), serials_2);

			// Old collection info still exists in the old format, but only has one user left to migrate
			let old_collection_info = old::CollectionInfo::<Runtime>::get(collection_id).unwrap();
			let expected = old::CollectionInformation {
				owned_tokens: BoundedVec::<
					old::TokenOwnership<AccountId, MaxTokensPerCollection>,
					MaxTokensPerCollection,
				>::truncate_from(vec![old_token_ownership_1]),
				..old
			};
			assert_eq!(old_collection_info, expected);

			// Perform 1 more step to migrate the first user
			let result = NftMigration::<Runtime>::step(result.last_key);
			assert!(!result.is_finished());

			// Collection info should now be the new format
			let expected_collection_info = CollectionInformation {
				owner: create_account(126),
				name: BoundedVec::<u8, CollectionNameStringLimit>::truncate_from(vec![1, 2, 3, 4]),
				metadata_scheme: MetadataScheme::try_from(b"metadata".as_slice()).unwrap(),
				royalties_schedule: None,
				max_issuance: Some(500),
				origin_chain: OriginChain::Root,
				next_serial_number: 2,
				collection_issuance: 5,
				cross_chain_compatibility: CrossChainCompatibility::default(),
			};
			let new_collection_info =
				pallet_nft::CollectionInfo::<Runtime>::get(collection_id).unwrap();
			assert_eq!(new_collection_info, expected_collection_info);

			// Check user 1 tokens
			for serial_number in serials_1.clone() {
				let token_info =
					pallet_nft::TokenInfo::<Runtime>::get(collection_id, serial_number).unwrap();
				assert_eq!(token_info.owner, account_1);
			}
			let owned_tokens =
				pallet_nft::OwnedTokens::<Runtime>::get(account_1, collection_id).unwrap();
			assert_eq!(owned_tokens.into_inner(), serials_1);

			// Attempting to perform one more step should return Finished
			let last_key = result.last_key;
			let result = NftMigration::<Runtime>::step(last_key.clone());
			assert!(result.is_finished());
		});
	}

	#[test]
	fn migrate_many_collections() {
		new_test_ext().execute_with(|| {
			// Insert 100 collections
			let collection_count: CollectionUuid = 100;
			for i in 0..collection_count {
				let serials = vec![1, 2, 3, i + 4];
				// Add token locks and TokenFlags for each serial
				for serial in serials.clone() {
					println!("token: ({},{})", i, serial);
					let token_lock_reason = TokenLockReason::Listed(serial as ListingId);
					insert_old_token_locks((i, serial), token_lock_reason);
					let token_flags = TokenFlags {
						transferable: false,
						burn_authority: Some(TokenBurnAuthority::Both),
					};
					insert_old_token_flags((i, serial), token_flags);
				}

				// Insert token ownership and collection info
				let old_token_ownership = old::TokenOwnership::<AccountId, MaxTokensPerCollection> {
					owner: create_account(1 + i as u64),
					owned_serials:
						BoundedVec::<SerialNumber, MaxTokensPerCollection>::truncate_from(serials),
				};
				let old = old::CollectionInformation {
					owner: create_account(2 + i as u64),
					name: BoundedVec::<u8, CollectionNameStringLimit>::truncate_from(vec![
						1, 2, 3, 4,
					]),
					metadata_scheme: MetadataScheme::try_from(b"metadata".as_slice()).unwrap(),
					royalties_schedule: None,
					max_issuance: Some(i),
					origin_chain: OriginChain::Root,
					next_serial_number: i + 4,
					collection_issuance: i + 5,
					cross_chain_compatibility: CrossChainCompatibility::default(),
					owned_tokens: BoundedVec::<
						old::TokenOwnership<AccountId, MaxTokensPerCollection>,
						MaxTokensPerCollection,
					>::truncate_from(vec![old_token_ownership]),
				};
				insert_old_data(i, old);
			}

			// Perform migration
			let mut last_key = None;
			for _ in 0..collection_count {
				let result = NftMigration::<Runtime>::step(last_key.clone());
				assert!(!result.is_finished());
				last_key = result.last_key;
			}
			// One last step to finish migration
			let result = NftMigration::<Runtime>::step(last_key.clone());
			assert!(result.is_finished());

			// Check that all collections have been migrated
			for i in 0..collection_count {
				let new_collection_info = pallet_nft::CollectionInfo::<Runtime>::get(i).unwrap();
				let expected_collection_info = CollectionInformation {
					owner: create_account(2 + i as u64),
					name: BoundedVec::<u8, CollectionNameStringLimit>::truncate_from(vec![
						1, 2, 3, 4,
					]),
					metadata_scheme: MetadataScheme::try_from(b"metadata".as_slice()).unwrap(),
					royalties_schedule: None,
					max_issuance: Some(i),
					origin_chain: OriginChain::Root,
					next_serial_number: i + 4,
					collection_issuance: i + 5,
					cross_chain_compatibility: CrossChainCompatibility::default(),
				};
				assert_eq!(new_collection_info, expected_collection_info);

				// Check ownership
				let account = create_account(1 + i as u64);
				let serials = vec![1, 2, 3, i + 4];
				for serial_number in serials.clone() {
					let token_lock_reason = TokenLockReason::Listed(serial_number as ListingId);
					let token_flags = TokenFlags {
						transferable: false,
						burn_authority: Some(TokenBurnAuthority::Both),
					};
					let expected_token_info = TokenInformation {
						owner: account,
						lock_status: Some(token_lock_reason),
						utility_flags: token_flags,
					};
					assert!(!old::TokenLocks::<Runtime>::contains_key((i, serial_number)));
					assert!(!old::TokenUtilityFlags::<Runtime>::contains_key((i, serial_number)));
					let token_info =
						pallet_nft::TokenInfo::<Runtime>::get(i, serial_number).unwrap();
					assert_eq!(token_info, expected_token_info);
				}
				let owned_tokens = pallet_nft::OwnedTokens::<Runtime>::get(account, i).unwrap();
				assert_eq!(owned_tokens.into_inner(), serials);
			}
		});
	}

	#[test]
	fn migrate_owner_with_many_tokens() {
		new_test_ext().execute_with(|| {
			let account_1 = create_account(123);
			let serials_1: Vec<u32> = (1..=MAX_TOKENS_PER_STEP * 2).collect();
			let old_token_ownership_1 = old::TokenOwnership::<AccountId, MaxTokensPerCollection> {
				owner: account_1,
				owned_serials: BoundedVec::<SerialNumber, MaxTokensPerCollection>::truncate_from(
					serials_1.clone(),
				),
			};

			let old = old::CollectionInformation {
				owner: create_account(126),
				name: BoundedVec::<u8, CollectionNameStringLimit>::truncate_from(vec![1, 2, 3, 4]),
				metadata_scheme: MetadataScheme::try_from(b"metadata".as_slice()).unwrap(),
				royalties_schedule: None,
				max_issuance: Some(500),
				origin_chain: OriginChain::Root,
				next_serial_number: 2,
				collection_issuance: 5,
				cross_chain_compatibility: CrossChainCompatibility::default(),
				owned_tokens: BoundedVec::<
					old::TokenOwnership<AccountId, MaxTokensPerCollection>,
					MaxTokensPerCollection,
				>::truncate_from(vec![old_token_ownership_1.clone()]),
			};
			let collection_id = 123;
			insert_old_data(collection_id, old.clone());

			// Perform 1 step to migrate MAX_TOKENS_PER_STEP tokens
			let result = NftMigration::<Runtime>::step(None);
			assert!(!result.is_finished());

			// Check tokens
			let migrated_serials = (1..=MAX_TOKENS_PER_STEP).collect::<Vec<u32>>();
			for serial_number in migrated_serials.clone() {
				let token_info =
					pallet_nft::TokenInfo::<Runtime>::get(collection_id, serial_number).unwrap();
				assert_eq!(token_info.owner, account_1);
			}
			let owned_tokens =
				pallet_nft::OwnedTokens::<Runtime>::get(account_1, collection_id).unwrap();
			assert_eq!(owned_tokens.into_inner(), migrated_serials);

			// Old collection info still exists in the old format, but the number of serials is reduced
			let serials_to_migrate: Vec<u32> =
				(MAX_TOKENS_PER_STEP + 1..=MAX_TOKENS_PER_STEP * 2).collect();
			let old_token_ownership_1 = old::TokenOwnership::<AccountId, MaxTokensPerCollection> {
				owner: account_1,
				owned_serials: BoundedVec::<SerialNumber, MaxTokensPerCollection>::truncate_from(
					serials_to_migrate.clone(),
				),
			};
			let old_collection_info = old::CollectionInfo::<Runtime>::get(collection_id).unwrap();
			let expected = old::CollectionInformation {
				owned_tokens: BoundedVec::<
					old::TokenOwnership<AccountId, MaxTokensPerCollection>,
					MaxTokensPerCollection,
				>::truncate_from(vec![old_token_ownership_1]),
				..old
			};
			assert_eq!(old_collection_info, expected);

			// Perform 1 more step to migrate the rest of the serials
			let result = NftMigration::<Runtime>::step(result.last_key);
			assert!(!result.is_finished());

			// Check tokens
			let migrated_serials = (1..=MAX_TOKENS_PER_STEP * 2).collect::<Vec<u32>>();
			for serial_number in migrated_serials.clone() {
				let token_info =
					pallet_nft::TokenInfo::<Runtime>::get(collection_id, serial_number).unwrap();
				assert_eq!(token_info.owner, account_1);
			}
			let owned_tokens =
				pallet_nft::OwnedTokens::<Runtime>::get(account_1, collection_id).unwrap();
			assert_eq!(owned_tokens.into_inner(), migrated_serials);

			// Collection info should now be the new format
			let expected_collection_info = CollectionInformation {
				owner: create_account(126),
				name: BoundedVec::<u8, CollectionNameStringLimit>::truncate_from(vec![1, 2, 3, 4]),
				metadata_scheme: MetadataScheme::try_from(b"metadata".as_slice()).unwrap(),
				royalties_schedule: None,
				max_issuance: Some(500),
				origin_chain: OriginChain::Root,
				next_serial_number: 2,
				collection_issuance: 5,
				cross_chain_compatibility: CrossChainCompatibility::default(),
			};
			let new_collection_info =
				pallet_nft::CollectionInfo::<Runtime>::get(collection_id).unwrap();
			assert_eq!(new_collection_info, expected_collection_info);

			// Attempting to perform one more step should return Finished
			let last_key = result.last_key;
			let result = NftMigration::<Runtime>::step(last_key.clone());
			assert!(result.is_finished());
		});
	}
}
