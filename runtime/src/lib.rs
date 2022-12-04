//! Root runtime config
#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use codec::{Decode, Encode};
use fp_rpc::TransactionStatus;
use frame_election_provider_support::{generate_solution_type, onchain, SequentialPhragmen};
use pallet_ethereum::{
	Call::transact, InvalidTransactionWrapper, Transaction as EthereumTransaction,
	TransactionAction,
};
use pallet_evm::{
	Account as EVMAccount, EVMCurrencyAdapter, EnsureAddressNever, EvmConfig, FeeCalculator,
	Runner as RunnerT,
};
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H160, H256, U256};
use sp_runtime::{
	create_runtime_str, generic,
	traits::{
		BlakeTwo256, Block as BlockT, DispatchInfoOf, Dispatchable, IdentityLookup,
		PostDispatchInfoOf, Verify,
	},
	transaction_validity::{
		InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity,
		TransactionValidityError,
	},
	ApplyExtrinsicResult, Percent,
};
pub use sp_runtime::{impl_opaque_keys, traits::NumberFor, Perbill, Permill};
use sp_std::prelude::*;

pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;

#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime,
	dispatch::GetDispatchInfo,
	ensure, parameter_types,
	traits::{
		fungibles::{Inspect, InspectMetadata},
		ConstU32, CurrencyToVote, Everything, IsInVec, KeyOwnerProofSystem, Randomness,
	},
	weights::{
		constants::{ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
		ConstantMultiplier, DispatchClass, IdentityFee, Weight,
	},
	PalletId, StorageValue,
};

use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot,
};
pub use pallet_grandpa::AuthorityId as GrandpaId;
use pallet_grandpa::{fg_primitives, AuthorityList as GrandpaAuthorityList};
pub use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use seed_runtime_constants::weights::BlockExecutionWeight;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

// Export for chain_specs
#[cfg(feature = "std")]
pub use pallet_staking::{Forcing, StakerStatus};
pub mod keys {
	pub use super::{BabeId, EthBridgeId, GrandpaId, ImOnlineId};
}
pub use seed_primitives::{
	ethy::{crypto::AuthorityId as EthBridgeId, ValidatorSet},
	AccountId, Address, AssetId, BabeId, Balance, BlockNumber, CollectionUuid, Hash, Index,
	Signature, TokenId,
};

mod bag_thresholds;

pub mod constants;
use constants::{
	RootAssetId, XrpAssetId, DAYS, EPOCH_DURATION_IN_SLOTS, MILLISECS_PER_BLOCK, MINUTES, ONE_ROOT,
	ONE_XRP, PRIMARY_PROBABILITY, SESSIONS_PER_ERA, SLOT_DURATION,
};

// Implementations of some helper traits passed into runtime modules as associated types.
pub mod impls;
use impls::{
	AddressMapping, EthereumEventRouter, EthereumFindAuthor, EvmCurrencyScaler, HandleTxValidation,
	PercentageOfWeight, SlashImbalanceHandler, StakingSessionTracker,
};

pub mod precompiles;
use precompiles::FutureversePrecompiles;

mod staking;
use staking::OnChainAccuracy;

mod weights;

pub mod runner;
use crate::impls::{FutureverseEnsureAddressSame, OnNewAssetSubscription};
use runner::{FeePreferencesData, FeePreferencesRunner};

use crate::constants::FEE_PROXY;

pub(crate) const LOG_TARGET: &str = "runtime";
#[cfg(test)]
mod tests;

/// Currency implementation mapped to XRP
pub type XrpCurrency = pallet_assets_ext::AssetCurrency<Runtime, XrpAssetId>;
/// Dual currency implementation mapped to ROOT & XRP for staking
pub type DualStakingCurrency =
	pallet_assets_ext::DualStakingCurrency<Runtime, XrpCurrency, Balances>;

/// This runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("root"),
	impl_name: create_runtime_str!("root"),
	authoring_version: 1,
	spec_version: 24,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 0,
};

/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
	sp_consensus_babe::BabeEpochConfiguration {
		c: PRIMARY_PROBABILITY,
		allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
	};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub babe: Babe,
		pub im_online: ImOnline,
		pub grandpa: Grandpa,
		pub ethy: EthBridge,
	}
}

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 4 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = WEIGHT_PER_SECOND / 2;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const Version: RuntimeVersion = VERSION;
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const SS58Prefix: u8 = 193;
}

/// Filters to prevent specific transactions from executing
pub enum CallFilter {}
impl frame_support::traits::Contains<Call> for CallFilter {
	fn contains(call: &Call) -> bool {
		match call {
			// Prevent asset `create` transactions from executing
			Call::Assets(func) => match func {
				pallet_assets::Call::create { .. } => false,
				_ => true,
			},
			_ => true,
		}
	}
}

impl frame_system::Config for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = IdentityLookup<AccountId>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = Header;
	/// The ubiquitous event type.
	type Event = Event;
	/// The ubiquitous origin type.
	type Origin = Origin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Runtime version.
	type Version = Version;
	/// Converts a pallet to an index of this pallet in the runtime.
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = RocksDbWeight;
	type BaseCallFilter = CallFilter;
	type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	pub const TransactionByteFee: Balance = 2_500;
	pub const OperationalFeeMultiplier: u8 = 5;
	pub const WeightToFeeReduction: Permill = Permill::from_parts(125);
}

impl pallet_transaction_payment::Config for Runtime {
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<XrpCurrency, TxFeePot>;
	type Event = Event;
	type WeightToFee = PercentageOfWeight<WeightToFeeReduction>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = ();
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}
impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
}

