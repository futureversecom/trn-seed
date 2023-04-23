#![cfg(test)]
use crate::{mock::*, *};
use frame_support::{
	assert_noop, assert_ok, error::BadOrigin, traits::tokens::fungibles::Transfer,
};
use hex_literal::hex;
use seed_primitives::Balance;
use seed_runtime::{
	impls::{ProxyPalletProvider, ProxyType},
	Inspect,
};

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

			// creation fails if not balance
			// assert_eq!(Assets::balance(MOCK_PAYMENT_ASSET_ID, &owner), 0);
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner), 0);
			assert_noop!(
				Futurepass::create(Origin::signed(owner), owner),
				pallet_balances::Error::<Test>::InsufficientBalance
			);

			// fund account (origin)
			// assert_ok!(Assets::transfer(Origin::signed(funder), MOCK_PAYMENT_ASSET_ID, owner,
			// FP_CREATION_RESERVE));
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE,
				false
			));
			// assert_eq!(Assets::balance(MOCK_PAYMENT_ASSET_ID, &owner), FP_CREATION_RESERVE);
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
