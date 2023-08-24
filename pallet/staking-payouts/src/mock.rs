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
	parameter_types,
	storage::StorageValue,
	traits::{Currency, GenesisBuild, Hooks, OnInitialize, OneSessionHandler},
	PalletId,
};
use frame_system::EnsureRoot;

use codec::{Decode, Encode};
use pallet_staking::{Bonded, ErasStakers, Nominators, StakerStatus, Validators};
use seed_pallet_common::{
	impl_pallet_assets_config, EthereumBridge, EthereumEventRouter as EthereumEventRouterT,
	EthereumEventSubscriber, EventRouterError, EventRouterResult,
};

use seed_pallet_common::{
	impl_pallet_assets_ext_config, impl_pallet_balance_config, impl_pallet_timestamp_config,
};
use seed_primitives::{ethy::EventProofId, AccountId, AccountId20, AssetId, Balance};
use sp_core::{ecdsa, H160, H256};
use sp_npos_elections::VoteWeight;
use sp_runtime::{
	testing::{Header, UintAuthorityId},
	traits::{BlakeTwo256, ConstU128, ConstU64, IdentityLookup, Zero},
	DispatchError, Perbill, SaturatedConversion,
};

use sp_staking::{EraIndex, SessionIndex};
use sp_std::collections::btree_map::BTreeMap;

pub type BlockNumber = u64;
pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
pub type Block = frame_system::mocking::MockBlock<TestRuntime>;

impl pallet_balances::Config for TestRuntime {
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<10>;
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
		pub other: TestSessionHandler,
	}
}

pub struct TestSessionHandler;
impl pallet_session::SessionHandler<AccountId> for TestSessionHandler {
	const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[];

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
	type ShouldEndSession = pallet_session::PeriodicSessions<(), ()>;
	type NextSessionRotation = pallet_session::PeriodicSessions<(), ()>;
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
	pub const SessionsPerEra: sp_staking::SessionIndex = SessionsPerEra::get();
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
	pub const PayoutPeriodLength: u128 = 90;

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
	pub const ExistentialDeposit: u128 = 10;
}

parameter_types! {
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(
			frame_support::weights::constants::WEIGHT_PER_SECOND * 2
		);
	pub static Period: BlockNumber = 5;
	pub static Offset: BlockNumber = 0;
}

// fn alice() -> AccountId {
// 	AccountId20([1; 20])
// }

// fn bob() -> AccountId {
// 	AccountId20([2; 20])
// }

// fn charlie() -> AccountId {
// 	AccountId20([3; 20])
// }

// fn dave() -> AccountId {
// 	AccountId20([4; 20])
// }

// fn eve() -> AccountId {
// 	AccountId20([5; 20])
// }

// fn ferdie() -> AccountId {
// 	AccountId20([5; 20])
// }

// fn controller_one() -> AccountId {
// 	AccountId20([6; 20])
// }

// fn controller_two() -> AccountId {
// 	AccountId20([6; 20])
// }

// fn controller_three() -> AccountId {
// 	AccountId20([6; 20])
// }

// fn controller_four() -> AccountId {
// 	AccountId20([7; 20])
// }

// fn controller_five() -> AccountId {
// 	AccountId20([8; 20])
// }

// fn stash_one() -> AccountId {
// 	AccountId20([9; 20])
// }

// fn stash_two() -> AccountId {
// 	AccountId20([10; 20])
// }

// fn stash_three() -> AccountId {
// 	AccountId20([11; 20])
// }

// fn stash_four() -> AccountId {
// 	AccountId20([12; 20])
// }

// fn stash_five() -> AccountId {
// 	AccountId20([13; 20])
// }

// fn nominator_one() -> AccountId {
// 	AccountId20([14; 20])
// }
// fn nominator_two() -> AccountId {
// 	AccountId20([15; 20])
// }

