use crate::*;
use frame_support::{
	dispatch::{GetStorageVersion, MaxEncodedLen},
	traits::StorageVersion,
	DefaultNoBound,
};
use pallet_migration::WeightInfo;
use pallet_xls20::Xls20TokenId;
use seed_primitives::migration::{MigrationStep, MigrationStepResult};
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;
use sp_std::marker::PhantomData;

#[allow(dead_code)]
pub(crate) const LOG_TARGET: &str = "migration";

mod old {
	use super::*;
	use frame_support::{storage_alias, Twox64Concat};

	/// Old Xls20TokenId is 64 bytes in length instead of 32.
	pub type Xls20TokenId = [u8; 64];

	#[storage_alias]
	pub type Xls20TokenMap<T: pallet_xls20::Config> = StorageDoubleMap<
		pallet_xls20::Pallet<T>,
		Twox64Concat,
		CollectionUuid,
		Twox64Concat,
		SerialNumber,
		Xls20TokenId,
	>;
}

#[derive(Encode, Decode, MaxEncodedLen, DefaultNoBound)]
pub struct Xls20Migration<T: pallet_xls20::Config> {
	phantom: PhantomData<T>,
}

impl<T: pallet_xls20::Config + pallet_migration::Config> MigrationStep for Xls20Migration<T> {
	const TARGET_VERSION: u16 = 1;

	type OldStorageValue = old::Xls20TokenId; // [u8; 64]
	type NewStorageValue = Xls20TokenId; // [u8; 32]

	fn version_check() -> bool {
		Xls20::on_chain_storage_version() == Self::TARGET_VERSION
	}

	fn on_complete() {
		StorageVersion::new(Self::TARGET_VERSION).put::<Xls20>();
	}

	fn max_step_weight() -> Weight {
		<T as pallet_migration::Config>::WeightInfo::current_migration_step()
	}

