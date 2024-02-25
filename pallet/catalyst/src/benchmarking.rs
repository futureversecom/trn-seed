use super::*;

use frame_benchmarking::v1::{account, benchmarks, BenchmarkError};
use frame_support::{
	assert_ok,
	traits::{fungibles::Create, EnsureOrigin},
};
use frame_system::{ensure_signed, Pallet as System, RawOrigin};
use pallet_catalyst_reward::Pallet as CatalystReward;
use pallet_plugstaking::Pallet as Staking;
use plug_utils::Pallet as Utility;
use Pallet as Catalyst;

use plug_primitives::{AssetId, BlockNumber};

const SEED: u32 = 0;

fn create_default_asset<T: Config>() -> (T::AccountId, AssetId, AssetId) {
	let asset_id: AssetId = 1;
	let catalyst_asset_id: AssetId = T::CatalystAssetId::get();
	let source_account: T::AccountId = account("source", 0, SEED);
	assert_ok!(T::Assets::create(asset_id, source_account.clone(), true, 1u32.into()));
	assert_ok!(T::Assets::create(catalyst_asset_id, source_account.clone(), true, 1u32.into()));
	(source_account, asset_id, catalyst_asset_id)
}

fn create_default_asset_plug<T: Config>() -> (T::AccountId, AssetId, AssetId, AssetId) {
	let asset_id: AssetId = 1;
	let catalyst_asset_id: AssetId = T::CatalystAssetId::get();
	let catalyst_voucher_asset_id: AssetId = T::CatalystVoucherAssetId::get();
	let plug_id: AssetId = T::PLUGAssetId::get();
	let source_account: T::AccountId = account("source", 0, SEED);
	assert_ok!(T::Assets::create(asset_id, source_account.clone(), true, 1u32.into()));
	assert_ok!(T::Assets::create(catalyst_asset_id, source_account.clone(), true, 1u32.into()));
	assert_ok!(T::Assets::create(
		catalyst_voucher_asset_id,
		source_account.clone(),
		true,
		1u32.into()
	));
	assert_ok!(T::Assets::create(plug_id, source_account.clone(), true, 1u32.into()));
	(source_account, asset_id, catalyst_asset_id, plug_id)
}

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

fn assert_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_has_event(generic_event.into());
}

fn init_pallets_storage<T: Config>() {
	let total_epochs = 10000;
	let epoch_delay_from_staking = 0;
	let catalyst_token: AssetId = T::CatalystAssetId::get();
	let asset_reward_token: AssetId = 1;

	assert_ok!(Staking::<T>::set_epoch_parameter(
		T::StakingAdminOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)
			.unwrap(),
		<T as pallet_catalyst_reward::Config>::CATStakingId::get(),
		1u128.into(),
		60u128.into()
	));
	assert_ok!(CatalystReward::<T>::init_yieldfarming(
		T::CATRewardAdminOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)
			.unwrap(),
		total_epochs,
		epoch_delay_from_staking,
		catalyst_token,
		asset_reward_token,
	));
}

