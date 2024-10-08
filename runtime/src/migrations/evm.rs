// Copyright 2022-2023 Futureverse Corporation Limited
//
// Licensed under the LGPL, Version 3.0 (the "License");
// you may not use this file except in compliance with the License.
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// You may obtain a copy of the License at the root of this project source code

use crate::{Runtime, Weight, EVM};
use core::str::FromStr;
use frame_support::traits::OnRuntimeUpgrade;
use pallet_evm::AddressMapping;
use sp_core::H160;

#[cfg(feature = "try-runtime")]
use {
  pallet_evm::AccountCodes,
  sp_core::U256,
  sp_runtime::DispatchError,
  sp_std::vec::Vec,
};

const EIP2470_EOA_ADDRESS: &str = "Bb6e024b9cFFACB947A71991E386681B1Cd1477D";
const EIP2470_CONTRACT_ADDRESS: &str = "ce0042B868300000d44A59004Da54A005ffdcf9f";
const EIP2470_CONTRACT_DATA: &str = "608060405234801561001057600080fd5b50610134806100206000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c80634af63f0214602d575b600080fd5b60cf60048036036040811015604157600080fd5b810190602081018135640100000000811115605b57600080fd5b820183602082011115606c57600080fd5b80359060200191846001830284011164010000000083111715608d57600080fd5b91908080601f016020809104026020016040519081016040528093929190818152602001838380828437600092019190915250929550509135925060eb915050565b604080516001600160a01b039092168252519081900360200190f35b6000818351602085016000f5939250505056fea26469706673582212206b44f8a82cb6b156bfcc3dc6aadd6df4eefd204bc928a4397fd15dacf6d5320564736f6c63430006020033";

pub struct Upgrade;

impl OnRuntimeUpgrade for Upgrade {
  #[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
    log::info!(target: "Migration", "EVM: Pre-Upgrade checks for EIP-2470.");

    let factory_deployer = H160::from_str(EIP2470_EOA_ADDRESS).unwrap();
    let factory_address = H160::from_str(EIP2470_CONTRACT_ADDRESS).unwrap();

    // Ensure deployer address is empty (no balance and nonce)
    assert!(EVM::is_account_empty(&factory_deployer), "Factory deployer should be empty");

    // Ensure factory address is empty (no balance, nonce and data)
    assert!(EVM::is_account_empty(&factory_address), "Factory contract should be empty");

    log::info!(target: "Migration", "EVM: Pre-Upgrade checks passed.");

		Ok(Vec::new())
	}

	fn on_runtime_upgrade() -> Weight {
    log::info!(target: "Migration", "🛠️ EVM: creating EIP-2470 factory deployer and factory contract 🛠️");

    let weight = v1::migrate::<Runtime>();

    log::info!(target: "Migration", "✅ EVM: EIP-2470 factory deployer and factory contract successfully created ✅");

    weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), DispatchError> {
    log::info!(target: "Migration", "EVM: Post-Upgrade checks for EIP-2470.");

    let factory_deployer = H160::from_str(EIP2470_EOA_ADDRESS).unwrap();
    let factory_address = H160::from_str(EIP2470_CONTRACT_ADDRESS).unwrap();

    // Check factory deployer account
    let (deployer_account, _) = EVM::account_basic(&factory_deployer);
    assert_eq!(deployer_account.balance, U256::zero(), "Factory deployer balance should be 0");
    assert_eq!(deployer_account.nonce, U256::one(), "Factory deployer nonce should be 1");

    // Check factory contract
    let (contract_account, _) = EVM::account_basic(&factory_address);
    assert_eq!(contract_account.balance, U256::zero(), "Factory contract balance should be 0");
    assert_eq!(contract_account.nonce, U256::one(), "Factory contract nonce should be 1");
    
    let contract_code = AccountCodes::<Runtime>::get(factory_address);
    assert!(!contract_code.is_empty(), "Factory contract should have code");
    
    let expected_code = hex::decode(EIP2470_CONTRACT_DATA).unwrap();
    assert_eq!(contract_code, expected_code, "Factory contract code mismatch");

    log::info!(target: "Migration", "EVM: Post-Upgrade checks passed.");

		Ok(())
	}
}

#[allow(dead_code)]
pub mod v1 {
	use super::*;

	pub fn migrate<T: frame_system::Config + pallet_evm::Config>() -> Weight
	where
    <Runtime as frame_system::Config>::AccountId: From<H160>,
	{
    let factory_deployer = H160::from_str(EIP2470_EOA_ADDRESS).unwrap();
    let factory_address = H160::from_str(EIP2470_CONTRACT_ADDRESS).unwrap();

		let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(6);

    // r: 3 + 3, w: 0
    if !(EVM::is_account_empty(&factory_deployer) && EVM::is_account_empty(&factory_address)) {
			log::info!(target: "Migration", "XRPLBridge: No migration was done, however migration code needs to be removed.");
      return weight;
    }

    // Set nonce for factory deployer
    // r: 0, w: 1
    frame_system::Pallet::<Runtime>::inc_account_nonce(&<Runtime as pallet_evm::Config>::AddressMapping::into_account_id(factory_deployer));

    // Set data (bytecode) and nonce for factory contract
    EVM::create_account(factory_address, hex::decode(EIP2470_CONTRACT_DATA).unwrap()); // r: 1, w: 2
    // r: 0, w: 1
    frame_system::Pallet::<Runtime>::inc_account_nonce(&<Runtime as pallet_evm::Config>::AddressMapping::into_account_id(factory_address));

		weight += <Runtime as frame_system::Config>::DbWeight::get().reads_writes(1, 4);

    weight
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::migrations::tests::new_test_ext;
    use frame_support::weights::Weight;

		#[test]
		fn migrate_eip2470() {
			new_test_ext().execute_with(|| {
        // Pre-upgrade checks
        let pre_upgrade_state = Upgrade::pre_upgrade().expect("Pre-upgrade should succeed");

        // Do runtime upgrade
        let weight = Upgrade::on_runtime_upgrade();

        // Validate weight is non-zero
        assert!(weight.any_gt(Weight::zero()), "Migration weight should be non-zero");

        // Post-upgrade checks
        Upgrade::post_upgrade(pre_upgrade_state).expect("Post-upgrade should succeed");

        // Validate future runtime upgrade fails (6 reads only)
        let new_weight = Upgrade::on_runtime_upgrade();
        assert!(new_weight.eq(&<Runtime as frame_system::Config>::DbWeight::get().reads(6)), "Migration weight mismatch");
			});
		}
	}
}
