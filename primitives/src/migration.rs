use codec::{Codec, MaxEncodedLen};
use frame_support::DefaultNoBound;
use frame_support::weights::Weight;
use crate::*;

pub struct MigrationStepResult<StorageKey> {
    is_finished: bool,
    pub weight_consumed: Weight,
    pub last_key: Option<StorageKey>,
}

impl<StorageKey> MigrationStepResult<StorageKey> {
    /// Generate a MigrationStepResult for a non-final step
    pub fn continue_step(weight_consumed: Weight, last_key: StorageKey) -> Self {
        Self { is_finished: false, weight_consumed, last_key: Some(last_key) }
    }

    /// Generate a MigrationStepResult for the final step
    pub fn finish_step(weight_consumed: Weight) -> Self {
        Self { is_finished: true, weight_consumed, last_key: None }
    }

    /// Returns whether the migration is finished.
    pub fn is_finished(&self) -> bool {
        self.is_finished
    }
}

/// A trait that allows to migrate storage from one version to another.
///
/// The migration is done in steps. The migration is finished when
/// `step()` returns `IsFinished::Yes`.
pub trait MigrationStep {
    /// Returns the version of the migration.
    const TARGET_VERSION: u16;

    type StorageKey;
    type OldStorageValue;

    type NewStorageValue;

    /// Check if the current storage version matches the target version.
    fn version_check() -> bool;

    /// Returns the maximum weight that can be consumed in a single step.
    fn max_step_weight() -> Weight;

    /// Convert from the OldStorageValue to the NewStorageValue
    fn convert(old: Self::OldStorageValue) -> Self::NewStorageValue;

    /// Process one step of the migration.
    ///
    /// Returns whether the migration is finished and the weight consumed.
    fn step(last_key: Option<Self::StorageKey>) -> MigrationStepResult<Self::StorageKey>;

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

pub struct NoopMigration {}

impl MigrationStep for NoopMigration {
    const TARGET_VERSION: u16 = 0;

    type StorageKey = ();
    type OldStorageValue = ();
    type NewStorageValue = ();

    fn version_check() -> bool {
        true
    }

    fn max_step_weight() -> Weight {
        Weight::zero()
    }

    fn convert(_: Self::OldStorageValue) -> Self::NewStorageValue {
        ()
    }

    fn step(_: Option<Self::StorageKey>) -> MigrationStepResult<Self::StorageKey> {
        MigrationStepResult::finish_step(Weight::zero())
    }
}