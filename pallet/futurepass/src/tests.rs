#![cfg(test)]
use super::*;
use crate::mock::*;
use frame_support::{
	assert_err, assert_noop, assert_ok, error::BadOrigin, traits::tokens::fungibles::Transfer,
};
use hex_literal::hex;

use seed_primitives::{AssetId, Balance};
use seed_runtime::{
	impls::{ProxyPalletProvider, ProxyType},
	Inspect,
};

type MockCall = crate::mock::Call;

const FP_CREATION_RESERVE: Balance = 148 + 126 * 1;
const FP_DELIGATE_RESERVE: Balance = 126 * 1;

#[test]
fn create_futurepass_by_owner() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);

			// assert the futurepass is not created yet for the owner account
			assert_eq!(Holders::<Test>::contains_key(&owner), false);

			// creation fails if not has sufficient balance
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner), 0);
			assert_noop!(
				Futurepass::create(Origin::signed(owner), owner),
				pallet_balances::Error::<Test>::InsufficientBalance
			);

			// fund owner
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE,
				false
			));
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner), FP_CREATION_RESERVE);
			// create futurepass account
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			// assert event (account creation)
			System::assert_has_event(
				Event::<Test>::FuturepassCreated {
					futurepass: AccountId::from(hex!("ffffffff00000000000000000000000000000001")),
					delegate: owner,
				}
				.into(),
			);
			// Check if the futurepass account is created and associated with the delegate account
			let futurepass = Holders::<Test>::get(&owner).unwrap();
			assert!(<Test as Config>::Proxy::exists(&futurepass, &owner, Some(ProxyType::Any)));

			// try to create futurepass for the owner again should result error
			assert_noop!(
				Futurepass::create(Origin::signed(owner), owner),
				Error::<Test>::AccountAlreadyRegistered
			);
		});
}

#[test]
fn create_futurepass_by_other() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let other = create_account(3);

			// fund other
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&other,
				FP_CREATION_RESERVE,
				false
			));
			// check balances
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &other), FP_CREATION_RESERVE);
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner), 0);
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(other), owner));
			// assert event (account creation)
			System::assert_has_event(
				Event::<Test>::FuturepassCreated {
					futurepass: AccountId::from(hex!("ffffffff00000000000000000000000000000001")),
					delegate: owner,
				}
				.into(),
			);
			// Check if the futurepass account is created and associated with the owner account
			let futurepass = Holders::<Test>::get(&owner).unwrap();
			assert!(<Test as Config>::Proxy::exists(&futurepass, &owner, Some(ProxyType::Any)));

			// check that FP_CREATION_RESERVE is paid by the caller(other)
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &other), 0);
		});
}

#[test]
fn register_delegate_by_owner_works() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let delegate = create_account(3);

			// fund owner
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE,
				false
			));
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			// check delegate is not a delegate yet
			assert_eq!(
				<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(ProxyType::Any)),
				false
			);

			// register delegate
			// owner needs another FP_DELIGATE_RESERVE for this
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_DELIGATE_RESERVE,
				false
			));
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner), FP_DELIGATE_RESERVE);
			assert_ok!(Futurepass::register_delegate(
				Origin::signed(owner),
				futurepass,
				delegate,
				ProxyType::Any
			));
			// assert event
			System::assert_has_event(
				Event::<Test>::DelegateRegistered {
					futurepass,
					delegate,
					proxy_type: ProxyType::Any,
				}
				.into(),
			);

			// check delegate is a proxy of futurepass
			assert!(<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(ProxyType::Any)));
		});
}

