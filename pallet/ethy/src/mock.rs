// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

use std::{
	sync::Arc,
	time::{SystemTime, UNIX_EPOCH},
};

use crate::ConstU32;
use codec::{Decode, Encode};
use ethereum_types::U64;
use frame_support::{
	parameter_types,
	storage::{StorageDoubleMap, StorageValue},
	traits::{AsEnsureOriginWithArg, UnixTime, ValidatorSet as ValidatorSetT},
	weights::Weight,
	PalletId,
};
use frame_system::{EnsureRoot, EnsureSigned};
use scale_info::TypeInfo;
use sp_application_crypto::RuntimeAppPublic;
use sp_core::{ByteArray, H160, H256, U256};
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{
		BlakeTwo256, Convert, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify,
	},
	DispatchError, Percent,
};

use seed_pallet_common::{
	EthCallFailure, EthCallOracleSubscriber, EthereumEventRouter, EthyToXrplBridgeAdapter,
	EventRouterResult, FinalSessionTracker,
};
use seed_primitives::{
	ethy::{crypto::AuthorityId, EventProofId},
	AssetId, Balance, Signature,
};

use crate::{
	self as pallet_ethy,
	sp_api_hidden_includes_decl_storage::hidden_include::{IterableStorageMap, StorageMap},
	types::{
		BridgeEthereumRpcApi, BridgeRpcError, CheckedEthCallRequest, CheckedEthCallResult,
		EthAddress, EthBlock, EthCallId, EthHash, LatestOrNumber, Log, TransactionReceipt,
	},
	Config,
};

pub const XRP_ASSET_ID: AssetId = 1;

type BlockNumber = u64;
pub type SessionIndex = u32;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Extrinsic = TestXt<RuntimeCall, ()>;
pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
pub type Block = frame_system::mocking::MockBlock<TestRuntime>;
pub type AssetsForceOrigin = EnsureRoot<AccountId>;

frame_support::construct_runtime!(
	pub enum TestRuntime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		EthBridge: pallet_ethy::{Pallet, Call, Storage, Event<T>, ValidateUnsigned},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Assets: pallet_assets::{Pallet, Storage, Config<T>, Event<T>},
		AssetsExt: pallet_assets_ext::{Pallet, Storage, Event<T>},
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
		Preimage: pallet_preimage,
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}
impl frame_system::Config for TestRuntime {
	type BlockWeights = ();
	type BlockLength = ();
	type BaseCallFilter = frame_support::traits::Everything;
	type RuntimeOrigin = RuntimeOrigin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type BlockHashCount = BlockHashCount;
	type RuntimeEvent = RuntimeEvent;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const NotarizationThreshold: Percent = Percent::from_parts(66_u8);
	pub const BridgePalletId: PalletId = PalletId(*b"ethybrdg");
	pub const EpochDuration: u64 = 1000_u64;
	pub const ChallengerBond: Balance = 100;
	pub const RelayerBond: Balance = 202;
	pub const XrpAssetId: AssetId = XRP_ASSET_ID;
	pub const MaxXrplKeys: u8 = 8;
	pub const MaxNewSigners: u8 = 20;
	pub const AuthorityChangeDelay: BlockNumber = 75;
}
impl Config for TestRuntime {
	type AuthorityChangeDelay = AuthorityChangeDelay;
	type AuthoritySet = MockValidatorSet;
	type BridgePalletId = BridgePalletId;
	type EthCallSubscribers = MockEthCallSubscriber;
	type EthereumRpcClient = MockEthereumRpcClient;
	type EthyId = AuthorityId;
	type EventRouter = MockEventRouter;
	type FinalSessionTracker = MockFinalSessionTracker;
	type NotarizationThreshold = NotarizationThreshold;
	type UnixTime = MockUnixTime;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type EpochDuration = EpochDuration;
	type ChallengeBond = ChallengerBond;
	type MultiCurrency = AssetsExt;
	type NativeAssetId = XrpAssetId;
	type RelayerBond = RelayerBond;
	type MaxXrplKeys = MaxXrplKeys;
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type MaxNewSigners = MaxNewSigners;
	type XrplBridgeAdapter = MockXrplBridgeAdapter;
}

