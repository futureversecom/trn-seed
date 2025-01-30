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

/// Prelude to be used in mocks and tests, for ease of use
pub mod test_prelude {
	pub use crate::{
		test_utils::{account_creation::*, test_constants::*, test_types::*, *},
		*,
	};
	pub use frame_support::{
		assert_err, assert_noop, assert_ok, assert_storage_noop, construct_runtime,
		dispatch::{DispatchError, DispatchResult},
		parameter_types,
		storage::{StorageMap, StorageValue},
		traits::BuildGenesisConfig,
		weights::{constants::RocksDbWeight as DbWeight, Weight},
		PalletId,
	};
	pub use frame_system::{EnsureRoot, RawOrigin};
	pub use seed_primitives::{
		test_utils::TestExt, AccountId, AssetId, Balance, CollectionUuid, MetadataScheme,
		SerialNumber, TokenId,
	};
	pub use sp_core::{H160, H256, U256};
	pub use sp_runtime::{
		testing::Header,
		traits::{BlakeTwo256, IdentityLookup},
		ArithmeticError, BoundedVec, BuildStorage,
		DispatchError::BadOrigin,
		Permill, TokenError,
	};
	pub use sp_std::{vec, vec::Vec};
}

pub mod test_types {
	pub type BlockNumber = u64;

	pub type UncheckedExtrinsic<Test> = frame_system::mocking::MockUncheckedExtrinsic<Test>;

	pub type Block<Test> = frame_system::mocking::MockBlock<Test>;
}

pub mod test_constants {
	use seed_primitives::AssetId;

	pub const ROOT_ASSET_ID: AssetId = 1;
	pub const XRP_ASSET_ID: AssetId = 2;
	pub const VTX_ASSET_ID: AssetId = 3;
	pub const SPENDING_ASSET_ID: AssetId = XRP_ASSET_ID;
}

/// Helper functions for creating accounts to be used in tests
pub mod account_creation {
	use seed_primitives::AccountId;
	use sp_core::H160;

	/// Create an AccountId from a u64 seed
	pub fn create_account(seed: u64) -> AccountId {
		AccountId::from(H160::from_low_u64_be(seed))
	}

	/// Creates a random AccountId
	pub fn random_account() -> AccountId {
		AccountId::from(H160::random())
	}

	/// Common account Alice
	pub fn alice() -> AccountId {
		create_account(1000)
	}

	/// Common account Bob
	pub fn bob() -> AccountId {
		create_account(2000)
	}

	/// Common account Charlie
	pub fn charlie() -> AccountId {
		create_account(3000)
	}

	/// Common account Dave
	pub fn dave() -> AccountId {
		create_account(4000)
	}
}

