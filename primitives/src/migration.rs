use codec::{Codec, MaxEncodedLen};
use frame_support::weights::Weight;
use crate::*;

/// IsFinished describes whether a migration is finished or not.
#[derive(PartialEq, Eq)]
pub enum IsFinished {
    Yes,
    No,
}

/// A trait that allows to migrate storage from one version to another.
///
/// The migration is done in steps. The migration is finished when
/// `step()` returns `IsFinished::Yes`.
pub trait MigrationStep: Codec + MaxEncodedLen {
    /// Returns the version of the migration.
    const TARGET_VERSION: u16;

    type StorageKey;
    type OldStorageValue;

    type NewStorageValue;

    /// Check if the current storage version matches the target version.
    fn version_check() -> bool;

    /// Returns the maximum weight that can be consumed in a single step.
    fn max_step_weight() -> Weight;

    fn convert(old: Self::OldStorageValue) -> Self::NewStorageValue;

    /// Process one step of the migration.
    ///
    /// Returns whether the migration is finished and the weight consumed.
    fn step(last_key: Option<Self::StorageKey>) -> (IsFinished, Weight, Option<Self::StorageKey>);

    /// Execute some pre-checks prior to running the first step of this migration.
    #[cfg(feature = "try-runtime")]
    fn pre_upgrade_step() -> Result<Vec<u8>, TryRuntimeError> {
        Ok(Vec::new())
    }

    /// Execute some post-checks after running the last step of this migration.
    #[cfg(feature = "try-runtime")]
    fn post_upgrade_step(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
        Ok(())
    }
}