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

use crate::{
	mock::{
		test_ext, AssetId, AssetsExt, AssetsExtPalletId, Balances, MockAccountId, NativeAssetId,
		Test,
	},
	AssetDeposit, Config, Error, Holds, NextAssetId,
};
use frame_support::{
	assert_err, assert_noop, assert_ok, assert_storage_noop,
	traits::fungibles::{Inspect, InspectMetadata, Transfer},
	PalletId,
};
use frame_system::RawOrigin;
use seed_pallet_common::{CreateExt, Hold, TransferExt};
use seed_primitives::Balance;
use sp_core::H160;
use sp_runtime::traits::{AccountIdConversion, Zero};

const TEST_PALLET_ID: PalletId = PalletId(*b"pal/test");

#[test]
fn transfer() {
	let alice = 1 as MockAccountId;
	let bob = 2 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;

	test_ext()
		.with_balances(&[(alice, 1_000_000)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, 1_000_000)])
		.build()
		.execute_with(|| {
			// native token transfer
			let alice_balance = AssetsExt::balance(NativeAssetId::get(), &alice);
			assert_ok!(<AssetsExt as Transfer<MockAccountId>>::transfer(
				NativeAssetId::get(),
				&alice,
				&bob,
				100,
				true
			));
			assert_eq!(alice_balance - 100, AssetsExt::balance(NativeAssetId::get(), &alice),);
			assert_eq!(100, AssetsExt::balance(NativeAssetId::get(), &bob),);

			// XRP transfer
			assert_ok!(<AssetsExt as Transfer<MockAccountId>>::transfer(
				xrp_asset_id,
				&alice,
				&bob,
				100,
				true
			));
			assert_eq!(alice_balance - 100, AssetsExt::balance(xrp_asset_id, &alice),);
			assert_eq!(100, AssetsExt::balance(xrp_asset_id, &bob),);
		});
}

#[test]
fn transfer_extrinsic() {
	let alice = 1 as MockAccountId;
	let bob = 2 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;

	test_ext()
		.with_balances(&[(alice, 1_000_000)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, 1_000_000)])
		.build()
		.execute_with(|| {
			// native token transfer
			let alice_balance = AssetsExt::balance(NativeAssetId::get(), &alice);
			assert_ok!(AssetsExt::transfer(Some(alice).into(), NativeAssetId::get(), bob, 100,));
			assert_eq!(alice_balance - 100, AssetsExt::balance(NativeAssetId::get(), &alice),);
			assert_eq!(100, AssetsExt::balance(NativeAssetId::get(), &bob),);

			// XRP transfer
			assert_ok!(AssetsExt::transfer(Some(alice).into(), xrp_asset_id, bob, 100,));
			assert_eq!(alice_balance - 100, AssetsExt::balance(xrp_asset_id, &alice),);
			assert_eq!(100, AssetsExt::balance(xrp_asset_id, &bob),);
		});
}

#[test]
fn transfer_extrinsic_low_balance() {
	let alice = 1 as MockAccountId;
	let bob = 2 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;

	test_ext()
		.with_balances(&[(alice, 99)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, 99)])
		.build()
		.execute_with(|| {
			// native token transfer with insufficient balance
			assert_noop!(
				AssetsExt::transfer(Some(alice).into(), NativeAssetId::get(), bob, 100,),
				pallet_balances::Error::<Test>::InsufficientBalance
			);

			// XRP transfer with insufficient balance
			assert_noop!(
				AssetsExt::transfer(Some(alice).into(), xrp_asset_id, bob, 100,),
				pallet_assets::Error::<Test>::BalanceLow
			);
		});
}

#[test]
fn transfer_keep_alive_extrinsic() {
	let initial_balance = 1_000_000;
	let alice = 1 as MockAccountId;
	let bob = 2 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;

	test_ext()
		.with_balances(&[(alice, initial_balance)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, initial_balance)])
		.build()
		.execute_with(|| {
			// Subtract one to allow for existential deposit/ minimum balance
			let transfer_amount = initial_balance - 1;

			// native token transfer
			assert_ok!(AssetsExt::transfer_keep_alive(
				Some(alice).into(),
				NativeAssetId::get(),
				bob,
				transfer_amount
			));
			assert_eq!(1, AssetsExt::balance(NativeAssetId::get(), &alice),);
			assert_eq!(transfer_amount, AssetsExt::balance(NativeAssetId::get(), &bob),);

			// XRP transfer
			assert_ok!(AssetsExt::transfer_keep_alive(
				Some(alice).into(),
				xrp_asset_id,
				bob,
				transfer_amount,
			));
			assert_eq!(1, AssetsExt::balance(xrp_asset_id, &alice),);
			assert_eq!(transfer_amount, AssetsExt::balance(xrp_asset_id, &bob),);
		});
}

