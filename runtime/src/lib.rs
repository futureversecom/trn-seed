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

//! Root runtime config
#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "512"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

extern crate alloc;

use alloc::{string::String, vec::Vec};
use codec::{Decode, Encode};
use fp_evm::weight_per_gas;
use fp_rpc::TransactionStatus;
use frame_election_provider_support::{generate_solution_type, onchain, SequentialPhragmen};
use pallet_dex::TradingPairStatus;
use pallet_ethereum::{
	Call::transact, InvalidTransactionWrapper, PostLogContent, Transaction as EthereumTransaction,
	TransactionAction,
};
use pallet_evm::{
	Account as EVMAccount, EnsureAddressNever, FeeCalculator, GasWeightMapping, Runner as RunnerT,
};
use pallet_staking::RewardDestination;
use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
use seed_pallet_common::MaintenanceCheck;
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H160, H256, U256};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		Block as BlockT, Bounded, DispatchInfoOf, Dispatchable, IdentityLookup, NumberFor,
		PostDispatchInfoOf, Verify,
	},
	transaction_validity::{
		InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity,
		TransactionValidityError,
	},
	ApplyExtrinsicResult, FixedPointNumber, Perbill, Percent, Permill, Perquintill,
};
use sp_std::prelude::*;

pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
use pallet_nft::CollectionDetail;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use static_assertions::const_assert;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime,
	dispatch::{DispatchClass, GetDispatchInfo},
	ensure,
	pallet_prelude::Hooks,
	parameter_types,
	traits::{
		fungibles::{metadata::Inspect as InspectMetadata, Inspect},
		tokens::{Fortitude, Preservation},
		AsEnsureOriginWithArg, ConstU128, ConstU32, EitherOfDiverse, Everything, Get, IsInVec,
		KeyOwnerProofSystem, LockIdentifier, Randomness,
	},
	weights::{
		constants::{
			ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_MILLIS,
			WEIGHT_REF_TIME_PER_SECOND,
		},
		ConstantMultiplier, IdentityFee, Weight,
	},
	PalletId, StorageValue,
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot, EnsureSigned,
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
use sp_runtime::traits::UniqueSaturatedInto;

pub mod keys {
	pub use super::{BabeId, EthBridgeId, GrandpaId, ImOnlineId};
}

pub use seed_pallet_common::FeeConfig;
pub use seed_primitives::{
	ethy::{crypto::AuthorityId as EthBridgeId, ValidatorSet},
	AccountId, Address, AssetId, BabeId, Balance, BlakeTwo256Hash, BlockNumber, CollectionUuid,
	Hash, Nonce, SerialNumber, Signature, TokenCount, TokenId,
};

mod bag_thresholds;

pub mod constants;

use constants::{
	deposit, RootAssetId, XrpAssetId, DAYS, EPOCH_DURATION_IN_SLOTS, MILLISECS_PER_BLOCK, MINUTES,
	ONE_ROOT, ONE_XRP, PRIMARY_PROBABILITY, SESSIONS_PER_ERA, SLOT_DURATION, VTX_ASSET_ID,
	XRP_ASSET_ID,
};

// Implementations of some helper traits passed into runtime modules as associated types.
pub mod impls;

use impls::{
	AddressMapping, DoughnutCallValidator, DoughnutFuturepassLookup, EthereumEventRouter,
	EthereumFindAuthor, EvmCurrencyScaler, FutureverseEVMCurrencyAdapter,
	FutureverseEnsureAddressSame, HandleTxValidation, OnNewAssetSubscription,
	SlashImbalanceHandler, StakingSessionTracker,
};
use pallet_fee_proxy::{get_fee_preferences_data, FeePreferencesData, FeePreferencesRunner};

pub mod precompiles;

use precompiles::FutureversePrecompiles;

mod staking;

use staking::OnChainAccuracy;

mod migrations;
mod weights;

use crate::voting::QuadraticVoteWeight;
use precompile_utils::constants::FEE_PROXY_ADDRESS;
use seed_primitives::migration::NoopMigration;

#[cfg(test)]
mod tests;
mod voting;

/// Currency implementation mapped to XRP
pub type XrpCurrency = pallet_assets_ext::AssetCurrency<Runtime, XrpAssetId>;

/// The runtime version information.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("root"),
	impl_name: create_runtime_str!("root"),
	authoring_version: 1,
	spec_version: 84,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 19,
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
/// We allow for 1 seconds of compute with a 4 seconds average block time.
pub const WEIGHT_MILLISECS_PER_BLOCK: u64 = 1000;
pub const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_MILLISECS_PER_BLOCK * WEIGHT_REF_TIME_PER_MILLIS, u64::MAX);

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

impl frame_support::traits::Contains<RuntimeCall> for CallFilter {
	fn contains(call: &RuntimeCall) -> bool {
		// Check whether this call has been paused by the maintenance_mode pallet
		if pallet_maintenance_mode::MaintenanceChecker::<Runtime>::call_paused(call) {
			return false;
		}

		match call {
			// Prevent asset `create` transactions from executing
			RuntimeCall::Assets(pallet_assets::Call::create { .. }) => false,
			// Disable EthBridge `submit_challenge` call
			RuntimeCall::EthBridge(pallet_ethy::Call::submit_challenge { .. }) => false,
			// Disable XRPLBridge `submit_challenge` call
			RuntimeCall::XRPLBridge(pallet_xrpl_bridge::Call::submit_challenge { .. }) => false,
			// Calls to direct rewards to be re-staked are not allowed, as it does not make sense in
			// a dual-currency with pallet-staking context
			RuntimeCall::Staking(pallet_staking::Call::bond { payee, .. }) => {
				if let RewardDestination::Staked = payee {
					return false;
				}
				true
			},
			// Payouts are restricted until a new staking payout system is implemented
			RuntimeCall::Staking(pallet_staking::Call::payout_stakers { .. }) => false,
			// Disable Proxy::add_proxy
			RuntimeCall::Proxy(pallet_proxy::Call::add_proxy { .. }) => false,
			// Prevent new users from submitting their candidacy to the council
			RuntimeCall::Elections(pallet_elections_phragmen::Call::submit_candidacy {
				..
			}) => false,
			_ => true,
		}
	}
}

parameter_types! {
	/// TargetBlockFullness, AdjustmentVariable and MinimumMultiplier values were picked from the
	/// substrate repo. They are the same as the one on Webb, Edgeware, Astar and Phala. Moonbeam
	/// and Polkadot have slightly different values.

	/// The portion of the `NORMAL_DISPATCH_RATIO` that we adjust the fees with. Blocks filled less
	/// than this will decrease the weight and more will increase.
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	/// The adjustment variable of the runtime. Higher values will cause `TargetBlockFullness` to
	/// change the fees more rapidly. This low value causes changes to occur slowly over time.
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
	/// Minimum amount of the multiplier. This value cannot be too low. A test case should ensure
	/// that combined with `AdjustmentVariable`, we can recover from the minimum.
	/// See `multiplier_can_grow_from_zero` in here.
	/// This value is currently only used by pallet-transaction-payment as an assertion that the
	/// next multiplier is always > min value.
	pub MinimumMultiplier: Multiplier = FeeControl::minimum_multiplier();
	/// The maximum amount of the multiplier.
	pub MaximumMultiplier: Multiplier = Bounded::max_value();
}

pub type SlowAdjustingFeeUpdate<R> = TargetedFeeAdjustment<
	R,
	TargetBlockFullness,
	AdjustmentVariable,
	MinimumMultiplier,
	MaximumMultiplier,
>;

impl frame_system::Config for Runtime {
	/// The block type for the runtime.
	type Block = Block;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = IdentityLookup<AccountId>;
	/// The nonce type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256Hash;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
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
	pub const OperationalFeeMultiplier: u8 = 5;
}

pub struct FeeControlWeightToFee;

impl frame_support::weights::WeightToFee for FeeControlWeightToFee {
	type Balance = Balance;

	fn weight_to_fee(weight: &Weight) -> Self::Balance {
		FeeControl::weight_to_fee(weight)
	}
}

pub struct FeeControlLengthToFee;

impl frame_support::weights::WeightToFee for FeeControlLengthToFee {
	type Balance = Balance;

	fn weight_to_fee(weight: &Weight) -> Self::Balance {
		FeeControl::length_to_fee(weight)
	}
}

