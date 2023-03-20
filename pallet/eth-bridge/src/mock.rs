/* Copyright 2019-2021 Centrality Investments Limited
 *
 * Licensed under the LGPL, Version 3.0 (the "License");
 * you may not use this file except in compliance with the License.
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * You may obtain a copy of the License at the root of this project source code,
 * or at:
 *     https://centrality.ai/licenses/gplv3.txt
 *     https://centrality.ai/licenses/lgplv3.txt
 */
use crate::{
	self as pallet_eth_bridge,
	types::{
		BridgeEthereumRpcApi, BridgeRpcError, CheckedEthCallRequest, CheckedEthCallResult,
		EthBlock, EthCallId, EthHash, LatestOrNumber, Log, TransactionReceipt,
	},
	Config,
};
use codec::{Decode, Encode};
use ethereum_types::U64;
use frame_support::{
	parameter_types,
	storage::{StorageDoubleMap, StorageValue},
	traits::UnixTime,
	IterableStorageMap, PalletId,
};
use frame_system::EnsureRoot;
use scale_info::TypeInfo;
use seed_pallet_common::{
	ethy::{EthyAdapter, EthySigningRequest, State},
	validator_set::ValidatorSetAdapter,
	EthCallFailure, EthCallOracleSubscriber, EthereumEventRouter, EventRouterResult,
};
use seed_primitives::{
	ethy::{crypto::AuthorityId, EventProofId, ValidatorSetId},
	AssetId, Balance, EthAddress, Signature,
};
use sp_api_hidden_includes_construct_runtime::hidden_include::StorageMap;
use sp_core::{ByteArray, H160, H256, U256};
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{
		BlakeTwo256, Convert, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify,
	},
	DispatchError, Percent, RuntimeAppPublic,
};
use std::{
	default::Default,
	sync::Arc,
	time::{SystemTime, UNIX_EPOCH},
};