parameter_types! {
	pub const AssetDeposit: Balance = ONE_XRP;
	pub const AssetAccountDeposit: Balance = 16;
	pub const ApprovalDeposit: Balance = 1;
	pub const AssetsStringLimit: u32 = 50;
	/// Key = 32 bytes, Value = 36 bytes (32+1+1+1+1)
	// https://github.com/paritytech/substrate/blob/069917b/frame/assets/src/lib.rs#L257L271
	pub const MetadataDepositBase: Balance = 1 * 68;
	pub const MetadataDepositPerByte: Balance = 1;
}
pub type AssetsForceOrigin = EnsureRoot<AccountId>;

impl pallet_assets::Config for Runtime {
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
	type WeightInfo = weights::pallet_assets::WeightInfo<Runtime>;
	type AssetAccountDeposit = AssetAccountDeposit;
}

parameter_types! {
	pub const AssetsExtPalletId: PalletId = PalletId(*b"assetext");
	pub const MaxHolds: u32 = 16;
}
impl pallet_assets_ext::Config for Runtime {
	type Event = Event;
	type ParachainId = WorldId;
	type MaxHolds = MaxHolds;
	type NativeAssetId = RootAssetId;
	type OnNewAssetSubscription = OnNewAssetSubscription;
	type PalletId = AssetsExtPalletId;
}

parameter_types! {
	pub const NftPalletId: PalletId = PalletId(*b"nftokens");
	/// How long listings are open for by default
	pub const DefaultListingDuration: BlockNumber = DAYS * 3;
	pub const WorldId: seed_primitives::ParachainId = 100;
}
impl pallet_nft::Config for Runtime {
	type DefaultListingDuration = DefaultListingDuration;
	type Event = Event;
	type MultiCurrency = AssetsExt;
	type OnTransferSubscription = TokenApprovals;
	type OnNewAssetSubscription = OnNewAssetSubscription;
	type PalletId = NftPalletId;
	type ParachainId = WorldId;
	type WeightInfo = ();
}

parameter_types! {
	/// PalletId for Echo pallet
	pub const EchoPalletId: PalletId = PalletId(*b"pingpong");
}
impl pallet_echo::Config for Runtime {
	type Event = Event;
	type EthereumBridge = EthBridge;
	type PalletId = EchoPalletId;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
}
impl pallet_scheduler::Config for Runtime {
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	type Call = Call;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type OriginPrivilegeCmp = frame_support::traits::EqualPrivilegeOnly;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
	type PreimageProvider = ();
	type NoPreimagePostponement = ();
}

impl pallet_utility::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
}

parameter_types! {
	pub const XrpTxChallengePeriod: u32 = 10 * MINUTES;
	pub const XrpClearTxPeriod: u32 = 10 * DAYS;
	/// % threshold to emit event TicketSequenceThresholdReached
	pub const TicketSequenceThreshold: Percent = Percent::from_percent(66_u8);
}

impl pallet_xrpl_bridge::Config for Runtime {
	type Event = Event;
	type EthyAdapter = EthBridge;
	type MultiCurrency = AssetsExt;
	type ApproveOrigin = EnsureRoot<AccountId>;
	type WeightInfo = ();
	type XrpAssetId = XrpAssetId;
	type ChallengePeriod = XrpTxChallengePeriod;
	type ClearTxPeriod = XrpClearTxPeriod;
	type UnixTime = Timestamp;
	type TicketSequenceThreshold = TicketSequenceThreshold;
}

parameter_types! {
	pub const GetExchangeFee: (u32, u32) = (3, 1000);	// 0.3%
	pub const TradingPathLimit: u32 = 3;
	pub const DEXPalletId: PalletId = PalletId(*b"root/dex");
	pub const DEXBurnPalletId: PalletId = PalletId(*b"burn/dex");
	pub const LPTokenName: [u8; 10] = *b"Uniswap V2";
	pub const LPTokenSymbol: [u8; 6] = *b"UNI-V2";
	pub const LPTokenDecimals: u8 = 6; // same as native token decimals
}
impl pallet_dex::Config for Runtime {
	type Event = Event;
	type DEXPalletId = DEXPalletId;
	type DEXBurnPalletId = DEXBurnPalletId;
	type LPTokenName = LPTokenName;
	type LPTokenSymbol = LPTokenSymbol;
	type LPTokenDecimals = LPTokenDecimals;
	type GetExchangeFee = GetExchangeFee;
	type TradingPathLimit = TradingPathLimit;
	type WeightInfo = pallet_dex::weights::PlugWeight<Runtime>;
	type MultiCurrency = AssetsExt;
}

impl pallet_token_approvals::Config for Runtime {
	type GetTokenOwner = Nft;
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}
impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}

parameter_types! {
	pub const UncleGenerations: u32 = 0;
	// More than enough before migration to new architecture
	pub const MaxAuthorities: u32 = 4_096;
}
impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type UncleGenerations = UncleGenerations;
	type FilterUncle = ();
	type EventHandler = (Staking, ImOnline);
}

impl pallet_grandpa::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type KeyOwnerProofSystem = ();
	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		GrandpaId,
	)>>::IdentificationTuple;
	type HandleEquivocation = ();
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
}

