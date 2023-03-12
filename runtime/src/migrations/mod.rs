mod evm_chain_id;
mod fee_control;

use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};

pub struct AllMigrations;
impl OnRuntimeUpgrade for AllMigrations {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		fee_control::v2::Upgrade::pre_upgrade()?;

		Ok(())
	}

	fn on_runtime_upgrade() -> Weight {
		let mut weight = Weight::from(0u32);
		weight += evm_chain_id::v0::Upgrade::on_runtime_upgrade();
		weight += fee_control::v2::Upgrade::on_runtime_upgrade();

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		fee_control::v2::Upgrade::post_upgrade()?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::{Runtime, System};

	pub fn new_test_ext() -> sp_io::TestExternalities {
		let t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