#[macro_export]
macro_rules! impl_frame_system_config {
	($test:ident) => {
		parameter_types! {
			pub const BlockHashCount: u64 = 250;
		}

		type BlockNumber = u64;

		impl frame_system::Config for $test {
			type Block = frame_system::mocking::MockBlock<$test>;
			type BlockWeights = ();
			type BlockLength = ();
			type BaseCallFilter = frame_support::traits::Everything;
			type RuntimeOrigin = RuntimeOrigin;
			type Nonce = u32;
			type RuntimeCall = RuntimeCall;
			type Hash = H256;
			type Hashing = BlakeTwo256;
			type AccountId = AccountId;
			type Lookup = IdentityLookup<Self::AccountId>;
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
	};
}

#[macro_export]
macro_rules! impl_pallet_balance_config {
	($test:ident) => {
		parameter_types! {
			pub const MaxReserves: u32 = 50;
			pub const ExistentialDeposit: u128 = 1;
		}

		impl pallet_balances::Config for $test {
			type Balance = Balance;
			type RuntimeEvent = RuntimeEvent;
			type RuntimeHoldReason = ();
			type FreezeIdentifier = ();
			type DustRemoval = ();
			type ExistentialDeposit = ExistentialDeposit;
			type AccountStore = System;
			type MaxLocks = ();
			type WeightInfo = ();
			type MaxReserves = MaxReserves;
			type ReserveIdentifier = [u8; 8];
			type MaxHolds = sp_core::ConstU32<0>;
			type MaxFreezes = sp_core::ConstU32<0>;
		}
	};
}

#[macro_export]
macro_rules! impl_pallet_timestamp_config {
	($test:ident) => {
		parameter_types! {
			pub const MinimumPeriod: u64 = 5;
		}

		impl pallet_timestamp::Config for $test {
			type Moment = u64;
			type OnTimestampSet = ();
			type MinimumPeriod = MinimumPeriod;
			type WeightInfo = ();
		}
	};
}

#[macro_export]
macro_rules! impl_pallet_assets_config {
	($test:ident) => {
		parameter_types! {
			pub const AssetDeposit: Balance = 1_000_000;
			pub const AssetAccountDeposit: Balance = 16;
			pub const ApprovalDeposit: Balance = 1;
			pub const AssetsStringLimit: u32 = 50;
			pub const MetadataDepositBase: Balance = 1 * 68;
			pub const MetadataDepositPerByte: Balance = 1;
			pub const RemoveItemsLimit: u32 = 100;
		}

		impl pallet_assets::Config for $test {
			type RuntimeEvent = RuntimeEvent;
			type Balance = Balance;
			type AssetId = AssetId;
			type Currency = Balances;
			type ForceOrigin = EnsureRoot<AccountId>;
			type AssetDeposit = AssetDeposit;
			type MetadataDepositBase = MetadataDepositBase;
			type MetadataDepositPerByte = MetadataDepositPerByte;
			type ApprovalDeposit = ApprovalDeposit;
			type StringLimit = AssetsStringLimit;
			type Freezer = ();
			type Extra = ();
			type WeightInfo = ();
			type AssetAccountDeposit = AssetAccountDeposit;
			type RemoveItemsLimit = RemoveItemsLimit;
			type AssetIdParameter = AssetId;
			type CreateOrigin = frame_system::EnsureNever<AccountId>;
			type CallbackHandle = ();
			pallet_assets::runtime_benchmarks_enabled! {
				type BenchmarkHelper = ();
			}
		}
	};
}

#[macro_export]
macro_rules! impl_pallet_assets_ext_config {
	($test:ident) => {
		parameter_types! {
			pub const NativeAssetId: AssetId = 1;
			pub const AssetsExtPalletId: PalletId = PalletId(*b"assetext");
			pub const MaxHolds: u32 = 16;
			pub const TestParachainId: u32 = 100;
		}

		impl pallet_assets_ext::Config for $test {
			type RuntimeEvent = RuntimeEvent;
			type ParachainId = TestParachainId;
			type MaxHolds = MaxHolds;
			type NativeAssetId = NativeAssetId;
			type OnNewAssetSubscription = ();
			type PalletId = AssetsExtPalletId;
			type WeightInfo = ();
		}
	};
}

#[macro_export]
macro_rules! impl_pallet_nft_config {
	($test:ident) => {
		parameter_types! {
			pub const NftPalletId: PalletId = PalletId(*b"nftokens");
			pub const MaxTokensPerCollection: u32 = 10_000;
			pub const MintLimit: u32 = 1000;
			pub const Xls20PaymentAsset: AssetId = 2;
			pub const StringLimit: u32 = 50;
			pub const FeePotId: PalletId = PalletId(*b"txfeepot");
		}

		impl pallet_nft::Config for Test {
			type RuntimeEvent = RuntimeEvent;
			type RuntimeCall = RuntimeCall;
			type MaxTokensPerCollection = MaxTokensPerCollection;
			type MintLimit = MintLimit;
			type OnTransferSubscription = ();
			type OnNewAssetSubscription = ();
			type MultiCurrency = AssetsExt;
			type PalletId = NftPalletId;
			type ParachainId = TestParachainId;
			type Xls20MintRequest = ();
			type WeightInfo = ();
			type StringLimit = StringLimit;
			type NFIRequest = ();
			type Migrator = ();
		}
	};
}

#[macro_export]
macro_rules! impl_pallet_sft_config {
	($test:ident) => {
		parameter_types! {
			pub const SftPalletId: PalletId = PalletId(*b"sftokens");
			pub const MaxTokensPerSftCollection: u32 = 10_000;
			pub const MaxSerialsPerSftMint: u32 = 100;
			pub const MaxOwnersPerSftToken: u32 = 100;
		}

		impl pallet_sft::Config for Test {
			type RuntimeEvent = RuntimeEvent;
			type MultiCurrency = AssetsExt;
			type NFTExt = Nft;
			type OnTransferSubscription = ();
			type OnNewAssetSubscription = ();
			type PalletId = SftPalletId;
			type ParachainId = TestParachainId;
			type StringLimit = StringLimit;
			type WeightInfo = ();
			type MaxTokensPerSftCollection = MaxTokensPerSftCollection;
			type MaxSerialsPerMint = MaxSerialsPerSftMint;
			type MaxOwnersPerSftToken = MaxOwnersPerSftToken;
			type NFIRequest = ();
		}
	};
}

#[macro_export]
macro_rules! impl_pallet_marketplace_config {
	($test:ident) => {
		parameter_types! {
			pub const MarketplacePalletId: PalletId = PalletId(*b"marketpl");
			pub const DefaultListingDuration: u64 = 5;
			pub const MaxOffers: u32 = 10;
			pub const MaxTokensPerListing: u32 = 100;
			pub const MarketplaceNetworkFeePercentage: Permill = Permill::from_perthousand(5);
			pub const MarketplaceDefaultFeeTo: Option<PalletId> = None;
		}

		impl pallet_marketplace::Config for Test {
			type RuntimeCall = RuntimeCall;
			type DefaultListingDuration = DefaultListingDuration;
			type RuntimeEvent = RuntimeEvent;
			type MultiCurrency = AssetsExt;
			type NFTExt = Nft;
			type NetworkFeePercentage = MarketplaceNetworkFeePercentage;
			type PalletId = MarketplacePalletId;
			type WeightInfo = ();
			type MaxTokensPerListing = MaxTokensPerListing;
			type MaxOffers = MaxOffers;
			type DefaultFeeTo = MarketplaceDefaultFeeTo;
		}
	};
}

#[macro_export]
macro_rules! impl_pallet_fee_control_config {
	($test:ident) => {
		impl pallet_fee_control::Config for $test {
			type RuntimeEvent = RuntimeEvent;
			type WeightInfo = ();
			type FeeConfig = ();
		}
	};
}

#[macro_export]
macro_rules! impl_pallet_evm_config {
	($test:ident) => {
		pub struct FindAuthorTruncated;
		impl FindAuthor<H160> for FindAuthorTruncated {
			fn find_author<'a, I>(_digests: I) -> Option<H160>
			where
				I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
			{
				None
			}
		}

		pub struct MockAddressMapping;
		impl AddressMapping<AccountId> for MockAddressMapping {
			fn into_account_id(address: H160) -> AccountId {
				address.into()
			}
		}

		pub struct MockBlockHashMapping<$test>(PhantomData<$test>);
		impl<$test> BlockHashMapping for MockBlockHashMapping<$test> {
			fn block_hash(_number: u32) -> H256 {
				H256::default()
			}
		}

		pub struct FixedGasWeightMapping;
		impl GasWeightMapping for FixedGasWeightMapping {
			fn gas_to_weight(_gas: u64, _without_base_weight: bool) -> Weight {
				Weight::zero()
			}
			fn weight_to_gas(_weight: Weight) -> u64 {
				0u64
			}
		}

		parameter_types! {
			pub GasLimitPovSizeRatio: u64 = 0;
		}

		impl pallet_evm::Config for $test {
			type FeeCalculator = FeeControl;
			type GasWeightMapping = FixedGasWeightMapping;
			type BlockHashMapping = MockBlockHashMapping<$test>;
			type CallOrigin = EnsureAddressNever<AccountId>;
			type WithdrawOrigin = EnsureAddressNever<AccountId>;
			type AddressMapping = MockAddressMapping;
			type Currency = Balances;
			type RuntimeEvent = RuntimeEvent;
			type Runner = pallet_evm::runner::stack::Runner<Self>;
			type PrecompilesType = ();
			type PrecompilesValue = ();
			type ChainId = ();
			type BlockGasLimit = ();
			type OnChargeTransaction = ();
			type FindAuthor = FindAuthorTruncated;
			type HandleTxValidation = ();
			type WeightPerGas = ();
			type OnCreate = ();
			type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
			type Timestamp = Timestamp;
			type WeightInfo = ();
		}
	};
}

#[macro_export]
macro_rules! impl_pallet_dex_config {
	($test:ident) => {
		parameter_types! {
			pub const GetExchangeFee: (u32, u32) = (3, 1000); // 0.3% fee
			pub const TradingPathLimit: u32 = 3;
			pub const DEXBurnPalletId: PalletId = PalletId(*b"burnaddr");
			pub const LPTokenDecimals: u8 = 6;
			pub const DefaultFeeTo: Option<PalletId> = None;
		}

		impl pallet_dex::Config for $test {
			type RuntimeEvent = RuntimeEvent;
			type GetExchangeFee = GetExchangeFee;
			type TradingPathLimit = TradingPathLimit;
			type DEXBurnPalletId = DEXBurnPalletId;
			type LPTokenDecimals = LPTokenDecimals;
			type DefaultFeeTo = DefaultFeeTo;
			type WeightInfo = ();
			type MultiCurrency = AssetsExt;
		}
	};
}

#[macro_export]
macro_rules! impl_pallet_fee_proxy_config {
	($test:ident) => {
		impl<RuntimeId> ErcIdConversion<RuntimeId> for $test
		where
			RuntimeId: From<u32> + Into<u32>,
		{
			type EvmId = Address;

			fn evm_id_to_runtime_id(
				evm_id: Self::EvmId,
				_precompile_address_prefix: &[u8; 4],
			) -> Option<RuntimeId> {
				if H160::from(evm_id) == H160::from_low_u64_be(16000) {
					// Our expected value for the test
					return Some(RuntimeId::from(16000));
				}
				None
			}
			fn runtime_id_to_evm_id(
				runtime_id: RuntimeId,
				_precompile_address_prefix: &[u8; 4],
			) -> Self::EvmId {
				let id: u32 = runtime_id.into();
				Self::EvmId::from(H160::from_low_u64_be(id as u64))
			}
		}

		parameter_types! {
			pub const XrpAssetId: AssetId = MOCK_PAYMENT_ASSET_ID;
		}

		pub type XrpCurrency = pallet_assets_ext::AssetCurrency<Test, XrpAssetId>;

		impl pallet_fee_proxy::Config for $test {
			type RuntimeCall = RuntimeCall;
			type RuntimeEvent = RuntimeEvent;
			type PalletsOrigin = OriginCaller;
			type FeeAssetId = XrpAssetId;
			type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<XrpCurrency, ()>;
			type ErcIdConversion = Self;
			type EVMBaseFeeProvider = FeeControl;
			type MaintenanceChecker = ();
		}
	};
}

#[macro_export]
macro_rules! impl_pallet_transaction_payment_config {
	($test:ident) => {
		pub struct FeeControlWeightToFee;
		impl WeightToFee for FeeControlWeightToFee {
			type Balance = Balance;
			fn weight_to_fee(weight: &Weight) -> Self::Balance {
				FeeControl::weight_to_fee(weight)
			}
		}

		pub struct FeeControlLengthToFee;
		impl WeightToFee for FeeControlLengthToFee {
			type Balance = Balance;
			fn weight_to_fee(weight: &Weight) -> Self::Balance {
				FeeControl::length_to_fee(weight)
			}
		}

		parameter_types! {
			pub const OperationalFeeMultiplier: u8 = 1;
		}

		impl pallet_transaction_payment::Config for $test {
			type OnChargeTransaction = FeeProxy;
			type RuntimeEvent = RuntimeEvent;
			type WeightToFee = FeeControlWeightToFee;
			type LengthToFee = FeeControlLengthToFee;
			type FeeMultiplierUpdate = ();
			type OperationalFeeMultiplier = OperationalFeeMultiplier;
		}
	};
}

#[macro_export]
macro_rules! impl_pallet_futurepass_config {
	($test:ident) => {
		pub struct MockProxyProvider;

		impl<T: pallet_futurepass::Config> pallet_futurepass::ProxyProvider<T> for MockProxyProvider
		where
			<T as frame_system::Config>::AccountId: From<sp_core::H160>,
		{
			fn exists(
				futurepass: &T::AccountId,
				delegate: &T::AccountId,
				proxy_type: Option<T::ProxyType>,
			) -> bool {
				false
			}
			fn owner(futurepass: &T::AccountId) -> Option<T::AccountId> {
				None
			}
			fn delegates(futurepass: &T::AccountId) -> Vec<(T::AccountId, T::ProxyType)> {
				vec![]
			}
			fn add_delegate(
				_: &T::AccountId,
				futurepass: &T::AccountId,
				delegate: &T::AccountId,
				proxy_type: &u8,
			) -> DispatchResult {
				Ok(())
			}
			fn remove_delegate(
				_: &T::AccountId,
				futurepass: &T::AccountId,
				delegate: &T::AccountId,
			) -> DispatchResult {
				Ok(())
			}
			fn remove_account(
				receiver: &T::AccountId,
				futurepass: &T::AccountId,
			) -> DispatchResult {
				Ok(())
			}
			fn proxy_call(
				caller: T::RuntimeOrigin,
				futurepass: T::AccountId,
				call: <T as pallet_futurepass::Config>::RuntimeCall,
			) -> DispatchResult {
				Ok(())
			}
		}

		#[derive(
			Copy,
			Clone,
			Eq,
			PartialEq,
			Ord,
			PartialOrd,
			Encode,
			Decode,
			RuntimeDebug,
			MaxEncodedLen,
			TypeInfo,
		)]
		pub enum ProxyType {
			NoPermission = 0,
			Any = 1,
			NonTransfer = 2,
			Governance = 3,
			Staking = 4,
			Owner = 255,
		}

		impl Default for ProxyType {
			fn default() -> Self {
				Self::Any
			}
		}

		impl TryFrom<u8> for ProxyType {
			type Error = &'static str;
			fn try_from(value: u8) -> Result<Self, Self::Error> {
				match value {
					0 => Ok(ProxyType::NoPermission),
					1 => Ok(ProxyType::Any),
					2 => Ok(ProxyType::NonTransfer),
					3 => Ok(ProxyType::Governance),
					4 => Ok(ProxyType::Staking),
					255 => Ok(ProxyType::Owner),
					_ => Err("Invalid value for ProxyType"),
				}
			}
		}

		impl Into<u8> for ProxyType {
			fn into(self) -> u8 {
				match self {
					ProxyType::NoPermission => 0,
					ProxyType::Any => 1,
					ProxyType::NonTransfer => 2,
					ProxyType::Governance => 3,
					ProxyType::Staking => 4,
					ProxyType::Owner => 255,
				}
			}
		}

		impl InstanceFilter<RuntimeCall> for ProxyType {
			fn filter(&self, c: &RuntimeCall) -> bool {
				match self {
					ProxyType::Owner => true,
					ProxyType::Any => true,
					// TODO - need to add allowed calls under this category in v2. allowing all for
					// now.
					ProxyType::NonTransfer => true,
					ProxyType::Governance => false,
					ProxyType::Staking => false,
					ProxyType::NoPermission => false,
				}
			}

			fn is_superset(&self, o: &Self) -> bool {
				match (self, o) {
					(x, y) if x == y => true,
					(ProxyType::Owner, _) | (ProxyType::Any, _) => true,
					(_, ProxyType::Owner) | (_, ProxyType::Any) => false,
					_ => false,
				}
			}
		}

		pub struct MockFuturepassCallValidator;
		impl seed_pallet_common::ExtrinsicChecker for MockFuturepassCallValidator {
			type Call = RuntimeCall;
			type Extra = ();
			type Result = bool;
			fn check_extrinsic(_call: &Self::Call, _extra: &Self::Extra) -> Self::Result {
				false
			}
		}

		impl pallet_futurepass::Config for $test {
			type RuntimeEvent = RuntimeEvent;
			type Proxy = MockProxyProvider;
			type RuntimeCall = RuntimeCall;
			type BlacklistedCallValidator = MockFuturepassCallValidator;
			type ApproveOrigin = EnsureRoot<AccountId>;
			type ProxyType = ProxyType;
			type WeightInfo = ();
			#[cfg(feature = "runtime-benchmarks")]
			type MultiCurrency = AssetsExt;
		}
	};
}