impl pallet_session::Config for Runtime {
	type Event = Event;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	// Essentially just Aura, but lets be pedantic.
	type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_session::historical::Config for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

parameter_types! {
	pub const BagThresholds: &'static [u64] = &bag_thresholds::THRESHOLDS;
}
impl pallet_bags_list::Config for Runtime {
	type Event = Event;
	type ScoreProvider = Staking;
	type WeightInfo = weights::pallet_bags_list::WeightInfo<Runtime>;
	type BagThresholds = BagThresholds;
	type Score = sp_npos_elections::VoteWeight;
}

parameter_types! {
	// phase durations. 1/4 of the last session for each.
	// in testing: 1min or half of the session for each
	pub SignedPhase: u32 = EPOCH_DURATION_IN_SLOTS / 4;
	pub UnsignedPhase: u32 = EPOCH_DURATION_IN_SLOTS / 4;
	// signed config
	pub const SignedMaxSubmissions: u32 = 16;
	pub const SignedMaxRefunds: u32 = 16 / 4;
	// 40 DOTs fixed deposit..
	pub const SignedDepositBase: Balance = ONE_ROOT * 40;
	// 0.01 DOT per KB of solution data.
	pub const SignedDepositByte: Balance = ONE_ROOT / 1024;
	// Intentionally zero reward to prevent inflation
	// `pallet_election_provider_multi_phase::RewardHandler` could be configured to offset any rewards
	pub SignedRewardBase: Balance = 0;
	pub BetterUnsignedThreshold: Perbill = Perbill::from_rational(5u32, 10_000);
	// 4 hour session, 1 hour unsigned phase, 32 offchain executions.
	pub OffchainRepeat: BlockNumber = UnsignedPhase::get() / 32;
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
pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Runtime;
	type Solver = SequentialPhragmen<AccountId, OnChainAccuracy>;
	type DataProvider = Staking;
	type WeightInfo = ();
}

parameter_types! {
	/// A limit for off-chain phragmen unsigned solution submission.
	///
	/// We want to keep it as high as possible, but can't risk having it reject,
	/// so we always subtract the base block execution weight.
	pub OffchainSolutionWeightLimit: Weight = RuntimeBlockWeights::get()
		.get(DispatchClass::Normal)
		.max_extrinsic
		.expect("Normal extrinsics have weight limit configured by default; qed")
		.saturating_sub(BlockExecutionWeight::get());

	/// A limit for off-chain phragmen unsigned solution length.
	///
	/// We allow up to 90% of the block's size to be consumed by the solution.
	pub OffchainSolutionLengthLimit: u32 = Perbill::from_rational(90_u32, 100) *
		*RuntimeBlockLength::get()
		.max
		.get(DispatchClass::Normal);
}
impl pallet_election_provider_multi_phase::MinerConfig for Runtime {
	type AccountId = AccountId;
	type MaxLength = OffchainSolutionLengthLimit;
	type MaxWeight = OffchainSolutionWeightLimit;
	type Solution = NposCompactSolution16;
	type MaxVotesPerVoter = <
		<Self as pallet_election_provider_multi_phase::Config>::DataProvider
		as
		frame_election_provider_support::ElectionDataProvider
	>::MaxVotesPerVoter;

	// The unsigned submissions have to respect the weight of the submit_unsigned call, thus their
	// weight estimate function is wired to this call's weight.
	fn solution_weight(v: u32, t: u32, a: u32, d: u32) -> Weight {
		<
			<Self as pallet_election_provider_multi_phase::Config>::WeightInfo
			as
			pallet_election_provider_multi_phase::WeightInfo
		>::submit_unsigned(v, t, a, d)
	}
}

impl pallet_election_provider_multi_phase::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type EstimateCallFee = TransactionPayment;
	type SignedPhase = SignedPhase;
	type UnsignedPhase = UnsignedPhase;
	type SignedMaxSubmissions = SignedMaxSubmissions;
	type SignedMaxRefunds = SignedMaxRefunds;
	type SignedRewardBase = SignedRewardBase;
	type SignedDepositBase = SignedDepositBase;
	type SignedDepositByte = SignedDepositByte;
	type SignedDepositWeight = ();
	type SignedMaxWeight =
		<Self::MinerConfig as pallet_election_provider_multi_phase::MinerConfig>::MaxWeight;
	type MinerConfig = Self;
	type SlashHandler = SlashImbalanceHandler;
	type RewardHandler = (); // nothing to do upon rewards
	type BetterUnsignedThreshold = BetterUnsignedThreshold;
	type BetterSignedThreshold = ();
	type OffchainRepeat = OffchainRepeat;
	type MinerTxPriority = NposSolutionPriority;
	type DataProvider = Staking;
	type Fallback = onchain::UnboundedExecution<OnChainSeqPhragmen>;
	type GovernanceFallback = onchain::UnboundedExecution<OnChainSeqPhragmen>;
	type Solver = SequentialPhragmen<
		AccountId,
		pallet_election_provider_multi_phase::SolutionAccuracyOf<Self>,
		(),
	>;
	type BenchmarkingConfig = staking::ElectionBenchmarkConfig;
	type ForceOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::pallet_election_provider_multi_phase::WeightInfo<Runtime>;
	type MaxElectingVoters = MaxElectingVoters;
	type MaxElectableTargets = MaxElectableTargets;
}