impl pallet_transaction_payment::Config for Runtime {
	type OnChargeTransaction = FeeProxy;
	type RuntimeEvent = RuntimeEvent;
	type WeightToFee = FeeControlWeightToFee;
	type LengthToFee = FeeControlLengthToFee;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Runtime>;
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
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = ();
	type FreezeIdentifier = ();
	type MaxHolds = ();
	type MaxFreezes = ();
}

parameter_types! {
	// Note, this is unused in favor of a storage value in AssetsExt when calling AssetsExt::create_asset
	pub const AssetDeposit: Balance = ONE_ROOT;
	pub const AssetAccountDeposit: Balance = 16;
	pub const ApprovalDeposit: Balance = 1;
	pub const AssetsStringLimit: u32 = 50;
	/// Key = 32 bytes, Value = 36 bytes (32+1+1+1+1)
	// https://github.com/paritytech/substrate/blob/069917b/frame/assets/src/lib.rs#L257L271
	pub const MetadataDepositBase: Balance = 68;
	pub const MetadataDepositPerByte: Balance = 1;
	pub const RemoveItemsLimit: u32 = 100;
}
pub type AssetsForceOrigin = EnsureRoot<AccountId>;

impl pallet_assets::Config for Runtime {
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
	type WeightInfo = weights::pallet_assets::WeightInfo<Runtime>;
	type AssetAccountDeposit = AssetAccountDeposit;
	type RemoveItemsLimit = RemoveItemsLimit;
	type AssetIdParameter = AssetId;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type CallbackHandle = ();
	pallet_assets::runtime_benchmarks_enabled! {
		type BenchmarkHelper = ();
	}
}

parameter_types! {
	pub const AssetsExtPalletId: PalletId = PalletId(*b"assetext");
	pub const MaxHolds: u32 = 16;
}
impl pallet_assets_ext::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ParachainId = WorldId;
	type MaxHolds = MaxHolds;
	type NativeAssetId = RootAssetId;
	type OnNewAssetSubscription = OnNewAssetSubscription;
	type PalletId = AssetsExtPalletId;
	type WeightInfo = weights::pallet_assets_ext::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MaxDataLength: u32 = 100;
	pub const MaxByteLength: u32 = 500;
	pub const NFINetworkFeePercentage: Permill = Permill::from_perthousand(5);
}

impl pallet_nfi::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = AssetsExt;
	type NFTExt = Nft;
	type SFTExt = Sft;
	type NetworkFeePercentage = NFINetworkFeePercentage;
	type MaxDataLength = MaxDataLength;
	type MaxByteLength = MaxByteLength;
	type WeightInfo = weights::pallet_nfi::WeightInfo<Runtime>;
	type ChainId = EVMChainId;
}

parameter_types! {
	pub const NftPalletId: PalletId = PalletId(*b"nftokens");
	pub const CollectionNameStringLimit: u32 = 50;
	pub const WorldId: seed_primitives::ParachainId = 100;
	pub const MaxTokensPerCollection: u32 = 1_000_000;
	pub const MintLimit: u32 = 1_000;
	pub const TransferLimit: u32 = 1_000;
	pub const NftAdditionalDataLength: u32 = 100;
	pub const MaxPendingIssuances: u32 = 1_000_000;
}
impl pallet_nft::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type MaxTokensPerCollection = MaxTokensPerCollection;
	type MintLimit = MintLimit;
	type TransferLimit = TransferLimit;
	type OnTransferSubscription = TokenApprovals;
	type OnNewAssetSubscription = OnNewAssetSubscription;
	type MultiCurrency = AssetsExt;
	type PalletId = NftPalletId;
	type ParachainId = WorldId;
	type StringLimit = CollectionNameStringLimit;
	type MaxDataLength = NftAdditionalDataLength;
	type WeightInfo = weights::pallet_nft::WeightInfo<Runtime>;
	type Xls20MintRequest = Xls20;
	type NFIRequest = Nfi;
	type MaxPendingIssuances = MaxPendingIssuances;
}

parameter_types! {
	pub const LiquidityPoolsPalletId: PalletId = PalletId(*b"lqdpools");
	pub const LiquidityPoolsUnsignedInterval: BlockNumber = MINUTES / 2;
	/// How many users to rollover at a block time
	pub const RolloverBatchSize: u32 = 99;
	pub const InterestRateBasePoint: u32 = 1_000_000;
	pub const MaxPoolsPerOnIdle: u32 = 50;
}
impl pallet_liquidity_pools::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = LiquidityPoolsPalletId;
	type UnsignedInterval = LiquidityPoolsUnsignedInterval;
	type PoolId = u32;
	type MaxStringLength = MaxStringLength;
	type RolloverBatchSize = RolloverBatchSize;
	type InterestRateBasePoint = InterestRateBasePoint;
	type MultiCurrency = AssetsExt;
	type WeightInfo = weights::pallet_liquidity_pools::WeightInfo<Runtime>;
	type MaxPoolsPerOnIdle = MaxPoolsPerOnIdle;
}

parameter_types! {
	pub const MarketplacePalletId: PalletId = PalletId(*b"marketpl");
	/// How long listings are open for by default
	pub const DefaultListingDuration: BlockNumber = DAYS * 3;
	/// How long offers are valid for by default
	pub const DefaultOfferDuration: BlockNumber = DAYS * 30;
	pub const MaxTokensPerListing: u32 = 1000;
	pub const MaxListingsPerMultiBuy: u32 = 50;
	pub const MaxOffers: u32 = 100;
	pub const MaxRemovableOffers: u32 = 10;
	pub const MarketplaceNetworkFeePercentage: Permill = Permill::from_perthousand(5);
	pub const DefaultTxFeePotId: Option<PalletId> = Some(TxFeePotId::get());
}
impl pallet_marketplace::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type DefaultListingDuration = DefaultListingDuration;
	type DefaultOfferDuration = DefaultOfferDuration;
	type RuntimeEvent = RuntimeEvent;
	type DefaultFeeTo = DefaultFeeTo;
	type MultiCurrency = AssetsExt;
	type NFTExt = Nft;
	type SFTExt = Sft;
	type PalletId = MarketplacePalletId;
	type NetworkFeePercentage = MarketplaceNetworkFeePercentage;
	type WeightInfo = weights::pallet_marketplace::WeightInfo<Runtime>;
	type MaxTokensPerListing = MaxTokensPerListing;
	type MaxListingsPerMultiBuy = MaxListingsPerMultiBuy;
	type MaxOffers = MaxOffers;
	type MaxRemovableOffers = MaxRemovableOffers;
}

parameter_types! {
	pub const SftPalletId: PalletId = PalletId(*b"sftokens");
	pub const MaxTokensPerSftCollection: u32 = 1_000_000;
	pub const MaxOwnersPerSftCollection: u32 = 1_000_000;
	pub const MaxSftPendingIssuances: u32 = 1_000_000;
	pub const SftAdditionalDataLength: u32 = 100;
	pub const MaxSerialsPerMint: u32 = 1000; // Higher values can be storage heavy
}
impl pallet_sft::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = AssetsExt;
	type NFTExt = Nft;
	type OnTransferSubscription = TokenApprovals;
	type OnNewAssetSubscription = OnNewAssetSubscription;
	type PalletId = SftPalletId;
	type ParachainId = WorldId;
	type StringLimit = CollectionNameStringLimit;
	type MaxDataLength = SftAdditionalDataLength;
	type WeightInfo = weights::pallet_sft::WeightInfo<Runtime>;
	type MaxTokensPerSftCollection = MaxTokensPerSftCollection;
	type MaxSerialsPerMint = MaxSerialsPerMint;
	type MaxOwnersPerSftToken = MaxOwnersPerSftCollection;
	type NFIRequest = Nfi;
	type MaxSftPendingIssuances = MaxSftPendingIssuances;
}

parameter_types! {
	pub const MaxTokensPerXls20Mint: u32 = 1000;
	pub const Xls20PalletId: PalletId = PalletId(*b"xls20nft");
}
impl pallet_xls20::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaxTokensPerXls20Mint = MaxTokensPerXls20Mint;
	type MultiCurrency = AssetsExt;
	type NFTExt = Nft;
	type NFTCollectionInfo = Nft;
	type WeightInfo = weights::pallet_xls20::WeightInfo<Runtime>;
	type Xls20PaymentAsset = XrpAssetId;
	type PalletId = Xls20PalletId;
	type NFTMinter = Nft;
}

