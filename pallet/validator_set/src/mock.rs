use crate as pallet_validator_set;
use frame_support::{
	parameter_types,
	traits::{ConstU16, ConstU64},
	PalletId,
};
use frame_system as system;
use frame_system::EnsureRoot;
use seed_pallet_common::FinalSessionTracker;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Percent,
};
use sp_runtime::traits::{IdentifyAccount, Verify};
use seed_primitives::Signature;
use seed_primitives::validator::crypto::AuthorityId;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		DefaultValidatorWhiteList: pallet_validator_set::{Pallet, Call, Storage, Event<T>},
	}
);

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = ConstU64<250>;
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

parameter_types! {
	pub const NotarizationThreshold: Percent = Percent::from_parts(66_u8);
	/// The bridge contract address (if any) paired with the bridge pallet
	pub const RemoteChainBridgeContractAddress: [u8; 20] = hex_literal::hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d");
	pub const BridgePalletId: PalletId = PalletId(*b"bridgeid");
}

/// Mock final session tracker
pub struct MockFinalSessionTracker;
impl FinalSessionTracker for MockFinalSessionTracker {
	fn is_active_session_final() -> bool {
		// at block 2, the active session is final
		frame_system::Pallet::<Test>::block_number() == 2
	}
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
	where
		Call: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: Call,
		_public: <Signature as Verify>::Signer,
		_account: AccountId,
		nonce: u64,
	) -> Option<(Call, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
		Some((call, (nonce, ())))
	}
}

impl pallet_validator_set::Config for Test {
	type Event = Event;
	type ApproveOrigin = EnsureRoot<Self::AccountId>;
	type AuthoritySet = ();
	type XrplBridgeCall = ();
	type ValidatorId = AuthorityId;
	type FinalSessionTracker = MockFinalSessionTracker;
	type BridgePalletId = BridgePalletId;
	type EpochDuration = ();
	type BridgeContractAddress = RemoteChainBridgeContractAddress;
	type NotarizationThreshold = NotarizationThreshold;
	type ChainWebsocketClient = ();
	type UnixTime = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}
