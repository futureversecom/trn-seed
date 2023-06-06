use crate::mock::*;
use frame_support::{assert_ok, weights::GetDispatchInfo};
use frame_system::RawOrigin;
use pallet_transaction_payment::ChargeTransactionPayment;
use seed_primitives::AccountId20;
use sp_runtime::traits::{Dispatchable, SignedExtension};

#[test]
fn normal_upgrade_costs_more() {
	new_test_ext().execute_with(|| {
		let set_code_pallet_call = frame_system::Call::<Test>::set_code {
			code: substrate_test_runtime_client::runtime::wasm_binary_unwrap().to_vec(),
		};

		let runtime_call = <Test as frame_system::Config>::Call::from(set_code_pallet_call);
		let dispatch_info = runtime_call.get_dispatch_info();

		let alice = AccountId20([1; 20]);
		let balance = 1000000000000;

		assert_ok!(Assets::force_create(RawOrigin::Root.into(), 2, alice, true, 20));
		assert_ok!(Assets::mint(RawOrigin::Signed(alice).into(), 2, alice, balance));

		let pre = <ChargeTransactionPayment<Test> as SignedExtension>::pre_dispatch(
			ChargeTransactionPayment::from(0),
			&alice,
			&runtime_call,
			&dispatch_info,
			1,
		)
		.unwrap();

		let post_dispatch_info = runtime_call.clone().dispatch(RawOrigin::Root.into()).unwrap();

		assert_ok!(<ChargeTransactionPayment<Test> as SignedExtension>::post_dispatch(
			Some(pre),
			&runtime_call.get_dispatch_info(),
			&post_dispatch_info,
			50,
			&Ok(())
		));

		// This amount may change in the future if the fees change, and that is okay
		assert_eq!(Assets::balance(2, &alice), 999874986713);
	})
}

#[test]
fn set_code_cheap_costs_less() {
	new_test_ext().execute_with(|| {
		let set_code_pallet_call = pallet_seed_utils::Call::<Test>::set_code_cheap {
			code: substrate_test_runtime_client::runtime::wasm_binary_unwrap().to_vec(),
		};
		let runtime_call = <Test as frame_system::Config>::Call::from(set_code_pallet_call);
		let dispatch_info = runtime_call.get_dispatch_info();

		let alice = AccountId20([1; 20]);
		let balance = 1000000000000;

		assert_ok!(Assets::force_create(RawOrigin::Root.into(), 2, alice, true, 20));
		assert_ok!(Assets::mint(RawOrigin::Signed(alice).into(), 2, alice, balance));

		let pre = <ChargeTransactionPayment<Test> as SignedExtension>::pre_dispatch(
			ChargeTransactionPayment::from(0),
			&alice,
			&runtime_call,
			&dispatch_info,
			1,
		)
		.unwrap();

		let post_dispatch_info = runtime_call.clone().dispatch(RawOrigin::Root.into()).unwrap();

		// The transaction is made cheap through avoiding the normal payment through `Pays::No`. The
		// check for that payment setting occurs in the post_dispatch so we must invoke that here as
		// well
		assert_ok!(<ChargeTransactionPayment<Test> as SignedExtension>::post_dispatch(
			Some(pre),
			&runtime_call.get_dispatch_info(),
			&post_dispatch_info,
			50,
			&Ok(())
		));

		assert_eq!(Assets::balance(2, &alice), balance - WithdrawAmount::get());
	})
}
