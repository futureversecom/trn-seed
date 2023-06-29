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

#![cfg(test)]
use super::*;
use crate::mock::*;
use frame_support::{
	assert_err, assert_noop, assert_ok,
	traits::tokens::fungibles::{Mutate, Transfer},
};
use hex_literal::hex;
use seed_primitives::{AccountId, AssetId, Balance};
use seed_runtime::{impls::ProxyType, Inspect};

use pallet_nft::CrossChainCompatibility;
use seed_primitives::{CollectionUuid, MetadataScheme};

type MockCall = crate::mock::Call;

const FP_CREATION_RESERVE: Balance = 148 + 126; // ProxyDepositBase + ProxyDepositFactor * 1(num of delegates)
const FP_DELEGATE_RESERVE: Balance = 126 * 1; // ProxyDepositFactor * 1(num of delegates)

fn transfer_funds(asset_id: AssetId, source: &AccountId, destination: &AccountId, amount: Balance) {
	assert_ok!(AssetsExt::transfer(asset_id, &source, &destination, amount, false));
}

fn setup_collection(owner: &AccountId) -> CollectionUuid {
	let collection_id = Nft::next_collection_uuid().unwrap();
	let collection_name = b"test-collection".to_vec();
	let metadata_scheme = MetadataScheme::try_from(b"<CID>".as_slice()).unwrap();
	assert_ok!(Nft::create_collection(
		Some(owner.clone()).into(),                 // owner
		BoundedVec::truncate_from(collection_name), // name
		0,                                          // initial_issuance
		None,                                       // max_issuance
		None,                                       // token_owner
		metadata_scheme,                            // metadata_scheme
		None,                                       // royalties_schedule
		CrossChainCompatibility::default(),         // origin_chain
	));
	collection_id
}

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
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &owner, FP_CREATION_RESERVE);
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
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &other, FP_CREATION_RESERVE);
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
			let (signer, delegate) = create_random_pair();
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			// fund owner
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &owner, FP_CREATION_RESERVE);
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			// check delegate is not a delegate yet
			assert_eq!(
				<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(ProxyType::Any)),
				false
			);

			// register delegate
			// owner needs another FP_DELEGATE_RESERVE for this
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &owner, FP_DELEGATE_RESERVE);
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner), FP_DELEGATE_RESERVE);

			let signature = signer
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate,
						&proxy_type,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;
			assert_ok!(Futurepass::register_delegate_with_signature(
				Origin::signed(owner),
				futurepass,
				delegate,
				proxy_type,
				deadline,
				signature,
			));
			// assert event
			System::assert_has_event(
				Event::<Test>::DelegateRegistered { futurepass, delegate, proxy_type }.into(),
			);

			// check delegate is a proxy of futurepass
			assert!(<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(ProxyType::Any)));
		});
}

#[test]
fn register_delegate_by_non_delegate_fails() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let delegate1 = create_account(3);
			let other = create_account(5);
			let deadline = 200;

			// fund owner
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &owner, FP_CREATION_RESERVE);
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();
			// fund the other
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &other, FP_DELEGATE_RESERVE);
			// Try to register_delegate by other (non owner)
			assert_noop!(
				Futurepass::register_delegate_with_signature(
					Origin::signed(other),
					futurepass,
					delegate1,
					ProxyType::Any,
					deadline,
					[0u8; 65],
				),
				Error::<Test>::NotFuturepassOwner
			);
		});
}

#[test]
fn register_delegate_with_not_allowed_proxy_type_fails() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let delegate1 = create_account(3);
			let deadline = 200;

			// fund owner
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &owner, FP_CREATION_RESERVE);
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();
			// fund the owner
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &owner, FP_DELEGATE_RESERVE);
			// register_delegate with proxy_type != ProxyType::Any
			assert_noop!(
				Futurepass::register_delegate_with_signature(
					Origin::signed(owner),
					futurepass,
					delegate1,
					ProxyType::NonTransfer,
					deadline,
					[0u8; 65],
				),
				Error::<Test>::PermissionDenied
			);
		});
}

#[test]
fn register_delegate_fails_if_deadline_expired() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.with_block_number(201) // Note: block number is 201 - which causes deadline to be expired
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let delegate = create_account(3);
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			// fund owner
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &owner, FP_CREATION_RESERVE);
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();
			// fund the owner
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &owner, FP_DELEGATE_RESERVE);

			assert_noop!(
				Futurepass::register_delegate_with_signature(
					Origin::signed(owner),
					futurepass,
					delegate,
					proxy_type,
					deadline,
					[0u8; 65],
				),
				Error::<Test>::ExpiredDeadline
			);
		});
}

