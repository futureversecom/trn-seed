//! Integration tests for evm config
#![cfg(test)]

use crate::{
	constants::ONE_XRP,
	impls::scale_wei_to_6dp,
	tests::{alice, bob, charlie, ExtBuilder},
	Assets, AssetsExt, BaseFee, Dex, Ethereum, EthereumChainId, FeeProxy, Origin, Runtime,
	XrpCurrency, EVM,
};
use ethabi::Token;
use ethereum::EIP1559Transaction;
use frame_support::{
	assert_ok,
	dispatch::{GetDispatchInfo, RawOrigin},
	traits::{fungible::Inspect, fungibles::Inspect as Inspects},
};
use pallet_ethereum::{Transaction, TransactionAction};
use pallet_transaction_payment::ChargeTransactionPayment;
use precompile_utils::{constants::ERC20_PRECOMPILE_ADDRESS_PREFIX, ErcIdConversion};
use seed_client::chain_spec::get_account_id_from_seed;
use seed_primitives::{AssetId, Balance};
use sp_core::{ecdsa, H160, H256, U256};
use sp_runtime::{traits::SignedExtension, DispatchError::BadOrigin};

/// Base gas used for an EVM transaction
pub const BASE_TX_GAS_COST: u128 = 21000;
pub const MINIMUM_XRP_TX_COST: u128 = 315_000;

#[test]
fn evm_base_transaction_cost_uses_xrp() {
	ExtBuilder::default().build().execute_with(|| {
		let base_tx_gas_cost_scaled =
			scale_wei_to_6dp(BASE_TX_GAS_COST * BaseFee::base_fee_per_gas().as_u128());
		let charlie_initial_balance = XrpCurrency::balance(&charlie());
		assert_eq!(base_tx_gas_cost_scaled, MINIMUM_XRP_TX_COST); // ensure minimum tx price is 0.315 XRP

		let transaction = Transaction::EIP1559(EIP1559Transaction {
			nonce: U256::zero(),
			max_priority_fee_per_gas: U256::from(1_u64),
			max_fee_per_gas: BaseFee::base_fee_per_gas(),
			gas_limit: U256::from(BASE_TX_GAS_COST),
			action: TransactionAction::Call(bob().into()),
			value: U256::zero(),
			input: vec![],
			access_list: vec![],
			chain_id: EthereumChainId::get(),
			r: H256::default(),
			s: H256::default(),
			odd_y_parity: true,
		});

		// gas only in xrp
		assert_ok!(Ethereum::transact(
			Origin::from(pallet_ethereum::RawOrigin::EthereumTransaction(charlie().into())),
			transaction,
		));

		let charlie_new_balance = XrpCurrency::balance(&charlie());
		assert!(charlie_new_balance < charlie_initial_balance);
		let empty_call_gas_cost = charlie_initial_balance - charlie_new_balance;
		assert_eq!(empty_call_gas_cost, base_tx_gas_cost_scaled); // 0.315 XRP is lowest cost of TX
	});
}

#[test]
fn evm_transfer_transaction_uses_xrp() {
	ExtBuilder::default().build().execute_with(|| {
		let charlie_initial_balance = XrpCurrency::balance(&charlie());

		// transfer in xrp
		let transaction = Transaction::EIP1559(EIP1559Transaction {
			nonce: U256::one(),
			max_priority_fee_per_gas: U256::from(1_u64),
			max_fee_per_gas: BaseFee::base_fee_per_gas(),
			gas_limit: U256::from(BASE_TX_GAS_COST),
			action: TransactionAction::Call(bob().into()),
			value: U256::from(5 * 10_u128.pow(18_u32)), // transfer value, 5 XRP
			chain_id: EthereumChainId::get(),
			input: vec![],
			access_list: vec![],
			r: H256::default(),
			s: H256::default(),
			odd_y_parity: true,
		});
		assert_ok!(Ethereum::transact(
			Origin::from(pallet_ethereum::RawOrigin::EthereumTransaction(charlie().into())),
			transaction,
		));

		let expected_total_cost_of_tx = scale_wei_to_6dp(
			BASE_TX_GAS_COST * BaseFee::base_fee_per_gas().as_u128() + 5 * 10_u128.pow(18_u32),
		);
		let charlie_balance_change = charlie_initial_balance - XrpCurrency::balance(&charlie());
		assert_eq!(charlie_balance_change, expected_total_cost_of_tx);
		assert_eq!(charlie_initial_balance + 5 * ONE_XRP, XrpCurrency::balance(&bob()),);
	});
}

