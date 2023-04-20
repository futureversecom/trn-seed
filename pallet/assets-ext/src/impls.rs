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

use frame_support::{
	ensure,
	pallet_prelude::DispatchResult,
	traits::{
		Currency, ExistenceRequirement, Get, Imbalance, LockIdentifier, LockableCurrency,
		SignedImbalance, WithdrawReasons,
	},
};
use sp_runtime::{
	traits::{AccountIdConversion, Zero},
	DispatchError,
};
use sp_std::marker::PhantomData;

use frame_support::traits::{
	fungible,
	fungibles::{self, Inspect, Transfer, Unbalanced},
	tokens::{DepositConsequence, WithdrawConsequence},
};

use seed_primitives::{AssetId, Balance};

use crate::{
	imbalances::{self, NegativeImbalance, PositiveImbalance},
	Config, Event, Pallet,
};

/// Generic shim for statically defined instance of Currency over a pallet-assets managed asset
pub struct AssetCurrency<T, U>(PhantomData<(T, U)>);

impl<T, U> fungible::Inspect<T::AccountId> for AssetCurrency<T, U>
where
	T: Config,
	U: Get<AssetId>,
{
	type Balance = Balance;

	fn total_issuance() -> Balance {
		<pallet_assets::Pallet<T>>::total_issuance(U::get())
	}

	fn minimum_balance() -> Balance {
		<pallet_assets::Pallet<T>>::minimum_balance(U::get())
	}

	fn balance(who: &T::AccountId) -> Balance {
		<pallet_assets::Pallet<T>>::balance(U::get(), who)
	}

	fn reducible_balance(who: &T::AccountId, keep_alive: bool) -> Balance {
		<pallet_assets::Pallet<T> as fungibles::Inspect<_>>::reducible_balance(
			U::get(),
			who,
			keep_alive,
		)
	}

	fn can_deposit(who: &T::AccountId, amount: Balance, mint: bool) -> DepositConsequence {
		<pallet_assets::Pallet<T>>::can_deposit(U::get(), who, amount, mint)
	}

	fn can_withdraw(who: &T::AccountId, amount: Balance) -> WithdrawConsequence<Balance> {
		<pallet_assets::Pallet<T>>::can_withdraw(U::get(), who, amount)
	}
}