#[test]
fn register_delegate_fails_on_signature_mismatch() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let (signer_1, delegate_1) = create_random_pair();
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			// fund owner
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &owner, FP_CREATION_RESERVE);
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();
			// fund the owner
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &owner, FP_DELEGATE_RESERVE);
			let signature = signer_1
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate_1,
						&proxy_type,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;
			assert_noop!(
				Futurepass::register_delegate_with_signature(
					Origin::signed(owner),
					futurepass,
					delegate_1,
					ProxyType::NonTransfer, // Note: proxy type is different
					deadline,
					signature,
				),
				Error::<Test>::PermissionDenied
			);
			assert_noop!(
				Futurepass::register_delegate_with_signature(
					Origin::signed(owner),
					futurepass,
					create_account(3), // Note: delegate is different
					proxy_type,
					deadline,
					signature,
				),
				Error::<Test>::RegisterDelegateSignerMismatch
			);
			assert_noop!(
				Futurepass::register_delegate_with_signature(
					Origin::signed(owner),
					futurepass,
					delegate_1,
					proxy_type,
					300, // Note: deadline is different
					signature,
				),
				Error::<Test>::RegisterDelegateSignerMismatch
			);
		});
}

#[test]
fn register_delegate_failures_common() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let (signer_1, delegate_1) = create_random_pair();
			let delegate2 = create_account(4);
			let other = create_account(5);
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			// fund owner
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &owner, FP_CREATION_RESERVE);
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			let signature = signer_1
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate_1,
						&proxy_type,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;

			// Try to register_delegate to non existent FP
			assert_noop!(
				Futurepass::register_delegate_with_signature(
					Origin::signed(owner),
					create_random(),
					delegate_1,
					proxy_type,
					deadline,
					signature,
				),
				Error::<Test>::NotFuturepassOwner
			);
			// register_delegate by owner without sufficient reserve balance
			assert_noop!(
				Futurepass::register_delegate_with_signature(
					Origin::signed(owner),
					futurepass,
					delegate_1,
					proxy_type,
					deadline,
					signature,
				),
				pallet_balances::Error::<Test>::InsufficientBalance
			);

			// fund the owner and other
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &owner, FP_DELEGATE_RESERVE);
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &other, FP_DELEGATE_RESERVE);
			// register delegate by owner successfully
			assert_ok!(Futurepass::register_delegate_with_signature(
				Origin::signed(owner),
				futurepass,
				delegate_1,
				proxy_type,
				deadline,
				signature,
			));
			assert!(<Test as Config>::Proxy::exists(&futurepass, &delegate_1, Some(proxy_type)));

			// try to register the same delegate1 again should fail
			assert_noop!(
				Futurepass::register_delegate_with_signature(
					Origin::signed(owner),
					futurepass,
					delegate_1,
					proxy_type,
					deadline,
					[0u8; 65],
				),
				Error::<Test>::DelegateAlreadyExists
			);
			// register_delegate by another delegate should fail - NOTE: for V1
			// fund delegate1
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &delegate_1, FP_DELEGATE_RESERVE);
			assert_noop!(
				Futurepass::register_delegate_with_signature(
					Origin::signed(delegate_1),
					futurepass,
					delegate2,
					proxy_type,
					deadline,
					[0u8; 65],
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
			let (signer, delegate) = create_random_pair();
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			// fund owner
			transfer_funds(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELEGATE_RESERVE,
			);
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			let signature = signer
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate,
						&proxy_type,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;

			// register delegate
			assert_ok!(Futurepass::register_delegate_with_signature(
				Origin::signed(owner),
				futurepass,
				delegate,
				proxy_type,
				deadline,
				signature,
			));
			assert!(<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(proxy_type)));

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
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner), FP_DELEGATE_RESERVE);

			// check delegate is not a proxy of futurepass
			assert_eq!(
				<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(proxy_type)),
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
			let (signer, delegate) = create_random_pair();
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			// fund owner
			transfer_funds(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELEGATE_RESERVE,
			);
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			let signature = signer
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate,
						&proxy_type,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;

			// register delegate
			assert_ok!(Futurepass::register_delegate_with_signature(
				Origin::signed(owner),
				futurepass,
				delegate,
				proxy_type,
				deadline,
				signature,
			));
			assert!(<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(proxy_type)));

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
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &delegate), FP_DELEGATE_RESERVE);

			// check delegate is not a proxy of futurepass
			assert_eq!(
				<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(proxy_type)),
				false
			);
		});
}

#[test]
// unregister_delegate called by non (owner | delegate self) should fail
fn unregister_delegate_by_not_permissioned_fails() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let (signer_1, delegate_1) = create_random_pair();
			let (signer_2, delegate_2) = create_random_pair();
			let other = create_account(5);
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			// fund owner
			transfer_funds(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + 2 * FP_DELEGATE_RESERVE,
			);
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			let signature_1 = signer_1
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate_1,
						&proxy_type,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;
			// register delegate
			assert_ok!(Futurepass::register_delegate_with_signature(
				Origin::signed(owner),
				futurepass,
				delegate_1,
				proxy_type,
				deadline,
				signature_1,
			));
			assert!(<Test as Config>::Proxy::exists(&futurepass, &delegate_1, Some(proxy_type)));

			let signature_2 = signer_2
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate_2,
						&proxy_type,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;
			assert_ok!(Futurepass::register_delegate_with_signature(
				Origin::signed(owner),
				futurepass,
				delegate_2,
				proxy_type,
				deadline,
				signature_2,
			));
			assert!(<Test as Config>::Proxy::exists(&futurepass, &delegate_2, Some(proxy_type)));

			// unregister_delegate by other(non(owner | delegate)) fails
			assert_noop!(
				Futurepass::unregister_delegate(Origin::signed(other), futurepass, delegate_1),
				Error::<Test>::PermissionDenied
			);
			// unregister_delegate by another delegate fails
			assert_noop!(
				Futurepass::unregister_delegate(Origin::signed(delegate_2), futurepass, delegate_1),
				Error::<Test>::PermissionDenied
			);
		});
}

