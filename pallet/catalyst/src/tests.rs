use super::*;
use crate::mock::{
	AccountId, AdminAccountId, CATStakingId, CatalystAssetId, CatalystVoucherAssetId, PLUGAssetId,
};
use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use mock::{
	Catalyst, CatalystReward, RuntimeEvent as TestEvent, RuntimeOrigin as Origin, Staking, System,
	Test, Timestamp, Utils,
};
use sp_core::U256;
use sp_runtime::{BuildStorage, TokenError};

pub const BLOCK_TIME: u64 = 1000;

#[test]
fn list_cat_should_work() {
	new_test_ext().execute_with(|| {
		let (
			catalyst_id,
			start_block,
			end_block,
			catalyst_asset_id,
			otto_asset_id,
			_plug_asset_id,
			offered_amount,
			catalyst_next_price,
			catalyst_next_time_diminishing,
			catalyst_next_number,
			catalyst_price_range,
		) = list_cat(true);

		System::assert_last_event(TestEvent::Catalyst(crate::Event::CATEnabled(
			catalyst_id,
			start_block,
			end_block,
			catalyst_asset_id,
			offered_amount,
			otto_asset_id,
		)));

		assert_eq!(
			Catalyst::cat_statuses(catalyst_id),
			CATStatus::Enabled(CATParameters {
				offered_asset: catalyst_asset_id,
				required_asset: otto_asset_id,
				offered_amount,
				start_block,
				end_block,
				next_price: catalyst_next_price,
				next_time_diminishing: catalyst_next_time_diminishing,
				next_cat_number: catalyst_next_number,
				price: catalyst_price_range.clone(),
			}),
		);
	});
}

#[test]
fn list_cat_should_fail_with_non_admin_account() {
	new_test_ext().execute_with(|| {
		let (
			catalyst_id,
			start_block,
			end_block,
			catalyst_asset_id,
			otto_asset_id,
			_plug_asset_id,
			offered_amount,
			catalyst_next_price,
			catalyst_next_time_diminishing,
			catalyst_next_number,
			catalyst_price_range,
		) = list_cat(false);

		let non_admin_account_id = 9999;

		assert_ne!(AdminAccountId::get(), non_admin_account_id);
		assert_noop!(
			Catalyst::list_cat(
				Origin::signed(non_admin_account_id),
				catalyst_id,
				start_block,
				end_block,
				catalyst_asset_id,
				otto_asset_id,
				offered_amount,
				catalyst_next_price,
				catalyst_next_time_diminishing,
				catalyst_next_number,
				catalyst_price_range.clone(),
			),
			BadOrigin
		);
	});
}

#[test]
fn list_cat_should_fail_with_cat_id_in_use() {
	new_test_ext().execute_with(|| {
		let (
			catalyst_id,
			start_block,
			end_block,
			catalyst_asset_id,
			otto_asset_id,
			_plug_asset_id,
			offered_amount,
			catalyst_next_price,
			catalyst_next_time_diminishing,
			catalyst_next_number,
			catalyst_price_range,
		) = list_cat(true);

		assert_noop!(
			Catalyst::list_cat(
				Origin::signed(AdminAccountId::get()),
				catalyst_id,
				start_block,
				end_block,
				catalyst_asset_id,
				otto_asset_id,
				offered_amount,
				catalyst_next_price,
				catalyst_next_time_diminishing,
				catalyst_next_number,
				catalyst_price_range.clone(),
			),
			Error::<Test>::CatIdInUse
		);
	});
}

#[test]
fn list_cat_should_fail_with_invalid_block_number() {
	new_test_ext().execute_with(|| {
		let (
			catalyst_id,
			_start_block,
			_end_block,
			catalyst_asset_id,
			otto_asset_id,
			_plug_asset_id,
			offered_amount,
			catalyst_next_price,
			catalyst_next_time_diminishing,
			catalyst_next_number,
			catalyst_price_range,
		) = list_cat(false);

		let start_block = 0;
		let end_block = 1;

		assert_noop!(
			Catalyst::list_cat(
				Origin::signed(AdminAccountId::get()),
				catalyst_id,
				start_block,
				end_block,
				catalyst_asset_id,
				otto_asset_id,
				offered_amount,
				catalyst_next_price,
				catalyst_next_time_diminishing,
				catalyst_next_number,
				catalyst_price_range.clone(),
			),
			Error::<Test>::InvalidStartblock
		);

		let start_block = 1;
		assert_noop!(
			Catalyst::list_cat(
				Origin::signed(AdminAccountId::get()),
				catalyst_id,
				start_block,
				end_block,
				catalyst_asset_id,
				otto_asset_id,
				offered_amount,
				catalyst_next_price,
				catalyst_next_time_diminishing,
				catalyst_next_number,
				catalyst_price_range.clone(),
			),
			Error::<Test>::InvalidEndblock
		);
	});
}

#[test]
fn disable_cat_should_work() {
	new_test_ext().execute_with(|| {
		let catalyst_id = 0;

		assert_ok!(Catalyst::disable_cat(Origin::signed(TEST_ACCOUNT), catalyst_id,));

		System::assert_last_event(TestEvent::Catalyst(crate::Event::CATDisabled(catalyst_id)));
	});
}

#[test]
fn disable_cat_should_fail_with_non_admin_account() {
	new_test_ext().execute_with(|| {
		let catalyst_id = 0;

		let non_admin_account_id = 9999;

		assert_ne!(AdminAccountId::get(), non_admin_account_id);
		assert_noop!(
			Catalyst::disable_cat(Origin::signed(non_admin_account_id), catalyst_id),
			BadOrigin
		);
	});
}

