//! Integration tests for evm config
#![cfg(test)]

use ethereum::EIP1559Transaction;
use frame_support::{assert_ok, traits::fungible::Inspect};
use pallet_ethereum::{Transaction, TransactionAction};
use sp_core::{H256, U256};

use crate::{
	constants::ONE_XRP,
	tests::{bob, charlie, ExtBuilder},
	BaseFee, Ethereum, EthereumChainId, Origin, XrpCurrency,
};

#[test]
fn evm_transfer_and_gas_uses_xrp() {
	ExtBuilder::default().build().execute_with(|| {
		let charlie_initial_xrp = XrpCurrency::balance(&charlie());

		let transaction = Transaction::EIP1559(EIP1559Transaction {
			nonce: U256::zero(),
			max_priority_fee_per_gas: U256::from(1_u64),
			max_fee_per_gas: BaseFee::base_fee_per_gas(),
			gas_limit: U256::from(1_000_000_u64),
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
		assert!(charlie_xrp_after_call_1 < charlie_initial_xrp);
		let empty_call_gas = charlie_initial_xrp - charlie_xrp_after_call_1;
		println!("{:?}", empty_call_gas);
		assert!(empty_call_gas < 2 * ONE_XRP); // keep gas cost low

		// transfer in xrp
		let transaction = Transaction::EIP1559(EIP1559Transaction {
			nonce: U256::one(),
			max_priority_fee_per_gas: U256::from(1_u64),
			max_fee_per_gas: BaseFee::base_fee_per_gas(),
			gas_limit: U256::from(1_000_000_u64),
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

		assert!(XrpCurrency::balance(&charlie()) < charlie_xrp_after_call_1 + 5 * ONE_XRP);
		assert_eq!(charlie_initial_xrp + 5 * ONE_XRP, XrpCurrency::balance(&bob()),);
	});
}