parameter_types! {
	/// PalletId for Echo pallet
	pub const EchoPalletId: PalletId = PalletId(*b"pingpong");
}
impl pallet_echo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type EthereumBridge = EthBridge;
	type PalletId = EchoPalletId;
	type WeightInfo = weights::pallet_echo::WeightInfo<Runtime>;
}

impl pallet_fee_proxy::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type PalletsOrigin = OriginCaller;
	type FeeAssetId = XrpAssetId;
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<XrpCurrency, TxFeePot>;
	type ErcIdConversion = Self;
	type EVMBaseFeeProvider = FeeControl;
	type MaintenanceChecker = pallet_maintenance_mode::MaintenanceChecker<Runtime>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
	pub const NoPreimagePostponement: Option<u32> = Some(10);
}
impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	// If this is made public, we would need to add a check for maintenance mode as well.
	// If maintenance mode is enabled, people can schedule a call and bypass maintenance mode
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type OriginPrivilegeCmp = frame_support::traits::EqualPrivilegeOnly;
	type WeightInfo = weights::pallet_scheduler::WeightInfo<Runtime>;
	type Preimages = Preimage;
}

parameter_types! {
	pub const PreimageMaxSize: u32 = 4096 * 1024;
	pub const PreimageBaseDeposit: Balance = deposit(2, 64);
	pub const PreimageByteDeposit: Balance = deposit(0, 1);
}

impl pallet_preimage::Config for Runtime {
	type WeightInfo = weights::pallet_preimage::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type BaseDeposit = PreimageBaseDeposit;
	type ByteDeposit = PreimageByteDeposit;
}

parameter_types! {
	pub const MaxResolvers: u8 = 10;
	pub const MaxTags: u8 = 10;
	pub const MaxEntries: u8 = 100;
	pub const MaxServiceEndpoints: u8 = 10;
	pub const SyloStringLimit: u16 = 500;
}

impl pallet_sylo_data_verification::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type SyloDataPermissionsProvider = pallet_sylo_data_permissions::Pallet<Runtime>;
	type ApproveOrigin = EnsureRoot<AccountId>;
	type MaxResolvers = MaxResolvers;
	type MaxTags = MaxTags;
	type MaxEntries = MaxEntries;
	type MaxServiceEndpoints = MaxServiceEndpoints;
	type StringLimit = SyloStringLimit;
	type WeightInfo = weights::pallet_sylo_data_verification::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MaxPermissions: u32 = 100;
	pub const MaxPermissionRecords: u32 = 100;
	pub const MaxExpiringPermissions: u32 = 10;
	pub const PermissionRemovalDelay: u32 = 648000; // 30 days
}

impl pallet_sylo_data_permissions::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type SyloDataVerificationProvider = pallet_sylo_data_verification::Pallet<Runtime>;
	type MaxPermissions = MaxPermissions;
	type MaxResolvers = MaxResolvers;
	type MaxTags = MaxTags;
	type MaxEntries = MaxEntries;
	type MaxServiceEndpoints = MaxServiceEndpoints;
	type MaxPermissionRecords = MaxPermissionRecords;
	type MaxExpiringPermissions = MaxExpiringPermissions;
	type PermissionRemovalDelay = PermissionRemovalDelay;
	type StringLimit = SyloStringLimit;
	type WeightInfo = weights::pallet_sylo_data_permissions::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MaxCallIds: u32 = 200;
	pub const XrplMaxMessageLength: u32 = 2048;
	pub const XrplMaxSignatureLength: u32 = 2048;
}

impl pallet_sylo_action_permissions::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type FuturepassLookup = impls::FuturepassLookup;
	type BlacklistedCallProvider = impls::SyloActionsCallValidator;
	type MaxCallIds = MaxCallIds;
	type StringLimit = SyloStringLimit;
	type XrplMaxMessageLength = XrplMaxMessageLength;
	type XrplMaxSignatureLength = XrplMaxSignatureLength;
	type WeightInfo = weights::pallet_sylo_action_permissions::WeightInfo<Runtime>;
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	// The maximum amount of signatories allowed in the multisig
	pub const MaxSignatories: u32 = 100;
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = weights::pallet_multisig::WeightInfo<Runtime>;
}

parameter_types! {
	pub const XrpTxChallengePeriod: u32 = 10 * MINUTES;
	/// % threshold to emit event TicketSequenceThresholdReached
	pub const TicketSequenceThreshold: Percent = Percent::from_percent(66_u8);
	/// NOTE - XRPTransactionLimitPerLedger should be more than or equal to XRPTransactionLimit
	pub const XRPTransactionLimit: u32 = 1_000_000;
	pub const XRPTransactionLimitPerLedger: u32 = 1_000_000;
	/// NOTE - This value can't be set too high. 5000 is roughly 25% of the max block weight
	pub const MaxPrunedTransactionsPerBlock: u32 = 5000;
	pub const MaxDelayedPaymentsPerBlock: u32 = 1000;
	pub const DelayedPaymentBlockLimit: BlockNumber = 1000;
	/// The xrpl peg address
	pub const XrplPalletId: PalletId = PalletId(*b"xrpl-peg");
}

impl pallet_xrpl_bridge::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type EthyAdapter = EthBridge;
	type MultiCurrency = AssetsExt;
	type ApproveOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::pallet_xrpl_bridge::WeightInfo<Runtime>;
	type XrpAssetId = XrpAssetId;
	type NativeAssetId = RootAssetId;
	type PalletId = XrplPalletId;
	type ChallengePeriod = XrpTxChallengePeriod;
	type MaxPrunedTransactionsPerBlock = MaxPrunedTransactionsPerBlock;
	type MaxDelayedPaymentsPerBlock = MaxDelayedPaymentsPerBlock;
	type DelayedPaymentBlockLimit = DelayedPaymentBlockLimit;
	type UnixTime = Timestamp;
	type TicketSequenceThreshold = TicketSequenceThreshold;
	type XRPTransactionLimit = XRPTransactionLimit;
	type XRPLTransactionLimitPerLedger = XRPTransactionLimitPerLedger;
	type NFTExt = Nft;
	type Xls20Ext = Xls20;
}

parameter_types! {
	pub const MaxMessageLength: u32 = 2048;
	pub const MaxSignatureLength: u32 = 80;
}

impl pallet_xrpl::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type CallValidator = impls::MaintenanceModeCallValidator;
	type FuturepassLookup = impls::FuturepassLookup;
	type PalletsOrigin = OriginCaller;
	type MaxMessageLength = MaxMessageLength;
	type MaxSignatureLength = MaxSignatureLength;
	type WeightInfo = weights::pallet_xrpl::WeightInfo<Runtime>;
}

parameter_types! {
	pub const GetExchangeFee: (u32, u32) = (3, 1000);	// 0.3%
	pub const TradingPathLimit: u32 = 3;
	pub const DEXBurnPalletId: PalletId = PalletId(*b"burn/dex");
	pub const LPTokenDecimals: u8 = 18;
	pub const DefaultFeeTo: Option<PalletId> = Some(TxFeePotId::get());
}
impl pallet_dex::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type DEXBurnPalletId = DEXBurnPalletId;
	type LPTokenDecimals = LPTokenDecimals;
	type GetExchangeFee = GetExchangeFee;
	type TradingPathLimit = TradingPathLimit;
	type DefaultFeeTo = DefaultFeeTo;
	type WeightInfo = weights::pallet_dex::WeightInfo<Runtime>;
	type MultiCurrency = AssetsExt;
}

impl pallet_token_approvals::Config for Runtime {
	type NFTExt = Nft;
	type WeightInfo = weights::pallet_token_approvals::WeightInfo<Runtime>;
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

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type EventHandler = (Staking, ImOnline);
}

parameter_types! {
	// More than enough before migration to new architecture
	pub const MaxAuthorities: u32 = 4_096;
	// Equivocation constants.
	pub const ReportLongevity: u64 = BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
	pub const MaxSetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}
impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
	type EquivocationReportSystem =
		pallet_grandpa::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	// Essentially just Aura, but lets be pedantic.
	type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = weights::pallet_session::WeightInfo<Runtime>;
}