parameter_types! {
	// Six sessions in an era (24 hours).
	pub const SessionsPerEra: sp_staking::SessionIndex = SESSIONS_PER_ERA;
	// 28 eras for unbonding (28 days).
	pub const BondingDuration: sp_staking::EraIndex = 28;
	pub const SlashDeferDuration: sp_staking::EraIndex = 27;
	pub const MaxNominatorRewardedPerValidator: u32 = 256;
	pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(17);
	// 16
	pub const MaxNominations: u32 = <NposCompactSolution16 as frame_election_provider_support::NposSolution>::LIMIT as u32;
	// holds XRP from staking slashes
	// this could be controlled by pallet-treasury later
	pub const SlashPotId: PalletId = PalletId(*b"slashpot");
	/// Holds XRP transaction fees for distribution to validators according to stake & undistributed reward remainders
	pub const TxFeePotId: PalletId = PalletId(*b"txfeepot");
}
type SlashCancelOrigin = EnsureRoot<AccountId>;
impl pallet_staking::Config for Runtime {
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
	type Slash = SlashImbalanceHandler;
	type UnixTime = Timestamp;
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	// A super-majority of the council can cancel the slash.
	type SlashCancelOrigin = SlashCancelOrigin;
	type SessionInterface = Self;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
	type NextNewSession = Session;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider = onchain::UnboundedExecution<OnChainSeqPhragmen>;
	type VoterList = VoterList;
	type MaxUnlockingChunks = frame_support::traits::ConstU32<32>;
	type BenchmarkingConfig = staking::StakingBenchmarkConfig;
	type OnStakerSlash = ();
	type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
}

impl pallet_offences::Config for Runtime {
	type Event = Event;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

parameter_types! {
	pub NposSolutionPriority: TransactionPriority =
		Perbill::from_percent(90) * TransactionPriority::max_value();
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
	pub const MaxPeerDataEncodingSize: u32 = 1_000;
}
impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type Event = Event;
	type ValidatorSet = Historical;
	type NextSessionRotation = Babe;
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = weights::pallet_im_online::WeightInfo<Runtime>;
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
	type MaxPeerDataEncodingSize = MaxPeerDataEncodingSize;
}
impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	Call: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = Call;
}
// end staking stuff

parameter_types! {
	// NOTE: Currently it is not possible to change the epoch duration after the chain has started.
	//       Attempting to do so will brick block production.
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS as u64;
	pub const ExpectedBlockTime: u64 = MILLISECS_PER_BLOCK;
	pub const ReportLongevity: u64 =
		BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
}
impl pallet_babe::Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;
	type DisabledValidators = Session;
	type KeyOwnerProofSystem = Historical;
	type KeyOwnerProof = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		pallet_babe::AuthorityId,
	)>>::Proof;
	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		pallet_babe::AuthorityId,
	)>>::IdentificationTuple;
	type HandleEquivocation =
		pallet_babe::EquivocationHandler<Self::KeyOwnerIdentification, Offences, ReportLongevity>;
	type MaxAuthorities = MaxAuthorities;
	type WeightInfo = ();
}

impl pallet_sudo::Config for Runtime {
	type Event = Event;
	type Call = Call;
}

impl pallet_tx_fee_pot::Config for Runtime {
	type FeeCurrency = XrpCurrency;
	type TxFeePotId = TxFeePotId;
}

parameter_types! {
	/// The bridge pallet address
	pub const BridgePalletId: PalletId = PalletId(*b"ethybrdg");
	/// Bond amount for a challenger
	pub const ChallengeBond: Balance = 100 * ONE_XRP;
	/// % threshold of notarizations required to verify or prove bridge events
	pub const NotarizationThreshold: Percent = Percent::from_percent(66_u8);
	/// Bond amount for a relayer
	pub const RelayerBond: Balance = 100 * ONE_XRP;
	/// Max Xrpl notary (validator) public keys
	pub const MaxXrplKeys: u8 = 8;
	pub const MaxNewSigners: u8 = 20;
}

impl pallet_ethy::Config for Runtime {
	/// Reports the current validator / notary set
	type AuthoritySet = Historical;
	/// The pallet bridge address (destination for incoming messages, source for outgoing)
	type BridgePalletId = BridgePalletId;
	/// The runtime call type.
	type Call = Call;
	/// The bond required to make a challenge
	type ChallengeBond = ChallengeBond;
	// The duration in blocks of one epoch
	type EpochDuration = EpochDuration;
	/// The runtime event type.
	type Event = Event;
	/// Subscribers to completed 'eth_call' jobs
	type EthCallSubscribers = ();
	/// Subscribers to completed event
	type EventRouter = EthereumEventRouter;
	/// Provides Ethereum JSON-RPC client to the pallet (OCW friendly)
	type EthereumRpcClient = pallet_ethy::EthereumRpcClient;
	/// The identifier type for Ethy notaries
	type EthyId = EthBridgeId;
	/// Reports final session status of an era
	type FinalSessionTracker = StakingSessionTracker;
	type MaxNewSigners = MaxNewSigners;
	/// Handles multi-currency fungible asset system
	type MultiCurrency = AssetsExt;
	/// The native asset id used for challenger and relayer bonds
	type NativeAssetId = XrpAssetId;
	/// The threshold of positive notarizations to approve an event claim
	type NotarizationThreshold = NotarizationThreshold;
	/// The bond required to become a relayer
	type RelayerBond = RelayerBond;
	/// The pallet handling scheduled Runtime calls
	type Scheduler = Scheduler;
	/// Timestamp provider
	type UnixTime = Timestamp;
	/// Pallets origin type
	type PalletsOrigin = OriginCaller;
	/// Max Xrpl notary (validator) public keys
	type MaxXrplKeys = MaxXrplKeys;
	/// Xrpl-bridge adapter
	type XrplBridgeAdapter = XRPLBridge;
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

// Start frontier/EVM stuff

/// Current approximation of the gas/s consumption considering
/// EVM execution over compiled WASM (on 4.4Ghz CPU).
/// Given the 500ms Weight, from which 75% only are used for transactions,
/// the total EVM execution gas limit is: GAS_PER_SECOND * 0.500 * 0.75 ~= 15_000_000.
pub const GAS_PER_SECOND: u64 = 15_000_000;

/// Approximate ratio of the amount of Weight per Gas.
/// u64 works for approximations because Weight is a very small unit compared to gas.
pub const WEIGHT_PER_GAS: u64 = WEIGHT_PER_SECOND / GAS_PER_SECOND;

pub struct FutureverseGasWeightMapping;

impl pallet_evm::GasWeightMapping for FutureverseGasWeightMapping {
	fn gas_to_weight(gas: u64) -> Weight {
		gas.saturating_mul(WEIGHT_PER_GAS)
	}
	fn weight_to_gas(weight: Weight) -> u64 {
		u64::try_from(weight.wrapping_div(WEIGHT_PER_GAS)).unwrap_or(u32::MAX as u64)
	}
}

/// This is unused while Futureverse fullness is inconsistent
pub struct BaseFeeThreshold;
impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
	fn lower() -> Permill {
		Permill::zero()
	}
	fn ideal() -> Permill {
		// blocks > 5% full trigger fee increase, < 5% full trigger fee decrease
		Permill::from_parts(50_000)
	}
	fn upper() -> Permill {
		Permill::one()
	}
}

