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

use super::*;

use frame_benchmarking::{account as bench_account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, traits::fungibles::Inspect};
use frame_system::RawOrigin;

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
		assert_eq!(DepositsActive::<T>::get(), !activate);

	}: _(RawOrigin::Root, activate)
	verify {
		assert_eq!(DepositsActive::<T>::get(), activate);
	}

	activate_withdrawals {
		let activate = true;
		// Sanity check
		assert_eq!(WithdrawalsActive::<T>::get(), !activate);

	}: _(RawOrigin::Root, activate)
	verify {
		assert_eq!(WithdrawalsActive::<T>::get(), activate);
	}

	activate_deposits_delay {
		let activate = false;
		// Sanity check
		assert_eq!(DepositsDelayActive::<T>::get(), !activate);

	}: _(RawOrigin::Root, activate)
	verify {
		assert_eq!(DepositsDelayActive::<T>::get(), activate);
	}

	activate_withdrawals_delay {
		let activate = false;
		// Sanity check
		assert_eq!(WithdrawalsDelayActive::<T>::get(), !activate);

	}: _(RawOrigin::Root, activate)
	verify {
		assert_eq!(WithdrawalsDelayActive::<T>::get(), activate);
	}

	withdraw {
		let alice = account::<T>("Alice");
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

		// This is a hack. Getting the generated AssetId is pretty hard so this is a workaround.
		let asset_id = AssetIdToErc20::<T>::iter_keys().next().unwrap();
		assert_ok!(T::MultiCurrency::mint_into(asset_id, &alice, alice_balance));

		let withdraw_amount: Balance = 100u32.into();
		let beneficiary = account::<T>("Beneficiary").into();

	}: _(origin::<T>(&alice), asset_id, withdraw_amount, beneficiary)
	verify {
		let expected_balance = alice_balance - withdraw_amount;
		let actual_balance = T::MultiCurrency::balance(asset_id, &alice);
		assert_eq!(actual_balance, expected_balance);
	}

	claim_delayed_payment {
		let alice = account::<T>("Alice");
		let alice_balance: Balance = 10000u32.into();
		let delay: BlockNumberFor<T> = 1000u32.into();
		let withdraw_amount: Balance = 100u32.into();

		assert_ok!(Erc20Peg::<T>::activate_withdrawals(RawOrigin::Root.into(), true));
		assert_ok!(Erc20Peg::<T>::activate_deposits(RawOrigin::Root.into(), true));
		assert_ok!(Erc20Peg::<T>::activate_withdrawals_delay(RawOrigin::Root.into(), true));

		let source = account::<T>("Source").into();
		let token_address = account::<T>("TokenAddress").into();
		let amount = 10000u32.into();
		let beneficiary = account::<T>("Beneficiary").into();
		let data = ethabi::encode(&[Token::Address(token_address), Token::Uint(amount), Token::Address(beneficiary)]);
		assert_ok!(Erc20Peg::<T>::on_event(&source, &data));

		let asset_id = AssetIdToErc20::<T>::iter_keys().next().unwrap();
		let eth_address = Erc20ToAssetId::<T>::iter_keys().next().unwrap();

		assert_ok!(T::MultiCurrency::mint_into(asset_id, &alice, alice_balance));
		assert_ok!(Erc20Peg::<T>::set_payment_delay(RawOrigin::Root.into(), asset_id, withdraw_amount, delay));

		let delayed_payment_id = NextDelayedPaymentId::<T>::get();
		let payment_block = frame_system::Pallet::<T>::block_number() + delay;

		assert_ok!(Erc20Peg::<T>::withdraw(origin::<T>(&alice).into(), asset_id, withdraw_amount, beneficiary));

		let message = WithdrawMessage {
			token_address: eth_address,
			amount: withdraw_amount.into(),
			beneficiary,
		};

		assert_eq!(DelayedPaymentSchedule::<T>::get(payment_block), vec![delayed_payment_id]);
		assert_eq!(
			DelayedPayments::<T>::get(delayed_payment_id),
			Some(PendingPayment::Withdrawal(message))
		);
	}: _(RawOrigin::Root, payment_block, delayed_payment_id)
	verify {
		assert_eq!(
			DelayedPaymentSchedule::<T>::get(payment_block),
			vec![] as Vec<DelayedPaymentId>
		);
		assert!(DelayedPayments::<T>::get(delayed_payment_id).is_none());
	}


	set_erc20_peg_address {
		let alice: EthAddress = account::<T>("Alice").into();
		// Sanity check
		assert_ne!(ContractAddress::<T>::get(), alice);

	}: _(RawOrigin::Root, alice)
	verify {
		assert_eq!(ContractAddress::<T>::get(), alice);
	}

	set_root_peg_address {
		let alice: EthAddress = account::<T>("Alice").into();
		// Sanity check
		assert_ne!(RootPegContractAddress::<T>::get(), alice);
	}: _(RawOrigin::Root, alice)
	verify {
		assert_eq!(RootPegContractAddress::<T>::get(), alice);
	}

	set_erc20_asset_map {
		let asset_id: AssetId = 12;
		let token_address: H160 = H160::from_low_u64_be(13);
		// Sanity check
		assert!(Erc20ToAssetId::<T>::get(token_address).is_none());
		assert!(AssetIdToErc20::<T>::get(asset_id).is_none());
	}: _(RawOrigin::Root, asset_id, token_address)
	verify {
		assert_eq!(Erc20ToAssetId::<T>::get(token_address).unwrap(), asset_id);
		assert_eq!(AssetIdToErc20::<T>::get(asset_id).unwrap(), token_address);
	}

	set_erc20_meta {
		let alice: EthAddress = account::<T>("Alice").into();
		let details: Vec<(EthAddress, BoundedVec<u8, T::StringLimit>, u8)> = vec![(alice, BoundedVec::truncate_from(vec![0]), 100)];
		// Sanity check
		assert_eq!(Erc20Meta::<T>::get(details[0].0.clone()), None);

	}: _(RawOrigin::Root, details.clone())
	verify {
		assert_eq!(Erc20Meta::<T>::get(details[0].0.clone()), Some((details[0].1.clone(), details[0].2)));
	}

	set_payment_delay {
		let asset_id: AssetId = 0x1A4u32.into();
		let min_balance: Balance = 10u32.into();
		let delay: BlockNumberFor<T> = 20u32.into();
		// Sanity check
		assert_eq!(PaymentDelay::<T>::get(asset_id), None);

	}: _(RawOrigin::Root, asset_id.clone(), min_balance.clone(), delay.clone())
	verify {
		assert_eq!(PaymentDelay::<T>::get(asset_id), Some((min_balance, delay)));
	}
}

impl_benchmark_test_suite!(
	Erc20Peg,
	seed_primitives::test_utils::TestExt::<crate::mock::Test>::default().build(),
	crate::mock::Test
);