impl pallet_session::historical::Config for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

parameter_types! {
	pub const BagThresholds: &'static [u64] = &bag_thresholds::THRESHOLDS;
}

impl pallet_bags_list::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
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
	// 40 ROOT fixed deposit..
	pub const SignedDepositBase: Balance = ONE_ROOT * 40;
	// 0.01 ROOT per KB of solution data.
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
	/// Setup election pallet to support maximum winners upto 1200. This will mean Staking Pallet
	/// cannot have active validators higher than this count.
	pub const MaxActiveValidators: u32 = 1200;
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
	type WeightInfo = weights::frame_election_provider_support::WeightInfo<Runtime>;
	type MaxWinners = MaxActiveValidators;
	type VotersBound = MaxElectingVoters;
	type TargetsBound = MaxElectableTargets;
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

	type MaxWinners = MaxActiveValidators;
}

impl pallet_election_provider_multi_phase::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
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
	type RewardHandler = ();
	// nothing to do upon rewards
	type BetterUnsignedThreshold = BetterUnsignedThreshold;
	type BetterSignedThreshold = ();
	type OffchainRepeat = OffchainRepeat;
	type MinerTxPriority = NposSolutionPriority;
	type DataProvider = Staking;
	type Fallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type GovernanceFallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
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
	type MaxWinners = MaxActiveValidators;
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
	pub const MaxUnlockingChunks: u32 = 32;
	pub const HistoryDepth: u32 = 84;
}

impl pallet_staking::Config for Runtime {
	type MaxNominations = MaxNominations;
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;
	// Decides the total reward to be distributed each era
	// For root network it is the balance of the tx fee pot
	type EraPayout = TxFeePot;
	type RuntimeEvent = RuntimeEvent;
	// In our current implementation we have filtered the payout_stakers call so this Reward will
	// never be triggered. We have decided to keep the TxFeePot in the case this is overlooked
	// to prevent unwanted changes in Root token issuance
	type Reward = TxFeePot;
	// Handles any era reward amount indivisible among stakers at end of an era.
	// some account should receive the amount to ensure total issuance of XRP is constant (vs.
	// burnt)
	type RewardRemainder = TxFeePot;
	// Upon slashing two situations can happen:
	// 1) if there are no reporters, this handler is given the whole slashed imbalance
	// 2) any indivisible slash imbalance (not sent to reporter(s)) is sent here
	// StakingPot nullifies the imbalance to keep issuance of ROOT constant (vs. burnt)
	type Slash = SlashImbalanceHandler;
	type UnixTime = Timestamp;
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	type AdminOrigin = EnsureRoot<Self::AccountId>; // Keeping this as root for now.
	type SessionInterface = Self;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
	type NextNewSession = Session;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type VoterList = VoterList;
	type TargetList = pallet_staking::UseValidatorsMap<Runtime>;
	type MaxUnlockingChunks = MaxUnlockingChunks;
	type BenchmarkingConfig = staking::StakingBenchmarkConfig;
	type WeightInfo = weights::pallet_staking::WeightInfo<Runtime>;
	type HistoryDepth = HistoryDepth;
	type EventListeners = ();
}

impl pallet_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

parameter_types! {
	pub NposSolutionPriority: TransactionPriority =
		Perbill::from_percent(90) * TransactionPriority::MAX;
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::MAX;
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
	pub const MaxPeerDataEncodingSize: u32 = 1_000;
}

impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type RuntimeEvent = RuntimeEvent;
	type ValidatorSet = Historical;
	type NextSessionRotation = Babe;
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = weights::pallet_im_online::WeightInfo<Runtime>;
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = RuntimeCall;
}
// end staking stuff

parameter_types! {
	// NOTE: Currently it is not possible to change the epoch duration after the chain has started.
	//       Attempting to do so will brick block production.
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS as u64;
	pub const ExpectedBlockTime: u64 = MILLISECS_PER_BLOCK;
}
impl pallet_babe::Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;
	type DisabledValidators = Session;
	type KeyOwnerProof =
		<Historical as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;
	type MaxAuthorities = MaxAuthorities;
	type WeightInfo = ();
	type EquivocationReportSystem =
		pallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = weights::pallet_sudo::WeightInfo<Runtime>;
}

impl pallet_tx_fee_pot::Config for Runtime {
	type FeeCurrency = XrpCurrency;
	type StakeCurrency = Balances;
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
	/// 75 blocks is 5 minutes before the end of the era
	pub const AuthorityChangeDelay: BlockNumber = 75_u32;

	pub const MaxEthData: u32 = 3200;
	pub const MaxChallenges: u32 = 100;
	pub const MaxMessagesPerBlock: u32 = 1000;
	pub const MaxCallRequests: u32 = 1000;
	pub const MaxProcessedMessageIds: u32 = 1000;
}

impl pallet_ethy::Config for Runtime {
	/// Length of time the bridge will be paused while the authority set changes
	type AuthorityChangeDelay = AuthorityChangeDelay;
	/// Reports the current validator / notary set
	type AuthoritySet = Historical;
	/// The pallet bridge address (destination for incoming messages, source for outgoing)
	type BridgePalletId = BridgePalletId;
	/// The runtime call type.
	type RuntimeCall = RuntimeCall;
	/// The bond required to make a challenge
	type ChallengeBond = ChallengeBond;
	// The duration in blocks of one epoch
	type EpochDuration = EpochDuration;
	/// Subscribers to completed 'eth_call' jobs
	type EthCallSubscribers = ();
	/// Provides Ethereum JSON-RPC client to the pallet (OCW friendly)
	type EthereumRpcClient = pallet_ethy::EthereumRpcClient;
	/// The runtime event type.
	type RuntimeEvent = RuntimeEvent;
	/// Subscribers to completed event
	type EventRouter = EthereumEventRouter;
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
	/// Pallets origin type
	type PalletsOrigin = OriginCaller;
	type MaxProcessedMessageIds = MaxProcessedMessageIds;
	/// Timestamp provider
	type UnixTime = Timestamp;
	/// Max Xrpl notary (validator) public keys
	type MaxXrplKeys = MaxXrplKeys;
	/// Xrpl-bridge adapter
	type XrplBridgeAdapter = XRPLBridge;
	type MaxAuthorities = MaxAuthorities;
	type MaxEthData = MaxEthData;
	type MaxChallenges = MaxChallenges;
	type MaxMessagesPerBlock = MaxMessagesPerBlock;
	type MaxCallRequests = MaxCallRequests;
	type WeightInfo = weights::pallet_ethy::WeightInfo<Runtime>;
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

parameter_types! {
	pub const DefaultChainId: u64 = 7672;
}
impl pallet_evm_chain_id::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ApproveOrigin = EnsureRoot<AccountId>;
	type DefaultChainId = DefaultChainId;
	type WeightInfo = weights::pallet_evm_chain_id::WeightInfo<Runtime>;
}

// Start frontier/EVM stuff
// Number suitable for TRN, based on the gas benchmarks on current standard spec.
const BLOCK_GAS_LIMIT: u64 = 15_000_000;
// Default value from Frontier
const MAX_POV_SIZE: u64 = 5 * 1024 * 1024;

parameter_types! {
	pub BlockGasLimit: U256 = U256::from(BLOCK_GAS_LIMIT);
	// https://github.com/polkadot-evm/frontier/pull/1039#issuecomment-1600291912
	pub const GasLimitPovSizeRatio: u64 = BLOCK_GAS_LIMIT.saturating_div(MAX_POV_SIZE);
	pub PrecompilesValue: FutureversePrecompiles<Runtime> = FutureversePrecompiles::<_>::new();
	pub WeightPerGas: Weight = Weight::from_parts(weight_per_gas(BLOCK_GAS_LIMIT, NORMAL_DISPATCH_RATIO, WEIGHT_MILLISECS_PER_BLOCK), 0);
}