parameter_types! {
	/// Floor network base fee per gas
	/// 0.000015 XRP per gas, 15000 GWEI
	pub const DefaultBaseFeePerGas: u64 = 15_000_000_000_000;
}

impl pallet_base_fee::Config for Runtime {
	type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
	type Event = Event;
	type Threshold = BaseFeeThreshold;
	type DefaultElasticity = ();
}

parameter_types! {
	/// Ethereum ChainId
	/// 3999 (local/dev/default)
	/// TODO: Configured on live chains via one-time setStorage tx at key `:EthereumChainId:`
	pub storage EthereumChainId: u64 = 3_999;
	pub BlockGasLimit: U256
		= U256::from(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT / WEIGHT_PER_GAS);
	pub PrecompilesValue: FutureversePrecompiles<Runtime> = FutureversePrecompiles::<_>::new();
}

/// Modified london config with higher contract create fee
const fn seed_london() -> EvmConfig {
	let mut c = EvmConfig::london();
	c.gas_transaction_create = 2_000_000;
	c
}
pub static SEED_EVM_CONFIG: EvmConfig = seed_london();

impl pallet_evm::Config for Runtime {
	type FeeCalculator = BaseFee;
	type GasWeightMapping = FutureverseGasWeightMapping;
	type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;
	type CallOrigin = FutureverseEnsureAddressSame<AccountId>;
	type WithdrawOrigin = EnsureAddressNever<AccountId>;
	type AddressMapping = AddressMapping<AccountId>;
	type Currency = EvmCurrencyScaler<XrpCurrency>;
	type Event = Event;
	type Runner = FeePreferencesRunner<Self, Self>;
	type PrecompilesType = FutureversePrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type ChainId = EthereumChainId;
	type BlockGasLimit = BlockGasLimit;
	type OnChargeTransaction = EVMCurrencyAdapter<Self::Currency, TxFeePot>;
	type FindAuthor = EthereumFindAuthor<Babe>;
	// internal EVM config
	fn config() -> &'static EvmConfig {
		&SEED_EVM_CONFIG
	}
	type HandleTxValidation = HandleTxValidation<pallet_evm::Error<Runtime>>;
}

impl pallet_ethereum::Config for Runtime {
	type Event = Event;
	type StateRoot = pallet_ethereum::IntermediateStateRoot<Runtime>;
	type HandleTxValidation = HandleTxValidation<InvalidTransactionWrapper>;
}

pub struct TransactionConverter;
impl fp_rpc::ConvertTransaction<UncheckedExtrinsic> for TransactionConverter {
	fn convert_transaction(&self, transaction: pallet_ethereum::Transaction) -> UncheckedExtrinsic {
		UncheckedExtrinsic::new_unsigned(
			pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
		)
	}
}

impl fp_rpc::ConvertTransaction<sp_runtime::OpaqueExtrinsic> for TransactionConverter {
	fn convert_transaction(
		&self,
		transaction: pallet_ethereum::Transaction,
	) -> sp_runtime::OpaqueExtrinsic {
		let extrinsic = UncheckedExtrinsic::new_unsigned(
			pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
		);
		let encoded = extrinsic.encode();
		sp_runtime::OpaqueExtrinsic::decode(&mut &encoded[..])
			.expect("Encoded extrinsic is always valid")
	}
}
// end frontier/EVM stuff

parameter_types! {
	/// The ERC20 peg address
	pub const PegPalletId: PalletId = PalletId(*b"erc20peg");
}

impl pallet_erc20_peg::Config for Runtime {
	/// Handles Ethereum events
	type EthBridge = EthBridge;
	/// Runtime currency system
	type MultiCurrency = AssetsExt;
	/// PalletId/Account for this module
	type PegPalletId = PegPalletId;
	/// The overarching event type.
	type Event = Event;
}

parameter_types! {
	pub const NftPegPalletId: PalletId = PalletId(*b"rn/nftpg");
	pub const DelayLength: BlockNumber = 5;
	pub const MaxAddresses: u32 = 10;
	pub const MaxIdsPerMultipleMint: u32 = 50;
}

impl pallet_nft_peg::Config for Runtime {
	type Event = Event;
	type PalletId = NftPegPalletId;
	type DelayLength = DelayLength;
	type MaxAddresses = MaxAddresses;
	type MaxTokensPerCollection = MaxIdsPerMultipleMint;
	type EthBridge = EthBridge;
}