	/// Convert from 64 byte utf-8 array to 32 byte array
	fn convert(old: Self::OldStorageValue) -> Result<Self::NewStorageValue, &'static str> {
		let hex_str = core::str::from_utf8(&old).map_err(|_| "Invalid UTF-8")?;
		let bytes = hex::decode(hex_str).map_err(|_| "Invalid hex data")?;
		let mut new = [0u8; 32];
		new.copy_from_slice(&bytes);
		Ok(new)
	}

	/// Migrate one token
	fn step(last_key: Option<Vec<u8>>) -> MigrationStepResult {
		let mut iter = if let Some(last_key) = last_key {
			old::Xls20TokenMap::<T>::iter_from(last_key)
		} else {
			old::Xls20TokenMap::<T>::iter()
		};

		if let Some((key1, key2, old)) = iter.next() {
			match Self::convert(old) {
				Ok(new_value) => {
					pallet_xls20::Xls20TokenMap::<T>::insert(key1, key2, new_value);
				},
				Err(e) => {
					// Remove the invalid value if we encounter an error during conversion
					log::error!(target: LOG_TARGET, "🦆 Error migrating token_id ({:?},{:?}) : {:?}", key1, key2, e);
					pallet_xls20::Xls20TokenMap::<T>::remove(key1, key2);
				},
			}
			let last_key = old::Xls20TokenMap::<T>::hashed_key_for(key1, key2);
			MigrationStepResult::continue_step(Self::max_step_weight(), last_key)
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
	use hex_literal::hex;

	/// Helper function to manually insert fake data into storage map
	fn insert_old_data(token_id: TokenId, old_value: old::Xls20TokenId) {
		let mut key = Twox64Concat::hash(&(token_id.0).encode());
		let key_2 = Twox64Concat::hash(&(token_id.1).encode());
		key.extend_from_slice(&key_2);
		Map::unsafe_storage_put::<old::Xls20TokenId>(b"Xls20", b"Xls20TokenMap", &key, old_value);
	}

	#[test]
	fn convert_works() {
		new_test_ext().execute_with(|| {
			let old: [u8; 64] = "000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66"
				.as_bytes()
				.try_into()
				.unwrap();
			let expected: [u8; 32] =
				hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66");
			let new = Xls20Migration::<Runtime>::convert(old).unwrap();
			assert_eq!(new, expected);
		});
	}

	#[test]
	fn convert_works_explicit() {
		new_test_ext().execute_with(|| {
			// Original string: "000800003AE03CAAE14B04F03ACC3DB34EE0B13362C533A016E5C2F800000001"
			let old: [u8; 64] = [
				48, 48, 48, 56, 48, 48, 48, 48, 51, 65, 69, 48, 51, 67, 65, 65, 69, 49, 52, 66, 48,
				52, 70, 48, 51, 65, 67, 67, 51, 68, 66, 51, 52, 69, 69, 48, 66, 49, 51, 51, 54, 50,
				67, 53, 51, 51, 65, 48, 49, 54, 69, 53, 67, 50, 70, 56, 48, 48, 48, 48, 48, 48, 48,
				49,
			];
			//  Manually convert above u8 array to hex array
			//  0,  0,  0,  8,  0,  0,  0,  0,  3,  A,  E,  0,  3,  C,  A,  A,  E,  1,  4,  B ...
			//  0x00,   0x08,   0x00,   0x00,   0x3A,   0xE0,   0x3C,   0xAA,   0xE1,   0x4B  ...
			//  0,      8,      0,      0,      58,     224,    60,     170,    225,    75    ...

			let expected: [u8; 32] = [
				0, 8, 0, 0, 58, 224, 60, 170, 225, 75, 4, 240, 58, 204, 61, 179, 78, 224, 177, 51,
				98, 197, 51, 160, 22, 229, 194, 248, 0, 0, 0, 1,
			];
			let new = Xls20Migration::<Runtime>::convert(old).unwrap();
			assert_eq!(new, expected);
		});
	}

	#[test]
	fn migrate_single_step() {
		new_test_ext().execute_with(|| {
			let old: [u8; 64] = "000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66"
				.as_bytes()
				.try_into()
				.unwrap();
			let token_id: TokenId = (1, 2);
			insert_old_data(token_id, old);

			let result = Xls20Migration::<Runtime>::step(None);
			assert!(!result.is_finished());
			let expected: [u8; 32] =
				hex!("000b013a95f14b0e44f78a264e41713c64b5f89242540ee2bc8b858e00000d66");
			let new = pallet_xls20::Xls20TokenMap::<Runtime>::get(token_id.0, token_id.1).unwrap();
			assert_eq!(new, expected);

			// Attempting to perform one more step should return Finished
			let last_key = result.last_key;
			let result = Xls20Migration::<Runtime>::step(last_key.clone());
			assert!(result.is_finished());
		});
	}

	#[test]
	fn migrate_many_steps() {
		new_test_ext().execute_with(|| {
			// Insert 100 tokens in 10 different collections
			let collection_count = 10;
			let token_count = 100;
			for i in 0..collection_count {
				for j in 0..token_count {
					let token_id: TokenId = (i, j);
					// insert collection_id and serial_number into first 2 bytes of old
					let string = format!("{:0>8}{:0>8}{:0>48}", token_id.0.to_string(), token_id.1.to_string(), 0);
					let old: [u8; 64] = string.as_bytes().try_into().unwrap();
					insert_old_data(token_id, old);
				}
			}

			// Perform migration
			let mut last_key = None;
			for _ in 0..collection_count * token_count {
				let result = Xls20Migration::<Runtime>::step(last_key.clone());
				assert!(!result.is_finished());
				last_key = result.last_key;
			}
			// One last step to finish migration
			let result = Xls20Migration::<Runtime>::step(last_key.clone());
			assert!(result.is_finished());

			// Check that all tokens have been migrated
			for i in 0..collection_count {
				for j in 0..token_count {
					let token_id: TokenId = (i, j);
					let string = format!("{:0>8}{:0>8}{:0>48}", token_id.0.to_string(), token_id.1.to_string(), 0);
					let old: [u8; 64] = string.as_bytes().try_into().unwrap();
					let expected = Xls20Migration::<Runtime>::convert(old).unwrap();
					let new = pallet_xls20::Xls20TokenMap::<Runtime>::get(token_id.0, token_id.1)
						.unwrap();
					assert_eq!(new, expected);
				}
			}
		});
	}
}
