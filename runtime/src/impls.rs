// Copyright 2018-2021 Parity Techn ologies(UK) Ltd. and Centrality Investments Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Some configurable implementations as associated type for the substrate runtime.

use crate::{Runtime, Session};
use frame_support::{
    pallet_prelude::*,
    traits::{
        tokens::{fungible::Inspect, DepositConsequence, WithdrawConsequence},
        Currency, ExistenceRequirement, FindAuthor, SignedImbalance, WithdrawReasons,
    },
};
use pallet_evm::AddressMapping as AddressMappingT;
use root_primitives::{AccountId, Balance};
use sp_core::{H160, U256};
use sp_runtime::{
    traits::{SaturatedConversion, Zero},
    ConsensusEngineId,
};
use sp_std::{marker::PhantomData, prelude::*};

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

/// Wraps spending currency (MYCL) for use by the EVM
/// Scales balances into 18dp equivalents which ethereum tooling and contracts expect
pub struct EvmCurrencyScaler<I: Inspect<AccountId>>(PhantomData<I>);
impl<I: Inspect<AccountId, Balance = Balance> + Currency<AccountId>> Inspect<AccountId>
for EvmCurrencyScaler<I>
{
    type Balance = Balance;

    /// The total amount of issuance in the system.
    fn total_issuance() -> Self::Balance {
        <I as Inspect<AccountId>>::total_issuance()
    }

    /// The minimum balance any single account may have.
    fn minimum_balance() -> Self::Balance {
        <I as Inspect<AccountId>>::minimum_balance()
    }

    /// Get the balance of `who`.
    /// Scaled up so values match expectations of an 18dp asset
    fn balance(who: &AccountId) -> Self::Balance {
        Self::reducible_balance(who, true)
    }

    /// Get the maximum amount that `who` can withdraw/transfer successfully.
    /// Scaled up so values match expectations of an 18dp asset
    fn reducible_balance(who: &AccountId, keep_alive: bool) -> Self::Balance {
        // Careful for overflow!
        let raw = I::reducible_balance(who, keep_alive);
        U256::from(raw).saturating_mul(U256::from(XRP_UNIT_VALUE)).saturated_into()
    }

    /// Returns `true` if the balance of `who` may be increased by `amount`.
    fn can_deposit(_who: &AccountId, _amount: Self::Balance, _mint: bool) -> DepositConsequence {
        unimplemented!();
    }

    /// Returns `Failed` if the balance of `who` may not be decreased by `amount`, otherwise
    /// the consequence.
    fn can_withdraw(who: &AccountId, amount: Self::Balance) -> WithdrawConsequence<Self::Balance> {
        I::can_withdraw(who, amount)
    }
}

/// Currency impl for EVM usage
/// It proxies to the inner curreny impl while leaving some unused methods
/// unimplemented
impl<I> Currency<AccountId> for EvmCurrencyScaler<I>
    where
        I: Inspect<AccountId, Balance = Balance>,
        I: Currency<
            AccountId,
            Balance = Balance,
            PositiveImbalance = pallet_balances::PositiveImbalance<Runtime>,
            NegativeImbalance = pallet_balances::NegativeImbalance<Runtime>,
        >,
{
    type Balance = <I as Currency<AccountId>>::Balance;
    type PositiveImbalance = <I as Currency<AccountId>>::PositiveImbalance;
    type NegativeImbalance = <I as Currency<AccountId>>::NegativeImbalance;

    fn free_balance(who: &AccountId) -> Self::Balance {
        Self::balance(who)
    }
    fn total_issuance() -> Self::Balance {
        <I as Currency<AccountId>>::total_issuance()
    }
    fn minimum_balance() -> Self::Balance {
        <I as Currency<AccountId>>::minimum_balance()
    }
    fn total_balance(who: &AccountId) -> Self::Balance {
        Self::balance(who)
    }
    fn transfer(
        from: &AccountId,
        to: &AccountId,
        value: Self::Balance,
        req: ExistenceRequirement,
    ) -> DispatchResult {
        I::transfer(from, to, scale_wei_to_6dp(value), req)
    }
    fn ensure_can_withdraw(
        _who: &AccountId,
        _amount: Self::Balance,
        _reasons: WithdrawReasons,
        _new_balance: Self::Balance,
    ) -> DispatchResult {
        unimplemented!();
    }
    fn withdraw(
        who: &AccountId,
        value: Self::Balance,
        reasons: WithdrawReasons,
        req: ExistenceRequirement,
    ) -> Result<Self::NegativeImbalance, DispatchError> {
        I::withdraw(who, scale_wei_to_6dp(value), reasons, req)
    }
    fn deposit_into_existing(
        who: &AccountId,
        value: Self::Balance,
    ) -> Result<Self::PositiveImbalance, DispatchError> {
        I::deposit_into_existing(who, scale_wei_to_6dp(value))
    }
    fn deposit_creating(who: &AccountId, value: Self::Balance) -> Self::PositiveImbalance {
        I::deposit_creating(who, scale_wei_to_6dp(value))
    }
    fn make_free_balance_be(
        who: &AccountId,
        balance: Self::Balance,
    ) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
        I::make_free_balance_be(who, scale_wei_to_6dp(balance))
    }
    fn can_slash(_who: &AccountId, _value: Self::Balance) -> bool {
        false
    }
    fn slash(_who: &AccountId, _value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
        unimplemented!();
    }
    fn burn(mut _amount: Self::Balance) -> Self::PositiveImbalance {
        unimplemented!();
    }
    fn issue(mut _amount: Self::Balance) -> Self::NegativeImbalance {
        unimplemented!();
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
        AccountId: From<H160>,
{
    fn into_account_id(address: H160) -> AccountId {
        address.into()
    }
}