construct_runtime! {
	pub enum Runtime where
		Block = Block,
		NodeBlock = generic::Block<Header, sp_runtime::OpaqueExtrinsic>,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>} = 0,
		Babe: pallet_babe = 1,
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent}= 2,
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 3,
		Utility: pallet_utility::{Pallet, Call, Event} = 4,

		// Monetary
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 5,
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>, Config<T>} = 6,
		AssetsExt: pallet_assets_ext::{Pallet, Call, Storage, Config<T>, Event<T>} = 7,
		Authorship: pallet_authorship::{Pallet, Call, Storage} = 8,
		Staking: pallet_staking::{Pallet, Call, Storage, Config<T>, Event<T>} = 9,
		Offences: pallet_offences::{Pallet, Storage, Event} = 10,

		// Validators
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 11,
		Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event, ValidateUnsigned} = 12,
		ImOnline: pallet_im_online::{Pallet, Call, Storage, Event<T>, ValidateUnsigned, Config<T>} = 13,

		// World
		Sudo: pallet_sudo::{Pallet, Call, Storage, Config<T>, Event<T>} = 14,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>} = 15,
		Dex: pallet_dex::{Pallet, Call, Storage, Event<T>} = 16,
		Nft: pallet_nft::{Pallet, Call, Storage, Config<T>, Event<T>} = 17,
		XRPLBridge: pallet_xrpl_bridge::{Pallet, Call, Storage, Config<T>, Event<T>} = 18,
		TokenApprovals: pallet_token_approvals::{Pallet, Call, Storage} = 19,
		Historical: pallet_session::historical::{Pallet} = 20,
		Echo: pallet_echo::{Pallet, Call, Storage, Event} = 21,

		// Election pallet. Only works with staking
		ElectionProviderMultiPhase: pallet_election_provider_multi_phase::{Pallet, Call, Storage, Event<T>, ValidateUnsigned} = 22,
		VoterList: pallet_bags_list::{Pallet, Call, Storage, Event<T>} = 23,
		TxFeePot: pallet_tx_fee_pot::{Pallet, Storage} = 24,

		EthBridge: pallet_ethy::{Pallet, Call, Storage, Event<T>, ValidateUnsigned, Config<T>} = 25,

		// EVM
		Ethereum: pallet_ethereum::{Pallet, Call, Storage, Event, Config, Origin} = 26,
		EVM: pallet_evm::{Pallet, Config, Call, Storage, Event<T>} = 27,
		BaseFee: pallet_base_fee::{Pallet, Call, Storage, Config<T>, Event} = 28,
		Erc20Peg: pallet_erc20_peg::{Pallet, Call, Storage, Event<T>} = 29,
		NftPeg: pallet_nft_peg::{Pallet, Call, Storage, Event<T>} = 30,
	}
}

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	fp_self_contained::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = fp_self_contained::CheckedExtrinsic<AccountId, Call, SignedExtra, H160>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(
			extrinsic: <Block as BlockT>::Extrinsic,
		) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: Block, data: sp_inherents::InherentData) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}

		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
	}

	impl sp_consensus_babe::BabeApi<Block> for Runtime {
		fn configuration() -> sp_consensus_babe::BabeGenesisConfiguration {
			// The choice of `c` parameter (where `1 - c` represents the
			// probability of a slot being empty), is done in accordance to the
			// slot duration and expected target block time, for safely
			// resisting network delays of maximum two seconds.
			// <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
			sp_consensus_babe::BabeGenesisConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: BABE_GENESIS_EPOCH_CONFIG.c,
				genesis_authorities: Babe::authorities().to_vec(),
				randomness: Babe::randomness(),
				allowed_slots: BABE_GENESIS_EPOCH_CONFIG.allowed_slots,
			}
		}

		fn current_epoch_start() -> sp_consensus_babe::Slot {
			Babe::current_epoch_start()
		}

		fn current_epoch() -> sp_consensus_babe::Epoch {
			Babe::current_epoch()
		}

		fn next_epoch() -> sp_consensus_babe::Epoch {
			Babe::next_epoch()
		}

		fn generate_key_ownership_proof(
			_slot: sp_consensus_babe::Slot,
			authority_id: sp_consensus_babe::AuthorityId,
		) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
			key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Babe::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}
	}

	impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {
		fn convert_transaction(transaction: EthereumTransaction) -> <Block as BlockT>::Extrinsic {
			UncheckedExtrinsic::new_unsigned(
				pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
			)
		}
	}

	impl pallet_dex_rpc_runtime_api::DexApi<
		Block,
		Runtime,
	> for Runtime {
		fn quote(
			amount_a: u128,
			reserve_a: u128,
			reserve_b: u128,
		) -> Result<u128, sp_runtime::DispatchError> {
			Dex::quote(amount_a.into(), reserve_a, reserve_b).map(|r| r.low_u128())
		}

		fn get_amounts_out(
			amount_in: Balance,
			path: Vec<AssetId>,
		) -> Result<Vec<Balance>, sp_runtime::DispatchError> {
			Dex::get_amounts_out(amount_in, &path)
		}

		fn get_amounts_in(
			amount_out: Balance,
			path: Vec<AssetId>,
		) -> Result<Vec<Balance>, sp_runtime::DispatchError> {
			Dex::get_amounts_in(amount_out, &path)
		}
	}

	impl pallet_nft_rpc_runtime_api::NftApi<
		Block,
		AccountId,
		Runtime,
	> for Runtime {
		fn owned_tokens(collection_id: CollectionUuid, who: AccountId) -> Vec<TokenId> {
			Nft::owned_tokens(collection_id, &who)
		}
		fn token_uri(token_id: TokenId) -> Vec<u8> {
			Nft::token_uri(token_id)
		}
	}

	impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
		fn chain_id() -> u64 {
			<Runtime as pallet_evm::Config>::ChainId::get()
		}

		fn account_basic(address: H160) -> EVMAccount {
			// scaling is handled by the EvmCurrencyScaler inside pallet_evm
			EVM::account_basic(&address).0
		}

		fn gas_price() -> U256 {
			BaseFee::min_gas_price().0
		}

		fn account_code_at(address: H160) -> Vec<u8> {
			EVM::account_codes(address)
		}

		fn author() -> H160 {
			<pallet_evm::Pallet<Runtime>>::find_author()
		}

		fn storage_at(address: H160, index: U256) -> H256 {
			let mut tmp = [0u8; 32];
			index.to_big_endian(&mut tmp);
			EVM::account_storages(address, H256::from_slice(&tmp[..]))
		}

		fn call(
			from: H160,
			to: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			max_fee_per_gas: Option<U256>,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
			access_list: Option<Vec<(H160, Vec<H256>)>>,
		) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {

			let config = if estimate {
				let mut config = <Runtime as pallet_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			<Runtime as pallet_evm::Config>::Runner::call(
				from,
				to,
				data,
				value,
				gas_limit.low_u64(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.unwrap_or_default(),
				false,
				false,
				config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config()),
			).map_err(|err| err.error.into())
		}

		fn create(
			from: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			max_fee_per_gas: Option<U256>,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
			access_list: Option<Vec<(H160, Vec<H256>)>>,
		) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
			let config = if estimate {
				let mut config = <Runtime as pallet_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			<Runtime as pallet_evm::Config>::Runner::create(
				from,
				data,
				value,
				gas_limit.low_u64(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.unwrap_or_default(),
				false,
				false,
				config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config()),
			).map_err(|err| err.error.into())
		}

		fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
			Ethereum::current_transaction_statuses()
		}

		fn current_block() -> Option<pallet_ethereum::Block> {
			Ethereum::current_block()
		}

		fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
			Ethereum::current_receipts()
		}

		fn current_all() -> (
			Option<pallet_ethereum::Block>,
			Option<Vec<pallet_ethereum::Receipt>>,
			Option<Vec<TransactionStatus>>
		) {
			(
				Ethereum::current_block(),
				Ethereum::current_receipts(),
				Ethereum::current_transaction_statuses()
			)
		}

		fn extrinsic_filter(
			xts: Vec<<Block as BlockT>::Extrinsic>,
		) -> Vec<EthereumTransaction> {
			xts.into_iter().filter_map(|xt| match xt.0.function {
				Call::Ethereum(transact { transaction }) => Some(transaction),
				_ => None
			}).collect::<Vec<EthereumTransaction>>()
		}

		fn elasticity() -> Option<Permill> {
			Some(BaseFee::elasticity())
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> fg_primitives::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			_authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
		}
	}

	impl seed_primitives::ethy::EthyApi<Block> for Runtime {
		fn validator_set() -> ValidatorSet<EthBridgeId> {
			EthBridge::validator_set()
		}
		fn xrpl_signers() -> ValidatorSet<EthBridgeId> {
			let door_signers = EthBridge::notary_xrpl_keys();
			ValidatorSet {
				proof_threshold: door_signers.len().saturating_sub(1) as u32, // tolerate 1 missing witness
				validators: door_signers,
				id: EthBridge::notary_set_id(), // the set Id is the same as the overall Ethy set Id
			}
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade() -> (Weight, Weight) {
			log::info!("try-runtime::on_runtime_upgrade.");

			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade().map_err(|err|{
				log::info!("try-runtime::on_runtime_upgrade failed with: {:?}", err);
				err
			}).unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}

		fn execute_block_no_check(block: Block) -> Weight {
			Executive::execute_block_no_check(block)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;

			// Trying to add benchmarks directly to the Session Pallet caused cyclic dependency
			// issues. To get around that, we separated the Session benchmarks into its own crate,
			// which is why we need these two lines below.
			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_election_provider_support_benchmarking::Pallet as EPSBench;
			use frame_system_benchmarking::Pallet as SystemBench;
			use frame_benchmarking::baseline::Pallet as BaselineBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, TrackedStorageKey};

			// Trying to add benchmarks directly to the Session Pallet caused cyclic dependency
			// issues. To get around that, we separated the Session benchmarks into its own crate,
			// which is why we need these two lines below.
			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_election_provider_support_benchmarking::Pallet as EPSBench;
			use frame_system_benchmarking::Pallet as SystemBench;
			use frame_benchmarking::baseline::Pallet as BaselineBench;

			impl pallet_session_benchmarking::Config for Runtime {}
			impl pallet_election_provider_support_benchmarking::Config for Runtime {}
			impl frame_system_benchmarking::Config for Runtime {}
			impl frame_benchmarking::baseline::Config for Runtime {}

			// We took this from the substrate examples as the configurations are pretty close.
			let whitelist: Vec<TrackedStorageKey> = vec![
				// Block Number
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
				// Total Issuance
				hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
				// Execution Phase
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
				// Event Count
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
				// System Events
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
			];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}
}

