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

//! Some configurable implementations as associated type for the substrate runtime.

use core::ops::Mul;

use fp_evm::{CheckEvmTransaction, InvalidEvmTransactionError};
use frame_support::{
	dispatch::{EncodeLike, RawOrigin},
	pallet_prelude::*,
	traits::{
		fungible::Inspect,
		tokens::{DepositConsequence, Fortitude, Preservation, Provenance, WithdrawConsequence},
		CallMetadata, Currency, ExistenceRequirement, FindAuthor, GetCallMetadata, Imbalance,
		InstanceFilter, OnUnbalanced, ReservableCurrency, SignedImbalance, WithdrawReasons,
	},
	weights::WeightToFee,
};
use pallet_evm::{AddressMapping as AddressMappingT, EnsureAddressOrigin, OnChargeEVMTransaction};
use sp_core::{H160, U256};
use sp_runtime::{
	generic::{Era, SignedPayload},
	traits::{
		AccountIdConversion, Dispatchable, Extrinsic, LookupError, SaturatedConversion, Saturating,
		StaticLookup, UniqueSaturatedInto, Verify, Zero,
	},
	ConsensusEngineId, Permill,
};
use sp_std::{marker::PhantomData, prelude::*};
use trn_pact::types::{Numeric, PactType, StringLike};

use precompile_utils::{
	constants::{
		FEE_PROXY_ADDRESS, FUTUREPASS_PRECOMPILE_ADDRESS_PREFIX, FUTUREPASS_REGISTRAR_PRECOMPILE,
	},
	keccak256, Address, ErcIdConversion,
};
use seed_pallet_common::{
	utils::{scale_decimals_to_wei, scale_wei_to_correct_decimals},
	EthereumEventRouter as EthereumEventRouterT, EthereumEventSubscriber, EventRouterError,
	EventRouterResult, FinalSessionTracker, MaintenanceCheck, OnNewAssetSubscriber,
};
use seed_primitives::{AccountId, Balance, Nonce, Signature};

use crate::{
	BlockHashCount, Runtime, RuntimeCall, Session, SessionsPerEra, SlashPotId, Staking, System,
	UncheckedExtrinsic, EVM,
};
use doughnut_rs::Topping;

/// Constant factor for scaling CPAY to its smallest indivisible unit
const XRP_UNIT_VALUE: Balance = 10_u128.pow(12);

/// Convert 18dp wei values to 6dp equivalents (XRP)
/// fractional amounts < `XRP_UNIT_VALUE` are rounded up by adding 1 / 0.000001 xrp
pub fn scale_wei_to_6dp(value: Balance) -> Balance {
	scale_wei_to_correct_decimals(value.into(), 6)
}

/// convert 6dp (XRP) to 18dp (wei)
pub fn scale_6dp_to_wei(value: Balance) -> Balance {
	scale_decimals_to_wei(value.into(), 6)
}

/// Wraps spending currency (XRP) for use by the EVM
/// Scales balances into 18dp equivalents which ethereum tooling and contracts expect
pub struct EvmCurrencyScaler<C>(PhantomData<C>);

impl<C: Inspect<AccountId, Balance = Balance> + Currency<AccountId>> Inspect<AccountId>
	for EvmCurrencyScaler<C>
{
	type Balance = Balance;

	/// The total amount of issuance in the system.
	fn total_issuance() -> Self::Balance {
		<C as Inspect<AccountId>>::total_issuance()
	}

	/// The minimum balance any single account may have.
	fn minimum_balance() -> Self::Balance {
		<C as Inspect<AccountId>>::minimum_balance()
	}

	/// The total blance of `who`
	fn total_balance(who: &AccountId) -> Self::Balance {
		<C as Inspect<AccountId>>::total_balance(who)
	}

	/// Get the balance of `who`.
	/// Scaled up so values match expectations of an 18dp asset
	fn balance(who: &AccountId) -> Self::Balance {
		Self::reducible_balance(who, Preservation::Expendable, Fortitude::Polite)
	}

	/// Get the maximum amount that `who` can withdraw/transfer successfully.
	/// Scaled up so values match expectations of an 18dp asset
	/// preservation has been hardcoded to Preservation::Expendable to provide a similar experience
	/// to users coming from Ethereum (Following POLA principles)
	fn reducible_balance(
		who: &AccountId,
		_preservation: Preservation,
		force: Fortitude,
	) -> Self::Balance {
		// Careful for overflow!
		let raw = C::reducible_balance(who, Preservation::Expendable, force);
		U256::from(raw).saturating_mul(U256::from(XRP_UNIT_VALUE)).saturated_into()
	}

	/// Returns `true` if the balance of `who` may be increased by `amount`.
	fn can_deposit(
		who: &AccountId,
		amount: Self::Balance,
		provenance: Provenance,
	) -> DepositConsequence {
		C::can_deposit(who, amount, provenance)
	}

	/// Returns `Failed` if the balance of `who` may not be decreased by `amount`, otherwise
	/// the consequence.
	fn can_withdraw(who: &AccountId, amount: Self::Balance) -> WithdrawConsequence<Self::Balance> {
		C::can_withdraw(who, amount)
	}
}

