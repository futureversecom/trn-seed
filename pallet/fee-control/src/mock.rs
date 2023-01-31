use crate::{self as fee_control, *};

use frame_system::EnsureRoot;
use pallet_evm::{AddressMapping, BlockHashMapping, EnsureAddressNever};
pub use seed_primitives::types::{AccountId, Balance};
use seed_primitives::AssetId;

use frame_support::{parameter_types, traits::FindAuthor, weights::WeightToFee, PalletId};
use precompile_utils::{Address, ErcIdConversion};
use sp_core::{H160, H256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	ConsensusEngineId, Perbill,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub const MOCK_PAYMENT_ASSET_ID: AssetId = 100;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>},
		FeeControl: fee_control::{Pallet, Call, Storage, Event<T>},
		MockPallet: mock_pallet::pallet::{Pallet, Call},
		FeeProxy: pallet_fee_proxy::{Pallet, Call, Storage, Event<T>},
		Dex: pallet_dex::{Pallet, Call, Storage, Event<T>},
		AssetsExt: pallet_assets_ext::{Pallet, Call, Storage, Event<T>},
		Evm: pallet_evm::{Pallet, Call, Storage, Event<T>}
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

type BlockNumber = u64;

impl frame_system::Config for Test {
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

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ();
	type AccountStore = System;
	type MaxLocks = ();
	type WeightInfo = ();
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
pub type AssetsForceOrigin = EnsureRoot<AccountId>;

impl pallet_assets::Config for Test {
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
	pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const TransactionByteFee: Balance = 2_500;
	pub const OperationalFeeMultiplier: u8 = 1;
}

pub struct LengthToFeeZero;
impl WeightToFee for LengthToFeeZero {
	type Balance = Balance;

	fn weight_to_fee(weight: &Weight) -> Self::Balance {
		0
	}
}

impl pallet_transaction_payment::Config for Test {
	type OnChargeTransaction = FeeProxy;
	type Event = Event;
	type WeightToFee = FeeControl;
	type LengthToFee = LengthToFeeZero;
	type FeeMultiplierUpdate = ();
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
}

parameter_types! {
	/// Floor network base fee per gas
	/// 0.000015 XRP per gas, 15000 GWEI
	pub const DefaultEvmBaseFeePerGas: u64 = 15_000_000_000_000;
	pub const WeightToFeeReduction: Perbill = Perbill::from_parts(125);

}

impl crate::Config for Test {
	type Event = Event;
	type DefaultEvmBaseFeePerGas = DefaultEvmBaseFeePerGas;
	type WeightToFeeReduction = WeightToFeeReduction;
	type WeightInfo = ();
}

impl mock_pallet::pallet::Config for Test {}

parameter_types! {
	pub const XrpAssetId: AssetId = MOCK_PAYMENT_ASSET_ID;
}

pub type XrpCurrency = pallet_assets_ext::AssetCurrency<Test, XrpAssetId>;

impl pallet_fee_proxy::Config for Test {
	type Call = Call;
	type Event = Event;
	type PalletsOrigin = OriginCaller;
	type FeeAssetId = XrpAssetId;
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<XrpCurrency, ()>;
	type ErcIdConversion = Self;
}

// Mock ErcIdConversion for testing purposes
impl<RuntimeId> ErcIdConversion<RuntimeId> for Test
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
			return Some(RuntimeId::from(16000))
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
	pub const GetExchangeFee: (u32, u32) = (3, 1000); // 0.3% fee
	pub const TradingPathLimit: u32 = 3;
	pub const DEXPalletId: PalletId = PalletId(*b"mock/dex");
	pub const DEXBurnPalletId: PalletId = PalletId(*b"burnaddr");
	pub const LPTokenName: [u8; 10] = *b"Uniswap V2";
	pub const LPTokenSymbol: [u8; 6] = *b"UNI-V2";
	pub const LPTokenDecimals: u8 = 6;
}

impl pallet_dex::Config for Test {
	type Event = Event;
	type GetExchangeFee = GetExchangeFee;
	type TradingPathLimit = TradingPathLimit;
	type DEXPalletId = DEXPalletId;
	type DEXBurnPalletId = DEXBurnPalletId;
	type LPTokenName = LPTokenName;
	type LPTokenSymbol = LPTokenSymbol;
	type LPTokenDecimals = LPTokenDecimals;
	type WeightInfo = ();
	type MultiCurrency = AssetsExt;
}

parameter_types! {
	pub const NativeAssetId: AssetId = 1;
	pub const AssetsExtPalletId: PalletId = PalletId(*b"assetext");
	pub const MaxHolds: u32 = 16;
	pub const TestParachainId: u32 = 100;
}

impl pallet_assets_ext::Config for Test {
	type Event = Event;
	type ParachainId = TestParachainId;
	type MaxHolds = MaxHolds;
	type NativeAssetId = NativeAssetId;
	type OnNewAssetSubscription = ();
	type PalletId = AssetsExtPalletId;
	type WeightInfo = ();
}

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

pub struct MockBlockHashMapping<Test>(PhantomData<Test>);
impl<Test> BlockHashMapping for MockBlockHashMapping<Test> {
	fn block_hash(_number: u32) -> H256 {
		H256::default()
	}
}

impl pallet_evm::Config for Test {
	type FeeCalculator = FeeControl;
	type GasWeightMapping = ();
	type BlockHashMapping = MockBlockHashMapping<Test>;
	type CallOrigin = EnsureAddressNever<AccountId>;
	type WithdrawOrigin = EnsureAddressNever<AccountId>;
	type AddressMapping = MockAddressMapping;
	type Currency = Balances;
	type Event = Event;
	type Runner = pallet_evm::runner::stack::Runner<Self>;
	type PrecompilesType = ();
	type PrecompilesValue = ();
	type ChainId = ();
	type BlockGasLimit = ();
	type OnChargeTransaction = ();
	type FindAuthor = FindAuthorTruncated;
	type HandleTxValidation = ();
}

// Mock pallet for testing extrinsics with a specific weight
pub mod mock_pallet {
	#[frame_support::pallet]
	pub mod pallet {
		use frame_support::pallet_prelude::*;
		use frame_system::pallet_prelude::*;
		#[pallet::pallet]
		#[pallet::generate_store(pub(super) trait Store)]
		pub struct Pallet<T>(_);

		#[pallet::config]
		pub trait Config: frame_system::Config {}

		#[pallet::genesis_config]
		pub struct GenesisConfig<T: Config> {
			_marker: PhantomData<T>,
		}

		#[cfg(feature = "std")]
		impl<T: Config> Default for GenesisConfig<T> {
			fn default() -> Self {
				GenesisConfig { _marker: Default::default() }
			}
		}

		#[pallet::genesis_build]
		impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
			fn build(&self) {
				unimplemented!()
			}
		}

		// Some expected weight, given by a balances transfer
		pub const WEIGHT: Weight = 0;

		#[pallet::call]
		impl<T: Config> Pallet<T> {
			// For tests. Charge some expected fee amount
			#[pallet::weight(WEIGHT)]
			pub fn mock_charge_fee(_origin: OriginFor<T>) -> DispatchResult {
				Ok(())
			}
		}
	}
}

#[derive(Default)]
pub struct TestExt;

impl TestExt {
	pub fn build(self) -> sp_io::TestExternalities {
		let storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
		let mut ext: sp_io::TestExternalities = storage.into();
		ext.execute_with(|| System::initialize(&1, &[0u8; 32].into(), &Default::default()));
		ext
	}
}
