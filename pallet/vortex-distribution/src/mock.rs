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

use crate as pallet_vortex_distribution;
use crate::PRECISION_MULTIPLIER;
use frame_support::traits::{ConstU32, Hooks};
use seed_pallet_common::test_prelude::*;
use seed_pallet_common::AttributionProvider;
use sp_runtime::{testing::TestXt, BuildStorage};
use sp_runtime::{
	traits::{Saturating, Zero},
	Permill,
};
use sp_staking::currency_to_vote::SaturatingCurrencyToVote;
use std::cell::RefCell;
use std::ops::Div;

pub type Extrinsic = TestXt<RuntimeCall, ()>;
pub const MILLISECS_PER_BLOCK: u64 = 4_000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);

pub const BLOCK_TIME: u64 = 1000;
pub fn run_to_block(n: u64) {
	while System::block_number() < n {
		System::set_block_number(System::block_number() + 1);
		Vortex::on_initialize(System::block_number());
		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME);
	}
}

pub fn calculate_vtx_price(
	assets: &Vec<(AssetId, Balance, u8)>,
	prices: &Vec<(AssetId, Balance)>,
	vtx_total_supply: Balance,
) -> Balance {
	let mut asset_value_usd = 0_u128;
	for i in 0..assets.len() {
		let decimal_factor: Balance = 10u128.pow(assets[i].2 as u32).into();
		asset_value_usd += assets[i].1.saturating_mul(prices[i].1).div(decimal_factor);
	}

	let vtx_decimal_factor: Balance = 10u128.pow(6).into(); // VTX 6 decimal points
	let vtx_total_supply = vtx_total_supply.div(vtx_decimal_factor);
	let vtx_price = if vtx_total_supply == Zero::zero() {
		1u128.into()
	} else {
		asset_value_usd / vtx_total_supply
	};
	vtx_price
}

pub fn calculate_vtx(
	assets: &Vec<(AssetId, Balance, u8)>,
	prices: &Vec<(AssetId, Balance)>,
	bootstrap_root: Balance,
	root_price: Balance,
	vtx_price: Balance,
) -> (Balance, Balance, Balance) {
	let mut fee_vault_asset_value = 0_u128;
	for i in 0..assets.len() {
		let decimal_factor: Balance = 10u128.pow(assets[i].2 as u32).into();
		fee_vault_asset_value += assets[i].1.saturating_mul(prices[i].1).div(decimal_factor);
	}
	let root_decimal_factor: Balance = 10u128.pow(6).into();
	let bootstrap_asset_value = bootstrap_root.saturating_mul(root_price).div(root_decimal_factor);

	// calculate in drops for higher precision
	let vtx_decimal_factor: Balance = 10u128.pow(6).into();
	let total_vortex_network_reward = fee_vault_asset_value
		.saturating_mul(vtx_decimal_factor)
		.saturating_mul(PRECISION_MULTIPLIER)
		.div(vtx_price);
	let total_vortex_bootstrap = bootstrap_asset_value
		.saturating_mul(vtx_decimal_factor)
		.saturating_mul(PRECISION_MULTIPLIER)
		.div(vtx_price);
	let total_vortex = total_vortex_network_reward.saturating_add(total_vortex_bootstrap);

	(total_vortex_network_reward, total_vortex_bootstrap, total_vortex)
}

pub fn calculate_vtx_redeem(
	redeem_asset_list: &Vec<(AssetId, Balance)>,
	redeem_vtx_amount: Balance,
	total_vortex: Balance,
) -> Vec<(AssetId, Balance)> {
	let mut redeem = vec![];
	for (asset_id, asset_balance) in redeem_asset_list.into_iter() {
		// First, we calculate the ratio between the asset balance and the total vortex
		// issued. then multiply it with the vortex token amount the user wants to redeem to
		// get the resulting asset token amount.
		let redeem_amount = redeem_vtx_amount.saturating_mul(*asset_balance) / total_vortex;

		redeem.push((*asset_id, redeem_amount));
	}

	redeem
}

