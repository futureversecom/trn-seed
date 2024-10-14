use crate::*;
use frame_support::{
	dispatch::{GetStorageVersion, MaxEncodedLen},
	traits::StorageVersion,
	DefaultNoBound,
};
use pallet_migration::WeightInfo;
use seed_primitives::migration::{MigrationStep, MigrationStepResult};
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;
use sp_std::marker::PhantomData;
use pallet_xls20::Xls20TokenId;

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
	type NewStorageValue = Xls20TokenId;      // [u8; 32]

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
	fn step(last_key: Option<Vec<u8>>, verbose: bool) -> MigrationStepResult {
		let mut iter = if let Some(last_key) = last_key {
			old::Xls20TokenMap::<T>::iter_from(last_key)
		} else {
			old::Xls20TokenMap::<T>::iter()
		};

		if let Some((key1, key2, old)) = iter.next() {
			if verbose {
				log::debug!(target: LOG_TARGET, "🦆 Migrating XLS-20 token_id: ({:?},{:?})", key1, key2);
			}
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

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade_step() -> Result<Vec<u8>, TryRuntimeError> {
		let sample: Vec<_> = old::Xls20TokenMap::<T>::iter().take(100).collect();
		log::debug!(target: LOG_TARGET, "🦆 Taking sample of {} token ids", sample.len());
		Ok(sample.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade_step(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let sample =
			<Vec<(CollectionUuid, SerialNumber, Self::OldStorageValue)> as Decode>::decode(&mut &state[..])
				.expect("🦆 pre_upgrade_step provides a valid state; qed");

		log::debug!(target: LOG_TARGET, "Validating sample of {} token_ids", sample.len());
		for (collection_id, serial_number, old) in sample {
			let new =
				pallet_xls20::Xls20TokenMap::<Runtime>::get(collection_id, serial_number).unwrap();
			let converted = Self::convert(old).expect("Will fail prior if invalid");
			ensure!(new == converted, "🦆 Invalid token_id migration");
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::migrations::tests::new_test_ext;
	use hex_literal::hex;

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
}