#[test]
fn transfer_keep_alive_extrinsic_above_min() {
	let initial_balance = 1_000_000;
	let alice = 1 as MockAccountId;
	let bob = 2 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;

	test_ext()
		.with_balances(&[(alice, initial_balance)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, initial_balance)])
		.build()
		.execute_with(|| {
			// native token transfer
			assert_noop!(
				AssetsExt::transfer_keep_alive(
					Some(alice).into(),
					NativeAssetId::get(),
					bob,
					initial_balance
				),
				pallet_balances::Error::<Test>::KeepAlive
			);

			// XRP transfer
			assert_noop!(
				AssetsExt::transfer_keep_alive(
					Some(alice).into(),
					xrp_asset_id,
					bob,
					initial_balance,
				),
				pallet_assets::Error::<Test>::BalanceLow
			);
		});
}

#[test]
fn mint_extrinsic() {
	let alice = 1 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;
	let initial_balance = 1_000_000;

	test_ext()
		.with_asset(xrp_asset_id, "XRP", &[(alice, initial_balance)])
		.build()
		.execute_with(|| {
			// native token mint is blocked
			assert_noop!(
				AssetsExt::mint(Some(alice).into(), NativeAssetId::get(), alice, 100),
				Error::<Test>::NoPermission
			);

			// XRP mint from owner
			let xrp_owner = 100 as MockAccountId;
			assert_ok!(AssetsExt::mint(Some(xrp_owner).into(), xrp_asset_id, xrp_owner, 100));
			assert_eq!(100, AssetsExt::balance(xrp_asset_id, &xrp_owner));

			// XRP mint from not owner
			assert_noop!(
				AssetsExt::mint(Some(alice).into(), xrp_asset_id, alice, 100),
				pallet_assets::Error::<Test>::NoPermission
			);
		});
}

#[test]
fn burn_extrinsic() {
	let xrp_owner = 100 as MockAccountId;
	let alice = 1 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;
	let initial_balance = 1_000_000;

	test_ext()
		.with_balances(&[(alice, initial_balance)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, initial_balance), (xrp_owner, initial_balance)])
		.build()
		.execute_with(|| {
			// native token burn is blocked
			assert_noop!(
				AssetsExt::burn_from(Some(alice).into(), NativeAssetId::get(), alice, 100),
				Error::<Test>::NoPermission
			);

			// XRP burn from owner
			assert_ok!(AssetsExt::burn_from(Some(xrp_owner).into(), xrp_asset_id, xrp_owner, 100));
			assert_eq!(initial_balance - 100, AssetsExt::balance(xrp_asset_id, &xrp_owner));

			// XRP burn from not owner
			assert_noop!(
				AssetsExt::burn_from(Some(alice).into(), xrp_asset_id, alice, 100),
				pallet_assets::Error::<Test>::NoPermission
			);
		});
}

#[test]
fn split_transfer() {
	let alice = 1 as MockAccountId;
	let bob = 2 as MockAccountId;
	let ferdie = 3 as MockAccountId;
	let ken = 4 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;

	test_ext()
		.with_balances(&[(alice, 1_000_000)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, 1_000_000)])
		.build()
		.execute_with(|| {
			let transfers = [(bob, 10_000), (ferdie, 15_000), (ken, 20_000)];

			// native token transfer
			let alice_balance = AssetsExt::balance(NativeAssetId::get(), &alice);
			assert_ok!(AssetsExt::split_transfer(&alice, NativeAssetId::get(), &transfers));
			let total = transfers.iter().map(|x| x.1).sum::<Balance>();

			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &alice), alice_balance - total);
			for (recipient, balance) in transfers {
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &recipient), balance);
			}

			// XRP transfer
			let alice_balance = AssetsExt::balance(xrp_asset_id, &alice);
			assert_ok!(AssetsExt::split_transfer(&alice, xrp_asset_id, &transfers));
			let total = transfers.iter().map(|x| x.1).sum::<Balance>();

			assert_eq!(AssetsExt::balance(xrp_asset_id, &alice), alice_balance - total);
			for (recipient, balance) in transfers {
				assert_eq!(AssetsExt::balance(xrp_asset_id, &recipient), balance);
			}
		});
}

#[test]
fn split_transfer_not_enough_balance() {
	let alice = 1 as MockAccountId;
	let bob = 2 as MockAccountId;
	let ferdie = 3 as MockAccountId;
	let ken = 4 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;

	test_ext()
		.with_balances(&[(alice, 1_000_000)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, 1_000_000)])
		.build()
		.execute_with(|| {
			let transfers = [(bob, 400_000), (ferdie, 300_000), (ken, 300_001)];

			// native token transfer
			assert_noop!(
				AssetsExt::split_transfer(&alice, NativeAssetId::get(), &transfers),
				Error::<Test>::BalanceLow
			);

			// XRP transfer
			assert_noop!(
				AssetsExt::split_transfer(&alice, xrp_asset_id, &transfers),
				Error::<Test>::BalanceLow
			);
		});
}

