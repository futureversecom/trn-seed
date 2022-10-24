use crate as pallet_validator_set;
use crate::{BridgeXrplWebsocketApi, ChainCallId, Config, ValidatorIdOf};
use async_trait::async_trait;
use frame_support::{
	parameter_types,
	storage::{StorageDoubleMap, StorageValue},
	traits::{ConstU16, ConstU64, UnixTime, ValidatorSet as ValidatorSetT},
	PalletId,
};
use frame_system as system;
use frame_system::EnsureRoot;
use pallet_session::historical as pallet_session_historical;
use seed_pallet_common::{
	xrpl_types::{BridgeRpcError, XrplTxHash},
	EthyXrplBridgeAdapter, FinalSessionTracker,
};
use seed_primitives::{
	ethy::{crypto::AuthorityId as AuthorityIdE, EventProofId},
	validator::crypto::AuthorityId,
	xrpl::{LedgerIndex, XrpTransaction},
	AssetId, Balance, BlockNumber, Signature,
};
use sp_application_crypto::RuntimeAppPublic;
use sp_core::{ByteArray, H160, H256};
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use sp_runtime::{
	testing::{Header, TestXt, UintAuthorityId},
	traits::{
		BlakeTwo256, Convert, ConvertInto, Extrinsic as ExtrinsicT, IdentifyAccount,
		IdentityLookup, Verify,
	},
	DispatchError, Percent,
};
use std::sync::Arc;
use tokio::sync::{mpsc, mpsc::Receiver};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub type SessionIndex = u32;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		Historical: pallet_session_historical::{Pallet},
		Assets: pallet_assets::{Pallet, Storage, Config<T>, Event<T>},
		AssetsExt: pallet_assets_ext::{Pallet, Call, Storage, Config<T>, Event<T>},
		XRPLBridge: pallet_xrpl_bridge::{Pallet, Call, Storage, Event<T>},
		DefaultValidatorSet: pallet_validator_set::{Pallet, Call, Storage, Event<T>},
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
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

/// Mock final session tracker
pub struct MockFinalSessionTracker;
impl FinalSessionTracker for MockFinalSessionTracker {
	fn is_active_session_final() -> bool {
		// at block 2, the active session is final
		frame_system::Pallet::<Test>::block_number() == 2
	}
}

pub type Extrinsic = TestXt<Call, ()>;

pub(crate) mod test_storage {
	//! storage used by tests to store mock XrplBlocks and TransactionReceipts
	use super::AccountId; //, MockBlockResponse, MockReceiptResponse
	use crate::Config;
	use frame_support::decl_storage;
	//, MockBlockResponse, MockReceiptResponse
	use seed_pallet_common::xrpl_types::{ChainCallId, CheckedChainCallResult};
	//use seed_pallet_common::XrplCallFailure;
	use seed_primitives::xrpl::XrplAddress;

	pub struct Module<T>(sp_std::marker::PhantomData<T>);
	decl_storage! {
		trait Store for Module<T: Config> as XrplBridgeTest {
			//pub BlockResponseAt: map hasher(identity) u64 => Option<MockBlockResponse>;
			pub CallAt: double_map hasher(twox_64_concat) u64, hasher(twox_64_concat) XrplAddress => Option<Vec<u8>>;
			//pub TransactionReceiptFor: map hasher(twox_64_concat) XrplTxHash => Option<MockReceiptResponse>;
			pub Timestamp: Option<u64>;
			pub Validators: Vec<AccountId>;
			pub LastCallResult: Option<(ChainCallId, CheckedChainCallResult)>;
			//pub LastCallFailure: Option<(ChainCallId, XrplCallFailure)>;
		}
	}
}

pub struct NoopConverter<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> Convert<T::AccountId, Option<T::AccountId>> for NoopConverter<T> {
	fn convert(address: T::AccountId) -> Option<T::AccountId> {
		Some(address)
	}
}

pub struct MockValidatorSet;
impl ValidatorSetT<AccountId> for MockValidatorSet {
	type ValidatorId = AccountId;
	type ValidatorIdOf = NoopConverter<Test>;
	/// Returns current session index.
	fn session_index() -> SessionIndex {
		1
	}
	/// Returns the active set of validators.
	fn validators() -> Vec<Self::ValidatorId> {
		test_storage::Validators::get()
	}
}
impl MockValidatorSet {
	/// Mock n validator stashes
	pub fn mock_n_validators(n: u8) {
		let validators: Vec<AccountId> =
			(1..=n as u64).map(|i| H160::from_low_u64_be(i).into()).collect();
		test_storage::Validators::put(validators);
	}
}

