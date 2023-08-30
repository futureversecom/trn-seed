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

use crate::{self as pallet_staking_payouts, BalanceOf, Config};
use _feps::SortedListProvider;
use frame_election_provider_support::{generate_solution_type, onchain, SequentialPhragmen};
use frame_support::{
	assert_ok, parameter_types,
	storage::StorageValue,
	traits::{Currency, GenesisBuild, Get, Hooks, OnInitialize, OneSessionHandler},
	PalletId,
};
use frame_system::EnsureRoot;

use codec::{Decode, Encode};
use pallet_session::SessionHandler;
use pallet_staking::{
	Bonded, EraPayout, ErasStakers, Nominators, RewardDestination, StakerStatus, StashOf,
	ValidatorPrefs, Validators,
};
use seed_pallet_common::{impl_pallet_assets_config, EventRouterError, EventRouterResult};

use seed_pallet_common::{
	impl_pallet_assets_ext_config, impl_pallet_balance_config, impl_pallet_timestamp_config,
};
use seed_primitives::{ethy::EventProofId, AccountId, AccountId20, AssetId, Balance};
use sp_core::{ecdsa, H160, H256};
use sp_npos_elections::VoteWeight;
use sp_runtime::{
	testing::{Header, UintAuthorityId},
	traits::{BlakeTwo256, ConstU128, ConstU64, IdentityLookup, OpaqueKeys, Zero},
	DispatchError, Perbill, SaturatedConversion,
};

use sp_staking::{EraIndex, SessionIndex};
use sp_std::collections::btree_map::BTreeMap;
use test_accounts::*;

pub type BlockNumber = u64;
pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
pub type Block = frame_system::mocking::MockBlock<TestRuntime>;

pub const BLOCK_TIME: u64 = 4000;

pub mod test_accounts {
	use seed_primitives::AccountId20;

	pub const ALICE: AccountId20 = AccountId20([1; 20]);
	pub const BOB: AccountId20 = AccountId20([2; 20]);
	pub const CHARLIE: AccountId20 = AccountId20([3; 20]);
	pub const DAVE: AccountId20 = AccountId20([4; 20]);
	pub const CONTROLLER_ONE: AccountId20 = AccountId20([5; 20]);
	pub const CONTROLLER_TWO: AccountId20 = AccountId20([6; 20]);
	pub const CONTROLLER_THREE: AccountId20 = AccountId20([7; 20]);
	pub const CONTROLLER_FOUR: AccountId20 = AccountId20([8; 20]);
	pub const CONTROLLER_FIVE: AccountId20 = AccountId20([9; 20]);
	pub const STASH_ONE: AccountId20 = AccountId20([10; 20]);
	pub const STASH_TWO: AccountId20 = AccountId20([11; 20]);
	pub const STASH_THREE: AccountId20 = AccountId20([12; 20]);
	pub const STASH_FOUR: AccountId20 = AccountId20([13; 20]);
	pub const STASH_FIVE: AccountId20 = AccountId20([14; 20]);
	pub const NOMINATOR_ONE: AccountId20 = AccountId20([15; 20]);
	pub const NOMINATOR_TWO: AccountId20 = AccountId20([16; 20]);
	pub const AUX_ACCOUNT_ONE: AccountId20 = AccountId20([17; 20]);
	pub const AUX_ACCOUNT_TWO: AccountId20 = AccountId20([18; 20]);
	pub const AUX_ACCOUNT_THREE: AccountId20 = AccountId20([19; 20]);
	pub const AUX_ACCOUNT_FOUR: AccountId20 = AccountId20([20; 20]);
	pub const AUX_ACCOUNT_FIVE: AccountId20 = AccountId20([21; 20]);
	pub const AUX_ACCOUNT_SIX: AccountId20 = AccountId20([22; 20]);
	pub const ACCOUNT_OTHER: AccountId20 = AccountId20([23; 20]);
}