pub const XRP_ASSET_ID: AssetId = 1;
type BlockNumber = u64;
pub type SessionIndex = u32;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Extrinsic = TestXt<Call, ()>;
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
		EthBridge: pallet_eth_bridge::{Pallet, Call, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Assets: pallet_assets::{Pallet, Storage, Config<T>, Event<T>},
		AssetsExt: pallet_assets_ext::{Pallet, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}
impl frame_system::Config for TestRuntime {
	type BlockWeights = ();
	type BlockLength = ();
	type BaseCallFilter = frame_support::traits::Everything;
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type BlockHashCount = BlockHashCount;
	type Event = Event;
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
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for TestRuntime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ();
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
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
	pub const NativeAssetId: AssetId = 1;
	pub const AssetsExtPalletId: PalletId = PalletId(*b"assetext");
	pub const MaxHolds: u32 = 16;
	pub const TestParachainId: u32 = 100;
}

impl pallet_assets_ext::Config for TestRuntime {
	type Event = Event;
	type ParachainId = TestParachainId;
	type MaxHolds = MaxHolds;
	type NativeAssetId = NativeAssetId;
	type OnNewAssetSubscription = ();
	type PalletId = AssetsExtPalletId;
	type WeightInfo = ();
}

parameter_types! {
	pub const NotarizationThreshold: Percent = Percent::from_parts(66_u8);
	pub const EthBridgePalletId: PalletId = PalletId(*b"eth-brdg");
	pub const EpochDuration: u64 = 1000_u64;
	pub const ChallengerBond: Balance = 100;
	pub const RelayerBond: Balance = 202;
	pub const XrpAssetId: AssetId = XRP_ASSET_ID;
	pub const MaxXrplKeys: u8 = 8;
	pub const MaxNewSigners: u8 = 20;
	pub const ValidatorChangeDelay: BlockNumber = 75;
}
impl Config for TestRuntime {
	type Event = Event;
	type PalletId = EthBridgePalletId;
	type RelayerBond = RelayerBond;
	type NativeAssetId = XrpAssetId;
	type MultiCurrency = AssetsExt;
	type ChallengeBond = ChallengerBond;
	type ValidatorSet = MockValidatorSetAdapter;
	type EthyAdapter = MockEthyAdapter;
	type NotarizationThreshold = NotarizationThreshold;
	type EventRouter = MockEventRouter;
	type EthCallSubscribers = MockEthCallSubscriber;
	type RpcClient = MockEthereumRpcClient;
	type Call = Call;
	type UnixTime = MockUnixTime;
	type WeightInfo = ();
}

pub struct MockValidatorSetAdapter;
impl ValidatorSetAdapter<AuthorityId> for MockValidatorSetAdapter {
	fn get_validator_set_id() -> ValidatorSetId {
		0
	}

	fn get_validator_set() -> Vec<AuthorityId> {
		test_storage::Validators::get()
	}

	fn get_next_validator_set() -> Vec<AuthorityId> {
		vec![
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
		]
	}

	fn get_xrpl_validator_set() -> Vec<AuthorityId> {
		vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
		]
	}

	fn get_xrpl_door_signers() -> Vec<AuthorityId> {
		vec![
			AuthorityId::from_slice(&[1_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[2_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[3_u8; 33]).unwrap(),
			AuthorityId::from_slice(&[4_u8; 33]).unwrap(),
		]
	}

	fn get_xrpl_notary_keys(validator_list: &Vec<AuthorityId>) -> Vec<AuthorityId> {
		let xrpl_door_signers = Self::get_xrpl_door_signers();
		validator_list
			.into_iter()
			.filter(|validator| xrpl_door_signers.contains(validator))
			.map(|validator| -> AuthorityId { validator.clone() })
			.take(8)
			.collect()
	}

	fn set_validator_set(validator_set: Vec<AuthorityId>) {
		test_storage::Validators::put(validator_set);
	}
}

impl MockValidatorSetAdapter {
	/// Mock n validator stashes
	pub fn mock_n_validators(n: u8) {
		let validators: Vec<AuthorityId> =
			(1..=n as u8).map(|i| AuthorityId::from_slice(&[i; 33]).unwrap()).collect();
		test_storage::Validators::put(validators);
	}

	pub fn add_to_validator_set(validator: &AuthorityId) {
		test_storage::Validators::append(validator);
	}
}

pub struct MockEthyAdapter;
impl EthyAdapter for MockEthyAdapter {
	fn request_for_proof(
		_request: EthySigningRequest,
		_event_proof_id: Option<EventProofId>,
	) -> Result<EventProofId, DispatchError> {
		Ok(1)
	}

	fn get_ethy_state() -> State {
		State::Active
	}

	fn get_next_event_proof_id() -> EventProofId {
		1 + 1
	}
}

pub struct NoopConverter<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> Convert<T::AccountId, Option<T::AccountId>> for NoopConverter<T> {
	fn convert(address: T::AccountId) -> Option<T::AccountId> {
		Some(address)
	}
}

pub struct MockEventRouter;
impl EthereumEventRouter for MockEventRouter {
	fn route(_source: &H160, _destination: &H160, _data: &[u8]) -> EventRouterResult {
		Ok(1000)
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
	use seed_primitives::ethy::crypto::AuthorityId;

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
			pub Validators: Vec<AuthorityId>;
			pub LastCallResult: Option<(EthCallId, CheckedEthCallResult)>;
			pub LastCallFailure: Option<(EthCallId, EthCallFailure)>;
		}
	}
}

impl frame_system::offchain::SigningTypes for TestRuntime {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for TestRuntime
where
	Call: From<C>,
{
	type Extrinsic = Extrinsic;
	type OverarchingCall = Call;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for TestRuntime
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

#[derive(Clone, Copy, Default)]
pub struct ExtBuilder {
	relayer: Option<AccountId>,
	with_keystore: bool,
	endowed_account: Option<(AccountId, Balance)>,
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
	pub fn with_endowed_account(mut self, account: H160, balance: Balance) -> Self {
		self.endowed_account = Some((AccountId::from(account), balance));
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

		let mut ext: sp_io::TestExternalities = ext.into();

		ext.execute_with(|| System::initialize(&1, &[0u8; 32].into(), &Default::default()));

		if let Some(relayer) = self.relayer {
			ext.execute_with(|| {
				assert!(EthBridge::deposit_relayer_bond(Origin::signed(relayer.into())).is_ok());
				assert!(EthBridge::set_relayer(Origin::root(), relayer).is_ok());
			});
		}

		if self.with_keystore {
			let keystore = KeyStore::new();
			SyncCryptoStore::ecdsa_generate_new(&keystore, AuthorityId::ID, None).unwrap();
			ext.register_extension(KeystoreExt(Arc::new(keystore)));
		}

		ext
	}
}