/// Currency impl for EVM usage
/// It proxies to the inner currency impl while leaving some unused methods
/// unimplemented
impl<C> Currency<AccountId> for EvmCurrencyScaler<C>
where
	C: Currency<AccountId, Balance = Balance>,
{
	type Balance = Balance;
	type PositiveImbalance = C::PositiveImbalance;
	type NegativeImbalance = C::NegativeImbalance;

	fn total_balance(who: &AccountId) -> Self::Balance {
		C::total_balance(who)
	}
	fn can_slash(_who: &AccountId, _value: Self::Balance) -> bool {
		false
	}
	fn total_issuance() -> Self::Balance {
		C::total_issuance()
	}
	fn minimum_balance() -> Self::Balance {
		C::minimum_balance()
	}
	fn burn(amount: Self::Balance) -> Self::PositiveImbalance {
		C::burn(scale_wei_to_6dp(amount))
	}
	fn issue(amount: Self::Balance) -> Self::NegativeImbalance {
		C::issue(scale_wei_to_6dp(amount))
	}
	fn free_balance(who: &AccountId) -> Self::Balance {
		C::free_balance(who)
	}
	fn ensure_can_withdraw(
		who: &AccountId,
		amount: Self::Balance,
		reasons: WithdrawReasons,
		new_balance: Self::Balance,
	) -> DispatchResult {
		C::ensure_can_withdraw(who, scale_wei_to_6dp(amount), reasons, new_balance)
	}
	fn transfer(
		from: &AccountId,
		to: &AccountId,
		value: Self::Balance,
		req: ExistenceRequirement,
	) -> DispatchResult {
		// After the Substrate v1.0 update, transactions that are attempting to transfer 0 will
		// fail if the destination account does not exist.
		// This is due to the amount being less than the existential deposit returning an error
		// In all EVM transactions, even if the value is set to 0, a transfer of that amount
		// will be initiated by the executor which will fail.
		// A workaround is to simply return Ok() if the value is 0, bypassing the actual transfer
		if value == Self::Balance::default() {
			return Ok(());
		}
		C::transfer(from, to, scale_wei_to_6dp(value), req)
	}
	fn slash(who: &AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
		C::slash(who, scale_wei_to_6dp(value))
	}
	fn deposit_into_existing(
		who: &AccountId,
		value: Self::Balance,
	) -> Result<Self::PositiveImbalance, DispatchError> {
		C::deposit_into_existing(who, scale_wei_to_6dp(value))
	}
	fn deposit_creating(who: &AccountId, value: Self::Balance) -> Self::PositiveImbalance {
		C::deposit_creating(who, scale_wei_to_6dp(value))
	}
	fn withdraw(
		who: &AccountId,
		value: Self::Balance,
		reasons: WithdrawReasons,
		req: ExistenceRequirement,
	) -> Result<Self::NegativeImbalance, DispatchError> {
		C::withdraw(who, scale_wei_to_6dp(value), reasons, req)
	}
	fn make_free_balance_be(
		who: &AccountId,
		balance: Self::Balance,
	) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
		C::make_free_balance_be(who, scale_wei_to_6dp(balance))
	}
}

/// Find block author formatted for ethereum compat
pub struct EthereumFindAuthor<F>(PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for EthereumFindAuthor<F> {
	fn find_author<'a, I>(digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		if let Some(author_index) = F::find_author(digests) {
			if let Some(stash) = Session::validators().get(author_index as usize) {
				return Some(Into::<H160>::into(*stash));
			}
		}
		None
	}
}

/// EVM to Root address mapping impl
pub struct AddressMapping<AccountId>(PhantomData<AccountId>);

impl<AccountId> AddressMappingT<AccountId> for AddressMapping<AccountId>
where
	AccountId:
		From<H160> + From<seed_primitives::AccountId> + EncodeLike<seed_primitives::AccountId>,
{
	fn into_account_id(address: H160) -> AccountId {
		// metamask -> getBalance RPC -> account_basic -> EVM::account_basic ->
		// T::AddressMapping::into_account_id checked_extrinsic (apply) ->
		// pre_dispatch_self_contained -> validate_transaction_in_block -> EVM::account_basic
		if let Some(futurepass) =
			pallet_futurepass::DefaultProxy::<Runtime>::get::<AccountId>(address.into())
		{
			return futurepass.into();
		}
		address.into()
	}
}

impl<RuntimeId> ErcIdConversion<RuntimeId> for Runtime
where
	RuntimeId: From<u32> + Into<u32>,
{
	type EvmId = Address;

	// Get runtime Id from EVM address
	fn evm_id_to_runtime_id(
		evm_id: Self::EvmId,
		precompile_address_prefix: &[u8; 4],
	) -> Option<RuntimeId> {
		let h160_address: H160 = evm_id.into();
		let (prefix_part, id_part) = h160_address.as_fixed_bytes().split_at(4);

		if prefix_part == precompile_address_prefix {
			let mut buf = [0u8; 4];
			buf.copy_from_slice(&id_part[..4]);
			let runtime_id: RuntimeId = u32::from_be_bytes(buf).into();

			Some(runtime_id)
		} else {
			None
		}
	}
	// Get EVM address from runtime_id (i.e. asset_id or collection_id)
	fn runtime_id_to_evm_id(
		runtime_id: RuntimeId,
		precompile_address_prefix: &[u8; 4],
	) -> Self::EvmId {
		let mut buf = [0u8; 20];
		let id: u32 = runtime_id.into();
		buf[0..4].copy_from_slice(precompile_address_prefix);
		buf[4..8].copy_from_slice(&id.to_be_bytes());

		H160::from(buf).into()
	}
}

