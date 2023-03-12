pub mod v0 {
	use crate::EVMChainId;
	use frame_support::{
		dispatch::GetStorageVersion,
		traits::{OnRuntimeUpgrade, StorageVersion},
		weights::Weight,
	};

	pub struct Upgrade;
	impl OnRuntimeUpgrade for Upgrade {
		fn on_runtime_upgrade() -> Weight {
			let current = EVMChainId::current_storage_version();
			let onchain = EVMChainId::on_chain_storage_version();

			log::info!(target: "Evm Chain Id", "Running migration with current storage version {current:?} / onchain {onchain:?}");

			if onchain == 0 {
				StorageVersion::new(0).put::<EVMChainId>();
			}

			100
		}
	}
}