#[test]
fn register_delegate_failures() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let delegate1 = create_account(3);
			let delegate2 = create_account(4);
			let other = create_account(5);

			// fund owner
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE,
				false
			));
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			// Try to register_delegate to non existent FP
			assert_noop!(
				Futurepass::register_delegate(
					Origin::signed(owner),
					create_random(),
					delegate1,
					ProxyType::Any
				),
				Error::<Test>::NotFuturepassOwner
			);
			// register_delegate by owner without sufficient reserve balance
			assert_noop!(
				Futurepass::register_delegate(
					Origin::signed(owner),
					futurepass,
					delegate1,
					ProxyType::Any
				),
				pallet_balances::Error::<Test>::InsufficientBalance
			);
			// fund the owner and other
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_DELIGATE_RESERVE,
				false
			));
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&other,
				FP_DELIGATE_RESERVE,
				false
			));
			// Try to register_delegate by other (non owner)
			assert_noop!(
				Futurepass::register_delegate(
					Origin::signed(other),
					futurepass,
					delegate1,
					ProxyType::Any
				),
				Error::<Test>::NotFuturepassOwner
			);
			// register_delegate with proxy_type != ProxyType::Any
			assert_noop!(
				Futurepass::register_delegate(
					Origin::signed(owner),
					futurepass,
					delegate1,
					ProxyType::NonTransfer
				),
				Error::<Test>::PermissionDenied
			);
			// register delegate by owner successfully
			assert_ok!(Futurepass::register_delegate(
				Origin::signed(owner),
				futurepass,
				delegate1,
				ProxyType::Any
			));
			assert!(<Test as Config>::Proxy::exists(&futurepass, &delegate1, Some(ProxyType::Any)));

			// try to register the same delegate1 again should fail
			assert_noop!(
				Futurepass::register_delegate(
					Origin::signed(owner),
					futurepass,
					delegate1,
					ProxyType::Any
				),
				Error::<Test>::DelegateAlreadyExists
			);
			// register_delegate by another delegate should fail - NOTE: for V1
			// fund delegate1
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&delegate1,
				FP_DELIGATE_RESERVE,
				false
			));
			assert_noop!(
				Futurepass::register_delegate(
					Origin::signed(delegate1),
					futurepass,
					delegate2,
					ProxyType::Any
				),
				Error::<Test>::NotFuturepassOwner
			);
		});
}

#[test]
fn unregister_delegate_by_owner_works() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let delegate = create_account(3);

			// fund owner
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELIGATE_RESERVE,
				false
			));
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();
			// register delegate
			assert_ok!(Futurepass::register_delegate(
				Origin::signed(owner),
				futurepass,
				delegate,
				ProxyType::Any
			));
			assert!(<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(ProxyType::Any)));

			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner), 0);
			// unregister_delegate
			assert_ok!(Futurepass::unregister_delegate(
				Origin::signed(owner),
				futurepass,
				delegate
			));
			// assert event
			System::assert_has_event(
				Event::<Test>::DelegateUnregistered { futurepass, delegate }.into(),
			);

			// check the reserved amount has been received by the caller. i.e the owner
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner), FP_DELIGATE_RESERVE);

			// check delegate is not a proxy of futurepass
			assert_eq!(
				<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(ProxyType::Any)),
				false
			);
		});
}

#[test]
fn unregister_delegate_by_the_delegate_works() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let delegate = create_account(3);

			// fund owner
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELIGATE_RESERVE,
				false
			));
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();
			// register delegate
			assert_ok!(Futurepass::register_delegate(
				Origin::signed(owner),
				futurepass,
				delegate,
				ProxyType::Any
			));
			assert!(<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(ProxyType::Any)));

			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &delegate), 0);
			// unregister_delegate
			assert_ok!(Futurepass::unregister_delegate(
				Origin::signed(delegate),
				futurepass,
				delegate
			));
			// assert event
			System::assert_has_event(
				Event::<Test>::DelegateUnregistered { futurepass, delegate }.into(),
			);
			// check the reserved amount has been received by the caller. i.e the delegate
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &delegate), FP_DELIGATE_RESERVE);

			// check delegate is not a proxy of futurepass
			assert_eq!(
				<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(ProxyType::Any)),
				false
			);
		});
}

#[test]
fn unregister_delegate_failures() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let delegate1 = create_account(3);
			let delegate2 = create_account(4);
			let other = create_account(5);

			// fund owner
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + 2 * FP_DELIGATE_RESERVE,
				false
			));
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();
			// register delegate
			assert_ok!(Futurepass::register_delegate(
				Origin::signed(owner),
				futurepass,
				delegate1,
				ProxyType::Any
			));
			assert!(<Test as Config>::Proxy::exists(&futurepass, &delegate1, Some(ProxyType::Any)));
			assert_ok!(Futurepass::register_delegate(
				Origin::signed(owner),
				futurepass,
				delegate2,
				ProxyType::Any
			));
			assert!(<Test as Config>::Proxy::exists(&futurepass, &delegate2, Some(ProxyType::Any)));

			// unregister_delegate on a non existent futurepass fails
			assert_noop!(
				Futurepass::unregister_delegate(Origin::signed(owner), create_random(), delegate1),
				Error::<Test>::PermissionDenied
			);
			// unregister_delegate on a non delegate fails
			assert_noop!(
				Futurepass::unregister_delegate(Origin::signed(owner), futurepass, create_random()),
				Error::<Test>::DelegateNotRegistered
			);
			// unregister_delegate by other(non(owner | delegate)) fails
			assert_noop!(
				Futurepass::unregister_delegate(Origin::signed(other), futurepass, delegate1),
				Error::<Test>::PermissionDenied
			);
			// unregister_delegate by another delegate fails
			assert_noop!(
				Futurepass::unregister_delegate(Origin::signed(delegate2), futurepass, delegate1),
				Error::<Test>::PermissionDenied
			);
			// owner can not unregister by itself
			assert_noop!(
				Futurepass::unregister_delegate(Origin::signed(owner), futurepass, owner),
				Error::<Test>::OwnerCannotUnregister
			);
		});
}