impl frame_system::offchain::SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	Call: From<C>,
{
	type OverarchingCall = Call;
	type Extrinsic = Extrinsic;
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

/// Returns a fake timestamp based on the current block number
pub struct MockUnixTime;
impl UnixTime for MockUnixTime {
	fn now() -> core::time::Duration {
		match test_storage::Timestamp::get() {
			// Use configured value for tests requiring precise timestamps
			Some(s) => core::time::Duration::new(s, 0),
			// fallback, use block number to derive timestamp for tests that only care abut block
			// progression
			None => core::time::Duration::new(System::block_number() * 5, 0),
		}
	}
}

parameter_types! {
	pub const NotarizationThreshold: Percent = Percent::from_parts(66_u8);
	/// The bridge contract address (if any) paired with the bridge pallet
	pub const RemoteChainBridgeContractAddress: [u8; 20] = hex_literal::hex!("a86e122EdbDcBA4bF24a2Abf89F5C230b37DF49d");
	pub const BridgePalletId: PalletId = PalletId(*b"bridgeid");
	pub const EpochDuration: u64 = 1000_u64;
}

impl Config for Test {
	type Event = Event;
	type MultiCurrency = AssetsExt;
	type ApproveOrigin = EnsureRoot<Self::AccountId>;
	type AuthoritySet = MockValidatorSet;
	type XrplBridgeCall = XRPLBridge;
	type ValidatorId = AuthorityId;
	type FinalSessionTracker = MockFinalSessionTracker;
	type BridgePalletId = BridgePalletId;
	type EpochDuration = EpochDuration;
	type BridgeContractAddress = RemoteChainBridgeContractAddress;
	type NotarizationThreshold = NotarizationThreshold;
	type ChainWebsocketClient = MockChainWebsocketClient;
	type UnixTime = MockUnixTime;
}

pub struct MockChainWebsocketClient;

impl MockChainWebsocketClient {}

#[async_trait]
impl BridgeXrplWebsocketApi for MockChainWebsocketClient {
	async fn transaction_entry_request(
		xrp_transaction: XrpTransaction,
		ledger_index: LedgerIndex,
		call_id: ChainCallId,
	) -> Result<Receiver<Result<XrplTxHash, BridgeRpcError>>, BridgeRpcError> {
		let (_tx, rx) = mpsc::channel(4);
		Ok(rx)
	}
}

// Time is measured by number of blocks.
pub const MILLISECS_PER_BLOCK: u64 = 4_000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

parameter_types! {
	pub const XrpTxChallengePeriod: u32 = 10 * MINUTES;
	pub const XrpClearTxPeriod: u32 = 10 * DAYS;
}

impl pallet_xrpl_bridge::Config for Test {
	type Event = Event;
	type EthyAdapter = MockEthyAdapter;
	type MultiCurrency = AssetsExt;
	type ApproveOrigin = EnsureRoot<Self::AccountId>;
	type WeightInfo = ();
	type XrpAssetId = XrpAssetId;
	type ChallengePeriod = XrpTxChallengePeriod;
	type ClearTxPeriod = XrpClearTxPeriod;
	type UnixTime = MockUnixTime;
}

pub struct MockEthyAdapter;

impl EthyXrplBridgeAdapter<AuthorityIdE> for MockEthyAdapter {
	/// Mock implementation of EthyXrplBridgeAdapter
	fn sign_xrpl_transaction(_tx_data: &[u8]) -> Result<EventProofId, DispatchError> {
		Ok(1)
	}
	fn validators() -> Vec<AuthorityIdE> {
		// some hard coded validators
		vec![
			AuthorityIdE::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityIdE::from_slice(&[2_u8; 33]).unwrap(),
			AuthorityIdE::from_slice(&[3_u8; 33]).unwrap(),
		]
	}
}

parameter_types! {
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ();
	type AccountStore = System;
	type MaxLocks = ();
	type WeightInfo = ();
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
}

parameter_types! {
	pub const AssetsExtPalletId: PalletId = PalletId(*b"assetext");
	pub const MaxHolds: u32 = 16;
	pub const XrpAssetId: AssetId = 2;
	pub const TestParachainId: u32 = 100;
}

impl pallet_assets_ext::Config for Test {
	type Event = Event;
	type ParachainId = TestParachainId;
	type MaxHolds = MaxHolds;
	type NativeAssetId = XrpAssetId;
	type PalletId = AssetsExtPalletId;
}

parameter_types! {
	pub const AssetDeposit: Balance = 1_000_000;
	pub const AssetAccountDeposit: Balance = 16;
	pub const ApprovalDeposit: Balance = 1;
	pub const AssetsStringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 1 * 68;
	pub const MetadataDepositPerByte: Balance = 1;
}
pub type AssetsForceOrigin = EnsureRoot<AccountId>;

impl pallet_assets::Config for Test {
	type Event = Event;
	type Balance = Balance;
	type AssetId = AssetId;
	type Currency = Balances;
	type ForceOrigin = AssetsForceOrigin;
	type AssetDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = ();
	type AssetAccountDeposit = AssetAccountDeposit;
}

parameter_types! {
	pub const Period: u64 = 1;
	pub const Offset: u64 = 0;
}

impl pallet_session::Config for Test {
	type Event = Event;
	type ValidatorId = AccountId;
	type ValidatorIdOf = ConvertInto;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, TestSessionManager>;
	type SessionHandler = (DefaultValidatorSet,);
	type Keys = UintAuthorityId;
	type WeightInfo = ();
}

pub struct TestSessionManager;
impl pallet_session::SessionManager<AccountId> for TestSessionManager {
	fn new_session(_new_index: SessionIndex) -> Option<Vec<AccountId>> {
		Some(MockValidatorSet::validators())
	}
	fn end_session(_: SessionIndex) {}
	fn start_session(_: SessionIndex) {}
}

impl pallet_session::historical::SessionManager<AccountId, u64> for TestSessionManager {
	fn new_session(_new_index: SessionIndex) -> Option<Vec<(AccountId, u64)>> {
		let mut i: u64 = 0;
		let validators: Vec<(AccountId, u64)> = MockValidatorSet::validators()
			.into_iter()
			.map(|val| {
				i += 1;
				(val, i)
			})
			.collect();
		Option::from(validators)
	}
	fn start_session(_: SessionIndex) {}
	fn end_session(_: SessionIndex) {}
}

impl pallet_session::historical::Config for Test {
	type FullIdentification = u64;
	type FullIdentificationOf = ();
}

pub fn init_keys() -> Vec<<Test as Config>::ValidatorId> {
	// fake ecdsa public keys to represent the mocked validators
	let n = 9_u8;
	let mock_notary_keys: Vec<<Test as Config>::ValidatorId> = (1_u8..=n)
		.map(|k| <Test as Config>::ValidatorId::from_slice(&[k; 33]).unwrap())
		.collect();

	MockValidatorSet::mock_n_validators(mock_notary_keys.len() as u8);

	let mut uint_key: Vec<u64> = Vec::new();
	for i in 1..=n {
		uint_key.push(i as u64);
	}
	UintAuthorityId::set_all_keys(uint_key);
	mock_notary_keys
}
// Build genesis storage according to the mock runtime.
/*pub fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}*/

#[derive(Clone, Copy, Default)]
pub struct ExtBuilder {
	relayer: Option<AccountId>,
	with_keystore: bool,
	next_session_final: bool,
	active_session_final: bool,
}

impl ExtBuilder {
	pub fn with_keystore(&mut self) -> &mut Self {
		self.with_keystore = true;
		self
	}
	pub fn active_session_final(&mut self) -> &mut Self {
		self.active_session_final = true;
		self
	}
	pub fn next_session_final(&mut self) -> &mut Self {
		self.next_session_final = true;
		self
	}
	pub fn build(self) -> sp_io::TestExternalities {
		let mut ext: sp_io::TestExternalities =
			frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into();

		ext.execute_with(|| System::initialize(&1, &[0u8; 32].into(), &Default::default()));

		if self.with_keystore {
			let keystore = KeyStore::new();
			SyncCryptoStore::ecdsa_generate_new(&keystore, AuthorityId::ID, None).unwrap();
			ext.register_extension(KeystoreExt(Arc::new(keystore)));
		}

		if self.next_session_final {
			ext.execute_with(|| frame_system::Pallet::<Test>::set_block_number(1));
		} else if self.active_session_final {
			ext.execute_with(|| frame_system::Pallet::<Test>::set_block_number(2));
		}

		ext
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let relayer = H160::from_low_u64_be(123);
	let t = ExtBuilder::default().with_keystore().build();
	let mut result: sp_io::TestExternalities = t.into();
	// Set the default keys, otherwise session will discard the validator.
	result.execute_with(|| {
		let mut i: u64 = 0;
		for validator in MockValidatorSet::validators() {
			System::inc_providers(&validator);
			assert_eq!(Session::set_keys(Origin::signed(validator), i.into(), vec![]), Ok(()));
			i += 1;
		}
	});
	result
}