impl pallet_evm::Config for Runtime {
	type FeeCalculator = FeeControl;
	type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
	type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;
	type CallOrigin = FutureverseEnsureAddressSame<AccountId>;
	type WithdrawOrigin = EnsureAddressNever<AccountId>;
	type AddressMapping = AddressMapping<AccountId>;
	type Currency = EvmCurrencyScaler<XrpCurrency>;
	type RuntimeEvent = RuntimeEvent;
	type Runner = FeePreferencesRunner<Self, Self, Futurepass>;
	type PrecompilesType = FutureversePrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type ChainId = EVMChainId;
	type BlockGasLimit = BlockGasLimit;
	type OnChargeTransaction = FutureverseEVMCurrencyAdapter<Self::Currency, TxFeePot>;
	type FindAuthor = EthereumFindAuthor<Babe>;
	type HandleTxValidation = HandleTxValidation<pallet_evm::Error<Runtime>>;
	type WeightPerGas = WeightPerGas;
	type OnCreate = ();
	type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
	type Timestamp = Timestamp;
	type WeightInfo = pallet_evm::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
	pub const ExtraDataLength: u32 = 30;
}

impl pallet_ethereum::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type StateRoot = pallet_ethereum::IntermediateStateRoot<Runtime>;
	type PostLogContent = PostBlockAndTxnHashes;
	type ExtraDataLength = ExtraDataLength;
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
	/// Limit that determines max delays stored simultaneously in a single block
	/// NOTE: This value is estimated from the weight information of process deposit (with padding)
	pub const MaxDelaysPerBlock: u32 =  500;
	/// Needs to be large enough to handle the maximum number of blocks that can be ready at once
	pub const MaxReadyBlocks: u32 = 100_000;
}

impl pallet_erc20_peg::Config for Runtime {
	/// Handles Ethereum events
	type EthBridge = EthBridge;
	/// Runtime currency system
	type MultiCurrency = AssetsExt;
	/// PalletId/Account for this module
	type PegPalletId = PegPalletId;
	/// The overarching event type.
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_erc20_peg::WeightInfo<Runtime>;
	type NativeAssetId = RootAssetId;
	type StringLimit = AssetsStringLimit;
	type MaxDelaysPerBlock = MaxDelaysPerBlock;
	type MaxReadyBlocks = MaxReadyBlocks;
}

parameter_types! {
	pub const NftPegPalletId: PalletId = PalletId(*b"rn/nftpg");
	pub const DelayLength: BlockNumber = 5;
	pub const MaxAddresses: u32 = 10;
	pub const MaxCollectionsPerWithdraw: u32 = 10;
	// These values must be the same so blocked tokens can be safely reclaimed
	// Ref: https://github.com/futureversecom/trn-seed/pull/674
	pub const MaxIdsPerMultipleMint: u32 = 50;
	pub const MaxSerialsPerWithdraw: u32 = 50;
}

impl pallet_nft_peg::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = NftPegPalletId;
	type DelayLength = DelayLength;
	type MaxAddresses = MaxAddresses;
	type MaxTokensPerMint = MaxIdsPerMultipleMint;
	type EthBridge = EthBridge;
	type NftPegWeightInfo = weights::pallet_nft_peg::WeightInfo<Runtime>;
	type MaxCollectionsPerWithdraw = MaxCollectionsPerWithdraw;
	type MaxSerialsPerWithdraw = MaxSerialsPerWithdraw;
	type NFTMinter = Nft;
}

impl pallet_fee_control::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_fee_control::WeightInfo<Runtime>;
	type FeeConfig = ();
}

impl pallet_doughnut::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type CallValidator = DoughnutCallValidator;
	type FuturepassLookup = DoughnutFuturepassLookup;
	type WeightInfo = weights::pallet_doughnut::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ConfigDepositBase: u64 = 10;
	pub const FriendDepositFactor: u64 = 1;
	pub const MaxFriends: u32 = 3;
	pub const RecoveryDeposit: u64 = 10;
}

impl pallet_recovery::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ConfigDepositBase = ConfigDepositBase;
	type FriendDepositFactor = FriendDepositFactor;
	type MaxFriends = MaxFriends;
	type RecoveryDeposit = RecoveryDeposit;
	type WeightInfo = pallet_recovery::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	// One storage item; key size 32, value size 8
	pub ProxyDepositBase: Balance = deposit(1, 8);
	// Additional storage item size of 21 bytes (20 bytes AccountId + 1 byte sizeof(ProxyType)).
	pub ProxyDepositFactor: Balance = deposit(0, 21);
	pub AnnouncementDepositBase: Balance = deposit(1, 8);
	// Additional storage item size of 56 bytes:
	// - 20 bytes AccountId
	// - 32 bytes Hasher (Blake2256)
	// - 4 bytes BlockNumber (u32)
	pub AnnouncementDepositFactor: Balance = deposit(0, 56);
	pub const MaxProxies: u32 = 32;
	pub const MaxPending: u32 = 32;
}

impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;

	type ProxyType = impls::ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = MaxProxies;
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256Hash;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
	type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
}

impl pallet_futurepass::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Proxy = impls::ProxyPalletProvider;
	type RuntimeCall = RuntimeCall;
	type BlacklistedCallValidator = impls::FuturepassCallValidator;
	type ProxyType = impls::ProxyType;
	type WeightInfo = weights::pallet_futurepass::WeightInfo<Self>;

	#[cfg(feature = "runtime-benchmarks")]
	type MultiCurrency = AssetsExt;
}

parameter_types! {
	pub const VtxHeldPotId: PalletId = PalletId(*b"vtx/hpot");
	pub const VtxVortexPotId: PalletId = PalletId(*b"vtx/vpot");
	pub const VtxRootPotId: PalletId = PalletId(*b"vtx/rpot");
	pub const FeePotId: PalletId = TxFeePotId::get();
	pub const UnsignedInterval: BlockNumber =  MINUTES / 2;
	pub const PayoutBatchSize: u32 =  99;
	pub const VortexAssetId: AssetId = VTX_ASSET_ID;
	pub const GasAssetId: AssetId = XRP_ASSET_ID;
	pub const MaxAssetPrices: u32 = 500;
	pub const MaxRewards: u32 = 500;
	// Maximum number of partners for attribution. The value was decided by the team after looking at the future projections.
	pub const MaxAttributionPartners: u32 = 200;
	pub const MaxStringLength: u32 = 1_000;
}

impl pallet_vortex_distribution::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_vortex_distribution::WeightInfo<Runtime>;
	type NativeAssetId = RootAssetId;
	type VtxAssetId = VortexAssetId;
	type VtxHeldPotId = VtxHeldPotId;
	type VtxDistPotId = VtxVortexPotId;
	type RootPotId = VtxRootPotId;
	type TxFeePotId = FeePotId;
	type UnsignedInterval = UnsignedInterval;
	type PayoutBatchSize = PayoutBatchSize;
	type VtxDistIdentifier = u32;
	type MultiCurrency = AssetsExt;
	type MaxAssetPrices = MaxAssetPrices;
	type MaxRewards = MaxRewards;
	type MaxStringLength = MaxStringLength;
	type HistoryDepth = HistoryDepth;
	type GasAssetId = GasAssetId;
	type PartnerAttributionProvider = PartnerAttribution;
	type MaxAttributionPartners = MaxAttributionPartners;
}

impl pallet_partner_attribution::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ApproveOrigin = EnsureRoot<AccountId>;
	type EnsureFuturepass = impls::EnsureFuturepass<AccountId>;
	type FuturepassCreator = Futurepass;
	type WeightInfo = weights::pallet_partner_attribution::WeightInfo<Runtime>;
	type MaxPartners = MaxAttributionPartners;

	#[cfg(feature = "runtime-benchmarks")]
	type MultiCurrency = AssetsExt;
}

impl pallet_maintenance_mode::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type StringLimit = AssetsStringLimit;
	type WeightInfo = weights::pallet_maintenance_mode::WeightInfo<Self>;
	type SudoPallet = Sudo;
	type TimestampPallet = Timestamp;
	type ImOnlinePallet = ImOnline;
	type EthyPallet = EthBridge;
	type DemocracyPallet = Democracy;
	type PreimagePallet = Preimage;
	type CouncilPallet = Council;
	type SchedulerPallet = Scheduler;
}

parameter_types! {
	pub const CrowdSalePalletId: PalletId = PalletId(*b"crowdsal");
	// Some low limit to prevent overworking on_initialize
	pub const MaxSalesPerBlock: u32 = 5;
	// Limit for bounded vec of max consecutive sales. Should be a reasonable upper bound
	pub const MaxConsecutiveSales: u32 = 2_000;
	// Maximum number of payments to be processed per offchain_worker call for auto distributing sales
	pub const MaxPaymentsPerBlock: u32 = 100;
	pub const MaxSaleDuration: BlockNumber = 1_944_000; // ~3 months
}