#[test]
fn transfer_futurepass_works() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let delegate = create_account(3);
			let other = create_account(4);

			// fund owner
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELIGATE_RESERVE,
				false
			));
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();
			// register delegate
			assert_ok!(Futurepass::register_delegate(
				Origin::signed(owner),
				futurepass,
				delegate,
				ProxyType::Any
			));
			// check delegate is a proxy of futurepass
			assert!(<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(ProxyType::Any)));

			// transfer the ownership to other
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner), 0);
			assert_ok!(Futurepass::transfer_futurepass(Origin::signed(owner), other));
			// assert event
			System::assert_has_event(
				Event::<Test>::FuturepassTransferred {
					old_owner: owner,
					new_owner: other,
					futurepass,
				}
				.into(),
			);
			// owner should be other now
			assert_eq!(Holders::<Test>::get(&other), Some(futurepass));
			assert_eq!(Holders::<Test>::get(&owner), None);
			// only the new owner(i.e other) should be a delegate
			assert!(<Test as Config>::Proxy::exists(&futurepass, &other, Some(ProxyType::Any)));
			assert_eq!(
				<Test as Config>::Proxy::exists(&futurepass, &owner, Some(ProxyType::Any)),
				false
			);
			assert_eq!(
				<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(ProxyType::Any)),
				false
			);
			// caller(the owner) should receive the reserved balance diff
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner), FP_DELIGATE_RESERVE);
		});
}

#[test]
fn transfer_futurepass_failures() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let delegate = create_account(3);
			let owner2 = create_account(4);

			// fund owner
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELIGATE_RESERVE,
				false
			));
			// create FP for owner
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();
			// register delegate
			assert_ok!(Futurepass::register_delegate(
				Origin::signed(owner),
				futurepass,
				delegate,
				ProxyType::Any
			));
			// check delegate is a proxy of futurepass
			assert!(<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(ProxyType::Any)));

			// fund owner2
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner2,
				FP_CREATION_RESERVE,
				false
			));
			// create FP for owner2
			assert_ok!(Futurepass::create(Origin::signed(owner2), owner2));
			let futurepass2 = Holders::<Test>::get(&owner2).unwrap();

			// call transfer_futurepass by other than owner should fail
			assert_noop!(
				Futurepass::transfer_futurepass(Origin::signed(create_random()), create_random()),
				Error::<Test>::NotFuturepassOwner
			);
			// call transfer_futurepass for another futurepass owner should fail
			assert_noop!(
				Futurepass::transfer_futurepass(Origin::signed(owner), owner2),
				Error::<Test>::AccountAlreadyRegistered
			);
		});
}

#[test]
fn proxy_extrinsic_simple_transfer_works() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let delegate = create_account(3);
			let other = create_account(4);

			// fund owner
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELIGATE_RESERVE,
				false
			));
			// create FP for owner
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();
			// register delegate
			assert_ok!(Futurepass::register_delegate(
				Origin::signed(owner),
				futurepass,
				delegate,
				ProxyType::Any
			));

			// fund futurepass with some tokens
			let fund_amount: Balance = 1000;
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&futurepass,
				fund_amount,
				false
			));
			assert_eq!(
				AssetsExt::reducible_balance(MOCK_NATIVE_ASSET_ID, &futurepass, false),
				fund_amount
			);
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &other), 0);

			// transfer other via proxy_extrinsic
			let transfer_amount: Balance = 100;
			let inner_call = Box::new(MockCall::Balances(pallet_balances::Call::transfer {
				dest: other,
				value: transfer_amount,
			}));
			// call proxy_extrinsic by owner
			assert_ok!(Futurepass::proxy_extrinsic(
				Origin::signed(owner),
				futurepass,
				inner_call.clone(),
			));
			// assert event ProxyExecuted
			System::assert_has_event(Event::<Test>::ProxyExecuted { result: Ok(()) }.into());
			//check balances
			assert_eq!(
				AssetsExt::reducible_balance(MOCK_NATIVE_ASSET_ID, &futurepass, false),
				fund_amount - transfer_amount
			);
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &other), transfer_amount);

			// call proxy_extrinsic by delegate
			assert_ok!(Futurepass::proxy_extrinsic(
				Origin::signed(delegate),
				futurepass,
				inner_call,
			));
			//check balances
			assert_eq!(
				AssetsExt::reducible_balance(MOCK_NATIVE_ASSET_ID, &futurepass, false),
				fund_amount - 2 * transfer_amount
			);
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &other), 2 * transfer_amount);
		});
}