#[test]
fn transfer_insufficient_funds() {
	let alice = 1 as MockAccountId;
	let bob = 2 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;
	let initial_balance = 1_000_000;

	test_ext()
		.with_balances(&[(alice, initial_balance)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, initial_balance)])
		.build()
		.execute_with(|| {
			assert_noop!(
				<AssetsExt as Transfer<MockAccountId>>::transfer(
					NativeAssetId::get(),
					&alice,
					&bob,
					initial_balance + 1,
					true
				),
				pallet_balances::Error::<Test>::InsufficientBalance
			);
			assert_noop!(
				<AssetsExt as Transfer<MockAccountId>>::transfer(
					xrp_asset_id,
					&alice,
					&bob,
					initial_balance + 1,
					true
				),
				pallet_assets::Error::<Test>::BalanceLow
			);
		});
}

#[test]
fn transfer_held_funds() {
	let alice = 1 as MockAccountId;
	let bob = 2 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;
	let initial_balance = 1_000_000;

	test_ext()
		.with_balances(&[(alice, initial_balance)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, initial_balance)])
		.build()
		.execute_with(|| {
			let hold_amount = initial_balance - AssetsExt::minimum_balance(NativeAssetId::get());
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				NativeAssetId::get(),
				hold_amount
			));
			assert_noop!(
				<AssetsExt as Transfer<MockAccountId>>::transfer(
					NativeAssetId::get(),
					&alice,
					&bob,
					hold_amount,
					true
				),
				pallet_balances::Error::<Test>::InsufficientBalance
			);

			let hold_amount = initial_balance - AssetsExt::minimum_balance(xrp_asset_id);
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				xrp_asset_id,
				hold_amount
			));
			assert_noop!(
				<AssetsExt as Transfer<MockAccountId>>::transfer(
					xrp_asset_id,
					&alice,
					&bob,
					hold_amount,
					true
				),
				pallet_assets::Error::<Test>::BalanceLow
			);
		});
}

#[test]
fn place_hold() {
	let alice = 1 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;
	let initial_balance = 1_000_000;

	test_ext()
		.with_balances(&[(alice, initial_balance)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, initial_balance)])
		.build()
		.execute_with(|| {
			let hold_amount = initial_balance - AssetsExt::minimum_balance(NativeAssetId::get());
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				NativeAssetId::get(),
				hold_amount
			));

			// the hold amount is recorded accurately
			assert_eq!(
				AssetsExt::hold_balance(&TEST_PALLET_ID, &alice, &NativeAssetId::get()),
				hold_amount
			);

			let hold_amount = initial_balance - AssetsExt::minimum_balance(xrp_asset_id);
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				xrp_asset_id,
				hold_amount
			));
			// the hold amount is recorded accurately
			assert_eq!(
				AssetsExt::hold_balance(&TEST_PALLET_ID, &alice, &xrp_asset_id),
				hold_amount
			);
			// the hold amount is held by pallet-assets-ext
			assert_eq!(
				AssetsExt::balance(
					xrp_asset_id,
					&AssetsExtPalletId::get().into_account_truncating()
				),
				hold_amount,
			);
		});
}

#[test]
fn place_hold_insufficient_funds() {
	let alice = 1 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;
	let initial_balance = 1_000_000;

	test_ext()
		.with_balances(&[(alice, initial_balance)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, initial_balance)])
		.build()
		.execute_with(|| {
			assert_noop!(
				<AssetsExt as Hold>::place_hold(
					TEST_PALLET_ID,
					&alice,
					NativeAssetId::get(),
					initial_balance + 1
				),
				pallet_balances::Error::<Test>::InsufficientBalance
			);
			assert_noop!(
				<AssetsExt as Hold>::place_hold(
					TEST_PALLET_ID,
					&alice,
					xrp_asset_id,
					initial_balance + 1
				),
				pallet_assets::Error::<Test>::BalanceLow
			);
		});
}