#[test]
fn paticipate_should_work_with_both_otto_and_plug_asset() {
	new_test_ext().execute_with(|| {
		init_yieldfarming();

		let (
			catalyst_id,
			_start_block,
			_end_block,
			_catalyst_asset_id,
			otto_asset_id,
			plug_asset_id,
			_offered_amount,
			_catalyst_next_price,
			_catalyst_next_time_diminishing,
			_catalyst_next_number,
			_catalyst_price_range,
		) = list_cat(true);

		let cat_amount = 1;
		// mint should work with PLUG token
		let plug_balance_before = Utils::asset_balance(plug_asset_id, &TEST_ACCOUNT);
		let vault_account_plug_balance_before =
			Utils::asset_balance(plug_asset_id, &Catalyst::get_vault_account(catalyst_id).unwrap());
		let plug_cost = Catalyst::get_total_cost(catalyst_id, cat_amount, plug_asset_id).unwrap();
		assert_ok!(Catalyst::participate(
			Origin::signed(TEST_ACCOUNT),
			catalyst_id,
			cat_amount,
			plug_asset_id
		));
		assert_eq!(
			Utils::asset_balance(plug_asset_id, &TEST_ACCOUNT),
			plug_balance_before - plug_cost
		);
		assert_eq!(
			Utils::asset_balance(plug_asset_id, &Catalyst::get_vault_account(catalyst_id).unwrap()),
			vault_account_plug_balance_before + plug_cost
		);

		// mint should work with OTTO token
		let otto_balance_before = Utils::asset_balance(otto_asset_id, &TEST_ACCOUNT);
		let vault_account_otto_balance_before =
			Utils::asset_balance(otto_asset_id, &Catalyst::get_vault_account(catalyst_id).unwrap());
		let otto_cost = Catalyst::get_total_cost(catalyst_id, cat_amount, otto_asset_id).unwrap();
		assert_ok!(Catalyst::participate(
			Origin::signed(TEST_ACCOUNT),
			catalyst_id,
			cat_amount,
			otto_asset_id
		));
		assert_eq!(
			Utils::asset_balance(otto_asset_id, &TEST_ACCOUNT),
			otto_balance_before - otto_cost
		);
		assert_eq!(
			Utils::asset_balance(otto_asset_id, &Catalyst::get_vault_account(catalyst_id).unwrap()),
			vault_account_otto_balance_before + otto_cost
		);
	});
}

#[test]
fn paticipate_should_fail_with_cat_disabled() {
	new_test_ext().execute_with(|| {
		let catalyst_id = 0;
		let plug_asset_id = P_PLUG_ASSET_ID;
		let cat_amount = 1;
		assert_noop!(
			Catalyst::participate(
				Origin::signed(TEST_ACCOUNT),
				catalyst_id,
				cat_amount,
				plug_asset_id
			),
			Error::<Test>::CatIsNotEnabled
		);
	});
}

#[test]
fn paticipate_should_work_with_exact_price_expectation_when_using_plug_token() {
	new_test_ext().execute_with(|| {
		init_yieldfarming();

		let (
			catalyst_id,
			_start_block,
			_end_block,
			_catalyst_asset_id,
			_otto_asset_id,
			plug_asset_id,
			_offered_amount,
			_catalyst_next_price,
			_catalyst_next_time_diminishing,
			_catalyst_next_number,
			_catalyst_price_range,
		) = list_cat(true);
		let cat_amount = 10;
		// mint should work with PLUG token
		let plug_balance_before = Utils::asset_balance(plug_asset_id, &TEST_ACCOUNT);
		let vault_account_plug_balance_before =
			Utils::asset_balance(plug_asset_id, &Catalyst::get_vault_account(catalyst_id).unwrap());

		let plug_cost = FixedU128::from_inner(250518598819049816888).into_inner();
		assert_ok!(Catalyst::participate(
			Origin::signed(TEST_ACCOUNT),
			catalyst_id,
			cat_amount,
			plug_asset_id
		));
		assert_eq!(
			Utils::asset_balance(plug_asset_id, &TEST_ACCOUNT),
			plug_balance_before - plug_cost
		);
		assert_eq!(
			Utils::asset_balance(plug_asset_id, &Catalyst::get_vault_account(catalyst_id).unwrap()),
			vault_account_plug_balance_before + plug_cost
		);
	});
}

#[test]
fn paticipate_should_work_within_price_range() {
	new_test_ext().execute_with(|| {
		init_yieldfarming();

		let (
			catalyst_id,
			_start_block,
			_end_block,
			_catalyst_asset_id,
			_otto_asset_id,
			plug_asset_id,
			_offered_amount,
			_catalyst_next_price,
			_catalyst_next_time_diminishing,
			_catalyst_next_number,
			_catalyst_price_range,
		) = list_cat(true);

		// should work when mint 50 catalyst each time (including exceed max catalyst amount
		for cat_amount in vec![50; 12] {
			// mint should work with PLUG token
			let plug_balance_before = Utils::asset_balance(plug_asset_id, &TEST_ACCOUNT);
			let vault_account_plug_balance_before = Utils::asset_balance(
				plug_asset_id,
				&Catalyst::get_vault_account(catalyst_id).unwrap(),
			);
			let plug_cost =
				Catalyst::get_total_cost(catalyst_id, cat_amount, plug_asset_id).unwrap();
			assert_ok!(Catalyst::participate(
				Origin::signed(TEST_ACCOUNT),
				catalyst_id,
				cat_amount,
				plug_asset_id
			));
			assert_eq!(
				Utils::asset_balance(plug_asset_id, &TEST_ACCOUNT),
				plug_balance_before - plug_cost
			);
			assert_eq!(
				Utils::asset_balance(
					plug_asset_id,
					&Catalyst::get_vault_account(catalyst_id).unwrap()
				),
				vault_account_plug_balance_before + plug_cost
			);
		}
	});
}