#[test]
fn proxy_extrinsic_non_transfer_call_works() {
	let funder = create_account(1);
	let endowed = [(funder, 2_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let delegate = create_account(3);
			let other = create_account(4);

			// fund owner
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELIGATE_RESERVE,
				false
			));
			// create FP for owner
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();
			// register delegate
			assert_ok!(Futurepass::register_delegate(
				Origin::signed(owner),
				futurepass,
				delegate,
				ProxyType::Any
			));

			// fund futurepass with some tokens
			let fund_amount: Balance = 1_500_000;
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&futurepass,
				fund_amount,
				false
			));

			let asset_id = 5;
			let inner_call = Box::new(MockCall::Assets(pallet_assets::Call::create {
				id: asset_id,
				admin: futurepass,
				min_balance: 1,
			}));
			// call proxy_extrinsic
			assert_ok!(Futurepass::proxy_extrinsic(Origin::signed(owner), futurepass, inner_call,));
			// assert event (asset creation)
			System::assert_has_event(
				pallet_assets::Event::<Test>::Created {
					asset_id,
					creator: futurepass,
					owner: futurepass,
				}
				.into(),
			);
		});
}

#[test]
fn proxy_extrinsic_failures() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let delegate = create_account(3);
			let other = create_account(4);

			// fund owner
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELIGATE_RESERVE,
				false
			));
			// create FP for owner
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();
			// register delegate
			assert_ok!(Futurepass::register_delegate(
				Origin::signed(owner),
				futurepass,
				delegate,
				ProxyType::Any
			));

			// fund futurepass with some tokens
			let fund_amount: Balance = 1000;
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&futurepass,
				fund_amount,
				false
			));
			assert_eq!(
				AssetsExt::reducible_balance(MOCK_NATIVE_ASSET_ID, &futurepass, false),
				fund_amount
			);
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &other), 0);

			let transfer_amount: Balance = 100;
			let inner_call = Box::new(MockCall::Balances(pallet_balances::Call::transfer {
				dest: other,
				value: transfer_amount,
			}));

			// call proxy_extrinsic by non (owner | delegate) fails
			System::reset_events();
			assert_err!(
				Futurepass::proxy_extrinsic(Origin::signed(other), futurepass, inner_call.clone()),
				pallet_proxy::Error::<Test>::NotProxy
			);
			// assert event (ProxyExecuted with error)
			System::assert_has_event(
				Event::<Test>::ProxyExecuted {
					result: Err(pallet_proxy::Error::<Test>::NotProxy.into()),
				}
				.into(),
			);

			//check balances
			assert_eq!(
				AssetsExt::reducible_balance(MOCK_NATIVE_ASSET_ID, &futurepass, false),
				fund_amount
			);
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &other), 0);

			// call proxy_extrinsic for a non futurepass account fails
			assert_err!(
				Futurepass::proxy_extrinsic(
					Origin::signed(other),
					create_random(),
					inner_call.clone()
				),
				pallet_proxy::Error::<Test>::NotProxy
			);
			System::assert_has_event(
				Event::<Test>::ProxyExecuted {
					result: Err(pallet_proxy::Error::<Test>::NotProxy.into()),
				}
				.into(),
			);

			// proxy_extrinsic does not care about wrapped internal call failure. It's task is to
			// only dispatch the internal call
			let futurpass_balance: Balance =
				AssetsExt::reducible_balance(MOCK_NATIVE_ASSET_ID, &futurepass, false);
			let inner_call = Box::new(MockCall::Balances(pallet_balances::Call::transfer {
				dest: other,
				value: futurpass_balance + 1,
			}));

			// call proxy_extrinsic by owner
			System::reset_events();
			assert_ok!(Futurepass::proxy_extrinsic(Origin::signed(owner), futurepass, inner_call,));
			// assert event ProxyExecuted
			System::assert_has_event(Event::<Test>::ProxyExecuted { result: Ok(()) }.into());
			// assert event pallet_proxy::ProxyExecuted with the error
			System::assert_has_event(
				pallet_proxy::Event::<Test>::ProxyExecuted {
					result: Err(pallet_balances::Error::<Test>::InsufficientBalance.into()),
				}
				.into(),
			);
		});
}