#[test]
fn release_hold() {
	let alice = 1 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;
	let initial_balance = 1_000_000;

	test_ext()
		.with_balances(&[(alice, initial_balance)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, initial_balance)])
		.build()
		.execute_with(|| {
			let hold_amount = initial_balance - AssetsExt::minimum_balance(NativeAssetId::get());
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				NativeAssetId::get(),
				hold_amount
			));
			assert_eq!(
				AssetsExt::hold_balance(&TEST_PALLET_ID, &alice, &NativeAssetId::get()),
				hold_amount
			);
			assert_ok!(<AssetsExt as Hold>::release_hold(
				TEST_PALLET_ID,
				&alice,
				NativeAssetId::get(),
				hold_amount
			));
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &alice), initial_balance,);
			assert!(
				AssetsExt::hold_balance(&TEST_PALLET_ID, &alice, &NativeAssetId::get()).is_zero()
			);
			let hold_amount = initial_balance - AssetsExt::minimum_balance(xrp_asset_id);
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				xrp_asset_id,
				hold_amount
			));
			assert_eq!(
				AssetsExt::hold_balance(&TEST_PALLET_ID, &alice, &xrp_asset_id),
				hold_amount
			);
			assert_ok!(<AssetsExt as Hold>::release_hold(
				TEST_PALLET_ID,
				&alice,
				xrp_asset_id,
				hold_amount
			));
			assert_eq!(AssetsExt::balance(xrp_asset_id, &alice), initial_balance,);
			assert!(AssetsExt::hold_balance(&TEST_PALLET_ID, &alice, &xrp_asset_id).is_zero());
		});
}

#[test]
fn release_hold_partial() {
	let alice = 1 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;
	let initial_balance = 1_000_000;

	test_ext()
		.with_balances(&[(alice, initial_balance)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, initial_balance)])
		.build()
		.execute_with(|| {
			let hold_amount = initial_balance - AssetsExt::minimum_balance(NativeAssetId::get());
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				NativeAssetId::get(),
				hold_amount
			));
			assert_ok!(<AssetsExt as Hold>::release_hold(
				TEST_PALLET_ID,
				&alice,
				NativeAssetId::get(),
				5
			));
			assert_ok!(<AssetsExt as Hold>::release_hold(
				TEST_PALLET_ID,
				&alice,
				NativeAssetId::get(),
				5
			));
			assert_eq!(
				AssetsExt::reducible_balance(NativeAssetId::get(), &alice, false),
				initial_balance - hold_amount + 5 + 5,
			);

			let hold_amount = initial_balance - AssetsExt::minimum_balance(xrp_asset_id);
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				xrp_asset_id,
				hold_amount
			));
			assert_ok!(<AssetsExt as Hold>::release_hold(TEST_PALLET_ID, &alice, xrp_asset_id, 5));
			assert_ok!(<AssetsExt as Hold>::release_hold(TEST_PALLET_ID, &alice, xrp_asset_id, 5));
			assert_eq!(
				AssetsExt::reducible_balance(xrp_asset_id, &alice, false),
				initial_balance - hold_amount + 5 + 5,
			);
			assert_eq!(
				AssetsExt::balance(xrp_asset_id, &alice),
				initial_balance - hold_amount + 5 + 5,
			);
		});
}

#[test]
fn release_hold_insufficient_funds() {
	let alice = 1 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;
	let initial_balance = 1_000_000;

	test_ext()
		.with_balances(&[(alice, initial_balance)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, initial_balance)])
		.build()
		.execute_with(|| {
			let hold_amount = initial_balance - AssetsExt::minimum_balance(NativeAssetId::get());
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				NativeAssetId::get(),
				hold_amount
			));
			assert_err!(
				<AssetsExt as Hold>::release_hold(
					TEST_PALLET_ID,
					&alice,
					NativeAssetId::get(),
					hold_amount * 2
				),
				Error::<Test>::BalanceLow
			);

			let hold_amount = initial_balance - AssetsExt::minimum_balance(xrp_asset_id);
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				xrp_asset_id,
				hold_amount
			));
			assert_err!(
				<AssetsExt as Hold>::release_hold(
					TEST_PALLET_ID,
					&alice,
					xrp_asset_id,
					hold_amount * 2
				),
				Error::<Test>::BalanceLow
			);
		});
}