/// Calculate partner attribution rewards for testing
/// This mirrors the logic in the pallet's do_calculate_partner_attribution_rewards function
pub fn calculate_attribution_rewards(
	attributions: &[(AccountId, Balance, Option<Permill>)],
	xrp_price: Balance,
	vtx_price: Balance,
	total_network_reward: Balance,
) -> Vec<(AccountId, Balance)> {
	let fee_vault_asset_value = total_network_reward
		.saturating_mul(vtx_price)
		.saturating_div(PRECISION_MULTIPLIER); // in drops with price multiplier

	let mut partner_attribution_rewards = Vec::new();

	for (account, amount, fee_percentage) in attributions {
		// Skip attributions without fee percentage
		if fee_percentage.is_none() {
			continue;
		}

		let attribution_fee_value_usd = amount.saturating_mul(xrp_price);
		// Note - calculating this way to get optimal precision
		let vtx_attribution_reward = (fee_percentage.unwrap()
			* attribution_fee_value_usd.saturating_mul(total_network_reward))
		.div(fee_vault_asset_value);
		partner_attribution_rewards.push((*account, vtx_attribution_reward));
	}

	partner_attribution_rewards
}
construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		Timestamp: pallet_timestamp,
		Vortex: pallet_vortex_distribution,
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
	type CurrencyToVote = SaturatingCurrencyToVote;
	type RewardRemainder = ();
	type RuntimeEvent = RuntimeEvent;
	type Slash = ();
	type Reward = ();
	type SessionsPerEra = ();
	type SlashDeferDuration = ();
	type BondingDuration = BondingDuration;
	type SessionInterface = ();
	type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
	type NextNewSession = ();
	type MaxNominatorRewardedPerValidator = ConstU32<64>;
	type OffendingValidatorsThreshold = ();
	type ElectionProvider = frame_election_provider_support::NoElection<(
		AccountId,
		BlockNumber,
		Staking,
		ConstU32<10>,
	)>;
	type GenesisElectionProvider = Self::ElectionProvider;
	// type VoterList = pallet_bags_list::Pallet<Self>;
	type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Self>;
	type MaxUnlockingChunks = ConstU32<32>;
	// type OnStakerSlash = Pools;
	type HistoryDepth = HistoryDepth;
	type TargetList = pallet_staking::UseValidatorsMap<Test>;
	type BenchmarkingConfig = pallet_staking::TestBenchmarkingConfig;
	type WeightInfo = ();
	type AdminOrigin = EnsureRoot<AccountId>;
	type EventListeners = ();
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	RuntimeCall: From<C>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = Extrinsic;
}

parameter_types! {
	pub const VtxHeldPotId: PalletId = PalletId(*b"vtx/hpot");
	pub const VtxVortexPotId: PalletId = PalletId(*b"vtx/vpot");
	pub const VtxRootPotId: PalletId = PalletId(*b"vtx/rpot");
	pub const TxFeePotId: PalletId = PalletId(*b"txfeepot");
	pub const UnsignedInterval: BlockNumber =  MINUTES / 2;
	pub const PayoutBatchSize: u32 =  799;
	pub const HistoryDepth: u32 = 84;
	pub const VortexAssetId: AssetId = 3;
	pub const RootAssetId: AssetId = 1;
	pub const XrpAssetId: seed_primitives::AssetId = XRP_ASSET_ID;
}

/// Thread local storage for test attributions.
thread_local! {
	static TEST_ATTRIBUTIONS: RefCell<Vec<(AccountId, Balance, Option<Permill>)>> = RefCell::new(Vec::new());
}

/// Mock implementation of AttributionProvider for testing
pub struct MockPartnerAttribution;
impl MockPartnerAttribution {
	/// Set the test attributions for the mock
	pub fn set_test_attributions(attributions: Vec<(AccountId, Balance, Option<Permill>)>) {
		TEST_ATTRIBUTIONS.with(|cell| {
			*cell.borrow_mut() = attributions;
		});
	}

	/// Clear all test attributions
	pub fn clear_test_attributions() {
		TEST_ATTRIBUTIONS.with(|cell| {
			cell.borrow_mut().clear();
		});
	}

	/// Get current test attributions (for debugging)
	pub fn get_current_attributions() -> Vec<(AccountId, Balance, Option<Permill>)> {
		TEST_ATTRIBUTIONS.with(|cell| cell.borrow().clone())
	}
}