impl pallet_crowdsale::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletId = CrowdSalePalletId;
	type StringLimit = AssetsStringLimit;
	type ProxyCallValidator = impls::CrowdsaleProxyVaultValidator;
	type MultiCurrency = AssetsExt;
	type NFTExt = Nft;
	type MaxSalesPerBlock = MaxSalesPerBlock;
	type MaxConsecutiveSales = MaxConsecutiveSales;
	type MaxPaymentsPerBlock = MaxPaymentsPerBlock;
	type MaxSaleDuration = MaxSaleDuration;
	type UnsignedInterval = UnsignedInterval;
	type WeightInfo = weights::pallet_crowdsale::WeightInfo<Self>;
}

parameter_types! {
	// The upper limit of weight used per block for migrations is 10%
	// Note, this could still be smaller if we set a smaller BlockLimit within pallet-migration
	pub MaxMigrationWeight: Weight = Perbill::from_percent(10) * MAXIMUM_BLOCK_WEIGHT;
}

impl pallet_migration::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// Set to NoopMigration if no migration is in progress
	type CurrentMigration = NoopMigration;
	type MaxMigrationWeight = MaxMigrationWeight;
	type WeightInfo = weights::pallet_migration::WeightInfo<Runtime>;
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 5 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
	pub MaxCollectivesProposalWeight: Weight = Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type MaxMembers = CouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type MaxProposalWeight = MaxCollectivesProposalWeight;
}

parameter_types! {
	pub const CandidacyBond: Balance = 1500 * ONE_ROOT;
	// 1 storage item created, key size is 32 bytes, value size is 16+16.
	pub const VotingBondBase: Balance = deposit(1, 64);
	// additional data per vote is 32 bytes (account id).
	pub const VotingBondFactor: Balance = deposit(0, 32);
	pub const TermDuration: BlockNumber = 25 * MINUTES;
	pub const DesiredMembers: u32 = 5;
	pub const DesiredRunnersUp: u32 = 3;
	pub const MaxVotesPerVoter: u32 = 16;
	pub const MaxVoters: u32 = 512;
	pub const MaxCandidates: u32 = 64;
	pub const ElectionsPhragmenPalletId: LockIdentifier = *b"phrelect";
}

// Make sure that there are no more than `MaxMembers` members elected via elections-phragmen.
const_assert!(DesiredMembers::get() <= CouncilMaxMembers::get());

impl pallet_elections_phragmen::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = ElectionsPhragmenPalletId;
	type Currency = Balances;
	type ChangeMembers = Council;
	// NOTE: this implies that council's genesis members cannot be set directly and must come from
	// this module.
	type InitializeMembers = Council;
	type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;
	type CandidacyBond = CandidacyBond;
	type VotingBondBase = VotingBondBase;
	type VotingBondFactor = VotingBondFactor;
	type LoserCandidate = ();
	type KickedMember = ();
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type TermDuration = TermDuration;
	type MaxVoters = MaxVoters;
	type MaxVotesPerVoter = MaxVotesPerVoter;
	type MaxCandidates = MaxCandidates;
	type WeightInfo = pallet_elections_phragmen::weights::SubstrateWeight<Runtime>;
}

// Define the privileged set of accounts that can fast-track proposals
pub struct FastTrackMembers;

impl frame_support::traits::SortedMembers<AccountId> for FastTrackMembers {
	fn sorted_members() -> Vec<AccountId> {
		// Return all Council members - as an example.
		pallet_collective::Members::<Runtime, CouncilCollective>::get()
	}
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber =  28 * DAYS;
	pub const VotingPeriod: BlockNumber = 28 * DAYS;
	pub const VoteLockPeriod: BlockNumber = 7 * DAYS;
	pub const FastTrackVotingPeriod: BlockNumber = 5 * MINUTES;
	pub const MinimumDeposit: Balance = 1500 * ONE_ROOT;
	pub const EnactmentPeriod: BlockNumber = 30 * DAYS;
	pub const CooloffPeriod: BlockNumber = 28 * DAYS;
	pub const MaxProposals: u32 = 100;
	pub const MaxVotes: u32 = 200;
}

impl pallet_democracy::Config for Runtime {
	type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Preimages = Preimage;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type VoteLockingPeriod = VoteLockPeriod;
	// Same as EnactmentPeriod
	type MinimumDeposit = MinimumDeposit;
	type InstantAllowed = frame_support::traits::ConstBool<true>;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	type CooloffPeriod = CooloffPeriod;
	type MaxVotes = MaxVotes;
	type MaxProposals = MaxProposals;
	type MaxDeposits = ConstU32<100>;
	type MaxBlacklisted = ConstU32<100>;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 5>;
	/// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 5>;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>;
	type SubmitOrigin = EnsureSigned<AccountId>;
	/// Two fifths of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 5>;
	type InstantOrigin = EitherOfDiverse<
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>,
		frame_system::EnsureSignedBy<FastTrackMembers, AccountId>,
	>;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// To cancel a proposal before it has been passed, the council must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>,
	>;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cool-off period.
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, CouncilCollective>;
	type PalletsOrigin = OriginCaller;
	// NOTE: check where we want this to be
	type Slash = SlashImbalanceHandler;
	type VoteWeight = QuadraticVoteWeight;
}