pub struct MockXrplBridgeAdapter;
impl EthyToXrplBridgeAdapter<H160> for MockXrplBridgeAdapter {
	/// Mock implementation of EthyToXrplBridgeAdapter
	fn submit_signer_list_set_request(_: Vec<(H160, u16)>) -> Result<EventProofId, DispatchError> {
		Ok(1)
	}
}

parameter_types! {
	pub const AssetDeposit: Balance = 1_000_000;
	pub const AssetAccountDeposit: Balance = 16;
	pub const ApprovalDeposit: Balance = 1;
	pub const AssetsStringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 1 * 68;
	pub const MetadataDepositPerByte: Balance = 1;
}

impl pallet_assets::Config for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
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
	type RemoveItemsLimit = ConstU32<1000>;
	type AssetIdParameter = codec::Compact<u32>;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type CallbackHandle = ();
}

parameter_types! {
	pub const NativeAssetId: AssetId = 1;
	pub const AssetsExtPalletId: PalletId = PalletId(*b"assetext");
	pub const MaxHolds: u32 = 16;
	pub const TestParachainId: u32 = 100;
}

impl pallet_assets_ext::Config for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
	type ParachainId = TestParachainId;
	type MaxHolds = MaxHolds;
	type NativeAssetId = NativeAssetId;
	type OnNewAssetSubscription = ();
	type PalletId = AssetsExtPalletId;
	type WeightInfo = ();
}

parameter_types! {
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for TestRuntime {
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ();
	type AccountStore = System;
	type MaxLocks = ();
	type WeightInfo = ();
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
}

parameter_types! {
	pub const MaxScheduledPerBlock: u32 = 50;
}
impl pallet_scheduler::Config for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = ();
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type OriginPrivilegeCmp = frame_support::traits::EqualPrivilegeOnly;
	type WeightInfo = ();
	type Preimages = Preimage;
}

parameter_types! {
	// TODO! Marko
	pub const PreimageBaseDeposit: Balance = 1_000_000_000;
	// TODO! Marko
	// One cent: $10,000 / MB
	pub const PreimageByteDeposit: Balance = 1_000_000_000;
}

impl pallet_preimage::Config for TestRuntime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type BaseDeposit = PreimageBaseDeposit;
	type ByteDeposit = PreimageByteDeposit;
}

/// Values in EthBlock that we store in mock storage
#[derive(PartialEq, Eq, Encode, Decode, Debug, Clone, Default, TypeInfo)]
pub struct MockBlockResponse {
	pub block_hash: H256,
	pub block_number: u64,
	pub timestamp: U256,
}

/// Mock data for an Ethereum log
/// NB: `ethereum_types::Log` does not implement SCALE/TypeInfo so can't be used directly in storage
/// `MockLog` is used in its place and converted to `Log` on read
#[derive(PartialEq, Eq, Encode, Decode, Debug, Clone, Default, TypeInfo)]
pub struct MockLog {
	pub topics: Vec<H256>,
	pub data: Vec<u8>,
	pub address: EthAddress,
	pub transaction_hash: Option<H256>,
}

impl Into<Log> for MockLog {
	fn into(self) -> Log {
		Log {
			address: self.address,
			data: self.data,
			topics: self.topics,
			transaction_hash: self.transaction_hash,
			..Default::default()
		}
	}
}

impl From<Log> for MockLog {
	fn from(l: Log) -> Self {
		Self {
			address: l.address,
			data: l.data,
			topics: l.topics,
			transaction_hash: l.transaction_hash,
		}
	}
}

pub(crate) struct MockLogBuilder(MockLog);

impl MockLogBuilder {
	pub fn new() -> Self {
		Self(MockLog::default())
	}
	pub fn build(&self) -> MockLog {
		self.0.clone()
	}
	pub fn address(&mut self, address: EthAddress) -> &mut Self {
		self.0.address = address;
		self
	}
	pub fn topics(&mut self, topics: Vec<H256>) -> &mut Self {
		self.0.topics = topics;
		self
	}
	pub fn data(&mut self, data: &[u8]) -> &mut Self {
		self.0.data = data.to_vec();
		self
	}
	pub fn transaction_hash(&mut self, transaction_hash: H256) -> &mut Self {
		self.0.transaction_hash = Some(transaction_hash);
		self
	}
}