#[test]
fn unregister_delegate_by_owner_itself_fails() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);

			// fund owner
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &owner, FP_CREATION_RESERVE);
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();
			assert!(<Test as Config>::Proxy::exists(&futurepass, &owner, Some(ProxyType::Any)));

			// owner can not unregister by itself
			assert_noop!(
				Futurepass::unregister_delegate(Origin::signed(owner), futurepass, owner),
				Error::<Test>::OwnerCannotUnregister
			);
		});
}

#[test]
fn unregister_delegate_failures_common() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let (signer_1, delegate_1) = create_random_pair();
			let deadline = 200;

			// fund owner
			transfer_funds(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELEGATE_RESERVE,
			);
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();
			// register delegate

			let signature_1 = signer_1
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate_1,
						&ProxyType::Any,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;
			assert_ok!(Futurepass::register_delegate_with_signature(
				Origin::signed(owner),
				futurepass,
				delegate_1,
				ProxyType::Any,
				deadline,
				signature_1,
			));
			assert!(<Test as Config>::Proxy::exists(
				&futurepass,
				&delegate_1,
				Some(ProxyType::Any)
			));

			// unregister_delegate on a non existent futurepass fails
			assert_noop!(
				Futurepass::unregister_delegate(Origin::signed(owner), create_random(), delegate_1),
				Error::<Test>::PermissionDenied
			);
			// unregister_delegate on a non delegate fails
			assert_noop!(
				Futurepass::unregister_delegate(Origin::signed(owner), futurepass, create_random()),
				Error::<Test>::DelegateNotRegistered
			);
		});
}

#[test]
fn transfer_futurepass_to_address_works() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let (signer, delegate) = create_random_pair();
			let other = create_account(4);
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			// fund owner
			transfer_funds(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELEGATE_RESERVE,
			);
			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			let signature = signer
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate,
						&proxy_type,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;
			// register delegate
			assert_ok!(Futurepass::register_delegate_with_signature(
				Origin::signed(owner),
				futurepass,
				delegate,
				proxy_type,
				deadline,
				signature,
			));
			// check delegate is a proxy of futurepass
			assert!(<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(proxy_type)));

			// transfer the ownership to other
			// fund owner since it requires FP_DELEGATE_RESERVE to add new owner
			// the owner will get back the old reserve amount
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &owner, FP_DELEGATE_RESERVE);
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner), FP_DELEGATE_RESERVE);
			assert_ok!(Futurepass::transfer_futurepass(Origin::signed(owner), Some(other)));
			// assert event
			System::assert_has_event(
				Event::<Test>::FuturepassTransferred {
					old_owner: owner,
					new_owner: Some(other),
					futurepass,
				}
				.into(),
			);
			// owner should be other now
			assert_eq!(Holders::<Test>::get(&other), Some(futurepass));
			assert_eq!(Holders::<Test>::get(&owner), None);
			// only the new owner(i.e other) should be a delegate
			assert!(<Test as Config>::Proxy::exists(&futurepass, &other, Some(proxy_type)));
			assert_eq!(
				<Test as Config>::Proxy::exists(&futurepass, &owner, Some(proxy_type)),
				false
			);
			assert_eq!(
				<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(proxy_type)),
				false
			);
			// caller(the owner) should receive the reserved balance diff
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner), 2 * FP_DELEGATE_RESERVE);
		});
}