impl<T, U> Currency<T::AccountId> for AssetCurrency<T, U>
where
	T: Config,
	U: Get<AssetId>,
{
	type Balance = Balance;
	type NegativeImbalance = imbalances::NegativeImbalance<T>;
	type PositiveImbalance = imbalances::PositiveImbalance<T>;

	fn free_balance(who: &T::AccountId) -> Self::Balance {
		<pallet_assets::Pallet<T>>::reducible_balance(U::get(), who, false)
	}
	fn total_issuance() -> Self::Balance {
		<pallet_assets::Pallet<T>>::total_issuance(U::get())
	}
	fn minimum_balance() -> Self::Balance {
		<pallet_assets::Pallet<T>>::minimum_balance(U::get())
	}
	fn total_balance(who: &T::AccountId) -> Self::Balance {
		<pallet_assets::Pallet<T>>::balance(U::get(), who)
	}
	fn transfer(
		from: &T::AccountId,
		to: &T::AccountId,
		value: Self::Balance,
		req: ExistenceRequirement,
	) -> DispatchResult {
		// used by evm
		let keep_alive = match req {
			ExistenceRequirement::KeepAlive => false,
			ExistenceRequirement::AllowDeath => true,
		};
		<Pallet<T>>::transfer(U::get(), from, to, value, keep_alive).map(|_| ())
	}
	fn ensure_can_withdraw(
		who: &T::AccountId,
		amount: Self::Balance,
		_reasons: WithdrawReasons,
		new_balance: Self::Balance,
	) -> DispatchResult {
		if amount.is_zero() {
			return Ok(())
		}
		let min_balance = Self::free_balance(who);
		ensure!(new_balance >= min_balance, pallet_assets::Error::<T>::BalanceLow);
		Ok(())
	}
	/// Withdraw some free balance from an account, respecting existence requirements.
	///
	/// Is a no-op if value to be withdrawn is zero.
	fn withdraw(
		who: &T::AccountId,
		value: Self::Balance,
		_reasons: WithdrawReasons,
		_req: ExistenceRequirement,
	) -> Result<Self::NegativeImbalance, DispatchError> {
		// used by pallet-transaction payment & pallet-evm
		<pallet_assets::Pallet<T>>::decrease_balance(U::get(), who, value)?;

		<Pallet<T>>::deposit_event(Event::InternalWithdraw {
			asset_id: U::get(),
			who: who.clone(),
			amount: value,
		});
		Ok(NegativeImbalance::new(value, U::get()))
	}
	/// Deposit some `value` into the free balance of an existing target account `who`.
	///
	/// Is a no-op if the `value` to be deposited is zero.
	fn deposit_into_existing(
		who: &T::AccountId,
		value: Self::Balance,
	) -> Result<Self::PositiveImbalance, DispatchError> {
		// used by pallet-transaction payment & pallet-staking
		if value.is_zero() {
			return Ok(PositiveImbalance::new(0, U::get()))
		}
		<pallet_assets::Pallet<T>>::increase_balance(U::get(), who, value)?;
		<Pallet<T>>::deposit_event(Event::InternalDeposit {
			asset_id: U::get(),
			who: who.clone(),
			amount: value,
		});
		Ok(PositiveImbalance::new(value, U::get()))
	}
	/// Deposit some `value` into the free balance of `who`, possibly creating a new account.
	///
	/// This function is a no-op if:
	/// - the `value` to be deposited is zero; or
	/// - the `value` to be deposited is less than the required ED and the account does not yet
	///   exist; or
	/// - the deposit would necessitate the account to exist and there are no provider references;
	///   or
	/// - `value` is so large it would cause the balance of `who` to overflow.
	fn deposit_creating(who: &T::AccountId, value: Self::Balance) -> Self::PositiveImbalance {
		Self::deposit_into_existing(who, value).unwrap_or_default()
	}
	/// Force the new free balance of a target account `who` to some new value `balance`.
	fn make_free_balance_be(
		who: &T::AccountId,
		new_balance: Self::Balance,
	) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
		// used by pallet-evm correct_and_deposit_fee
		let free = Self::free_balance(who);

		if let Some(decrease_by) = free.checked_sub(new_balance) {
			if let Ok(n) = Self::withdraw(
				who,
				decrease_by,
				WithdrawReasons::all(),
				ExistenceRequirement::AllowDeath,
			) {
				return SignedImbalance::Negative(n)
			}
		}

		if let Some(increase_by) = new_balance.checked_sub(free) {
			let p = Self::deposit_creating(who, increase_by);
			return SignedImbalance::Positive(p)
		}

		// no change to balance
		// either withdraw failed or free == new_balance
		return SignedImbalance::Positive(PositiveImbalance::default())
	}
	// unused staking/inflation related methods
	fn can_slash(_who: &T::AccountId, _value: Self::Balance) -> bool {
		false
	}
	fn slash(
		_who: &T::AccountId,
		_value: Self::Balance,
	) -> (Self::NegativeImbalance, Self::Balance) {
		(NegativeImbalance::default(), 0)
	}
	fn burn(_amount: Self::Balance) -> Self::PositiveImbalance {
		PositiveImbalance::default()
	}
	fn issue(_amount: Self::Balance) -> Self::NegativeImbalance {
		NegativeImbalance::default()
	}
}

/// Dual currency shim for staking
/// Maps stake operations to currency `S` and reward operations to currency `R`
pub struct DualStakingCurrency<T, R, S>(PhantomData<(T, R, S)>);

