// // Copyright 2022-2023 Futureverse Corporation Limited
// //
// // Licensed under the Apache License, Version 2.0 (the "License");
// // you may not use this file except in compliance with the License.
// // You may obtain a copy of the License at
// //
// //     http://www.apache.org/licenses/LICENSE-2.0
// //
// // Unless required by applicable law or agreed to in writing, software
// // distributed under the License is distributed on an "AS IS" BASIS,
// // WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// // See the License for the specific language governing permissions and
// // limitations under the License.
// // You may obtain a copy of the License at the root of this project source code

// use crate::{self as pallet_staking_payouts, Config, PING};
// use frame_election_provider_support::generate_solution_type;
// use frame_support::{parameter_types, storage::StorageValue, PalletId};
// use frame_system::EnsureRoot;
// use seed_pallet_common::{
// 	EthereumBridge, EthereumEventRouter as EthereumEventRouterT, EthereumEventSubscriber,
// 	EventRouterError, EventRouterResult,
// };

// use seed_primitives::{ethy::EventProofId, AccountId};
// use sp_core::{H160, H256};
// use sp_runtime::{
// 	testing::Header,
// 	traits::{BlakeTwo256, IdentityLookup},
// 	DispatchError, Perbill, SaturatedConversion,
// };

// pub type BlockNumber = u64;
// pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
// pub type Block = frame_system::mocking::MockBlock<TestRuntime>;

// frame_support::construct_runtime!(
// 	pub enum TestRuntime where
// 		Block = Block,
// 		NodeBlock = Block,
// 		UncheckedExtrinsic = UncheckedExtrinsic,
// 	{
// 		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
// 		Echo: pallet_staking_payouts::{Pallet, Call, Storage, Event},
// 	}
// );

// parameter_types! {
// 	pub const BlockHashCount: u64 = 250;
// }
// impl frame_system::Config for TestRuntime {
// 	type BlockWeights = ();
// 	type BlockLength = ();
// 	type BaseCallFilter = frame_support::traits::Everything;
// 	type Origin = Origin;
// 	type Index = u64;
// 	type BlockNumber = BlockNumber;
// 	type Call = Call;
// 	type Hash = H256;
// 	type Hashing = BlakeTwo256;
// 	type AccountId = AccountId;
// 	type Lookup = IdentityLookup<Self::AccountId>;
// 	type Header = Header;
// 	type BlockHashCount = BlockHashCount;
// 	type Event = Event;
// 	type DbWeight = ();
// 	type Version = ();
// 	type PalletInfo = PalletInfo;
// 	type AccountData = ();
// 	type OnNewAccount = ();
// 	type OnKilledAccount = ();
// 	type SystemWeightInfo = ();
// 	type SS58Prefix = ();
// 	type OnSetCode = ();
// 	type MaxConsumers = frame_support::traits::ConstU32<16>;
// }

// parameter_types! {
// 	// phase durations. 1/4 of the last session for each.
// 	// in testing: 1min or half of the session for each
// 	pub SignedPhase: u32 = EPOCH_DURATION_IN_SLOTS / 4;
// 	pub UnsignedPhase: u32 = EPOCH_DURATION_IN_SLOTS / 4;
// 	// signed config
// 	pub const SignedMaxSubmissions: u32 = 16;
// 	pub const SignedMaxRefunds: u32 = 16 / 4;
// 	// 40 DOTs fixed deposit..
// 	pub const SignedDepositBase: Balance = ONE_ROOT * 40;
// 	// 0.01 DOT per KB of solution data.
// 	pub const SignedDepositByte: Balance = ONE_ROOT / 1024;
// 	// Intentionally zero reward to prevent inflation
// 	// `pallet_election_provider_multi_phase::RewardHandler` could be configured to offset any
// rewards 	pub SignedRewardBase: Balance = 0;
// 	pub BetterUnsignedThreshold: Perbill = Perbill::from_rational(5u32, 10_000);
// 	// 4 hour session, 1 hour unsigned phase, 32 offchain executions.
// 	pub OffchainRepeat: BlockNumber = UnsignedPhase::get() / 32;
// 	/// We take the top 22500 nominators as electing voters..
// 	pub const MaxElectingVoters: u32 = 22_500;
// 	/// ... and all of the validators as electable targets. Whilst this is the case, we cannot and
// 	/// shall not increase the size of the validator intentions.
// 	pub const MaxElectableTargets: u16 = u16::MAX;
// }

// generate_solution_type!(
// 	#[compact]
// 	pub struct NposCompactSolution16::<
// 		VoterIndex = u32,
// 		TargetIndex = u16,
// 		Accuracy = sp_runtime::PerU16,
// 		MaxVoters = MaxElectingVoters,
// 	>(16)
// );

