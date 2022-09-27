use crate::{
	mock::{test_ext, AssetId, AssetsExt, AssetsExtPalletId, MockAccountId, NativeAssetId, Test},
	Error, Holds, NextAssetId,
};
use frame_support::{
	assert_err, assert_noop, assert_ok, assert_storage_noop,
	traits::tokens::fungibles::{Inspect, Transfer},
	PalletId,
};
use seed_pallet_common::{CreateExt, Hold};
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
			assert_ok!(AssetsExt::transfer(NativeAssetId::get(), &alice, &bob, 100, true));
			assert_eq!(alice_balance - 100, AssetsExt::balance(NativeAssetId::get(), &alice),);
			assert_eq!(100, AssetsExt::balance(NativeAssetId::get(), &bob),);

			// XRP transfer
			assert_ok!(AssetsExt::transfer(xrp_asset_id, &alice, &bob, 100, true));
			assert_eq!(alice_balance - 100, AssetsExt::balance(xrp_asset_id, &alice),);
			assert_eq!(100, AssetsExt::balance(xrp_asset_id, &bob),);
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
				AssetsExt::transfer(NativeAssetId::get(), &alice, &bob, initial_balance + 1, true),
				pallet_balances::Error::<Test>::InsufficientBalance
			);
			assert_noop!(
				AssetsExt::transfer(xrp_asset_id, &alice, &bob, initial_balance + 1, true),
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
				AssetsExt::transfer(NativeAssetId::get(), &alice, &bob, hold_amount, true),
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
				AssetsExt::transfer(xrp_asset_id, &alice, &bob, hold_amount, true),
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
				AssetsExt::get_hold_balance(&TEST_PALLET_ID, &alice, &NativeAssetId::get()),
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
				AssetsExt::get_hold_balance(&TEST_PALLET_ID, &alice, &xrp_asset_id),
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
				AssetsExt::get_hold_balance(&TEST_PALLET_ID, &alice, &NativeAssetId::get()),
				hold_amount
			);
			assert_ok!(<AssetsExt as Hold>::release_hold(
				TEST_PALLET_ID,
				&alice,
				NativeAssetId::get(),
				hold_amount
			));
			assert_eq!(AssetsExt::balance(NativeAssetId::get(), &alice), initial_balance,);
			assert!(AssetsExt::get_hold_balance(&TEST_PALLET_ID, &alice, &NativeAssetId::get())
				.is_zero());
			let hold_amount = initial_balance - AssetsExt::minimum_balance(xrp_asset_id);
			assert_ok!(<AssetsExt as Hold>::place_hold(
				TEST_PALLET_ID,
				&alice,
				xrp_asset_id,
				hold_amount
			));
			assert_eq!(
				AssetsExt::get_hold_balance(&TEST_PALLET_ID, &alice, &xrp_asset_id),
				hold_amount
			);
			assert_ok!(<AssetsExt as Hold>::release_hold(
				TEST_PALLET_ID,
				&alice,
				xrp_asset_id,
				hold_amount
			));
			assert_eq!(AssetsExt::balance(xrp_asset_id, &alice), initial_balance,);
			assert!(AssetsExt::get_hold_balance(&TEST_PALLET_ID, &alice, &xrp_asset_id).is_zero());
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
			AssetsExt::transfer(NativeAssetId::get() + 1, &alice, &bob, 100, true,),
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

		pub const ALICE: MockAccountId = 1;

		// create token & verify asset_uuid increment
		let usdc = <AssetsExt as CreateExt>::create(&ALICE).unwrap();
		assert_eq!(usdc, 1 << 10 | 100);
		assert_eq!(AssetsExt::minimum_balance(usdc), 1);
		assert_eq!(AssetsExt::total_issuance(usdc), 0);

		// create token & verify asset_uuid increment
		let weth = <AssetsExt as CreateExt>::create(&ALICE).unwrap();
		assert_eq!(weth, 2 << 10 | 100);
		assert_eq!(AssetsExt::minimum_balance(weth), 1);
		assert_eq!(AssetsExt::total_issuance(usdc), 0);
	});
}