impl<T, R, S> Currency<T::AccountId> for DualStakingCurrency<T, R, S>
where
	T: Config,
	R: Currency<
		T::AccountId,
		Balance = Balance,
		NegativeImbalance = imbalances::NegativeImbalance<T>,
		PositiveImbalance = imbalances::PositiveImbalance<T>,
	>,
	S: Currency<
			T::AccountId,
			Balance = Balance,
			NegativeImbalance = pallet_balances::NegativeImbalance<T>,
			PositiveImbalance = pallet_balances::PositiveImbalance<T>,
		> + LockableCurrency<T::AccountId, Balance = Balance>,
{
	type Balance = Balance;
	type NegativeImbalance = imbalances::NegativeImbalance<T>;
	type PositiveImbalance = imbalances::PositiveImbalance<T>;
	// these functions proxy to `S` for staking currency inspection
	fn free_balance(who: &T::AccountId) -> Self::Balance {
		S::free_balance(who)
	}
	fn total_issuance() -> Self::Balance {
		S::total_issuance()
	}
	fn minimum_balance() -> Self::Balance {
		S::minimum_balance()
	}
	fn total_balance(who: &T::AccountId) -> Self::Balance {
		S::total_balance(who)
	}
	fn transfer(
		from: &T::AccountId,
		to: &T::AccountId,
		value: Self::Balance,
		req: ExistenceRequirement,
	) -> DispatchResult {
		S::transfer(from, to, value, req)
	}
	fn ensure_can_withdraw(
		who: &T::AccountId,
		amount: Self::Balance,
		reasons: WithdrawReasons,
		new_balance: Self::Balance,
	) -> DispatchResult {
		S::ensure_can_withdraw(who, amount, reasons, new_balance)
	}
	fn withdraw(
		who: &T::AccountId,
		value: Self::Balance,
		reasons: WithdrawReasons,
		req: ExistenceRequirement,
	) -> Result<Self::NegativeImbalance, DispatchError> {
		let n = S::withdraw(who, value, reasons, req)?;
		Ok(Self::NegativeImbalance::new(n.peek(), T::NativeAssetId::get()))
	}
	fn can_slash(who: &T::AccountId, value: Self::Balance) -> bool {
		S::can_slash(who, value)
	}
	fn slash(who: &T::AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
		let (n, b) = S::slash(who, value);
		(Self::NegativeImbalance::new(n.peek(), T::NativeAssetId::get()), b)
	}
	fn burn(amount: Self::Balance) -> Self::PositiveImbalance {
		Self::PositiveImbalance::new(S::burn(amount).peek(), T::NativeAssetId::get())
	}
	fn issue(amount: Self::Balance) -> Self::NegativeImbalance {
		R::issue(amount)
	}
	// these functions proxy to `R` for reward payouts
	fn deposit_into_existing(
		who: &T::AccountId,
		value: Self::Balance,
	) -> Result<Self::PositiveImbalance, DispatchError> {
		let tx_fee_pot_Account = &T::FeePotId::get().into_account_truncating();
		if R::free_balance(tx_fee_pot_Account) > value.into() {
			R::transfer(tx_fee_pot_Account, who, value, ExistenceRequirement::AllowDeath);
			Ok(PositiveImbalance::default())
		} else {
			R::deposit_into_existing(who, value)
		}
	}
	fn deposit_creating(who: &T::AccountId, value: Self::Balance) -> Self::PositiveImbalance {
		let tx_fee_pot_account = &T::FeePotId::get().into_account_truncating();
		if R::free_balance(tx_fee_pot_account) > value {
			R::transfer(tx_fee_pot_account, who, value, ExistenceRequirement::AllowDeath);
			PositiveImbalance::default()
		} else {
			Self::deposit_into_existing(who, value).unwrap_or_default()
		}
	}
	fn make_free_balance_be(
		who: &T::AccountId,
		new_balance: Self::Balance,
	) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
		R::make_free_balance_be(who, new_balance)
	}
}

