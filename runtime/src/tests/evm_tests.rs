//! Integration tests for evm config
#![cfg(test)]

use ethereum::EIP1559Transaction;
use frame_support::{assert_ok, traits::fungible::Inspect};
use pallet_ethereum::{Transaction, TransactionAction};
use sp_core::{H256, U256};

use crate::{
	constants::ONE_XRP,
	impls::scale_wei_to_6dp,
	tests::{bob, charlie, ExtBuilder},
	BaseFee, Ethereum, EthereumChainId, Origin, XrpCurrency,
};

/// Base gas used for an EVM transaction
pub const BASE_TX_GAS_COST: u128 = 21000;

#[test]
fn evm_base_transaction_cost_uses_xrp() {
	ExtBuilder::default().build().execute_with(|| {
		let base_tx_gas_cost_scaled =
			scale_wei_to_6dp(BASE_TX_GAS_COST * BaseFee::base_fee_per_gas().as_u128());
		let charlie_initial_balance = XrpCurrency::balance(&charlie());
		assert_eq!(base_tx_gas_cost_scaled, 315_000); // ensure minimum tx price is 0.315 XRP

		let transaction = Transaction::EIP1559(EIP1559Transaction {
			nonce: U256::zero(),
			max_priority_fee_per_gas: U256::from(1_u64),
			max_fee_per_gas: BaseFee::base_fee_per_gas(),
			gas_limit: U256::from(21_000),
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

		let charlie_xrp_after_call_1 = XrpCurrency::balance(&charlie());
		assert!(charlie_xrp_after_call_1 < charlie_initial_balance);
		let empty_call_gas_cost = charlie_initial_balance - charlie_xrp_after_call_1;
		assert!(empty_call_gas_cost == base_tx_gas_cost_scaled); // 0.315 XRP is lowest cost of TX
	});
}

#[test]
fn evm_transfer_transaction_uses_xrp() {
	ExtBuilder::default().build().execute_with(|| {
		let base_tx_gas_cost_scaled =
			scale_wei_to_6dp(BASE_TX_GAS_COST * BaseFee::base_fee_per_gas().as_u128());
		let charlie_initial_balance = XrpCurrency::balance(&charlie());

		// transfer in xrp
		let transaction = Transaction::EIP1559(EIP1559Transaction {
			nonce: U256::one(),
			max_priority_fee_per_gas: U256::from(1_u64),
			max_fee_per_gas: BaseFee::base_fee_per_gas(),
			gas_limit: U256::from(21_000),
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