fn mock_public_to_uint_auth_key_helper(public: sp_core::ecdsa::Public) -> UintAuthorityId {
	// For tests, should be a fake public key with uniform vector of one number, so just grab one of
	// them
	UintAuthorityId(public.0[0].into())
}

fn mock_account_to_uint_auth_key_helper(account_id: AccountId20) -> UintAuthorityId {
	// For tests, should be a fake account with uniform vector of one number, so just grab one of
	// them
	UintAuthorityId(account_id.0[0].into())
}

fn mock_account_to_public_helper(account_id: AccountId20) -> sp_core::ecdsa::Public {
	// For tests, should be a fake account with uniform vector of one number, so just grab one of
	// them
	sp_core::ecdsa::Public([account_id.0[0]; 33])
}

pub(crate) fn active_era() -> EraIndex {
	Staking::active_era().unwrap().index
}

pub(crate) fn current_era() -> EraIndex {
	Staking::current_era().unwrap()
}

pub(crate) fn bond(stash: AccountId, ctrl: AccountId, val: Balance) {
	let _ = Balances::make_free_balance_be(&stash, val);
	let _ = Balances::make_free_balance_be(&ctrl, val);
	assert_ok!(Staking::bond(Origin::signed(stash), ctrl, val, RewardDestination::Controller));
}

pub(crate) fn bond_validator(stash: AccountId, ctrl: AccountId, val: Balance) {
	bond(stash, ctrl, val);
	assert_ok!(Staking::validate(Origin::signed(ctrl), ValidatorPrefs::default()));
	// assert_ok!(Session::set_keys(Origin::signed(ctrl), SessionKeys { other: ctrl.into() },
	// vec![]));
	assert_ok!(Session::set_keys(
		Origin::signed(ctrl),
		SessionKeys { other: mock_account_to_public_helper(ctrl).into() },
		vec![]
	));
}

pub(crate) fn bond_nominator(
	stash: AccountId,
	ctrl: AccountId,
	val: Balance,
	target: Vec<AccountId>,
) {
	bond(stash, ctrl, val);
	assert_ok!(Staking::nominate(Origin::signed(ctrl), target));
}

/// Time it takes to finish a session.
///
/// Note, if you see `time_per_session() - BLOCK_TIME`, it is fine. This is because we set the
/// timestamp after on_initialize, so the timestamp is always one block old.
pub(crate) fn time_per_session() -> u64 {
	Period::get() * BLOCK_TIME
}

/// Time it takes to finish an era.
///
/// Note, if you see `time_per_era() - BLOCK_TIME`, it is fine. This is because we set the
/// timestamp after on_initialize, so the timestamp is always one block old.
pub(crate) fn time_per_era() -> u64 {
	time_per_session() * SessionsPerEra::get() as u64
}

/// Time that will be calculated for the reward per era.
pub(crate) fn reward_time_per_era() -> u64 {
	time_per_era() - BLOCK_TIME
}

/// Progress to the given block, triggering session and era changes as we progress.
///
/// This will finalize the previous block, initialize up to the given block, essentially simulating
/// a block import/propose process where we first initialize the block, then execute some stuff (not
/// in the function), and then finalize the block.
pub(crate) fn run_to_block(n: BlockNumber) {
	Staking::on_finalize(System::block_number());
	for b in (System::block_number() + 1)..=n {
		System::set_block_number(b);
		<Session as Hooks<u64>>::on_initialize(b);
		<Staking as Hooks<u64>>::on_initialize(b);
		<StakingPayout as Hooks<u64>>::on_initialize(b);
		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
		if b != n {
			Staking::on_finalize(System::block_number());
		}
	}
}

/// Progresses from the current block number (whatever that may be) to the `P * session_index + 1`.
pub(crate) fn start_session(session_index: SessionIndex) {
	let end: u64 = if Offset::get().is_zero() {
		(session_index as u64) * Period::get()
	} else {
		Offset::get() + (session_index.saturating_sub(1) as u64) * Period::get()
	};
	run_to_block(end);
	// // session must have progressed properly.
	assert_eq!(
		Session::current_index(),
		session_index,
		"current session index = {}, expected = {}",
		Session::current_index(),
		session_index,
	);
}