#[test]
fn paticipate_should_work_among_two_price_ranges() {
	new_test_ext().execute_with(|| {
		init_yieldfarming();

		let (
			catalyst_id,
			_start_block,
			_end_block,
			_catalyst_asset_id,
			otto_asset_id,
			_plug_asset_id,
			_offered_amount,
			_catalyst_next_price,
			_catalyst_next_time_diminishing,
			_catalyst_next_number,
			_catalyst_price_range,
		) = list_cat(true);

		// should work when mint 150 catalyst each time (including exceed max catalyst amount
		for cat_amount in vec![150; 5] {
			// mint should work with OTTO token
			let otto_balance_before = Utils::asset_balance(otto_asset_id, &TEST_ACCOUNT);
			let vault_account_otto_balance_before = Utils::asset_balance(
				otto_asset_id,
				&Catalyst::get_vault_account(catalyst_id).unwrap(),
			);
			let otto_cost =
				Catalyst::get_total_cost(catalyst_id, cat_amount, otto_asset_id).unwrap();
			assert_ok!(Catalyst::participate(
				Origin::signed(TEST_ACCOUNT),
				catalyst_id,
				cat_amount,
				otto_asset_id
			));
			assert_eq!(
				Utils::asset_balance(otto_asset_id, &TEST_ACCOUNT),
				otto_balance_before - otto_cost
			);
			assert_eq!(
				Utils::asset_balance(
					otto_asset_id,
					&Catalyst::get_vault_account(catalyst_id).unwrap()
				),
				vault_account_otto_balance_before + otto_cost
			);
		}
	});
}

#[test]
fn paticipate_should_work_among_three_price_ranges() {
	new_test_ext().execute_with(|| {
		init_yieldfarming();

		let (
			catalyst_id,
			_start_block,
			_end_block,
			_catalyst_asset_id,
			_otto_asset_id,
			plug_asset_id,
			_offered_amount,
			_catalyst_next_price,
			_catalyst_next_time_diminishing,
			_catalyst_next_number,
			_catalyst_price_range,
		) = list_cat(true);

		// should work when mint 220 catalyst each time (including exceed max catalyst amount
		for cat_amount in vec![220; 4] {
			// mint should work with PLUG token
			let plug_balance_before = Utils::asset_balance(plug_asset_id, &TEST_ACCOUNT);
			let vault_account_plug_balance_before = Utils::asset_balance(
				plug_asset_id,
				&Catalyst::get_vault_account(catalyst_id).unwrap(),
			);
			let plug_cost =
				Catalyst::get_total_cost(catalyst_id, cat_amount, plug_asset_id).unwrap();
			assert_ok!(Catalyst::participate(
				Origin::signed(TEST_ACCOUNT),
				catalyst_id,
				cat_amount,
				plug_asset_id
			));
			assert_eq!(
				Utils::asset_balance(plug_asset_id, &TEST_ACCOUNT),
				plug_balance_before - plug_cost
			);
			assert_eq!(
				Utils::asset_balance(
					plug_asset_id,
					&Catalyst::get_vault_account(catalyst_id).unwrap()
				),
				vault_account_plug_balance_before + plug_cost
			);
		}
	});
}

#[test]
fn paticipate_should_work_among_four_price_ranges() {
	new_test_ext().execute_with(|| {
		init_yieldfarming();

		let (
			catalyst_id,
			_start_block,
			_end_block,
			_catalyst_asset_id,
			otto_asset_id,
			_plug_asset_id,
			_offered_amount,
			_catalyst_next_price,
			_catalyst_next_time_diminishing,
			_catalyst_next_number,
			_catalyst_price_range,
		) = list_cat(true);

		// should work when mint 330 catalyst each time (including exceed max catalyst amount
		for cat_amount in vec![330; 3] {
			// mint should work with OTTO token
			let otto_balance_before = Utils::asset_balance(otto_asset_id, &TEST_ACCOUNT);
			let vault_account_otto_balance_before = Utils::asset_balance(
				otto_asset_id,
				&Catalyst::get_vault_account(catalyst_id).unwrap(),
			);
			let otto_cost =
				Catalyst::get_total_cost(catalyst_id, cat_amount, otto_asset_id).unwrap();
			assert_ok!(Catalyst::participate(
				Origin::signed(TEST_ACCOUNT),
				catalyst_id,
				cat_amount,
				otto_asset_id
			));
			assert_eq!(
				Utils::asset_balance(otto_asset_id, &TEST_ACCOUNT),
				otto_balance_before - otto_cost
			);
			assert_eq!(
				Utils::asset_balance(
					otto_asset_id,
					&Catalyst::get_vault_account(catalyst_id).unwrap()
				),
				vault_account_otto_balance_before + otto_cost
			);
		}
	});
}

