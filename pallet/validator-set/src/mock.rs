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
use std::{
	default::Default,
	sync::Arc,
	time::{SystemTime, UNIX_EPOCH},
};

use codec::{Decode, Encode};
use frame_support::{
	parameter_types,
	storage::{StorageDoubleMap, StorageValue},
	traits::{GenesisBuild, UnixTime, ValidatorSet as ValidatorSetT},
	PalletId,
};
use frame_system::EnsureRoot;
use scale_info::TypeInfo;
use sp_core::{ByteArray, H160, H256, U256};
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{
		BlakeTwo256, Convert, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify,
	},
	DispatchError, Percent, RuntimeAppPublic,
};

use seed_pallet_common::{
	ethy::{
		BridgeAdapter, EthereumBridgeAdapter, EthyAdapter, EthySigningRequest, State,
		XRPLBridgeAdapter,
	},
	validator_set::{ValidatorSetChangeHandler, ValidatorSetChangeInfo},
	FinalSessionTracker,
};
use seed_primitives::{
	ethy::{crypto::AuthorityId, EventProofId},
	xrpl::XrplAccountId,
	AssetId, Balance, EthAddress, Signature,
};

use crate::{self as pallet_validator_set, Config};

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
		ValidatorSet: pallet_validator_set::{Pallet, Call, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
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
	pub const NotarizationThreshold: Percent = Percent::from_parts(66_u8);
	pub const ValidatorSetPalletId: PalletId = PalletId(*b"valdtrst");
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
	type PalletId = ValidatorSetPalletId;
	type EthyId = AuthorityId;
	type EpochDuration = EpochDuration;
	type ValidatorChangeDelay = ValidatorChangeDelay;
	type FinalSessionTracker = MockFinalSessionTracker;
	type Scheduler = Scheduler;
	type Call = Call;
	type PalletsOrigin = OriginCaller;
	type MaxXrplKeys = MaxXrplKeys;
	type EthyAdapter = MockEthyAdapter;
	type XRPLBridgeAdapter = MockXrplBridgeAdapter;
	type EthBridgeAdapter = MockEthBridgeAdapter;
	type MaxNewSigners = MaxNewSigners;
	type WeightInfo = ();
}

pub struct MockEthyAdapter;
impl EthyAdapter for MockEthyAdapter {
	fn request_for_proof(
		request: EthySigningRequest,
		event_proof_id: Option<EventProofId>,
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
impl ValidatorSetChangeHandler<AuthorityId> for MockEthyAdapter {
	fn validator_set_change_in_progress(info: ValidatorSetChangeInfo<AuthorityId>) {
		()
	}

	fn validator_set_change_finalized(info: ValidatorSetChangeInfo<AuthorityId>) {
		()
	}
}

pub struct MockXrplBridgeAdapter;
impl BridgeAdapter for MockXrplBridgeAdapter {
	fn get_pallet_id() -> PalletId {
		PalletId(*b"xrplbrdg")
	}
}
impl XRPLBridgeAdapter for MockXrplBridgeAdapter {
	fn get_signer_list_set_payload(_: Vec<(XrplAccountId, u16)>) -> Result<Vec<u8>, DispatchError> {
		Ok(Vec::default())
	}
}

pub struct MockEthBridgeAdapter;
impl BridgeAdapter for MockEthBridgeAdapter {
	fn get_pallet_id() -> PalletId {
		PalletId(*b"eth-brdg")
	}
}
impl EthereumBridgeAdapter for MockEthBridgeAdapter {
	fn get_contract_address() -> EthAddress {
		EthAddress::from_low_u64_be(1)
	}

	fn get_notarization_threshold() -> Percent {
		Percent::from_parts(66_u8)
	}
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
	pub const MaxScheduledPerBlock: u32 = 50;
}
impl pallet_scheduler::Config for TestRuntime {
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	type Call = Call;
	type MaximumWeight = ();
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type OriginPrivilegeCmp = frame_support::traits::EqualPrivilegeOnly;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = ();
	type PreimageProvider = ();
	type NoPreimagePostponement = ();
}

pub(crate) mod test_storage {
	//! storage used by tests to store mock EthBlocks and TransactionReceipts
	use super::AccountId;
	use crate::Config;
	use frame_support::decl_storage;

	pub struct Module<T>(sp_std::marker::PhantomData<T>);
	decl_storage! {
		trait Store for Module<T: Config> as EthBridgeTest {
			pub Timestamp: Option<u64>;
			pub Validators: Vec<AccountId>;
			pub Forcing: bool;
		}
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
	with_keystore: bool,
	next_session_final: bool,
	active_session_final: bool,
	endowed_account: Option<(AccountId, Balance)>,
	xrp_door_signer: Option<[u8; 33]>,
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

		if !endowed_accounts.is_empty() {
			pallet_balances::GenesisConfig::<TestRuntime> { balances: endowed_accounts }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

		if self.xrp_door_signer.is_some() {
			let xrp_door_signers: Vec<AuthorityId> =
				vec![AuthorityId::from_slice(self.xrp_door_signer.unwrap().as_slice()).unwrap()];
			pallet_validator_set::GenesisConfig::<TestRuntime> { xrp_door_signers }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

		let mut ext: sp_io::TestExternalities = ext.into();

		ext.execute_with(|| System::initialize(&1, &[0u8; 32].into(), &Default::default()));

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