construct_runtime!(
	pub enum Runtime {
		System: frame_system = 0,
		Babe: pallet_babe = 1,
		Timestamp: pallet_timestamp = 2,
		Scheduler: pallet_scheduler = 3,
		Utility: pallet_utility = 4,
		Recovery: pallet_recovery = 33,
		Multisig: pallet_multisig = 28,

		// Monetary
		Balances: pallet_balances = 5,
		Assets: pallet_assets = 6,
		AssetsExt: pallet_assets_ext = 7,
		Authorship: pallet_authorship = 8,
		Staking: pallet_staking = 9,
		Offences: pallet_offences = 10,

		// Validators
		Session: pallet_session = 11,
		Grandpa: pallet_grandpa = 12,
		ImOnline: pallet_im_online = 13,

		// World
		Sudo: pallet_sudo = 14,
		TransactionPayment: pallet_transaction_payment = 15,
		Dex: pallet_dex = 16,
		Nft: pallet_nft = 17,
		Sft: pallet_sft = 43,
		XRPLBridge: pallet_xrpl_bridge = 18,
		Xrpl: pallet_xrpl = 35,
		TokenApprovals: pallet_token_approvals = 19,
		Historical: pallet_session::historical = 20,
		Echo: pallet_echo = 21,
		Marketplace: pallet_marketplace = 44,
		Preimage: pallet_preimage = 45,
		VortexDistribution: pallet_vortex_distribution = 46,
		PartnerAttribution: pallet_partner_attribution = 53,
		FeeProxy: pallet_fee_proxy = 31,
		FeeControl: pallet_fee_control = 40,
		Xls20: pallet_xls20 = 42,
		Doughnut: pallet_doughnut = 48,
		MaintenanceMode: pallet_maintenance_mode = 47,
		Crowdsale: pallet_crowdsale = 49,
		Nfi: pallet_nfi = 50,
		Migration: pallet_migration = 51,
		SyloDataVerification: pallet_sylo_data_verification = 52,
		LiquidityPools: pallet_liquidity_pools = 54,
		SyloDataPermissions: pallet_sylo_data_permissions = 55,
		SyloActionPermissions: pallet_sylo_action_permissions = 56,

		// Election pallet. Only works with staking
		ElectionProviderMultiPhase: pallet_election_provider_multi_phase = 22,
		VoterList: pallet_bags_list = 23,
		TxFeePot: pallet_tx_fee_pot = 24,

		// EVM
		Ethereum: pallet_ethereum = 26,
		EVM: pallet_evm = 27,
		EVMChainId: pallet_evm_chain_id = 41,
		EthBridge: pallet_ethy = 25,
		Erc20Peg: pallet_erc20_peg::{Pallet, Call, Storage, Event<T>} = 29,
		NftPeg: pallet_nft_peg = 30,

		// FuturePass Account
		Proxy: pallet_proxy = 32,
		Futurepass: pallet_futurepass = 34,

		// Governance
		Council: pallet_collective::<Instance1> = 57,
		Elections: pallet_elections_phragmen = 58,
		Democracy: pallet_democracy = 59,
	}
);

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256Hash>;
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
	pallet_maintenance_mode::MaintenanceChecker<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic =
	fp_self_contained::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra, H160>;

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	(migrations::AllMigrations,),
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

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
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

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
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
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl sp_consensus_babe::BabeApi<Block> for Runtime {
		fn configuration() -> sp_consensus_babe::BabeConfiguration {
			// The choice of `c` parameter (where `1 - c` represents the
			// probability of a slot being empty), is done in accordance to the
			// slot duration and expected target block time, for safely
			// resisting network delays of maximum two seconds.
			// <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
			sp_consensus_babe::BabeConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: BABE_GENESIS_EPOCH_CONFIG.c,
				authorities: Babe::authorities().to_vec(),
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
			Dex::get_amounts_out(amount_in, &path).map_err(|e| match e {
				sp_runtime::DispatchError::Arithmetic(_)  =>
					sp_runtime::DispatchError::Other("Insufficient Liquidity"),
					e => e,
			})
		}

		fn get_amounts_in(
			amount_out: Balance,
			path: Vec<AssetId>,
		) -> Result<Vec<Balance>, sp_runtime::DispatchError> {
			Dex::get_amounts_in(amount_out, &path).map_err(|e| match e {
				sp_runtime::DispatchError::Arithmetic(_)  =>
					sp_runtime::DispatchError::Other("Insufficient Liquidity"),
					e => e,
			})
		}

		fn get_lp_token_id(
			asset_id_a: AssetId,
			asset_id_b: AssetId,
		) -> Result<AssetId, sp_runtime::DispatchError> {
			Dex::get_lp_token_id(asset_id_a, asset_id_b)
		}

		fn get_liquidity(
			asset_id_a: AssetId,
			asset_id_b: AssetId,
		) -> (Balance, Balance) {
			Dex::get_liquidity(asset_id_a, asset_id_b)
		}

		fn get_trading_pair_status(
			asset_id_a: AssetId,
			asset_id_b: AssetId,
		) -> TradingPairStatus {
			Dex::get_trading_pair_status(asset_id_a, asset_id_b)
		}
	}

	impl pallet_nft_rpc_runtime_api::NftApi<
		Block,
		AccountId,
		Runtime,
	> for Runtime {
		fn owned_tokens(collection_id: CollectionUuid, who: AccountId, cursor: SerialNumber, limit: u16) -> (SerialNumber, TokenCount, Vec<SerialNumber>) {
			Nft::owned_tokens(collection_id, &who, cursor, limit)
		}
		fn token_uri(token_id: TokenId) -> Vec<u8> {
			Nft::token_uri(token_id)
		}
		fn collection_details(collection_id: CollectionUuid) -> Result<CollectionDetail<AccountId>, sp_runtime::DispatchError> {
			Nft::collection_details(collection_id)
		}
	}

	impl pallet_assets_ext_rpc_runtime_api::AssetsExtApi<
		Block,
		AccountId,
	> for Runtime {
		fn free_balance(asset_id: AssetId, who: AccountId, keep_alive: bool) -> String {
			let preservation = match keep_alive {
				true => Preservation::Preserve,
				false => Preservation::Expendable,
			};
			let bal = AssetsExt::reducible_balance(asset_id, &who, preservation, Fortitude::Polite);
			alloc::format!("{}", bal)
		 }
	}

	impl pallet_sft_rpc_runtime_api::SftApi<Block, Runtime> for Runtime {
		fn token_uri(token_id: TokenId) -> Vec<u8> {
			Sft::token_uri(token_id)
		}
	}

	impl pallet_sylo_data_permissions_rpc_runtime_api::SyloDataPermissionsApi<Block, AccountId> for Runtime {
		fn get_permissions(
			data_author: AccountId,
			grantee: AccountId,
			data_ids: Vec<String>,
		) -> Result<pallet_sylo_data_permissions::GetPermissionsResult, sp_runtime::DispatchError> {
			SyloDataPermissions::get_permissions(
				data_author,
				grantee,
				data_ids,
			)
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
			FeeControl::min_gas_price().0
		}

		fn account_code_at(address: H160) -> Vec<u8> {
			pallet_evm::AccountCodes::<Runtime>::get(address)
		}

		fn author() -> H160 {
			<pallet_evm::Pallet<Runtime>>::find_author()
		}

		fn storage_at(address: H160, index: U256) -> H256 {
			let mut tmp = [0u8; 32];
			index.to_big_endian(&mut tmp);
			pallet_evm::AccountStorages::<Runtime>::get(address, H256::from_slice(&tmp[..]))
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

			let is_transactional = false;
			let validate = true;
			let evm_config = config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config());

			// TODO: refactor the code once we are on polkadot-v1.1.0, ref - https://github.com/polkadot-evm/frontier/pull/1121
			let mut estimated_transaction_len = data.len() +
				20 + // to
				20 + // from
				32 + // value
				32 + // gas_limit
				32 + // nonce
				1 + // TransactionAction
				8 + // chain id
				65; // signature

			if max_fee_per_gas.is_some() {
				estimated_transaction_len += 32;
			}
			if max_priority_fee_per_gas.is_some() {
				estimated_transaction_len += 32;
			}
			if access_list.is_some() {
				estimated_transaction_len += access_list.encoded_size();
			}

			let gas_limit = gas_limit.min(u64::MAX.into()).low_u64();
			let without_base_extrinsic_weight = true;

			let (weight_limit, proof_size_base_cost) =
				match <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(
					gas_limit,
					without_base_extrinsic_weight
				) {
					weight_limit if weight_limit.proof_size() > 0 => {
						(Some(weight_limit), Some(estimated_transaction_len as u64))
					}
					_ => (None, None),
				};

			<Runtime as pallet_evm::Config>::Runner::call(
				from,
				to,
				data,
				value,
				gas_limit.unique_saturated_into(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.unwrap_or_default(),
				is_transactional,
				validate,
				weight_limit,
				proof_size_base_cost,
				evm_config,
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

			let is_transactional = false;
			let validate = true;
			let evm_config = config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config());

			// TODO: refactor the code once we are on polkadot-v1.1.0, ref - https://github.com/polkadot-evm/frontier/pull/1121
			let mut estimated_transaction_len = data.len() +
				20 + // from
				32 + // value
				32 + // gas_limit
				32 + // nonce
				1 + // TransactionAction
				8 + // chain id
				65; // signature

			if max_fee_per_gas.is_some() {
				estimated_transaction_len += 32;
			}
			if max_priority_fee_per_gas.is_some() {
				estimated_transaction_len += 32;
			}
			if access_list.is_some() {
				estimated_transaction_len += access_list.encoded_size();
			}

			let gas_limit = if gas_limit > U256::from(u64::MAX) {
				u64::MAX
			} else {
				gas_limit.low_u64()
			};
			let without_base_extrinsic_weight = true;

			let (weight_limit, proof_size_base_cost) =
				match <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(
					gas_limit,
					without_base_extrinsic_weight
				) {
					weight_limit if weight_limit.proof_size() > 0 => {
						(Some(weight_limit), Some(estimated_transaction_len as u64))
					}
					_ => (None, None),
				};

			<Runtime as pallet_evm::Config>::Runner::create(
				from,
				data,
				value,
				gas_limit.unique_saturated_into(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.unwrap_or_default(),
				is_transactional,
				validate,
				weight_limit,
				proof_size_base_cost,
				evm_config,
			).map_err(|err| err.error.into())
		}

		fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
			pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
		}

		fn current_block() -> Option<pallet_ethereum::Block> {
			pallet_ethereum::CurrentBlock::<Runtime>::get()
		}

		fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
			pallet_ethereum::CurrentReceipts::<Runtime>::get()
		}

		fn current_all() -> (
			Option<pallet_ethereum::Block>,
			Option<Vec<pallet_ethereum::Receipt>>,
			Option<Vec<TransactionStatus>>
		) {
			(
				pallet_ethereum::CurrentBlock::<Runtime>::get(),
				pallet_ethereum::CurrentReceipts::<Runtime>::get(),
				pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get(),
			)
		}

		fn extrinsic_filter(
			xts: Vec<<Block as BlockT>::Extrinsic>,
		) -> Vec<EthereumTransaction> {
			xts.into_iter().filter_map(|xt| match xt.0.function {
				RuntimeCall::Ethereum(transact { transaction }) => Some(transaction),
				_ => None
			}).collect::<Vec<EthereumTransaction>>()
		}

		fn elasticity() -> Option<Permill> {
			// We currently do not use or set elasticity; always return zero
			Some(Permill::zero())
		}

		fn gas_limit_multiplier_support() {}

		fn pending_block(
			xts: Vec<<Block as sp_runtime::traits::Block>::Extrinsic>
		) -> (
			Option<pallet_ethereum::Block>, Option<sp_std::prelude::Vec<TransactionStatus>>
		) {
			for ext in xts.into_iter() {
				let _ = Executive::apply_extrinsic(ext);
			}

			Ethereum::on_finalize(System::block_number() + 1);
			(
				pallet_ethereum::CurrentBlock::<Runtime>::get(),
				pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
			)
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
			let door_signers = pallet_ethy::NotaryXrplKeys::<Runtime>::get().into_inner();
			ValidatorSet {
				proof_threshold: door_signers.len().saturating_sub(1) as u32, // tolerate 1 missing witness
				validators: door_signers,
				id: pallet_ethy::NotarySetId::<Runtime>::get(), // the set Id is the same as the overall Ethy set Id
			}
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			log::info!("try-runtime::on_runtime_upgrade.");

			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).map_err(|err|{
				log::info!("try-runtime::on_runtime_upgrade failed with: {:?}", err);
				err
			}).unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}

		fn execute_block(block: Block, state_root_check: bool, signature_check: bool, select: frame_try_runtime::TryStateSelect) -> Weight {
			log::info!(
				target: "runtime::kusama", "try-runtime: executing block #{} ({:?}) / root checks: {:?} / sanity-checks: {:?}",
				block.header.number,
				block.header.hash(),
				state_root_check,
				select,
			);
			Executive::try_execute_block(block, state_root_check, signature_check, select).expect("try_execute_block failed")
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
	let fee_proxy = TransactionAction::Call(H160::from_low_u64_be(FEE_PROXY_ADDRESS));

	if action == fee_proxy {
		let (input, gas_limit, max_fee_per_gas, max_priority_fee_per_gas) = match eth_tx {
			EthereumTransaction::EIP1559(t) => {
				(t.input, t.gas_limit, t.max_fee_per_gas, t.max_priority_fee_per_gas)
			},
			_ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?,
		};

		let (payment_asset_id, _target, _input) =
			FeePreferencesRunner::<Runtime, Runtime, Futurepass>::decode_input(input)?;

		let FeePreferencesData { max_fee_scaled, path, .. } =
			get_fee_preferences_data::<Runtime, Runtime, Futurepass>(
				gas_limit.as_u64(),
				<Runtime as pallet_fee_proxy::Config>::EVMBaseFeeProvider::evm_base_fee_per_gas(),
				Some(max_fee_per_gas),
				Some(max_priority_fee_per_gas),
				payment_asset_id,
			)?;

		let amounts_in = Dex::get_amounts_in(max_fee_scaled, &path)
			.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

		// ensure user owns max payment amount (in tokens) - once converted from max_fee_per_gas
		let user_asset_balance = <pallet_assets_ext::Pallet<Runtime> as Inspect<
			<Runtime as frame_system::Config>::AccountId,
		>>::reducible_balance(
			payment_asset_id,
			&<Runtime as frame_system::Config>::AccountId::from(*source),
			Preservation::Expendable,
			Fortitude::Polite,
		);
		ensure!(
			amounts_in[0] <= user_asset_balance,
			TransactionValidityError::Invalid(InvalidTransaction::Payment)
		);
	}

	Ok(())
}