#[test]
fn do_finalize_ieo_should_work() {
	new_test_ext().execute_with(|| {
		init_yieldfarming();

		let (
			catalyst_id,
			_start_block,
			end_block,
			_catalyst_asset_id,
			_otto_asset_id,
			_plug_asset_id,
			_offered_amount,
			_catalyst_next_price,
			_catalyst_next_time_diminishing,
			_catalyst_next_number,
			_catalyst_price_range,
		) = list_cat(true);
		assert_ok!(Catalyst::list_ieo(
			Origin::signed(AdminAccountId::get()),
			catalyst_id,
			end_block - 100
		));

		// 6 decimals (otto decimal 12, plug decimals 18) * 25 as the rate
		let plug_rate = 25000000;

		// First 10
		// amount summed from excel for easiness
		let plug_amount = 10_020743953000 * plug_rate;
		assert_eq!(
			Catalyst::do_finalize_ieo(
				plug_amount,
				_catalyst_next_price,
				_catalyst_next_number,
				_catalyst_next_time_diminishing,
				&_catalyst_price_range
			),
			(1000, FixedU128::from_inner(1_004613660713), FixedU128::from_float(365.25), 11)
		);

		// cross range 29995 - 30002
		let plug_amount = 6998_207697300000 * plug_rate;
		let _catalyst_next_number = 29995;
		let _catalyst_next_price = FixedU128::from_inner(999_536014400000);
		assert_eq!(
			Catalyst::do_finalize_ieo(
				plug_amount,
				_catalyst_next_price,
				_catalyst_next_number,
				_catalyst_next_time_diminishing,
				&_catalyst_price_range
			),
			(700, FixedU128::from_inner(1000_021227757058), FixedU128::from_float(365.25), 30002)
		);

		// cross range 49995 - 50000 - 50002
		let plug_amount = 27990_264896773600 * plug_rate;
		let _catalyst_next_number = 49995;
		let _catalyst_next_price = FixedU128::from_inner(3997_777694003700);

		assert_eq!(
			Catalyst::do_finalize_ieo(
				plug_amount,
				_catalyst_next_price,
				_catalyst_next_number,
				_catalyst_next_time_diminishing,
				&_catalyst_price_range
			),
			(700, FixedU128::from_inner(3999_492994639107), FixedU128::from_float(1461.0), 50002)
		);

		// cross range 39995 - 40000 - 50000 - 50002
		let plug_amount = 28864891_261683500000 * plug_rate;
		let _catalyst_next_number = 39995;
		let _catalyst_next_price = FixedU128::from_inner(1998_888982344170);

		assert_eq!(
			Catalyst::do_finalize_ieo(
				plug_amount,
				_catalyst_next_price,
				_catalyst_next_number,
				_catalyst_next_time_diminishing,
				&_catalyst_price_range
			),
			(
				1000700,
				FixedU128::from_inner(3999_492994641379),
				FixedU128::from_float(1461.0),
				50002
			)
		);
	});
}

#[test]
fn list_ieo_should_work() {
	new_test_ext().execute_with(|| {
		init_yieldfarming();

		let (
			catalyst_id,
			_start_block,
			end_block,
			_catalyst_asset_id,
			_otto_asset_id,
			_plug_asset_id,
			_offered_amount,
			_catalyst_next_price,
			_catalyst_next_time_diminishing,
			_catalyst_next_number,
			_catalyst_price_range,
		) = list_cat(true);

		let end_block = end_block - 100;
		assert_ok!(Catalyst::list_ieo(
			Origin::signed(AdminAccountId::get()),
			catalyst_id,
			end_block
		));

		let now = System::block_number();
		System::assert_last_event(TestEvent::Catalyst(crate::Event::PlugCataIEOEnabled(
			now,
			end_block,
			PLUGAssetId::get(),
			CatalystVoucherAssetId::get(),
		)));

		let ieo_status = Catalyst::ieo_statuses(catalyst_id);
		assert_eq!(ieo_status, IEOStatus::Enabled(end_block));

		// should work when disable and then list_ieo again to update end_block
		assert_ok!(Catalyst::disable_ieo(Origin::signed(AdminAccountId::get()), catalyst_id));
		let end_block = end_block - 50;
		assert_ok!(Catalyst::list_ieo(
			Origin::signed(AdminAccountId::get()),
			catalyst_id,
			end_block
		));
		let ieo_status = Catalyst::ieo_statuses(catalyst_id);
		assert_eq!(ieo_status, IEOStatus::Enabled(end_block));
	});
}

#[test]
fn list_ieo_should_fail_with_correct_errors() {
	new_test_ext().execute_with(|| {
		init_yieldfarming();

		let (
			catalyst_id,
			_start_block,
			end_block,
			_catalyst_asset_id,
			_otto_asset_id,
			_plug_asset_id,
			_offered_amount,
			_catalyst_next_price,
			_catalyst_next_time_diminishing,
			_catalyst_next_number,
			_catalyst_price_range,
		) = list_cat(true);

		let end_block = end_block - 100;

		// Not CATAAdminOrigin
		assert_noop!(Catalyst::list_ieo(Origin::signed(100), catalyst_id, end_block), BadOrigin);

		// Invalid end block
		let now = System::block_number();
		assert_noop!(
			Catalyst::list_ieo(Origin::signed(AdminAccountId::get()), catalyst_id, now - 1),
			Error::<Test>::InvalidEndblock
		);

		assert_ok!(Catalyst::list_ieo(
			Origin::signed(AdminAccountId::get()),
			catalyst_id,
			end_block
		));

		assert_noop!(
			Catalyst::list_ieo(Origin::signed(AdminAccountId::get()), catalyst_id, end_block),
			Error::<Test>::IeoAlreadyEnabled
		);
	});
}

#[test]
fn disable_ieo_should_work() {
	new_test_ext().execute_with(|| {
		let catalyst_id = 0;

		assert_noop!(Catalyst::disable_ieo(Origin::signed(100), catalyst_id), BadOrigin);

		assert_ok!(Catalyst::disable_ieo(Origin::signed(AdminAccountId::get()), catalyst_id,));

		System::assert_last_event(TestEvent::Catalyst(crate::Event::PlugCataIEODisabled(
			catalyst_id,
		)));

		assert_eq!(Catalyst::ieo_statuses(catalyst_id), IEOStatus::NotEnabled);
	});
}

