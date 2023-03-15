use crate::{
	Block, BlockHashCount, Header, RuntimeBlockLength, RuntimeBlockWeights, SS58Prefix,
	UncheckedExtrinsic, Version,
};
use frame_support::{pallet_prelude::*, weights::constants::RocksDbWeight};
use seed_primitives::{AccountId, BlockNumber, Hash, Index};
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentityLookup},
};
use sp_std::prelude::*;

// Do not add new runtime components in here!
frame_support::construct_runtime! {
	pub enum Runtime where
		Block = Block,
		NodeBlock = generic::Block<Header, sp_runtime::OpaqueExtrinsic>,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Example: pallet_example,
	}
}
impl frame_system::Config for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = IdentityLookup<AccountId>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = Header;
	/// The ubiquitous event type.
	type Event = Event;
	/// The ubiquitous origin type.
	type Origin = Origin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Runtime version.
	type Version = Version;
	/// Converts a pallet to an index of this pallet in the runtime.
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = RocksDbWeight;
	type BaseCallFilter = CallFilter;
	type SystemWeightInfo = crate::weights::frame_system::WeightInfo<Runtime>;
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}
pub enum CallFilter {}
impl frame_support::traits::Contains<Call> for CallFilter {
	fn contains(_call: &Call) -> bool {
		true
	}
}
impl pallet_example::Config for Runtime {
	type Event = Event;
}