fn transaction_asset_check(
	source: &H160,
	eth_tx: EthereumTransaction,
	action: TransactionAction,
) -> Result<(), TransactionValidityError> {
	let fee_proxy = TransactionAction::Call(H160::from_low_u64_be(FEE_PROXY));

	if action == fee_proxy {
		let (input, gas_limit, max_fee_per_gas) = match eth_tx {
			EthereumTransaction::Legacy(t) => (t.input, t.gas_limit, None),
			EthereumTransaction::EIP2930(t) => (t.input, t.gas_limit, None),
			EthereumTransaction::EIP1559(t) => (t.input, t.gas_limit, Some(t.max_fee_per_gas)),
		};

		let (payment_asset_id, max_payment, _target, _input) =
			FeePreferencesRunner::<Runtime, Runtime>::decode_input(input)?;
		// ensure user owns max payment amount
		let user_asset_balance = <pallet_assets_ext::Pallet<Runtime> as Inspect<
			<Runtime as frame_system::Config>::AccountId,
		>>::reducible_balance(
			payment_asset_id,
			&<Runtime as frame_system::Config>::AccountId::from(*source),
			false,
		);
		ensure!(
			user_asset_balance >= max_payment,
			TransactionValidityError::Invalid(InvalidTransaction::Payment)
		);
		let FeePreferencesData { account: _, path, total_fee_scaled } =
			runner::get_fee_preferences_data::<Runtime, Runtime>(
				source,
				gas_limit.as_u64(),
				max_fee_per_gas,
				payment_asset_id,
			)?;

		if total_fee_scaled > 0 {
			let amounts = Dex::get_amounts_in(total_fee_scaled, &path)
				.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
			ensure!(
				amounts[0] <= max_payment,
				TransactionValidityError::Invalid(InvalidTransaction::Payment)
			);
			return Ok(())
		}
	}
	Ok(())
}