/// Values in TransactionReceipt that we store in mock storage
#[derive(PartialEq, Eq, Encode, Decode, Clone, Default, TypeInfo)]
pub struct MockReceiptResponse {
	pub block_hash: H256,
	pub block_number: u64,
	pub transaction_hash: H256,
	pub status: u64,
	pub logs: Vec<MockLog>,
	/// The top-level address called by the tx
	pub to: Option<EthAddress>,
}

/// Builder for creating EthBlocks
pub(crate) struct MockBlockBuilder(EthBlock);

impl MockBlockBuilder {
	pub fn new() -> Self {
		Self(EthBlock::default())
	}
	pub fn build(&self) -> EthBlock {
		self.0.clone()
	}
	pub fn block_hash(&mut self, block_hash: H256) -> &mut Self {
		self.0.hash = Some(block_hash);
		self
	}
	pub fn block_number(&mut self, block_number: u64) -> &mut Self {
		self.0.number = Some(U64::from(block_number));
		self
	}
	pub fn timestamp(&mut self, timestamp: U256) -> &mut Self {
		self.0.timestamp = timestamp;
		self
	}
}

/// Builder for creating TransactionReceipts
pub(crate) struct MockReceiptBuilder(TransactionReceipt);

impl MockReceiptBuilder {
	pub fn new() -> Self {
		Self(TransactionReceipt { status: Some(U64::from(1)), ..Default::default() })
	}
	pub fn build(&self) -> TransactionReceipt {
		self.0.clone()
	}
	pub fn block_number(&mut self, block_number: u64) -> &mut Self {
		self.0.block_number = U64::from(block_number);
		self
	}
	pub fn status(&mut self, status: u64) -> &mut Self {
		self.0.status = Some(U64::from(status));
		self
	}
	pub fn transaction_hash(&mut self, tx_hash: H256) -> &mut Self {
		self.0.transaction_hash = tx_hash;
		self
	}
	pub fn to(&mut self, to: EthAddress) -> &mut Self {
		self.0.to = Some(to);
		self
	}
	pub fn logs(&mut self, logs: Vec<MockLog>) -> &mut Self {
		self.0.logs = logs.into_iter().map(Into::into).collect();
		self
	}
}

pub(crate) mod test_storage {
	//! storage used by tests to store mock EthBlocks and TransactionReceipts
	use frame_support::decl_storage;

	use seed_pallet_common::EthCallFailure;

	use crate::{
		types::{CheckedEthCallResult, EthAddress, EthCallId, EthHash},
		Config,
	};

	use super::{AccountId, MockBlockResponse, MockReceiptResponse};

	pub struct Module<T>(sp_std::marker::PhantomData<T>);
	decl_storage! {
		trait Store for Module<T: Config> as EthBridgeTest {
			pub BlockResponseAt: map hasher(identity) u64 => Option<MockBlockResponse>;
			pub CallAt: double_map hasher(twox_64_concat) u64, hasher(twox_64_concat) EthAddress => Option<Vec<u8>>;
			pub TransactionReceiptFor: map hasher(twox_64_concat) EthHash => Option<MockReceiptResponse>;
			pub Timestamp: Option<u64>;
			pub Validators: Vec<AccountId>;
			pub LastCallResult: Option<(EthCallId, CheckedEthCallResult)>;
			pub LastCallFailure: Option<(EthCallId, EthCallFailure)>;
			pub Forcing: bool;
		}
	}
}

/// set the block timestamp
pub fn mock_timestamp(now: u64) {
	test_storage::Timestamp::put(now);
}

// get the system unix timestamp in seconds
pub fn now() -> u64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("after unix epoch")
		.as_secs()
}

/// Builder for `CheckedEthCallRequest`
pub struct CheckedEthCallRequestBuilder(CheckedEthCallRequest);

impl CheckedEthCallRequestBuilder {
	pub fn new() -> Self {
		Self(CheckedEthCallRequest {
			max_block_look_behind: 3_u64,
			target: EthAddress::from_low_u64_be(1),
			timestamp: now(),
			check_timestamp: now() + 3 * 5, // 3 blocks
			..Default::default()
		})
	}
	pub fn build(self) -> CheckedEthCallRequest {
		self.0
	}
	pub fn target(mut self, target: EthAddress) -> Self {
		self.0.target = target;
		self
	}
	pub fn try_block_number(mut self, try_block_number: u64) -> Self {
		self.0.try_block_number = try_block_number;
		self
	}
	pub fn max_block_look_behind(mut self, max_block_look_behind: u64) -> Self {
		self.0.max_block_look_behind = max_block_look_behind;
		self
	}
	pub fn check_timestamp(mut self, check_timestamp: u64) -> Self {
		self.0.check_timestamp = check_timestamp;
		self
	}
	pub fn timestamp(mut self, timestamp: u64) -> Self {
		self.0.timestamp = timestamp;
		self
	}
}