// fn aux_account_one() -> AccountId {
// 	AccountId20([14; 20])
// }
// fn aux_account_two() -> AccountId {
// 	AccountId20([16; 20])
// }
// fn aux_account_one() -> AccountId {
// 	AccountId20([14; 20])
// }
// fn aux_account_two() -> AccountId {
// 	AccountId20([16; 20])
// }
// fn aux_account_three() -> AccountId {
// 	AccountId20([14; 20])
// }
// fn aux_account_four() -> AccountId {
// 	AccountId20([16; 20])
// }

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

	// pub fn slash_defer_duration(self, eras: EraIndex) -> Self {
	// 	SlashDeferDuration::get().with(|v| *v.borrow_mut() = eras);
	// 	self
	// }

	pub fn invulnerables(mut self, invulnerables: Vec<AccountId>) -> Self {
		self.invulnerables = invulnerables;
		self
	}
	// pub fn session_per_era(self, length: SessionIndex) -> Self {
	// 	SessionsPerEra::get().with(|v| *v.borrow_mut() = length);
	// 	self
	// }
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

		let alice = AccountId20([1; 20]);
		let bob = AccountId20([2; 20]);
		let charlie = AccountId20([3; 20]);
		let dave = AccountId20([4; 20]);
		let controller_one = AccountId20([5; 20]);
		let controller_two = AccountId20([6; 20]);
		let controller_three = AccountId20([7; 20]);
		let controller_four = AccountId20([8; 20]);
		let controller_five = AccountId20([9; 20]);
		let stash_one = AccountId20([10; 20]);
		let stash_two = AccountId20([11; 20]);
		let stash_three = AccountId20([12; 20]);
		let stash_four = AccountId20([13; 20]);
		let stash_five = AccountId20([14; 20]);
		let nominator_one = AccountId20([15; 20]);
		let nominator_two = AccountId20([16; 20]);
		let aux_account_one = AccountId20([17; 20]);
		let aux_account_two = AccountId20([18; 20]);
		let aux_account_three = AccountId20([19; 20]);
		let aux_account_four = AccountId20([20; 20]);
		let aux_account_five = AccountId20([21; 20]);
		let aux_account_six = AccountId20([22; 20]);
		let account_other = AccountId20([23; 20]);

		let _ = pallet_balances::GenesisConfig::<TestRuntime> {
			balances: vec![
				// (1, 10 * self.balance_factor),
				(alice, 10 * self.balance_factor),
				(bob, 20 * self.balance_factor),
				(charlie, 300 * self.balance_factor),
				(dave, 400 * self.balance_factor),
				// controllers
				(controller_one, self.balance_factor),
				(controller_two, self.balance_factor),
				(controller_three, self.balance_factor),
				(controller_four, self.balance_factor),
				(controller_five, self.balance_factor),
				// stashes
				(stash_one, self.balance_factor * 1000),
				(stash_two, self.balance_factor * 2000),
				(stash_three, self.balance_factor * 2000),
				(stash_four, self.balance_factor * 2000),
				(stash_five, self.balance_factor * 2000),
				// optional nominator
				(nominator_one, self.balance_factor * 2000),
				(nominator_two, self.balance_factor * 2000),
				// aux accounts
				(aux_account_one, self.balance_factor),
				(aux_account_two, self.balance_factor * 2000),
				(aux_account_three, self.balance_factor),
				(aux_account_four, self.balance_factor * 2000),
				(aux_account_five, self.balance_factor),
				(aux_account_six, self.balance_factor * 2000),
				// This allows us to have a total_payout different from 0.
				(account_other, 1_000_000_000_000),
			],
		}
		.assimilate_storage(&mut storage);

		let mut stakers = vec![];
		if self.has_stakers {
			stakers = vec![
				// (stash, ctrl, stake, status)
				// these two will be elected in the default test where we elect 2.
				(
					stash_one,
					controller_one,
					self.balance_factor * 1000,
					StakerStatus::<AccountId>::Validator,
				),
				(
					stash_two,
					controller_two,
					self.balance_factor * 1000,
					StakerStatus::<AccountId>::Validator,
				),
				// a loser validator
				(
					stash_three,
					controller_three,
					self.balance_factor * 500,
					StakerStatus::<AccountId>::Validator,
				),
				// an idle validator
				(
					stash_four,
					controller_four,
					self.balance_factor * 1000,
					StakerStatus::<AccountId>::Idle,
				),
			];
			// optionally add a nominator
			if self.nominate {
				stakers.push((
					nominator_one,
					nominator_two,
					self.balance_factor * 500,
					StakerStatus::<AccountId>::Nominator(vec![stash_one, stash_two]),
				))
			}
			// replace any of the status if needed.
			self.status.into_iter().for_each(|(stash, status)| {
				let (_, _, _, ref mut prev_status) = stakers
					.iter_mut()
					.find(|s| s.0 == stash)
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

						(
							id,
							id,
							SessionKeys { other: Decode::decode(&mut &id_encoded[..]).unwrap() },
						)
					})
					.collect()
			} else {
				// set some dummy validators in genesis.
				(0..self.validator_count as u64)
					.map(|id| {
						let id = AccountId20([id.try_into().unwrap(); 20]);
						let id_encoded = id.encode();
						(
							id,
							id,
							SessionKeys { other: Decode::decode(&mut &id_encoded[..]).unwrap() },
						)
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
