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

use crate::{self as pallet_futurepass, *};
use frame_support::traits::{
	fungibles::{Inspect, Transfer},
	Currency, ExistenceRequirement, InstanceFilter, ReservableCurrency,
};
use seed_pallet_common::test_prelude::*;
use seed_runtime::{
	impls::{ProxyPalletProvider, ProxyType},
	AnnouncementDepositBase, AnnouncementDepositFactor, ProxyDepositBase, ProxyDepositFactor,
};
use sp_core::{ecdsa, Pair};
use sp_runtime::traits::{BlakeTwo256, IdentityLookup};

pub const MOCK_PAYMENT_ASSET_ID: AssetId = 100;
pub const MOCK_NATIVE_ASSET_ID: AssetId = ROOT_ASSET_ID;

construct_runtime!(
	pub enum Test where
		Block = Block<Test>,
		NodeBlock = Block<Test>,
		UncheckedExtrinsic = UncheckedExtrinsic<Test>,
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		AssetsExt: pallet_assets_ext,
		Nft: pallet_nft,
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
impl_pallet_nft_config!(Test);
impl_pallet_assets_ext_config!(Test);
impl_pallet_fee_control_config!(Test);
impl_pallet_dex_config!(Test);

impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		if matches!(c, RuntimeCall::Proxy(..) | RuntimeCall::Futurepass(..)) {
			// Whitelist currently includes
			// pallet_futurepass::Call::register_delegate_with_signature,
			// pallet_futurepass::Call::unregister_delegate
			if !matches!(
				c,
				RuntimeCall::Futurepass(
					pallet_futurepass::Call::register_delegate_with_signature { .. }
				) | RuntimeCall::Futurepass(pallet_futurepass::Call::unregister_delegate { .. }) |
					RuntimeCall::Futurepass(pallet_futurepass::Call::transfer_futurepass { .. })
			) {
				return false
			}
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

impl pallet_futurepass::ProxyProvider<Test> for ProxyPalletProvider {
	fn exists(futurepass: &AccountId, delegate: &AccountId, proxy_type: Option<ProxyType>) -> bool {
		pallet_proxy::Pallet::<Test>::find_proxy(futurepass, delegate, proxy_type).is_ok()
	}

	fn owner(futurepass: &AccountId) -> Option<AccountId> {
		let (proxy_definitions, _) = pallet_proxy::Proxies::<Test>::get(futurepass);
		proxy_definitions
			.into_iter()
			.map(|proxy_def| (proxy_def.delegate, proxy_def.proxy_type))
			.filter(|(_, proxy_type)| proxy_type == &ProxyType::Owner)
			.map(|(owner, _)| owner)
			.next()
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
		proxy_type: &u8,
	) -> DispatchResult {
		// pay cost for proxy creation; transfer funds/deposit from delegator to FP account (which
		// executes proxy creation)
		let (proxy_definitions, reserve_amount) = pallet_proxy::Proxies::<Test>::get(futurepass);
		// get proxy_definitions length + 1 (cost of upcoming insertion); cost to reserve
		let new_reserve = pallet_proxy::Pallet::<Test>::deposit(proxy_definitions.len() as u32 + 1);
		let extra_reserve_required = new_reserve - reserve_amount;
		<pallet_balances::Pallet<Test> as Currency<_>>::transfer(
			funder,
			futurepass,
			extra_reserve_required,
			ExistenceRequirement::KeepAlive,
		)?;
		let proxy_type = ProxyType::try_from(*proxy_type)?;

		pallet_proxy::Pallet::<Test>::add_proxy_delegate(futurepass, *delegate, proxy_type, 0)
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

	fn remove_account(receiver: &AccountId, futurepass: &AccountId) -> DispatchResult {
		let (_, old_deposit) = pallet_proxy::Proxies::<Test>::take(futurepass);
		<pallet_balances::Pallet<Test> as ReservableCurrency<_>>::unreserve(
			futurepass,
			old_deposit,
		);
		<pallet_balances::Pallet<Test> as Currency<_>>::transfer(
			futurepass,
			receiver,
			old_deposit,
			ExistenceRequirement::AllowDeath,
		)?;
		Ok(())
	}

	fn proxy_call(
		caller: <Test as frame_system::Config>::RuntimeOrigin,
		futurepass: AccountId,
		call: RuntimeCall,
	) -> DispatchResult {
		let call = pallet_proxy::Call::<Test>::proxy {
			real: futurepass.into(),
			force_proxy_type: None,
			call: call.into(),
		};

		RuntimeCall::dispatch(call.into(), caller).map_err(|e| e.error)?;
		Ok(())
	}
}

parameter_types! {
	/// 4 byte futurepass account prefix
	pub const FuturepassPrefix: [u8; 4] = [0xFF; 4];
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Proxy = ProxyPalletProvider;
	type RuntimeCall = RuntimeCall;
	type ApproveOrigin = EnsureRoot<AccountId>;
	type ProxyType = ProxyType;
	type WeightInfo = ();

	type FuturepassMigrator = MockMigrationProvider;
	#[cfg(feature = "runtime-benchmarks")]
	type MultiCurrency = pallet_assets_ext::Pallet<Test>;
}

pub struct MockMigrationProvider;

impl<T: pallet_nft::Config + pallet_assets_ext::Config> crate::FuturepassMigrator<T>
	for MockMigrationProvider
where
	<T as frame_system::Config>::AccountId: From<H160>,
{
	fn transfer_asset(
		asset_id: AssetId,
		current_owner: &T::AccountId,
		new_owner: &T::AccountId,
	) -> DispatchResult {
		let amount = <pallet_assets_ext::Pallet<T> as Inspect<
			<T as frame_system::Config>::AccountId,
		>>::reducible_balance(asset_id, current_owner, false);
		<pallet_assets_ext::Pallet<T> as Transfer<<T as frame_system::Config>::AccountId>>::transfer(
			asset_id,
			current_owner,
			new_owner,
			amount,
			false,
		)?;
		Ok(())
	}

	fn transfer_nfts(
		collection_id: u32,
		current_owner: &T::AccountId,
		new_owner: &T::AccountId,
	) -> DispatchResult {
		let collection_info = pallet_nft::CollectionInfo::<T>::get(collection_id)
			.ok_or(pallet_nft::Error::<T>::NoCollectionFound)?;
		let serials = collection_info
			.owned_tokens
			.into_iter()
			.filter(|ownership| ownership.owner == *current_owner)
			.flat_map(|ownership| ownership.owned_serials)
			.collect::<Vec<_>>();
		let serials_bounded: BoundedVec<_, <T as pallet_nft::Config>::MaxTokensPerCollection> =
			BoundedVec::try_from(serials)
				.map_err(|_| pallet_nft::Error::<T>::TokenLimitExceeded)?;

		pallet_nft::Pallet::<T>::do_transfer(
			collection_id,
			serials_bounded,
			current_owner,
			new_owner,
		)?;
		Ok(())
	}
}

// pub fn random_account() -> AccountId {
// 	AccountId::from(H160::random())
// }

pub fn create_random_pair() -> (ecdsa::Pair, AccountId) {
	let (pair, _) = ecdsa::Pair::generate();
	let account: AccountId = pair.public().try_into().unwrap();
	(pair, account)
}

#[derive(Default)]
pub struct TestExt {
	balances: Vec<(AccountId, Balance)>,
	xrp_balances: Vec<(AssetId, AccountId, Balance)>,
	block_number: BlockNumber,
}

impl TestExt {
	/// Configure some native token balances
	pub fn with_balances(mut self, balances: &[(AccountId, Balance)]) -> Self {
		self.balances = balances.to_vec();
		self
	}
	/// Configure some XRP asset balances
	pub fn with_xrp_balances(mut self, balances: &[(AccountId, Balance)]) -> Self {
		self.xrp_balances = balances
			.to_vec()
			.into_iter()
			.map(|(who, balance)| (MOCK_PAYMENT_ASSET_ID, who, balance))
			.collect();
		self
	}
	/// Configure block number
	pub fn with_block_number(mut self, block_number: BlockNumber) -> Self {
		self.block_number = block_number;
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		if !self.balances.is_empty() {
			pallet_balances::GenesisConfig::<Test> { balances: self.balances }
				.assimilate_storage(&mut storage)
				.unwrap();
		}
		if !self.xrp_balances.is_empty() {
			let assets = vec![(MOCK_PAYMENT_ASSET_ID, create_account(10), true, 1)];
			let metadata = vec![(MOCK_PAYMENT_ASSET_ID, b"XRP".to_vec(), b"XRP".to_vec(), 6_u8)];
			let accounts = self.xrp_balances;
			pallet_assets::GenesisConfig::<Test> { assets, metadata, accounts }
				.assimilate_storage(&mut storage)
				.unwrap();
		}

		let block_number = std::cmp::max(self.block_number, 1);

		let mut ext: sp_io::TestExternalities = storage.into();
		ext.execute_with(|| {
			System::initialize(&block_number, &[0u8; 32].into(), &Default::default())
		});
		ext
	}
}