/// Progress until the given era.
pub(crate) fn start_active_era(era_index: EraIndex) {
	start_session((era_index * <SessionsPerEra as Get<u32>>::get()).into());
	assert_eq!(active_era(), era_index);
	// One way or another, current_era must have changed before the active era, so they must
	// match at this point.
	assert_eq!(current_era(), active_era());
}

pub(crate) fn current_total_payout_for_duration(duration: u64) -> Balance {
	let (payout, _rest) = <TestRuntime as pallet_staking::Config>::EraPayout::era_payout(
		Staking::eras_total_stake(active_era()),
		Balances::total_issuance(),
		duration,
	);
	assert!(payout > 0);
	payout
}

impl pallet_balances::Config for TestRuntime {
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = ();
}

impl_pallet_assets_config!(TestRuntime);
impl_pallet_assets_ext_config!(TestRuntime);
impl_pallet_timestamp_config!(TestRuntime);

impl pallet_tx_fee_pot::Config for TestRuntime {
	type FeeCurrency = XrpCurrency;
	type TxFeePotId = TxFeePotId;
}

frame_support::construct_runtime!(
	pub enum TestRuntime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances,
		Assets: pallet_assets,
		Staking: pallet_staking::{Pallet, Call, Storage, Event<T>},
		StakingPayout: pallet_staking_payouts::{Pallet, Storage, Event},
		AssetsExt: pallet_assets_ext,
		TxFeePot: pallet_tx_fee_pot::{Pallet, Storage},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		Historical: pallet_session::historical::{Pallet, Storage},
		BagsList: pallet_bags_list::{Pallet, Call, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
	}
);

const THRESHOLDS: [sp_npos_elections::VoteWeight; 9] =
	[10, 20, 30, 40, 50, 60, 1_000, 2_000, 10_000];

parameter_types! {
	pub static BagThresholds: &'static [sp_npos_elections::VoteWeight] = &THRESHOLDS;
	// pub static MaxNominations: u32 = 16;
	pub static RewardOnUnbalanceWasCalled: bool = false;
	pub static LedgerSlashPerEra: (BalanceOf<TestRuntime>, BTreeMap<EraIndex, BalanceOf<TestRuntime>>) = (Zero::zero(), BTreeMap::new());
}

impl pallet_bags_list::Config for TestRuntime {
	type Event = Event;
	type WeightInfo = ();
	type ScoreProvider = Staking;
	type BagThresholds = BagThresholds;
	type Score = VoteWeight;
}

impl pallet_session::historical::Config for TestRuntime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<TestRuntime>;
}

impl sp_runtime::BoundToRuntimeAppPublic for TestSessionHandler {
	// type Public = UintAuthorityId;
	type Public = sp_application_crypto::ecdsa::AppPublic;
}

sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		// pub other: OtherSessionHandler,
		pub other: TestSessionHandler,
	}
}

pub struct TestSessionHandler;
impl pallet_session::SessionHandler<AccountId> for TestSessionHandler {
	const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[sp_core::testing::ECDSA];

	fn on_genesis_session<Ks: sp_runtime::traits::OpaqueKeys>(_validators: &[(AccountId, Ks)]) {}

	fn on_new_session<Ks: sp_runtime::traits::OpaqueKeys>(
		_: bool,
		_: &[(AccountId, Ks)],
		_: &[(AccountId, Ks)],
	) {
	}

	fn on_disabled(_: u32) {}
}

impl pallet_session::Config for TestRuntime {
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<TestRuntime, Staking>;
	type Keys = SessionKeys;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionHandler = TestSessionHandler;
	type Event = Event;
	type ValidatorId = AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<TestRuntime>;
	type WeightInfo = ();
}