#[test]
fn evm_call_success_by_any_address() {
	ExtBuilder::default().build().execute_with(|| {
		let result = EVM::call(
			Origin::signed(charlie()),
			charlie().into(),
			bob().into(),
			Vec::new(),
			U256::default(),
			1000000,
			U256::from(1_500_000_000_000_000u64),
			None,
			None,
			Vec::new(),
		);
		result.expect("EVM can be called");
	});
}

#[test]
fn evm_call_fail_by_origin_mismatch() {
	ExtBuilder::default().build().execute_with(|| {
		let result = EVM::call(
			Origin::signed(alice()),
			charlie().into(),
			bob().into(),
			Vec::new(),
			U256::default(),
			1000000,
			U256::from(1_500_000_000_000_000u64),
			None,
			None,
			Vec::new(),
		);
		assert!(result.is_err());
		assert_eq!(result.unwrap_err().error, BadOrigin);
	});
}

#[test]
/// This test tests whether we can call evm.call using FeeProxy.call_with_fee_preferences()
fn fee_proxy_call_evm_with_fee_preferences() {
	ExtBuilder::default().build().execute_with(|| {
		let new_account = get_account_id_from_seed::<ecdsa::Public>("Im Broke");

		// The next minted asset id
		let payment_asset = AssetsExt::next_asset_uuid().unwrap();

		// Lets create an asset
		assert_ok!(AssetsExt::create_asset(
			RawOrigin::Signed(alice()).into(),
			b"Test".to_vec(),
			b"Test".to_vec(),
			6,
			None,
			None
		));

		// Check Bob's initial balance is 0
		assert_eq!(AssetsExt::reducible_balance(payment_asset, &bob(), false), 0);

		// Mint these assets into Alice and new_account
		assert_ok!(Assets::mint(
			RawOrigin::Signed(alice()).into(),
			payment_asset,
			alice(),
			10_000_000_000_000_000
		));
		assert_ok!(Assets::mint(
			RawOrigin::Signed(alice()).into(),
			payment_asset,
			new_account,
			10_000_000_000_000_000
		));

		// Add liquidity to the dex, this will allow for exchange internally when the call is made
		assert_ok!(Dex::add_liquidity(
			RawOrigin::Signed(alice()).into(),
			2,
			payment_asset,
			1_000_000_000_000,
			10_000,
			1,
			1,
			1
		));

		let transfer_amount: Balance = 12345;
		let target: H160 = <Runtime as ErcIdConversion<AssetId>>::runtime_id_to_evm_id(
			payment_asset,
			ERC20_PRECOMPILE_ADDRESS_PREFIX,
		)
		.into();

		// Setup input for an erc20 transfer to Bob
		let mut input: Vec<u8> = [0xa9, 0x05, 0x9c, 0xbb].to_vec();
		input.append(&mut ethabi::encode(&[
			Token::Address(bob().into()),
			Token::Uint(transfer_amount.into()),
		]));
		// Setup inner EVM.call call
		let access_list: Vec<(H160, Vec<H256>)> = vec![];
		let inner_call = crate::Call::EVM(pallet_evm::Call::call {
			source: new_account.into(),
			target,
			input,
			value: U256::default(),
			gas_limit: 50_000,
			max_fee_per_gas: U256::from(1_600_000_000_000_000_u64),
			max_priority_fee_per_gas: None,
			nonce: None,
			access_list,
		});

		let max_payment: Balance = 10_000_000_000_000_000;
		let call = crate::Call::FeeProxy(pallet_fee_proxy::Call::call_with_fee_preferences {
			payment_asset,
			max_payment,
			call: Box::new(inner_call.clone()),
		});

		let dispatch_info = call.get_dispatch_info();

		// Call pre_dispatch, which hits OnChargeTransaction and exchanges the fee
		assert_ok!(<ChargeTransactionPayment<Runtime> as SignedExtension>::pre_dispatch(
			ChargeTransactionPayment::from(0),
			&new_account,
			&call,
			&dispatch_info,
			1,
		));

		// call.dispatch();
		assert_ok!(FeeProxy::call_with_fee_preferences(
			RawOrigin::Signed(new_account).into(),
			payment_asset,
			max_payment,
			Box::new(inner_call)
		));

		// Check Bob has been transferred the correct amount
		assert_eq!(AssetsExt::reducible_balance(payment_asset, &bob(), false), transfer_amount);
	});
}