impl fp_self_contained::SelfContainedCall for RuntimeCall {
	type SignedInfo = H160;

	fn is_self_contained(&self) -> bool {
		match self {
			RuntimeCall::Ethereum(call) => call.is_self_contained(),
			RuntimeCall::Xrpl(call) => call.is_self_contained(),
			RuntimeCall::Doughnut(call) => call.is_self_contained(),
			_ => false,
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => call.check_self_contained(),
			RuntimeCall::Xrpl(call) => call.check_self_contained(),
			RuntimeCall::Doughnut(call) => call.check_self_contained(),
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
			RuntimeCall::Ethereum(ref call) => {
				Some(validate_self_contained_inner(self, call, signed_info, dispatch_info, len))
			},
			RuntimeCall::Xrpl(ref call) => {
				call.validate_self_contained(signed_info, dispatch_info, len)
			},
			RuntimeCall::Doughnut(ref call) => {
				call.validate_self_contained(signed_info, dispatch_info, len)
			},
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
			RuntimeCall::Ethereum(call) => {
				call.pre_dispatch_self_contained(signed_info, dispatch_info, len)
			},
			RuntimeCall::Xrpl(ref call) => {
				call.pre_dispatch_self_contained(signed_info, dispatch_info, len)
			},
			RuntimeCall::Doughnut(ref call) => {
				call.pre_dispatch_self_contained(signed_info, dispatch_info, len)
			},
			_ => None,
		}
	}

	fn apply_self_contained(
		self,
		info: Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<Self>,
		len: usize,
	) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
		match self {
			call @ RuntimeCall::Ethereum(pallet_ethereum::Call::transact { .. }) => {
				Some(call.dispatch(RuntimeOrigin::from(
					pallet_ethereum::RawOrigin::EthereumTransaction(info),
				)))
			},
			RuntimeCall::Xrpl(call) => pallet_xrpl::Call::<Runtime>::apply_self_contained(
				call.into(),
				&info,
				dispatch_info,
				len,
			),
			RuntimeCall::Doughnut(call) => pallet_doughnut::Call::<Runtime>::apply_self_contained(
				call.into(),
				&info,
				dispatch_info,
				len,
			),
			_ => None,
		}
	}
}

fn validate_self_contained_inner(
	call: &RuntimeCall,
	eth_call: &pallet_ethereum::Call<Runtime>,
	signed_info: &<RuntimeCall as fp_self_contained::SelfContainedCall>::SignedInfo,
	dispatch_info: &DispatchInfoOf<RuntimeCall>,
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
			RuntimeCall::Ethereum(pallet_ethereum::Call::transact { transaction }) => {
				transaction_asset_check(signed_info, transaction, action)
			},
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
extern crate core;

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
		[pallet_multisig, Multisig]
		[pallet_session, SessionBench::<Runtime>]
		[pallet_bags_list, VoterList]
		[pallet_election_provider_multi_phase, ElectionProviderMultiPhase]
		[frame_election_provider_support, EPSBench::<Runtime>]
		[pallet_recovery, Recovery]
		[pallet_proxy, Proxy]
		[pallet_preimage, Preimage]
		[pallet_sudo, Sudo]
		// Local
		[pallet_nft, Nft]
		[pallet_nfi, Nfi]
		[pallet_sft, Sft]
		[pallet_fee_control, FeeControl]
		[pallet_nft_peg, NftPeg]
		[pallet_xrpl_bridge, XRPLBridge]
		[pallet_xrpl, Xrpl]
		[pallet_erc20_peg, Erc20Peg]
		[pallet_ethy, EthBridge]
		[pallet_echo, Echo]
		[pallet_assets_ext, AssetsExt]
		[pallet_evm_chain_id, EVMChainId]
		[pallet_token_approvals, TokenApprovals]
		[pallet_xls20, Xls20]
		[pallet_futurepass, Futurepass]
		[pallet_vortex_distribution, VortexDistribution]
		[pallet_partner_attribution, PartnerAttribution]
		[pallet_dex, Dex]
		[pallet_maintenance_mode, MaintenanceMode]
		[pallet_liquidity_pools, LiquidityPools]
		[pallet_marketplace, Marketplace]
		[pallet_doughnut, Doughnut]
		[pallet_maintenance_mode, MaintenanceMode]
		[pallet_crowdsale, Crowdsale]
		[pallet_evm, EVM]
		[pallet_migration, Migration]
		[pallet_sylo_data_verification, SyloDataVerification]
		[pallet_sylo_data_permissions, SyloDataPermissions]
		[pallet_sylo_action_permissions, SyloActionPermissions]
	);
}
