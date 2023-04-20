/* Copyright 2019-2021 Centrality Investments Limited
 *
 * Licensed under the LGPL, Version 3.0 (the "License");
 * you may not use this file except in compliance with the License.
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * You may obtain a copy of the License at the root of this project source code,
 * or at:
 *     https://centrality.ai/licenses/gplv3.txt
 *     https://centrality.ai/licenses/lgplv3.txt
 */
use crate::{self as pallet_futurepass, *};
use frame_support::{
	parameter_types,
	traits::{Currency, ExistenceRequirement, FindAuthor, InstanceFilter},
	weights::WeightToFee,
	PalletId,
};
use frame_system::{limits, EnsureRoot};
use pallet_evm::{AddressMapping, BlockHashMapping, EnsureAddressNever, FeeCalculator};
use precompile_utils::{constants::FUTUREPASS_PRECOMPILE_ADDRESS_PREFIX, Address, ErcIdConversion};
use seed_pallet_common::*;
use seed_primitives::types::{AccountId, AssetId, Balance};
use seed_runtime::{
	constants::currency::deposit,
	impls::{ProxyPalletProvider, ProxyType},
	AnnouncementDepositBase, AnnouncementDepositFactor, ProxyDepositBase, ProxyDepositFactor,
};
use sp_core::{H160, H256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	ConsensusEngineId,
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
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		FeeControl: pallet_fee_control,
		// TransactionPayment: pallet_transaction_payment,
		// FeeProxy: pallet_fee_proxy,
		Dex: pallet_dex,
		// Evm: pallet_evm,
		Proxy: pallet_proxy,
		Futurepass: pallet_futurepass,
	}
);

impl_frame_system_config!(Test);
impl_pallet_balance_config!(Test);
impl_pallet_assets_config!(Test);
impl_pallet_assets_ext_config!(Test);
impl_pallet_fee_control_config!(Test);
// impl_pallet_transaction_payment_config!(Test);
// impl_pallet_fee_proxy_config!(Test);
impl_pallet_dex_config!(Test);
// impl_pallet_timestamp_config!(Test); // required for pallet-evm
// impl_pallet_evm_config!(Test);

impl InstanceFilter<Call> for ProxyType {
	fn filter(&self, c: &Call) -> bool {
		if matches!(c, Call::Proxy(..) | Call::Futurepass(..)) {
			return false
		}
		match self {
			_ => true,
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			_ => false,
		}
	}
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

impl pallet_futurepass::ProxyProvider<Test> for ProxyPalletProvider {
	fn exists(futurepass: &AccountId, delegate: &AccountId, proxy_type: Option<ProxyType>) -> bool {
		pallet_proxy::Pallet::<Test>::find_proxy(futurepass, delegate, proxy_type)
			.map(|_| true)
			.unwrap_or(false)
	}

	fn delegates(futurepass: &AccountId) -> Vec<(AccountId, ProxyType)> {
		let (proxy_definitions, _) = pallet_proxy::Proxies::<Test>::get(futurepass);
		proxy_definitions
			.into_iter()
			.map(|proxy_def| (proxy_def.delegate, proxy_def.proxy_type))
			.collect()
	}

	/// Adding a delegate requires funding the futurepass account (from funder) with the cost of the
	/// proxy creation.
	/// The futurepass cannot pay for itself as it may not have any funds.
	fn add_delegate(
		funder: &AccountId,
		futurepass: &AccountId,
		delegate: &AccountId,
		proxy_type: &ProxyType,
	) -> DispatchResult {
		// pay cost for proxy creation; transfer funds/deposit from delegator to FP account (which
		// executes proxy creation)
		let (proxy_definitions, _) = pallet_proxy::Proxies::<Test>::get(futurepass);
		// get proxy_definitions length + 1 (cost of upcoming insertion); cost to reserve
		let creation_cost =
			pallet_proxy::Pallet::<Test>::deposit(proxy_definitions.len() as u32 + 1);
		<pallet_balances::Pallet<Test> as Currency<_>>::transfer(
			funder,
			futurepass,
			creation_cost,
			ExistenceRequirement::KeepAlive,
		)?;

		pallet_proxy::Pallet::<Test>::add_proxy_delegate(futurepass, *delegate, *proxy_type, 0)
	}

	/// Removing a delegate requires refunding the potential funder (who may have funded the
	/// creation of futurepass or added the delegates) with the cost of the proxy creation.
	/// The futurepass accrues deposits (as reserved balance) by the funder(s) when delegates are
	/// added to the futurepass account.
	/// Removing delegates unreserves the deposits (funds) from the futurepass account - which
	/// should be paid back out to potential receiver(s).
	fn remove_delegate(
		receiver: &AccountId,
		futurepass: &AccountId,
		delegate: &AccountId,
	) -> DispatchResult {
		let proxy_def = pallet_proxy::Pallet::<Test>::find_proxy(futurepass, delegate, None)?;
		// get deposits before proxy removal (value gets mutated in removal)
		let (_, pre_removal_deposit) = pallet_proxy::Proxies::<Test>::get(futurepass);

		let result = pallet_proxy::Pallet::<Test>::remove_proxy_delegate(
			futurepass,
			*delegate,
			proxy_def.proxy_type,
			0,
		);
		if result.is_ok() {
			let (_, post_removal_deposit) = pallet_proxy::Proxies::<Test>::get(futurepass);
			let removal_refund = pre_removal_deposit - post_removal_deposit;
			<pallet_balances::Pallet<Test> as Currency<_>>::transfer(
				futurepass,
				receiver,
				removal_refund,
				ExistenceRequirement::KeepAlive,
			)?;
		}
		result
	}

	fn proxy_call(
		caller: <Test as frame_system::Config>::Origin,
		futurepass: AccountId,
		call: Call,
	) -> DispatchResult {
		let call = pallet_proxy::Call::<Test>::proxy {
			real: futurepass.into(),
			force_proxy_type: None,
			call: call.into(),
		};

		<Call as Dispatchable>::dispatch(call.into(), caller)
			.map(|_| ())
			.map_err(|e| e.error)
	}
}

parameter_types! {
	/// 4 byte futurepass account prefix
	pub const FuturepassPrefix: [u8; 4] = [0xFF; 4];
}

impl crate::Config for Test {
	type Event = Event;
	type FuturepassPrefix = FuturepassPrefix;
	type Proxy = ProxyPalletProvider;
	type Call = Call;
	type ApproveOrigin = EnsureRoot<AccountId>;
	type ProxyType = ProxyType;
	type WeightInfo = ();
}

pub fn create_account(seed: u64) -> AccountId {
	AccountId::from(H160::from_low_u64_be(seed))
}

#[derive(Default)]
// #[derive(Clone, Copy, Default)]
pub struct TestExt;

impl TestExt {
	pub fn build(self) -> sp_io::TestExternalities {
		let storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
		let mut ext: sp_io::TestExternalities = storage.into();
		ext.execute_with(|| System::initialize(&1, &[0u8; 32].into(), &Default::default()));
		ext
	}
}
