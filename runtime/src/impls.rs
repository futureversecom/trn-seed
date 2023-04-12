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

//! Some configurable implementations as associated type for the substrate runtime.

use core::ops::Mul;
use evm::backend::Basic;
use fp_evm::{CheckEvmTransaction, InvalidEvmTransactionError};
use frame_support::{
	dispatch::{EncodeLike, RawOrigin},
	pallet_prelude::*,
	traits::{
		fungible::Inspect,
		tokens::{DepositConsequence, WithdrawConsequence},
		Currency, ExistenceRequirement, FindAuthor, InstanceFilter, IsSubType, OnUnbalanced,
		SignedImbalance, WithdrawReasons,
	},
	weights::WeightToFee,
};
use pallet_evm::{AddressMapping as AddressMappingT, EnsureAddressOrigin};
use pallet_futurepass::ProxyProvider;
use pallet_transaction_payment::OnChargeTransaction;
use sp_core::{H160, U256};
use sp_runtime::{
	generic::{Era, SignedPayload},
	traits::{
		AccountIdConversion, DispatchInfoOf, Extrinsic, PostDispatchInfoOf, SaturatedConversion,
		Verify, Zero,
	},
	ConsensusEngineId, Permill,
};
use sp_std::{marker::PhantomData, prelude::*};

use precompile_utils::{constants::FEE_PROXY_ADDRESS, Address, ErcIdConversion};
use seed_pallet_common::{
	EthereumEventRouter as EthereumEventRouterT, EthereumEventSubscriber, EventRouterError,
	EventRouterResult, FinalSessionTracker, OnNewAssetSubscriber,
};
use seed_primitives::{AccountId, Balance, Index, Signature};

use crate::{
	BlockHashCount, Call, Runtime, Session, SessionsPerEra, SlashPotId, Staking, System,
	UncheckedExtrinsic,
};

/// Constant factor for scaling CPAY to its smallest indivisible unit
const XRP_UNIT_VALUE: Balance = 10_u128.pow(12);