#[test]
fn place_and_release_hold_multiple_assets_and_pallets() {
	let alice = 1 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;
	let doge_asset_id = 69 as AssetId;
	let dn_asset_id = 420 as AssetId;
	let initial_balance = 1_000_000;
	let other_pallet_id = PalletId(*b"p4l/t3st");
	test_ext()
		.with_asset(xrp_asset_id, "XRP", &[(alice, initial_balance)])
		.with_asset(doge_asset_id, "DOGE", &[(alice, initial_balance)])
		.with_asset(dn_asset_id, "DN", &[(alice, initial_balance)])
		.build()
		.execute_with(|| {
			let hold_amount = initial_balance - AssetsExt::minimum_balance(xrp_asset_id);
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				xrp_asset_id,
				hold_amount
			));
			assert_ok!(<AssetsExt as Hold>::place_hold(
				other_pallet_id,
				&alice,
				doge_asset_id,
				hold_amount
			));
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				dn_asset_id,
				hold_amount
			));

			// release from wrong pallet
			assert_noop!(
				<AssetsExt as Hold>::release_hold(
					other_pallet_id,
					&alice,
					xrp_asset_id,
					hold_amount
				),
				Error::<Test>::BalanceLow
			);

			assert_ok!(<AssetsExt as Hold>::release_hold(
				other_pallet_id,
				&alice,
				doge_asset_id,
				hold_amount - 1,
			));
			assert_ok!(<AssetsExt as Hold>::release_hold(
				TEST_PALLET_ID,
				&alice,
				dn_asset_id,
				hold_amount,
			));
			assert_ok!(<AssetsExt as Hold>::release_hold(
				TEST_PALLET_ID,
				&alice,
				xrp_asset_id,
				hold_amount,
			));
			assert_ok!(<AssetsExt as Hold>::release_hold(
				other_pallet_id,
				&alice,
				doge_asset_id,
				1,
			));

			// the hold amount is held by pallet-assets-ext
			assert!(AssetsExt::balance(
				xrp_asset_id,
				&AssetsExtPalletId::get().into_account_truncating()
			)
			.is_zero(),);
			assert!(AssetsExt::balance(
				dn_asset_id,
				&AssetsExtPalletId::get().into_account_truncating()
			)
			.is_zero(),);
			assert!(AssetsExt::balance(
				doge_asset_id,
				&AssetsExtPalletId::get().into_account_truncating()
			)
			.is_zero(),);
			assert_eq!(AssetsExt::balance(xrp_asset_id, &alice), initial_balance);
			assert_eq!(AssetsExt::balance(dn_asset_id, &alice), initial_balance);
			assert_eq!(AssetsExt::balance(doge_asset_id, &alice), initial_balance);
			// storage cleared
			assert!(
				!Holds::<Test>::contains_key(dn_asset_id, alice) &&
					!Holds::<Test>::contains_key(doge_asset_id, alice) &&
					!Holds::<Test>::contains_key(xrp_asset_id, alice)
			);
		});
}

#[test]
fn spend_hold() {
	let alice = 1 as MockAccountId;
	let bob = 2 as MockAccountId;
	let charlie = 3 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;
	let initial_balance = 1_000_000;

	test_ext()
		.with_balances(&[(alice, initial_balance)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, initial_balance)])
		.build()
		.execute_with(|| {
			let hold_amount = initial_balance - AssetsExt::minimum_balance(NativeAssetId::get());
			let spends = [(bob, 50_000), (charlie, 10_000)];
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				NativeAssetId::get(),
				hold_amount
			));
			assert_ok!(<AssetsExt as Hold>::spend_hold(
				TEST_PALLET_ID,
				&alice,
				NativeAssetId::get(),
				&spends,
			));
			for (payee, amount) in spends {
				assert_eq!(AssetsExt::balance(NativeAssetId::get(), &payee), amount);
			}

			let hold_amount = initial_balance - AssetsExt::minimum_balance(xrp_asset_id);
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				xrp_asset_id,
				hold_amount
			));
			assert_ok!(<AssetsExt as Hold>::spend_hold(
				TEST_PALLET_ID,
				&alice,
				xrp_asset_id,
				&spends,
			));
			for (payee, amount) in spends {
				assert_eq!(AssetsExt::balance(xrp_asset_id, &payee), amount);
			}

			// some odd cases
			// spend to self, spend 0 and empty
			assert_ok!(<AssetsExt as Hold>::spend_hold(
				TEST_PALLET_ID,
				&alice,
				xrp_asset_id,
				&[(alice, 1)],
			));
			assert_storage_noop!({
				let _ = <AssetsExt as Hold>::spend_hold(TEST_PALLET_ID, &alice, xrp_asset_id, &[]);
			});
			assert_storage_noop!({
				let _ = <AssetsExt as Hold>::spend_hold(
					TEST_PALLET_ID,
					&alice,
					xrp_asset_id,
					&[(bob, 0)],
				);
			});
		});
}

