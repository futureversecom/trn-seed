//! Integration tests for evm config
#![cfg(test)]

use super::{TxBuilder, BASE_TX_GAS_COST, MINIMUM_XRP_TX_COST};
use crate::{
	impls::scale_wei_to_6dp,
	tests::{charlie, ExtBuilder},
	Ethereum, FeeControl, XrpCurrency,
};

use frame_support::{
	assert_ok,
	traits::{fungible::Inspect, fungibles::Inspect as Inspects},
};

#[test]
fn abba() {
	ExtBuilder::default().build().execute_with(|| {
		let base_tx_gas_cost_scaled =
			scale_wei_to_6dp(BASE_TX_GAS_COST * FeeControl::base_fee_per_gas().as_u128());
		let charlie_initial_balance = XrpCurrency::balance(&charlie());
		assert_eq!(base_tx_gas_cost_scaled, MINIMUM_XRP_TX_COST); // ensure minimum tx price is 0.315 XRP

		let (origin, tx) = TxBuilder::default().origin(charlie()).build();
		// gas only in xrp
		assert_ok!(Ethereum::transact(origin, tx));

		let charlie_new_balance = XrpCurrency::balance(&charlie());
		assert!(charlie_new_balance < charlie_initial_balance);
		let empty_call_gas_cost = charlie_initial_balance - charlie_new_balance;
		assert_eq!(empty_call_gas_cost, base_tx_gas_cost_scaled); // 0.315 XRP is lowest cost of TX
	});
}