#[test]
fn transfer_futurepass_to_none_works() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let (signer, delegate) = create_random_pair();
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			// fund owner
			transfer_funds(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELEGATE_RESERVE,
			);
			assert_eq!(
				AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner),
				FP_CREATION_RESERVE + FP_DELEGATE_RESERVE
			);

			// create FP
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			let signature = signer
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate,
						&proxy_type,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;
			// register delegate
			assert_ok!(Futurepass::register_delegate_with_signature(
				Origin::signed(owner),
				futurepass,
				delegate,
				proxy_type,
				deadline,
				signature,
			));
			// check delegate is a proxy of futurepass
			assert!(<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(proxy_type)));

			// transfer the ownership to none
			// fund owner since it requires FP_DELEGATE_RESERVE to add new owner
			// the owner will get back the old reserve amount
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner), 0);
			assert_ok!(Futurepass::transfer_futurepass(Origin::signed(owner), None));
			// assert event
			System::assert_has_event(
				Event::<Test>::FuturepassTransferred {
					old_owner: owner,
					new_owner: None,
					futurepass,
				}
				.into(),
			);
			assert_eq!(Holders::<Test>::get(&owner), None);
			assert_eq!(
				<Test as Config>::Proxy::exists(&futurepass, &owner, Some(proxy_type)),
				false
			);
			assert_eq!(
				<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(proxy_type)),
				false
			);
			// caller(the owner) should receive the reserved balance diff
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner), 400);
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
			let (signer, delegate) = create_random_pair();
			let owner2 = create_account(4);
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			// fund owner
			transfer_funds(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELEGATE_RESERVE,
			);
			// create FP for owner
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			let signature = signer
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate,
						&proxy_type,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;
			// register delegate
			assert_ok!(Futurepass::register_delegate_with_signature(
				Origin::signed(owner),
				futurepass,
				delegate,
				proxy_type,
				deadline,
				signature,
			));
			// check delegate is a proxy of futurepass
			assert!(<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(proxy_type)));

			// fund owner2
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &owner2, FP_CREATION_RESERVE);
			// create FP for owner2
			assert_ok!(Futurepass::create(Origin::signed(owner2), owner2));

			// call transfer_futurepass by other than owner should fail
			assert_noop!(
				Futurepass::transfer_futurepass(
					Origin::signed(create_random()),
					Some(create_random())
				),
				Error::<Test>::NotFuturepassOwner
			);
			// call transfer_futurepass for another futurepass owner should fail
			assert_noop!(
				Futurepass::transfer_futurepass(Origin::signed(owner), Some(owner2)),
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
			let (signer, delegate) = create_random_pair();
			let other = create_account(4);
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			// fund owner
			transfer_funds(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELEGATE_RESERVE,
			);
			// create FP for owner
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			let signature = signer
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate,
						&proxy_type,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;
			// register delegate
			assert_ok!(Futurepass::register_delegate_with_signature(
				Origin::signed(owner),
				futurepass,
				delegate,
				proxy_type,
				deadline,
				signature,
			));

			// fund futurepass with some tokens
			let fund_amount: Balance = 1000;
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &futurepass, fund_amount);
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
			let owner_root_balance = AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner);
			let owner_gas_balance = AssetsExt::balance(MOCK_PAYMENT_ASSET_ID, &owner);
			assert_ok!(Futurepass::proxy_extrinsic(
				Origin::signed(owner),
				futurepass,
				inner_call.clone(),
			));
			// assert event ProxyExecuted
			System::assert_has_event(
				Event::<Test>::ProxyExecuted { delegate: owner, result: Ok(()) }.into(),
			);
			// check balances
			assert_eq!(
				AssetsExt::reducible_balance(MOCK_NATIVE_ASSET_ID, &futurepass, false),
				fund_amount - transfer_amount
			);
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &other), transfer_amount);
			// owner's(i.e caller's) balance not changed
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &owner), owner_root_balance);
			assert_eq!(AssetsExt::balance(MOCK_PAYMENT_ASSET_ID, &owner), owner_gas_balance);

			// call proxy_extrinsic by delegate
			let delegate_root_balance = AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &delegate);
			let delegate_gas_balance = AssetsExt::balance(MOCK_PAYMENT_ASSET_ID, &delegate);
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
			// delegate's(i.e caller's) balance not changed
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &delegate), delegate_root_balance);
			assert_eq!(AssetsExt::balance(MOCK_PAYMENT_ASSET_ID, &delegate), delegate_gas_balance);
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
			let (signer, delegate) = create_random_pair();
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			// fund owner
			transfer_funds(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELEGATE_RESERVE,
			);
			// create FP for owner
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			let signature = signer
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate,
						&proxy_type,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;
			// register delegate
			assert_ok!(Futurepass::register_delegate_with_signature(
				Origin::signed(owner),
				futurepass,
				delegate,
				proxy_type,
				deadline,
				signature,
			));

			// fund futurepass with some tokens
			let fund_amount: Balance = 1_500_000;
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &futurepass, fund_amount);

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
fn proxy_extrinsic_by_non_delegate_fails() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let (signer, delegate) = create_random_pair();
			let other = create_account(4);
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			// fund owner
			transfer_funds(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELEGATE_RESERVE,
			);
			// create FP for owner
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			let signature = signer
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate,
						&proxy_type,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;
			// register delegate
			assert_ok!(Futurepass::register_delegate_with_signature(
				Origin::signed(owner),
				futurepass,
				delegate,
				proxy_type,
				deadline,
				signature,
			));

			// fund futurepass with some tokens
			let fund_amount: Balance = 1000;
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &futurepass, fund_amount);
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
					delegate: other,
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
		});
}