#[test]
fn clear_orderbook_should_work() {
	new_test_ext().execute_with(|| {
		let (
			catalyst_id,
			start_block,
			end_block,
			catalyst_asset_id,
			otto_asset_id,
			_plug_asset_id,
			offered_amount,
			catalyst_next_price,
			catalyst_next_time_diminishing,
			catalyst_next_number,
			catalyst_price_range,
		) = list_cat(false);

		assert_noop!(Catalyst::clear_orderbook(Origin::signed(100), catalyst_id), BadOrigin);

		assert_ok!(Catalyst::clear_orderbook(Origin::signed(AdminAccountId::get()), catalyst_id,));

		System::assert_last_event(TestEvent::Catalyst(crate::Event::PlugCataIEOOrderBookClear(
			catalyst_id,
		)));

		init_yieldfarming();

		assert_ok!(Catalyst::list_cat(
			Origin::signed(AdminAccountId::get()),
			catalyst_id,
			start_block,
			end_block,
			catalyst_asset_id,
			otto_asset_id,
			offered_amount,
			catalyst_next_price,
			catalyst_next_time_diminishing,
			catalyst_next_number,
			catalyst_price_range.clone(),
		));

		let end_block = end_block - 100;
		assert_ok!(Catalyst::list_ieo(
			Origin::signed(AdminAccountId::get()),
			catalyst_id,
			end_block
		));

		let plug_amount = 10u128.pow(18 + 6) * 26;

		assert_ok!(Catalyst::deposit_plug(Origin::signed(TEST_ACCOUNT), catalyst_id, plug_amount));

		assert_noop!(
			Catalyst::clear_orderbook(Origin::signed(AdminAccountId::get()), catalyst_id),
			Error::<Test>::IeoIsNotDisabled
		);

		assert_ok!(Catalyst::disable_ieo(Origin::signed(AdminAccountId::get()), catalyst_id));

		assert_ok!(Catalyst::clear_orderbook(Origin::signed(AdminAccountId::get()), catalyst_id));

		assert_eq!(Catalyst::ieo_total_gathered(catalyst_id), Default::default());
		assert_eq!(Catalyst::ieo_orderbook(catalyst_id, TEST_ACCOUNT), Default::default());
	});
}

#[test]
fn deposit_plug_should_work() {
	new_test_ext().execute_with(|| {
		init_yieldfarming();

		let (
			catalyst_id,
			_start_block,
			end_block,
			_catalyst_asset_id,
			_otto_asset_id,
			_plug_asset_id,
			_offered_amount,
			_catalyst_next_price,
			_catalyst_next_time_diminishing,
			_catalyst_next_number,
			_catalyst_price_range,
		) = list_cat(true);

		let end_block = end_block - 100;
		assert_ok!(Catalyst::list_ieo(
			Origin::signed(AdminAccountId::get()),
			catalyst_id,
			end_block
		));

		let plug_amount = 10u128.pow(18 + 6) * 26;

		assert_ok!(Catalyst::deposit_plug(Origin::signed(TEST_ACCOUNT), catalyst_id, plug_amount));

		System::assert_last_event(TestEvent::Catalyst(crate::Event::PlugOfferReceived(
			TEST_ACCOUNT,
			plug_amount,
		)));

		assert_eq!(Catalyst::ieo_orderbook(catalyst_id, TEST_ACCOUNT), (plug_amount, false));
		assert_eq!(Catalyst::ieo_total_gathered(catalyst_id), plug_amount);
	});
}

#[test]
fn deposit_plug_should_fail_with_correct_errors() {
	new_test_ext().execute_with(|| {
		init_yieldfarming();

		let (
			catalyst_id,
			start_block,
			end_block,
			catalyst_asset_id,
			otto_asset_id,
			_plug_asset_id,
			offered_amount,
			catalyst_next_price,
			catalyst_next_time_diminishing,
			catalyst_next_number,
			catalyst_price_range,
		) = list_cat(false);

		let plug_amount = 10u128.pow(18 + 6) * 26;

		// Not verified user
		assert_noop!(
			Catalyst::deposit_plug(Origin::signed(100), catalyst_id, plug_amount),
			BadOrigin
		);

		// Cat is not enabled
		assert_noop!(
			Catalyst::deposit_plug(Origin::signed(TEST_ACCOUNT), catalyst_id, plug_amount),
			Error::<Test>::CatIsNotEnabled
		);

		assert_ok!(Catalyst::list_cat(
			Origin::signed(AdminAccountId::get()),
			catalyst_id,
			start_block,
			end_block,
			catalyst_asset_id,
			otto_asset_id,
			offered_amount,
			catalyst_next_price,
			catalyst_next_time_diminishing,
			catalyst_next_number,
			catalyst_price_range.clone(),
		));

		// Ieo is not enabled
		assert_noop!(
			Catalyst::deposit_plug(Origin::signed(TEST_ACCOUNT), catalyst_id, plug_amount),
			Error::<Test>::IeoIsNotEnabled
		);

		let end_block = end_block - 100;
		assert_ok!(Catalyst::list_ieo(
			Origin::signed(AdminAccountId::get()),
			catalyst_id,
			end_block
		));

		run_to_block(end_block);
		// Ieo already done
		assert_noop!(
			Catalyst::deposit_plug(Origin::signed(TEST_ACCOUNT), catalyst_id, plug_amount),
			Error::<Test>::IeoAlreadyDone
		);
	});
}