/// Mock ethereum rpc client
pub struct MockEthereumRpcClient;

impl MockEthereumRpcClient {
	/// store given block as the next response
	pub fn mock_block_response_at(block_number: u64, mock_block: EthBlock) {
		let mock_block_response = MockBlockResponse {
			block_hash: mock_block.hash.unwrap(),
			block_number: mock_block.number.unwrap().as_u64(),
			timestamp: mock_block.timestamp,
		};
		test_storage::BlockResponseAt::insert(block_number, mock_block_response);
	}
	/// Mock a tx receipt response for a hash
	pub fn mock_transaction_receipt_for(tx_hash: EthHash, mock_tx_receipt: TransactionReceipt) {
		let mock_receipt_response = MockReceiptResponse {
			block_hash: mock_tx_receipt.block_hash,
			block_number: mock_tx_receipt.block_number.as_u64(),
			transaction_hash: mock_tx_receipt.transaction_hash,
			status: mock_tx_receipt.status.unwrap_or_default().as_u64(),
			to: mock_tx_receipt.to,
			logs: mock_tx_receipt.logs.into_iter().map(From::from).collect(),
		};
		test_storage::TransactionReceiptFor::insert(tx_hash, mock_receipt_response);
	}
	/// setup a mock returndata for an `eth_call` at `block` and `contract` address
	pub fn mock_call_at(block_number: u64, contract: H160, return_data: &[u8]) {
		test_storage::CallAt::insert(block_number, contract, return_data.to_vec())
	}
}