#[test]
fn proxy_extrinsic_to_futurepass_non_whitelist_fails() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let (signer, delegate) = create_random_pair();
			let other = create_account(4);
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			// fund owner
			transfer_funds(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELEGATE_RESERVE,
			);
			// create FP for owner
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			let signature = signer
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate,
						&proxy_type,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;
			// register delegate
			assert_ok!(Futurepass::register_delegate_with_signature(
				Origin::signed(owner),
				futurepass,
				delegate,
				proxy_type,
				deadline,
				signature,
			));

			// fund futurepass with some tokens
			let fund_amount: Balance = 1000;
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &futurepass, fund_amount);
			assert_eq!(
				AssetsExt::reducible_balance(MOCK_NATIVE_ASSET_ID, &futurepass, false),
				fund_amount
			);
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &other), 0);

			// pallet_futurepass calls other than the whitelist can not be called via
			// proxy_extrinsic
			let inner_call =
				Box::new(MockCall::Futurepass(Call::create { account: create_random() }));
			// call proxy_extrinsic by owner
			System::reset_events();
			assert_ok!(Futurepass::proxy_extrinsic(Origin::signed(owner), futurepass, inner_call));
			// assert event ProxyExecuted
			System::assert_has_event(
				Event::<Test>::ProxyExecuted { delegate: owner, result: Ok(()) }.into(),
			);
			// assert event pallet_proxy::ProxyExecuted with the error
			System::assert_has_event(
				pallet_proxy::Event::<Test>::ProxyExecuted {
					result: Err(frame_system::Error::<Test>::CallFiltered.into()),
				}
				.into(),
			);
		});
}

#[test]
fn proxy_extrinsic_to_proxy_pallet_fails() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let (signer, delegate) = create_random_pair();
			let other = create_account(4);
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			// fund owner
			transfer_funds(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELEGATE_RESERVE,
			);
			// create FP for owner
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			let signature = signer
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate,
						&proxy_type,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;
			// register delegate
			assert_ok!(Futurepass::register_delegate_with_signature(
				Origin::signed(owner),
				futurepass,
				delegate,
				proxy_type,
				deadline,
				signature,
			));

			// fund futurepass with some tokens
			let fund_amount: Balance = 1000;
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &futurepass, fund_amount);
			assert_eq!(
				AssetsExt::reducible_balance(MOCK_NATIVE_ASSET_ID, &futurepass, false),
				fund_amount
			);
			assert_eq!(AssetsExt::balance(MOCK_NATIVE_ASSET_ID, &other), 0);

			// pallet_proxy calls can not be called via proxy_extrinsic
			let inner_call = Box::new(MockCall::Proxy(pallet_proxy::Call::add_proxy {
				delegate: create_random(),
				proxy_type,
				delay: 0,
			}));
			// call proxy_extrinsic by owner
			System::reset_events();
			assert_ok!(Futurepass::proxy_extrinsic(Origin::signed(owner), futurepass, inner_call));
			// assert event ProxyExecuted
			System::assert_has_event(
				Event::<Test>::ProxyExecuted { delegate: owner, result: Ok(()) }.into(),
			);
			// assert event pallet_proxy::ProxyExecuted with the error
			System::assert_has_event(
				pallet_proxy::Event::<Test>::ProxyExecuted {
					result: Err(frame_system::Error::<Test>::CallFiltered.into()),
				}
				.into(),
			);
		});
}

#[test]
fn proxy_extrinsic_failures_common() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let (signer, delegate) = create_random_pair();
			let other = create_account(4);
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			// fund owner
			transfer_funds(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&owner,
				FP_CREATION_RESERVE + FP_DELEGATE_RESERVE,
			);
			// create FP for owner
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			let signature = signer
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate,
						&proxy_type,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;
			// register delegate
			assert_ok!(Futurepass::register_delegate_with_signature(
				Origin::signed(owner),
				futurepass,
				delegate,
				proxy_type,
				deadline,
				signature,
			));

			// fund futurepass with some tokens
			let fund_amount: Balance = 1000;
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &futurepass, fund_amount);
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
					delegate: other,
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
			System::assert_has_event(
				Event::<Test>::ProxyExecuted { delegate: owner, result: Ok(()) }.into(),
			);
			// assert event pallet_proxy::ProxyExecuted with the error
			System::assert_has_event(
				pallet_proxy::Event::<Test>::ProxyExecuted {
					result: Err(pallet_balances::Error::<Test>::InsufficientBalance.into()),
				}
				.into(),
			);
		});
}

#[test]
fn whitelist_works() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let (signer, delegate) = create_random_pair();
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			// fund owner
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &owner, FP_CREATION_RESERVE);
			// create FP for owner
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			// fund futurepass with some tokens
			let fund_amount: Balance = 1000;
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &futurepass, fund_amount);
			assert_eq!(
				AssetsExt::reducible_balance(MOCK_NATIVE_ASSET_ID, &futurepass, false),
				fund_amount
			);

			let signature = signer
				.sign_prehashed(
					&Futurepass::generate_add_delegate_eth_signed_message(
						&futurepass,
						&delegate,
						&proxy_type,
						&deadline,
					)
					.unwrap()
					.1,
				)
				.0;

			// pallet_futurepass::Call::register_delegate works via proxy_extrinsic
			let inner_call =
				Box::new(MockCall::Futurepass(Call::register_delegate_with_signature {
					futurepass,
					delegate,
					proxy_type,
					deadline,
					signature,
				}));
			System::reset_events();
			assert_ok!(Futurepass::proxy_extrinsic(Origin::signed(owner), futurepass, inner_call,));
			// assert event ProxyExecuted
			System::assert_has_event(
				Event::<Test>::ProxyExecuted { delegate: owner, result: Ok(()) }.into(),
			);
			System::assert_has_event(
				Event::<Test>::DelegateRegistered { futurepass, delegate, proxy_type }.into(),
			);
			// check delegate is a delegate
			assert_eq!(
				<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(proxy_type)),
				true
			);

			// pallet_futurepass::Call::unregister_delegate works via proxy_extrinsic
			let inner_call =
				Box::new(MockCall::Futurepass(Call::unregister_delegate { futurepass, delegate }));
			System::reset_events();
			assert_ok!(Futurepass::proxy_extrinsic(Origin::signed(owner), futurepass, inner_call,));
			// assert event ProxyExecuted
			System::assert_has_event(
				Event::<Test>::ProxyExecuted { delegate: owner, result: Ok(()) }.into(),
			);
			System::assert_has_event(
				Event::<Test>::DelegateUnregistered { futurepass, delegate }.into(),
			);
			// check delegate is not a delegate
			assert_eq!(
				<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(proxy_type)),
				false
			);
		});
}