#[test]
fn redeem_cata_should_work() {
	new_test_ext().execute_with(|| {
		init_yieldfarming();

		let (
			catalyst_id,
			_start_block,
			end_block,
			_catalyst_asset_id,
			_otto_asset_id,
			_plug_asset_id,
			_offered_amount,
			_catalyst_next_price,
			_catalyst_next_time_diminishing,
			_catalyst_next_number,
			_catalyst_price_range,
		) = list_cat(true);

		let end_block = end_block - 100;
		assert_ok!(Catalyst::list_ieo(
			Origin::signed(AdminAccountId::get()),
			catalyst_id,
			end_block
		));

		let account_a: AccountId = 1;
		let vault_catalyst_amount_before = Utils::asset_balance(
			CatalystAssetId::get(),
			&Catalyst::get_vault_account(catalyst_id).unwrap(),
		);
		let plug_amount_a = 10u128.pow(18) * 50;
		let account_b: AccountId = 2;
		let plug_amount_b = 10u128.pow(18) * 150;
		let account_c: AccountId = 3;
		let plug_amount_c = 10u128.pow(18) * 250;
		assert_ok!(Catalyst::deposit_plug(Origin::signed(account_a), catalyst_id, plug_amount_a));
		assert_ok!(Catalyst::deposit_plug(Origin::signed(account_b), catalyst_id, plug_amount_b));
		assert_ok!(Catalyst::deposit_plug(Origin::signed(account_c), catalyst_id, plug_amount_c));

		run_to_block_with_cata_hooks(end_block);
		assert_ok!(Catalyst::pay_unsigned(Origin::none(), end_block));
		run_to_block_with_cata_hooks(end_block + 1);
		assert_ok!(Catalyst::pay_unsigned(Origin::none(), end_block + 1));

		assert_eq!(
			Utils::asset_balance(
				CatalystAssetId::get(),
				&Catalyst::get_vault_account(catalyst_id).unwrap()
			),
			vault_catalyst_amount_before + 17
		);
		assert_eq!(Utils::asset_balance(CatalystVoucherAssetId::get(), &account_a), 188);
		assert_eq!(Utils::asset_balance(CatalystVoucherAssetId::get(), &account_b), 566);
		assert_eq!(Utils::asset_balance(CatalystVoucherAssetId::get(), &account_c), 944);
		assert_noop!(
			Catalyst::redeem_cata(Origin::signed(account_a), catalyst_id, 300),
			TokenError::FundsUnavailable
		);
		assert_noop!(
			Catalyst::redeem_cata(Origin::signed(account_a), catalyst_id, 101),
			Error::<Test>::InvalidAmount
		);

		assert_ok!(Catalyst::redeem_cata(Origin::signed(account_a), catalyst_id, 100));
		assert_ok!(Catalyst::redeem_cata(Origin::signed(account_b), catalyst_id, 400));
		assert_ok!(Catalyst::redeem_cata(Origin::signed(account_c), catalyst_id, 900));
		assert_eq!(Utils::asset_balance(CatalystVoucherAssetId::get(), &account_a), 88);
		assert_eq!(Utils::asset_balance(CatalystVoucherAssetId::get(), &account_b), 166);
		assert_eq!(Utils::asset_balance(CatalystVoucherAssetId::get(), &account_c), 44);
	});
}

#[test]
fn calculate_cat_price_should_work() {
	new_test_ext().execute_with(|| {
		let (
			_catalyst_id,
			_start_block,
			_end_block,
			_catalyst_asset_id,
			_otto_asset_id,
			_plug_asset_id,
			_offered_amount,
			_catalyst_next_price,
			_catalyst_next_time_diminishing,
			_catalyst_next_number,
			catalyst_price_range,
		) = list_cat(true);

		// cat 1
		let cat_price = Catalyst::get_cat_price_at(1, &catalyst_price_range);
		assert_eq!(cat_price, FixedU128::one() / FixedU128::from(10u128.pow(6)));

		// cat 1100
		let cat_price = Catalyst::get_cat_price_at(1100, &catalyst_price_range);
		assert_eq!(cat_price, FixedU128::from_inner(1_658436414116));

		// cat 10000
		let cat_price = Catalyst::get_cat_price_at(10000, &catalyst_price_range);
		assert_eq!(cat_price, FixedU128::from_inner(99_742323741896));

		// cat 10001
		let cat_price = Catalyst::get_cat_price_at(10001, &catalyst_price_range);
		assert_eq!(cat_price, FixedU128::from_inner(99_788246204604));

		// cat 20505
		let cat_price = Catalyst::get_cat_price_at(20505, &catalyst_price_range);
		assert_eq!(cat_price, FixedU128::from_inner(517_647086862671));

		// cat 50000
		let cat_price = Catalyst::get_cat_price_at(50000, &catalyst_price_range);
		assert_eq!(cat_price, FixedU128::from_inner(3999_163458274803));

		// cat 50001
		let cat_price = Catalyst::get_cat_price_at(50001, &catalyst_price_range);
		assert_eq!(cat_price, FixedU128::from_inner(3999_440668764580));
	});
}

#[test]
fn paticipate_should_work_correct_time_diminishing_less_than_50000() {
	new_test_ext().execute_with(|| {
		init_yieldfarming();

		let (
			catalyst_id,
			_start_block,
			_end_block,
			_catalyst_asset_id,
			otto_asset_id,
			plug_asset_id,
			_offered_amount,
			_catalyst_next_price,
			_catalyst_next_time_diminishing,
			_catalyst_next_number,
			_catalyst_price_range,
		) = list_cat(true);

		let cat_amount = 20000;

		// mint should work with OTTO token
		let otto_balance_before = Utils::asset_balance(otto_asset_id, &TEST_ACCOUNT);
		let vault_account_otto_balance_before =
			Utils::asset_balance(otto_asset_id, &Catalyst::get_vault_account(catalyst_id).unwrap());
		let otto_cost = Catalyst::get_total_cost(catalyst_id, cat_amount, otto_asset_id).unwrap();
		assert_ok!(Catalyst::participate(
			Origin::signed(TEST_ACCOUNT),
			catalyst_id,
			cat_amount,
			otto_asset_id
		));
		assert_eq!(
			Utils::asset_balance(otto_asset_id, &TEST_ACCOUNT),
			otto_balance_before - otto_cost
		);
		assert_eq!(
			Utils::asset_balance(otto_asset_id, &Catalyst::get_vault_account(catalyst_id).unwrap()),
			vault_account_otto_balance_before + otto_cost
		);
		let current_epoch = CatalystReward::get_epoch_id();
		let time_diminishing: FixedU128 = CatalystReward::get_time_diminishing(current_epoch);
		assert_eq!(
			time_diminishing,
			FixedU128::from_float(365.25)
		);
	});
}

