use frame_support::weights::Weight;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;
use sp_std::marker::PhantomData;
use sp_std::vec::Vec;

pub struct MigrationStepResult {
	is_finished: bool,
	pub weight_consumed: Weight,
	pub last_key: Option<Vec<u8>>,
}

impl MigrationStepResult {
	/// Generate a MigrationStepResult for a non-final step
	pub fn continue_step(weight_consumed: Weight, last_key: Vec<u8>) -> Self {
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

	type OldStorageValue;

	type NewStorageValue;

	/// Check if the current storage version matches the target version.
	fn version_check() -> bool;

	/// Called when the migration is complete.
	fn on_complete();

	/// Returns the maximum weight that can be consumed in a single step.
	fn max_step_weight() -> Weight;

	/// Convert from the OldStorageValue to the NewStorageValue
	fn convert(old: Self::OldStorageValue) -> Result<Self::NewStorageValue, &'static str>;

	/// Process one step of the migration.
	/// Returns whether the migration is finished and the weight consumed.
	fn step(last_key: Option<Vec<u8>>, verbose: bool) -> MigrationStepResult;

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

pub struct NoopMigration;

impl MigrationStep for NoopMigration {
	const TARGET_VERSION: u16 = 0;

	type OldStorageValue = ();
	type NewStorageValue = ();

	fn version_check() -> bool {
		true
	}

	fn on_complete() {}

	fn max_step_weight() -> Weight {
		Weight::zero()
	}

	fn convert(_: Self::OldStorageValue) -> Result<Self::NewStorageValue, &'static str> {
		Ok(())
	}

	fn step(_: Option<Vec<u8>>, _: bool) -> MigrationStepResult {
		MigrationStepResult::finish_step(Weight::zero())
	}
}