impl fp_self_contained::SelfContainedCall for Call {
	type SignedInfo = H160;

	fn is_self_contained(&self) -> bool {
		match self {
			Call::Ethereum(call) => call.is_self_contained(),
			_ => false,
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		match self {
			Call::Ethereum(call) => call.check_self_contained(),
			_ => None,
		}
	}

	fn validate_self_contained(
		&self,
		signed_info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<Self>,
		len: usize,
	) -> Option<TransactionValidity> {
		match self {
			Call::Ethereum(ref call) =>
				Some(validate_self_contained_inner(&self, &call, signed_info, dispatch_info, len)),
			_ => None,
		}
	}

	fn pre_dispatch_self_contained(
		&self,
		signed_info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<Self>,
		len: usize,
	) -> Option<Result<(), TransactionValidityError>> {
		match self {
			Call::Ethereum(call) =>
				call.pre_dispatch_self_contained(signed_info, dispatch_info, len),
			_ => None,
		}
	}

	fn apply_self_contained(
		self,
		info: Self::SignedInfo,
	) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
		match self {
			call @ Call::Ethereum(pallet_ethereum::Call::transact { .. }) => Some(
				call.dispatch(Origin::from(pallet_ethereum::RawOrigin::EthereumTransaction(info))),
			),
			_ => None,
		}
	}
}

fn validate_self_contained_inner(
	call: &Call,
	eth_call: &pallet_ethereum::Call<Runtime>,
	signed_info: &<Call as fp_self_contained::SelfContainedCall>::SignedInfo,
	dispatch_info: &DispatchInfoOf<Call>,
	len: usize,
) -> TransactionValidity {
	if let pallet_ethereum::Call::transact { ref transaction } = eth_call {
		// Previously, ethereum transactions were contained in an unsigned
		// extrinsic, we now use a new form of dedicated extrinsic defined by
		// frontier, but to keep the same behavior as before, we must perform
		// the controls that were performed on the unsigned extrinsic.
		use sp_runtime::traits::SignedExtension as _;
		let (input_len, action) = match transaction {
			pallet_ethereum::Transaction::Legacy(t) => (t.input.len(), t.action),
			pallet_ethereum::Transaction::EIP2930(t) => (t.input.len(), t.action),
			pallet_ethereum::Transaction::EIP1559(t) => (t.input.len(), t.action),
		};

		let extra_validation =
			SignedExtra::validate_unsigned(call, &call.get_dispatch_info(), input_len)?;

		// Perform tx submitter asset balance checks required for fee proxying
		match call.clone() {
			Call::Ethereum(pallet_ethereum::Call::transact { transaction }) =>
				transaction_asset_check(signed_info, transaction, action),
			_ => Ok(()),
		}?;

		// Then, do the controls defined by the ethereum pallet.
		let self_contained_validation = eth_call
			.validate_self_contained(signed_info, dispatch_info, len)
			.ok_or(TransactionValidityError::Invalid(InvalidTransaction::BadProof))??;

		Ok(extra_validation.combine_with(self_contained_validation))
	} else {
		Err(TransactionValidityError::Unknown(
			sp_runtime::transaction_validity::UnknownTransaction::CannotLookup,
		))
	}
}

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		// Substrate
		[frame_system, SystemBench::<Runtime>]
		[frame_benchmarking, BaselineBench::<Runtime>]
		[pallet_babe, Babe]
		[pallet_balances, Balances]
		[pallet_timestamp, Timestamp]
		[pallet_scheduler, Scheduler]
		[pallet_utility, Utility]
		[pallet_assets, Assets]
		[pallet_staking, Staking]
		[pallet_grandpa, Grandpa]
		[pallet_im_online, ImOnline]
		[pallet_session, SessionBench::<Runtime>]
		[pallet_bags_list, VoterList]
		[pallet_election_provider_multi_phase, ElectionProviderMultiPhase]
		[pallet_election_provider_support_benchmarking, EPSBench::<Runtime>]
		// Local
		[pallet_nft, Nft]
		// [pallet_xrpl_bridge, XRPLBridge]
		// [pallet_dex, Dex]
	);
}