#[test]
fn paticipate_should_work_correct_time_diminishing_equal_50000() {
	new_test_ext().execute_with(|| {
		init_yieldfarming();

		let (
			catalyst_id,
			_start_block,
			_end_block,
			_catalyst_asset_id,
			otto_asset_id,
			plug_asset_id,
			_offered_amount,
			_catalyst_next_price,
			_catalyst_next_time_diminishing,
			_catalyst_next_number,
			_catalyst_price_range,
		) = list_cat(true);

		let cat_amount = 50000;

		// mint should work with OTTO token
		let otto_balance_before = Utils::asset_balance(otto_asset_id, &TEST_ACCOUNT);
		let vault_account_otto_balance_before =
			Utils::asset_balance(otto_asset_id, &Catalyst::get_vault_account(catalyst_id).unwrap());
		let otto_cost = Catalyst::get_total_cost(catalyst_id, cat_amount, otto_asset_id).unwrap();
		assert_ok!(Catalyst::participate(
			Origin::signed(TEST_ACCOUNT),
			catalyst_id,
			cat_amount,
			otto_asset_id
		));
		assert_eq!(
			Utils::asset_balance(otto_asset_id, &TEST_ACCOUNT),
			otto_balance_before - otto_cost
		);
		assert_eq!(
			Utils::asset_balance(otto_asset_id, &Catalyst::get_vault_account(catalyst_id).unwrap()),
			vault_account_otto_balance_before + otto_cost
		);
		let current_epoch = CatalystReward::get_epoch_id();
		let time_diminishing: FixedU128 = CatalystReward::get_time_diminishing(current_epoch);
		assert_eq!(
			time_diminishing,
			FixedU128::from_float(730.5)
		);
	});
}

#[test]
fn paticipate_should_work_correct_time_diminishing_larger_than_50000() {
	new_test_ext().execute_with(|| {
		init_yieldfarming();

		let (
			catalyst_id,
			_start_block,
			_end_block,
			_catalyst_asset_id,
			otto_asset_id,
			plug_asset_id,
			_offered_amount,
			_catalyst_next_price,
			_catalyst_next_time_diminishing,
			_catalyst_next_number,
			_catalyst_price_range,
		) = list_cat(true);

		let cat_amount = 50001;

		// mint should work with OTTO token
		let otto_balance_before = Utils::asset_balance(otto_asset_id, &TEST_ACCOUNT);
		let vault_account_otto_balance_before =
			Utils::asset_balance(otto_asset_id, &Catalyst::get_vault_account(catalyst_id).unwrap());
		let otto_cost = Catalyst::get_total_cost(catalyst_id, cat_amount, otto_asset_id).unwrap();
		assert_ok!(Catalyst::participate(
			Origin::signed(TEST_ACCOUNT),
			catalyst_id,
			cat_amount,
			otto_asset_id
		));
		assert_eq!(
			Utils::asset_balance(otto_asset_id, &TEST_ACCOUNT),
			otto_balance_before - otto_cost
		);
		assert_eq!(
			Utils::asset_balance(otto_asset_id, &Catalyst::get_vault_account(catalyst_id).unwrap()),
			vault_account_otto_balance_before + otto_cost
		);
		let current_epoch = CatalystReward::get_epoch_id();
		let time_diminishing: FixedU128 = CatalystReward::get_time_diminishing(current_epoch);
		assert_eq!(
			time_diminishing,
			FixedU128::from_inner(730504777921580490000)
		);
	});
}

pub(crate) const TEST_ACCOUNT: <Test as frame_system::Config>::AccountId = 0;

const P_PLUG_ASSET_ID: AssetId = PLUGAssetId::get();
const P_OTTO_ASSET_ID: AssetId = 1;
const P_CATALYST_ASSET_ID: AssetId = CatalystAssetId::get();
const P_CATALYST_VOUCHER_ASSET_ID: AssetId = CatalystVoucherAssetId::get();
const DEFAULT_EPOCH_START: u128 = 1;
const DEFAULT_EPOCH_DURATION: u128 = 60;

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = <frame_system::GenesisConfig<Test> as BuildStorage>::build_storage(
		&frame_system::GenesisConfig::default(),
	)
	.unwrap();

	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(TEST_ACCOUNT, FixedU128::from(1000000).into_inner()),
			(1, FixedU128::from(1000000).into_inner()),
			(2, FixedU128::from(1000000).into_inner()),
			(3, FixedU128::from(1000000).into_inner()),
			(Catalyst::get_vault_account(0).unwrap_or(0), FixedU128::from(1000000).into_inner()),
		],
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	pallet_assets::GenesisConfig::<Test> {
		assets: vec![
			// id, owner, is_sufficient, min_balance
			(P_PLUG_ASSET_ID, AdminAccountId::get(), true, 1),
			(P_OTTO_ASSET_ID, AdminAccountId::get(), true, 1),
			(P_CATALYST_ASSET_ID, AdminAccountId::get(), true, 1),
			(P_CATALYST_VOUCHER_ASSET_ID, AdminAccountId::get(), true, 1),
		],
		metadata: vec![
			// id, name, symbol, decimals
			(P_PLUG_ASSET_ID, "PLUG TOKEN".into(), "PLUG".into(), 18),
			(P_OTTO_ASSET_ID, "OTTO TOKEN".into(), "OTTO".into(), 12),
			(P_CATALYST_ASSET_ID, "CATALYST TOKEN".into(), "CATA".into(), 0),
			(P_CATALYST_VOUCHER_ASSET_ID, "CATALYST VOUCHER TOKEN".into(), "CATAV".into(), 2),
		],
		accounts: vec![
			// id, account_id, balance
			(P_PLUG_ASSET_ID, TEST_ACCOUNT, FixedU128::from(10u128.pow(18)).into_inner()),
			(P_PLUG_ASSET_ID, 1, FixedU128::from(10u128.pow(18)).into_inner()),
			(P_PLUG_ASSET_ID, 2, FixedU128::from(10u128.pow(18)).into_inner()),
			(P_PLUG_ASSET_ID, 3, FixedU128::from(10u128.pow(18)).into_inner()),
			(
				P_PLUG_ASSET_ID,
				Catalyst::get_vault_account(0).unwrap_or(0),
				FixedU128::from(10u128.pow(12)).into_inner(),
			),
			(P_OTTO_ASSET_ID, TEST_ACCOUNT, FixedU128::from(1000000).into_inner()),
			(
				P_OTTO_ASSET_ID,
				Catalyst::get_vault_account(0).unwrap_or(0),
				FixedU128::from(1000000).into_inner(),
			),
			(P_CATALYST_ASSET_ID, TEST_ACCOUNT, FixedU128::from(1000000).into_inner()),
			(
				P_CATALYST_ASSET_ID,
				Catalyst::get_vault_account(0).unwrap_or(0),
				FixedU128::from(1).into_inner(),
			),
			(
				P_CATALYST_VOUCHER_ASSET_ID,
				Catalyst::get_vault_account(0).unwrap_or(0),
				FixedU128::from(1).into_inner(),
			),
			(P_CATALYST_VOUCHER_ASSET_ID, TEST_ACCOUNT, FixedU128::from(1).into_inner()),
		],
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	let mut ext: sp_io::TestExternalities = storage.into();
	ext.execute_with(|| System::set_block_number(1));
	ext.execute_with(|| {
		Staking::set_epoch_parameter(
			Origin::signed(AdminAccountId::get()),
			CATStakingId::get(),
			U256::from(DEFAULT_EPOCH_START),
			U256::from(DEFAULT_EPOCH_DURATION),
		)
		.unwrap();
	});
	ext
}

