use crate::*;
use frame_support::dispatch::{GetStorageVersion, MaxEncodedLen};
use frame_support::DefaultNoBound;
use pallet_migration::WeightInfo;
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

	type StorageKey = (CollectionUuid, SerialNumber);
	type OldStorageValue = [u8; 64];
	type NewStorageValue = [u8; 32];

	fn version_check() -> bool {
		Xls20::on_chain_storage_version() == Self::TARGET_VERSION
	}

	fn max_step_weight() -> Weight {
		<T as pallet_migration::Config>::WeightInfo::current_migration_step()
	}

	fn convert(old: Self::OldStorageValue) -> Self::NewStorageValue {
		let mut new_token_id = [0; 32];
		new_token_id.copy_from_slice(&old[..32]);
		new_token_id
	}

	fn step(last_key: Option<Self::StorageKey>) -> MigrationStepResult<Self::StorageKey> {
		let mut iter = if let Some(last_key) = last_key {
			old::Xls20TokenMap::<T>::iter_from(old::Xls20TokenMap::<T>::hashed_key_for(
				last_key.0, last_key.1,
			))
		} else {
			old::Xls20TokenMap::<T>::iter()
		};

		if let Some((key1, key2, old)) = iter.next() {
			// log::debug!(target: LOG_TARGET, "🦆 Migrating XLS-20 token_id: ({:?},{:?})", key1, key2);
			let new_value = Self::convert(old);
			pallet_xls20::Xls20TokenMap::<Runtime>::insert(key1, key2, new_value);
			MigrationStepResult::continue_step(Self::max_step_weight(), (key1, key2))
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
		let sample = <Vec<(CollectionUuid, SerialNumber, old::Xls20TokenId)> as Decode>::decode(
			&mut &state[..],
		)
		.expect("🦆 pre_upgrade_step provides a valid state; qed");

		log::debug!(target: LOG_TARGET, "Validating sample of {} token_ids", sample.len());
		for (collection_id, serial_number, old) in sample {
			let new =
				pallet_xls20::Xls20TokenMap::<Runtime>::get(collection_id, serial_number).unwrap();
			ensure!(new == old[..32], "🦆 Invalid token_id migration");
		}
		Ok(())
	}
}
