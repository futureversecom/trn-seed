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

use crate as pallet_vortex;
use frame_support::{
	parameter_types,
	traits::{ConstU32, GenesisBuild, Hooks},
	PalletId,
};
use frame_system::EnsureRoot;
// use pallet_evm::{AddressMapping, BlockHashMapping, EnsureAddressNever};
use seed_pallet_common::*;
use seed_primitives::{AccountId, AssetId, Balance};
use sp_core::{H160, H256};
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{BlakeTwo256, IdentityLookup},
};

pub type Extrinsic = TestXt<RuntimeCall, ()>;
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub const XRP_ASSET_ID: AssetId = 2;
pub const MILLISECS_PER_BLOCK: u64 = 4_000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);

pub fn create_account(seed: u64) -> AccountId {
	AccountId::from(H160::from_low_u64_be(seed))
}

pub fn to_eth(amount: u128) -> u128 {
	amount * 1_000_000_000_000_000_000_u128
}

pub const BLOCK_TIME: u64 = 1000;
pub fn run_to_block(n: u64) {
	while System::block_number() < n {
		System::set_block_number(System::block_number() + 1);
		Vortex::on_initialize(System::block_number());
		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME);
	}
}

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		Timestamp: pallet_timestamp,
		Vortex: pallet_vortex,
		Staking: pallet_staking,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);
impl_pallet_timestamp_config!(Test);

pallet_staking_reward_curve::build! {
	const I_NPOS: sp_runtime::curve::PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000,
		max_inflation: 0_100_000,
		ideal_stake: 0_500_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

parameter_types! {
	pub const RewardCurve: &'static sp_runtime::curve::PiecewiseLinear<'static> = &I_NPOS;
	pub static BondingDuration: u32 = 3;
}

impl pallet_staking::Config for Test {
	type MaxNominations = ConstU32<16>;
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type UnixTime = pallet_timestamp::Pallet<Self>;
	type CurrencyToVote = frame_support::traits::SaturatingCurrencyToVote;
	type RewardRemainder = ();
	type RuntimeEvent = RuntimeEvent;
	type Slash = ();
	type Reward = ();
	type SessionsPerEra = ();
	type SlashDeferDuration = ();
	type SlashCancelOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type BondingDuration = BondingDuration;
	type SessionInterface = ();
	type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
	type NextNewSession = ();
	type MaxNominatorRewardedPerValidator = ConstU32<64>;
	type OffendingValidatorsThreshold = ();
	type ElectionProvider =
		frame_election_provider_support::NoElection<(AccountId, BlockNumber, Staking)>;
	type GenesisElectionProvider = Self::ElectionProvider;
	// type VoterList = pallet_bags_list::Pallet<Self>;
	type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Self>;
	type MaxUnlockingChunks = ConstU32<32>;
	// type OnStakerSlash = Pools;
	type HistoryDepth = HistoryDepth;
	type TargetList = pallet_staking::UseValidatorsMap<Test>;
	type OnStakerSlash = ();
	type BenchmarkingConfig = pallet_staking::TestBenchmarkingConfig;
	type WeightInfo = ();
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	RuntimeCall: From<C>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = Extrinsic;
}

parameter_types! {
	pub const VtxVortexPotId: PalletId = PalletId(*b"vtx/vpot");
	pub const VtxRootPotId: PalletId = PalletId(*b"vtx/rpot");
	pub const TxFeePotId: PalletId = PalletId(*b"txfeepot");
	pub const UnsignedInterval: BlockNumber =  MINUTES / 2;
	pub const PayoutBatchSize: u32 =  799;
	pub const HistoryDepth: u32 = 84;
	pub const VortexAssetId: AssetId = 2;
	pub const RootAssetId: AssetId = 1;
	pub const XrpAssetId: seed_primitives::AssetId = XRP_ASSET_ID;
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type NativeAssetId = RootAssetId;
	type VtxAssetId = VortexAssetId;
	type VtxDistPotId = VtxVortexPotId;
	type RootPotId = VtxRootPotId;
	type TxFeePotId = TxFeePotId;
	type UnsignedInterval = UnsignedInterval;
	type PayoutBatchSize = PayoutBatchSize;
	type VtxDistIdentifier = u32;
	type VtxDistAdminOrigin = EnsureRoot<AccountId>;
	type MultiCurrency = AssetsExt;
	type HistoryDepth = HistoryDepth;
	type MaxAssetPrices = ConstU32<1000>;
	type MaxRewards = ConstU32<10_000>;
	type MaxStringLength = ConstU32<1000>;
}

#[derive(Default)]
struct AssetsFixture {
	pub id: AssetId,
	pub symbol: Vec<u8>,
	pub endowments: Vec<(AccountId, Balance)>,
}

impl AssetsFixture {
	fn new(id: AssetId, symbol: &[u8], endowments: &[(AccountId, Balance)]) -> Self {
		Self { id, symbol: symbol.to_vec(), endowments: endowments.to_vec() }
	}
}
#[derive(Default)]
pub struct TestExt {
	assets: Vec<AssetsFixture>,
	balances: Vec<(AccountId, Balance)>,
}

impl TestExt {
	/// Configure an asset with id, name and some endowments
	/// total supply = sum(endowments)
	pub fn with_asset(
		mut self,
		id: AssetId,
		name: &str,
		endowments: &[(AccountId, Balance)],
	) -> Self {
		self.assets.push(AssetsFixture::new(id, name.as_bytes(), endowments));
		self
	}
	/// Configure some native token balances
	pub fn with_balances(mut self, balances: &[(AccountId, Balance)]) -> Self {
		self.balances = balances.to_vec();
		self
	}

	pub fn benchmark() -> Self {
		let alice: AccountId = create_account(1);
		Self::default()
			.with_balances(&[(alice, 1_000_000)])
			.with_asset(
				<Test as crate::Config>::NativeAssetId::get(),
				"ROOT",
				&[(alice, 1_000_000)],
			)
			.with_asset(<Test as crate::Config>::VtxAssetId::get(), "VORTEX", &[(alice, 0)])
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut ext = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		if !self.assets.is_empty() {
			let mut metadata = Vec::with_capacity(self.assets.len());
			let mut assets = Vec::with_capacity(self.assets.len());
			let mut accounts = Vec::<(AssetId, AccountId, Balance)>::default();

			let default_owner = create_account(1);
			for AssetsFixture { id, symbol, endowments } in self.assets {
				assets.push((id, default_owner, true, 1));
				metadata.push((id, symbol.clone(), symbol, 6));
				for (payee, balance) in endowments {
					accounts.push((id, payee, balance));
				}
			}

			pallet_assets::GenesisConfig::<Test> { assets, metadata, accounts }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

		if !self.balances.is_empty() {
			pallet_balances::GenesisConfig::<Test> { balances: self.balances }
				.assimilate_storage(&mut ext)
				.unwrap();
		}

		let mut ext: sp_io::TestExternalities = ext.into();
		ext.execute_with(|| {
			System::initialize(&1, &[0u8; 32].into(), &Default::default());
		});

		ext
	}
}