/// Handles negative imbalances resulting from slashes by moving the amount to a predestined holding
/// account
pub struct SlashImbalanceHandler;

// Slash handler for elections-phragmen-pallet (imbalance is in $ROOT)
/// On slash move funds to a dedicated slash pot, it could be managed by treasury later
impl OnUnbalanced<pallet_balances::NegativeImbalance<Runtime>> for SlashImbalanceHandler {
	fn on_nonzero_unbalanced(amount: pallet_balances::NegativeImbalance<Runtime>) {
		<Runtime as pallet_election_provider_multi_phase::Config>::Currency::resolve_creating(
			&SlashPotId::get().into_account_truncating(),
			amount,
		);
	}
}

/// Submits a transaction with the node's public and signature type. Adheres to the signed extension
/// format of the chain.
impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		public: <Signature as Verify>::Signer,
		account: AccountId,
		nonce: Nonce,
	) -> Option<(RuntimeCall, <UncheckedExtrinsic as Extrinsic>::SignaturePayload)> {
		let tip = 0;
		// take the biggest period possible.
		let period =
			BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;
		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let era = Era::mortal(period, current_block);
		let extra = (
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(era),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_maintenance_mode::MaintenanceChecker::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				log::error!("unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (account, signature, extra)))
	}
}

/// Tracks session/era status of the staking pallet
pub struct StakingSessionTracker;

impl FinalSessionTracker for StakingSessionTracker {
	/// Returns whether the active session is the final session of an era
	fn is_active_session_final() -> bool {
		use pallet_staking::Forcing;
		let active_era = Staking::active_era().map(|e| e.index).unwrap_or(0);
		// This is only `Some` when current era has already progressed to the next era, while the
		// active era is one behind (i.e. in the *last session of the active era*, or *first session
		// of the new current era*, depending on how you look at it).
		if let Some(era_start_session_index) = Staking::eras_start_session_index(active_era) {
			if Session::current_index()
				== era_start_session_index + SessionsPerEra::get().saturating_sub(1)
			{
				// natural era rotation
				return true;
			}
		}

		// check if era is going to be forced e.g. due to forced re-election
		match Staking::force_era() {
			Forcing::ForceNew | Forcing::ForceAlways => true,
			Forcing::NotForcing | Forcing::ForceNone => false,
		}
	}
}

/// Handles routing verified bridge messages to other pallets
pub struct EthereumEventRouter;

impl EthereumEventRouterT for EthereumEventRouter {
	/// Route an event to a handler at `destination`
	/// - `source` the sender address on Ethereum
	/// - `destination` the intended handler (pseudo) address
	/// - `data` the Ethereum ABI encoded event data
	fn route(source: &H160, destination: &H160, data: &[u8]) -> EventRouterResult {
		// Route event to specific subscriber pallet
		if destination == &<pallet_echo::Pallet<Runtime> as EthereumEventSubscriber>::address() {
			<pallet_echo::Pallet<Runtime> as EthereumEventSubscriber>::process_event(source, data)
				.map_err(|(w, err)| (w, EventRouterError::FailedProcessing(err)))
		} else if destination
			== &<pallet_erc20_peg::Pallet<Runtime> as EthereumEventSubscriber>::address()
		{
			<pallet_erc20_peg::Pallet<Runtime> as EthereumEventSubscriber>::process_event(
				source, data,
			)
			.map_err(|(w, err)| (w, EventRouterError::FailedProcessing(err)))
		} else if destination
			== &<pallet_nft_peg::Pallet<Runtime> as EthereumEventSubscriber>::address()
		{
			<pallet_nft_peg::Pallet<Runtime> as EthereumEventSubscriber>::process_event(
				source, data,
			)
			.map_err(|(w, err)| (w, EventRouterError::FailedProcessing(err)))
		} else {
			Err((Weight::zero(), EventRouterError::NoReceiver))
		}
	}
}

pub struct OnNewAssetSubscription;

impl<RuntimeId> OnNewAssetSubscriber<RuntimeId> for OnNewAssetSubscription
where
	RuntimeId: From<u32> + Into<u32>,
{
	fn on_asset_create(runtime_id: RuntimeId, precompile_address_prefix: &[u8; 4]) {
		// Insert some code into the evm for the precompile address,
		// This will mean the precompile address passes checks that reference an address's byte code
		// i.e. EXTCODESIZE
		let address = <Runtime as ErcIdConversion<RuntimeId>>::runtime_id_to_evm_id(
			runtime_id,
			precompile_address_prefix,
		);
		pallet_evm::Pallet::<Runtime>::create_account(
			address.into(),
			b"TRN Asset Precompile".to_vec(),
		);
	}
}

/// `WeightToFee` implementation converts weight to fee using a fixed % deduction
pub struct PercentageOfWeight<M>(sp_std::marker::PhantomData<M>);

impl<M> WeightToFee for PercentageOfWeight<M>
where
	M: Get<Permill>,
{
	type Balance = Balance;

	fn weight_to_fee(weight: &Weight) -> Balance {
		M::get().mul(weight.ref_time() as Balance)
	}
}

pub struct HandleTxValidation<E: From<InvalidEvmTransactionError>>(PhantomData<E>);

impl<E: From<InvalidEvmTransactionError>> fp_evm::HandleTxValidation<E> for HandleTxValidation<E> {
	fn with_balance_for(
		evm_config: &CheckEvmTransaction<E>,
		who: &fp_evm::Account,
	) -> Result<(), E> {
		let decoded_override_destination = H160::from_low_u64_be(FEE_PROXY_ADDRESS);
		// If we are not overriding with a fee preference, proceed with calculating a fee
		if evm_config.transaction.to != Some(decoded_override_destination) {
			// call default trait function instead
			<() as fp_evm::HandleTxValidation<E>>::with_balance_for(evm_config, who)?
		}
		Ok(())
	}
}

pub struct FutureverseEnsureAddressSame<AccountId>(sp_std::marker::PhantomData<AccountId>);

impl<OuterOrigin, AccountId> EnsureAddressOrigin<OuterOrigin>
	for FutureverseEnsureAddressSame<AccountId>
where
	OuterOrigin: Into<Result<RawOrigin<AccountId>, OuterOrigin>> + From<RawOrigin<AccountId>>,
	AccountId: Into<H160> + Copy,
{
	type Success = AccountId;

	fn try_address_origin(address: &H160, origin: OuterOrigin) -> Result<AccountId, OuterOrigin> {
		origin.into().and_then(|o| match o {
			RawOrigin::Signed(who) if &who.into() == address => Ok(who),
			r => Err(OuterOrigin::from(r)),
		})
	}
}

pub struct MaintenanceModeCallValidator;
impl seed_pallet_common::ExtrinsicChecker for MaintenanceModeCallValidator {
	type Call = RuntimeCall;
	type Extra = ();
	type Result = bool;
	fn check_extrinsic(call: &Self::Call, _extra: &Self::Extra) -> Self::Result {
		!pallet_maintenance_mode::MaintenanceChecker::<Runtime>::call_paused(call)
	}
}

pub struct FuturepassLookup;
impl StaticLookup for FuturepassLookup {
	type Source = H160;
	type Target = H160;

	/// Lookup a futurepass for a given address
	fn lookup(holder: Self::Source) -> Result<Self::Target, LookupError> {
		pallet_futurepass::Holders::<Runtime>::get::<AccountId>(holder.into())
			.map(|futurepass| futurepass.into())
			.ok_or(LookupError)
	}

	/// Lookup holder for a given futurepass using ProxyPalletProvider.
	/// Returns 0 address (default) if no holder is found.
	fn unlookup(futurepass: Self::Target) -> Self::Source {
		<ProxyPalletProvider as pallet_futurepass::ProxyProvider<Runtime>>::owner(
			&futurepass.into(),
		)
		.unwrap_or_default()
		.into()
	}
}
impl seed_pallet_common::ExtrinsicChecker for FuturepassLookup {
	type Call = <Runtime as frame_system::Config>::RuntimeCall;
	type Extra = ();
	type Result = bool;
	fn check_extrinsic(call: &Self::Call, _extra: &Self::Extra) -> Self::Result {
		match call {
			// Check for direct Futurepass proxy_extrinsic call
			RuntimeCall::Futurepass(pallet_futurepass::Call::proxy_extrinsic { .. }) => true,
			// Check for FeeProxy call containing Futurepass proxy_extrinsic call
			RuntimeCall::FeeProxy(pallet_fee_proxy::Call::call_with_fee_preferences {
				call: inner_call,
				..
			}) => {
				matches!(
					inner_call.as_ref(),
					RuntimeCall::Futurepass(pallet_futurepass::Call::proxy_extrinsic { .. })
				)
			},
			// All other cases
			_ => false,
		}
	}
}

pub struct FuturepassCallValidator;
impl seed_pallet_common::ExtrinsicChecker for FuturepassCallValidator {
	type Call = <Runtime as frame_system::Config>::RuntimeCall;
	type Extra = ();
	type Result = bool;
	fn check_extrinsic(call: &Self::Call, _extra: &Self::Extra) -> Self::Result {
		matches!(call, RuntimeCall::Xrpl(pallet_xrpl::Call::transact { .. }))
	}
}

pub struct ProxyPalletProvider;

impl pallet_futurepass::ProxyProvider<Runtime> for ProxyPalletProvider {
	fn exists(futurepass: &AccountId, delegate: &AccountId, proxy_type: Option<ProxyType>) -> bool {
		pallet_proxy::Pallet::<Runtime>::find_proxy(futurepass, delegate, proxy_type).is_ok()
	}

	fn owner(futurepass: &AccountId) -> Option<AccountId> {
		let (proxy_definitions, _) = pallet_proxy::Proxies::<Runtime>::get(futurepass);
		proxy_definitions
			.into_iter()
			.map(|proxy_def| (proxy_def.delegate, proxy_def.proxy_type))
			.filter(|(_, proxy_type)| proxy_type == &ProxyType::Owner)
			.map(|(owner, _)| owner)
			.next()
	}

	fn delegates(futurepass: &AccountId) -> Vec<(AccountId, ProxyType)> {
		let (proxy_definitions, _) = pallet_proxy::Proxies::<Runtime>::get(futurepass);
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
		let (proxy_definitions, reserve_amount) = pallet_proxy::Proxies::<Runtime>::get(futurepass);
		// get proxy_definitions length + 1 (cost of upcoming insertion); cost to reserve
		let new_reserve =
			pallet_proxy::Pallet::<Runtime>::deposit(proxy_definitions.len() as u32 + 1);
		let extra_reserve_required = new_reserve - reserve_amount;

		// Check if the futurepass account has balance less than the existential deposit
		// If it does, fund with the ED to allow the Futurepass to reserve balance while still
		// keeping the account alive
		let account_balance = pallet_balances::Pallet::<Runtime>::balance(futurepass);
		let minimum_balance = crate::ExistentialDeposit::get();
		let extra_reserve_required = extra_reserve_required.saturating_add(minimum_balance);
		let missing_balance = extra_reserve_required.saturating_sub(account_balance);

		// If the Futurepass cannot afford to pay for the proxy creation, fund it from the funder account
		if missing_balance > 0 {
			<pallet_balances::Pallet<Runtime> as Currency<_>>::transfer(
				funder,
				futurepass,
				missing_balance,
				ExistenceRequirement::KeepAlive,
			)?;
		}

		let proxy_type = ProxyType::try_from(*proxy_type)?;

		pallet_proxy::Pallet::<Runtime>::add_proxy_delegate(futurepass, *delegate, proxy_type, 0)
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
		let proxy_def = pallet_proxy::Pallet::<Runtime>::find_proxy(futurepass, delegate, None)?;
		// get deposits before proxy removal (value gets mutated in removal)
		let (_, pre_removal_deposit) = pallet_proxy::Proxies::<Runtime>::get(futurepass);

		let result = pallet_proxy::Pallet::<Runtime>::remove_proxy_delegate(
			futurepass,
			*delegate,
			proxy_def.proxy_type,
			0,
		);
		if result.is_ok() {
			let (_, post_removal_deposit) = pallet_proxy::Proxies::<Runtime>::get(futurepass);
			let removal_refund = pre_removal_deposit - post_removal_deposit;
			<pallet_balances::Pallet<Runtime> as Currency<_>>::transfer(
				futurepass,
				receiver,
				removal_refund,
				ExistenceRequirement::KeepAlive,
			)?;
		}
		result
	}

	/// Removing futurepass refunds caller with reserved balance (deposits) of the futurepass.
	fn remove_account(receiver: &AccountId, futurepass: &AccountId) -> DispatchResult {
		let (_, old_deposit) = pallet_proxy::Proxies::<Runtime>::take(futurepass);
		<pallet_balances::Pallet<Runtime> as ReservableCurrency<_>>::unreserve(
			futurepass,
			old_deposit,
		);
		<pallet_balances::Pallet<Runtime> as Currency<_>>::transfer(
			futurepass,
			receiver,
			old_deposit,
			ExistenceRequirement::AllowDeath,
		)?;
		Ok(())
	}

	fn proxy_call(
		caller: <Runtime as frame_system::Config>::RuntimeOrigin,
		futurepass: AccountId,
		call: RuntimeCall,
	) -> DispatchResult {
		let call = pallet_proxy::Call::<Runtime>::proxy {
			real: futurepass,
			force_proxy_type: None,
			call: call.into(),
		};

		RuntimeCall::dispatch(call.into(), caller).map_err(|e| e.error)?;
		Ok(())
	}
}

#[derive(
	Copy,
	Clone,
	Default,
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
	#[default]
	Any = 1,
	NonTransfer = 2,
	Governance = 3,
	Staking = 4,
	Owner = 255,
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

impl From<ProxyType> for u8 {
	fn from(proxy_type: ProxyType) -> u8 {
		match proxy_type {
			ProxyType::NoPermission => 0,
			ProxyType::Any => 1,
			ProxyType::NonTransfer => 2,
			ProxyType::Governance => 3,
			ProxyType::Staking => 4,
			ProxyType::Owner => 255,
		}
	}
}

// Precompile side proxy filter.
// NOTE - Precompile and Substrate side filters should be in sync
// TODO(surangap): check if the granualarity can be improved to the call level (V2)
impl pallet_evm_precompiles_futurepass::EvmProxyCallFilter for ProxyType {
	fn is_evm_proxy_call_allowed(
		&self,
		call: &pallet_evm_precompiles_futurepass::EvmSubCall,
		_recipient_has_code: bool,
	) -> bool {
		if call.to.0 == H160::from_low_u64_be(FUTUREPASS_REGISTRAR_PRECOMPILE)
			|| call.to.0.as_bytes().starts_with(FUTUREPASS_PRECOMPILE_ADDRESS_PREFIX)
		{
			// Whitelist for precompile side
			let sub_call_selector = &call.call_data.inner[..4];
			if sub_call_selector
				== &keccak256!("registerDelegateWithSignature(address,uint8,uint32,bytes)")[..4]
				|| sub_call_selector == &keccak256!("unregisterDelegate(address)")[..4]
				|| sub_call_selector == &keccak256!("transferOwnership(address)")[..4]
			{
				return true;
			}
			return false;
		}
		match self {
			ProxyType::Owner => true,
			ProxyType::Any => true,
			// ProxyType::NonTransfer can not have value. i.e call.value == U256::zero()
			ProxyType::NonTransfer => false,
			ProxyType::Governance => false,
			ProxyType::Staking => false,
			ProxyType::NoPermission => false,
		}
	}
}

// substrate side proxy filter
impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		// NOTE - any call for Proxy, Futurepass pallets can not be proxied except the Whitelist.
		// this may seems extra restrictive than Proxy pallet. But if a delegate has permission to
		// proxy a call of the proxy pallet, they should be able to call it directly in the pallet.
		// This keeps the logic simple and avoids unnecessary loops
		// TODO - implement the whitelist as a list that can be configured in the runtime.
		if matches!(c, RuntimeCall::Proxy(..) | RuntimeCall::Futurepass(..)) {
			// Whitelist currently includes pallet_futurepass::Call::register_delegate,
			// pallet_futurepass::Call::unregister_delegate
			// pallet_futurepass::Call::transfer_futurepass
			if !matches!(
				c,
				RuntimeCall::Futurepass(
					pallet_futurepass::Call::register_delegate_with_signature { .. }
				) | RuntimeCall::Futurepass(pallet_futurepass::Call::unregister_delegate { .. })
					| RuntimeCall::Futurepass(pallet_futurepass::Call::transfer_futurepass { .. })
			) {
				return false;
			}

			// the whitelisted calls above should only be able to be called by
			// the owner of the futurepass
			return self == &ProxyType::Owner;
		}
		match self {
			ProxyType::Owner => true,
			ProxyType::Any => true,
			// TODO - need to add allowed calls under this category in v2. allowing all for now.
			ProxyType::NonTransfer => false,
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

/// Futureverse EVM currency adapter, mainly handles tx fees and associated 18DP(wei) to 6DP(XRP)
/// conversion for fees.
pub struct FutureverseEVMCurrencyAdapter<C, OU>(PhantomData<(C, OU)>);

type NegativeImbalanceOf<C, T> =
	<C as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

impl<T, C, OU> OnChargeEVMTransaction<T> for FutureverseEVMCurrencyAdapter<C, OU>
where
	T: pallet_evm::Config + pallet_assets_ext::Config,
	C: Currency<<T as frame_system::Config>::AccountId>,
	C::PositiveImbalance: Imbalance<
		<C as Currency<<T as frame_system::Config>::AccountId>>::Balance,
		Opposite = C::NegativeImbalance,
	>,
	C::NegativeImbalance: Imbalance<
		<C as Currency<<T as frame_system::Config>::AccountId>>::Balance,
		Opposite = C::PositiveImbalance,
	>,
	OU: OnUnbalanced<NegativeImbalanceOf<C, T>>,
	U256: UniqueSaturatedInto<<C as Currency<<T as frame_system::Config>::AccountId>>::Balance>,
	C::Balance: From<u128>,
	u128: From<C::Balance>,
{
	type LiquidityInfo = Option<NegativeImbalanceOf<C, T>>;

	fn withdraw_fee(who: &H160, fee: U256) -> Result<Self::LiquidityInfo, pallet_evm::Error<T>> {
		if fee.is_zero() {
			return Ok(None);
		}
		let account_id = T::AddressMapping::into_account_id(*who);
		let imbalance = C::withdraw(
			&account_id,
			fee.unique_saturated_into(),
			WithdrawReasons::FEE,
			ExistenceRequirement::AllowDeath,
		)
		.map_err(|e| {
			log::error!(target: "assets", "failed to withdraw fee {:?}; amount (XRP): {}", e, fee.as_u128());
			pallet_evm::Error::<T>::BalanceLow
		})?;
		Ok(Some(imbalance)) // Imbalance returned here is 6DP
	}

	fn correct_and_deposit_fee(
		who: &H160,
		corrected_fee: U256,
		base_fee: U256,
		already_withdrawn: Self::LiquidityInfo,
	) -> Self::LiquidityInfo {
		if let Some(paid) = already_withdrawn {
			let account_id = T::AddressMapping::into_account_id(*who);

			// NOTE: Here paid is in 6DP and corrected_fee is in 18DP. Hence convert paid to 18DP
			// before any calculation.
			let paid_18dp: C::Balance = scale_6dp_to_wei(paid.peek().into()).into();

			// Calculate how much refund we should return
			let refund_amount = paid_18dp.saturating_sub(corrected_fee.unique_saturated_into());
			// refund to the account that paid the fees. If this fails, the
			// account might have dropped below the existential balance. In
			// that case we don't refund anything.
			let refund_imbalance = C::deposit_into_existing(&account_id, refund_amount)
				.unwrap_or_else(|_| C::PositiveImbalance::zero());

			// Make sure this works with 0 ExistentialDeposit
			// https://github.com/paritytech/substrate/issues/10117
			// If we tried to refund something, the account still empty and the ED is set to 0,
			// we call `make_free_balance_be` with the refunded amount.
			let refund_imbalance = if C::minimum_balance().is_zero()
				&& refund_amount > C::Balance::zero()
				&& C::total_balance(&account_id).is_zero()
			{
				// Known bug: Substrate tried to refund to a zeroed AccountData, but
				// interpreted the account to not exist.
				match C::make_free_balance_be(&account_id, refund_amount) {
					SignedImbalance::Positive(p) => p,
					_ => C::PositiveImbalance::zero(),
				}
			} else {
				refund_imbalance
			};

			// merge the imbalance caused by paying the fees and refunding parts of it again.
			let adjusted_paid = paid
				.offset(refund_imbalance)
				.same()
				.unwrap_or_else(|_| C::NegativeImbalance::zero());

			// base_fee is in 18DP, adjusted_paid is in 6DP. Hence we need to scale base_fee to 6DP
			// before the split
			let base_fee_6dp: C::Balance =
				scale_wei_to_6dp(base_fee.unique_saturated_into().into()).into();

			let (base_fee, tip) = adjusted_paid.split(base_fee_6dp);
			// Handle base fee. Can be either burned, rationed, etc ...
			OU::on_unbalanced(base_fee); // base_fee here is in 6DP
			return Some(tip); // tip here is in 6DP
		}
		None
	}

	fn pay_priority_fee(tip: Self::LiquidityInfo) {
		// Default Ethereum behaviour: issue the tip to the block author.
		if let Some(tip) = tip {
			let account_id = T::AddressMapping::into_account_id(EVM::find_author());
			// tip is in 6DP. We should convert it to 18DP before passing it down to C, as another
			// 18DP to 6DP conversion happening there.
			let tip_18dp: C::Balance = scale_6dp_to_wei(tip.peek().into()).into();
			let _ = C::deposit_into_existing(&account_id, tip_18dp);
		}
	}
}

/// Ensures that the origin is a Futurepass account
pub struct EnsureFuturepass<AccountId>(sp_std::marker::PhantomData<AccountId>);

impl<O, AccountId> EnsureOrigin<O> for EnsureFuturepass<AccountId>
where
	O: Into<Result<RawOrigin<AccountId>, O>> + From<RawOrigin<AccountId>>,
	AccountId: Clone + Into<H160> + From<H160>,
{
	type Success = H160;

	fn try_origin(o: O) -> Result<Self::Success, O> {
		o.into().and_then(|o| match o {
			RawOrigin::Signed(who) => {
				let address: H160 = who.clone().into();

				// Check prefix for futurepass match
				if address.as_bytes()[..4] != *FUTUREPASS_PRECOMPILE_ADDRESS_PREFIX {
					return Err(RawOrigin::Signed(who).into());
				}

				// Check if the Futurepass has an owner (must exist)
				// - not run in benchmarks as we need to create new dependencies in pallet-partner-attribution
				#[cfg(not(feature = "runtime-benchmarks"))]
				return <ProxyPalletProvider as pallet_futurepass::ProxyProvider<Runtime>>::owner(
					&address.into(),
				)
				.map(|_| address)
				.ok_or_else(|| RawOrigin::Signed(who).into());

				#[cfg(feature = "runtime-benchmarks")]
				return Ok(address);
			},
			r => Err(r.into()),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<O, ()> {
		Ok(RawOrigin::Root.into())
	}
}

pub struct DoughnutCallValidator;
impl seed_pallet_common::ExtrinsicChecker for DoughnutCallValidator {
	type Call = RuntimeCall;
	type Extra = Topping;
	type Result = DispatchResult;
	fn check_extrinsic(call: &Self::Call, topping: &Self::Extra) -> DispatchResult {
		// matcher to select the actual call to validate
		let actual_call: Self::Call = match &call {
			RuntimeCall::Futurepass(pallet_futurepass::Call::proxy_extrinsic {
				call: inner_call,
				..
			}) => *inner_call.clone(),
			RuntimeCall::FeeProxy(pallet_fee_proxy::Call::call_with_fee_preferences {
				call: inner_call_1,
				..
			}) => {
				if let RuntimeCall::Futurepass(pallet_futurepass::Call::proxy_extrinsic {
					call: inner_call_2,
					..
				}) = *inner_call_1.clone()
				{
					*inner_call_2.clone()
				} else {
					*inner_call_1.clone()
				}
			},
			_ => call.clone(),
		};

		if pallet_maintenance_mode::MaintenanceChecker::<Runtime>::call_paused(&actual_call) {
			return Err(frame_system::Error::<Runtime>::CallFiltered.into());
		}

		let CallMetadata { function_name, pallet_name } = actual_call.get_call_metadata();
		// selective matching the inner call for permission validations
		match &actual_call {
			// Balances
			RuntimeCall::Balances(pallet_balances::Call::transfer { dest, value }) => {
				let who = <Runtime as frame_system::Config>::Lookup::lookup(*dest)
					.map_err(|_| pallet_doughnut::Error::<Runtime>::ToppingPermissionDenied)?;
				let destination: [u8; 20] = who.into();
				let value_u128: u128 = *value;

				topping
					.validate_module(
						pallet_name,
						function_name,
						// TODO: change the u64 conversion once pact Numeric support u128
						&[
							PactType::StringLike(StringLike(destination.to_vec())),
							PactType::Numeric(Numeric(value_u128 as u64)),
						],
					)
					.map_err(|_| pallet_doughnut::Error::<Runtime>::ToppingPermissionDenied)?;
				Ok(())
			},
			RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive { dest, value }) => {
				let who = <Runtime as frame_system::Config>::Lookup::lookup(*dest)
					.map_err(|_| pallet_doughnut::Error::<Runtime>::ToppingPermissionDenied)?;
				let destination: [u8; 20] = who.into();
				let value_u128: u128 = *value;

				topping
					.validate_module(
						pallet_name,
						function_name,
						// TODO: change the u64 conversion once pact Numeric support u128
						&[
							PactType::StringLike(StringLike(destination.to_vec())),
							PactType::Numeric(Numeric(value_u128 as u64)),
						],
					)
					.map_err(|_| pallet_doughnut::Error::<Runtime>::ToppingPermissionDenied)?;
				Ok(())
			},
			// Futurepass
			RuntimeCall::Futurepass(pallet_futurepass::Call::create { account }) => {
				let owner_account: [u8; 20] = (*account).into();
				topping
					.validate_module(
						pallet_name,
						function_name,
						&[PactType::StringLike(StringLike(owner_account.to_vec()))],
					)
					.map_err(|_| pallet_doughnut::Error::<Runtime>::ToppingPermissionDenied)?;
				Ok(())
			},
			// System
			RuntimeCall::System(frame_system::Call::remark { remark }) => {
				topping
					.validate_module(
						pallet_name,
						function_name,
						&[PactType::StringLike(StringLike(remark.to_vec()))],
					)
					.map_err(|_| pallet_doughnut::Error::<Runtime>::ToppingPermissionDenied)?;
				Ok(())
			},
			// AssetsExt
			RuntimeCall::AssetsExt(pallet_assets_ext::Call::transfer {
				asset_id,
				destination,
				amount,
				keep_alive,
			}) => {
				let asset_id_u64: u64 = (*asset_id).into();
				let who = <Runtime as frame_system::Config>::Lookup::lookup(*destination)
					.map_err(|_| pallet_doughnut::Error::<Runtime>::ToppingPermissionDenied)?;
				let destination: [u8; 20] = who.into();
				let keep_alive_u64: u64 = (*keep_alive).into();

				topping
					.validate_module(
						pallet_name,
						function_name,
						// TODO: change the u64 conversion once pact Numeric support u128
						&[
							PactType::Numeric(Numeric(asset_id_u64)),
							PactType::StringLike(StringLike(destination.to_vec())),
							PactType::Numeric(Numeric(*amount as u64)),
							PactType::Numeric(Numeric(keep_alive_u64)),
						],
					)
					.map_err(|_| pallet_doughnut::Error::<Runtime>::ToppingPermissionDenied)?;
				Ok(())
			},

			_ => Err(pallet_doughnut::Error::<Runtime>::ToppingPermissionDenied.into()),
		}
	}
}

pub struct DoughnutFuturepassLookup;
impl StaticLookup for DoughnutFuturepassLookup {
	type Source = H160;
	type Target = H160;

	/// Lookup a futurepass for a given address
	fn lookup(holder: Self::Source) -> Result<Self::Target, LookupError> {
		pallet_futurepass::Holders::<Runtime>::get::<AccountId>(holder.into())
			.map(|futurepass| futurepass.into())
			.ok_or(LookupError)
	}

	/// Lookup holder for a given futurepass using ProxyPalletProvider.
	/// Returns 0 address (default) if no holder is found.
	fn unlookup(futurepass: Self::Target) -> Self::Source {
		<ProxyPalletProvider as pallet_futurepass::ProxyProvider<Runtime>>::owner(
			&futurepass.into(),
		)
		.unwrap_or_default()
		.into()
	}
}
impl seed_pallet_common::ExtrinsicChecker for DoughnutFuturepassLookup {
	type Call = <Runtime as frame_system::Config>::RuntimeCall;
	type Extra = ();
	type Result = DispatchResult;

	fn check_extrinsic(call: &Self::Call, _permission_object: &Self::Extra) -> DispatchResult {
		match call {
			// Check for direct Futurepass proxy_extrinsic call
			RuntimeCall::Futurepass(pallet_futurepass::Call::proxy_extrinsic { .. }) => Ok(()),
			// Check for FeeProxy call containing Futurepass proxy_extrinsic call
			RuntimeCall::FeeProxy(pallet_fee_proxy::Call::call_with_fee_preferences {
				call: inner_call,
				..
			}) if matches!(
				inner_call.as_ref(),
				RuntimeCall::Futurepass(pallet_futurepass::Call::proxy_extrinsic { .. })
			) =>
			{
				Ok(())
			},
			// All other cases
			_ => Err(pallet_doughnut::Error::<Runtime>::ToppingPermissionDenied.into()),
		}
	}
}

pub struct CrowdsaleProxyVaultValidator;
impl seed_pallet_common::ExtrinsicChecker for CrowdsaleProxyVaultValidator {
	type Call = RuntimeCall;
	type Extra = ();
	type Result = DispatchResult;

	fn check_extrinsic(call: &Self::Call, _permission_object: &Self::Extra) -> Self::Result {
		// check maintenance mode
		if pallet_maintenance_mode::MaintenanceChecker::<Runtime>::call_paused(call) {
			return Err(frame_system::Error::<Runtime>::CallFiltered.into());
		}

		match call {
			RuntimeCall::System(frame_system::Call::remark { .. }) => Ok(()),
			RuntimeCall::Nft(pallet_nft::Call::set_base_uri { .. }) => Ok(()),
			RuntimeCall::Nft(pallet_nft::Call::set_name { .. }) => Ok(()),
			RuntimeCall::Nft(pallet_nft::Call::set_royalties_schedule { .. }) => Ok(()),
			_ => Err(pallet_crowdsale::Error::<Runtime>::ExtrinsicForbidden.into()),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn wei_to_xrp_units_scaling() {
		let amounts_18 = vec![
			0_u128,
			1_u128,
			999_u128,                       // entirely < 0.0001
			1_000_000_000_000_000_000_u128, // no fractional bits < 0.0001
			1_000_000_000_000_000_001_u128, // fractional bits <  0.0001
			1_000_000_500_000_000_000_u128, // fractional bits <  0.0001
			1_000_001_000_000_000_000_u128, // fractional bits at 0.0001
		];
		let amounts_6 = vec![0, 1, 1, 1_000_000, 1_000_001, 1_000_001, 1_000_001];
		for (amount_18, amount_6) in amounts_18.into_iter().zip(amounts_6.into_iter()) {
			println!("{:?}/{:?}", amount_18, amount_6);
			assert_eq!(scale_wei_to_6dp(amount_18), amount_6);
		}
	}
}