#[test]
fn delegate_can_not_call_whitelist_via_proxy_extrinsic() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			let owner = create_account(2);
			let delegate = create_account(3);
			let delegate2 = create_account(4);

			// fund owner
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &owner, FP_CREATION_RESERVE);
			// create FP for owner
			assert_ok!(Futurepass::create(Origin::signed(owner), owner));
			let futurepass = Holders::<Test>::get(&owner).unwrap();

			// fund futurepass with some tokens
			let fund_amount: Balance = 1000;
			transfer_funds(MOCK_NATIVE_ASSET_ID, &funder, &futurepass, fund_amount);
			assert_eq!(
				AssetsExt::reducible_balance(MOCK_NATIVE_ASSET_ID, &futurepass, false),
				fund_amount
			);

			// pallet_futurepass::Call::register_delegate works via proxy_extrinsic
			let inner_call = Box::new(MockCall::Futurepass(Call::register_delegate {
				futurepass,
				delegate,
				proxy_type: ProxyType::Any,
			}));
			System::reset_events();
			assert_ok!(Futurepass::proxy_extrinsic(Origin::signed(owner), futurepass, inner_call,));
			// assert event ProxyExecuted
			System::assert_has_event(
				Event::<Test>::ProxyExecuted { delegate: owner, result: Ok(()) }.into(),
			);
			System::assert_has_event(
				Event::<Test>::DelegateRegistered {
					futurepass,
					delegate,
					proxy_type: ProxyType::Any,
				}
				.into(),
			);
			// check delegate is a delegate
			assert_eq!(
				<Test as Config>::Proxy::exists(&futurepass, &delegate, Some(ProxyType::Any)),
				true
			);

			// check delegate can not call register_delegate via proxy_extrinsic
			let inner_call2 = Box::new(MockCall::Futurepass(Call::register_delegate {
				futurepass,
				delegate: delegate2,
				proxy_type: ProxyType::Any,
			}));

			assert_err!(
				Futurepass::proxy_extrinsic(Origin::signed(delegate), futurepass, inner_call2),
				Error::<Test>::NotFuturepassOwner
			);
			assert_eq!(
				<Test as Config>::Proxy::exists(&futurepass, &delegate2, Some(ProxyType::Any)),
				false
			);
		});
}

#[test]
fn futurepass_admin_migrator_set_by_sudo() {
	let futurepass_admin_migrator = create_account(1337);

	TestExt::default().build().execute_with(|| {
		assert_eq!(MigrationAdmin::<Test>::get(), None);

		// fails if not root
		assert_noop!(
			Futurepass::set_futurepass_migrator(
				Origin::signed(futurepass_admin_migrator),
				futurepass_admin_migrator,
			),
			sp_runtime::DispatchError::BadOrigin,
		);

		// set new admin migrator succeeds from sudo
		assert_ok!(Futurepass::set_futurepass_migrator(
			frame_system::RawOrigin::Root.into(),
			futurepass_admin_migrator,
		));
		System::assert_has_event(
			Event::<Test>::FuturepassMigratorSet { migrator: futurepass_admin_migrator }.into(),
		);
	});
}

