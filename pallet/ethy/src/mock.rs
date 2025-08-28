// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

use crate::{
	self as pallet_ethy,
	types::{
		BridgeEthereumRpcApi, BridgeRpcError, CheckedEthCallRequest, CheckedEthCallResult,
		EthAddress, EthBlock, EthCallId, EthHash, LatestOrNumber, Log, TransactionReceipt,
	},
	Config,
};
use codec::{Decode, Encode};
use ethereum_types::U64;
use frame_support::{
	pallet_prelude::{OptionQuery, ValueQuery},
	storage_alias,
	traits::{UnixTime, ValidatorSet as ValidatorSetT},
	Identity, Twox64Concat,
};
use scale_info::TypeInfo;
use seed_pallet_common::test_prelude::*;
use seed_primitives::{
	ethy::{crypto::AuthorityId, EventProofId},
	Signature,
};
use sp_application_crypto::RuntimeAppPublic;
use sp_core::{ByteArray, Get};
use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
use sp_runtime::{
	testing::TestXt,
	traits::{Convert, Extrinsic as ExtrinsicT, Verify},
	Percent,
};
use std::{
	sync::Arc,
	time::{SystemTime, UNIX_EPOCH},
};

pub type SessionIndex = u32;
pub type Extrinsic = TestXt<RuntimeCall, ()>;

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		EthBridge: pallet_ethy,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		Scheduler: pallet_scheduler,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);
impl_pallet_scheduler_config!(Test);

parameter_types! {
	pub const NotarizationThreshold: Percent = Percent::from_parts(66_u8);
	pub const BridgePalletId: PalletId = PalletId(*b"ethybrdg");
	pub const EpochDuration: u64 = 1000_u64;
	pub const ChallengerBond: Balance = 100;
	pub const RelayerBond: Balance = 202;
	pub const MaxXrplKeys: u8 = 8;
	pub const MaxNewSigners: u8 = 20;
	pub const AuthorityChangeDelay: BlockNumber = 75;
	pub const MaxAuthorities: u32 = 1000;
	pub const MaxEthData: u32 = 1024;
	pub const MaxChallenges: u32 = 100;
	pub const MaxMessagesPerBlock: u32 = 1000;
	pub const MaxCallRequests: u32 = 1000;
	pub const MaxProcessedMessageIds: u32 = 10;
}
impl Config for Test {
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
	type NativeAssetId = NativeAssetId;
	type RelayerBond = RelayerBond;
	type MaxXrplKeys = MaxXrplKeys;
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type MaxNewSigners = MaxNewSigners;
	type XrplBridgeAdapter = MockXrplBridgeAdapter;
	type MaxAuthorities = MaxAuthorities;
	type MaxEthData = MaxEthData;
	type MaxChallenges = MaxChallenges;
	type MaxMessagesPerBlock = MaxMessagesPerBlock;
	type MaxCallRequests = MaxCallRequests;
	type WeightInfo = ();
	type MaxProcessedMessageIds = MaxProcessedMessageIds;
	/// No-op merge in tests
	type FrontierLogMerge = crate::NoFrontierMerge;
}

