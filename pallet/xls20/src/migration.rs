use seed_pallet_common::Migration;
use crate::*;

impl<T: Config> Migration for Pallet<T> {
    type StorageKey1 = CollectionUuid;
    type StorageKey2 = SerialNumber;
    type OldStorageMap = Xls20TokenMap<T>;
    type NewStorageMap = TokenIdToXls20<T>;
    type OldStorageValue = Xls20TokenId;
    type NewStorageValue = [u8; 32];

    fn convert(value: Self::OldStorageValue) -> Self::NewStorageValue {
        let mut new_token_id = [0; 32];
        new_token_id.copy_from_slice(&value[..32]);
        new_token_id
    }

    fn migrate_next() -> Weight {
        let Some((key1, key2, old_value)) = Self::OldStorageMap::drain().next() else {
            return T::DbWeight::get().writes(1)
        };
        let new_value = Self::convert(old_value);
        Self::NewStorageMap::insert(key1, key2, &new_value);
        T::DbWeight::get().reads_writes(1, 1)
    }

    // Get the value, this can either be from the old storage or from the new storage
    fn get(key1: &Self::StorageKey1, key2: &Self::StorageKey2) -> Option<Self::NewStorageValue> {
        if let Some(value) = Self::OldStorageMap::get(key1, key2) {
            Some(Self::convert(value))
        } else {
            Self::OldStorageMap::get(key1, key2)
        }
    }

    fn insert(key1: &Self::StorageKey1, key2: &Self::StorageKey2, value: Option<Self::NewStorageValue>) {
        match value {
            Some(value) => Self::NewStorageMap::insert(key1, key2, value),
            None => Self::NewStorageMap::remove(key1, key2)
        }
    }

    fn ensure_migrated() -> frame_support::dispatch::DispatchResult {
        todo!()
    }
}