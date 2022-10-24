use frame_support::{
	construct_runtime, parameter_types,
	traits::{ConstU16, ConstU64},
	PalletId,
};
use frame_system as system;
use frame_system::{limits, EnsureRoot};
use sp_core::{H160, H256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	DispatchError,
};

pub use seed_pallet_common::{EventProofAdapter, ValidatorAdapter};
use seed_primitives::{
	validator::{EventProofId, ValidatorSetId},
	AccountId, AssetId, Balance, BlockNumber,
};

use crate as pallet_event_proof;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		EventProof: pallet_event_proof::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub BlockLength: limits::BlockLength = limits::BlockLength::max(2 * 1024);
}

impl frame_system::Config for Test {
	type BlockWeights = ();
	type BlockLength = BlockLength;
	type BaseCallFilter = frame_support::traits::Everything;
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Call = Call;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type BlockHashCount = ConstU64<250>;
	type Event = Event;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_event_proof::Config for Test {
	type Event = Event;
	type ApproveOrigin = EnsureRoot<Self::AccountId>;
	type WeightInfo = ();
}

pub struct MockValidatorAdapter;

impl ValidatorAdapter for MockValidatorAdapter {
	/// Mock implementation of ValidatorAdapter
	fn validator_set_id() -> ValidatorSetId {
		1
	}

	fn bridge_paused(flag: bool) {
		todo!()
	}

	fn bridge_kill() {
		todo!()
	}
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}
