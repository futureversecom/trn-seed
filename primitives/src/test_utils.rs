#![cfg(feature = "std")]
use crate::*;
use frame_support::{sp_io, traits::GenesisBuild};
use sp_core::H160;

/// Generec TestExternalities builder to be used across all pallets
pub struct TestExt<T: frame_system::Config + pallet_balances::Config + pallet_assets::Config> {
	balances: Vec<(T::AccountId, <T as pallet_balances::Config>::Balance)>,
	xrp_balances: Vec<(
		<T as pallet_assets::Config>::AssetId,
		T::AccountId,
		<T as pallet_assets::Config>::Balance,
	)>,
	block_number: BlockNumber,
}

impl<T> Default for TestExt<T>
where
	T: frame_system::Config + pallet_balances::Config + pallet_assets::Config,
	<T as pallet_balances::Config>::Balance: From<Balance>,
{
	/// Create new TestExt with default values
	fn default() -> Self {
		Self { balances: vec![], xrp_balances: vec![], block_number: 1 }
	}
}

impl<T> TestExt<T>
where
	T: frame_system::Config + pallet_balances::Config + pallet_assets::Config,
	<T as frame_system::Config>::BlockNumber: From<u64>,
	<T as pallet_balances::Config>::Balance: From<Balance>,
	<T as pallet_assets::Config>::Balance: From<Balance>,
	T::AccountId: From<H160>,
	<T as pallet_assets::Config>::AssetId: From<AssetId>,
	<T as frame_system::Config>::Hash: From<[u8; 32]>,
{
	/// Configure some native token balances
	pub fn with_balances(
		mut self,
		balances: &[(T::AccountId, <T as pallet_balances::Config>::Balance)],
	) -> Self {
		self.balances = balances.to_vec();
		self
	}

	/// Configure some XRP asset balances
	pub fn with_xrp_balances(
		mut self,
		balances: &[(T::AccountId, <T as pallet_assets::Config>::Balance)],
	) -> Self {
		self.xrp_balances = balances
			.to_vec()
			.into_iter()
			.map(|(who, balance)| (2.into(), who, balance))
			.collect();
		self
	}

	/// Configure starting block number
	pub fn with_block_number(mut self, block_number: BlockNumber) -> Self {
		self.block_number = block_number;
		self
	}

	/// Build the Text Externalities for general use across all pallets
	pub fn build(self) -> sp_io::TestExternalities {
		let mut ext = frame_system::GenesisConfig::default().build_storage::<T>().unwrap();

		// add assets to Genesis Config
		if !self.xrp_balances.is_empty() {
			let assets =
				vec![(2.into(), T::AccountId::from(H160::from_low_u64_be(100)), true, 1.into())];
			let metadata = vec![(2.into(), b"XRP".to_vec(), b"XRP".to_vec(), 6_u8)];
			let accounts = self.xrp_balances;

			pallet_assets::GenesisConfig::<T> { assets, metadata, accounts }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

		// add initial balances to Genesis Config
		if !self.balances.is_empty() {
			pallet_balances::GenesisConfig::<T> { balances: self.balances }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

		let mut ext: sp_io::TestExternalities = ext.into();
		ext.execute_with(|| {
			frame_system::Pallet::<T>::initialize(
				&self.block_number.into(),
				&[0u8; 32].into(),
				&Default::default(),
			)
		});

		ext
	}
}
