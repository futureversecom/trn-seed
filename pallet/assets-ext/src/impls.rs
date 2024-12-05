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

use frame_support::{
	ensure,
	pallet_prelude::DispatchResult,
	traits::{Currency, ExistenceRequirement, Get, SignedImbalance, WithdrawReasons},
};
use sp_runtime::{traits::Zero, DispatchError};
use sp_std::marker::PhantomData;

use frame_support::traits::{
	fungible,
	fungibles::{self, Inspect, Mutate, Unbalanced},
	tokens::{
		DepositConsequence, Fortitude, Precision, Preservation, Provenance, WithdrawConsequence,
	},
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

	fn active_issuance() -> Self::Balance {
		<pallet_assets::Pallet<T>>::active_issuance(U::get())
	}

	fn minimum_balance() -> Balance {
		<pallet_assets::Pallet<T>>::minimum_balance(U::get())
	}

	fn total_balance(who: &T::AccountId) -> Self::Balance {
		<pallet_assets::Pallet<T>>::total_balance(U::get(), who)
	}

	fn balance(who: &T::AccountId) -> Balance {
		<pallet_assets::Pallet<T>>::balance(U::get(), who)
	}

	fn reducible_balance(
		who: &T::AccountId,
		preservation: Preservation,
		force: Fortitude,
	) -> Balance {
		<pallet_assets::Pallet<T> as fungibles::Inspect<_>>::reducible_balance(
			U::get(),
			who,
			preservation,
			force,
		)
	}

	fn can_deposit(
		who: &T::AccountId,
		amount: Balance,
		provenance: Provenance,
	) -> DepositConsequence {
		<pallet_assets::Pallet<T>>::can_deposit(U::get(), who, amount, provenance)
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
		<pallet_assets::Pallet<T>>::reducible_balance(
			U::get(),
			who,
			Preservation::Expendable,
			Fortitude::Polite,
		)
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
		let preservation = match req {
			ExistenceRequirement::KeepAlive => Preservation::Preserve,
			ExistenceRequirement::AllowDeath => Preservation::Expendable,
		};
		<Pallet<T> as Mutate<T::AccountId>>::transfer(U::get(), from, to, value, preservation)
			.map(|_| ())
	}
	fn ensure_can_withdraw(
		who: &T::AccountId,
		amount: Self::Balance,
		_reasons: WithdrawReasons,
		new_balance: Self::Balance,
	) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
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
		req: ExistenceRequirement,
	) -> Result<Self::NegativeImbalance, DispatchError> {
		let preservation = match req {
			ExistenceRequirement::KeepAlive => Preservation::Preserve,
			ExistenceRequirement::AllowDeath => Preservation::Expendable,
		};
		// used by pallet-transaction payment & pallet-evm
		<pallet_assets::Pallet<T>>::decrease_balance(
			U::get(),
			who,
			value,
			Precision::Exact,
			preservation,
			Fortitude::Polite,
		)?;

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
			return Ok(PositiveImbalance::new(0, U::get()));
		}
		<pallet_assets::Pallet<T>>::increase_balance(U::get(), who, value, Precision::Exact)?;
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
				return SignedImbalance::Negative(n);
			}
		}

		if let Some(increase_by) = new_balance.checked_sub(free) {
			let p = Self::deposit_creating(who, increase_by);
			return SignedImbalance::Positive(p);
		}

		// no change to balance
		// either withdraw failed or free == new_balance
		SignedImbalance::Positive(PositiveImbalance::default())
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::{test_ext, AssetsExt, Test};
	use seed_pallet_common::test_prelude::*;

	const TEST_ASSET_ID: AssetId = 5;
	parameter_types! {
		pub const TestAssetId: AssetId = TEST_ASSET_ID;
	}
	type TestAssetCurrency = AssetCurrency<Test, TestAssetId>;

	#[test]
	fn deposit_creating() {
		test_ext()
			.with_asset(TEST_ASSET_ID, "TST", &[(alice(), 1_000_000)])
			.build()
			.execute_with(|| {
				// new account
				let _ = TestAssetCurrency::deposit_creating(&bob(), 500);
				assert_eq!(AssetsExt::balance(TEST_ASSET_ID, &bob()), 500,);

				// existing account
				let _ = TestAssetCurrency::deposit_creating(&bob(), 500);
				assert_eq!(AssetsExt::balance(TEST_ASSET_ID, &bob()), 500 + 500);

				assert_eq!(AssetsExt::total_issuance(TEST_ASSET_ID), 1_000_000 + 500 + 500);
			});
	}

	#[test]
	fn withdraw() {
		test_ext()
			.with_asset(TEST_ASSET_ID, "TST", &[(alice(), 1_000_000)])
			.build()
			.execute_with(|| {
				let _ = TestAssetCurrency::withdraw(
					&alice(),
					500,
					WithdrawReasons::all(),
					ExistenceRequirement::AllowDeath,
				);
				assert_eq!(AssetsExt::balance(TEST_ASSET_ID, &alice()), 1_000_000 - 500,);
				assert_eq!(AssetsExt::total_issuance(TEST_ASSET_ID), 1_000_000 - 500);

				assert_noop!(
					TestAssetCurrency::withdraw(
						&alice(),
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
		test_ext()
			.with_asset(TEST_ASSET_ID, "TST", &[(alice(), 1_000_000)])
			.build()
			.execute_with(|| {
				let _ = TestAssetCurrency::make_free_balance_be(&alice(), 999_500);
				assert_eq!(AssetsExt::balance(TEST_ASSET_ID, &alice()), 999_500);
				assert_eq!(AssetsExt::total_issuance(TEST_ASSET_ID), 999_500);

				let _ = TestAssetCurrency::make_free_balance_be(&alice(), 1_000_000);
				assert_eq!(AssetsExt::balance(TEST_ASSET_ID, &alice()), 1_000_000);
				assert_eq!(AssetsExt::total_issuance(TEST_ASSET_ID), 1_000_000);

				assert_storage_noop!(TestAssetCurrency::make_free_balance_be(&alice(), 1_000_000));
			});
	}
}