impl<T, R, S> LockableCurrency<T::AccountId> for DualStakingCurrency<T, R, S>
where
	T: Config,
	R: Currency<
		T::AccountId,
		Balance = Balance,
		NegativeImbalance = imbalances::NegativeImbalance<T>,
		PositiveImbalance = imbalances::PositiveImbalance<T>,
	>,
	S: Currency<
			T::AccountId,
			Balance = Balance,
			NegativeImbalance = pallet_balances::NegativeImbalance<T>,
			PositiveImbalance = pallet_balances::PositiveImbalance<T>,
		> + LockableCurrency<T::AccountId, Balance = Balance>,
{
	type Moment = S::Moment;
	type MaxLocks = S::MaxLocks;
	fn set_lock(
		id: LockIdentifier,
		who: &T::AccountId,
		amount: Self::Balance,
		reasons: WithdrawReasons,
	) {
		S::set_lock(id, who, amount, reasons)
	}
	fn extend_lock(
		id: LockIdentifier,
		who: &T::AccountId,
		amount: Self::Balance,
		reasons: WithdrawReasons,
	) {
		S::extend_lock(id, who, amount, reasons)
	}
	fn remove_lock(id: LockIdentifier, who: &T::AccountId) {
		S::remove_lock(id, who)
	}
}

#[cfg(test)]
mod tests {
	use crate::mock::{test_ext, AssetId, AssetsExt, MockAccountId, Test};
	use frame_support::{assert_noop, assert_storage_noop, parameter_types};

	use super::*;

	const TEST_ASSET_ID: AssetId = 5;
	parameter_types! {
		pub const TestAssetId: AssetId = TEST_ASSET_ID;
	}
	type TestAssetCurrency = AssetCurrency<Test, TestAssetId>;

	#[test]
	fn deposit_creating() {
		let alice = 1 as MockAccountId;
		let bob = 2 as MockAccountId;
		test_ext()
			.with_asset(TEST_ASSET_ID, "TST", &[(alice, 1_000_000)])
			.build()
			.execute_with(|| {
				// new account
				let _ = TestAssetCurrency::deposit_creating(&bob, 500);
				assert_eq!(AssetsExt::balance(TEST_ASSET_ID, &bob), 500,);

				// existing account
				let _ = TestAssetCurrency::deposit_creating(&bob, 500);
				assert_eq!(AssetsExt::balance(TEST_ASSET_ID, &bob), 500 + 500);

				assert_eq!(AssetsExt::total_issuance(TEST_ASSET_ID), 1_000_000 + 500 + 500);
			});
	}

	#[test]
	fn withdraw() {
		let alice = 1 as MockAccountId;
		test_ext()
			.with_asset(TEST_ASSET_ID, "TST", &[(alice, 1_000_000)])
			.build()
			.execute_with(|| {
				let _ = TestAssetCurrency::withdraw(
					&alice,
					500,
					WithdrawReasons::all(),
					ExistenceRequirement::AllowDeath,
				);
				assert_eq!(AssetsExt::balance(TEST_ASSET_ID, &alice), 1_000_000 - 500,);
				assert_eq!(AssetsExt::total_issuance(TEST_ASSET_ID), 1_000_000 - 500);

				assert_noop!(
					TestAssetCurrency::withdraw(
						&alice,
						1_000_000,
						WithdrawReasons::all(),
						ExistenceRequirement::AllowDeath,
					),
					pallet_assets::Error::<Test>::BalanceLow
				);
				assert_eq!(AssetsExt::total_issuance(TEST_ASSET_ID), 1_000_000 - 500);
			});
	}

	#[test]
	fn make_free_balance_be() {
		let alice = 1 as MockAccountId;
		test_ext()
			.with_asset(TEST_ASSET_ID, "TST", &[(alice, 1_000_000)])
			.build()
			.execute_with(|| {
				let _ = TestAssetCurrency::make_free_balance_be(&alice, 999_500);
				assert_eq!(AssetsExt::balance(TEST_ASSET_ID, &alice), 999_500);
				assert_eq!(AssetsExt::total_issuance(TEST_ASSET_ID), 999_500);

				let _ = TestAssetCurrency::make_free_balance_be(&alice, 1_000_000);
				assert_eq!(AssetsExt::balance(TEST_ASSET_ID, &alice), 1_000_000);
				assert_eq!(AssetsExt::total_issuance(TEST_ASSET_ID), 1_000_000);

				assert_storage_noop!(TestAssetCurrency::make_free_balance_be(&alice, 1_000_000));
			});
	}
}