#[test]
fn futurepass_migration_multiple_assets() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];
	let futurepass_admin_migrator = create_account(1337);

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			MigrationAdmin::<Test>::put(futurepass_admin_migrator);

			// create EOA and respective futurepass
			let (eoa, evm_futurepass) = (create_account(420), create_account(421));

			// setup assets with eoa as owner, mint assets to eoa EVM futurepass account
			assert_ok!(AssetsExt::mint_into(MOCK_NATIVE_ASSET_ID, &evm_futurepass, 1000));
			assert_ok!(AssetsExt::mint_into(MOCK_PAYMENT_ASSET_ID, &evm_futurepass, 500));
			assert_eq!(
				AssetsExt::reducible_balance(MOCK_NATIVE_ASSET_ID, &evm_futurepass, false),
				1000
			);
			assert_eq!(
				AssetsExt::reducible_balance(MOCK_PAYMENT_ASSET_ID, &evm_futurepass, false),
				500
			);

			// fund migrator
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&futurepass_admin_migrator,
				FP_CREATION_RESERVE,
				false
			));

			// perform migration
			assert_ok!(Futurepass::migrate_evm_futurepass(
				Origin::signed(futurepass_admin_migrator),
				eoa,
				evm_futurepass,
				vec![MOCK_NATIVE_ASSET_ID, MOCK_PAYMENT_ASSET_ID],
				vec![],
			));
			let futurepass = Holders::<Test>::get(&eoa).unwrap();
			System::assert_has_event(
				Event::<Test>::FuturepassAssetsMigrated {
					evm_futurepass,
					futurepass,
					assets: vec![MOCK_NATIVE_ASSET_ID, MOCK_PAYMENT_ASSET_ID],
					collections: vec![],
				}
				.into(),
			);

			// assert evm futurepass has assets
			assert_eq!(
				AssetsExt::reducible_balance(MOCK_NATIVE_ASSET_ID, &evm_futurepass, false),
				0
			);
			assert_eq!(
				AssetsExt::reducible_balance(MOCK_PAYMENT_ASSET_ID, &evm_futurepass, false),
				0
			);
			assert_eq!(
				AssetsExt::reducible_balance(MOCK_NATIVE_ASSET_ID, &futurepass, false),
				1000
			);
			assert_eq!(
				AssetsExt::reducible_balance(MOCK_PAYMENT_ASSET_ID, &futurepass, false),
				500
			);
		});
}

#[test]
fn futurepass_migration_single_collection() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];
	let futurepass_admin_migrator = create_account(1337);

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			MigrationAdmin::<Test>::put(futurepass_admin_migrator);

			// create EOA and respective futurepass
			let (eoa, evm_futurepass) = (create_account(420), create_account(421));

			// setup collection with eoa as owner, mint tokens to eoa EVM futurepass account
			let collection_id = setup_collection(&eoa);
			assert_ok!(Nft::mint(Some(eoa).into(), collection_id, 5, Some(evm_futurepass),));

			// mint some other tokens to funder - these should not be migrated
			assert_ok!(Nft::mint(Some(eoa).into(), collection_id, 15, Some(funder),));

			// assert evm futurepass has tokens
			assert_eq!(Nft::token_balance_of(&evm_futurepass, collection_id), 5);

			// fund migrator
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&futurepass_admin_migrator,
				FP_CREATION_RESERVE,
				false
			));
			// create FP for owner
			assert_ok!(Futurepass::migrate_evm_futurepass(
				Origin::signed(futurepass_admin_migrator),
				eoa,
				evm_futurepass,
				vec![],
				vec![collection_id],
			));
			let futurepass = Holders::<Test>::get(&eoa).unwrap();
			System::assert_has_event(
				Event::<Test>::FuturepassAssetsMigrated {
					evm_futurepass,
					futurepass,
					assets: vec![],
					collections: vec![collection_id],
				}
				.into(),
			);

			// assert evm futurepass has no tokens
			assert_eq!(Nft::token_balance_of(&evm_futurepass, collection_id), 0);

			// assert new futurepass has no tokens
			assert_eq!(Nft::token_balance_of(&futurepass, collection_id), 5);

			// assert funder has tokens (untouched)
			assert_eq!(Nft::token_balance_of(&funder, collection_id), 15);
		});
}

#[test]
fn futurepass_migration_multiple_collections() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];
	let futurepass_admin_migrator = create_account(1337);

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			MigrationAdmin::<Test>::put(futurepass_admin_migrator);

			// create EOA and respective futurepass
			let (eoa, evm_futurepass) = (create_account(420), create_account(421));

			// setup collections with eoa as owner, mint tokens to eoa EVM futurepass account
			let collection_id_1 = setup_collection(&eoa);
			assert_ok!(Nft::mint(Some(eoa).into(), collection_id_1, 5, Some(evm_futurepass),));
			let collection_id_2 = setup_collection(&eoa);
			assert_ok!(Nft::mint(Some(eoa).into(), collection_id_2, 9, Some(evm_futurepass),));

			// fund migrator
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&futurepass_admin_migrator,
				FP_CREATION_RESERVE,
				false
			));
			assert_ok!(Futurepass::migrate_evm_futurepass(
				Origin::signed(futurepass_admin_migrator),
				eoa,
				evm_futurepass,
				vec![],
				vec![collection_id_1, collection_id_2],
			));
			let futurepass = Holders::<Test>::get(&eoa).unwrap();
			System::assert_has_event(
				Event::<Test>::FuturepassAssetsMigrated {
					evm_futurepass,
					futurepass,
					assets: vec![],
					collections: vec![collection_id_1, collection_id_2],
				}
				.into(),
			);

			// assert evm futurepass has no tokens
			assert_eq!(Nft::token_balance_of(&evm_futurepass, collection_id_1), 0);
			assert_eq!(Nft::token_balance_of(&futurepass, collection_id_1), 5);
			assert_eq!(Nft::token_balance_of(&evm_futurepass, collection_id_2), 0);
			assert_eq!(Nft::token_balance_of(&futurepass, collection_id_2), 9);
		});
}