pub const ROOT_ASSET_ID: AssetId = 1;
pub const XRP_ASSET_ID: AssetId = 2;

parameter_types! {
	pub const BlockHashCount: u64 = 250;

	/// Getter for the ROOT asset Id
	pub const RootAssetId: seed_primitives::AssetId = ROOT_ASSET_ID;
	/// Getter for the XRP asset Id
	pub const XrpAssetId: seed_primitives::AssetId = XRP_ASSET_ID;
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

pub const MILLISECS_PER_BLOCK: u64 = 4_000;
pub const INIT_TIMESTAMP: u64 = 30_000;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

pub const EPOCH_DURATION_IN_SLOTS: BlockNumber = 4 * HOURS;
pub const ROOT_DECIMALS: u8 = 6;
pub const ONE_ROOT: Balance = (10 as Balance).pow(ROOT_DECIMALS as u32);

parameter_types! {
	// phase durations. 1/4 of the last session for each.
	// in testing: 1min or half of the session for each
	pub SignedPhase: u32 = (EPOCH_DURATION_IN_SLOTS / 4).try_into().unwrap();
	pub UnsignedPhase: u32 = (EPOCH_DURATION_IN_SLOTS / 4).try_into().unwrap();
	// signed config
	pub const SignedMaxSubmissions: u32 = 16;
	pub const SignedMaxRefunds: u32 = 16 / 4;
	// 40 DOTs fixed deposit..
	pub const SignedDepositBase: Balance = ONE_ROOT * 40;
	// 0.01 DOT per KB of solution data.
	pub const SignedDepositByte: Balance = ONE_ROOT / 1024;

	pub SignedRewardBase: Balance = 0;
	pub BetterUnsignedThreshold: Perbill = Perbill::from_rational(5u32, 10_000);
	// 4 hour session, 1 hour unsigned phase, 32 offchain executions.
	pub OffchainRepeat: BlockNumber =  (UnsignedPhase::get() / 32).into();
	/// We take the top 22500 nominators as electing voters..
	pub const MaxElectingVoters: u32 = 22_500;
	/// ... and all of the validators as electable targets. Whilst this is the case, we cannot and
	/// shall not increase the size of the validator intentions.
	pub const MaxElectableTargets: u16 = u16::MAX;
}

generate_solution_type!(
	#[compact]
	pub struct NposCompactSolution16::<
		VoterIndex = u32,
		TargetIndex = u16,
		Accuracy = sp_runtime::PerU16,
		MaxVoters = MaxElectingVoters,
	>(16)
);

parameter_types! {
	// Six sessions in an era (24 hours).
	pub const SessionsPerEra: sp_staking::SessionIndex = 6;
	// 28 eras for unbonding (28 days).
	pub const BondingDuration: sp_staking::EraIndex = 28;
	pub const SlashDeferDuration: sp_staking::EraIndex = 27;
	pub const MaxNominatorRewardedPerValidator: u32 = 256;
	pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(17);
	// 16
	pub const MaxNominations: u32 = <NposCompactSolution16 as
frame_election_provider_support::NposSolution>::LIMIT as u32; 	// holds XRP from staking slashes
	// this could be controlled by pallet-treasury later
	pub const SlashPotId: PalletId = PalletId(*b"slashpot");

	 pub const TxFeePotId: PalletId = PalletId(*b"txfeepot");
}
type SlashCancelOrigin = EnsureRoot<AccountId>;
pub type XrpCurrency = pallet_assets_ext::AssetCurrency<TestRuntime, XrpAssetId>;

/// Dual currency implementation mapped to ROOT & XRP for staking
pub type DualStakingCurrency =
	pallet_assets_ext::DualStakingCurrency<TestRuntime, XrpCurrency, Balances>;

impl<C> frame_system::offchain::SendTransactionTypes<C> for TestRuntime
where
	Call: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = Call;
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = TestRuntime;
	type Solver = SequentialPhragmen<AccountId, sp_runtime::Perbill>;
	type DataProvider = Staking;
	type WeightInfo = ();
}