#[test]
fn spend_hold_to_holder_non_native() {
	let alice = 1 as MockAccountId;
	let bob = 2 as MockAccountId;
	let initial_balance_alice = 10_000;
	let initial_balance_bob = 20_000;
	let xrp_asset_id = 2 as AssetId;

	test_ext()
		.with_asset(
			xrp_asset_id,
			"XRP",
			&[(alice, initial_balance_alice), (bob, initial_balance_bob)],
		)
		.build()
		.execute_with(|| {
			let transfer_amount = 10_000;
			assert_eq!(
				AssetsExt::balance(xrp_asset_id, &TEST_PALLET_ID.into_account_truncating()),
				0
			);

			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				xrp_asset_id,
				initial_balance_alice
			));
			assert_eq!(AssetsExt::balance(xrp_asset_id, &alice), 0);

			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&bob,
				xrp_asset_id,
				initial_balance_bob
			));
			assert_eq!(AssetsExt::balance(xrp_asset_id, &bob), 0);

			assert_ok!(<AssetsExt as Hold>::spend_hold(
				TEST_PALLET_ID,
				&alice,
				xrp_asset_id,
				&[(bob, transfer_amount)],
			));

			// Check reducible balance, should not include holds amount
			assert_eq!(AssetsExt::balance(xrp_asset_id, &bob), transfer_amount);
			assert_eq!(AssetsExt::balance(xrp_asset_id, &alice), 0);

			// Bob can still unreserve his hold
			assert_ok!(<AssetsExt as Hold>::release_hold(
				TEST_PALLET_ID,
				&bob,
				xrp_asset_id,
				initial_balance_bob
			));
			assert_eq!(
				AssetsExt::balance(xrp_asset_id, &bob),
				transfer_amount + initial_balance_bob
			);

			// Further spends should fail due to insufficient balance
			assert_noop!(
				<AssetsExt as Hold>::spend_hold(
					TEST_PALLET_ID,
					&alice,
					xrp_asset_id,
					&[(bob, transfer_amount)],
				),
				Error::<Test>::BalanceLow
			);
		});
}
#[test]
fn spend_hold_to_holder_native() {
	let alice = 1 as MockAccountId;
	let bob = 2 as MockAccountId;
	let initial_balance_alice = 10_000;
	let initial_balance_bob = 20_000;

	test_ext()
		.with_balances(&[(alice, initial_balance_alice), (bob, initial_balance_bob)])
		.build()
		.execute_with(|| {
			let transfer_amount = 10_000;
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &TEST_PALLET_ID.into_account_truncating()),
				0
			);

			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				NativeAssetId::get(),
				initial_balance_alice
			));
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &alice), 0);

			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&bob,
				NativeAssetId::get(),
				initial_balance_bob
			));
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &bob), 0);

			assert_ok!(<AssetsExt as Hold>::spend_hold(
				TEST_PALLET_ID,
				&alice,
				NativeAssetId::get(),
				&[(bob, transfer_amount)],
			));

			// Check reducible balance, should not include holds amount
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &bob), transfer_amount);
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &alice), 0);

			// Bob can still unreserve his hold
			assert_ok!(<AssetsExt as Hold>::release_hold(
				TEST_PALLET_ID,
				&bob,
				NativeAssetId::get(),
				initial_balance_bob
			));
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &bob),
				transfer_amount + initial_balance_bob
			);

			// Further spends should fail due to insufficient balance
			assert_noop!(
				<AssetsExt as Hold>::spend_hold(
					TEST_PALLET_ID,
					&alice,
					NativeAssetId::get(),
					&[(bob, transfer_amount)],
				),
				Error::<Test>::BalanceLow
			);
		});
}

#[test]
fn spend_hold_to_non_holder() {
	let alice = 1 as MockAccountId;
	let bob = 2 as MockAccountId;
	let initial_balance_alice = 10_000;
	let initial_balance_bob = 20_000;

	test_ext()
		.with_balances(&[(alice, initial_balance_alice), (bob, initial_balance_bob)])
		.build()
		.execute_with(|| {
			let spends = [(bob, initial_balance_alice)];
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				NativeAssetId::get(),
				initial_balance_alice
			));
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &alice), 0);
			assert_ok!(<AssetsExt as Hold>::spend_hold(
				TEST_PALLET_ID,
				&alice,
				NativeAssetId::get(),
				&spends,
			));
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &alice), 0);
			assert_eq!(
				AssetsExt::balance(NativeAssetId::get(), &bob),
				initial_balance_alice + initial_balance_bob
			);
		});
}

#[test]
fn spend_hold_insufficient_funds() {
	let alice = 1 as MockAccountId;
	let bob = 2 as MockAccountId;
	let charlie = 3 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;
	let initial_balance = 1_000_000;

	test_ext()
		.with_balances(&[(alice, initial_balance)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, initial_balance)])
		.build()
		.execute_with(|| {
			let hold_amount = initial_balance - AssetsExt::minimum_balance(NativeAssetId::get());
			let spends = [(bob, hold_amount - 10), (charlie, 11)];
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				NativeAssetId::get(),
				hold_amount
			));
			assert_err!(
				<AssetsExt as Hold>::spend_hold(
					TEST_PALLET_ID,
					&alice,
					NativeAssetId::get(),
					&spends,
				),
				Error::<Test>::BalanceLow
			);

			let hold_amount = initial_balance - AssetsExt::minimum_balance(xrp_asset_id);
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				xrp_asset_id,
				hold_amount
			));
			assert_err!(
				<AssetsExt as Hold>::spend_hold(
					TEST_PALLET_ID,
					&alice,
					NativeAssetId::get(),
					&spends,
				),
				Error::<Test>::BalanceLow
			);
		});
}

