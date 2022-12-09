
use frame_support::{assert_ok, dispatch::GetDispatchInfo, traits::fungibles::Mutate};
use frame_system::RawOrigin;
use pallet_transaction_payment::ChargeTransactionPayment;
use sp_runtime::{traits::SignedExtension, Perbill};
use seed_pallet_common::CreateExt;
use crate::mock::*;

#[test]
fn charges_default_extrinsic_amount() {
	TestExt::default().build().execute_with(|| {
        let account = AccountId::default();
        assert_ok!(AssetsExt::create(&account.into()));

        let starting_fee_token_asset_balance = 4200000069;
        assert_ok!(AssetsExt::mint_into(100, &account, starting_fee_token_asset_balance));
    
        let fee_token_balance = Assets::balance(100, account);
        assert_eq!(fee_token_balance, starting_fee_token_asset_balance);
        assert_ok!(MockPallet::mock_charge_fee(RawOrigin::Signed(account).into()));

        let call = mock_pallet::pallet::Call::mock_charge_fee {};
        let dispatch_info = call.get_dispatch_info();

		assert_ok!(<ChargeTransactionPayment<Test> as SignedExtension>::pre_dispatch(
			ChargeTransactionPayment::from(0),
			&account,
			&call.into(),
			&dispatch_info,
			1,
		));

        assert_eq!(Assets::balance(100, account), 4199997532);
    });
}

#[test]
fn charges_extrinsic_fee_based_on_setting() {
	TestExt::default().build().execute_with(|| {
        let account = AccountId::default();
        assert_ok!(AssetsExt::create(&account.into()));

        let starting_fee_token_asset_balance = 4200000069;
        assert_ok!(AssetsExt::mint_into(100, &account, starting_fee_token_asset_balance));
    
        let fee_token_balance = Assets::balance(100, account);
        assert_eq!(fee_token_balance, starting_fee_token_asset_balance);
        assert_ok!(MockPallet::mock_charge_fee(RawOrigin::Signed(account).into()));

        assert_ok!(FeeOracle::do_set_extrinsic_weight_to_fee_factor(Perbill::from_percent(42)));

        let call = mock_pallet::pallet::Call::mock_charge_fee {};
        let dispatch_info = call.get_dispatch_info();

		assert_ok!(<ChargeTransactionPayment<Test> as SignedExtension>::pre_dispatch(
			ChargeTransactionPayment::from(0),
			&account,
			&call.into(),
			&dispatch_info,
			1,
		));

        assert_eq!(Assets::balance(100, account), 4076579309);
    });
}