impl pallet_staking::Config for TestRuntime {
	type MaxNominations = MaxNominations;
	type Currency = DualStakingCurrency;
	type CurrencyBalance = Balance;
	type CurrencyToVote = frame_support::traits::U128CurrencyToVote;
	// Decides the total reward to be distributed each era
	// For root network it is the balance of the tx fee pot
	type EraPayout = TxFeePot;
	type Event = Event;
	// After a validator payout is made (to it and all its stakers), this receives the pending
	// positive imbalance (total amount newly minted during the payout process) since the XRP
	// already exists the issuance should not be modified
	//
	// pallet-staking validator payouts always _mint_ tokens (with `deposit_creating`) assuming an
	// inflationary model instead rewards should be redistributed from fees only
	type Reward = TxFeePot;
	// Handles any era reward amount indivisible among stakers at end of an era.
	// some account should receive the amount to ensure total issuance of XRP is constant (vs.
	// burnt)
	type RewardRemainder = TxFeePot;
	// Upon slashing two situations can happen:
	// 1) if there are no reporters, this handler is given the whole slashed imbalance
	// 2) any indivisible slash imbalance (not sent to reporter(s)) is sent here
	// StakingPot nullifies the imbalance to keep issuance of XRP constant (vs. burnt)
	type Slash = ();
	type UnixTime = pallet_timestamp::Pallet<Self>;
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	// A super-majority of the council can cancel the slash.
	type SlashCancelOrigin = SlashCancelOrigin;
	type SessionInterface = Self;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
	type NextNewSession = Session;
	type ElectionProvider = onchain::UnboundedExecution<OnChainSeqPhragmen>;
	type GenesisElectionProvider = onchain::UnboundedExecution<OnChainSeqPhragmen>;
	// type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Self>;
	type VoterList = BagsList;
	type MaxUnlockingChunks = frame_support::traits::ConstU32<32>;
	type BenchmarkingConfig = pallet_staking::TestBenchmarkingConfig;
	type OnStakerSlash = ();
	type WeightInfo = pallet_staking::weights::SubstrateWeight<TestRuntime>;
}

parameter_types! {
	pub const PayoutPeriodLength: u32 = 90;

}
impl Config for TestRuntime {
	type Event = Event;
	type Currency = DualStakingCurrency;
	type CurrencyBalance = Balance;
	type PayoutPeriodLength = PayoutPeriodLength;
	type WeightInfo = ();
}

pub(crate) mod test_storage {
	//! storage used by tests to store mock EthBlocks and TransactionReceipts
	use crate::Config;
	use frame_support::decl_storage;
	use seed_primitives::ethy::EventProofId;

	pub struct Module<T>(sp_std::marker::PhantomData<T>);
	decl_storage! {
		trait Store for Module<T: Config> as EthBridgeTest {
			pub NextEventProofId: EventProofId;
		}
	}
}

#[allow(dead_code)]
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<TestRuntime>().unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 1;
}

parameter_types! {
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(
			frame_support::weights::constants::WEIGHT_PER_SECOND * 2
		);
	pub static Period: BlockNumber = 5;
	pub static Offset: BlockNumber = 0;
}

pub struct ExtBuilder {
	nominate: bool,
	validator_count: u32,
	minimum_validator_count: u32,
	invulnerables: Vec<AccountId>,
	has_stakers: bool,
	initialize_first_session: bool,
	pub min_nominator_bond: Balance,
	min_validator_bond: Balance,
	balance_factor: Balance,
	status: BTreeMap<AccountId, StakerStatus<AccountId>>,
	stakes: BTreeMap<AccountId, Balance>,
	stakers: Vec<(AccountId, AccountId, Balance, StakerStatus<AccountId>)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			nominate: true,
			validator_count: 2,
			minimum_validator_count: 0,
			balance_factor: 1,
			invulnerables: vec![],
			has_stakers: true,
			initialize_first_session: true,
			min_nominator_bond: ExistentialDeposit::get(),
			min_validator_bond: ExistentialDeposit::get(),
			status: Default::default(),
			stakes: Default::default(),
			stakers: Default::default(),
		}
	}
}