#[test]
fn place_hold_asset_does_not_exist() {
	let alice = 1 as MockAccountId;

	test_ext().build().execute_with(|| {
		assert_noop!(
			<AssetsExt as Hold>::place_hold(TEST_PALLET_ID, &alice, NativeAssetId::get() + 1, 100),
			pallet_assets::Error::<Test>::Unknown,
		);
	});
}

#[test]
fn transfer_asset_does_not_exist() {
	let alice = 1 as MockAccountId;
	let bob = 2 as MockAccountId;

	test_ext().build().execute_with(|| {
		assert_noop!(
			<AssetsExt as Transfer<MockAccountId>>::transfer(
				NativeAssetId::get() + 1,
				&alice,
				&bob,
				100,
				true,
			),
			pallet_assets::Error::<Test>::Unknown,
		);
	});
}

#[test]
fn next_asset_uuid_works() {
	test_ext().build().execute_with(|| {
		// This tests assumes parachain_id is set to 100 in mock

		// check default value (set upon build)
		assert_eq!(<NextAssetId<Test>>::get(), 1);

		// asset uuid structure:
		// | 22 asset_id bits | 10 parachain_id bits |
		// |          1           |   100   |
		// 0b000000000000000000001_0001100100

		// Test with first asset_uuid is equivalent to expected binary
		let expected_result = 0b000000000000000000001_0001100100 as u32;
		assert_eq!(AssetsExt::next_asset_uuid().unwrap(), expected_result);

		// Test with max available for 22 bits
		let next_asset_id = (1 << 22) - 2;
		assert_eq!(next_asset_id, 0b0000000000_1111111111111111111110 as u32);
		<NextAssetId<Test>>::put(next_asset_id);
		let expected_result = 0b1111111111111111111110_0001100100 as u32;
		assert_eq!(AssetsExt::next_asset_uuid().unwrap(), expected_result);

		// Next asset_uuid should fail (Reaches 22 bits max)
		let next_asset_id = (1 << 22) - 1;
		assert_eq!(next_asset_id, 0b0000000000_1111111111111111111111 as u32);
		<NextAssetId<Test>>::put(next_asset_id);
		assert_noop!(AssetsExt::next_asset_uuid(), Error::<Test>::NoAvailableIds);
	});
}

#[test]
fn create() {
	test_ext().build().execute_with(|| {
		// check default value (set upon build)
		assert_eq!(<NextAssetId<Test>>::get(), 1);

		// asset uuid structure:
		// | 22 asset_id bits | 10 parachain_id bits |
		// |          1           |   100   |
		// 0b000000000000000000001_0001100100

		// Test with first asset_uuid is equivalent to expected binary
		let expected_result = 0b000000000000000000001_0001100100 as u32;
		assert_eq!(AssetsExt::next_asset_uuid().unwrap(), expected_result);

		pub const ALICE: MockAccountId = 1;
		let parachain_id: u32 = <Test as Config>::ParachainId::get().into();

		// create token & verify asset_uuid increment
		let usdc = <AssetsExt as CreateExt>::create(&ALICE, None).unwrap();
		assert_eq!(usdc, 1 << 10 | parachain_id);
		assert_eq!(AssetsExt::minimum_balance(usdc), 1);
		assert_eq!(AssetsExt::total_issuance(usdc), 0);
		assert!(!pallet_evm::Pallet::<Test>::is_account_empty(
			&H160::from_low_u64_be(usdc as u64).into()
		));

		// create token & verify asset_uuid increment
		let weth = <AssetsExt as CreateExt>::create(&ALICE, None).unwrap();
		assert_eq!(weth, 2 << 10 | parachain_id);
		assert_eq!(AssetsExt::minimum_balance(weth), 1);
		assert_eq!(AssetsExt::total_issuance(weth), 0);
		assert!(!pallet_evm::Pallet::<Test>::is_account_empty(
			&H160::from_low_u64_be(weth as u64).into()
		));
	});
}

