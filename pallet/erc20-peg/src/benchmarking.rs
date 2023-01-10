// /* Copyright 2019-2021 Centrality Investments Limited
// *
// * Licensed under the LGPL, Version 3.0 (the "License");
// * you may not use this file except in compliance with the License.
// * Unless required by applicable law or agreed to in writing, software
// * distributed under the License is distributed on an "AS IS" BASIS,
// * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// * See the License for the specific language governing permissions and
// * limitations under the License.
// * You may obtain a copy of the License at the root of this project source code,
// * or at:
// * https://centrality.ai/licenses/gplv3.txt
// * https://centrality.ai/licenses/lgplv3.txt
// */
use super::*;

use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, traits::fungibles::Inspect};
use frame_system::RawOrigin;
use sp_runtime::traits::StaticLookup;

use crate::Pallet as Erc20Peg;

/// This is a helper function to get an account.
pub fn account<T: Config>(name: &'static str) -> T::AccountId {
	bench_account(name, 0, 0)
}

pub fn origin<T: Config>(acc: &T::AccountId) -> RawOrigin<T::AccountId> {
	RawOrigin::Signed(acc.clone())
}

benchmarks! {
	activate_deposits {
		let activate = true;
		// Sanity check
		assert_eq!(DepositsActive::get(), !activate);

	}: _(RawOrigin::Root, activate)
	verify {
		assert_eq!(DepositsActive::get(), activate);
	}

	activate_withdrawals {
		let activate = true;
		// Sanity check
		assert_eq!(WithdrawalsActive::get(), !activate);

	}: _(RawOrigin::Root, activate)
	verify {
		assert_eq!(WithdrawalsActive::get(), activate);
	}

	withdraw {
		let alice = account::<T>("Alice");
		let asset_id = 100u32.into();
		let alice_balance: Balance = 10000u32.into();
		assert_ok!(Erc20Peg::<T>::activate_withdrawals(RawOrigin::Root.into(), true));
		assert_ok!(Erc20Peg::<T>::activate_deposits(RawOrigin::Root.into(), true));

		// Activate asset_id
		let source = account::<T>("Source").into();
		let token_address = account::<T>("TokenAddress").into();
		let amount = 10000u32.into();
		let beneficiary = account::<T>("Beneficiary").into();
		let data = ethabi::encode(&[Token::Address(token_address), Token::Uint(amount), Token::Address(beneficiary)]);
		assert_ok!(Erc20Peg::<T>::on_event(&source, &data));
		assert_ok!(T::MultiCurrency::mint_into(asset_id, &alice, alice_balance));

		let withdraw_amount: Balance = 100u32.into();
		let beneficiary = account::<T>("Beneficiary").into();

	}: _(origin::<T>(&alice), asset_id, withdraw_amount, beneficiary)
	verify {
		let expected_balance = alice_balance - withdraw_amount;
		let actual_balance = T::MultiCurrency::balance(asset_id, &alice);
		assert_eq!(actual_balance, expected_balance);
	}

	set_contract_address {
		let alice: EthAddress = account::<T>("Alice").into();
		// Sanity check
		assert_ne!(Erc20Peg::<T>::contract_address(), alice);

	}: _(RawOrigin::Root, alice)
	verify {
		assert_eq!(Erc20Peg::<T>::contract_address(), alice);
	}

	set_erc20_meta {
		let alice: EthAddress = account::<T>("Alice").into();
		let details: Vec<(EthAddress, Vec<u8>, u8)> = vec![(alice, vec![0], 100)];
		// Sanity check
		assert_eq!(Erc20Meta::get(details[0].0.clone()), None);

	}: _(RawOrigin::Root, details.clone())
	verify {
		assert_eq!(Erc20Meta::get(details[0].0.clone()), Some((details[0].1.clone(), details[0].2)));
	}

	set_payment_delay {
		let asset_id: AssetId = 0x1A4u32.into();
		let min_balance: Balance = 10u32.into();
		let delay: T::BlockNumber = 20u32.into();
		// Sanity check
		assert_eq!(PaymentDelay::<T>::get(asset_id), None);

	}: _(RawOrigin::Root, asset_id.clone(), min_balance.clone(), delay.clone())
	verify {
		assert_eq!(PaymentDelay::<T>::get(asset_id), Some((min_balance, delay)));
	}
}

impl_benchmark_test_suite!(Erc20Peg, crate::mock::new_test_ext(), crate::mock::Test,);
