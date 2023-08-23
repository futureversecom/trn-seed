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

use crate::{self as pallet_staking_payouts, Config};
use frame_election_provider_support::{generate_solution_type, onchain, SequentialPhragmen};
use frame_support::{parameter_types, storage::StorageValue, PalletId};
use frame_system::EnsureRoot;

use seed_pallet_common::{
	impl_pallet_assets_config, EthereumBridge, EthereumEventRouter as EthereumEventRouterT,
	EthereumEventSubscriber, EventRouterError, EventRouterResult,
};

use seed_pallet_common::{
	impl_pallet_assets_ext_config, impl_pallet_balance_config, impl_pallet_timestamp_config,
};
use seed_primitives::{ethy::EventProofId, AccountId, AssetId, Balance};
use sp_core::{H160, H256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, ConstU128, ConstU64, IdentityLookup},
	DispatchError, Perbill, SaturatedConversion,
};

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
	}
);

impl pallet_session::historical::Config for TestRuntime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<TestRuntime>;
}

sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		pub foo: sp_runtime::testing::UintAuthorityId,
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
	type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Self>;
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

#[derive(Clone, Copy, Default)]
pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut ext: sp_io::TestExternalities = frame_system::GenesisConfig::default()
			.build_storage::<TestRuntime>()
			.unwrap()
			.into();

		ext.execute_with(|| frame_system::Pallet::<TestRuntime>::set_block_number(1));

		ext
	}
}

#[allow(dead_code)]
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<TestRuntime>().unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