// parameter_types! {
// 	// Six sessions in an era (24 hours).
// 	pub const SessionsPerEra: sp_staking::SessionIndex = SessionsPerEra::get();
// 	// 28 eras for unbonding (28 days).
// 	pub const BondingDuration: sp_staking::EraIndex = 28;
// 	pub const SlashDeferDuration: sp_staking::EraIndex = 27;
// 	pub const MaxNominatorRewardedPerValidator: u32 = 256;
// 	pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(17);
// 	// 16
// 	pub const MaxNominations: u32 = <NposCompactSolution16 as
// frame_election_provider_support::NposSolution>::LIMIT as u32; 	// holds XRP from staking slashes
// 	// this could be controlled by pallet-treasury later
// 	pub const SlashPotId: PalletId = PalletId(*b"slashpot");
// 	/// Holds XRP transaction fees for distribution to validators according to stake & undistributed
// reward remainders 	pub const TxFeePotId: PalletId = PalletId(*b"txfeepot");
// }
// type SlashCancelOrigin = EnsureRoot<AccountId>;
// impl pallet_staking::Config for TestRuntime {
// 	type MaxNominations = MaxNominations;
// 	type Currency = DualStakingCurrency;
// 	type CurrencyBalance = Balance;
// 	type CurrencyToVote = frame_support::traits::U128CurrencyToVote;
// 	// Decides the total reward to be distributed each era
// 	// For root network it is the balance of the tx fee pot
// 	type EraPayout = TxFeePot;
// 	type Event = Event;
// 	// After a validator payout is made (to it and all its stakers), this receives the pending
// 	// positive imbalance (total amount newly minted during the payout process) since the XRP
// 	// already exists the issuance should not be modified
// 	//
// 	// pallet-staking validator payouts always _mint_ tokens (with `deposit_creating`) assuming an
// 	// inflationary model instead rewards should be redistributed from fees only
// 	type Reward = TxFeePot;
// 	// Handles any era reward amount indivisible among stakers at end of an era.
// 	// some account should receive the amount to ensure total issuance of XRP is constant (vs.
// 	// burnt)
// 	type RewardRemainder = TxFeePot;
// 	// Upon slashing two situations can happen:
// 	// 1) if there are no reporters, this handler is given the whole slashed imbalance
// 	// 2) any indivisible slash imbalance (not sent to reporter(s)) is sent here
// 	// StakingPot nullifies the imbalance to keep issuance of XRP constant (vs. burnt)
// 	type Slash = SlashImbalanceHandler;
// 	type UnixTime = Timestamp;
// 	type SessionsPerEra = SessionsPerEra;
// 	type BondingDuration = BondingDuration;
// 	type SlashDeferDuration = SlashDeferDuration;
// 	// A super-majority of the council can cancel the slash.
// 	type SlashCancelOrigin = SlashCancelOrigin;
// 	type SessionInterface = Self;
// 	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
// 	type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
// 	type NextNewSession = Session;
// 	type ElectionProvider = ElectionProviderMultiPhase;
// 	type GenesisElectionProvider = onchain::UnboundedExecution<OnChainSeqPhragmen>;
// 	type VoterList = VoterList;
// 	type MaxUnlockingChunks = frame_support::traits::ConstU32<32>;
// 	type BenchmarkingConfig = staking::StakingBenchmarkConfig;
// 	type OnStakerSlash = ();
// 	type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
// }

// parameter_types! {
// 	pub const MockEchoPalletId: PalletId = PalletId(*b"pingpong");
// }
// impl Config for TestRuntime {
// 	type Event = Event;
// 	type PalletId = MockEchoPalletId;
// 	type WeightInfo = ();
// }

// pub(crate) mod test_storage {
// 	//! storage used by tests to store mock EthBlocks and TransactionReceipts
// 	use crate::Config;
// 	use frame_support::decl_storage;
// 	use seed_primitives::ethy::EventProofId;

// 	pub struct Module<T>(sp_std::marker::PhantomData<T>);
// 	decl_storage! {
// 		trait Store for Module<T: Config> as EthBridgeTest {
// 			pub NextEventProofId: EventProofId;
// 		}
// 	}
// }

// #[derive(Clone, Copy, Default)]
// pub struct ExtBuilder;

// impl ExtBuilder {
// 	pub fn build(self) -> sp_io::TestExternalities {
// 		let mut ext: sp_io::TestExternalities = frame_system::GenesisConfig::default()
// 			.build_storage::<TestRuntime>()
// 			.unwrap()
// 			.into();

// 		ext.execute_with(|| frame_system::Pallet::<TestRuntime>::set_block_number(1));

// 		ext
// 	}
// }

// /// Check the system event record contains `event`
// pub(crate) fn has_event(event: crate::Event) -> bool {
// 	System::events()
// 		.into_iter()
// 		.map(|r| r.event)
// 		// .filter_map(|e| if let Event::Nft(inner) = e { Some(inner) } else { None })
// 		.find(|e| *e == Event::Echo(event.clone()))
// 		.is_some()
// }

// #[allow(dead_code)]
// pub fn new_test_ext() -> sp_io::TestExternalities {
// 	let t = frame_system::GenesisConfig::default().build_storage::<TestRuntime>().unwrap();

// 	let mut ext = sp_io::TestExternalities::new(t);
// 	ext.execute_with(|| System::set_block_number(1));
// 	ext
// }