impl ExtBuilder {
	pub fn nominate(mut self, nominate: bool) -> Self {
		self.nominate = nominate;
		self
	}
	pub fn validator_count(mut self, count: u32) -> Self {
		self.validator_count = count;
		self
	}
	pub fn minimum_validator_count(mut self, count: u32) -> Self {
		self.minimum_validator_count = count;
		self
	}

	pub fn invulnerables(mut self, invulnerables: Vec<AccountId>) -> Self {
		self.invulnerables = invulnerables;
		self
	}

	pub fn period(self, length: BlockNumber) -> Self {
		PERIOD.with(|v| *v.borrow_mut() = length);
		self
	}
	pub fn has_stakers(mut self, has: bool) -> Self {
		self.has_stakers = has;
		self
	}
	pub fn initialize_first_session(mut self, init: bool) -> Self {
		self.initialize_first_session = init;
		self
	}
	pub fn offset(self, offset: BlockNumber) -> Self {
		OFFSET.with(|v| *v.borrow_mut() = offset);
		self
	}
	pub fn min_nominator_bond(mut self, amount: Balance) -> Self {
		self.min_nominator_bond = amount;
		self
	}
	pub fn min_validator_bond(mut self, amount: Balance) -> Self {
		self.min_validator_bond = amount;
		self
	}
	pub fn set_status(mut self, who: AccountId, status: StakerStatus<AccountId>) -> Self {
		self.status.insert(who, status);
		self
	}
	pub fn set_stake(mut self, who: AccountId, stake: Balance) -> Self {
		self.stakes.insert(who, stake);
		self
	}
	pub fn add_staker(
		mut self,
		stash: AccountId,
		ctrl: AccountId,
		stake: Balance,
		status: StakerStatus<AccountId>,
	) -> Self {
		self.stakers.push((stash, ctrl, stake, status));
		self
	}
	pub fn balance_factor(mut self, factor: Balance) -> Self {
		self.balance_factor = factor;
		self
	}