benchmarks! {
	list_ieo {
		System::<T>::set_block_number(BlockNumber::from(1u8).into());
		let catalyst_id = T::CATIdentifier::default();
		let start_block: u32 = 1;
		let end_block: u32 = 999999;
		let catalyst_asset_id: AssetId = T::CatalystAssetId::get();
		let otto_asset_id = 1;
		let offered_amount = 50000;
		let catalyst_next_price = FixedU128::from(1);
		let catalyst_next_time_diminishing = FixedU128::saturating_from_rational(1461u32, 4u32);
		let catalyst_next_number = 0;
		let catalyst_price_range = vec![
			(1, 10000, FixedU128::from_inner(1000460410996910000), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(10001, 20000, FixedU128::from_inner(100016114278466), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(20001, 30000, FixedU128::from_inner(100006933828104), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(30001, 40000, FixedU128::from_inner(100006930794455), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(40001, 50000, FixedU128::from_inner(100006931711911), FixedU128::saturating_from_rational(1461u32, 4u32)),
		];
		assert_ok!(Catalyst::<T>::list_cat(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id, start_block.into(), end_block.into(), catalyst_asset_id, otto_asset_id, offered_amount, catalyst_next_price, catalyst_next_time_diminishing, catalyst_next_number, catalyst_price_range.clone()));

		let blocktime = System::<T>::block_number();
		let plug_asset_id = T::PLUGAssetId::get();
		let cata_voucher_asset_id = T::CatalystVoucherAssetId::get();
	}: _<T::RuntimeOrigin>(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id, end_block.into())
	verify {
		assert_last_event::<T>(Event::<T>::PlugCataIEOEnabled(blocktime, end_block.into(), plug_asset_id, cata_voucher_asset_id).into());
	}

	disable_ieo {
		System::<T>::set_block_number(BlockNumber::from(1u8).into());
		let catalyst_id = T::CATIdentifier::default();
		let start_block: u32 = 1;
		let end_block: u32 = 999999;
		let catalyst_asset_id: AssetId = T::CatalystAssetId::get();
		let otto_asset_id = 1;
		let offered_amount = 50000;
		let catalyst_next_price = FixedU128::from(1);
		let catalyst_next_time_diminishing = FixedU128::saturating_from_rational(1461u32, 4u32);
		let catalyst_next_number = 0;
		let catalyst_price_range = vec![
			(1, 10000, FixedU128::from_inner(1000460410996910000), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(10001, 20000, FixedU128::from_inner(100016114278466), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(20001, 30000, FixedU128::from_inner(100006933828104), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(30001, 40000, FixedU128::from_inner(100006930794455), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(40001, 50000, FixedU128::from_inner(100006931711911), FixedU128::saturating_from_rational(1461u32, 4u32)),
		];
		assert_ok!(Catalyst::<T>::list_cat(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id, start_block.into(), end_block.into(), catalyst_asset_id, otto_asset_id, offered_amount, catalyst_next_price, catalyst_next_time_diminishing, catalyst_next_number, catalyst_price_range.clone()));
		assert_ok!(Catalyst::<T>::list_ieo(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id, end_block.into()));
	}: _<T::RuntimeOrigin>(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id)
	verify {
		assert_last_event::<T>(Event::<T>::PlugCataIEODisabled(catalyst_id).into());
	}

	deposit_plug {
		create_default_asset_plug::<T>();
		System::<T>::set_block_number(BlockNumber::from(1u8).into());

		let admin_origin = <T as pallet::Config>::VerifiedUserOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let admin_account: T::AccountId = ensure_signed(admin_origin.clone()).unwrap();

		let catalyst_id = T::CATIdentifier::default();
		let start_block: u32 = 1;
		let end_block: u32 = 999999;
		let catalyst_asset_id: AssetId = T::CatalystAssetId::get();
		let otto_asset_id = 1;
		let offered_amount = 50000;
		let catalyst_next_price = FixedU128::from(1);
		let catalyst_next_time_diminishing = FixedU128::saturating_from_rational(1461u32, 4u32);
		let catalyst_next_number = 0;
		let catalyst_price_range = vec![
			(1, 10000, FixedU128::from_inner(1000460410996910000), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(10001, 20000, FixedU128::from_inner(100016114278466), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(20001, 30000, FixedU128::from_inner(100006933828104), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(30001, 40000, FixedU128::from_inner(100006930794455), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(40001, 50000, FixedU128::from_inner(100006931711911), FixedU128::saturating_from_rational(1461u32, 4u32)),
		];
		assert_ok!(Catalyst::<T>::list_cat(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id, start_block.into(), end_block.into(), catalyst_asset_id, otto_asset_id, offered_amount, catalyst_next_price, catalyst_next_time_diminishing, catalyst_next_number, catalyst_price_range.clone()));
		assert_ok!(Catalyst::<T>::list_ieo(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id, end_block.into()));

		let plug_asset_id = T::PLUGAssetId::get();

		Utility::<T>::mint(T::AdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, plug_asset_id, admin_account.clone(), offered_amount.clone());

		let cat_amount = 1;

	}: _<T::RuntimeOrigin>(admin_origin.clone(), catalyst_id, cat_amount)
	verify {
		assert_last_event::<T>(Event::<T>::PlugOfferReceived(admin_account, cat_amount).into());
	}

	clear_orderbook {
		create_default_asset_plug::<T>();
		System::<T>::set_block_number(BlockNumber::from(1u8).into());

		let admin_origin = <T as pallet::Config>::VerifiedUserOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let admin_account: T::AccountId = ensure_signed(admin_origin.clone()).unwrap();

		let catalyst_id = T::CATIdentifier::default();
		let start_block: u32 = 1;
		let end_block: u32 = 999999;
		let catalyst_asset_id: AssetId = T::CatalystAssetId::get();
		let otto_asset_id = 1;
		let offered_amount = 50000;
		let catalyst_next_price = FixedU128::from(1);
		let catalyst_next_time_diminishing = FixedU128::saturating_from_rational(1461u32, 4u32);
		let catalyst_next_number = 0;
		let catalyst_price_range = vec![
			(1, 10000, FixedU128::from_inner(1000460410996910000), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(10001, 20000, FixedU128::from_inner(100016114278466), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(20001, 30000, FixedU128::from_inner(100006933828104), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(30001, 40000, FixedU128::from_inner(100006930794455), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(40001, 50000, FixedU128::from_inner(100006931711911), FixedU128::saturating_from_rational(1461u32, 4u32)),
		];
		assert_ok!(Catalyst::<T>::list_cat(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id, start_block.into(), end_block.into(), catalyst_asset_id, otto_asset_id, offered_amount, catalyst_next_price, catalyst_next_time_diminishing, catalyst_next_number, catalyst_price_range.clone()));
		assert_ok!(Catalyst::<T>::list_ieo(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id, end_block.into()));

		let plug_asset_id = T::PLUGAssetId::get();

		Utility::<T>::mint(T::AdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, plug_asset_id, admin_account.clone(), offered_amount.clone());

		let cat_amount = 1;
		assert_ok!(Catalyst::<T>::deposit_plug(admin_origin, catalyst_id, cat_amount));
		assert_ok!(Catalyst::<T>::disable_ieo(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id));

	}: _<T::RuntimeOrigin>(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id)
	verify {
		assert_last_event::<T>(Event::<T>::PlugCataIEOOrderBookClear(catalyst_id).into());
	}

	redeem_cata {
		create_default_asset_plug::<T>();
		init_pallets_storage::<T>();
		System::<T>::set_block_number(BlockNumber::from(1u8).into());

		let admin_origin = <T as pallet::Config>::VerifiedUserOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let admin_account: T::AccountId = ensure_signed(admin_origin.clone()).unwrap();

		let catalyst_id = T::CATIdentifier::default();
		let start_block: u32 = 1;
		let mut end_block: u32 = 999999;
		let catalyst_asset_id = T::CatalystAssetId::get();
		let otto_asset_id = 1;
		let offered_amount = 50000;
		let catalyst_next_price = FixedU128::from(1);
		let catalyst_next_time_diminishing = FixedU128::saturating_from_rational(1461u32, 4u32);
		let catalyst_next_number = 0;
		let catalyst_price_range = vec![
			(1, 10000, FixedU128::from_inner(1000460410996910000), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(10001, 20000, FixedU128::from_inner(100016114278466), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(20001, 30000, FixedU128::from_inner(100006933828104), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(30001, 40000, FixedU128::from_inner(100006930794455), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(40001, 50000, FixedU128::from_inner(100006931711911), FixedU128::saturating_from_rational(1461u32, 4u32)),
		];
		assert_ok!(Catalyst::<T>::list_cat(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id, start_block.into(), end_block.into(), catalyst_asset_id, otto_asset_id, offered_amount, catalyst_next_price, catalyst_next_time_diminishing, catalyst_next_number, catalyst_price_range.clone()));

		let end_block = end_block - 100;
		assert_ok!(Catalyst::<T>::list_ieo(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id, end_block.into()));

		let plug_asset_id = T::PLUGAssetId::get();

		let plug_amount_a = 10u128.pow(18 + 2) * 450;
		Utility::<T>::mint(T::AdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, plug_asset_id, admin_account.clone(), plug_amount_a.clone());
		assert_ok!(Catalyst::<T>::deposit_plug(admin_origin.clone(), catalyst_id, plug_amount_a));

		System::<T>::set_block_number(end_block.into());
		Catalyst::<T>::on_initialize(end_block.into());
		Catalyst::<T>::pay_unsigned(RawOrigin::None.into(), end_block.into());

		System::<T>::set_block_number((end_block+1).into());
		Catalyst::<T>::on_initialize((end_block+1).into());
		Catalyst::<T>::pay_unsigned(RawOrigin::None.into(), (end_block+1).into());

	}: _<T::RuntimeOrigin>(admin_origin.clone(), catalyst_id, 100)
	verify {
		assert_last_event::<T>(Event::<T>::PlugCataVoucherRedeemed(admin_account.clone(), 100).into());
	}

	pay_unsigned {
		create_default_asset_plug::<T>();
		init_pallets_storage::<T>();
		System::<T>::set_block_number(BlockNumber::from(1u8).into());

		let admin_origin = <T as pallet::Config>::VerifiedUserOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let admin_account: T::AccountId = ensure_signed(admin_origin.clone()).unwrap();

		let catalyst_id = T::CATIdentifier::default();
		let start_block: u32 = 1;
		let end_block: u32 = 999999;
		let catalyst_asset_id: AssetId = T::CatalystAssetId::get();
		let otto_asset_id = 1;
		let offered_amount = 50000;
		let catalyst_next_price = FixedU128::from(1);
		let catalyst_next_time_diminishing = FixedU128::saturating_from_rational(1461u32, 4u32);
		let catalyst_next_number = 0;
		let catalyst_price_range = vec![
			(1, 10000, FixedU128::from_inner(1000460410996910000), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(10001, 20000, FixedU128::from_inner(100016114278466), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(20001, 30000, FixedU128::from_inner(100006933828104), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(30001, 40000, FixedU128::from_inner(100006930794455), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(40001, 50000, FixedU128::from_inner(100006931711911), FixedU128::saturating_from_rational(1461u32, 4u32)),
		];
		assert_ok!(Catalyst::<T>::list_cat(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id, start_block.into(), end_block.into(), catalyst_asset_id, otto_asset_id, offered_amount, catalyst_next_price, catalyst_next_time_diminishing, catalyst_next_number, catalyst_price_range.clone()));

		let end_block = end_block - 100;
		assert_ok!(Catalyst::<T>::list_ieo(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id, end_block.into()));

		let plug_asset_id = T::PLUGAssetId::get();

		let plug_amount_a = 10u128.pow(18 + 6) * 450;
		Utility::<T>::mint(T::AdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, plug_asset_id, admin_account.clone(), plug_amount_a.clone());
		assert_ok!(Catalyst::<T>::deposit_plug(admin_origin.clone(), catalyst_id, plug_amount_a));

		System::<T>::set_block_number(end_block.into());
		Catalyst::<T>::on_initialize(end_block.into());
		Catalyst::<T>::pay_unsigned(RawOrigin::None.into(), end_block.into());

	}: _<T::RuntimeOrigin>(RawOrigin::None.into(), end_block.into())
	verify {
		assert_last_event::<T>(Event::<T>::PlugCataIEODone().into());
	}

	list_cat {
		let catalyst_id = T::CATIdentifier::default();
		let start_block: u32 = 1;
		let end_block: u32 = 999999;
		let catalyst_asset_id: AssetId = T::CatalystAssetId::get();
		let otto_asset_id = 1;
		let offered_amount = 50000;
		let catalyst_next_price = FixedU128::from(1);
		let catalyst_next_number = 0;
		let catalyst_next_time_diminishing = FixedU128::saturating_from_rational(1461u32, 4u32);
		let catalyst_price_range = vec![
			(1, 10000, FixedU128::from_inner(1000460410996910000), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(10001, 20000, FixedU128::from_inner(100016114278466), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(20001, 30000, FixedU128::from_inner(100006933828104), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(30001, 40000, FixedU128::from_inner(100006930794455), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(40001, 50000, FixedU128::from_inner(100006931711911), FixedU128::saturating_from_rational(1461u32, 4u32)),
		];
	}: _<T::RuntimeOrigin>(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id, start_block.into(), end_block.into(), catalyst_asset_id, otto_asset_id, offered_amount, catalyst_next_price, catalyst_next_time_diminishing, catalyst_next_number, catalyst_price_range.clone())
	verify {
		assert_last_event::<T>(Event::<T>::CATEnabled(catalyst_id, start_block.into(), end_block.into(), catalyst_asset_id, offered_amount, otto_asset_id).into());
	}

	disable_cat {
		let catalyst_id = T::CATIdentifier::default();
	}: _<T::RuntimeOrigin>(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id)
	verify {
		assert_last_event::<T>(Event::<T>::CATDisabled(catalyst_id).into());
	}

	participate {
		create_default_asset::<T>();
		init_pallets_storage::<T>();
		System::<T>::set_block_number(BlockNumber::from(1u8).into());

		let admin_origin = <T as pallet::Config>::VerifiedUserOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let admin_account: T::AccountId = ensure_signed(admin_origin.clone()).unwrap();

		let catalyst_id = T::CATIdentifier::default();
		let start_block: u32 = 1;
		let end_block: u32 = 999999;
		let catalyst_asset_id: AssetId = T::CatalystAssetId::get();
		let otto_asset_id = 1;
		let offered_amount = 50000;
		let catalyst_next_price = FixedU128::from(1);
		let catalyst_next_time_diminishing = FixedU128::saturating_from_rational(1461u32, 4u32);
		let catalyst_next_number = 0;
		let catalyst_price_range = vec![
			(1, 10000, FixedU128::from_inner(1000460410996910000), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(10001, 20000, FixedU128::from_inner(100016114278466), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(20001, 30000, FixedU128::from_inner(100006933828104), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(30001, 40000, FixedU128::from_inner(100006930794455), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(40001, 50000, FixedU128::from_inner(100006931711911), FixedU128::saturating_from_rational(1461u32, 4u32)),
		];
		assert_ok!(Catalyst::<T>::list_cat(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id, start_block.into(), end_block.into(), catalyst_asset_id, otto_asset_id, offered_amount, catalyst_next_price, catalyst_next_time_diminishing, catalyst_next_number, catalyst_price_range.clone()));

		let cat_amount = 1;
	}: _<T::RuntimeOrigin>(admin_origin.clone(), catalyst_id, cat_amount, otto_asset_id)
	verify {
		assert_last_event::<T>(Event::<T>::OfferReceived(catalyst_id, admin_account, cat_amount, 1000000000000000000u128.into()).into());
	}

	set_price_parameters {
		create_default_asset::<T>();
		init_pallets_storage::<T>();
		System::<T>::set_block_number(BlockNumber::from(1u8).into());

		let catalyst_id = T::CATIdentifier::default();
		let start_block: u32 = 1;
		let end_block: u32 = 999999;
		let catalyst_asset_id: AssetId = T::CatalystAssetId::get();
		let otto_asset_id = 1;
		let offered_amount = 50000;
		let catalyst_next_price = FixedU128::from(1);
		let catalyst_next_time_diminishing = FixedU128::saturating_from_rational(1461u32, 4u32);
		let catalyst_next_number = 0;
		let catalyst_price_range = vec![
			(1, 10000, FixedU128::from_inner(1000460410996910000), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(10001, 20000, FixedU128::from_inner(100016114278466), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(20001, 30000, FixedU128::from_inner(100006933828104), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(30001, 40000, FixedU128::from_inner(100006930794455), FixedU128::saturating_from_rational(1461u32, 4u32)),
			(40001, 50000, FixedU128::from_inner(100006931711911), FixedU128::saturating_from_rational(1461u32, 4u32)),
		];
		assert_ok!(Catalyst::<T>::list_cat(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id, start_block.into(), end_block.into(), catalyst_asset_id, otto_asset_id, offered_amount, catalyst_next_price, catalyst_next_time_diminishing, catalyst_next_number, catalyst_price_range.clone()));
	}: _<T::RuntimeOrigin>(<T as pallet::Config>::CATAdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, catalyst_id, catalyst_next_price, catalyst_next_time_diminishing, catalyst_price_range.clone())
	verify {
		assert_last_event::<T>(Event::<T>::CATPriceSet(catalyst_id, catalyst_next_price, catalyst_price_range.clone()).into());
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test)
}