#[test]
fn futurepass_migration_existing_futurepass_account() {
	let funder = create_account(1);
	let endowed = [(funder, 1_000_000)];
	let futurepass_admin_migrator = create_account(1337);

	TestExt::default()
		.with_balances(&endowed)
		.with_xrp_balances(&endowed)
		.build()
		.execute_with(|| {
			MigrationAdmin::<Test>::put(futurepass_admin_migrator);

			// create EOA and respective futurepass
			let (eoa, evm_futurepass) = (create_account(420), create_account(421));

			// setup collection with eoa as owner, mint tokens to eoa EVM futurepass account
			let collection_id = setup_collection(&eoa);
			assert_ok!(Nft::mint(Some(eoa).into(), collection_id, 5, Some(evm_futurepass),));

			// fund migrator
			assert_ok!(AssetsExt::transfer(
				MOCK_NATIVE_ASSET_ID,
				&funder,
				&futurepass_admin_migrator,
				FP_CREATION_RESERVE,
				false
			));

			// create FP for eoa
			assert_ok!(Futurepass::create(Origin::signed(futurepass_admin_migrator), eoa));
			let futurepass = Holders::<Test>::get(&eoa).unwrap();

			// migrate evm futurepass to new futurepass - where futurepass already exists
			assert_ok!(Futurepass::migrate_evm_futurepass(
				Origin::signed(futurepass_admin_migrator),
				eoa,
				evm_futurepass,
				vec![],
				vec![collection_id],
			));
			System::assert_has_event(
				Event::<Test>::FuturepassAssetsMigrated {
					evm_futurepass,
					futurepass,
					assets: vec![],
					collections: vec![collection_id],
				}
				.into(),
			);

			// assert evm futurepass has no tokens
			assert_eq!(Nft::token_balance_of(&evm_futurepass, collection_id), 0);

			// assert new futurepass has no tokens
			assert_eq!(Nft::token_balance_of(&futurepass, collection_id), 5);
		});
}

#[test]
fn futurepass_generate_add_delegate_eth_signed_message() {
	use seed_primitives::AccountId20;
	use sp_core::Pair;
	use sp_runtime::traits::Verify;

	TestExt::default()
		.build()
		.execute_with(|| {
			let futurepass: AccountId =  H160::from_slice(&hex!("FfFFFFff00000000000000000000000000000001")).into();
			let (signer, delegate) = {
				let pair = sp_core::ecdsa::Pair::from_seed(&hex!("7e9c7ad85df5cdc88659f53e06fb2eb9bab3ebc59083a3190eaf2c730332529c"));
				let delegate: AccountId = pair.public().try_into().unwrap(); // 0x420aC537F1a4f78d4Dfb3A71e902be0E3d480AFB
				(pair, delegate)
			};
			let proxy_type = ProxyType::Any;
			let deadline = 200;

			let (msg_hash, eth_signed_msg) = Futurepass::generate_add_delegate_eth_signed_message(
				&futurepass,
				&delegate,
				&proxy_type,
				&deadline,
			).unwrap();

			let want_msg_hash: [u8; 32] = hex!("64c6a93eb2c660b5f8333f0ea4cd7a95247d4731c77c7d3cb6b706c4ba5d8ab4");
			assert_eq!(hex::encode(msg_hash), hex::encode(want_msg_hash));

			let want_eth_signed_msg: [u8; 32] = hex!("3f7a9a5ffe28543f6fd258c2c81d53c3ec0daef97304b79bb61dc33b10e929c0");
			assert_eq!(hex::encode(eth_signed_msg), hex::encode(want_eth_signed_msg));

			// cast wallet sign --private-key 0x7e9c7ad85df5cdc88659f53e06fb2eb9bab3ebc59083a3190eaf2c730332529c "64c6a93eb2c660b5f8333f0ea4cd7a95247d4731c77c7d3cb6b706c4ba5d8ab4"
			let signature: seed_primitives::EthereumSignature = signer.sign_prehashed(&eth_signed_msg).into();

			// note: last byte (v) is set to 0 (while it is 27 on ethereum) because of the way the signature is generated in substrate
			let want_signature: [u8; 65] = hex!("94d1780e44c250d6c87b062e4c2e329deeec176513361fcf006869429f4bdfda549256c203096e9c580b89abbc5c61829cb5eb29270e342a82e21456712d7d4100");
			assert_eq!(hex::encode(signature.0.0), hex::encode(want_signature));

			assert!(signature.verify(hex::encode(msg_hash).as_ref(), &delegate));

			// eth provided sig has `v` set to 27, while substrate has it set to 0
			let eth_provided_signature: [u8; 65] = {
				let mut copied = want_signature.clone();
				copied[64] = 27; // set v to 27
				copied
			};
			println!("eth_provided_signature: {:?}", hex::encode(eth_provided_signature));
			match sp_io::crypto::secp256k1_ecdsa_recover(&eth_provided_signature, &eth_signed_msg) {
				Ok(pubkey_bytes) => {
					let recovered = AccountId20(keccak_256(&pubkey_bytes)[12..].try_into().unwrap());
					assert_eq!(recovered, delegate);
				},
				_ => assert!(false),
			};
		});
}
