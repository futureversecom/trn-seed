//! Integration tests for evm config
#![cfg(test)]

use super::{TxBuilder, BASE_TX_GAS_COST, MINIMUM_XRP_TX_COST};
use crate::{
	constants::ONE_XRP,
	impls::scale_wei_to_6dp,
	tests::{alice, bob, charlie, ExtBuilder},
	AccountId, Assets, AssetsExt, Dex, EVMChainId, Ethereum, FeeControl, FeeProxy, Origin, Runtime,
	TxFeePot, XrpCurrency, EVM,
};
use ethabi::Token;
use ethereum::EIP1559Transaction;
use frame_support::{
	assert_ok,
	dispatch::{GetDispatchInfo, RawOrigin},
	traits::{fungible::Inspect, fungibles::Inspect as Inspects, Get},
};
use frame_system::RawOrigin::Root;
use pallet_ethereum::TransactionAction;
use pallet_transaction_payment::ChargeTransactionPayment;
use precompile_utils::{constants::ERC20_PRECOMPILE_ADDRESS_PREFIX, ErcIdConversion};
use seed_client::chain_spec::get_account_id_from_seed;
use seed_primitives::{AssetId, Balance};
use sp_core::{ecdsa, H160, H256, U256};
use sp_runtime::{traits::SignedExtension, DispatchError::BadOrigin};

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