fn init_yieldfarming() {
	let total_epochs = 10000;
	let epoch_delay_from_staking = 0;
	let catalyst_token = P_CATALYST_ASSET_ID;
	let asset_reward_token = P_OTTO_ASSET_ID;

	assert_ok!(CatalystReward::init_yieldfarming(
		Origin::signed(AdminAccountId::get()),
		total_epochs,
		epoch_delay_from_staking,
		catalyst_token,
		asset_reward_token
	));
}

fn run_to_block(n: u64) {
	while System::block_number() < n {
		if System::block_number() > 1 {
			Staking::on_finalize(System::block_number());
			System::on_finalize(System::block_number());
		}
		System::set_block_number(System::block_number() + 1);
		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME);
		System::on_initialize(System::block_number());
		Staking::on_initialize(System::block_number());
	}
}

fn run_to_block_with_cata_hooks(n: u64) {
	while System::block_number() < n {
		if System::block_number() > 1 {
			Staking::on_finalize(System::block_number());
			Catalyst::on_finalize(System::block_number());
			System::on_finalize(System::block_number());
		}
		System::set_block_number(System::block_number() + 1);
		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME);
		System::on_initialize(System::block_number());
		Staking::on_initialize(System::block_number());
		Catalyst::on_initialize(System::block_number());
	}
}

fn list_cat(
	list: bool,
) -> (
	u32,
	u64,
	u64,
	u64,
	u64,
	u64,
	u128,
	sp_runtime::FixedU128,
	sp_runtime::FixedU128,
	u128,
	std::vec::Vec<(u128, u128, sp_runtime::FixedU128, sp_runtime::FixedU128)>,
) {
	let catalyst_id = 0;
	let start_block = 1;
	let end_block = 999;
	let catalyst_asset_id = P_CATALYST_ASSET_ID;
	let otto_asset_id = P_OTTO_ASSET_ID;
	let plug_asset_id = P_PLUG_ASSET_ID;
	let offered_amount = 60000;
	let catalyst_next_price = FixedU128::from_inner(1_000000000000);
	let catalyst_next_time_diminishing = FixedU128::saturating_from_rational(1461u32, 4u32);
	let catalyst_next_number = 1;
	let catalyst_price_range = vec![
		(
			1,
			10000,
			FixedU128::from_float(1.000460410996910000),
			FixedU128::saturating_from_rational(1461u32, 4u32),
		),
		(
			10001,
			20000,
			FixedU128::from_float(1.00016114278466),
			FixedU128::saturating_from_rational(1461u32, 4u32),
		),
		(
			20001,
			30000,
			FixedU128::from_float(1.00006933828104),
			FixedU128::saturating_from_rational(1461u32, 4u32),
		),
		(
			30001,
			40000,
			FixedU128::from_float(1.00006930794455),
			FixedU128::saturating_from_rational(1461u32, 4u32),
		),
		(
			40001,
			49999,
			FixedU128::from_float(1.00006931711911),
			FixedU128::saturating_from_rational(1461u32, 4u32),
		),
		(
			50000,
			50000,
			FixedU128::from_float(1.00006931711911),
			FixedU128::saturating_from_rational(1461u32, 2u32),
		),
		(
			50001,
			60000,
			FixedU128::from_float(1.00001308329988),
			FixedU128::saturating_from_rational(1461u32, 1u32),
		),
	];

	if list {
		assert_ok!(Catalyst::list_cat(
			Origin::signed(AdminAccountId::get()),
			catalyst_id,
			start_block,
			end_block,
			catalyst_asset_id,
			otto_asset_id,
			offered_amount,
			catalyst_next_price,
			catalyst_next_time_diminishing,
			catalyst_next_number,
			catalyst_price_range.clone(),
		));
	}

	(
		catalyst_id,
		start_block,
		end_block,
		catalyst_asset_id,
		otto_asset_id,
		plug_asset_id,
		offered_amount,
		catalyst_next_price,
		catalyst_next_time_diminishing,
		catalyst_next_number,
		catalyst_price_range,
	)
}