pub struct MockXrplBridgeAdapter;
impl EthyToXrplBridgeAdapter<H160> for MockXrplBridgeAdapter {
	/// Mock implementation of EthyToXrplBridgeAdapter
	fn submit_signer_list_set_request(
		_: Vec<(H160, u16)>,
	) -> Result<Vec<EventProofId>, DispatchError> {
		Ok(vec![1])
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

impl From<MockLog> for Log {
	fn from(value: MockLog) -> Log {
		Log {
			address: value.address,
			data: value.data,
			topics: value.topics,
			transaction_hash: value.transaction_hash,
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

#[storage_alias]
pub type BlockResponseAt<Test: Config> =
	StorageMap<crate::Pallet<Test>, Identity, u64, MockBlockResponse>;

#[storage_alias]
pub type TransactionReceiptFor<Test: Config> =
	StorageMap<crate::Pallet<Test>, Twox64Concat, EthHash, MockReceiptResponse>;

#[storage_alias]
pub type CallAt<Test: Config> =
	StorageDoubleMap<crate::Pallet<Test>, Twox64Concat, u64, Twox64Concat, EthAddress, Vec<u8>>;

#[storage_alias]
pub type Forcing<Test: Config> = StorageValue<crate::Pallet<Test>, bool, ValueQuery>;

#[storage_alias]
pub type Validators<Test: Config> = StorageValue<crate::Pallet<Test>, Vec<AccountId>, ValueQuery>;

#[storage_alias]
pub type Timestamp<Test: Config> = StorageValue<crate::Pallet<Test>, u64, OptionQuery>;

#[storage_alias]
pub type LastCallResult<Test: Config> =
	StorageValue<crate::Pallet<Test>, (EthCallId, CheckedEthCallResult), OptionQuery>;

#[storage_alias]
pub type LastCallFailure<Test: Config> =
	StorageValue<crate::Pallet<Test>, (EthCallId, EthCallFailure), OptionQuery>;

/// set the block timestamp
pub fn mock_timestamp(now: u64) {
	Timestamp::<Test>::put(now)
}

// get the system unix timestamp in seconds
pub fn now() -> u64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("after unix epoch")
		.as_secs()
}

/// Builder for `CheckedEthCallRequest`
pub struct CheckedEthCallRequestBuilder<MaxEthData: Get<u32>>(CheckedEthCallRequest<MaxEthData>);

impl<MaxEthData: Get<u32>> CheckedEthCallRequestBuilder<MaxEthData> {
	pub fn new() -> Self {
		Self(CheckedEthCallRequest {
			max_block_look_behind: 3_u64,
			target: EthAddress::from_low_u64_be(1),
			timestamp: now(),
			check_timestamp: now() + 3 * 5, // 3 blocks
			try_block_number: 0,
			input: BoundedVec::truncate_from(vec![]),
		})
	}
	pub fn build(self) -> CheckedEthCallRequest<MaxEthData> {
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
		BlockResponseAt::<Test>::insert(block_number, mock_block_response)
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
		TransactionReceiptFor::<Test>::insert(tx_hash, mock_receipt_response);
	}
	/// setup a mock returndata for an `eth_call` at `block` and `contract` address
	pub fn mock_call_at(block_number: u64, contract: H160, return_data: &[u8]) {
		CallAt::<Test>::insert(block_number, contract, return_data.to_vec())
	}
}

impl BridgeEthereumRpcApi for MockEthereumRpcClient {
	/// Returns an ethereum block given a block height
	fn get_block_by_number(
		block_number: LatestOrNumber,
	) -> Result<Option<EthBlock>, BridgeRpcError> {
		let mock_block_response = match block_number {
			LatestOrNumber::Latest => BlockResponseAt::<Test>::iter().last().map(|x| x.1).or(None),
			LatestOrNumber::Number(block) => BlockResponseAt::<Test>::get(block),
		};
		if mock_block_response.is_none() {
			return Ok(None);
		}
		let mock_block_response = mock_block_response.unwrap();

		let eth_block = EthBlock {
			number: Some(U64::from(mock_block_response.block_number)),
			hash: Some(mock_block_response.block_hash),
			timestamp: mock_block_response.timestamp,
			..Default::default()
		};
		Ok(Some(eth_block))
	}
	/// Returns an ethereum transaction receipt given a tx hash
	fn get_transaction_receipt(
		hash: EthHash,
	) -> Result<Option<TransactionReceipt>, BridgeRpcError> {
		let mock_receipt = TransactionReceiptFor::<Test>::get(hash);
		if mock_receipt.is_none() {
			return Ok(None);
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
			LatestOrNumber::Latest => {
				BlockResponseAt::<Test>::iter().last().unwrap().1.block_number
			},
		};
		CallAt::<Test>::get(block_number, target).ok_or(BridgeRpcError::HttpFetch)
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
		Validators::<Test>::get()
	}
}
impl MockValidatorSet {
	/// Mock n validator stashes
	pub fn mock_n_validators(n: u8) {
		let validators: Vec<AccountId> =
			(1..=n as u64).map(|i| H160::from_low_u64_be(i).into()).collect();
		Validators::<Test>::put(validators);
	}
}

pub struct MockEventRouter;
impl EthereumEventRouter for MockEventRouter {
	fn route(_source: &H160, _destination: &H160, _data: &[u8]) -> EventRouterResult {
		Ok(Weight::from_all(1000))
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
		LastCallResult::<Test>::put((
			call_id,
			CheckedEthCallResult::Ok(*return_data, block_number, block_timestamp),
		));
	}
	/// Stores the failed call info
	/// Available via `Self::failed_call_for()`
	fn on_eth_call_failed(call_id: Self::CallId, reason: EthCallFailure) {
		LastCallFailure::<Test>::put((call_id, reason))
	}
}

impl MockEthCallSubscriber {
	/// Returns last known successful call, if any
	pub fn success_result() -> Option<(EthCallId, CheckedEthCallResult)> {
		LastCallResult::<Test>::get()
	}
	/// Returns last known failed call, if any
	pub fn failed_result() -> Option<(EthCallId, EthCallFailure)> {
		LastCallFailure::<Test>::get()
	}
}

/// Mock final session tracker
pub struct MockFinalSessionTracker;
impl FinalSessionTracker for MockFinalSessionTracker {
	fn is_active_session_final() -> bool {
		// at block 100, or if we are forcing, the active session is final
		let forcing: bool = Forcing::<Test>::get();
		frame_system::Pallet::<Test>::block_number() == 100 || forcing
	}
}

/// Returns a fake timestamp based on the current block number
pub struct MockUnixTime;
impl UnixTime for MockUnixTime {
	fn now() -> core::time::Duration {
		match Timestamp::<Test>::get() {
			// Use configured value for tests requiring precise timestamps
			Some(s) => core::time::Duration::new(s, 0),
			// fallback, use block number to derive timestamp for tests that only care abut block
			// progression
			None => core::time::Duration::new(System::block_number() * 5, 0),
		}
	}
}

impl frame_system::offchain::SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	RuntimeCall: From<C>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = Extrinsic;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		_public: <Signature as Verify>::Signer,
		_account: AccountId,
		nonce: u32,
	) -> Option<(RuntimeCall, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
		Some((call, (nonce as u64, ())))
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
		let mut ext = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

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
			pallet_balances::GenesisConfig::<Test> { balances: endowed_accounts }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

		if self.xrp_door_signer.is_some() {
			let xrp_door_signers: Vec<AuthorityId> =
				vec![AuthorityId::from_slice(self.xrp_door_signer.unwrap().as_slice()).unwrap()];
			pallet_ethy::GenesisConfig::<Test> { xrp_door_signers }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

		let mut ext: sp_io::TestExternalities = ext.into();

		ext.execute_with(|| System::initialize(&1, &[0u8; 32].into(), &Default::default()));

		if let Some(relayer) = self.relayer {
			ext.execute_with(|| {
				assert!(EthBridge::deposit_relayer_bond(RuntimeOrigin::signed(relayer)).is_ok());
				assert!(EthBridge::set_relayer(RuntimeOrigin::root(), relayer).is_ok());
			});
		}

		if self.with_keystore {
			let keystore = MemoryKeystore::new();
			Keystore::ecdsa_generate_new(&keystore, AuthorityId::ID, None).unwrap();
			ext.register_extension(KeystoreExt(Arc::new(keystore)));
		}

		if self.next_session_final {
			ext.execute_with(|| frame_system::Pallet::<Test>::set_block_number(1));
		} else if self.active_session_final {
			ext.execute_with(|| frame_system::Pallet::<Test>::set_block_number(100));
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
