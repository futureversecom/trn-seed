// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

#[macro_export]
macro_rules! construct_test_runtime {
	(
		{
			$($module_name:ident: $module_path:ident $(,)?)*
		}
	) => {
		frame_support::construct_runtime!(
			pub enum Test where
				Block = Block,
				NodeBlock = Block,
				UncheckedExtrinsic = UncheckedExtrinsic,
			{
				$($module_name: $module_path),*
			}
		);

	};
}

#[macro_export]
macro_rules! impl_frame_system_config {
	($test:ident) => {
		parameter_types! {
			pub const BlockHashCount: u64 = 250;
		}

		type BlockNumber = u64;

		impl frame_system::Config for $test {
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
	};
}

#[macro_export]
macro_rules! impl_pallet_balance_config {
	($test:ident) => {
		parameter_types! {
			pub const MaxReserves: u32 = 50;
		}

		impl pallet_balances::Config for $test {
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
		}

		impl pallet_assets::Config for $test {
			type Event = Event;
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
			type Event = Event;
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
macro_rules! impl_pallet_fee_control_config {
	($test:ident) => {
		impl pallet_fee_control::Config for $test {
			type Event = Event;
			type WeightInfo = ();
			type DefaultValues = ();
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

		impl pallet_evm::Config for $test {
			type FeeCalculator = FeeControl;
			type GasWeightMapping = ();
			type BlockHashMapping = MockBlockHashMapping<$test>;
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
	};
}

#[macro_export]
macro_rules! impl_pallet_dex_config {
	($test:ident) => {
		parameter_types! {
			pub const GetExchangeFee: (u32, u32) = (3, 1000); // 0.3% fee
			pub const TradingPathLimit: u32 = 3;
			pub const DEXPalletId: PalletId = PalletId(*b"mock/dex");
			pub const DEXBurnPalletId: PalletId = PalletId(*b"burnaddr");
			pub const LPTokenName: [u8; 10] = *b"Uniswap V2";
			pub const LPTokenSymbol: [u8; 6] = *b"UNI-V2";
			pub const LPTokenDecimals: u8 = 6;
		}

		impl pallet_dex::Config for $test {
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
			pub const XrpAssetId: AssetId = MOCK_PAYMENT_ASSET_ID;
		}

		pub type XrpCurrency = pallet_assets_ext::AssetCurrency<Test, XrpAssetId>;

		impl pallet_fee_proxy::Config for $test {
			type Call = Call;
			type Event = Event;
			type PalletsOrigin = OriginCaller;
			type FeeAssetId = XrpAssetId;
			type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<XrpCurrency, ()>;
			type ErcIdConversion = Self;
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
			type Event = Event;
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

		impl<T: pallet_futurepass::Config> pallet_futurepass::ProxyProvider<T>
			for MockProxyProvider
		{
			fn exists(futurepass: &T::AccountId, delegate: &T::AccountId) -> bool {
				false
			}
			fn delegates(futurepass: &T::AccountId) -> Vec<T::AccountId> {
				vec![]
			}
			fn add_delegate(
				_: &T::AccountId,
				futurepass: &T::AccountId,
				delegate: &T::AccountId,
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
			fn proxy_call(
				caller: T::Origin,
				futurepass: T::AccountId,
				call: <T as pallet_futurepass::Config>::Call,
			) -> DispatchResult {
				Ok(())
			}
		}

		parameter_types! {
			/// 4 byte futurepass account prefix
			pub const FuturepassPrefix: [u8; 4] = [0xFF; 4];
		}

		impl pallet_futurepass::Config for $test {
			type Event = Event;
			type FuturepassPrefix = FuturepassPrefix;
			type Proxy = MockProxyProvider;
			type Call = Call;
			type ApproveOrigin = EnsureRoot<AccountId>;
			type WeightInfo = ();
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
			pub ProxyDepositBase: Balance = deposit(1, 8); // TODO - set 0 for futurepass
			// Additional storage item size of 21 bytes (20 bytes AccountId + 1 byte sizeof(ProxyType)).
			pub ProxyDepositFactor: Balance = deposit(0, 21); // TODO - set 0 for futurepass
			pub AnnouncementDepositBase: Balance = deposit(1, 8);
			// Additional storage item size of 56 bytes:
			// - 20 bytes AccountId
			// - 32 bytes Hasher (Blake2256)
			// - 4 bytes BlockNumber (u32)
			pub AnnouncementDepositFactor: Balance = deposit(0, 56);
		}

		impl pallet_proxy::Config for Test {
			type Event = Event;
			type Call = Call;
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