impl AttributionProvider<AccountId> for MockPartnerAttribution {
	fn get_attributions() -> Vec<(AccountId, Balance, Option<Permill>)> {
		// Return mock attribution data for testing
		TEST_ATTRIBUTIONS.with(|cell| cell.borrow().clone())
	}

	fn reset_balances() {
		// Mock implementation - clear the test attributions
		TEST_ATTRIBUTIONS.with(|cell| {
			cell.borrow_mut().clear();
		});
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn set_attributions(attributions: Vec<(AccountId, Balance, Option<Permill>)>) {
		TEST_ATTRIBUTIONS.with(|cell| {
			*cell.borrow_mut() = attributions;
		});
	}
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type NativeAssetId = RootAssetId;
	type VtxAssetId = VortexAssetId;
	type VtxHeldPotId = VtxHeldPotId;
	type VtxDistPotId = VtxVortexPotId;
	type RootPotId = VtxRootPotId;
	type TxFeePotId = TxFeePotId;
	type UnsignedInterval = UnsignedInterval;
	type PayoutBatchSize = PayoutBatchSize;
	type VtxDistIdentifier = u32;
	type MultiCurrency = AssetsExt;
	type HistoryDepth = HistoryDepth;
	type MaxAssetPrices = ConstU32<1000>;
	type MaxRewards = ConstU32<3_100>;
	type MaxStringLength = ConstU32<1000>;
	type PartnerAttributionProvider = MockPartnerAttribution;
	type GasAssetId = XrpAssetId;
	type MaxAttributionPartners = ConstU32<200>;
}

#[derive(Default)]
struct AssetsFixture {
	pub id: AssetId,
	pub symbol: Vec<u8>,
	pub decimals: u8,
	pub endowments: Vec<(AccountId, Balance)>,
}

impl AssetsFixture {
	fn new(id: AssetId, symbol: &[u8], decimals: u8, endowments: &[(AccountId, Balance)]) -> Self {
		Self { id, symbol: symbol.to_vec(), decimals, endowments: endowments.to_vec() }
	}
}
#[derive(Default)]
pub struct TestExt {
	assets: Vec<AssetsFixture>,
	balances: Vec<(AccountId, Balance)>,
	attributions: Vec<(AccountId, Balance, Option<Permill>)>,
}

impl TestExt {
	/// Configure an asset with id, name, decimals and some endowments
	/// total supply = sum(endowments)
	pub fn with_asset_decimals(
		mut self,
		id: AssetId,
		name: &str,
		decimals: u8,
		endowments: &[(AccountId, Balance)],
	) -> Self {
		self.assets.push(AssetsFixture::new(id, name.as_bytes(), decimals, endowments));
		self
	}
	/// with decimals defaulted to 6
	pub fn with_asset(
		mut self,
		id: AssetId,
		name: &str,
		endowments: &[(AccountId, Balance)],
	) -> Self {
		self.assets.push(AssetsFixture::new(id, name.as_bytes(), 6, endowments));
		self
	}
	/// Configure some native token balances
	pub fn with_balances(mut self, balances: &[(AccountId, Balance)]) -> Self {
		self.balances = balances.to_vec();
		self
	}
	/// Configure some attributions
	pub fn with_attributions(
		mut self,
		attributions: &[(AccountId, Balance, Option<Permill>)],
	) -> Self {
		self.attributions = attributions.to_vec();
		self
	}

	#[allow(dead_code)]
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
		let mut ext = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

		if !self.assets.is_empty() {
			let mut metadata = Vec::with_capacity(self.assets.len());
			let mut assets = Vec::with_capacity(self.assets.len());
			let mut accounts = Vec::<(AssetId, AccountId, Balance)>::default();

			let default_owner = create_account(1);
			for AssetsFixture { id, symbol, decimals, endowments } in self.assets {
				assets.push((id, default_owner, true, 1));
				metadata.push((id, symbol.clone(), symbol, decimals));
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

		// Clear existing attributions first to ensure clean state
		MockPartnerAttribution::clear_test_attributions();
		// Set new attributions
		MockPartnerAttribution::set_test_attributions(self.attributions);

		let mut ext: sp_io::TestExternalities = ext.into();
		ext.execute_with(|| {
			System::initialize(&1, &[0u8; 32].into(), &Default::default());
		});

		ext
	}
}