impl BridgeEthereumRpcApi for MockEthereumRpcClient {
	/// Returns an ethereum block given a block height
	fn get_block_by_number(
		block_number: LatestOrNumber,
	) -> Result<Option<EthBlock>, BridgeRpcError> {
		let mock_block_response = match block_number {
			LatestOrNumber::Latest =>
				test_storage::BlockResponseAt::iter().last().map(|x| x.1).or(None),
			LatestOrNumber::Number(block) => test_storage::BlockResponseAt::get(block),
		};
		println!("get_block_by_number at: {:?}", mock_block_response);
		if mock_block_response.is_none() {
			return Ok(None)
		}
		let mock_block_response = mock_block_response.unwrap();

		let eth_block = EthBlock {
			number: Some(U64::from(mock_block_response.block_number)),
			hash: Some(mock_block_response.block_hash),
			timestamp: U256::from(mock_block_response.timestamp),
			..Default::default()
		};
		Ok(Some(eth_block))
	}
	/// Returns an ethereum transaction receipt given a tx hash
	fn get_transaction_receipt(
		hash: EthHash,
	) -> Result<Option<TransactionReceipt>, BridgeRpcError> {
		let mock_receipt: Option<MockReceiptResponse> =
			test_storage::TransactionReceiptFor::get(hash);
		if mock_receipt.is_none() {
			return Ok(None)
		}
		let mock_receipt = mock_receipt.unwrap();
		let transaction_receipt = TransactionReceipt {
			block_hash: mock_receipt.block_hash,
			block_number: U64::from(mock_receipt.block_number),
			contract_address: None,
			to: mock_receipt.to,
			status: Some(U64::from(mock_receipt.status)),
			transaction_hash: mock_receipt.transaction_hash,
			logs: mock_receipt.logs.into_iter().map(Into::into).collect(),
			..Default::default()
		};
		Ok(Some(transaction_receipt))
	}
	fn eth_call(
		target: EthAddress,
		_input: &[u8],
		at_block: LatestOrNumber,
	) -> Result<Vec<u8>, BridgeRpcError> {
		let block_number = match at_block {
			LatestOrNumber::Number(n) => n,
			LatestOrNumber::Latest =>
				test_storage::BlockResponseAt::iter().last().unwrap().1.block_number,
		};
		println!("eth_call at: {:?}", block_number);
		test_storage::CallAt::get(block_number, target).ok_or(BridgeRpcError::HttpFetch)
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
	type ValidatorIdOf = NoopConverter<TestRuntime>;
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

pub struct MockEventRouter;
impl EthereumEventRouter for MockEventRouter {
	fn route(_source: &H160, _destination: &H160, _data: &[u8]) -> EventRouterResult {
		Ok(Weight::from_ref_time(1000))
	}
}

pub struct MockEthCallSubscriber;
impl EthCallOracleSubscriber for MockEthCallSubscriber {
	type CallId = EthCallId;
	/// Stores the successful call info
	/// Available via `Self::success_result_for()`
	fn on_eth_call_complete(
		call_id: Self::CallId,
		return_data: &[u8; 32],
		block_number: u64,
		block_timestamp: u64,
	) {
		test_storage::LastCallResult::put((
			call_id,
			CheckedEthCallResult::Ok(*return_data, block_number, block_timestamp),
		));
	}
	/// Stores the failed call info
	/// Available via `Self::failed_call_for()`
	fn on_eth_call_failed(call_id: Self::CallId, reason: EthCallFailure) {
		test_storage::LastCallFailure::put((call_id, reason));
	}
}

impl MockEthCallSubscriber {
	/// Returns last known successful call, if any
	pub fn success_result() -> Option<(EthCallId, CheckedEthCallResult)> {
		test_storage::LastCallResult::get()
	}
	/// Returns last known failed call, if any
	pub fn failed_result() -> Option<(EthCallId, EthCallFailure)> {
		test_storage::LastCallFailure::get()
	}
}

/// Mock final session tracker
pub struct MockFinalSessionTracker;
impl FinalSessionTracker for MockFinalSessionTracker {
	fn is_active_session_final() -> bool {
		// at block 100, or if we are forcing, the active session is final
		frame_system::Pallet::<TestRuntime>::block_number() == 100 || test_storage::Forcing::get()
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

impl frame_system::offchain::SigningTypes for TestRuntime {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for TestRuntime
where
	RuntimeCall: From<C>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = Extrinsic;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for TestRuntime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		_public: <Signature as Verify>::Signer,
		_account: AccountId,
		nonce: u64,
	) -> Option<(RuntimeCall, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
		Some((call, (nonce, ())))
	}
}

#[derive(Clone, Copy, Default)]
pub struct ExtBuilder {
	relayer: Option<AccountId>,
	with_keystore: bool,
	next_session_final: bool,
	active_session_final: bool,
	endowed_account: Option<(AccountId, Balance)>,
	xrp_door_signer: Option<[u8; 33]>,
}

impl ExtBuilder {
	pub fn relayer(&mut self, relayer: H160) -> &mut Self {
		self.relayer = Some(relayer.into());
		self
	}
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
	pub fn with_endowed_account(mut self, account: H160, balance: Balance) -> Self {
		self.endowed_account = Some((AccountId::from(account), balance));
		self
	}
	pub fn xrp_door_signers(mut self, xrp_door_signer: [u8; 33]) -> Self {
		self.xrp_door_signer = Some(xrp_door_signer);
		self
	}
	pub fn build(self) -> sp_io::TestExternalities {
		let mut ext =
			frame_system::GenesisConfig::default().build_storage::<TestRuntime>().unwrap();

		let mut endowed_accounts: Vec<(AccountId, Balance)> = vec![];
		if self.endowed_account.is_some() {
			// Endow specified account
			endowed_accounts.push(self.endowed_account.unwrap());
		}
		if let Some(relayer) = self.relayer {
			// Endow relayer with relayerbond amount
			endowed_accounts.push((relayer, RelayerBond::get()));
		}

		if !endowed_accounts.is_empty() {
			pallet_balances::GenesisConfig::<TestRuntime> { balances: endowed_accounts }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

		if self.xrp_door_signer.is_some() {
			let xrp_door_signers: Vec<AuthorityId> =
				vec![AuthorityId::from_slice(self.xrp_door_signer.unwrap().as_slice()).unwrap()];
			pallet_ethy::GenesisConfig::<TestRuntime> { xrp_door_signers }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

		let mut ext: sp_io::TestExternalities = ext.into();

		ext.execute_with(|| System::initialize(&1, &[0u8; 32].into(), &Default::default()));

		if let Some(relayer) = self.relayer {
			ext.execute_with(|| {
				assert!(
					EthBridge::deposit_relayer_bond(RuntimeOrigin::signed(relayer.into())).is_ok()
				);
				assert!(EthBridge::set_relayer(RuntimeOrigin::root(), relayer).is_ok());
			});
		}

		if self.with_keystore {
			let keystore = KeyStore::new();
			SyncCryptoStore::ecdsa_generate_new(&keystore, AuthorityId::ID, None).unwrap();
			ext.register_extension(KeystoreExt(Arc::new(keystore)));
		}

		if self.next_session_final {
			ext.execute_with(|| frame_system::Pallet::<TestRuntime>::set_block_number(1));
		} else if self.active_session_final {
			ext.execute_with(|| frame_system::Pallet::<TestRuntime>::set_block_number(100));
		}

		ext
	}
}

#[test]
fn get_block_by_number_mock_works() {
	ExtBuilder::default().build().execute_with(|| {
		let block_number: u64 = 120;
		let block_hash: H256 = H256::from_low_u64_be(121);
		let timestamp: U256 = U256::from(122);

		let mock_block = EthBlock {
			number: Some(U64::from(block_number)),
			hash: Some(block_hash),
			timestamp,
			..Default::default()
		};

		MockEthereumRpcClient::mock_block_response_at(block_number, mock_block.clone());

		let result = <MockEthereumRpcClient as BridgeEthereumRpcApi>::get_block_by_number(
			LatestOrNumber::Number(block_number),
		)
		.unwrap();
		assert_eq!(Some(mock_block), result);
	});
}

#[test]
fn mock_eth_call_at_latest_block() {
	ExtBuilder::default().build().execute_with(|| {
		for i in 0..10_u64 {
			let mock_block = EthBlock {
				number: Some(U64::from(i)),
				hash: Some(H256::from_low_u64_be(i)),
				..Default::default()
			};
			MockEthereumRpcClient::mock_block_response_at(i, mock_block.clone());
		}
		// checking this returns latest block
		MockEthereumRpcClient::mock_call_at(9, EthAddress::from_low_u64_be(1), &[1_u8, 2, 3]);

		assert_eq!(
			MockEthereumRpcClient::eth_call(
				EthAddress::from_low_u64_be(1),
				&[4_u8, 5, 6],
				LatestOrNumber::Latest
			),
			Ok(vec![1_u8, 2, 3])
		);
	});
}

#[test]
fn get_latest_block_by_number_mock_works() {
	ExtBuilder::default().build().execute_with(|| {
		let block_number = 12;

		let mock_block = EthBlock {
			number: Some(U64::from(block_number)),
			hash: Some(H256::default()),
			timestamp: U256::default(),
			..Default::default()
		};
		MockEthereumRpcClient::mock_block_response_at(block_number, mock_block.clone());

		let result = <MockEthereumRpcClient as BridgeEthereumRpcApi>::get_block_by_number(
			LatestOrNumber::Latest,
		)
		.unwrap();
		assert_eq!(Some(mock_block), result);
	});
}

#[test]
fn get_transaction_receipt_mock_works() {
	ExtBuilder::default().build().execute_with(|| {
		let block_number: u64 = 120;
		let block_hash: H256 = H256::from_low_u64_be(121);
		let tx_hash: EthHash = H256::from_low_u64_be(122);
		let status: U64 = U64::from(1);
		let source_address: EthAddress = H160::from_low_u64_be(123);
		let default_log = Log {
			address: source_address,
			topics: vec![Default::default()],
			transaction_hash: Some(tx_hash),
			..Default::default()
		};

		let mock_tx_receipt = TransactionReceipt {
			block_hash,
			block_number: U64::from(block_number),
			logs: vec![default_log],
			status: Some(status),
			to: Some(source_address),
			transaction_hash: tx_hash,
			..Default::default()
		};

		MockEthereumRpcClient::mock_transaction_receipt_for(tx_hash, mock_tx_receipt.clone());

		let result =
			<MockEthereumRpcClient as BridgeEthereumRpcApi>::get_transaction_receipt(tx_hash)
				.unwrap();
		assert_eq!(Some(mock_tx_receipt), result);
	});
}