	pub(crate) fn validator_controllers() -> Vec<AccountId> {
		Session::validators()
			.into_iter()
			.map(|s| Staking::bonded(&s).expect("no controller for validator"))
			.collect()
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut storage =
			frame_system::GenesisConfig::default().build_storage::<TestRuntime>().unwrap();

		let _ = pallet_balances::GenesisConfig::<TestRuntime> {
			balances: vec![
				// (1, 10 * self.balance_factor),
				(ALICE, 10 * self.balance_factor),
				(BOB, 20 * self.balance_factor),
				(CHARLIE, 300 * self.balance_factor),
				(DAVE, 400 * self.balance_factor),
				// controllers
				(CONTROLLER_ONE, self.balance_factor),
				(CONTROLLER_TWO, self.balance_factor),
				(CONTROLLER_THREE, self.balance_factor),
				(CONTROLLER_FOUR, self.balance_factor),
				(CONTROLLER_FIVE, self.balance_factor),
				// stashes
				(STASH_ONE, self.balance_factor * 1000),
				(STASH_TWO, self.balance_factor * 2000),
				(STASH_THREE, self.balance_factor * 2000),
				(STASH_FOUR, self.balance_factor * 2000),
				(STASH_FIVE, self.balance_factor * 2000),
				// optional nominator
				(NOMINATOR_ONE, self.balance_factor * 2000),
				(NOMINATOR_TWO, self.balance_factor * 2000),
				// aux accounts
				(AUX_ACCOUNT_ONE, self.balance_factor),
				(AUX_ACCOUNT_TWO, self.balance_factor * 2000),
				(AUX_ACCOUNT_THREE, self.balance_factor),
				(AUX_ACCOUNT_FOUR, self.balance_factor * 2000),
				(AUX_ACCOUNT_FIVE, self.balance_factor),
				(AUX_ACCOUNT_SIX, self.balance_factor * 2000),
				// This allows us to have a total_payout different from 0.
				(ACCOUNT_OTHER, 1_000_000_000_000),
			],
		}
		.assimilate_storage(&mut storage);

		let mut stakers = vec![];
		if self.has_stakers {
			stakers = vec![
				// (stash, ctrl, stake, status)
				// these two will be elected in the default test where we elect 2.
				(
					STASH_ONE,
					CONTROLLER_ONE,
					self.balance_factor * 1000,
					StakerStatus::<AccountId>::Validator,
				),
				(
					STASH_TWO,
					CONTROLLER_TWO,
					self.balance_factor * 1000,
					StakerStatus::<AccountId>::Validator,
				),
				// a loser validator
				(
					STASH_THREE,
					CONTROLLER_THREE,
					self.balance_factor * 500,
					StakerStatus::<AccountId>::Validator,
				),
				// an idle validator
				(
					STASH_FOUR,
					CONTROLLER_FOUR,
					self.balance_factor * 1000,
					StakerStatus::<AccountId>::Idle,
				),
			];
			// optionally add a nominator
			if self.nominate {
				stakers.push((
					NOMINATOR_ONE,
					NOMINATOR_TWO,
					self.balance_factor * 500,
					StakerStatus::<AccountId>::Nominator(vec![STASH_ONE, STASH_TWO]),
				))
			}
			// replace any of the status if needed.
			self.status.into_iter().for_each(|(stash, status)| {
				let (_, _, _, ref mut prev_status) = stakers
					.iter_mut()
					.find(|s| s.to_owned().0 == stash)
					.expect("set_status staker should exist; qed");
				*prev_status = status;
			});

			// replaced any of the stakes if needed.
			self.stakes.into_iter().for_each(|(stash, stake)| {
				let (_, _, ref mut prev_stake, _) = stakers
					.iter_mut()
					.find(|s| s.0 == stash)
					.expect("set_stake staker should exits; qed.");
				*prev_stake = stake;
			});
			// extend stakers if needed.
			stakers.extend(self.stakers)
		}

		let _ = pallet_staking::GenesisConfig::<TestRuntime> {
			stakers: stakers.clone(),
			validator_count: self.validator_count,
			minimum_validator_count: self.minimum_validator_count,
			invulnerables: self.invulnerables,
			slash_reward_fraction: Perbill::from_percent(10),
			min_nominator_bond: self.min_nominator_bond,
			min_validator_bond: self.min_validator_bond,
			..Default::default()
		}
		.assimilate_storage(&mut storage);

		let _ = pallet_session::GenesisConfig::<TestRuntime> {
			keys: if self.has_stakers {
				// set the keys for the first session.
				stakers
					.into_iter()
					.map(|(id, ..)| {
						let id_encoded = id.encode();

						// Sample the (only) number from the account to use for a conversion to a
						// similarly fake public key
						let v = [id.0[0]; 33];

						// May need sp app crypto instead of core
						// sp_application_crypto::ecdsa
						let p = ecdsa::Public(v);

						(id, id, SessionKeys { other: mock_account_to_public_helper(id).into() })
					})
					.collect()
			} else {
				// set some dummy validators in genesis.
				(0..self.validator_count as u64)
					.map(|id| {
						let id = AccountId20([id.try_into().unwrap(); 20]);
						(id, id, SessionKeys { other: mock_account_to_public_helper(id).into() })
					})
					.collect()

				// self.stakers.iter().map
			},
		}
		.assimilate_storage(&mut storage);

		let mut ext = sp_io::TestExternalities::from(storage);

		if self.initialize_first_session {
			// We consider all test to start after timestamp is initialized This must be ensured by
			// having `timestamp::on_initialize` called before `staking::on_initialize`. Also, if
			// session length is 1, then it is already triggered.
			ext.execute_with(|| {
				System::set_block_number(1);
				<Session as Hooks<u64>>::on_initialize(1);
				<Staking as Hooks<u64>>::on_initialize(1);
				Timestamp::set_timestamp(INIT_TIMESTAMP);
			});
		}

		ext
	}

	pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
		let mut ext = self.build();
		ext.execute_with(test);
		ext.execute_with(post_conditions);
	}
}

fn post_conditions() {
	check_nominators();
	check_exposures();
	check_ledgers();
	check_count();
}

fn check_count() {
	let nominator_count = Nominators::<TestRuntime>::iter_keys().count() as u32;
	let validator_count = Validators::<TestRuntime>::iter().count() as u32;
	assert_eq!(nominator_count, Nominators::<TestRuntime>::count());
	assert_eq!(validator_count, Validators::<TestRuntime>::count());

	// the voters that the `VoterList` list is storing for us.
	let external_voters = <TestRuntime as pallet_staking::Config>::VoterList::count();
	assert_eq!(external_voters, nominator_count + validator_count);
}

fn check_ledgers() {
	// check the ledger of all stakers.
	// Bonded::<TestRuntime>::iter().for_each(|(_, ctrl)| assert_ledger_consistent(ctrl))
}

fn check_exposures() {
	// a check per validator to ensure the exposure struct is always sane.
	let era = Staking::active_era().unwrap();
	ErasStakers::<TestRuntime>::iter_prefix_values(era.index).for_each(|expo| {
		assert_eq!(
			expo.total as u128,
			expo.own as u128 + expo.others.iter().map(|e| e.value as u128).sum::<u128>(),
			"wrong total exposure.",
		);
	})
}

fn check_nominators() {
	// a check per nominator to ensure their entire stake is correctly distributed. Will only kick-
	// in if the nomination was submitted before the current era.
	let era = Staking::active_era().unwrap();
	<Nominators<TestRuntime>>::iter()
		.filter_map(
			|(nominator, nomination)| {
				if nomination.submitted_in > era.index {
					Some(nominator)
				} else {
					None
				}
			},
		)
		.for_each(|nominator| {
			// must be bonded.
			assert_is_stash(nominator);
			let mut sum = 0;
			Session::validators()
				.iter()
				.map(|v| Staking::eras_stakers(era.index, v))
				.for_each(|e| {
					let individual =
						e.others.iter().filter(|e| e.who == nominator).collect::<Vec<_>>();
					let len = individual.len();
					match len {
						0 => { /* not supporting this validator at all. */ },
						1 => sum += individual[0].value,
						_ => panic!("nominator cannot back a validator more than once."),
					};
				});

			let nominator_stake = Staking::slashable_balance_of(&nominator);
			// a nominator cannot over-spend.
			assert!(
				nominator_stake >= sum,
				"failed: Nominator({}) stake({}) >= sum divided({})",
				nominator,
				nominator_stake,
				sum,
			);

			let diff = nominator_stake - sum;
			assert!(diff < 100);
		});
}

fn assert_is_stash(acc: AccountId) {
	assert!(Staking::bonded(&acc).is_some(), "Not a stash.");
}

// We cannot use this because of visibility of UnlockChunk::balance. We need to check if it is okay
// to omit this check fn assert_ledger_consistent(ctrl: AccountId) {
// 	// ensures ledger.total == ledger.active + sum(ledger.unlocking).
// 	let ledger = Staking::ledger(ctrl).expect("Not a controller.");
// 	let real_total: Balance = ledger.unlocking.iter().fold(ledger.active, |a, c| a + c.value);
// 	assert_eq!(real_total, ledger.total);
// 	assert!(
// 		ledger.active >= Balances::minimum_balance() || ledger.active == 0,
// 		"{}: active ledger amount ({}) must be greater than ED {}",
// 		ctrl,
// 		ledger.active,
// 		Balances::minimum_balance()
// 	);
// }