#[test]
fn create_asset() {
	pub const ALICE: MockAccountId = 1;
	pub const BOB: MockAccountId = 2;
	let initial_balance = 1_000_000;
	test_ext()
		.with_balances(&[(ALICE, initial_balance), (BOB, initial_balance)])
		.build()
		.execute_with(|| {
			// create usdc token and verify metadata
			let name: Vec<u8> = b"USD-Coin".to_vec();
			let symbol: Vec<u8> = b"USDC".to_vec();
			let decimals: u8 = 6;
			let usdc = AssetsExt::next_asset_uuid().unwrap();
			let min_balance: Balance = 5;
			assert_ok!(AssetsExt::create_asset(
				Some(ALICE).into(),
				name.clone(),
				symbol.clone(),
				decimals,
				Some(min_balance),
				None
			));
			assert_eq!(AssetsExt::minimum_balance(usdc), min_balance);
			assert_eq!(AssetsExt::total_issuance(usdc), 0);
			assert_eq!(<AssetsExt as InspectMetadata<MockAccountId>>::name(&usdc), name);
			assert_eq!(<AssetsExt as InspectMetadata<MockAccountId>>::symbol(&usdc), symbol);
			assert_eq!(<AssetsExt as InspectMetadata<MockAccountId>>::decimals(&usdc), decimals);

			// create Weth token and verify metadata
			let name: Vec<u8> = b"Wrapd-Eth".to_vec();
			let symbol: Vec<u8> = b"WETH".to_vec();
			let decimals: u8 = 18;
			let weth = AssetsExt::next_asset_uuid().unwrap();
			assert_ok!(AssetsExt::create_asset(
				Some(BOB).into(),
				name.clone(),
				symbol.clone(),
				decimals,
				None,
				None
			));
			assert_eq!(AssetsExt::minimum_balance(weth), 1); // Defaults to 1 if None is set
			assert_eq!(AssetsExt::total_issuance(weth), 0);
			assert_eq!(<AssetsExt as InspectMetadata<MockAccountId>>::name(&weth), name);
			assert_eq!(<AssetsExt as InspectMetadata<MockAccountId>>::symbol(&weth), symbol);
			assert_eq!(<AssetsExt as InspectMetadata<MockAccountId>>::decimals(&weth), decimals);
		});
}

#[test]
fn create_asset_fails() {
	pub const ALICE: MockAccountId = 1;
	pub const BOB: MockAccountId = 2;
	let initial_balance = 5_000_000;

	test_ext().with_balances(&[(ALICE, initial_balance)]).build().execute_with(|| {
		assert_ok!(AssetsExt::set_asset_deposit(RawOrigin::Root.into(), 1));

		// Create asset insufficient balance should fail
		assert_noop!(
			AssetsExt::create_asset(
				Some(BOB).into(),
				b"USD-Coin".to_vec(),
				b"USDC".to_vec(),
				6,
				None,
				None
			),
			pallet_balances::Error::<Test>::InsufficientBalance
		);

		// Create asset 19 decimals should fail
		assert_noop!(
			AssetsExt::create_asset(
				Some(ALICE).into(),
				b"USD-Coin".to_vec(),
				b"USDC".to_vec(),
				19,
				None,
				None
			),
			Error::<Test>::DecimalsTooHigh
		);

		// Create asset insufficient name should fail
		let name: Vec<u8> = b"01234567891".to_vec();
		assert_noop!(
			AssetsExt::create_asset(Some(ALICE).into(), name, b"USDC".to_vec(), 6, None, None),
			pallet_assets::Error::<Test>::BadMetadata
		);

		// Create asset insufficient symbol should fail
		let symbol: Vec<u8> = b"01234567891".to_vec();
		assert_noop!(
			AssetsExt::create_asset(
				Some(ALICE).into(),
				b"USD-Coin".to_vec(),
				symbol,
				6,
				None,
				None
			),
			pallet_assets::Error::<Test>::BadMetadata
		);
	});
}

#[test]
fn set_asset_deposit_works() {
	test_ext().build().execute_with(|| {
		let alice: MockAccountId = 1;
		// Set asset deposit not root should fail
		assert_noop!(
			AssetsExt::set_asset_deposit(Some(alice).into(), 123,),
			frame_support::dispatch::DispatchError::BadOrigin
		);
		assert_eq!(AssetDeposit::<Test>::get(), 0);

		// Sudo call should pass
		assert_ok!(AssetsExt::set_asset_deposit(RawOrigin::Root.into(), 123,));
		assert_eq!(AssetDeposit::<Test>::get(), 123);
	});
}

#[test]
fn set_asset_deposit_reserves_the_correct_amount() {
	let alice: MockAccountId = 1;
	let initial_balance = 5_000_000;

	test_ext().with_balances(&[(alice, initial_balance)]).build().execute_with(|| {
		let deposit = 123;

		// Set asset deposit
		assert_ok!(AssetsExt::set_asset_deposit(RawOrigin::Root.into(), deposit,));
		assert_eq!(AssetDeposit::<Test>::get(), deposit);

		let name: Vec<u8> = b"USD-Coin".to_vec();
		let symbol: Vec<u8> = b"USDC".to_vec();
		let decimals: u8 = 6;
		assert_ok!(AssetsExt::create_asset(
			Some(alice).into(),
			name.clone(),
			symbol.clone(),
			decimals,
			None,
			None
		));

		// Alice balance should now be reduced by deposit amount
		let alice_balance = AssetsExt::reducible_balance(NativeAssetId::get(), &alice, false);
		assert_eq!(alice_balance, initial_balance - deposit);

		// The deposit should be reserved
		let alice_reserved = Balances::reserved_balance(&alice);
		assert_eq!(alice_reserved, deposit);
	});
}