/// Convert 18dp wei values to 6dp equivalents (XRP)
/// fractional amounts < `XRP_UNIT_VALUE` are rounded up by adding 1 / 0.000001 xrp
pub fn scale_wei_to_6dp(value: Balance) -> Balance {
	let (quotient, remainder) = (value / XRP_UNIT_VALUE, value % XRP_UNIT_VALUE);
	if remainder.is_zero() {
		quotient
	} else {
		// if value has a fractional part < CPAY unit value
		// it is lost in this divide operation
		quotient + 1
	}
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

	/// Get the balance of `who`.
	/// Scaled up so values match expectations of an 18dp asset
	fn balance(who: &AccountId) -> Self::Balance {
		Self::reducible_balance(who, false)
	}

	/// Get the maximum amount that `who` can withdraw/transfer successfully.
	/// Scaled up so values match expectations of an 18dp asset
	/// keep_alive has been hardcoded to false to provide a similar experience to users coming
	/// from Ethereum (Following POLA principles)
	fn reducible_balance(who: &AccountId, _keep_alive: bool) -> Self::Balance {
		// Careful for overflow!
		let raw = C::reducible_balance(who, false);
		U256::from(raw).saturating_mul(U256::from(XRP_UNIT_VALUE)).saturated_into()
	}

	/// Returns `true` if the balance of `who` may be increased by `amount`.
	fn can_deposit(who: &AccountId, amount: Self::Balance, mint: bool) -> DepositConsequence {
		C::can_deposit(who, amount, mint)
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

	fn free_balance(who: &AccountId) -> Self::Balance {
		C::free_balance(who)
	}
	fn total_issuance() -> Self::Balance {
		C::total_issuance()
	}
	fn minimum_balance() -> Self::Balance {
		C::minimum_balance()
	}
	fn total_balance(who: &AccountId) -> Self::Balance {
		C::total_balance(who)
	}
	fn transfer(
		from: &AccountId,
		to: &AccountId,
		value: Self::Balance,
		req: ExistenceRequirement,
	) -> DispatchResult {
		C::transfer(from, to, scale_wei_to_6dp(value), req)
	}
	fn ensure_can_withdraw(
		who: &AccountId,
		amount: Self::Balance,
		reasons: WithdrawReasons,
		new_balance: Self::Balance,
	) -> DispatchResult {
		C::ensure_can_withdraw(who, scale_wei_to_6dp(amount), reasons, new_balance)
	}
	fn withdraw(
		who: &AccountId,
		value: Self::Balance,
		reasons: WithdrawReasons,
		req: ExistenceRequirement,
	) -> Result<Self::NegativeImbalance, DispatchError> {
		C::withdraw(who, scale_wei_to_6dp(value), reasons, req)
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
	fn make_free_balance_be(
		who: &AccountId,
		balance: Self::Balance,
	) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
		C::make_free_balance_be(who, scale_wei_to_6dp(balance))
	}
	fn can_slash(_who: &AccountId, _value: Self::Balance) -> bool {
		false
	}
	fn slash(who: &AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
		C::slash(who, scale_wei_to_6dp(value))
	}
	fn burn(amount: Self::Balance) -> Self::PositiveImbalance {
		C::burn(scale_wei_to_6dp(amount))
	}
	fn issue(amount: Self::Balance) -> Self::NegativeImbalance {
		C::issue(scale_wei_to_6dp(amount))
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
				return Some(Into::<H160>::into(*stash))
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
			return futurepass.into()
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

/// Slash handler for pallet-staking (imbalance is in $ROOT)
/// On slash move funds to a dedicated slash pot, it could be managed by treasury later
impl OnUnbalanced<pallet_assets_ext::NegativeImbalance<Runtime>> for SlashImbalanceHandler {
	fn on_nonzero_unbalanced(amount: pallet_assets_ext::NegativeImbalance<Runtime>) {
		<Runtime as pallet_staking::Config>::Currency::resolve_creating(
			&SlashPotId::get().into_account_truncating(),
			amount,
		);
	}
}

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
	Call: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: Call,
		public: <Signature as Verify>::Signer,
		account: AccountId,
		nonce: Index,
	) -> Option<(Call, <UncheckedExtrinsic as Extrinsic>::SignaturePayload)> {
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
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				log::error!("unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (account, signature.into(), extra)))
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
			if Session::current_index() ==
				era_start_session_index + SessionsPerEra::get().saturating_sub(1)
			{
				// natural era rotation
				return true
			}
		}

		// check if era is going to be forced e.g. due to forced re-election
		return match Staking::force_era() {
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
		} else if destination ==
			&<pallet_erc20_peg::Pallet<Runtime> as EthereumEventSubscriber>::address()
		{
			<pallet_erc20_peg::Pallet<Runtime> as EthereumEventSubscriber>::process_event(
				source, data,
			)
			.map_err(|(w, err)| (w, EventRouterError::FailedProcessing(err)))
		} else if destination ==
			&<pallet_nft_peg::Pallet<Runtime> as EthereumEventSubscriber>::address()
		{
			<pallet_nft_peg::Pallet<Runtime> as EthereumEventSubscriber>::process_event(
				source, data,
			)
			.map_err(|(w, err)| (w, EventRouterError::FailedProcessing(err)))
		} else {
			Err((0, EventRouterError::NoReceiver))
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
		M::get().mul(*weight as Balance)
	}
}

pub struct HandleTxValidation<E: From<InvalidEvmTransactionError>>(PhantomData<E>);

impl<E: From<InvalidEvmTransactionError>> fp_evm::HandleTxValidation<E> for HandleTxValidation<E> {
	fn with_balance_for(evm_config: &CheckEvmTransaction<E>, who: &Basic) -> Result<(), E> {
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

pub struct ProxyPalletProvider;

impl pallet_futurepass::ProxyProvider<AccountId> for ProxyPalletProvider {
	fn exists(futurepass: &AccountId, delegate: &AccountId) -> bool {
		pallet_proxy::Pallet::<Runtime>::find_proxy(futurepass, delegate, None)
			.map(|_| true)
			.unwrap_or(false)
	}

	fn delegates(futurepass: &AccountId) -> Vec<AccountId> {
		let (proxy_definitions, _) = pallet_proxy::Proxies::<Runtime>::get(futurepass);
		proxy_definitions.into_iter().map(|proxy_def| proxy_def.delegate).collect()
	}

	/// Adding a delegate requires funding the futurepass account (from funder) with the cost of the
	/// proxy creation.
	/// The futurepass cannot pay for itself as it may not have any funds.
	fn add_delegate(
		funder: &AccountId,
		futurepass: &AccountId,
		delegate: &AccountId,
	) -> DispatchResult {
		// pay cost for proxy creation; transfer funds/deposit from delegator to FP account (which
		// executes proxy creation)
		let (proxy_definitions, _) = pallet_proxy::Proxies::<Runtime>::get(futurepass);
		// get proxy_definitions length + 1 (cost of upcoming insertion); cost to reserve
		let creation_cost =
			pallet_proxy::Pallet::<Runtime>::deposit(proxy_definitions.len() as u32 + 1);
		<pallet_balances::Pallet<Runtime> as Currency<_>>::transfer(
			funder,
			futurepass,
			creation_cost,
			ExistenceRequirement::KeepAlive,
		)?;

		let proxy_type = ProxyType::Any;
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
		let proxy_type = ProxyType::Any;

		// get deposits before proxy removal (value gets mutated in removal)
		let (_, pre_removal_deposit) = pallet_proxy::Proxies::<Runtime>::get(futurepass);

		let result = pallet_proxy::Pallet::<Runtime>::remove_proxy_delegate(
			futurepass, *delegate, proxy_type, 0,
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
	Any, /* TODO: ensure no calls are made to futurepass pallet (all extrinsics must be EOA
	      * origin) */
	NonTransfer,
	Governance,
	Staking,
	// Multicurrency,
	// CancelProxy,
}

impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}

// proxy call filter from ethereum side.
// TODO(surangap): check if the granualarity can be improved to the call level
impl pallet_evm_precompiles_futurepass::EvmProxyCallFilter for ProxyType {
	fn is_evm_proxy_call_allowed(
		&self,
		call: &pallet_evm_precompiles_futurepass::EvmSubCall,
		recipient_has_code: bool,
	) -> bool {
		use pallet_evm::PrecompileSet as _;
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => true,
			ProxyType::Governance => true,
			ProxyType::Staking => true,
			// TODO(surangap): implement the filter
			// ProxyType::Any => true,
			// ProxyType::NonTransfer => {
			// 	call.value == U256::zero()
			// 		&& match PrecompileName::from_address(call.to.0) {
			// 		Some(
			// 			PrecompileName::AuthorMappingPrecompile
			// 			| PrecompileName::ParachainStakingPrecompile,
			// 		) => true,
			// 		Some(ref precompile) if is_governance_precompile(precompile) => true,
			// 		_ => false,
			// 	}
			// }
			// ProxyType::Governance => {
			// 	call.value == U256::zero()
			// 		&& matches!(
			// 			PrecompileName::from_address(call.to.0),
			// 			Some(ref precompile) if is_governance_precompile(precompile)
			// 		)
			// }
			// ProxyType::Staking => {
			// 	call.value == U256::zero()
			// 		&& matches!(
			// 			PrecompileName::from_address(call.to.0),
			// 			Some(
			// 				PrecompileName::AuthorMappingPrecompile
			// 					| PrecompileName::ParachainStakingPrecompile
			// 			)
			// 		)
			// }
			// // The proxy precompile does not contain method cancel_proxy
			// ProxyType::CancelProxy => false,
			// ProxyType::Balances => {
			// 	// Allow only "simple" accounts as recipient (no code nor precompile).
			// 	// Note: Checking the presence of the code is not enough because some precompiles
			// 	// have no code.
			// 	!recipient_has_code && !PrecompilesValue::get().is_precompile(call.to.0)
			// }
			// ProxyType::AuthorMapping => {
			// 	call.value == U256::zero()
			// 		&& matches!(
			// 			PrecompileName::from_address(call.to.0),
			// 			Some(PrecompileName::AuthorMappingPrecompile)
			// 		)
			// }
			// // There is no identity precompile
			// ProxyType::IdentityJudgement => false,
		}
	}
}

// substrate side proxy filter
impl InstanceFilter<Call> for ProxyType {
	fn filter(&self, c: &Call) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => true, // TODO
			ProxyType::Governance => true,  // TODO
			ProxyType::Staking => true,     /* TODO
			                                  * ProxyType::NonTransfer => {
			                                  * 	matches!(
			                                  * 		c,
			                                  * 		Call::System(..)
			                                  * 			| Call::ParachainSystem(..)
			                                  * 			| Call::Timestamp(..)
			                                  * 			| Call::ParachainStaking(..)
			                                  * 			| Call::Democracy(..)
			                                  * 			| Call::Preimage(..)
			                                  * 			| Call::CouncilCollective(..)
			                                  * 			| Call::TreasuryCouncilCollective(..)
			                                  * 			| Call::TechCommitteeCollective(..)
			                                  * 			| Call::Identity(..)
			                                  * 			| Call::Utility(..)
			                                  * 			| Call::Proxy(..) | Call::AuthorMapping(..)
			                                  * 			| Call::CrowdloanRewards(
			                                  * 				pallet_crowdloan_rewards::Call::claim { .. }
			                                  * 			)
			                                  * 	)
			                                  * }
			                                  * ProxyType::Governance => matches!(
			                                  * 	c,
			                                  * 	Call::Democracy(..)
			                                  * 		| Call::Preimage(..)
			                                  * 		| Call::CouncilCollective(..)
			                                  * 		| Call::TreasuryCouncilCollective(..)
			                                  * 		| Call::TechCommitteeCollective(..)
			                                  * 		| Call::Utility(..)
			                                  * ),
			                                  * ProxyType::Staking => matches!(c,
			                                  * Call::Staking(..)),
			                                  * ProxyType::CancelProxy => matches!(
			                                  * 	c,
			                                  * 	Call::Proxy(pallet_proxy::Call::reject_announcement { .. })
			                                  * ), */
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

/// Switch gas payer to Futurepass if proxy called with a Futurepass account
pub struct FuturepassTransactionFee;

impl<T> OnChargeTransaction<T> for FuturepassTransactionFee
where
	T: frame_system::Config<AccountId = AccountId>
		+ pallet_transaction_payment::Config
		+ pallet_proxy::Config
		+ pallet_fee_proxy::Config,
	<T as frame_system::Config>::Call: IsSubType<pallet_proxy::Call<T>>,
{
	type Balance =
		<<T as pallet_fee_proxy::Config>::OnChargeTransaction as OnChargeTransaction<T>>::Balance;
	type LiquidityInfo = <<T as pallet_fee_proxy::Config>::OnChargeTransaction as OnChargeTransaction<T>>::LiquidityInfo;

	fn withdraw_fee(
		who: &T::AccountId,
		call: &<T as frame_system::Config>::Call,
		info: &DispatchInfoOf<<T as frame_system::Config>::Call>,
		fee: Self::Balance,
		tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		let mut who = who;
		// if the call is pallet_proxy::Call::proxy(), and the caller is a delegate of the FP(real),
		// we switch the gas payer to the FP
		if let Some(pallet_proxy::Call::proxy { real, .. }) = call.is_sub_type() {
			if ProxyPalletProvider::exists(real, who) {
				who = real;
			}
		}

		<<T as pallet_fee_proxy::Config>::OnChargeTransaction>::withdraw_fee(
			who, call, info, fee, tip,
		)
	}

	fn correct_and_deposit_fee(
		who: &T::AccountId,
		dispatch_info: &DispatchInfoOf<<T as frame_system::Config>::Call>,
		post_info: &PostDispatchInfoOf<<T as frame_system::Config>::Call>,
		corrected_fee: Self::Balance,
		tip: Self::Balance,
		already_withdrawn: Self::LiquidityInfo,
	) -> Result<(), TransactionValidityError> {
		// NOTE - ideally we should check and switch the account to FP here also, But we don't have
		// the call information within this function. What this means, if any extra fee was charged,
		// that fee wont return to FP but the caller. Ideally we could pass the required info via
		// pre, But this requires a new signed extension and some research.
		<<T as pallet_fee_proxy::Config>::OnChargeTransaction>::correct_and_deposit_fee(
			who,
			dispatch_info,
			post_info,
			corrected_fee,
			tip,
			already_withdrawn,
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn wei_to_xrp_units_scaling() {
		let amounts_18 = vec![
			1000000500000000000u128, // fractional bits <  0.0001
			1000000000000000001u128, // fractional bits <  0.0001
			1000001000000000000u128, // fractional bits at 0.0001
			1000000000000000000u128, // no fractional bits < 0.0001
			999u128,                 // entirely < 0.0001
			1u128,
			0u128,
		];
		let amounts_4 = vec![1000001_u128, 1000001, 1000001, 1000000, 1, 1, 0];
		for (amount_18, amount_4) in amounts_18.into_iter().zip(amounts_4.into_iter()) {
			println!("{:?}/{:?}", amount_18, amount_4);
			assert_eq!(scale_wei_to_6dp(amount_18), amount_4);
		}
	}
}