// TODO: satisfy `ProxyType` trait
#[macro_export]
macro_rules! impl_pallet_proxy_config {
	($test:ident) => {
		pub const fn deposit(items: u32, bytes: u32) -> Balance {
			items as Balance * 100 + (bytes as Balance) * 6
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
		}

		impl pallet_proxy::Config for Test {
			type RuntimeEvent = RuntimeEvent;
			type RuntimeCall = RuntimeCall;
			type Currency = Balances;

			type ProxyType = ProxyType;
			type ProxyDepositBase = ProxyDepositBase;
			type ProxyDepositFactor = ProxyDepositFactor;
			type MaxProxies = ConstU32<32>;
			type MaxPending = ConstU32<32>;
			type CallHasher = BlakeTwo256;
			type AnnouncementDepositBase = AnnouncementDepositBase;
			type AnnouncementDepositFactor = AnnouncementDepositFactor;
			type WeightInfo = ();
		}
	};
}

#[macro_export]
macro_rules! impl_pallet_scheduler_config {
	($test:ident) => {
		parameter_types! {
			pub const MaxScheduledPerBlock: u32 = 50;
			pub const MaximumWeight: Weight = Weight::from_parts(9_000_000_000_000, 9_000_000_000_000);
		}

		impl pallet_scheduler::Config for Test {
			type RuntimeEvent = RuntimeEvent;
			type RuntimeOrigin = RuntimeOrigin;
			type PalletsOrigin = OriginCaller;
			type RuntimeCall = RuntimeCall;
			type MaximumWeight = MaximumWeight;
			type ScheduleOrigin = EnsureRoot<AccountId>;
			type MaxScheduledPerBlock = MaxScheduledPerBlock;
			type OriginPrivilegeCmp = frame_support::traits::EqualPrivilegeOnly;
			type WeightInfo = ();
			type Preimages = ();
		}
	};
}
